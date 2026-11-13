//! El golpe con la porra: objetos golpeables (incluidos los enemigos),
//! detección e impulso.

use crate::{DT, ObjectDef, Player, TILE, TileMap, Tuning, secs_to_ticks};

/// Tipos de objeto golpeable. Cada uno con su carácter en los mapas ASCII y
/// su nombre de tipo en Tiled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrikeKind {
    /// `o` Roca: impulso normal, recarga el golpe. Reutilizable tras un
    /// breve enfriamiento.
    Rock,
    /// `x` Roca frágil: como la roca, pero se rompe y reaparece al rato.
    Fragile,
    /// `b` Hueso colgante: impulso fuerte, pero **no** recarga el golpe.
    Bone,
    /// `m` Musgo: impulso flojo, recarga el golpe.
    Moss,
    /// `e` Escarabajo: patrulla una plataforma; tocarlo mata. Golpearlo lo
    /// aturde (se vuelve inofensivo un rato) y recarga el golpe.
    Beetle,
    /// `w` Murciélago: vuela de lado a lado; tocarlo mata. Golpearlo lo
    /// derriba hasta el próximo reinicio y recarga el golpe.
    Bat,
}

impl StrikeKind {
    pub const ALL: [Self; 6] = [
        Self::Rock,
        Self::Fragile,
        Self::Bone,
        Self::Moss,
        Self::Beetle,
        Self::Bat,
    ];

    pub fn from_char(c: char) -> Option<Self> {
        Some(match c {
            'o' => Self::Rock,
            'x' => Self::Fragile,
            'b' => Self::Bone,
            'm' => Self::Moss,
            'e' => Self::Beetle,
            'w' => Self::Bat,
            _ => return None,
        })
    }

    /// Nombre del tipo en Tiled.
    pub fn name(self) -> &'static str {
        match self {
            Self::Rock => "rock",
            Self::Fragile => "fragile",
            Self::Bone => "bone",
            Self::Moss => "moss",
            Self::Beetle => "beetle",
            Self::Bat => "bat",
        }
    }

    pub fn from_name(n: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.name() == n)
    }

    /// Tamaño en píxeles (ancho, alto).
    pub fn size(self) -> (i32, i32) {
        match self {
            Self::Rock | Self::Fragile => (8, 8),
            Self::Bone => (4, 10),
            Self::Moss => (8, 6),
            Self::Beetle => (8, 6),
            Self::Bat => (8, 6),
        }
    }

    pub fn impulse_mult(self, t: &Tuning) -> f32 {
        match self {
            Self::Rock | Self::Fragile | Self::Beetle | Self::Bat => 1.0,
            Self::Bone => t.bone_mult,
            Self::Moss => t.moss_mult,
        }
    }

    pub fn refills(self) -> bool {
        !matches!(self, Self::Bone)
    }

    pub fn is_enemy(self) -> bool {
        matches!(self, Self::Beetle | Self::Bat)
    }
}

/// Un objeto golpeable en el mundo. No es sólido: el jugador lo atraviesa
/// (salvo los enemigos, que matan al tocarlos).
#[derive(Clone, Debug, PartialEq)]
pub struct Strikeable {
    pub kind: StrikeKind,
    /// Centro actual en píxeles.
    pub center: (f32, f32),
    /// Centro inicial (los que se mueven patrullan alrededor de él).
    pub origin: (f32, f32),
    /// Recorrido a cada lado (px) y sentido actual (-1 o 1).
    pub range: f32,
    pub dir: i8,
    /// Ticks hasta que se puede volver a golpear.
    pub cooldown: u32,
    /// Ticks hasta reaparecer (sólo frágiles rotas). 0 = presente.
    pub broken: u32,
    /// Ticks restantes de aturdimiento (escarabajo).
    pub stun: u32,
    /// Derribado hasta el próximo reinicio (murciélago).
    pub dead: bool,
    /// Ticks de vida, para animaciones deterministas.
    pub age: u32,
}

impl Strikeable {
    pub fn new(kind: StrikeKind, center: (f32, f32)) -> Self {
        Self::from_def(&ObjectDef::new(kind, center))
    }

    pub fn from_def(d: &ObjectDef) -> Self {
        let mut center = d.center;
        if d.kind == StrikeKind::Beetle {
            // Apoyado en el suelo de su celda.
            let cell_bottom = ((center.1 / TILE as f32).floor() + 1.0) * TILE as f32;
            center.1 = cell_bottom - d.kind.size().1 as f32 / 2.0;
        }
        Self {
            kind: d.kind,
            center,
            origin: center,
            range: d.range,
            dir: if d.dir < 0 { -1 } else { 1 },
            cooldown: 0,
            broken: 0,
            stun: 0,
            dead: false,
            age: 0,
        }
    }

    /// ¿Se puede golpear ahora?
    pub fn active(&self) -> bool {
        !self.dead && self.broken == 0 && self.cooldown == 0
    }

    /// ¿Mata al tocarlo?
    pub fn hostile(&self) -> bool {
        match self.kind {
            StrikeKind::Beetle => self.stun == 0,
            StrikeKind::Bat => !self.dead,
            _ => false,
        }
    }

    /// Caja (x0, y0, x1, y1) en píxeles.
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        let (w, h) = self.kind.size();
        let x0 = self.center.0 - w as f32 / 2.0;
        let y0 = self.center.1 - h as f32 / 2.0;
        (x0, y0, x0 + w as f32, y0 + h as f32)
    }

    /// Temporizadores: corren también durante el hit-stop.
    pub(crate) fn tick_timers(&mut self) {
        self.cooldown = self.cooldown.saturating_sub(1);
        self.broken = self.broken.saturating_sub(1);
    }

    /// Comportamiento (movimiento de enemigos). Se congela en el hit-stop.
    pub(crate) fn behave(&mut self, map: &TileMap, t: &Tuning) {
        self.age = self.age.wrapping_add(1);
        match self.kind {
            StrikeKind::Beetle => {
                if self.stun > 0 {
                    self.stun -= 1;
                    return;
                }
                let (w, h) = self.kind.size();
                let next = self.center.0 + self.dir as f32 * t.beetle_speed * DT;
                let half = w as f32 / 2.0;
                // Píxel justo delante del borde frontal.
                let front = if self.dir > 0 {
                    (next + half).floor() as i32
                } else {
                    (next - half).floor() as i32
                };
                let top = (self.center.1 - h as f32 / 2.0).floor() as i32;
                let bottom = (self.center.1 + h as f32 / 2.0).round() as i32;
                let wall = map.solid_in_rect(front, top, 1, h);
                let ground = map.solid_in_rect(front, bottom, 1, 1);
                if wall || !ground {
                    self.dir = -self.dir;
                } else {
                    self.center.0 = next;
                }
            }
            StrikeKind::Bat => {
                if self.dead || self.range <= 0.0 {
                    return;
                }
                // Vaivén senoidal: velocidad máxima `bat_speed`.
                let secs = self.age as f32 * DT;
                let w = t.bat_speed / self.range;
                self.center.0 = self.origin.0 + self.dir as f32 * self.range * (secs * w).sin();
                self.center.1 = self.origin.1 + 2.0 * (secs * 5.0).sin();
            }
            _ => {}
        }
    }
}

/// Algo ocurrido en la simulación que la plataforma quiere saber (efectos,
/// sonido). Se acumulan en [`crate::World::events`] hasta que se drenan.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimEvent {
    /// Golpe con éxito: punto de impacto, dirección del golpe y objeto.
    Strike {
        at: (f32, f32),
        dir: (i8, i8),
        kind: StrikeKind,
    },
    /// Una roca frágil se ha roto.
    Break {
        at: (f32, f32),
    },
    /// El golpe terminó sin tocar nada.
    Whiff,
    /// Un enemigo derribado o aturdido.
    EnemyDown {
        at: (f32, f32),
        kind: StrikeKind,
    },
    /// El jugador entra en fuego con la porra cargada: se queda sin golpes.
    Burn {
        at: (f32, f32),
    },
    /// El jugador muere (justo antes de reaparecer).
    Death {
        at: (f32, f32),
    },
    /// Se ha pasado a otra sala.
    RoomEnter {
        room: usize,
    },
    /// Se ha completado la última sala.
    ChapterComplete,
    /// El jugador salta.
    Jump,
    /// El jugador se agarra a una cornisa.
    Grab {
        at: (f32, f32),
    },
    /// El jugador toca suelo, con la velocidad de caída que llevaba (px/s).
    Land {
        speed: f32,
    },
    Respawn,
}

/// Resultado de un impacto, para que el mundo aplique hit-stop y shake.
pub(crate) struct Hit {
    pub dir: (i8, i8),
}

/// Vector unitario de una dirección de 8.
pub fn unit(dir: (i8, i8)) -> (f32, f32) {
    let (x, y) = (dir.0 as f32, dir.1 as f32);
    let len = (x * x + y * y).sqrt();
    if len == 0.0 {
        (0.0, 0.0)
    } else {
        (x / len, y / len)
    }
}

impl Player {
    pub fn center(&self) -> (f32, f32) {
        let (x, y) = self.exact_pos();
        (x + Self::W as f32 / 2.0, y + Self::H as f32 / 2.0)
    }

    /// Caja de golpe (x0, y0, x1, y1) para la dirección dada.
    pub fn strike_box(&self, dir: (i8, i8), t: &Tuning) -> (f32, f32, f32, f32) {
        let (cx, cy) = self.center();
        let (ux, uy) = unit(dir);
        let (hx, hy) = (cx + ux * t.strike_reach, cy + uy * t.strike_reach);
        let r = t.strike_size / 2.0;
        (hx - r, hy - r, hx + r, hy + r)
    }

    /// Gestiona la pulsación y la ventana activa del golpe. Devuelve el
    /// impacto si lo hay (ya aplicado al jugador y al objeto).
    pub(crate) fn update_strike(
        &mut self,
        t: &Tuning,
        pressed: bool,
        aim: (i8, i8),
        objects: &mut [Strikeable],
        events: &mut Vec<SimEvent>,
    ) -> Option<Hit> {
        self.strike_cooldown = self.strike_cooldown.saturating_sub(1);

        if pressed {
            // Apuntado instantáneo: la dirección es la del instante de pulsar.
            self.strike_buffer = secs_to_ticks(t.strike_buffer).max(1);
            self.strike_aim = aim;
        }
        if self.strike_buffer > 0
            && self.strike_cooldown == 0
            && self.strike_window == 0
            && self.strikes > 0
        {
            self.strike_buffer = 0;
            self.strike_dir = if self.strike_aim == (0, 0) {
                (self.facing, 0)
            } else {
                self.strike_aim
            };
            self.strike_window = secs_to_ticks(t.strike_window).max(1);
            self.swing = secs_to_ticks(0.12);
        } else {
            self.strike_buffer = self.strike_buffer.saturating_sub(1);
        }
        self.swing = self.swing.saturating_sub(1);

        if self.strike_window == 0 {
            return None;
        }

        let hb = self.strike_box(self.strike_dir, t);
        let (hcx, hcy) = ((hb.0 + hb.2) / 2.0, (hb.1 + hb.3) / 2.0);
        let mut best: Option<(usize, f32)> = None;
        for (i, o) in objects.iter().enumerate() {
            if !o.active() {
                continue;
            }
            let b = o.bounds();
            if hb.0 < b.2 && b.0 < hb.2 && hb.1 < b.3 && b.1 < hb.3 {
                let d = (o.center.0 - hcx).powi(2) + (o.center.1 - hcy).powi(2);
                if best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((i, d));
                }
            }
        }

        let Some((index, _)) = best else {
            self.strike_window -= 1;
            if self.strike_window == 0 {
                self.strike_cooldown = secs_to_ticks(t.strike_cooldown);
                events.push(SimEvent::Whiff);
            }
            return None;
        };

        // --- Impacto ---------------------------------------------------------
        let dir = self.strike_dir;
        let o = &mut objects[index];
        let speed = t.strike_speed * o.kind.impulse_mult(t);
        let (ux, uy) = unit(dir);
        // Sale despedido en la dirección opuesta al golpe.
        self.vel = (-ux * speed, -uy * speed);
        if t.strike_steer != 0.0 {
            // Opción a evaluar: empuje adicional del stick en el impacto.
            let (sx, sy) = unit(aim);
            self.vel.0 += sx * t.strike_steer;
            self.vel.1 += sy * t.strike_steer;
        }
        self.strike_window = 0;
        // Tras un impacto se puede encadenar en cuanto acaba el `float`.
        self.strike_cooldown = secs_to_ticks(t.strike_float);
        self.float = secs_to_ticks(t.strike_float);
        self.var_jump = 0;
        self.coyote = 0;
        self.on_ground = false;

        self.strikes -= 1;
        if o.kind.refills() {
            self.strikes = t.max_strikes as u8;
        }

        let at = o.center;
        o.cooldown = secs_to_ticks(t.object_cooldown);
        events.push(SimEvent::Strike {
            at,
            dir,
            kind: o.kind,
        });
        match o.kind {
            StrikeKind::Fragile => {
                o.broken = secs_to_ticks(t.fragile_respawn);
                events.push(SimEvent::Break { at });
            }
            StrikeKind::Beetle => {
                o.stun = secs_to_ticks(t.beetle_stun);
                events.push(SimEvent::EnemyDown { at, kind: o.kind });
            }
            StrikeKind::Bat => {
                o.dead = true;
                events.push(SimEvent::EnemyDown { at, kind: o.kind });
            }
            _ => {}
        }
        Some(Hit { dir })
    }
}
