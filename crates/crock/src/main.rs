//! cROCK — capa de plataforma sobre SDL3.
//!
//! Camino crítico en un solo hilo: eventos → simulación a paso fijo → render
//! interpolado → present. Nada más se ejecuta entre leer la entrada y
//! simular.

mod assets;
mod fx;
mod gfx;
mod input;
mod render;
mod stats;

use std::time::Duration;

use crock_sim::{DT_NS, SIM_HZ, World, step};
use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::{Mod, Scancode};
use sdl3::sys;

use crate::input::InputState;
use crate::stats::Stats;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PresentMode {
    VSync,
    Adaptive,
    Immediate,
}

impl PresentMode {
    fn sdl_value(self) -> i32 {
        match self {
            Self::VSync => 1,
            Self::Adaptive => sys::render::SDL_RENDERER_VSYNC_ADAPTIVE,
            Self::Immediate => sys::render::SDL_RENDERER_VSYNC_DISABLED,
        }
    }
    fn next(self) -> Self {
        match self {
            Self::VSync => Self::Adaptive,
            Self::Adaptive => Self::Immediate,
            Self::Immediate => Self::VSync,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::VSync => "vsync",
            Self::Adaptive => "adaptativo",
            Self::Immediate => "inmediato (tearing)",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        match s {
            "on" | "vsync" => Some(Self::VSync),
            "adaptive" => Some(Self::Adaptive),
            "off" | "immediate" => Some(Self::Immediate),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fullscreen {
    Windowed,
    Borderless,
    Exclusive,
}

impl Fullscreen {
    fn next(self) -> Self {
        match self {
            Self::Windowed => Self::Borderless,
            Self::Borderless => Self::Exclusive,
            Self::Exclusive => Self::Windowed,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Windowed => "ventana",
            Self::Borderless => "sin bordes",
            Self::Exclusive => "exclusiva",
        }
    }
}

const FPS_CAPS: [u32; 6] = [0, 60, 120, 144, 240, 360];
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
    present: PresentMode,
    fps_cap: u32,
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
    /// Empezar con el overlay de métricas visible.
    overlay: bool,
    /// Reproducir entradas grabadas (una por tick).
    replay: Option<String>,
    /// Grabar las entradas desde el último reinicio al salir.
    record: Option<String>,
}

fn parse_args() -> Options {
    let mut o = Options {
        present: PresentMode::VSync,
        fps_cap: 0,
        fullscreen: false,
        frames: None,
        screenshot: None,
        autoplay: false,
        chapter: String::new(),
        room: 1,
        overlay: false,
        replay: None,
        record: None,
    };
    for a in std::env::args().skip(1) {
        let (k, v) = a.split_once('=').unwrap_or((a.as_str(), ""));
        match k {
            "--vsync" => o.present = PresentMode::parse(v).unwrap_or(o.present),
            "--fps" => o.fps_cap = v.parse().unwrap_or(0),
            "--fullscreen" => o.fullscreen = true,
            "--frames" => o.frames = v.parse().ok(),
            "--screenshot" => o.screenshot = Some(v.to_string()),
            "--autoplay" => o.autoplay = true,
            "--replay" => o.replay = Some(v.to_string()),
            "--record" => o.record = Some(v.to_string()),
            "--chapter" => o.chapter = v.to_string(),
            "--room" => o.room = v.parse().unwrap_or(1),
            "--overlay" => o.overlay = true,
            "-h" | "--help" => {
                println!(
                    "crock [--vsync=on|adaptive|off] [--fps=N] [--fullscreen]\n\
                     \x20     [--chapter=N|nombre] [--room=N] [--overlay]\n\
                     \x20     [--replay=f] [--record=f]\n\
                     \x20     [--frames=N] [--screenshot=out.bmp] [--autoplay]"
                );
                std::process::exit(0);
            }
            _ => eprintln!("[crock] argumento desconocido: {a}"),
        }
    }
    o
}

fn set_present_mode(canvas: &sdl3::render::WindowCanvas, mode: PresentMode) -> PresentMode {
    if unsafe { sys::render::SDL_SetRenderVSync(canvas.raw(), mode.sdl_value()) } {
        return mode;
    }
    // El adaptativo no está en todos los backends: caer a vsync normal.
    eprintln!("[crock] modo de presentación {:?} no soportado", mode);
    unsafe { sys::render::SDL_SetRenderVSync(canvas.raw(), 1) };
    PresentMode::VSync
}

fn set_fullscreen(window: &sdl3::video::Window, mode: Fullscreen) {
    let w = window.raw();
    unsafe {
        match mode {
            Fullscreen::Windowed => {
                sys::video::SDL_SetWindowFullscreen(w, false);
            }
            Fullscreen::Borderless => {
                sys::video::SDL_SetWindowFullscreenMode(w, std::ptr::null());
                sys::video::SDL_SetWindowFullscreen(w, true);
            }
            Fullscreen::Exclusive => {
                // Modo nativo del monitor en exclusiva: permite direct scanout
                // y saltarse el compositor donde el sistema lo soporte.
                let display = sys::video::SDL_GetDisplayForWindow(w);
                let dm = sys::video::SDL_GetDesktopDisplayMode(display);
                sys::video::SDL_SetWindowFullscreenMode(w, dm);
                sys::video::SDL_SetWindowFullscreen(w, true);
            }
        }
    }
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = parse_args();

    let sdl = sdl3::init()?;
    let video = sdl.video()?;
    let gamepads = sdl.gamepad()?;

    let window = video
        .window("cROCK", 1280, 736)
        .resizable()
        .high_pixel_density()
        .build()?;
    let mut canvas = window.into_canvas();
    let mut present = set_present_mode(&canvas, opts.present);
    let mut fullscreen = Fullscreen::Windowed;
    if opts.fullscreen {
        fullscreen = Fullscreen::Exclusive;
        set_fullscreen(canvas.window(), fullscreen);
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
    let mut world = World::with_rooms(
        assets::load_chapter(&chapters[chapter])?,
        Default::default(),
    );
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
    let mut events = sdl.event_pump()?;
    let mut stats = Stats::new();
    let mut fx = fx::Fx::new();
    let mut replay: std::collections::VecDeque<crock_sim::Input> = match &opts.replay {
        Some(path) => crock_sim::replay::decode(&std::fs::read_to_string(path)?)?.into(),
        None => Default::default(),
    };
    let mut recording: Vec<crock_sim::Input> = Vec::new();

    let mut show_overlay = opts.overlay;
    let mut cap_idx = FPS_CAPS
        .iter()
        .position(|&c| c == opts.fps_cap)
        .unwrap_or(0);
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
        let cap = FPS_CAPS[cap_idx];
        if cap > 0 {
            let period = 1_000_000_000 / cap as u64;
            wait_until(next_deadline_ns);
            next_deadline_ns = (next_deadline_ns + period).max(now_ns());
        }

        let frame_start = now_ns();

        // --- Entrada ------------------------------------------------------
        for ev in events.poll_iter() {
            match ev {
                Event::Quit { .. } => break 'main,
                Event::Window {
                    win_event: WindowEvent::CloseRequested,
                    ..
                } => break 'main,
                Event::KeyDown {
                    scancode: Some(sc),
                    repeat: false,
                    keymod,
                    ..
                } => {
                    let handled = match sc {
                        Scancode::Escape => break 'main,
                        Scancode::R => {
                            world.restart();
                            recording.clear();
                            true
                        }
                        Scancode::F1 => {
                            show_overlay = !show_overlay;
                            true
                        }
                        Scancode::F2 => {
                            present = set_present_mode(&canvas, present.next());
                            true
                        }
                        Scancode::F3 => {
                            cap_idx = (cap_idx + 1) % FPS_CAPS.len();
                            next_deadline_ns = now_ns();
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
                            match assets::load_chapter(&chapters[chapter]) {
                                Ok(rooms) => {
                                    let (tuning, deaths) = (world.tuning.clone(), world.deaths);
                                    world = World::with_rooms(rooms, tuning);
                                    world.deaths = if sc == Scancode::F5 { deaths } else { 0 };
                                    world.goto_room(room);
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
                            fullscreen = fullscreen.next();
                            set_fullscreen(canvas.window(), fullscreen);
                            true
                        }
                        Scancode::Return if keymod.intersects(Mod::LALTMOD | Mod::RALTMOD) => {
                            fullscreen = if fullscreen == Fullscreen::Windowed {
                                Fullscreen::Exclusive
                            } else {
                                Fullscreen::Windowed
                            };
                            set_fullscreen(canvas.window(), fullscreen);
                            true
                        }
                        _ => false,
                    };
                    if !handled {
                        input.handle(&ev);
                    }
                }
                ref other => {
                    input.handle(other);
                }
            }
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

        // --- Simulación a paso fijo --------------------------------------
        let now = now_ns();
        let frame_dt = (now - last_ns).min(100_000_000) as f32 / 1e9;
        acc_ns += (now - last_ns).min(250_000_000);
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

        // Efectos visuales a partir de los eventos de la simulación.
        for ev in world.events.drain(..) {
            if matches!(ev, crock_sim::SimEvent::ChapterComplete) {
                let msg = format!(
                    "{} completado - muertes: {}",
                    chapters[chapter].name, world.deaths
                );
                banner = Some((msg, frame_start));
                world.deaths = 0;
            }
            fx.handle(&ev);
        }
        fx.update(frame_dt);

        // --- Render -------------------------------------------------------
        let view = render::draw_world(&mut canvas, &world, alpha, &fx, &gfx, show_overlay);
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
        if show_overlay {
            let (favg, fmax) = stats.frame_ms();
            let (wavg, wmax) = stats.work_ms();
            let p = &world.player;
            let mut lines = vec![
                format!("cROCK  sim {SIM_HZ} Hz  renderer {renderer}"),
                format!(
                    "F2 present: {}  F3 cap: {}  F4 delay: {} ms  F11 pantalla: {}",
                    present.label(),
                    if cap == 0 {
                        "no".to_string()
                    } else {
                        format!("{cap}")
                    },
                    FRAME_DELAYS_MS[delay_idx],
                    fullscreen.label()
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
                    "golpes {}  ventana {:2}  float {:2}  hitstop {:2}  sala {}",
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
                "R reiniciar  F1 overlay  F5 recargar  F6 capitulo  F7/F8 sala  Esc salir".into(),
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
