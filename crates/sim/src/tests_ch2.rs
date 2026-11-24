//! Capítulo 2: corrientes de aire, colmillo (doble carga) y propiedad de
//! sala `strikes`.

use super::*;

const NONE: Input = Input {
    x: 0,
    y: 0,
    jump_held: false,
    jump_pressed: false,
    strike_pressed: false,
    aim_x: 0,
    aim_y: 0,
};

fn mv(x: i8, y: i8) -> Input {
    Input { x, y, ..NONE }
}

fn strike(aim: (i8, i8)) -> Input {
    Input {
        x: aim.0,
        y: aim.1,
        strike_pressed: true,
        aim_x: aim.0,
        aim_y: aim.1,
        ..NONE
    }
}

fn world(src: &str) -> World {
    World::new(TileMap::parse(src).unwrap(), Tuning::default())
}

fn run(w: &mut World, ticks: u32, input: Input) {
    for _ in 0..ticks {
        step(w, &input);
    }
}

#[test]
fn wind_lifts_and_holding_down_cancels_it() {
    let src = "
#####
#...#
#...#
#...#
#...#
#...#
#...#
#.u.#
#.u.#
#.u.#
#.P.#
#####
";
    let mut w = world(src);
    let y0 = w.player.pos.1;
    run(&mut w, SIM_HZ / 4, NONE);
    assert!(w.player.pos.1 < y0 - 8, "la corriente lo levanta");
    // Sube hasta salir por arriba de la corriente y se queda flotando
    // alrededor de su borde, sin pasar del techo ni caer al suelo.
    run(&mut w, SIM_HZ * 2, NONE);
    let top = 7 * TILE;
    assert!(
        (w.player.pos.1 + Player::H - top).abs() < 12,
        "flota en el borde de la corriente: {}",
        w.player.pos.1
    );
    // Manteniendo abajo cae hasta el suelo.
    run(&mut w, SIM_HZ, mv(0, 1));
    assert_eq!(w.player.pos.1, y0);
    assert!(w.player.on_ground);
}

#[test]
fn tusk_grants_a_second_strike_for_the_chapter() {
    let a = TileMap::parse(
        "
#########
#.......#
#P.t...E#
#########
",
    )
    .unwrap();
    let b = TileMap::parse(
        "
#########
#.......#
#P......#
#########
",
    )
    .unwrap();
    let mut w = World::with_rooms(vec![a, b], Tuning::default());
    assert_eq!(w.player.strikes, 1);
    assert!(w.tusk_available());
    run(&mut w, SIM_HZ / 2, mv(1, 0));
    assert!(!w.tusk_available());
    assert!(
        w.events
            .iter()
            .any(|e| matches!(e, SimEvent::Pickup { .. }))
    );
    assert_eq!(w.player.strikes, 2);
    // Sobrevive a la muerte y al cambio de sala.
    w.die();
    assert_eq!(w.player.strikes, 2);
    w.goto_room(1);
    assert_eq!((w.player.max_strikes, w.player.strikes), (2, 2));
}

#[test]
fn two_strikes_chain_two_bones() {
    // Dos huesos (no recargan) uno encima de otro: con una carga sólo se
    // golpea el primero; con dos, los dos.
    let src = "
;! strikes=2
#######
#.....#
#.....#
#.....#
#.....#
#.....#
#.....#
#.....#
#.....#
#.....#
#.....#
#.....#
#.....#
#.....#
#.....#
#.....#
#..P..#
#######
";
    let mut w = world(src);
    assert_eq!(w.player.strikes, 2);
    run(&mut w, SIM_HZ / 4, NONE);
    let feet = w.player.pos.1;
    w.objects
        .push(Strikeable::new(StrikeKind::Bone, (28.0, feet as f32 + 4.0)));
    step(&mut w, &strike((0, 1)));
    run(&mut w, 2, NONE);
    let bone_hits = |w: &World| {
        w.events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    SimEvent::Strike {
                        kind: StrikeKind::Bone,
                        ..
                    }
                )
            })
            .count()
    };
    assert_eq!(bone_hits(&w), 1);
    assert_eq!(w.player.strikes, 1, "el hueso no recarga, queda una");
    // Otro hueso al subir: segundo golpe.
    run(&mut w, SIM_HZ / 10, NONE);
    let (cx, cy) = w.player.center();
    w.objects
        .push(Strikeable::new(StrikeKind::Bone, (cx, cy + 9.0)));
    step(&mut w, &strike((0, 1)));
    run(&mut w, 2, NONE);
    assert_eq!(bone_hits(&w), 2);
    assert_eq!(w.player.strikes, 0);
}

#[test]
fn room_property_loads_from_ascii_and_tiled() {
    let m = TileMap::parse(";! strikes=2\n#P#\n###\n").unwrap();
    assert_eq!(m.strikes, 2);
    assert!(TileMap::parse(";! dragones=2\n#P#\n###\n").is_err());
}

#[test]
fn striking_the_floor_launches_up_without_refilling() {
    let mut w = world(
        "
#####
#...#
#...#
#...#
#...#
#...#
#...#
#...#
#.P.#
#####
",
    );
    run(&mut w, SIM_HZ / 4, NONE);
    let y0 = w.player.pos.1;
    step(&mut w, &strike((0, 1)));
    assert!(w.events.iter().any(|e| matches!(
        e,
        SimEvent::Strike {
            kind: StrikeKind::Terrain,
            ..
        }
    )));
    assert!(w.player.vel.1 < -150.0, "sale hacia arriba");
    assert_eq!(w.player.strikes, 0, "el terreno no recarga");
    run(&mut w, SIM_HZ / 5, NONE);
    assert!(w.player.pos.1 < y0 - 24, "más alto que un salto");
}

#[test]
fn striking_a_wall_pushes_away_and_whiffs_in_open_air() {
    let mut w = world(
        "
#########
#.......#
#.......#
#.......#
#.......#
#P......#
#########
",
    );
    run(&mut w, SIM_HZ / 4, NONE);
    // Saltar y golpear la pared de la izquierda: sale hacia la derecha.
    step(
        &mut w,
        &Input {
            jump_pressed: true,
            jump_held: true,
            ..NONE
        },
    );
    run(
        &mut w,
        10,
        Input {
            jump_held: true,
            ..NONE
        },
    );
    step(&mut w, &strike((-1, 0)));
    assert!(w.player.vel.0 > 150.0, "{:?}", w.player.vel);
    // En el aire, lejos de todo: golpe al aire.
    let mut w = world(
        "
#########
#.......#
#.......#
#.......#
#.......#
#...P...#
#########
",
    );
    run(&mut w, SIM_HZ / 4, NONE);
    step(
        &mut w,
        &Input {
            jump_pressed: true,
            jump_held: true,
            ..NONE
        },
    );
    run(
        &mut w,
        12,
        Input {
            jump_held: true,
            ..NONE
        },
    );
    w.events.clear();
    step(&mut w, &strike((1, 0)));
    run(&mut w, SIM_HZ / 5, NONE);
    assert!(w.events.iter().any(|e| matches!(e, SimEvent::Whiff)));
}
