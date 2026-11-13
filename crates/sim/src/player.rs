use crate::strike::{Hit, SimEvent, Strikeable};
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
    /// Golpes disponibles (se recargan en el suelo o con ciertos objetos).
    pub strikes: u8,
    /// Ticks restantes de la ventana activa del golpe en curso.
    pub strike_window: u32,
    /// Ticks hasta poder iniciar otro golpe.
    pub strike_cooldown: u32,
    /// Dirección del último golpe (fijada al pulsar).
    pub strike_dir: (i8, i8),
    /// Ticks restantes de la animación de la porra (sólo visual).
    pub swing: u32,
    /// Ticks restantes sin gravedad ni control tras un impacto.
    pub float: u32,
    /// Ticks restantes de un golpe pulsado que aún no ha podido empezar.
    pub strike_buffer: u32,
    /// Dirección capturada al pulsar el golpe en buffer.
    pub strike_aim: (i8, i8),
    /// Dentro de fuego: la porra arde y no hay golpes.
    pub burning: bool,
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
            strikes: 1,
            strike_window: 0,
            strike_cooldown: 0,
            strike_dir: (1, 0),
            swing: 0,
            float: 0,
            strike_buffer: 0,
            strike_aim: (0, 0),
            burning: false,
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

    /// Fuego: mientras se está dentro no hay golpes (ni recarga).
    pub(crate) fn set_burning(&mut self, burning: bool, events: &mut Vec<SimEvent>) {
        if burning && (self.strikes > 0 || self.strike_window > 0) {
            events.push(SimEvent::Burn { at: self.center() });
        }
        if burning {
            self.strikes = 0;
            self.strike_window = 0;
            self.strike_buffer = 0;
        }
        self.burning = burning;
    }

    /// Durante el hit-stop el jugador no se actualiza, pero las pulsaciones
    /// quedan en buffer para ejecutarse en cuanto termine.
    pub(crate) fn buffer_during_hitstop(&mut self, t: &Tuning, input: &Input) {
        if input.jump_pressed {
            self.jump_buffer = secs_to_ticks(t.jump_buffer);
        }
        if input.strike_pressed {
            self.strike_buffer = secs_to_ticks(t.strike_buffer).max(1);
            self.strike_aim = (input.aim_x, input.aim_y);
        }
    }

    fn solid_at(&self, map: &TileMap, dx: i32, dy: i32) -> bool {
        map.solid_in_rect(self.pos.0 + dx, self.pos.1 + dy, Self::W, Self::H)
    }

    pub(crate) fn update(
        &mut self,
        map: &TileMap,
        t: &Tuning,
        input: &Input,
        objects: &mut [Strikeable],
        events: &mut Vec<SimEvent>,
    ) -> Option<Hit> {
        self.prev = self.exact_pos();

        // --- Suelo, coyote time y buffer de salto -------------------------
        let was_on_ground = self.on_ground;
        let fall_speed = self.vel.1;
        self.on_ground = self.vel.1 >= 0.0 && self.solid_at(map, 0, 1);
        if self.on_ground && !was_on_ground {
            events.push(SimEvent::Land { speed: fall_speed });
        }
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
        if self.on_ground && self.float == 0 && self.strike_window == 0 && !self.burning {
            self.strikes = t.max_strikes as u8;
        }

        // --- Golpe con la porra -------------------------------------------
        let hit = self.update_strike(
            t,
            input.strike_pressed,
            (input.aim_x, input.aim_y),
            objects,
            events,
        );
        let floating = self.float > 0;
        self.float = self.float.saturating_sub(1);

        // --- Horizontal ---------------------------------------------------
        if input.x != 0 {
            self.facing = input.x;
        }
        if floating {
            // Tras el impacto, la trayectoria es exactamente la del golpe.
            return self.finish_move(map, hit);
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
            events.push(SimEvent::Jump);
        }

        self.finish_move(map, hit)
    }

    fn finish_move(&mut self, map: &TileMap, hit: Option<Hit>) -> Option<Hit> {
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
        hit
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
