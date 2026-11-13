//! Dibujo con SDL_Renderer: rectángulos de prototipo + overlay de depuración.

use std::ffi::CString;

use crock_sim::{Player, TILE, Tile, World};
use sdl3::pixels::Color;
use sdl3::render::{FRect, WindowCanvas};
use sdl3::sys;

use crate::stats::{HISTORY, Stats};

/// Transformación de píxeles de juego a píxeles de salida (escala entera,
/// centrado con bandas negras).
#[derive(Clone, Copy)]
pub struct View {
    pub scale: f32,
    pub ox: f32,
    pub oy: f32,
}

impl View {
    pub fn fit(out_w: i32, out_h: i32, game_w: i32, game_h: i32) -> Self {
        let s = (out_w as f32 / game_w as f32).min(out_h as f32 / game_h as f32);
        let scale = if s >= 1.0 { s.floor() } else { s };
        Self {
            scale,
            ox: ((out_w as f32 - game_w as f32 * scale) / 2.0).floor(),
            oy: ((out_h as f32 - game_h as f32 * scale) / 2.0).floor(),
        }
    }

    fn rect(&self, x: f32, y: f32, w: f32, h: f32) -> FRect {
        // Redondeo a píxel de salida: el movimiento interpolado se ve suave
        // a cualquier escala sin emborronar los bordes.
        FRect::new(
            (self.ox + x * self.scale).round(),
            (self.oy + y * self.scale).round(),
            (w * self.scale).round(),
            (h * self.scale).round(),
        )
    }
}

pub fn output_size(canvas: &WindowCanvas) -> (i32, i32) {
    let (mut w, mut h) = (0, 0);
    unsafe { sys::render::SDL_GetCurrentRenderOutputSize(canvas.raw(), &mut w, &mut h) };
    (w, h)
}

pub fn draw_world(canvas: &mut WindowCanvas, world: &World, alpha: f32) {
    let (ow, oh) = output_size(canvas);
    let map = &world.map;
    let view = View::fit(ow, oh, map.width_px(), map.height_px());

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();

    canvas.set_draw_color(Color::RGB(28, 24, 34));
    let _ = canvas.fill_rect(view.rect(0.0, 0.0, map.width_px() as f32, map.height_px() as f32));

    let mut rects = Vec::with_capacity(256);
    for ty in 0..map.h {
        for tx in 0..map.w {
            if map.tile(tx, ty) == Tile::Solid {
                let t = TILE as f32;
                rects.push(view.rect(tx as f32 * t, ty as f32 * t, t, t));
            }
        }
    }
    canvas.set_draw_color(Color::RGB(92, 80, 70));
    let _ = canvas.fill_rects(&rects);

    // Jugador (interpolado entre los dos últimos ticks).
    let p = &world.player;
    let (x, y) = p.lerp_pos(alpha);
    let (w, h) = (Player::W as f32, Player::H as f32);
    canvas.set_draw_color(if p.on_ground {
        Color::RGB(214, 150, 92)
    } else {
        Color::RGB(236, 180, 110)
    });
    let _ = canvas.fill_rect(view.rect(x, y, w, h));
    // La porra, apuntando hacia donde mira.
    canvas.set_draw_color(Color::RGB(140, 96, 52));
    let club_x = if p.facing > 0 { x + w } else { x - 3.0 };
    let _ = canvas.fill_rect(view.rect(club_x, y + 3.0, 3.0, 2.0));
}

pub fn text(canvas: &mut WindowCanvas, x: f32, y: f32, s: &str) {
    if let Ok(c) = CString::new(s) {
        unsafe { sys::render::SDL_RenderDebugText(canvas.raw(), x, y, c.as_ptr()) };
    }
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
