//! Dibujo con SDL_Renderer: pixel art a doble resolución, cámara, HUD y
//! overlay de depuración. Sin texturas, cae a rectángulos de colores.

use std::ffi::CString;

use crock_sim::{Player, StrikeKind, TILE, Tile, World, ZoneKind, unit};
use sdl3::pixels::Color;
use sdl3::render::{FRect, WindowCanvas};
use sdl3::sys;

use crate::fx::{Fx, kind_color};
use crate::gfx::{ART, Gfx};
use crate::stats::{HISTORY, Stats};

/// Tamaño de la ventana de juego en unidades (40×23 tiles).
/// Lo que se ve de la sala (32×18 tiles, 16:9). Las salas más grandes se
/// recorren con la cámara.
pub const VIEW_W: f32 = 256.0;
pub const VIEW_H: f32 = 144.0;

/// Transformación de unidades de juego a píxeles (de la textura intermedia o
/// de la ventana): escala, desplazamiento y cámara.
#[derive(Clone, Copy)]
pub struct View {
    pub scale: f32,
    pub ox: f32,
    pub oy: f32,
    pub cam_x: f32,
    pub cam_y: f32,
}

impl View {
    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32) -> FRect {
        // Redondeo a píxel de salida: el movimiento interpolado se ve suave
        // a cualquier escala sin emborronar los bordes.
        let x0 = (self.ox + (x - self.cam_x) * self.scale).round();
        let y0 = (self.oy + (y - self.cam_y) * self.scale).round();
        let x1 = (self.ox + (x + w - self.cam_x) * self.scale).round();
        let y1 = (self.oy + (y + h - self.cam_y) * self.scale).round();
        FRect::new(x0, y0, x1 - x0, y1 - y0)
    }
}

pub fn output_size(canvas: &WindowCanvas) -> (i32, i32) {
    let (mut w, mut h) = (0, 0);
    unsafe { sys::render::SDL_GetCurrentRenderOutputSize(canvas.raw(), &mut w, &mut h) };
    (w, h)
}

/// Cámara centrada en el jugador (interpolado), sin salirse de la sala; si
/// la sala es más pequeña que la pantalla, centrada. Ajustada a píxel de arte.
fn camera(world: &World, alpha: f32) -> (f32, f32) {
    let (x, y) = world.player.lerp_pos(alpha);
    let (cx, cy) = (x + Player::W as f32 / 2.0, y + Player::H as f32 / 2.0);
    let axis = |c: f32, size: f32, view: f32| {
        if size <= view {
            (size - view) / 2.0
        } else {
            (c - view / 2.0).clamp(0.0, size - view)
        }
    };
    let mw = world.map.width_px() as f32;
    let mh = world.map.height_px() as f32;
    let snap = |v: f32| (v * ART).round() / ART;
    (snap(axis(cx, mw, VIEW_W)), snap(axis(cy, mh, VIEW_H)))
}

/// Cómo se escala la imagen de juego a la ventana.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scaling {
    /// Llenar la ventana (escala no entera, con filtro de pixel art nítido).
    Fill,
    /// Sólo múltiplos enteros (píxeles perfectos, con bandas).
    Integer,
}

/// Dibuja el mundo en la textura intermedia (resolución nativa del arte) y
/// la escala a la ventana. Devuelve la vista en coordenadas de ventana, para
/// el HUD.
pub fn draw_world(
    canvas: &mut WindowCanvas,
    world: &World,
    alpha: f32,
    fx: &Fx,
    gfx: &Gfx,
    debug: bool,
    scaling: Scaling,
) -> View {
    let (cam_x, cam_y) = camera(world, alpha);
    let mut view = View {
        scale: ART,
        ox: 0.0,
        oy: 0.0,
        cam_x,
        cam_y,
    };

    // Temblor: sobre todo en la dirección del golpe, decayendo.
    if world.shake > 0 {
        let k =
            world.shake as f32 / crock_sim::secs_to_ticks(world.tuning.shake_time).max(1) as f32;
        let n = ((world.tick.wrapping_mul(2_654_435_761) >> 7) % 1000) as f32 / 500.0 - 1.0;
        let (dx, dy) = unit(world.shake_dir);
        let amp = 2.0 * k * view.scale;
        view.ox += ((dx * n + dy * 0.3 * n) * amp).round();
        view.oy += ((dy * n - dx * 0.3 * n) * amp).round();
    }

    unsafe { sys::render::SDL_SetRenderTarget(canvas.raw(), gfx.target) };
    canvas.set_draw_color(Color::RGB(24, 20, 30));
    canvas.clear();
    draw_tiles(canvas, world, &view, gfx);
    draw_zones(canvas, world, &view, gfx);
    draw_objects(canvas, world, &view, gfx);
    draw_player(canvas, world, alpha, &view, gfx, debug);
    fx.draw(canvas, &view);
    unsafe { sys::render::SDL_SetRenderTarget(canvas.raw(), std::ptr::null_mut()) };

    // Escalar a la ventana.
    let (ow, oh) = output_size(canvas);
    let (tw, th) = (VIEW_W * ART, VIEW_H * ART);
    let mut s = (ow as f32 / tw).min(oh as f32 / th);
    if scaling == Scaling::Integer && s >= 1.0 {
        s = s.floor();
    }
    let (dw, dh) = ((tw * s).round(), (th * s).round());
    let dst = sys::rect::SDL_FRect {
        x: ((ow as f32 - dw) / 2.0).floor(),
        y: ((oh as f32 - dh) / 2.0).floor(),
        w: dw,
        h: dh,
    };
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    unsafe { sys::render::SDL_RenderTexture(canvas.raw(), gfx.target, std::ptr::null(), &dst) };
    View {
        scale: s * ART,
        ox: dst.x,
        oy: dst.y,
        cam_x,
        cam_y,
    }
}

fn visible_range(view: &View, w: i32, h: i32) -> (i32, i32, i32, i32) {
    let tx0 = ((view.cam_x / TILE as f32).floor() as i32 - 1).max(0);
    let ty0 = ((view.cam_y / TILE as f32).floor() as i32 - 1).max(0);
    let tx1 = (((view.cam_x + VIEW_W) / TILE as f32).ceil() as i32 + 1).min(w);
    let ty1 = (((view.cam_y + VIEW_H) / TILE as f32).ceil() as i32 + 1).min(h);
    (tx0, ty0, tx1, ty1)
}

fn draw_tiles(canvas: &mut WindowCanvas, world: &World, view: &View, gfx: &Gfx) {
    let map = &world.map;
    let t = TILE as f32;
    let (tx0, ty0, tx1, ty1) = visible_range(view, map.w, map.h);
    if let Some(sheet) = &gfx.cueva {
        for layer in &map.gfx {
            for ty in ty0..ty1 {
                for tx in tx0..tx1 {
                    let id = layer[(ty * map.w + tx) as usize];
                    if id > 0 {
                        sheet.draw(
                            canvas,
                            id as u32 - 1,
                            view.rect(tx as f32 * t, ty as f32 * t, t, t),
                            false,
                        );
                    }
                }
            }
        }
        return;
    }
    // Sin texturas: rectángulos.
    for ty in ty0..ty1 {
        for tx in tx0..tx1 {
            let (x, y) = (tx as f32 * t, ty as f32 * t);
            match map.tile(tx, ty) {
                Tile::Empty => {}
                Tile::Solid => {
                    canvas.set_draw_color(Color::RGB(92, 80, 70));
                    let _ = canvas.fill_rect(view.rect(x, y, t, t));
                }
                Tile::Spike(d) => {
                    let (sx, sy, sw, sh) = crock_sim::TileMap::spike_box(tx, ty, d);
                    canvas.set_draw_color(Color::RGB(230, 220, 196));
                    let _ = canvas.fill_rect(view.rect(sx as f32, sy as f32, sw as f32, sh as f32));
                }
            }
        }
    }
}

fn draw_zones(canvas: &mut WindowCanvas, world: &World, view: &View, gfx: &Gfx) {
    let t = TILE as f32;
    let pulse = ((world.tick as f32 / 40.0).sin() * 0.5 + 0.5) * 0.5 + 0.5;
    for z in &world.map.zones {
        let cells_x = (z.w + TILE - 1) / TILE;
        let cells_y = (z.h + TILE - 1) / TILE;
        for cy in 0..cells_y {
            for cx in 0..cells_x {
                let (x, y) = ((z.x + cx * TILE) as f32, (z.y + cy * TILE) as f32);
                match (z.kind, &gfx.objetos) {
                    (ZoneKind::Fire, Some(s)) => {
                        // Parpadeo: alterna el volteo por celda y tiempo.
                        let flip = ((world.tick / 18) as i32 + cx) % 2 == 0;
                        s.draw(canvas, 7, view.rect(x, y, t, t), flip);
                    }
                    (ZoneKind::Exit, Some(s)) => {
                        let left = z.x < world.map.width_px() / 2;
                        s.set_alpha((pulse * 255.0) as u8);
                        s.draw(canvas, 8, view.rect(x, y, t, t), left);
                        s.set_alpha(255);
                    }
                    (kind, None) => {
                        canvas.set_draw_color(match kind {
                            ZoneKind::Fire => Color::RGB(220, 90, 40),
                            ZoneKind::Exit => Color::RGB(240, 220, 140),
                        });
                        let _ = canvas.draw_rect(view.rect(x, y, t, t));
                    }
                }
            }
        }
    }
}

fn sprite_index(kind: StrikeKind) -> u32 {
    match kind {
        StrikeKind::Rock => 1,
        StrikeKind::Fragile => 2,
        StrikeKind::Bone => 3,
        StrikeKind::Moss => 4,
        StrikeKind::Beetle => 5,
        StrikeKind::Bat => 6,
    }
}

fn draw_objects(canvas: &mut WindowCanvas, world: &World, view: &View, gfx: &Gfx) {
    for o in &world.objects {
        let (x0, y0, x1, y1) = o.bounds();
        if o.dead {
            continue;
        }
        let Some(s) = &gfx.objetos else {
            // Sin texturas.
            let c = kind_color(o.kind);
            canvas.set_draw_color(Color::RGB(c.0, c.1, c.2));
            let r = view.rect(x0, y0, x1 - x0, y1 - y0);
            let _ = if o.broken > 0 {
                canvas.draw_rect(r)
            } else {
                canvas.fill_rect(r)
            };
            continue;
        };
        if o.broken > 0 {
            // Hueco donde reaparecerá la roca frágil.
            canvas.set_draw_color(Color::RGB(70, 60, 56));
            let _ = canvas.draw_rect(view.rect(x0, y0, x1 - x0, y1 - y0));
            continue;
        }
        let (cx, _) = o.center;
        // Sprite de 8×8 unidades: centrado, salvo el escarabajo (apoyado).
        let (sx, sy) = match o.kind {
            StrikeKind::Beetle => (cx - 4.0, y1 - 8.0),
            StrikeKind::Bat => (cx - 4.0, o.center.1 - 4.0 + ((o.age / 20) % 2) as f32 * 0.5),
            _ => (cx - 4.0, o.center.1 - 4.0),
        };
        if o.kind == StrikeKind::Bone {
            // Cuerda hasta el techo.
            canvas.set_draw_color(Color::RGB(110, 90, 70));
            let _ = canvas.fill_rect(view.rect(cx - 0.25, y0 - 6.0, 0.5, 6.0));
        }
        s.set_alpha(if o.cooldown > 0 { 120 } else { 255 });
        let dst = view.rect(sx, sy, 8.0, 8.0);
        if o.kind == StrikeKind::Beetle && o.stun > 0 {
            // Aturdido: patas arriba.
            s.draw_rotated(canvas, 5, dst, 180.0, (dst.w / 2.0, dst.h / 2.0), false);
        } else {
            s.draw(
                canvas,
                sprite_index(o.kind),
                dst,
                o.kind == StrikeKind::Beetle && o.dir < 0,
            );
        }
        s.set_alpha(255);
    }
}

fn draw_player(
    canvas: &mut WindowCanvas,
    world: &World,
    alpha: f32,
    view: &View,
    gfx: &Gfx,
    debug: bool,
) {
    let p = &world.player;
    let (x, y) = p.lerp_pos(alpha);
    let (w, h) = (Player::W as f32, Player::H as f32);
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let facing_left = p.facing < 0;

    if let Some(s) = &gfx.crock {
        let frame = if !p.on_ground {
            if p.vel.1 < 0.0 { 3 } else { 4 }
        } else if p.vel.0.abs() > 10.0 {
            1 + ((p.pos.0.unsigned_abs() / 6) % 2)
        } else {
            0
        };
        if p.burning {
            s.set_tint(255, 150, 120);
        } else if p.strikes == 0 {
            // Sin golpes: apagado, como pista de que hay que recargar.
            s.set_tint(160, 160, 200);
        }
        // Celda de 16×16 unidades (32 px de arte) con los pies abajo y centrada.
        s.draw(
            canvas,
            frame,
            view.rect(cx - 8.0, y + h - 16.0, 16.0, 16.0),
            facing_left,
        );
        s.set_tint(255, 255, 255);
    } else {
        canvas.set_draw_color(if p.strikes == 0 {
            Color::RGB(150, 130, 150)
        } else {
            Color::RGB(214, 150, 92)
        });
        let _ = canvas.fill_rect(view.rect(x, y, w, h));
    }

    // La porra: girada hacia el golpe mientras dura el swing; si no, en la
    // mano, inclinada hacia delante.
    let f = if facing_left { -1.0 } else { 1.0 };
    // La mano del sprite (puño) queda delante, a media altura.
    let (hx, hy) = (cx + f * 4.5, cy - 0.5);
    let (dir, swinging) = if p.swing > 0 {
        (unit(p.strike_dir), true)
    } else {
        ((f * 0.5, 0.87), false)
    };
    if let Some(s) = &gfx.porra {
        let angle = (dir.1.atan2(dir.0) as f64).to_degrees();
        let (len, thick) = (8.0, 3.5);
        let dst = view.rect(hx, hy - thick / 2.0, len, thick);
        let flip_v = dir.0 < 0.0;
        s.draw_rotated(canvas, 0, dst, angle, (0.0, dst.h / 2.0), flip_v);
    } else {
        canvas.set_draw_color(Color::RGB(140, 96, 52));
        let n = if swinging { 5 } else { 3 };
        for i in 0..n {
            let d = 1.0 + i as f32 * 1.6;
            let _ =
                canvas.fill_rect(view.rect(hx + dir.0 * d - 1.0, hy + dir.1 * d - 1.0, 2.0, 2.0));
        }
    }

    if debug {
        canvas.set_draw_color(Color::RGB(80, 220, 255));
        let _ = canvas.draw_rect(view.rect(x, y, w, h));
        if p.strike_window > 0 {
            let (x0, y0, x1, y1) = p.strike_box(p.strike_dir, &world.tuning);
            canvas.set_draw_color(Color::RGB(255, 80, 80));
            let _ = canvas.draw_rect(view.rect(x0, y0, x1 - x0, y1 - y0));
        }
        for o in world.objects.iter().filter(|o| o.hostile()) {
            let (x0, y0, x1, y1) = o.bounds();
            canvas.set_draw_color(Color::RGB(255, 160, 60));
            let _ = canvas.draw_rect(view.rect(x0 + 1.0, y0 + 1.0, x1 - x0 - 2.0, y1 - y0 - 2.0));
        }
    }
}

pub fn text(canvas: &mut WindowCanvas, x: f32, y: f32, s: &str) {
    if let Ok(c) = CString::new(s) {
        unsafe { sys::render::SDL_RenderDebugText(canvas.raw(), x, y, c.as_ptr()) };
    }
}

/// HUD mínimo arriba a la derecha de la pantalla de juego (texto de 8 px a
/// la escala del arte), y un mensaje centrado opcional.
pub fn draw_hud(canvas: &mut WindowCanvas, view: &View, lines: &[String], banner: Option<&str>) {
    let s = (view.scale / ART).max(1.0).floor();
    unsafe { sys::render::SDL_SetRenderScale(canvas.raw(), s, s) };
    let right = (view.ox + VIEW_W * view.scale) / s;
    let top = view.oy / s;
    canvas.set_blend_mode(sdl3::render::BlendMode::Blend);
    for (i, l) in lines.iter().enumerate() {
        let x = right - l.chars().count() as f32 * 8.0 - 6.0;
        let y = top + 6.0 + i as f32 * 11.0;
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 160));
        let _ = canvas.fill_rect(FRect::new(
            x - 3.0,
            y - 2.0,
            l.chars().count() as f32 * 8.0 + 6.0,
            12.0,
        ));
        canvas.set_draw_color(Color::RGB(240, 230, 210));
        text(canvas, x, y, l);
    }
    if let Some(b) = banner {
        let cx = (view.ox + VIEW_W * view.scale / 2.0) / s;
        let cy = (view.oy + VIEW_H * view.scale / 3.0) / s;
        let w = b.chars().count() as f32 * 8.0;
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 190));
        let _ = canvas.fill_rect(FRect::new(cx - w / 2.0 - 8.0, cy - 8.0, w + 16.0, 24.0));
        canvas.set_draw_color(Color::RGB(255, 230, 160));
        text(canvas, cx - w / 2.0, cy, b);
    }
    canvas.set_blend_mode(sdl3::render::BlendMode::None);
    unsafe { sys::render::SDL_SetRenderScale(canvas.raw(), 1.0, 1.0) };
}

/// Overlay de métricas. `lines` se dibuja arriba a la izquierda y debajo la
/// gráfica de tiempos de frame (blanco = intervalo, verde = trabajo de CPU).
pub fn draw_overlay(canvas: &mut WindowCanvas, lines: &[String], stats: &Stats, target_ms: f64) {
    let (_, oh) = output_size(canvas);
    let s = (oh as f32 / 540.0).floor().max(1.0);
    unsafe { sys::render::SDL_SetRenderScale(canvas.raw(), s, s) };

    let width = lines
        .iter()
        .map(|l| l.len())
        .max()
        .unwrap_or(0)
        .max(HISTORY / 8) as f32
        * 8.0
        + 8.0;
    let graph_h = 60.0;
    let height = lines.len() as f32 * 10.0 + graph_h + 12.0;
    canvas.set_blend_mode(sdl3::render::BlendMode::Blend);
    canvas.set_draw_color(Color::RGBA(0, 0, 0, 180));
    let _ = canvas.fill_rect(FRect::new(
        0.0,
        0.0,
        width.max(HISTORY as f32 + 8.0),
        height,
    ));

    canvas.set_draw_color(Color::RGB(230, 230, 230));
    for (i, l) in lines.iter().enumerate() {
        text(canvas, 4.0, 4.0 + i as f32 * 10.0, l);
    }

    // Gráfica: 1 px por frame, escala 0..=2×objetivo.
    let gy = lines.len() as f32 * 10.0 + 8.0;
    let max_ms = (target_ms * 2.0).max(1.0);
    let bar = |ns: u64| ((ns as f64 / 1e6 / max_ms).min(1.0) * graph_h as f64) as f32;
    for i in 0..HISTORY {
        let idx = (stats.head + i) % HISTORY;
        let x = 4.0 + i as f32;
        let fh = bar(stats.frame_ns[idx]);
        canvas.set_draw_color(Color::RGB(200, 200, 200));
        let _ = canvas.fill_rect(FRect::new(x, gy + graph_h - fh, 1.0, fh));
        let wh = bar(stats.work_ns[idx]);
        canvas.set_draw_color(Color::RGB(90, 220, 120));
        let _ = canvas.fill_rect(FRect::new(x, gy + graph_h - wh, 1.0, wh));
    }
    // Línea del objetivo.
    canvas.set_draw_color(Color::RGB(230, 80, 80));
    let ty = gy + graph_h - bar((target_ms * 1e6) as u64);
    let _ = canvas.fill_rect(FRect::new(4.0, ty, HISTORY as f32, 1.0));

    canvas.set_blend_mode(sdl3::render::BlendMode::None);
    unsafe { sys::render::SDL_SetRenderScale(canvas.raw(), 1.0, 1.0) };
}
