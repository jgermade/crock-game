//! cROCK — capa de plataforma sobre SDL3.
//!
//! Camino crítico en un solo hilo: eventos → simulación a paso fijo → render
//! interpolado → present. Nada más se ejecuta entre leer la entrada y
//! simular. El único hilo auxiliar es el de audio (de SDL), que nunca bloquea
//! a este.

mod assets;
mod audio;
mod fx;
mod gfx;
mod input;
mod menu;
mod render;
mod settings;
mod stats;

use std::collections::VecDeque;
use std::time::Duration;

use crock_sim::{DT_NS, SIM_HZ, SimEvent, World, step};
use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::{Mod, Scancode};
use sdl3::sys;

use crate::audio::{Audio, Sfx};
use crate::input::InputState;
use crate::menu::{Action, Menu, Nav, Page};
use crate::settings::{FPS_CAPS, Fullscreen, PresentMode, Settings, cycle};
use crate::stats::Stats;

const FRAME_DELAYS_MS: [u32; 6] = [0, 1, 2, 4, 6, 8];

fn now_ns() -> u64 {
    unsafe { sys::timer::SDL_GetTicksNS() }
}

/// Espera precisa: `sleep` hasta ~2 ms antes y spin-wait el resto.
fn wait_until(deadline_ns: u64) {
    const SPIN_NS: u64 = 2_000_000;
    loop {
        let now = now_ns();
        if now >= deadline_ns {
            return;
        }
        let rem = deadline_ns - now;
        if rem > SPIN_NS {
            std::thread::sleep(Duration::from_nanos(rem - SPIN_NS));
        } else {
            std::hint::spin_loop();
        }
    }
}

struct Options {
    present: Option<PresentMode>,
    fps_cap: Option<u32>,
    fullscreen: bool,
    /// Salir tras N frames (pruebas automáticas).
    frames: Option<u64>,
    /// Guardar captura BMP del último frame.
    screenshot: Option<String>,
    /// Entrada scriptada para pruebas: mantener derecha y saltar periódicamente.
    autoplay: bool,
    /// Capítulo inicial (índice o parte del nombre) y sala (desde 1).
    chapter: String,
    room: usize,
    overlay: bool,
    /// Reproducir entradas grabadas (una por tick).
    replay: Option<String>,
    /// Grabar las entradas desde el último reinicio al salir.
    record: Option<String>,
    /// Empezar jugando, sin pantalla de título.
    no_menu: bool,
    /// Pantalla a capturar con --screenshot: título, pausa u opciones.
    show_menu: Option<Page>,
}

fn parse_args() -> Options {
    let mut o = Options {
        present: None,
        fps_cap: None,
        fullscreen: false,
        frames: None,
        screenshot: None,
        autoplay: false,
        chapter: String::new(),
        room: 1,
        overlay: false,
        replay: None,
        record: None,
        no_menu: false,
        show_menu: None,
    };
    for a in std::env::args().skip(1) {
        let (k, v) = a.split_once('=').unwrap_or((a.as_str(), ""));
        match k {
            "--vsync" => o.present = PresentMode::parse(v),
            "--fps" => o.fps_cap = v.parse().ok(),
            "--fullscreen" => o.fullscreen = true,
            "--frames" => o.frames = v.parse().ok(),
            "--screenshot" => o.screenshot = Some(v.to_string()),
            "--autoplay" => o.autoplay = true,
            "--replay" => o.replay = Some(v.to_string()),
            "--record" => o.record = Some(v.to_string()),
            "--chapter" => o.chapter = v.to_string(),
            "--room" => o.room = v.parse().unwrap_or(1),
            "--overlay" => o.overlay = true,
            "--no-menu" => o.no_menu = true,
            "--menu" => {
                o.show_menu = match v {
                    "pause" => Some(Page::Pause),
                    "options" => Some(Page::Options),
                    _ => Some(Page::Title),
                }
            }
            "-h" | "--help" => {
                println!(
                    "crock [--vsync=on|adaptive|off] [--fps=N] [--fullscreen] [--overlay]\n\
                     \x20     [--chapter=N|nombre] [--room=N] [--no-menu]\n\
                     \x20     [--replay=f] [--record=f]\n\
                     \x20     [--frames=N] [--screenshot=out.bmp] [--autoplay] [--menu=title|pause|options]"
                );
                std::process::exit(0);
            }
            _ => eprintln!("[crock] argumento desconocido: {a}"),
        }
    }
    // Cualquier opción de pruebas o de sala concreta empieza jugando.
    if o.replay.is_some() || o.autoplay || o.frames.is_some() || !o.chapter.is_empty() || o.room > 1
    {
        o.no_menu = true;
    }
    o
}

fn renderer_name(canvas: &sdl3::render::WindowCanvas) -> String {
    unsafe {
        let p = sys::render::SDL_GetRendererName(canvas.raw());
        if p.is_null() {
            "?".into()
        } else {
            std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    }
}

fn save_screenshot(canvas: &sdl3::render::WindowCanvas, path: &str) {
    unsafe {
        let surf = sys::render::SDL_RenderReadPixels(canvas.raw(), std::ptr::null());
        if surf.is_null() {
            eprintln!("[crock] no se pudo leer el framebuffer");
            return;
        }
        let c = std::ffi::CString::new(path).unwrap();
        if !sys::surface::SDL_SaveBMP(surf, c.as_ptr()) {
            eprintln!("[crock] no se pudo guardar {path}");
        }
        sys::surface::SDL_DestroySurface(surf);
    }
}

/// Repetición de navegación con el stick (en menús).
struct StickNav {
    dir: (i8, i8),
    next_ns: u64,
}

impl StickNav {
    fn poll(&mut self, dir: (i8, i8), now: u64) -> Option<Nav> {
        let nav = match dir {
            (_, -1) => Some(Nav::Up),
            (_, 1) => Some(Nav::Down),
            (-1, 0) => Some(Nav::Left),
            (1, 0) => Some(Nav::Right),
            _ => None,
        };
        if dir != self.dir {
            self.dir = dir;
            self.next_ns = now + 400_000_000;
            return nav;
        }
        if nav.is_some() && now >= self.next_ns {
            self.next_ns = now + 120_000_000;
            return nav;
        }
        None
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = parse_args();
    let mut settings = Settings::load();
    if let Some(p) = opts.present {
        settings.present = p;
    }
    if let Some(f) = opts.fps_cap {
        settings.fps_cap = f;
    }
    if opts.fullscreen {
        settings.fullscreen = Fullscreen::Exclusive;
    }
    settings.overlay |= opts.overlay;

    let sdl = sdl3::init()?;
    let video = sdl.video()?;
    let gamepads = sdl.gamepad()?;
    let audio = Audio::new(&sdl, settings.volume_f());

    let window = video
        .window("cROCK", 1280, 720)
        .resizable()
        .high_pixel_density()
        .build()?;
    let mut canvas = window.into_canvas();
    settings.present = settings::apply_present(&canvas, settings.present);
    if settings.fullscreen != Fullscreen::Windowed {
        settings::apply_fullscreen(canvas.window(), settings.fullscreen);
    }
    let renderer = renderer_name(&canvas);

    let assets = assets::assets_dir();
    let mut tuning_watch = assets::TuningWatcher::new(&assets);
    let gfx = gfx::Gfx::load(&canvas, &assets);
    let chapters = assets::chapters(&assets);
    let mut chapter = opts
        .chapter
        .parse::<usize>()
        .ok()
        .map(|n| n.saturating_sub(1))
        .or_else(|| {
            let q = opts.chapter.to_lowercase();
            chapters
                .iter()
                .position(|c| !q.is_empty() && c.name.to_lowercase().contains(&q))
        })
        .unwrap_or(0)
        .min(chapters.len() - 1);
    let load = |ch: usize, tuning: crock_sim::Tuning| -> Result<World, String> {
        Ok(World::with_rooms(
            assets::load_chapter(&chapters[ch])?,
            tuning,
        ))
    };
    let mut world = load(chapter, Default::default())?;
    if opts.room > 1 {
        world.goto_room(opts.room - 1);
    }
    let mut banner: Option<(String, u64)> = None;
    if let Some((t, warns)) = tuning_watch.poll(now_ns()) {
        world.tuning = t;
        warns
            .iter()
            .for_each(|w| eprintln!("[crock] tuning.cfg: {w}"));
    }

    let mut input = InputState::new(gamepads);
    input.deadzone = settings.deadzone_f();
    input.strike_button = settings.strike_button;
    let mut events = sdl.event_pump()?;
    let mut stats = Stats::new();
    let mut fx = fx::Fx::new();
    let mut replay: VecDeque<crock_sim::Input> = match &opts.replay {
        Some(path) => crock_sim::replay::decode(&std::fs::read_to_string(path)?)?.into(),
        None => Default::default(),
    };
    let mut recording: Vec<crock_sim::Input> = Vec::new();

    let mut menu = Menu::new();
    if let Some(p) = opts.show_menu {
        menu.reset_to(Page::Title);
        if p != Page::Title {
            menu.reset_to(p);
        }
    } else if !opts.no_menu {
        menu.reset_to(Page::Title);
    }
    let mut stick_nav = StickNav {
        dir: (0, 0),
        next_ns: 0,
    };

    let mut delay_idx = 0usize;
    let mut message: Option<(String, u64)> = None;

    let mut acc_ns: u64 = 0;
    let mut last_ns = now_ns();
    let mut last_present_ns = last_ns;
    let mut next_deadline_ns = last_ns;
    let mut frame: u64 = 0;
    let mut alpha = 0.0f32;

    'main: loop {
        // --- Frame pacing: esperar ANTES de leer la entrada ----------------
        let delay_ns = FRAME_DELAYS_MS[delay_idx] as u64 * 1_000_000;
        if delay_ns > 0 {
            wait_until(last_present_ns + delay_ns);
        }
        let cap = settings.fps_cap;
        if cap > 0 {
            let period = 1_000_000_000 / cap as u64;
            wait_until(next_deadline_ns);
            next_deadline_ns = (next_deadline_ns + period).max(now_ns());
        }

        let frame_start = now_ns();
        let mut settings_changed = false;

        // --- Entrada ------------------------------------------------------
        let mut navs: Vec<Nav> = Vec::new();
        for ev in events.poll_iter() {
            match ev {
                Event::Quit { .. } => break 'main,
                Event::Window {
                    win_event: WindowEvent::CloseRequested,
                    ..
                } => break 'main,
                _ => {}
            }
            if menu.is_open() {
                if let Some(n) = input::menu_nav(&ev) {
                    navs.push(n);
                }
                // Mantener al día el estado de teclas y mandos (sueltas, hotplug).
                if !matches!(ev, Event::KeyDown { .. }) {
                    input.handle(&ev);
                }
                continue;
            }
            // Pausa: Esc o Start.
            let pause = matches!(
                ev,
                Event::KeyDown {
                    scancode: Some(Scancode::Escape),
                    repeat: false,
                    ..
                } | Event::GamepadButtonDown {
                    button: sdl3::gamepad::Button::Start,
                    ..
                }
            );
            if pause {
                menu.open(Page::Pause);
                audio.play(Sfx::MenuSelect, 0.5);
                continue;
            }
            if let Event::KeyDown {
                scancode: Some(sc),
                repeat: false,
                keymod,
                ..
            } = ev
            {
                let handled = match sc {
                    Scancode::R => {
                        world.restart();
                        recording.clear();
                        true
                    }
                    Scancode::F1 => {
                        settings.overlay = !settings.overlay;
                        settings_changed = true;
                        true
                    }
                    Scancode::F2 => {
                        settings.present = cycle(&PresentMode::ALL, settings.present, 1);
                        settings_changed = true;
                        true
                    }
                    Scancode::F3 => {
                        settings.fps_cap = cycle(&FPS_CAPS, settings.fps_cap, 1);
                        settings_changed = true;
                        true
                    }
                    Scancode::F4 => {
                        delay_idx = (delay_idx + 1) % FRAME_DELAYS_MS.len();
                        true
                    }
                    Scancode::F5 | Scancode::F6 => {
                        // F5 recarga el capítulo (misma sala); F6 pasa al siguiente.
                        let room = if sc == Scancode::F6 {
                            chapter = (chapter + 1) % chapters.len();
                            0
                        } else {
                            world.room
                        };
                        match load(chapter, world.tuning.clone()) {
                            Ok(mut w) => {
                                w.deaths = if sc == Scancode::F5 { world.deaths } else { 0 };
                                w.goto_room(room);
                                world = w;
                                message = Some((chapters[chapter].name.clone(), now_ns()));
                            }
                            Err(e) => {
                                eprintln!("[crock] {e}");
                                message = Some((e, now_ns()));
                            }
                        }
                        recording.clear();
                        true
                    }
                    Scancode::F7 | Scancode::F8 => {
                        let n = world.rooms.len();
                        let room = if sc == Scancode::F8 {
                            world.room + 1
                        } else {
                            world.room + n - 1
                        };
                        world.goto_room(room % n);
                        recording.clear();
                        true
                    }
                    Scancode::F11 => {
                        settings.fullscreen = cycle(&Fullscreen::ALL, settings.fullscreen, 1);
                        settings_changed = true;
                        true
                    }
                    Scancode::Return if keymod.intersects(Mod::LALTMOD | Mod::RALTMOD) => {
                        settings.fullscreen = if settings.fullscreen == Fullscreen::Windowed {
                            Fullscreen::Exclusive
                        } else {
                            Fullscreen::Windowed
                        };
                        settings_changed = true;
                        true
                    }
                    _ => false,
                };
                if handled {
                    continue;
                }
            }
            input.handle(&ev);
        }

        // --- Menús ----------------------------------------------------------
        if menu.is_open() {
            if let Some(n) = stick_nav.poll(input.menu_direction(), frame_start) {
                navs.push(n);
            }
            for n in navs {
                let action = menu.nav(n, &mut settings, &mut chapter, chapters.len());
                match action {
                    Action::None => {}
                    Action::Moved => audio.play(Sfx::MenuMove, 0.4),
                    Action::SettingsChanged => {
                        audio.play(Sfx::MenuMove, 0.4);
                        settings_changed = true;
                    }
                    Action::ChapterChanged => {
                        audio.play(Sfx::MenuMove, 0.4);
                        if let Ok(w) = load(chapter, world.tuning.clone()) {
                            world = w;
                        }
                    }
                    Action::Play => {
                        audio.play(Sfx::MenuSelect, 0.6);
                        if let Ok(w) = load(chapter, world.tuning.clone()) {
                            world = w;
                        }
                        recording.clear();
                    }
                    Action::Resume => audio.play(Sfx::MenuSelect, 0.5),
                    Action::RestartRoom => {
                        audio.play(Sfx::MenuSelect, 0.5);
                        world.restart();
                        recording.clear();
                    }
                    Action::ToTitle => {
                        audio.play(Sfx::MenuSelect, 0.5);
                        if let Ok(w) = load(chapter, world.tuning.clone()) {
                            world = w;
                        }
                    }
                    Action::Quit => break 'main,
                }
                if !menu.is_open() {
                    // Que el "aceptar" no se convierta en salto o golpe.
                    input.clear_latches();
                    break;
                }
            }
        } else {
            stick_nav.dir = input.menu_direction();
        }

        if settings_changed {
            settings.present = settings::apply_present(&canvas, settings.present);
            settings::apply_fullscreen(canvas.window(), settings.fullscreen);
            audio.set_volume(settings.volume_f());
            input.deadzone = settings.deadzone_f();
            input.strike_button = settings.strike_button;
            next_deadline_ns = now_ns();
            settings.save();
        }

        if let Some((t, warns)) = tuning_watch.poll(frame_start) {
            world.tuning = t;
            let msg = if warns.is_empty() {
                "tuning.cfg recargado".to_string()
            } else {
                format!("tuning.cfg: {}", warns.join("; "))
            };
            eprintln!("[crock] {msg}");
            message = Some((msg, frame_start));
        }

        // --- Simulación a paso fijo (parada con un menú abierto) ----------
        let now = now_ns();
        let frame_dt = (now - last_ns).min(100_000_000) as f32 / 1e9;
        if menu.is_open() {
            acc_ns = 0;
        } else {
            acc_ns += (now - last_ns).min(250_000_000);
        }
        last_ns = now;
        stats.ticks_this_frame = 0;
        while acc_ns >= DT_NS {
            let mut tick_input = input.take_tick_input();
            if let Some(r) = replay.pop_front() {
                tick_input = r;
            }
            if opts.autoplay {
                let t = world.tick;
                tick_input.x = if (t / 600).is_multiple_of(2) { 1 } else { -1 };
                tick_input.jump_pressed = t.is_multiple_of(120);
                tick_input.jump_held = t % 120 < 60;
                // Golpe en diagonal hacia abajo cada medio segundo.
                if t % 120 == 40 {
                    tick_input.strike_pressed = true;
                    tick_input.aim_x = tick_input.x;
                    tick_input.aim_y = 1;
                }
            }
            if opts.record.is_some() {
                recording.push(tick_input);
            }
            step(&mut world, &tick_input);
            acc_ns -= DT_NS;
            stats.ticks_this_frame += 1;
        }
        alpha = if stats.ticks_this_frame > 0 || frame == 0 {
            acc_ns as f32 / DT_NS as f32
        } else {
            alpha.max(acc_ns as f32 / DT_NS as f32)
        };

        // Efectos, sonido y vibración a partir de los eventos de la simulación.
        for ev in world.events.drain(..) {
            match ev {
                SimEvent::ChapterComplete => {
                    let msg = format!(
                        "{} completado - muertes: {}",
                        chapters[chapter].name, world.deaths
                    );
                    banner = Some((msg, frame_start));
                    world.deaths = 0;
                }
                SimEvent::Strike { .. } if settings.rumble => input.rumble(0.35, 0.6, 70),
                SimEvent::Death { .. } if settings.rumble => input.rumble(0.8, 0.4, 180),
                SimEvent::Land { speed } if settings.rumble && speed > 200.0 => {
                    input.rumble(0.25, 0.0, 50)
                }
                _ => {}
            }
            audio.on_event(&ev);
            fx.handle(&ev);
        }
        if !menu.is_open() {
            fx.update(frame_dt);
        }

        // --- Render -------------------------------------------------------
        let view = render::draw_world(
            &mut canvas,
            &world,
            alpha,
            &fx,
            &gfx,
            settings.overlay,
            settings.scaling,
        );
        if !menu.is_open() || menu.page() == Some(Page::Pause) {
            let hud = [
                format!(
                    "{} {}/{}",
                    world.map.name,
                    world.room + 1,
                    world.rooms.len()
                ),
                format!("muertes {}", world.deaths),
            ];
            let banner_text = banner
                .as_ref()
                .filter(|(_, t)| frame_start - t < 4_000_000_000)
                .map(|(m, _)| m.as_str());
            render::draw_hud(&mut canvas, &view, &hud, banner_text);
        }
        menu.draw(&mut canvas, &settings, &chapters[chapter].name);
        if settings.overlay {
            let (favg, fmax) = stats.frame_ms();
            let (wavg, wmax) = stats.work_ms();
            let p = &world.player;
            let mut lines = vec![
                format!("cROCK  sim {SIM_HZ} Hz  renderer {renderer}"),
                format!(
                    "F2 sincronia: {}  F3 limite: {}  F4 retardo: {} ms  F11 pantalla: {}",
                    settings.present.label(),
                    if cap == 0 {
                        "no".to_string()
                    } else {
                        format!("{cap}")
                    },
                    FRAME_DELAYS_MS[delay_idx],
                    settings.fullscreen.label()
                ),
                format!(
                    "frame {favg:5.2} ms (max {fmax:5.2}, {:4.0} fps)  cpu {wavg:4.2} ms (max {wmax:4.2})",
                    if favg > 0.0 { 1000.0 / favg } else { 0.0 }
                ),
                format!("ticks/frame {}  alpha {alpha:.2}", stats.ticks_this_frame),
                match stats.latency_ms() {
                    Some((l, a)) => format!("pulsacion->present {l:5.2} ms (media {a:5.2})"),
                    None => "pulsacion->present: pulsa salto o golpe".into(),
                },
                format!(
                    "pos {:4},{:4} vel {:6.1},{:6.1} {} coyote {:2} buf {:2}",
                    p.pos.0,
                    p.pos.1,
                    p.vel.0,
                    p.vel.1,
                    if p.on_ground { "suelo" } else { "aire " },
                    p.coyote,
                    p.jump_buffer
                ),
                format!(
                    "golpes {}  ventana {:2}  float {:2}  hitstop {:2}  {}",
                    p.strikes, p.strike_window, p.float, world.hitstop, chapters[chapter].name
                ),
            ];
            let pads = input.gamepad_names();
            lines.push(if pads.is_empty() {
                "mandos: ninguno".into()
            } else {
                format!("mandos: {}", pads.join(", "))
            });
            lines.push(
                "R reiniciar  F1 metricas  F5 recargar  F6 capitulo  F7/F8 sala  Esc pausa".into(),
            );
            if let Some((m, t)) = &message
                && frame_start - t < 3_000_000_000
            {
                lines.push(m.clone());
            }
            let target_ms = if cap > 0 {
                1000.0 / cap as f64
            } else if fmax > 0.0 {
                favg
            } else {
                16.67
            };
            render::draw_overlay(&mut canvas, &lines, &stats, target_ms);
        }

        frame += 1;
        let last_frame = opts.frames.is_some_and(|n| frame >= n);
        if last_frame && let Some(path) = &opts.screenshot {
            save_screenshot(&canvas, path);
        }

        let work_ns = now_ns() - frame_start;
        canvas.present();
        let presented = now_ns();

        // Latencia: sólo cuenta si la pulsación ya pasó por la simulación.
        if stats.ticks_this_frame > 0
            && let Some(ts) = input.pending_press_ts.take()
        {
            stats.push_latency(presented.saturating_sub(ts));
        }
        stats.push_frame(presented - last_present_ns, work_ns);
        last_present_ns = presented;

        if last_frame {
            let (favg, fmax) = stats.frame_ms();
            println!(
                "[crock] {frame} frames, {} ticks, frame medio {favg:.2} ms (max {fmax:.2}), renderer {renderer}",
                world.tick
            );
            break;
        }
    }
    if let Some(path) = &opts.record {
        std::fs::write(path, crock_sim::replay::encode(&recording))?;
        println!("[crock] {} ticks grabados en {path}", recording.len());
    }
    Ok(())
}
