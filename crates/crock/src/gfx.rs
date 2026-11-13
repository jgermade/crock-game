//! Texturas del pixel art (PNG cargados con SDL 3.4, sin SDL_image).
//!
//! El arte va a 16 px por tile (doble resolución que la física). Si algún
//! PNG falta, el render cae a rectángulos de colores.

use std::ffi::CString;
use std::path::Path;

use sdl3::render::{FRect, WindowCanvas};
use sdl3::sys;
use sys::rect::{SDL_FPoint, SDL_FRect};
use sys::render::SDL_Texture;

/// Píxeles de arte por unidad de juego (tiles de 24 px).
pub const ART: f32 = 3.0;

/// Celda de la hoja de Crock (px de arte; ver tools/import_crock.py).
pub const CROCK_CELL: f32 = 56.0;

/// Celdas de cada animación en crock.png (mismo orden que import_crock.py).
pub mod crock_frames {
    pub const IDLE: (u32, u32) = (0, 5);
    pub const RUN: (u32, u32) = (5, 12);
    pub const JUMP: u32 = 17;
    pub const FALL: (u32, u32) = (18, 2);
    pub const STRIKE_SIDE: u32 = 20;
    pub const STRIKE_UP: u32 = 21;
    pub const STRIKE_DOWN: u32 = 22;
}

pub struct Sheet {
    tex: *mut SDL_Texture,
    pub cell_w: f32,
    pub cell_h: f32,
}

impl Drop for Sheet {
    fn drop(&mut self) {
        unsafe { sys::render::SDL_DestroyTexture(self.tex) };
    }
}

pub struct Gfx {
    /// Textura intermedia donde se dibuja el mundo a resolución de arte.
    pub target: *mut SDL_Texture,
    pub cueva: Option<Sheet>,
    pub objetos: Option<Sheet>,
    pub crock: Option<Sheet>,
}

fn load(canvas: &WindowCanvas, path: &Path, cell_w: f32, cell_h: f32) -> Option<Sheet> {
    let c = CString::new(path.to_string_lossy().as_bytes()).ok()?;
    unsafe {
        let surf = sys::surface::SDL_LoadPNG(c.as_ptr());
        if surf.is_null() {
            eprintln!("[crock] no se pudo cargar {}", path.display());
            return None;
        }
        let tex = sys::render::SDL_CreateTextureFromSurface(canvas.raw(), surf);
        sys::surface::SDL_DestroySurface(surf);
        if tex.is_null() {
            return None;
        }
        sys::render::SDL_SetTextureScaleMode(tex, sys::surface::SDL_SCALEMODE_NEAREST);
        Some(Sheet {
            tex,
            cell_w,
            cell_h,
        })
    }
}

impl Gfx {
    pub fn load(canvas: &WindowCanvas, assets: &Path) -> Self {
        let g = assets.join("gfx");
        let (w, h) = (
            (crate::render::VIEW_W * ART) as i32,
            (crate::render::VIEW_H * ART) as i32,
        );
        let target = unsafe {
            let t = sys::render::SDL_CreateTexture(
                canvas.raw(),
                sys::pixels::SDL_PIXELFORMAT_RGBA8888,
                sys::render::SDL_TEXTUREACCESS_TARGET,
                w,
                h,
            );
            assert!(!t.is_null(), "no se pudo crear la textura de render");
            // Escalado no entero nítido (SDL 3.4).
            sys::render::SDL_SetTextureScaleMode(t, sys::surface::SDL_SCALEMODE_PIXELART);
            t
        };
        Self {
            target,
            cueva: load(canvas, &g.join("cueva.png"), 24.0, 24.0),
            objetos: load(canvas, &g.join("objetos.png"), 24.0, 24.0),
            crock: load(canvas, &g.join("crock.png"), CROCK_CELL, CROCK_CELL),
        }
    }
}

fn frect(r: FRect) -> SDL_FRect {
    SDL_FRect {
        x: r.x,
        y: r.y,
        w: r.w,
        h: r.h,
    }
}

impl Drop for Gfx {
    fn drop(&mut self) {
        unsafe { sys::render::SDL_DestroyTexture(self.target) };
    }
}

impl Sheet {
    /// Dibuja la celda `index` en `dst` (píxeles de salida).
    pub fn draw(&self, canvas: &mut WindowCanvas, index: u32, dst: FRect, flip: bool) {
        let src = SDL_FRect {
            x: index as f32 * self.cell_w,
            y: 0.0,
            w: self.cell_w,
            h: self.cell_h,
        };
        let d = frect(dst);
        let f = if flip {
            sys::surface::SDL_FLIP_HORIZONTAL
        } else {
            sys::surface::SDL_FLIP_NONE
        };
        unsafe {
            sys::render::SDL_RenderTextureRotated(
                canvas.raw(),
                self.tex,
                &src,
                &d,
                0.0,
                std::ptr::null(),
                f,
            );
        }
    }

    /// Dibuja rotada `angle` grados alrededor de `pivot` (relativo a `dst`).
    pub fn draw_rotated(
        &self,
        canvas: &mut WindowCanvas,
        index: u32,
        dst: FRect,
        angle: f64,
        pivot: (f32, f32),
        flip_v: bool,
    ) {
        let src = SDL_FRect {
            x: index as f32 * self.cell_w,
            y: 0.0,
            w: self.cell_w,
            h: self.cell_h,
        };
        let d = frect(dst);
        let p = SDL_FPoint {
            x: pivot.0,
            y: pivot.1,
        };
        let f = if flip_v {
            sys::surface::SDL_FLIP_VERTICAL
        } else {
            sys::surface::SDL_FLIP_NONE
        };
        unsafe {
            sys::render::SDL_RenderTextureRotated(canvas.raw(), self.tex, &src, &d, angle, &p, f);
        }
    }

    pub fn set_alpha(&self, a: u8) {
        unsafe { sys::render::SDL_SetTextureAlphaMod(self.tex, a) };
    }

    pub fn set_tint(&self, r: u8, g: u8, b: u8) {
        unsafe { sys::render::SDL_SetTextureColorMod(self.tex, r, g, b) };
    }
}
