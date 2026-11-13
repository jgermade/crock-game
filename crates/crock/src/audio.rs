//! Audio de baja latencia: efectos sintetizados al arrancar y un mezclador
//! que corre en el hilo de audio de SDL.
//!
//! El hilo principal nunca espera al de audio: le manda órdenes por un canal
//! y el mezclador las recoge al principio de cada callback. Con un buffer de
//! 256 muestras a 48 kHz, un sonido suena ~5 ms después de pedirlo.

use std::f32::consts::TAU;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use crock_sim::{SimEvent, StrikeKind};
use sdl3::audio::{AudioCallback, AudioFormat, AudioSpec, AudioStream, AudioStreamWithCallback};

const RATE: u32 = 48_000;
/// Voces simultáneas como máximo (las más viejas se descartan).
const MAX_VOICES: usize = 24;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(usize)]
pub enum Sfx {
    Jump,
    Land,
    HitRock,
    HitBone,
    HitMoss,
    HitBug,
    Whiff,
    Break,
    EnemyDown,
    Burn,
    Death,
    Room,
    Complete,
    MenuMove,
    MenuSelect,
}

const COUNT: usize = Sfx::MenuSelect as usize + 1;

enum Cmd {
    Play(usize, f32),
    Volume(f32),
}

struct Voice {
    data: Arc<[f32]>,
    pos: usize,
    gain: f32,
}

struct Mixer {
    rx: Receiver<Cmd>,
    bank: Vec<Arc<[f32]>>,
    voices: Vec<Voice>,
    volume: f32,
    out: Vec<f32>,
}

impl AudioCallback<f32> for Mixer {
    fn callback(&mut self, stream: &mut AudioStream, requested: i32) {
        while let Ok(cmd) = self.rx.try_recv() {
            match cmd {
                Cmd::Play(i, gain) => {
                    if self.voices.len() >= MAX_VOICES {
                        self.voices.remove(0);
                    }
                    self.voices.push(Voice {
                        data: self.bank[i].clone(),
                        pos: 0,
                        gain,
                    });
                }
                Cmd::Volume(v) => self.volume = v,
            }
        }
        let n = requested.max(0) as usize;
        self.out.clear();
        self.out.resize(n, 0.0);
        for v in &mut self.voices {
            let take = (v.data.len() - v.pos).min(n);
            for (o, s) in self.out[..take]
                .iter_mut()
                .zip(&v.data[v.pos..v.pos + take])
            {
                *o += s * v.gain;
            }
            v.pos += take;
        }
        self.voices.retain(|v| v.pos < v.data.len());
        for o in &mut self.out {
            // Saturación suave: nunca recorta de golpe.
            *o = (*o * self.volume).tanh();
        }
        let _ = stream.put_data_f32(&self.out);
    }
}

pub struct Audio {
    tx: Sender<Cmd>,
    _stream: Option<AudioStreamWithCallback<Mixer>>,
}

impl Audio {
    /// Abre el dispositivo de audio. Si falla, el juego sigue en silencio.
    pub fn new(sdl: &sdl3::Sdl, volume: f32) -> Self {
        let (tx, rx) = channel();
        // Buffer pequeño: menos latencia (lo respeta el backend si puede).
        sdl3::hint::set("SDL_AUDIO_DEVICE_SAMPLE_FRAMES", "256");
        let stream = sdl.audio().ok().and_then(|audio| {
            let spec = AudioSpec {
                freq: Some(RATE as i32),
                channels: Some(1),
                format: Some(AudioFormat::f32_sys()),
            };
            let mixer = Mixer {
                rx,
                bank: bank(),
                voices: Vec::with_capacity(MAX_VOICES),
                volume,
                out: Vec::with_capacity(4096),
            };
            match audio.open_playback_stream(&spec, mixer) {
                Ok(s) => {
                    let _ = s.resume();
                    Some(s)
                }
                Err(e) => {
                    eprintln!("[crock] sin audio: {e}");
                    None
                }
            }
        });
        Self {
            tx,
            _stream: stream,
        }
    }

    pub fn play(&self, sfx: Sfx, gain: f32) {
        let _ = self.tx.send(Cmd::Play(sfx as usize, gain));
    }

    pub fn set_volume(&self, v: f32) {
        let _ = self.tx.send(Cmd::Volume(v));
    }

    /// Sonidos para los eventos de la simulación.
    pub fn on_event(&self, ev: &SimEvent) {
        match *ev {
            SimEvent::Jump => self.play(Sfx::Jump, 0.5),
            SimEvent::Land { speed } if speed > 60.0 => {
                self.play(Sfx::Land, (speed / 240.0).clamp(0.25, 0.8))
            }
            SimEvent::Strike { kind, .. } => self.play(
                match kind {
                    StrikeKind::Rock | StrikeKind::Fragile => Sfx::HitRock,
                    StrikeKind::Bone => Sfx::HitBone,
                    StrikeKind::Moss => Sfx::HitMoss,
                    StrikeKind::Beetle | StrikeKind::Bat => Sfx::HitBug,
                },
                0.9,
            ),
            SimEvent::Whiff => self.play(Sfx::Whiff, 0.45),
            SimEvent::Grab { .. } => self.play(Sfx::Land, 0.4),
            SimEvent::Break { .. } => self.play(Sfx::Break, 0.7),
            SimEvent::EnemyDown { .. } => self.play(Sfx::EnemyDown, 0.6),
            SimEvent::Burn { .. } => self.play(Sfx::Burn, 0.6),
            SimEvent::Death { .. } => self.play(Sfx::Death, 0.8),
            SimEvent::RoomEnter { .. } => self.play(Sfx::Room, 0.6),
            SimEvent::ChapterComplete => self.play(Sfx::Complete, 0.7),
            _ => {}
        }
    }
}

// --- Síntesis ------------------------------------------------------------------

/// Generador de ruido determinista (xorshift).
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 23) as f32 - 1.0
    }
}

fn len(secs: f32) -> usize {
    (secs * RATE as f32) as usize
}

/// Envolvente: ataque lineal y caída exponencial.
fn env(i: usize, n: usize, attack: f32, curve: f32) -> f32 {
    let t = i as f32 / n as f32;
    let a = (i as f32 / (attack * RATE as f32).max(1.0)).min(1.0);
    a * (1.0 - t).powf(curve)
}

/// Tono con barrido de frecuencia (exponencial) y forma de onda.
fn sweep(secs: f32, f0: f32, f1: f32, square: f32, curve: f32) -> Vec<f32> {
    let n = len(secs);
    let mut phase = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let f = f0 * (f1 / f0).powf(t);
            phase = (phase + f / RATE as f32) % 1.0;
            let sine = (phase * TAU).sin();
            let sq = if phase < 0.5 { 1.0 } else { -1.0 };
            (sine * (1.0 - square) + sq * square * 0.6) * env(i, n, 0.002, curve)
        })
        .collect()
}

/// Ruido filtrado (paso bajo de un polo; `cut` en 0..1) con envolvente.
fn noise(secs: f32, cut0: f32, cut1: f32, seed: u32, attack: f32, curve: f32) -> Vec<f32> {
    let n = len(secs);
    let mut r = Noise(seed);
    let mut y = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let k = cut0 + (cut1 - cut0) * t;
            y += (r.next() - y) * k;
            y * env(i, n, attack, curve) * 1.6
        })
        .collect()
}

fn mix(parts: &[(&[f32], f32, usize)]) -> Vec<f32> {
    let n = parts
        .iter()
        .map(|(p, _, off)| p.len() + off)
        .max()
        .unwrap_or(0);
    let mut out = vec![0.0; n];
    for (p, g, off) in parts {
        for (i, s) in p.iter().enumerate() {
            out[i + off] += s * g;
        }
    }
    out
}

fn bank() -> Vec<Arc<[f32]>> {
    let mut b: Vec<Vec<f32>> = vec![Vec::new(); COUNT];
    let thud = sweep(0.14, 140.0, 45.0, 0.0, 2.0);
    b[Sfx::Jump as usize] = sweep(0.09, 260.0, 520.0, 0.35, 1.5);
    b[Sfx::Land as usize] = mix(&[
        (&sweep(0.07, 110.0, 60.0, 0.0, 2.0), 0.7, 0),
        (&noise(0.05, 0.25, 0.08, 7, 0.001, 2.0), 0.5, 0),
    ]);
    b[Sfx::HitRock as usize] = mix(&[
        (&thud, 1.0, 0),
        (&noise(0.06, 0.6, 0.1, 11, 0.0005, 2.5), 0.8, 0),
    ]);
    b[Sfx::HitBone as usize] = mix(&[
        (&sweep(0.08, 900.0, 600.0, 0.2, 3.0), 0.6, 0),
        (&sweep(0.12, 1400.0, 1300.0, 0.0, 4.0), 0.3, 0),
        (&noise(0.03, 0.8, 0.4, 13, 0.0005, 3.0), 0.5, 0),
    ]);
    b[Sfx::HitMoss as usize] = mix(&[
        (&sweep(0.12, 90.0, 40.0, 0.0, 1.5), 0.8, 0),
        (&noise(0.1, 0.1, 0.03, 17, 0.004, 1.5), 0.7, 0),
    ]);
    b[Sfx::HitBug as usize] = mix(&[
        (&thud, 0.7, 0),
        (&noise(0.05, 0.9, 0.5, 19, 0.0005, 2.0), 0.6, 0),
        (&sweep(0.06, 700.0, 300.0, 0.5, 2.0), 0.25, 0),
    ]);
    b[Sfx::Whiff as usize] = noise(0.13, 0.05, 0.35, 23, 0.04, 1.2);
    b[Sfx::Break as usize] = mix(&[
        (&thud, 0.6, 0),
        (&noise(0.28, 0.7, 0.2, 29, 0.0005, 2.0), 0.9, 0),
        (&noise(0.1, 0.9, 0.6, 31, 0.0005, 3.0), 0.5, len(0.06)),
    ]);
    b[Sfx::EnemyDown as usize] = sweep(0.16, 900.0, 220.0, 0.45, 1.3);
    b[Sfx::Burn as usize] = noise(0.35, 0.9, 0.5, 37, 0.01, 1.0);
    b[Sfx::Death as usize] = mix(&[
        (&sweep(0.35, 420.0, 70.0, 0.5, 1.2), 0.6, 0),
        (&noise(0.2, 0.5, 0.1, 41, 0.001, 2.0), 0.6, 0),
    ]);
    b[Sfx::Room as usize] = mix(&[
        (&sweep(0.12, 523.0, 523.0, 0.2, 2.0), 0.4, 0),
        (&sweep(0.18, 784.0, 784.0, 0.2, 2.0), 0.4, len(0.08)),
    ]);
    b[Sfx::Complete as usize] = mix(&[
        (&sweep(0.15, 523.0, 523.0, 0.3, 2.0), 0.4, 0),
        (&sweep(0.15, 659.0, 659.0, 0.3, 2.0), 0.4, len(0.12)),
        (&sweep(0.15, 784.0, 784.0, 0.3, 2.0), 0.4, len(0.24)),
        (&sweep(0.4, 1046.0, 1046.0, 0.3, 1.5), 0.4, len(0.36)),
    ]);
    b[Sfx::MenuMove as usize] = sweep(0.03, 660.0, 660.0, 0.5, 2.0);
    b[Sfx::MenuSelect as usize] = sweep(0.08, 660.0, 990.0, 0.5, 1.5);
    b.into_iter().map(Arc::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bank_is_complete_and_bounded() {
        let b = bank();
        assert_eq!(b.len(), COUNT);
        for (i, s) in b.iter().enumerate() {
            assert!(!s.is_empty(), "sonido {i} vacío");
            assert!(s.len() < RATE as usize, "sonido {i} demasiado largo");
            assert!(
                s.iter().all(|v| v.is_finite() && v.abs() < 3.0),
                "sonido {i} fuera de rango"
            );
        }
    }
}
