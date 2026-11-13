//! Solver de salas: búsqueda en haz sobre la simulación determinista.
//!
//! Encuentra una secuencia de entradas que lleva desde el punto de aparición
//! hasta la salida sin morir y la guarda como repetición. Sirve para
//! comprobar que una sala se puede pasar con las constantes actuales (no
//! que sea divertida ni que la ruta sea la prevista).
//!
//! ```text
//! cargo run --release -p crock-solver -- assets/levels/c1-01-entrada.tmj \
//!     [--out=ruta.rep] [--beam=600] [--secs=20] [--tuning=assets/tuning.cfg]
//! ```

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::path::Path;
use std::time::Instant;

use crock_sim::{
    Input, SIM_HZ, SimEvent, TILE, Tile, TileMap, Tuning, World, ZoneKind, replay, step,
};

/// Ticks entre decisiones (30 decisiones por segundo).
const DECISION_TICKS: usize = 8;
/// Máximo de estados del haz por tile.
const PER_CELL: usize = 4;

#[derive(Clone, Copy, Debug)]
struct Action {
    x: i8,
    /// 0 suelto, 1 pulsar (y mantener), 2 mantener.
    jump: u8,
    strike: Option<(i8, i8)>,
}

impl Action {
    fn inputs(self) -> [Input; DECISION_TICKS] {
        let mut out = [Input::default(); DECISION_TICKS];
        for (i, inp) in out.iter_mut().enumerate() {
            inp.x = self.x;
            inp.jump_held = self.jump != 0;
            inp.jump_pressed = self.jump == 1 && i == 0;
            match self.strike {
                Some((ax, ay)) => {
                    inp.y = ay;
                    inp.aim_x = ax;
                    inp.aim_y = ay;
                    inp.strike_pressed = i == 0;
                }
                None => {
                    inp.aim_x = self.x;
                }
            }
        }
        out
    }
}

fn actions() -> Vec<Action> {
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
    let mut v = Vec::new();
    for x in [-1, 0, 1] {
        for jump in 0..3 {
            v.push(Action {
                x,
                jump,
                strike: None,
            });
            for d in DIRS {
                v.push(Action {
                    x,
                    jump,
                    strike: Some(d),
                });
            }
        }
    }
    v
}

/// Coste (en tiles, 8-conexo) desde cada celda hasta la salida más
/// cercana, sin atravesar sólidos ni pinchos. Subir cuesta más que bajar:
/// así la heurística no premia quedarse pegado debajo de una repisa.
fn distance_field(map: &TileMap) -> Vec<f32> {
    let (w, h) = (map.w, map.h);
    let mut dist = vec![f32::INFINITY; (w * h) as usize];
    // Dijkstra con costes enteros (×100) para usar BinaryHeap.
    let mut heap = BinaryHeap::new();
    for z in map.zones.iter().filter(|z| z.kind == ZoneKind::Exit) {
        for ty in z.y / TILE..=(z.y + z.h - 1) / TILE {
            for tx in z.x / TILE..=(z.x + z.w - 1) / TILE {
                if tx >= 0 && ty >= 0 && tx < w && ty < h {
                    dist[(ty * w + tx) as usize] = 0.0;
                    heap.push(Reverse((0u32, tx, ty)));
                }
            }
        }
    }
    while let Some(Reverse((c, x, y))) = heap.pop() {
        if c as f32 / 100.0 > dist[(y * w + x) as usize] {
            continue;
        }
        for (dx, dy) in [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w || ny >= h || map.tile(nx, ny) != Tile::Empty {
                continue;
            }
            // El jugador iría de (nx,ny) a (x,y): dy < 0 en la búsqueda
            // inversa significa que el jugador baja; dy > 0, que sube.
            let vertical = match dy {
                1 => 250, // subir
                -1 => 60, // bajar
                _ => 0,
            };
            let step = if dx != 0 && dy != 0 { 141 } else { 100 };
            let nc = c + step + vertical;
            let i = (ny * w + nx) as usize;
            if (nc as f32 / 100.0) < dist[i] {
                dist[i] = nc as f32 / 100.0;
                heap.push(Reverse((nc, nx, ny)));
            }
        }
    }
    dist
}

struct Node {
    world: World,
    /// Índice en el historial (para reconstruir la ruta).
    hist: usize,
    score: f32,
}

fn score(w: &World, field: &[f32]) -> f32 {
    let (cx, cy) = w.player.center();
    let (tx, ty) = ((cx / TILE as f32) as i32, (cy / TILE as f32) as i32);
    let d = if tx >= 0 && ty >= 0 && tx < w.map.w && ty < w.map.h {
        field[(ty * w.map.w + tx) as usize]
    } else {
        1e6
    };
    // Menos distancia es mejor; estar de pie y con el golpe cargado son
    // estados más "estables" desde los que se puede intentar de todo.
    d - w.player.strikes as f32 * 0.5 - if w.player.on_ground { 0.5 } else { 0.0 }
}

/// Estado discretizado para la lista de visitados y la deduplicación.
type Key = (i32, i32, i32, i32, u8, bool, bool, u64);

fn key(w: &World) -> Key {
    let p = &w.player;
    // Estado de los objetos que cambia qué se puede hacer (rotos, derribados,
    // aturdidos, en enfriamiento). Las posiciones de enemigos no entran.
    let mut objs = 0u64;
    for (i, o) in w.objects.iter().enumerate().take(16) {
        let bits = (o.broken > 0) as u64
            | ((o.dead as u64) << 1)
            | (((o.stun > 0) as u64) << 2)
            | (((o.cooldown > 0) as u64) << 3);
        objs |= bits << (i * 4);
    }
    // Los enemigos se mueven: su posición aproximada también cuenta (esperar
    // a que un bicho pase es una jugada distinta).
    for o in w.objects.iter().filter(|o| o.hostile()) {
        let cell = ((o.center.0 / 4.0) as u64) ^ (((o.center.1 / 4.0) as u64) << 12);
        objs = objs.rotate_left(13) ^ cell.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }
    (
        p.pos.0.div_euclid(2),
        p.pos.1.div_euclid(2),
        (p.vel.0 / 30.0).round() as i32,
        (p.vel.1 / 30.0).round() as i32,
        p.strikes,
        p.on_ground,
        p.var_jump > 0,
        objs,
    )
}

enum Outcome {
    Alive(Box<World>),
    Dead,
    Done,
}

fn simulate(mut w: World, a: Action) -> Outcome {
    for inp in a.inputs() {
        step(&mut w, &inp);
        let mut done = false;
        for e in w.events.drain(..) {
            match e {
                SimEvent::Death { .. } => return Outcome::Dead,
                SimEvent::RoomEnter { .. } | SimEvent::ChapterComplete => done = true,
                _ => {}
            }
        }
        if done {
            return Outcome::Done;
        }
    }
    Outcome::Alive(Box::new(w))
}

fn main() {
    let mut map_path = None;
    let mut out = None;
    let mut beam = 600usize;
    let mut secs = 20.0f32;
    let mut tuning_path = None;
    for a in std::env::args().skip(1) {
        let (k, v) = a.split_once('=').unwrap_or((a.as_str(), ""));
        match k {
            "--out" => out = Some(v.to_string()),
            "--beam" => beam = v.parse().expect("--beam=N"),
            "--secs" => secs = v.parse().expect("--secs=S"),
            "--tuning" => tuning_path = Some(v.to_string()),
            _ if !a.starts_with("--") => map_path = Some(a.clone()),
            _ => panic!("argumento desconocido: {a}"),
        }
    }
    let map_path =
        map_path.expect("uso: crock-solver sala.tmj [--out=f.rep] [--beam=N] [--secs=S]");
    let (map, warnings) =
        crock_sim::tiled::load_file(Path::new(&map_path)).unwrap_or_else(|e| panic!("{e}"));
    for w in warnings {
        eprintln!("aviso: {w}");
    }
    let tuning = match tuning_path {
        Some(p) => Tuning::parse(&std::fs::read_to_string(p).expect("tuning")).0,
        None => Tuning::default(),
    };
    if !map.zones.iter().any(|z| z.kind == ZoneKind::Exit) {
        eprintln!("la sala no tiene salida");
        std::process::exit(2);
    }
    let field = distance_field(&map);
    let name = map.name.clone();
    let mut start = World::new(map, tuning);
    start.events.clear();

    let acts = actions();
    let mut visited: std::collections::HashSet<Key> = std::collections::HashSet::new();
    let mut history: Vec<(usize, Action)> = vec![(
        usize::MAX,
        Action {
            x: 0,
            jump: 0,
            strike: None,
        },
    )];
    let mut nodes = vec![Node {
        score: score(&start, &field),
        world: start,
        hist: 0,
    }];
    let max_depth = (secs * SIM_HZ as f32 / DECISION_TICKS as f32) as usize;
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let t0 = Instant::now();

    for depth in 0..max_depth {
        // Expandir en paralelo.
        let chunk = nodes.len().div_ceil(threads).max(1);
        let results: Vec<Vec<(usize, Action, Outcome)>> = std::thread::scope(|s| {
            let handles: Vec<_> = nodes
                .chunks(chunk)
                .enumerate()
                .map(|(ci, ns)| {
                    let acts = &acts;
                    s.spawn(move || {
                        let mut out = Vec::new();
                        for (ni, n) in ns.iter().enumerate() {
                            let can_strike = n.world.player.strikes > 0;
                            for &a in acts {
                                if a.strike.is_some() && !can_strike {
                                    continue;
                                }
                                out.push((ci * chunk + ni, a, simulate(n.world.clone(), a)));
                            }
                        }
                        out
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        let mut best: HashMap<Key, Node> = HashMap::new();
        for (parent, a, outcome) in results.into_iter().flatten() {
            match outcome {
                Outcome::Dead => {}
                Outcome::Done => {
                    history.push((nodes[parent].hist, a));
                    let mut route = Vec::new();
                    let mut h = history.len() - 1;
                    while h != 0 {
                        route.push(history[h].1);
                        h = history[h].0;
                    }
                    route.reverse();
                    let inputs: Vec<Input> = route.iter().flat_map(|a| a.inputs()).collect();
                    println!(
                        "{name}: completada en {:.2} s de juego ({} decisiones), búsqueda {:.1} s",
                        inputs.len() as f32 / SIM_HZ as f32,
                        route.len(),
                        t0.elapsed().as_secs_f32()
                    );
                    if let Some(path) = out {
                        std::fs::write(&path, replay::encode(&inputs))
                            .expect("escribir repetición");
                        println!("ruta guardada en {path}");
                    }
                    return;
                }
                Outcome::Alive(w) => {
                    let k = key(&w);
                    if visited.contains(&k) {
                        continue;
                    }
                    let sc = score(&w, &field);
                    let hist = history.len();
                    history.push((nodes[parent].hist, a));
                    let better = best.get(&k).is_none_or(|n| sc < n.score);
                    if better {
                        best.insert(
                            k,
                            Node {
                                world: *w,
                                hist,
                                score: sc,
                            },
                        );
                    }
                }
            }
        }
        let mut all: Vec<Node> = best.into_values().collect();
        all.sort_by(|a, b| a.score.total_cmp(&b.score));
        // 1) Cobertura: el mejor estado de cada tile alcanzado entra siempre
        //    (si no, los estados atascados bajo una repisa expulsan del haz a
        //    los que cogen carrerilla). 2) El resto, por puntuación, con
        //    pocos estados por tile y tipo de movimiento.
        let tile = |n: &Node| {
            let p = &n.world.player;
            (p.pos.0.div_euclid(TILE), p.pos.1.div_euclid(TILE))
        };
        let mut taken = vec![false; all.len()];
        let mut seen_tiles = std::collections::HashSet::new();
        for (i, n) in all.iter().enumerate() {
            if seen_tiles.insert(tile(n)) {
                taken[i] = true;
            }
        }
        let mut per_cell: HashMap<(i32, i32, i32, i32, u8), usize> = HashMap::new();
        let mut budget = beam.saturating_sub(seen_tiles.len());
        for (i, n) in all.iter().enumerate() {
            if budget == 0 {
                break;
            }
            if taken[i] {
                continue;
            }
            let p = &n.world.player;
            let cell = (
                p.pos.0.div_euclid(TILE),
                p.pos.1.div_euclid(TILE),
                (p.vel.1 / 60.0).round() as i32,
                (p.vel.0 / 60.0).round() as i32,
                p.strikes,
            );
            let c = per_cell.entry(cell).or_insert(0);
            if *c < PER_CELL {
                *c += 1;
                taken[i] = true;
                budget -= 1;
            }
        }
        let mut next: Vec<Node> = all
            .into_iter()
            .zip(taken)
            .filter_map(|(n, t)| t.then_some(n))
            .collect();
        next.sort_by(|a, b| a.score.total_cmp(&b.score));
        for n in &next {
            visited.insert(key(&n.world));
        }
        if next.is_empty() {
            println!("{name}: sin salida (todas las ramas mueren) en la decisión {depth}");
            std::process::exit(1);
        }
        if std::env::var_os("SOLVER_DEBUG").is_some() && depth % 30 == 0 {
            for n in next.iter().take(8) {
                let p = &n.world.player;
                eprintln!(
                    "    {:6.2} pos {:?} vel ({:.0},{:.0}) suelo {}",
                    n.score, p.pos, p.vel.0, p.vel.1, p.on_ground
                );
            }
            let maxx = next.iter().map(|n| n.world.player.pos.0).max().unwrap();
            eprintln!("    x máx {maxx}");
        }
        if depth % 30 == 0 {
            let p = &next[0].world.player;
            eprintln!(
                "  t={:5.1}s  mejor distancia {:5.1}  pos {:?}  estados {}",
                depth as f32 * DECISION_TICKS as f32 / SIM_HZ as f32,
                next[0].score,
                p.pos,
                next.len()
            );
        }
        nodes = next;
    }
    println!("{name}: no encontrada en {secs} s de juego");
    std::process::exit(1);
}
