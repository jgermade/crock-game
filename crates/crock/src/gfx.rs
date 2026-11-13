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

/// Píxeles de arte por unidad de juego.
pub const ART: f32 = 2.0;

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
    pub cueva: Option<Sheet>,
    pub objetos: Option<Sheet>,
    pub crock: Option<Sheet>,
    pub porra: Option<Sheet>,
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
        Self {
            cueva: load(canvas, &g.join("cueva.png"), 16.0, 16.0),
            objetos: load(canvas, &g.join("objetos.png"), 16.0, 16.0),
            crock: load(canvas, &g.join("crock.png"), 32.0, 32.0),
            porra: load(canvas, &g.join("porra.png"), 16.0, 7.0),
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
