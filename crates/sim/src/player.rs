use crate::{DT, Input, TileMap, Tuning, secs_to_ticks};

/// Caja de colisión en píxeles enteros.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Aabb {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// El troglodita.
///
/// La posición es entera (esquina superior izquierda de la caja) más un
/// resto fraccionario por eje. El movimiento avanza píxel a píxel contra la
/// rejilla, así que nunca atraviesa paredes ni queda incrustado, sea cual
/// sea la velocidad. Eje Y hacia abajo.
#[derive(Clone, Debug)]
pub struct Player {
    pub pos: (i32, i32),
    pub rem: (f32, f32),
    pub vel: (f32, f32),
    /// Posición exacta al inicio del último tick (para interpolar el render).
    pub prev: (f32, f32),
    pub on_ground: bool,
    pub facing: i8,
    /// Ticks restantes de coyote time.
    pub coyote: u32,
    /// Ticks restantes de un salto pulsado y aún no consumido.
    pub jump_buffer: u32,
    /// Ticks restantes de salto variable (mantener el botón sostiene la subida).
    pub var_jump: u32,
    var_jump_speed: f32,
}

impl Player {
    pub const W: i32 = 8;
    pub const H: i32 = 11;

    /// Crea al jugador con los pies centrados en `feet` (píxeles).
    pub fn spawn(feet: (i32, i32)) -> Self {
        let pos = (feet.0 - Self::W / 2, feet.1 - Self::H);
        Self {
            pos,
            rem: (0.0, 0.0),
            vel: (0.0, 0.0),
            prev: (pos.0 as f32, pos.1 as f32),
            on_ground: false,
            facing: 1,
            coyote: 0,
            jump_buffer: 0,
            var_jump: 0,
            var_jump_speed: 0.0,
        }
    }

    pub fn aabb(&self) -> Aabb {
        Aabb {
            x: self.pos.0,
            y: self.pos.1,
            w: Self::W,
            h: Self::H,
        }
    }

    /// Posición exacta (entera + resto).
    pub fn exact_pos(&self) -> (f32, f32) {
        (
            self.pos.0 as f32 + self.rem.0,
            self.pos.1 as f32 + self.rem.1,
        )
    }

    /// Posición interpolada entre el tick anterior y el actual. `alpha ∈ [0,1)`.
    pub fn lerp_pos(&self, alpha: f32) -> (f32, f32) {
        let (cx, cy) = self.exact_pos();
        (
            self.prev.0 + (cx - self.prev.0) * alpha,
            self.prev.1 + (cy - self.prev.1) * alpha,
        )
    }

    fn solid_at(&self, map: &TileMap, dx: i32, dy: i32) -> bool {
        map.solid_in_rect(self.pos.0 + dx, self.pos.1 + dy, Self::W, Self::H)
    }

    pub(crate) fn update(&mut self, map: &TileMap, t: &Tuning, input: &Input) {
        self.prev = self.exact_pos();

        // --- Suelo, coyote time y buffer de salto -------------------------
        self.on_ground = self.vel.1 >= 0.0 && self.solid_at(map, 0, 1);
        if self.on_ground {
            self.coyote = secs_to_ticks(t.coyote_time);
        } else {
            self.coyote = self.coyote.saturating_sub(1);
        }
        if input.jump_pressed {
            self.jump_buffer = secs_to_ticks(t.jump_buffer);
        } else {
            self.jump_buffer = self.jump_buffer.saturating_sub(1);
        }

        // --- Horizontal ---------------------------------------------------
        if input.x != 0 {
            self.facing = input.x;
        }
        let mult = if self.on_ground { 1.0 } else { t.air_mult };
        let target = t.run_speed * input.x as f32;
        let over_speed = self.vel.0.abs() > t.run_speed && self.vel.0.signum() == input.x as f32;
        let accel = if over_speed {
            t.run_reduce
        } else {
            t.run_accel
        };
        self.vel.0 = approach(self.vel.0, target, accel * mult * DT);

        // --- Vertical: gravedad -------------------------------------------
        let max_fall = if input.y > 0 {
            t.fast_max_fall
        } else {
            t.max_fall
        };
        if !self.on_ground {
            let near_apex = self.vel.1.abs() < t.half_grav_threshold;
            let g = if near_apex && input.jump_held {
                t.gravity * 0.5
            } else {
                t.gravity
            };
            self.vel.1 = approach(self.vel.1, max_fall, g * DT);
        }

        // --- Salto variable -----------------------------------------------
        if self.var_jump > 0 {
            if input.jump_held {
                self.vel.1 = self.vel.1.min(self.var_jump_speed);
                self.var_jump -= 1;
            } else {
                self.var_jump = 0;
            }
        }

        // --- Salto (con buffer + coyote) ----------------------------------
        if self.jump_buffer > 0 && self.coyote > 0 {
            self.jump_buffer = 0;
            self.coyote = 0;
            self.on_ground = false;
            self.vel.1 = -t.jump_speed;
            self.vel.0 += t.jump_h_boost * input.x as f32;
            self.var_jump_speed = self.vel.1;
            self.var_jump = secs_to_ticks(t.var_jump_time);
        }

        // --- Movimiento con colisión --------------------------------------
        if self.move_x(map, self.vel.0 * DT) {
            self.vel.0 = 0.0;
        }
        if self.move_y(map, self.vel.1 * DT) {
            if self.vel.1 < 0.0 {
                // Cabezazo contra el techo: se acaba el salto variable.
                self.var_jump = 0;
            }
            self.vel.1 = 0.0;
        }
    }

    /// Mueve en X; devuelve `true` si choca.
    fn move_x(&mut self, map: &TileMap, amount: f32) -> bool {
        self.rem.0 += amount;
        let mut mv = self.rem.0.round() as i32;
        if mv == 0 {
            return false;
        }
        self.rem.0 -= mv as f32;
        let sign = mv.signum();
        while mv != 0 {
            if self.solid_at(map, sign, 0) {
                self.rem.0 = 0.0;
                return true;
            }
            self.pos.0 += sign;
            mv -= sign;
        }
        false
    }

    /// Mueve en Y; devuelve `true` si choca.
    fn move_y(&mut self, map: &TileMap, amount: f32) -> bool {
        self.rem.1 += amount;
        let mut mv = self.rem.1.round() as i32;
        if mv == 0 {
            return false;
        }
        self.rem.1 -= mv as f32;
        let sign = mv.signum();
        while mv != 0 {
            if self.solid_at(map, 0, sign) {
                self.rem.1 = 0.0;
                return true;
            }
            self.pos.1 += sign;
            mv -= sign;
        }
        false
    }
}

fn approach(v: f32, target: f32, max_delta: f32) -> f32 {
    if v < target {
        (v + max_delta).min(target)
    } else {
        (v - max_delta).max(target)
    }
}
