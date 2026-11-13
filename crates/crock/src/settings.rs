//! Opciones del jugador (pantalla, audio, mando) y su persistencia en el
//! directorio de preferencias del usuario (`opciones.cfg`, `clave = valor`).

use std::ffi::CStr;
use std::path::PathBuf;

use sdl3::render::WindowCanvas;
use sdl3::sys;

use crate::render::Scaling;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PresentMode {
    VSync,
    Adaptive,
    Immediate,
}

impl PresentMode {
    pub const ALL: [Self; 3] = [Self::VSync, Self::Adaptive, Self::Immediate];

    fn sdl_value(self) -> i32 {
        match self {
            Self::VSync => 1,
            Self::Adaptive => sys::render::SDL_RENDERER_VSYNC_ADAPTIVE,
            Self::Immediate => sys::render::SDL_RENDERER_VSYNC_DISABLED,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::VSync => "vsync",
            Self::Adaptive => "adaptativa",
            Self::Immediate => "inmediata (tearing)",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Self::VSync => "vsync",
            Self::Adaptive => "adaptive",
            Self::Immediate => "immediate",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "on" | "vsync" => Some(Self::VSync),
            "adaptive" => Some(Self::Adaptive),
            "off" | "immediate" => Some(Self::Immediate),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fullscreen {
    Windowed,
    Borderless,
    Exclusive,
}

impl Fullscreen {
    pub const ALL: [Self; 3] = [Self::Windowed, Self::Borderless, Self::Exclusive];

    pub fn label(self) -> &'static str {
        match self {
            Self::Windowed => "ventana",
            Self::Borderless => "sin bordes",
            Self::Exclusive => "exclusiva",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Self::Windowed => "window",
            Self::Borderless => "borderless",
            Self::Exclusive => "exclusive",
        }
    }
}

/// Qué botón del mando golpea (el salto es siempre el botón sur).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StrikeButton {
    /// Botón oeste (X en Xbox, □ en PlayStation) y RB.
    West,
    /// Botón este (B en Xbox, ○ en PlayStation) y RB.
    East,
}

pub const FPS_CAPS: [u32; 6] = [0, 60, 120, 144, 240, 360];

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// Volumen 0..=10.
    pub volume: u8,
    pub fullscreen: Fullscreen,
    pub present: PresentMode,
    pub scaling: Scaling,
    /// Límite de fps (0 = sin límite).
    pub fps_cap: u32,
    /// Zona muerta del stick, en %.
    pub deadzone: u8,
    pub strike_button: StrikeButton,
    pub rumble: bool,
    /// Overlay de métricas visible.
    pub overlay: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            volume: 7,
            fullscreen: Fullscreen::Windowed,
            present: PresentMode::VSync,
            scaling: Scaling::Fill,
            fps_cap: 0,
            deadzone: 35,
            strike_button: StrikeButton::West,
            rumble: true,
            overlay: false,
        }
    }
}

/// Avanza `v` dentro de `all` (con vuelta) en el sentido `dir` (-1 o 1).
pub fn cycle<T: Copy + PartialEq>(all: &[T], v: T, dir: i32) -> T {
    let i = all.iter().position(|x| *x == v).unwrap_or(0) as i32;
    all[(i + dir).rem_euclid(all.len() as i32) as usize]
}

impl Settings {
    pub fn volume_f(&self) -> f32 {
        // Curva aproximadamente perceptual.
        (self.volume as f32 / 10.0).powf(1.7)
    }

    pub fn deadzone_f(&self) -> f32 {
        self.deadzone as f32 / 100.0
    }

    pub fn encode(&self) -> String {
        format!(
            "# Opciones de cROCK (se reescribe al cambiarlas en el juego)\n\
             volume = {}\nfullscreen = {}\npresent = {}\nscaling = {}\nfps_cap = {}\n\
             deadzone = {}\nstrike_button = {}\nrumble = {}\noverlay = {}\n",
            self.volume,
            self.fullscreen.key(),
            self.present.key(),
            match self.scaling {
                Scaling::Fill => "fill",
                Scaling::Integer => "integer",
            },
            self.fps_cap,
            self.deadzone,
            match self.strike_button {
                StrikeButton::West => "west",
                StrikeButton::East => "east",
            },
            self.rumble,
            self.overlay,
        )
    }

    /// Parsea partiendo de los valores por defecto; ignora claves raras.
    pub fn decode(src: &str) -> Self {
        let mut s = Self::default();
        for line in src.lines() {
            let line = line.split('#').next().unwrap().trim();
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let (k, v) = (k.trim(), v.trim());
            match k {
                "volume" => s.volume = v.parse::<u8>().map_or(s.volume, |n| n.min(10)),
                "fullscreen" => {
                    s.fullscreen = Fullscreen::ALL
                        .into_iter()
                        .find(|f| f.key() == v)
                        .unwrap_or(s.fullscreen)
                }
                "present" => s.present = PresentMode::parse(v).unwrap_or(s.present),
                "scaling" => {
                    s.scaling = if v == "integer" {
                        Scaling::Integer
                    } else {
                        Scaling::Fill
                    }
                }
                "fps_cap" => {
                    s.fps_cap = v
                        .parse()
                        .ok()
                        .filter(|n| FPS_CAPS.contains(n))
                        .unwrap_or(s.fps_cap)
                }
                "deadzone" => s.deadzone = v.parse::<u8>().map_or(s.deadzone, |n| n.clamp(10, 70)),
                "strike_button" => {
                    s.strike_button = if v == "east" {
                        StrikeButton::East
                    } else {
                        StrikeButton::West
                    }
                }
                "rumble" => s.rumble = v != "false",
                "overlay" => s.overlay = v == "true",
                _ => {}
            }
        }
        s
    }

    /// `opciones.cfg` en el directorio de preferencias de SDL.
    pub fn path() -> Option<PathBuf> {
        unsafe {
            let p = sys::filesystem::SDL_GetPrefPath(c"crock".as_ptr(), c"crock".as_ptr());
            if p.is_null() {
                return None;
            }
            let dir = PathBuf::from(CStr::from_ptr(p).to_string_lossy().into_owned());
            sys::stdinc::SDL_free(p as *mut _);
            Some(dir.join("opciones.cfg"))
        }
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|s| Self::decode(&s))
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(p) = Self::path()
            && let Err(e) = std::fs::write(&p, self.encode())
        {
            eprintln!(
                "[crock] no se pudieron guardar las opciones en {}: {e}",
                p.display()
            );
        }
    }
}

/// Aplica el modo de presentación; si no está soportado, cae a vsync.
pub fn apply_present(canvas: &WindowCanvas, mode: PresentMode) -> PresentMode {
    if unsafe { sys::render::SDL_SetRenderVSync(canvas.raw(), mode.sdl_value()) } {
        return mode;
    }
    eprintln!("[crock] modo de presentación {mode:?} no soportado");
    unsafe { sys::render::SDL_SetRenderVSync(canvas.raw(), 1) };
    PresentMode::VSync
}

pub fn apply_fullscreen(window: &sdl3::video::Window, mode: Fullscreen) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let s = Settings {
            volume: 3,
            fullscreen: Fullscreen::Exclusive,
            present: PresentMode::Immediate,
            scaling: Scaling::Integer,
            fps_cap: 144,
            deadzone: 25,
            strike_button: StrikeButton::East,
            rumble: false,
            overlay: true,
        };
        assert_eq!(Settings::decode(&s.encode()), s);
        let junk = Settings::decode("volume = 99\nfps_cap = 7\nfoo = 1\ndeadzone = 1");
        assert_eq!(junk.volume, 10);
        assert_eq!(junk.fps_cap, 0);
        assert_eq!(junk.deadzone, 10);
    }

    #[test]
    fn cycles_wrap() {
        assert_eq!(cycle(&FPS_CAPS, 0, -1), 360);
        assert_eq!(cycle(&FPS_CAPS, 360, 1), 0);
    }
}
