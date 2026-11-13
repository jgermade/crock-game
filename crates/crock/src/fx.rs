//! Partículas: puramente visuales, fuera de la simulación determinista.
//! Se alimentan de los [`SimEvent`] que emite la simulación.

use crock_sim::{SimEvent, StrikeKind, unit};
use sdl3::pixels::Color;
use sdl3::render::WindowCanvas;

use crate::render::View;

struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    gravity: f32,
    life: f32,
    max_life: f32,
    size: f32,
    color: (u8, u8, u8),
}

pub struct Fx {
    particles: Vec<Particle>,
    rng: u32,
}

pub fn kind_color(kind: StrikeKind) -> (u8, u8, u8) {
    match kind {
        StrikeKind::Rock => (150, 146, 150),
        StrikeKind::Fragile => (196, 170, 128),
        StrikeKind::Bone => (236, 228, 206),
        StrikeKind::Moss => (110, 168, 84),
        StrikeKind::Beetle => (84, 128, 196),
        StrikeKind::Bat => (132, 96, 156),
    }
}

impl Fx {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(256),
            rng: 0x2545_f491,
        }
    }

    pub fn clear(&mut self) {
        self.particles.clear();
    }

    /// Aleatorio en [0, 1) (xorshift; no hace falta nada mejor aquí).
    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / (1u32 << 24) as f32
    }

    fn burst(&mut self, at: (f32, f32), base: (f32, f32), n: usize, spec: BurstSpec) {
        for _ in 0..n {
            let a = (self.rand() - 0.5) * spec.spread;
            let (s, c) = a.sin_cos();
            let sp = spec.speed * (0.5 + self.rand());
            let (dx, dy) = (base.0 * c - base.1 * s, base.0 * s + base.1 * c);
            let life = spec.life * (0.6 + 0.8 * self.rand());
            let size = if self.rand() < 0.3 { 2.0 } else { 1.0 };
            self.particles.push(Particle {
                x: at.0,
                y: at.1,
                vx: dx * sp,
                vy: dy * sp,
                gravity: spec.gravity,
                life,
                max_life: life,
                size,
                color: spec.color,
            });
        }
    }

    pub fn handle(&mut self, ev: &SimEvent) {
        match *ev {
            SimEvent::Strike { at, dir, kind } => {
                // Esquirlas en la dirección del golpe y un destello hacia atrás.
                let d = unit(dir);
                let color = kind_color(kind);
                self.burst(
                    at,
                    d,
                    10,
                    BurstSpec {
                        speed: 90.0,
                        spread: 1.6,
                        life: 0.3,
                        gravity: 300.0,
                        color,
                    },
                );
                self.burst(
                    at,
                    (-d.0, -d.1),
                    6,
                    BurstSpec {
                        speed: 50.0,
                        spread: 2.4,
                        life: 0.15,
                        gravity: 0.0,
                        color: (255, 240, 200),
                    },
                );
            }
            SimEvent::Break { at } => {
                self.burst(
                    at,
                    (0.0, -1.0),
                    16,
                    BurstSpec {
                        speed: 70.0,
                        spread: 6.0,
                        life: 0.6,
                        gravity: 500.0,
                        color: kind_color(StrikeKind::Fragile),
                    },
                );
            }
            SimEvent::EnemyDown { at, kind } => {
                self.burst(
                    at,
                    (0.0, -1.0),
                    12,
                    BurstSpec {
                        speed: 60.0,
                        spread: 5.0,
                        life: 0.45,
                        gravity: 400.0,
                        color: kind_color(kind),
                    },
                );
            }
            SimEvent::Burn { at } => {
                // Brasas: la porra se quema.
                self.burst(
                    at,
                    (0.0, -1.0),
                    14,
                    BurstSpec {
                        speed: 45.0,
                        spread: 1.4,
                        life: 0.5,
                        gravity: -60.0,
                        color: (255, 150, 50),
                    },
                );
            }
            SimEvent::Death { at } => {
                // Mechones de pelo por todas partes.
                for color in [(86, 56, 36), (126, 86, 54), (226, 168, 112)] {
                    self.burst(
                        at,
                        (0.0, -1.0),
                        10,
                        BurstSpec {
                            speed: 110.0,
                            spread: 6.3,
                            life: 0.5,
                            gravity: 250.0,
                            color,
                        },
                    );
                }
            }
            SimEvent::Respawn | SimEvent::RoomEnter { .. } => self.clear(),
            SimEvent::Whiff
            | SimEvent::ChapterComplete
            | SimEvent::Jump
            | SimEvent::Land { .. } => {}
        }
    }

    pub fn update(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.vy += p.gravity * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
    }

    pub fn draw(&self, canvas: &mut WindowCanvas, view: &View) {
        canvas.set_blend_mode(sdl3::render::BlendMode::Blend);
        for p in &self.particles {
            let a = (p.life / p.max_life).clamp(0.0, 1.0);
            canvas.set_draw_color(Color::RGBA(
                p.color.0,
                p.color.1,
                p.color.2,
                (a * 255.0) as u8,
            ));
            let _ =
                canvas.fill_rect(view.rect(p.x - p.size / 2.0, p.y - p.size / 2.0, p.size, p.size));
        }
        canvas.set_blend_mode(sdl3::render::BlendMode::None);
    }
}

#[derive(Clone, Copy)]
struct BurstSpec {
    speed: f32,
    spread: f32,
    life: f32,
    gravity: f32,
    color: (u8, u8, u8),
}
