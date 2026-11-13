//! Estado de entrada: teclado + mandos (SDL3 gamepad), con flancos latcheados
//! entre ticks y timestamps de evento para medir latencia.

use std::collections::HashMap;

use crock_sim::Input;
use sdl3::GamepadSubsystem;
use sdl3::event::Event;
use sdl3::gamepad::{Axis, Button, Gamepad};
use sdl3::joystick::JoystickId;
use sdl3::keyboard::Scancode;

/// Zona muerta del stick (fracción de 1.0).
const STICK_DEADZONE: f32 = 0.35;

#[derive(Default)]
struct Pad {
    lx: f32,
    ly: f32,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    jump: bool,
}

pub struct InputState {
    gamepads: GamepadSubsystem,
    open: HashMap<u32, (Gamepad, Pad)>,
    // Teclado
    k_left: bool,
    k_right: bool,
    k_up: bool,
    k_down: bool,
    k_jump: bool,
    /// Flanco de salto pendiente de consumir por el próximo tick.
    jump_latch: bool,
    /// Timestamp (ns, reloj de SDL) de la última pulsación aún no presentada.
    pub pending_press_ts: Option<u64>,
}

fn is_jump_key(sc: Scancode) -> bool {
    matches!(
        sc,
        Scancode::Space | Scancode::C | Scancode::Z | Scancode::K
    )
}

impl InputState {
    pub fn new(gamepads: GamepadSubsystem) -> Self {
        Self {
            gamepads,
            open: HashMap::new(),
            k_left: false,
            k_right: false,
            k_up: false,
            k_down: false,
            k_jump: false,
            jump_latch: false,
            pending_press_ts: None,
        }
    }

    pub fn gamepad_names(&self) -> Vec<String> {
        self.open
            .values()
            .map(|(g, _)| g.name().unwrap_or_else(|| "mando".into()))
            .collect()
    }

    fn press(&mut self, ts: u64) {
        self.jump_latch = true;
        self.pending_press_ts.get_or_insert(ts);
    }

    /// Procesa un evento. Devuelve `true` si lo ha consumido.
    pub fn handle(&mut self, ev: &Event) -> bool {
        match *ev {
            Event::KeyDown {
                scancode: Some(sc),
                repeat: false,
                timestamp,
                ..
            } => {
                self.set_key(sc, true);
                if is_jump_key(sc) {
                    self.press(timestamp);
                }
                true
            }
            Event::KeyUp {
                scancode: Some(sc), ..
            } => {
                self.set_key(sc, false);
                true
            }
            Event::GamepadAdded { which, .. } => {
                if let Ok(g) = self.gamepads.open(which) {
                    self.open.insert(which.raw(), (g, Pad::default()));
                }
                true
            }
            Event::GamepadRemoved { which, .. } => {
                self.open.remove(&which.raw());
                true
            }
            Event::GamepadButtonDown {
                which,
                button,
                timestamp,
            } => {
                if self.set_button(which, button, true) {
                    self.press(timestamp);
                }
                true
            }
            Event::GamepadButtonUp { which, button, .. } => {
                self.set_button(which, button, false);
                true
            }
            Event::GamepadAxisMotion {
                which, axis, value, ..
            } => {
                if let Some((_, p)) = self.open.get_mut(&which.raw()) {
                    let v = (value as f32 / 32767.0).clamp(-1.0, 1.0);
                    match axis {
                        Axis::LeftX => p.lx = v,
                        Axis::LeftY => p.ly = v,
                        _ => {}
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn set_key(&mut self, sc: Scancode, down: bool) {
        match sc {
            Scancode::Left | Scancode::A => self.k_left = down,
            Scancode::Right | Scancode::D => self.k_right = down,
            Scancode::Up | Scancode::W => self.k_up = down,
            Scancode::Down | Scancode::S => self.k_down = down,
            sc if is_jump_key(sc) => self.k_jump = down,
            _ => {}
        }
    }

    /// Devuelve `true` si el botón es de salto y se acaba de pulsar.
    fn set_button(&mut self, which: JoystickId, b: Button, down: bool) -> bool {
        let Some((_, p)) = self.open.get_mut(&which.raw()) else {
            return false;
        };
        match b {
            Button::DPadLeft => p.left = down,
            Button::DPadRight => p.right = down,
            Button::DPadUp => p.up = down,
            Button::DPadDown => p.down = down,
            Button::South => {
                p.jump = down;
                return down;
            }
            _ => {}
        }
        false
    }

    /// Construye la entrada del próximo tick y consume los flancos latcheados.
    pub fn take_tick_input(&mut self) -> Input {
        let mut x = self.k_right as i8 - self.k_left as i8;
        let mut y = self.k_down as i8 - self.k_up as i8;
        let mut jump_held = self.k_jump;
        for (_, p) in self.open.values() {
            let (sx, sy) = stick_8dir(p.lx, p.ly);
            x += p.right as i8 - p.left as i8 + sx;
            y += p.down as i8 - p.up as i8 + sy;
            jump_held |= p.jump;
        }
        let input = Input {
            x: x.signum(),
            y: y.signum(),
            jump_held: jump_held || self.jump_latch,
            jump_pressed: self.jump_latch,
        };
        self.jump_latch = false;
        input
    }
}

/// Discretiza el stick a 8 direcciones por sectores de 45° (más justo que
/// umbralizar cada eje por separado: las diagonales tienen el mismo ancho).
fn stick_8dir(x: f32, y: f32) -> (i8, i8) {
    if x * x + y * y < STICK_DEADZONE * STICK_DEADZONE {
        return (0, 0);
    }
    let sector = ((y.atan2(x) / std::f32::consts::FRAC_PI_4).round() as i32).rem_euclid(8);
    const DIRS: [(i8, i8); 8] = [
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
        (0, -1),
        (1, -1),
    ];
    DIRS[sector as usize]
}

#[cfg(test)]
mod tests {
    use super::stick_8dir;

    #[test]
    fn stick_sectors() {
        assert_eq!(stick_8dir(0.1, 0.1), (0, 0));
        assert_eq!(stick_8dir(1.0, 0.0), (1, 0));
        assert_eq!(stick_8dir(0.7, 0.7), (1, 1));
        assert_eq!(stick_8dir(0.0, -1.0), (0, -1));
        assert_eq!(stick_8dir(-1.0, 0.05), (-1, 0));
        assert_eq!(stick_8dir(-0.6, -0.7), (-1, -1));
    }
}
