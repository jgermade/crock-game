//! Menús: título, pausa y opciones. Navegables con teclado y mando.

use sdl3::pixels::Color;
use sdl3::render::{FRect, WindowCanvas};
use sdl3::sys;

use crate::render::{Scaling, output_size, text};
use crate::settings::{FPS_CAPS, Fullscreen, PresentMode, Settings, StrikeButton, cycle};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Title,
    Pause,
    Options,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Nav {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
}

/// Lo que el bucle principal tiene que hacer tras una pulsación.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    None,
    /// Se ha movido la selección (para el sonido).
    Moved,
    Play,
    Resume,
    RestartRoom,
    ToTitle,
    Quit,
    /// Ha cambiado una opción: aplicar y guardar.
    SettingsChanged,
    /// Ha cambiado el capítulo elegido en el título.
    ChapterChanged,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Item {
    Play,
    Chapter,
    Options,
    Quit,
    Resume,
    Restart,
    ToTitle,
    Volume,
    Fullscreen,
    Present,
    Scaling,
    FpsCap,
    Deadzone,
    StrikeButton,
    Rumble,
    Overlay,
    Back,
}

pub struct Menu {
    stack: Vec<Page>,
    sel: usize,
    /// Selección de cada página de debajo, para volver a donde estaba.
    saved: Vec<usize>,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            sel: 0,
            saved: Vec::new(),
        }
    }

    pub fn is_open(&self) -> bool {
        !self.stack.is_empty()
    }

    pub fn page(&self) -> Option<Page> {
        self.stack.last().copied()
    }

    /// Abre una página encima de las actuales.
    pub fn open(&mut self, p: Page) {
        if !self.stack.is_empty() {
            self.saved.push(self.sel);
        }
        self.stack.push(p);
        self.sel = 0;
    }

    fn back(&mut self) {
        self.stack.pop();
        self.sel = self.saved.pop().unwrap_or(0);
    }

    /// Cierra todo y abre sólo `p`.
    pub fn reset_to(&mut self, p: Page) {
        self.close();
        self.open(p);
    }

    pub fn close(&mut self) {
        self.stack.clear();
        self.saved.clear();
    }

    fn items(page: Page) -> &'static [Item] {
        match page {
            Page::Title => &[Item::Play, Item::Chapter, Item::Options, Item::Quit],
            Page::Pause => &[Item::Resume, Item::Restart, Item::Options, Item::ToTitle],
            Page::Options => &[
                Item::Volume,
                Item::Fullscreen,
                Item::Present,
                Item::Scaling,
                Item::FpsCap,
                Item::Deadzone,
                Item::StrikeButton,
                Item::Rumble,
                Item::Overlay,
                Item::Back,
            ],
        }
    }

    /// Texto del elemento: (nombre, valor). Los de acción no tienen valor.
    fn label(item: Item, s: &Settings, chapter: &str) -> (String, String) {
        let yn = |b: bool| if b { "sí" } else { "no" }.to_string();
        let plain = |t: &str| (t.to_string(), String::new());
        match item {
            Item::Play => plain("Jugar"),
            Item::Chapter => plain(&format!("< {chapter} >")),
            Item::Options => plain("Opciones"),
            Item::Quit => plain("Salir"),
            Item::Resume => plain("Continuar"),
            Item::Restart => plain("Reiniciar sala"),
            Item::ToTitle => plain("Salir al título"),
            Item::Back => plain("Volver"),
            Item::Volume => (
                "Volumen".into(),
                "#".repeat(s.volume as usize) + &"-".repeat(10 - s.volume as usize),
            ),
            Item::Fullscreen => ("Pantalla".into(), s.fullscreen.label().into()),
            Item::Present => ("Sincronía".into(), s.present.label().into()),
            Item::Scaling => (
                "Escalado".into(),
                match s.scaling {
                    Scaling::Fill => "llenar",
                    Scaling::Integer => "entero",
                }
                .into(),
            ),
            Item::FpsCap => (
                "Límite de fps".into(),
                if s.fps_cap == 0 {
                    "no".to_string()
                } else {
                    s.fps_cap.to_string()
                },
            ),
            Item::Deadzone => ("Zona muerta".into(), format!("{}%", s.deadzone)),
            Item::StrikeButton => (
                "Golpe (mando)".into(),
                match s.strike_button {
                    StrikeButton::West => "X / RB",
                    StrikeButton::East => "B / RB",
                }
                .into(),
            ),
            Item::Rumble => ("Vibración".into(), yn(s.rumble)),
            Item::Overlay => ("Métricas".into(), yn(s.overlay)),
        }
    }

    /// Procesa una pulsación. Modifica las opciones en sitio.
    pub fn nav(
        &mut self,
        n: Nav,
        s: &mut Settings,
        chapter: &mut usize,
        chapters: usize,
    ) -> Action {
        let Some(page) = self.page() else {
            return Action::None;
        };
        let items = Self::items(page);
        let item = items[self.sel.min(items.len() - 1)];
        match n {
            Nav::Up => {
                self.sel = (self.sel + items.len() - 1) % items.len();
                Action::Moved
            }
            Nav::Down => {
                self.sel = (self.sel + 1) % items.len();
                Action::Moved
            }
            Nav::Back => match page {
                Page::Title => Action::None,
                Page::Pause => {
                    self.close();
                    Action::Resume
                }
                Page::Options => {
                    self.back();
                    Action::Moved
                }
            },
            Nav::Left | Nav::Right | Nav::Confirm => {
                let dir = if n == Nav::Left { -1 } else { 1 };
                let lr = n != Nav::Confirm;
                match item {
                    Item::Play if !lr => {
                        self.close();
                        Action::Play
                    }
                    Item::Chapter => {
                        *chapter = (*chapter as i32 + dir).rem_euclid(chapters as i32) as usize;
                        Action::ChapterChanged
                    }
                    Item::Options if !lr => {
                        self.open(Page::Options);
                        Action::Moved
                    }
                    Item::Quit if !lr => Action::Quit,
                    Item::Resume if !lr => {
                        self.close();
                        Action::Resume
                    }
                    Item::Restart if !lr => {
                        self.close();
                        Action::RestartRoom
                    }
                    Item::ToTitle if !lr => {
                        self.reset_to(Page::Title);
                        Action::ToTitle
                    }
                    Item::Back if !lr => {
                        self.back();
                        Action::Moved
                    }
                    Item::Volume => {
                        s.volume = (s.volume as i32 + dir).clamp(0, 10) as u8;
                        Action::SettingsChanged
                    }
                    Item::Fullscreen => {
                        s.fullscreen = cycle(&Fullscreen::ALL, s.fullscreen, dir);
                        Action::SettingsChanged
                    }
                    Item::Present => {
                        s.present = cycle(&PresentMode::ALL, s.present, dir);
                        Action::SettingsChanged
                    }
                    Item::Scaling => {
                        s.scaling = cycle(&[Scaling::Fill, Scaling::Integer], s.scaling, dir);
                        Action::SettingsChanged
                    }
                    Item::FpsCap => {
                        s.fps_cap = cycle(&FPS_CAPS, s.fps_cap, dir);
                        Action::SettingsChanged
                    }
                    Item::Deadzone => {
                        s.deadzone = (s.deadzone as i32 + 5 * dir).clamp(10, 70) as u8;
                        Action::SettingsChanged
                    }
                    Item::StrikeButton => {
                        s.strike_button = cycle(
                            &[StrikeButton::West, StrikeButton::East],
                            s.strike_button,
                            dir,
                        );
                        Action::SettingsChanged
                    }
                    Item::Rumble => {
                        s.rumble = !s.rumble;
                        Action::SettingsChanged
                    }
                    Item::Overlay => {
                        s.overlay = !s.overlay;
                        Action::SettingsChanged
                    }
                    _ => Action::None,
                }
            }
        }
    }

    /// Dibuja el menú encima de lo que haya (el juego, atenuado).
    pub fn draw(&self, canvas: &mut WindowCanvas, s: &Settings, chapter: &str) {
        let Some(page) = self.page() else { return };
        let (ow, oh) = output_size(canvas);
        canvas.set_blend_mode(sdl3::render::BlendMode::Blend);
        canvas.set_draw_color(Color::RGBA(
            10,
            8,
            14,
            if page == Page::Title { 150 } else { 190 },
        ));
        let _ = canvas.fill_rect(FRect::new(0.0, 0.0, ow as f32, oh as f32));

        // Escala del texto (fuente de 8 px) según la altura de la ventana.
        let u = (oh as f32 / 360.0).floor().max(1.0);
        let center = |canvas: &mut WindowCanvas, y: f32, str: &str, scale: f32, color: Color| {
            unsafe { sys::render::SDL_SetRenderScale(canvas.raw(), scale, scale) };
            let w = str.chars().count() as f32 * 8.0;
            canvas.set_draw_color(color);
            text(canvas, (ow as f32 / scale - w) / 2.0, y / scale, str);
            unsafe { sys::render::SDL_SetRenderScale(canvas.raw(), 1.0, 1.0) };
        };

        let (title, sub) = match page {
            Page::Title => ("cROCK", "un troglodita, una porra"),
            Page::Pause => ("PAUSA", ""),
            Page::Options => ("OPCIONES", ""),
        };
        let mut y = oh as f32 * 0.16;
        if page == Page::Title {
            // Sombra + título grande.
            let big = u * 5.0;
            center(canvas, y + big * 0.35, title, big, Color::RGB(40, 22, 16));
            center(canvas, y, title, big, Color::RGB(240, 196, 120));
            y += big * 10.0;
            center(canvas, y, sub, u, Color::RGB(200, 180, 150));
            y += u * 36.0;
        } else {
            center(canvas, y, title, u * 3.0, Color::RGB(240, 196, 120));
            y += u * 40.0;
        }

        let items = Self::items(page);
        let scale = if page == Page::Options {
            u * 1.5
        } else {
            u * 2.0
        };
        for (i, item) in items.iter().enumerate() {
            let (name, value) = Self::label(*item, s, chapter);
            let selected = i == self.sel;
            let color = if selected {
                Color::RGB(255, 230, 150)
            } else {
                Color::RGB(170, 160, 150)
            };
            if value.is_empty() {
                let shown = if selected {
                    format!("> {name} <")
                } else {
                    name
                };
                center(canvas, y, &shown, scale, color);
            } else {
                // Dos columnas: nombre a la izquierda del centro, valor a la derecha.
                unsafe { sys::render::SDL_SetRenderScale(canvas.raw(), scale, scale) };
                let mid = ow as f32 / 2.0 / scale;
                let nw = name.chars().count() as f32 * 8.0;
                canvas.set_draw_color(color);
                text(canvas, mid - 12.0 - nw, y / scale, &name);
                text(canvas, mid + 12.0, y / scale, &value);
                if selected {
                    text(canvas, mid - 12.0 - nw - 20.0, y / scale, ">");
                }
                unsafe { sys::render::SDL_SetRenderScale(canvas.raw(), 1.0, 1.0) };
            }
            y += scale * 13.0;
        }

        let help = "flechas/cruceta: mover   enter/A: aceptar   esc/B: volver";
        center(
            canvas,
            oh as f32 - u * 14.0,
            help,
            u,
            Color::RGB(120, 110, 104),
        );
        canvas.set_blend_mode(sdl3::render::BlendMode::None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_flow() {
        let mut m = Menu::new();
        let mut s = Settings::default();
        let mut ch = 0;
        m.reset_to(Page::Title);
        // Abajo: capítulo; derecha: siguiente capítulo.
        assert_eq!(m.nav(Nav::Down, &mut s, &mut ch, 3), Action::Moved);
        assert_eq!(
            m.nav(Nav::Right, &mut s, &mut ch, 3),
            Action::ChapterChanged
        );
        assert_eq!(ch, 1);
        assert_eq!(m.nav(Nav::Left, &mut s, &mut ch, 3), Action::ChapterChanged);
        assert_eq!(m.nav(Nav::Left, &mut s, &mut ch, 3), Action::ChapterChanged);
        assert_eq!(ch, 2, "da la vuelta");
        // Opciones: bajar volumen y volver.
        m.nav(Nav::Down, &mut s, &mut ch, 3);
        m.nav(Nav::Confirm, &mut s, &mut ch, 3);
        assert_eq!(m.page(), Some(Page::Options));
        assert_eq!(
            m.nav(Nav::Left, &mut s, &mut ch, 3),
            Action::SettingsChanged
        );
        assert_eq!(s.volume, 6);
        m.nav(Nav::Back, &mut s, &mut ch, 3);
        assert_eq!(m.page(), Some(Page::Title));
        // Vuelve a "Opciones"; dos arriba: "Jugar".
        m.nav(Nav::Up, &mut s, &mut ch, 3);
        m.nav(Nav::Up, &mut s, &mut ch, 3);
        assert_eq!(m.nav(Nav::Confirm, &mut s, &mut ch, 3), Action::Play);
        assert!(!m.is_open());
    }

    #[test]
    fn pause_back_resumes() {
        let mut m = Menu::new();
        let mut s = Settings::default();
        let mut ch = 0;
        m.open(Page::Pause);
        assert_eq!(m.nav(Nav::Back, &mut s, &mut ch, 1), Action::Resume);
        assert!(!m.is_open());
    }
}
