//! Las rutas de `assets/replays/` completan sus salas con las constantes de
//! `assets/tuning.cfg`. Si este test falla tras cambiar constantes o editar
//! una sala, regenera las rutas con `tools/solve_all.sh` (y si el solver no
//! encuentra ruta, la sala ha dejado de poder pasarse).

use std::path::Path;

use crate::{SimEvent, Tuning, World, replay, step};

fn assets() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn rooms() -> Vec<String> {
    let list = std::fs::read_to_string(assets().join("levels/capitulo1.txt")).unwrap();
    let mut v: Vec<String> = list
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.trim_end_matches(".tmj").to_string())
        .collect();
    v.push("golpe".into());
    v.push("cornisas".into());
    v
}

#[test]
fn shipped_replays_complete_their_rooms() {
    let (tuning, warns) =
        Tuning::parse(&std::fs::read_to_string(assets().join("tuning.cfg")).unwrap());
    assert!(warns.is_empty(), "{warns:?}");
    for name in rooms() {
        let map = crate::test_util::level(&name);
        let src = std::fs::read_to_string(assets().join(format!("replays/{name}.rep")))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let inputs = replay::decode(&src).unwrap();
        let mut w = World::new(map, tuning.clone());
        let mut done = false;
        for inp in &inputs {
            step(&mut w, inp);
            for e in w.events.drain(..) {
                match e {
                    SimEvent::Death { .. } => {
                        panic!("{name}: la ruta muere (regenera con tools/solve_all.sh)")
                    }
                    SimEvent::ChapterComplete | SimEvent::RoomEnter { .. } => done = true,
                    _ => {}
                }
            }
        }
        assert!(
            done,
            "{name}: la ruta no llega a la salida (regenera con tools/solve_all.sh)"
        );
    }
}

#[test]
fn chapter_rooms_load_cleanly() {
    for name in rooms() {
        let m = crate::test_util::level(&name);
        assert!(
            m.zones.iter().any(|z| z.kind == crate::ZoneKind::Exit),
            "{name}: sin salida"
        );
    }
}
