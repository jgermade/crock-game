//! cROCK — simulación determinista, independiente de la plataforma.
//!
//! Todo el estado del juego vive en [`World`] y avanza con [`step`] a un
//! paso fijo de [`DT`] segundos (`SIM_HZ` ticks por segundo). Nada aquí
//! depende de SDL, del reloj ni de hilos: con la misma entrada se obtiene
//! siempre el mismo resultado, lo que permite tests, repeticiones y ajuste
//! en caliente de constantes.

mod map;
mod player;
pub mod replay;
mod strike;
pub mod tiled;
mod tuning;

use std::sync::Arc;

pub use map::{Dir4, ObjectDef, TILE, Tile, TileMap, Zone, ZoneKind};
pub use player::{Aabb, Player};
pub use strike::{SimEvent, StrikeKind, Strikeable, unit};
pub use tuning::Tuning;

/// Frecuencia de simulación (ticks por segundo).
pub const SIM_HZ: u32 = 240;
/// Paso fijo de simulación en segundos.
pub const DT: f32 = 1.0 / SIM_HZ as f32;
/// Paso fijo de simulación en nanosegundos (para el bucle de la plataforma).
pub const DT_NS: u64 = 1_000_000_000 / SIM_HZ as u64;

/// Convierte segundos a ticks enteros de simulación (redondeando).
pub fn secs_to_ticks(s: f32) -> u32 {
    (s * SIM_HZ as f32).round().max(0.0) as u32
}

/// Entrada de un tick. La plataforma la construye justo antes de simular.
///
/// `*_pressed` son flancos: `true` sólo en el primer tick tras la pulsación,
/// aunque la pulsación y la suelta hayan ocurrido dentro del mismo frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    /// Dirección horizontal digital: -1, 0 o 1.
    pub x: i8,
    /// Dirección vertical digital: -1 (arriba), 0 o 1 (abajo).
    pub y: i8,
    pub jump_held: bool,
    pub jump_pressed: bool,
    pub strike_pressed: bool,
    /// Dirección de apuntado del golpe, capturada en el instante de pulsar
    /// (puede diferir de `x, y` si se soltó la dirección dentro del frame).
    pub aim_x: i8,
    pub aim_y: i8,
}

/// Estado completo del juego.
#[derive(Clone, Debug)]
pub struct World {
    /// Salas del capítulo, en orden (datos inmutables y compartidos: copiar
    /// un `World` es barato). `map` es la sala actual.
    pub rooms: Arc<[Arc<TileMap>]>,
    pub room: usize,
    pub map: Arc<TileMap>,
    pub player: Player,
    pub tuning: Tuning,
    pub objects: Vec<Strikeable>,
    pub tick: u64,
    /// Muertes acumuladas (en todo el capítulo).
    pub deaths: u32,
    /// Ticks restantes de hit-stop (la simulación del jugador se congela).
    pub hitstop: u32,
    /// Ticks restantes de temblor de pantalla y su dirección.
    pub shake: u32,
    pub shake_dir: (i8, i8),
    /// Eventos pendientes de que la plataforma los drene.
    pub events: Vec<SimEvent>,
}

impl World {
    /// Mundo de una sola sala.
    pub fn new(map: TileMap, tuning: Tuning) -> Self {
        Self::with_rooms(vec![map], tuning)
    }

    /// Mundo con varias salas que se recorren en orden por sus salidas.
    pub fn with_rooms(rooms: Vec<TileMap>, tuning: Tuning) -> Self {
        assert!(!rooms.is_empty(), "hace falta al menos una sala");
        let rooms: Arc<[Arc<TileMap>]> = rooms.into_iter().map(Arc::new).collect();
        let map = rooms[0].clone();
        let mut w = Self {
            player: Player::spawn(map.spawn),
            rooms,
            room: 0,
            map,
            tuning,
            objects: Vec::new(),
            tick: 0,
            deaths: 0,
            hitstop: 0,
            shake: 0,
            shake_dir: (0, 0),
            events: Vec::new(),
        };
        w.restart();
        w
    }

    /// Salta a una sala concreta (empezando en su punto de aparición).
    pub fn goto_room(&mut self, room: usize) {
        self.room = room % self.rooms.len();
        self.map = self.rooms[self.room].clone();
        self.restart();
        self.events.push(SimEvent::RoomEnter { room: self.room });
    }

    /// Reinicio instantáneo de la sala: jugador y objetos a su estado
    /// inicial, sin transición.
    pub fn restart(&mut self) {
        self.player = Player::spawn(self.map.spawn);
        self.objects = self.map.objects.iter().map(Strikeable::from_def).collect();
        self.hitstop = 0;
        self.shake = 0;
        self.events.push(SimEvent::Respawn);
    }

    /// Muerte: se cuenta, se avisa y se reinicia la sala en el mismo tick.
    pub fn die(&mut self) {
        let at = self.player.center();
        self.deaths += 1;
        self.restart();
        self.events.push(SimEvent::Death { at });
    }

    /// Hash barato del estado relevante, para tests de determinismo.
    pub fn state_hash(&self) -> u64 {
        let p = &self.player;
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |v: u64| {
            h ^= v;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        };
        mix(self.tick);
        mix(self.room as u64);
        mix(self.deaths as u64);
        mix(p.pos.0 as u64);
        mix(p.pos.1 as u64);
        mix(p.rem.0.to_bits() as u64);
        mix(p.rem.1.to_bits() as u64);
        mix(p.vel.0.to_bits() as u64);
        mix(p.vel.1.to_bits() as u64);
        mix(p.strikes as u64);
        for o in &self.objects {
            mix(o.cooldown as u64 | (o.broken as u64) << 32);
            mix(o.center.0.to_bits() as u64 | (o.center.1.to_bits() as u64) << 32);
            mix(o.stun as u64 | (o.dead as u64) << 32);
        }
        h
    }
}

/// Avanza la simulación exactamente un tick de [`DT`] segundos.
pub fn step(world: &mut World, input: &Input) {
    world.tick += 1;

    for o in &mut world.objects {
        o.tick_timers();
    }
    world.shake = world.shake.saturating_sub(1);

    if world.hitstop > 0 {
        // Congelado, pero sin perder entradas: un salto o golpe pulsado
        // durante el hit-stop se ejecuta en cuanto termina.
        world.hitstop -= 1;
        world.player.buffer_during_hitstop(&world.tuning, input);
        return;
    }

    for o in &mut world.objects {
        o.behave(&world.map, &world.tuning);
    }

    let t = &world.tuning;
    if let Some(hit) =
        world
            .player
            .update(&world.map, t, input, &mut world.objects, &mut world.events)
    {
        world.hitstop = secs_to_ticks(t.hitstop);
        world.shake = secs_to_ticks(t.shake_time);
        world.shake_dir = hit.dir;
    }

    let p = &world.player;
    let (x, y, w, h) = (p.pos.0, p.pos.1, Player::W, Player::H);

    // Peligros: pinchos, enemigos y caer al vacío. El golpe se resuelve
    // antes, así que golpear a un enemigo justo al tocarlo salva la vida.
    let fell = y > world.map.height_px() + 16;
    let spiked = world.map.spike_hit(x, y, w, h, p.vel);
    // Caja de daño de los enemigos 1 px más pequeña por cada lado: perdona
    // los roces.
    let bitten = world.objects.iter().any(|o| {
        let (x0, y0, x1, y1) = o.bounds();
        o.hostile()
            && (x as f32) < x1 - 1.0
            && x0 + 1.0 < (x + w) as f32
            && (y as f32) < y1 - 1.0
            && y0 + 1.0 < (y + h) as f32
    });
    if fell || spiked || bitten {
        world.die();
        return;
    }

    // Zonas.
    let mut in_fire = false;
    let mut at_exit = false;
    for z in &world.map.zones {
        if z.overlaps(x, y, w, h) {
            match z.kind {
                ZoneKind::Fire => in_fire = true,
                ZoneKind::Exit => at_exit = true,
            }
        }
    }
    world.player.set_burning(in_fire, &mut world.events);

    if at_exit {
        if world.room + 1 >= world.rooms.len() {
            world.events.push(SimEvent::ChapterComplete);
        }
        world.goto_room(world.room + 1);
    }
}

#[cfg(test)]
mod test_util;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_replays;
#[cfg(test)]
mod tests_rooms;
#[cfg(test)]
mod tests_strike;
