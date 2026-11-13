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

fn mv(x: i8) -> Input {
    Input { x, ..NONE }
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

fn died(w: &mut World) -> bool {
    let d = w.events.iter().any(|e| matches!(e, SimEvent::Death { .. }));
    w.events.clear();
    d
}

#[test]
fn landing_on_spikes_kills_and_restarts() {
    let mut w = world(
        "
#######
#.....#
#P....#
##^^^##
",
    );
    run(&mut w, SIM_HZ / 2, NONE);
    assert!(!died(&mut w));
    let mut killed = false;
    for _ in 0..SIM_HZ {
        step(&mut w, &mv(1));
        if died(&mut w) {
            killed = true;
            break;
        }
    }
    assert!(killed);
    assert_eq!(w.deaths, 1);
    let spawn = Player::spawn(w.map.spawn);
    assert!(
        (w.player.pos.0 - spawn.pos.0).abs() <= 2,
        "reapareció en el inicio"
    );
}

#[test]
fn spikes_only_kill_moving_against_them() {
    let m = TileMap::parse(
        "
#####
#.P.#
#.^.#
#####
",
    )
    .unwrap();
    // Caja que solapa la mitad letal de los pinchos que apuntan arriba.
    let (x, y) = (2 * TILE, 2 * TILE + 5);
    assert!(
        m.spike_hit(x, y, 8, 3, (0.0, 10.0)),
        "cayendo encima: muere"
    );
    assert!(m.spike_hit(x, y, 8, 3, (0.0, 0.0)), "quieto encima: muere");
    assert!(!m.spike_hit(x, y, 8, 3, (0.0, -10.0)), "subiendo: no muere");
    assert!(
        !m.spike_hit(x, 2 * TILE, 8, 3, (0.0, 10.0)),
        "mitad superior: no letal"
    );
}

const BEETLE_ROOM: &str = "
############
#..........#
#P.........#
#####..e...#
############
";

#[test]
fn beetle_patrols_and_turns_at_edges() {
    let mut w = world(
        "
##########
#P.......#
#........#
#...e....#
#..####..#
##########
",
    );
    let x0 = w.objects[0].center.0;
    let mut min_x = x0;
    let mut max_x = x0;
    for _ in 0..SIM_HZ * 6 {
        step(&mut w, &NONE);
        let x = w.objects[0].center.0;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
    }
    // Plataforma de x=24 a x=56: nunca se sale de ella.
    assert!(
        min_x - 4.0 >= 24.0 - 0.01 && max_x + 4.0 <= 56.0 + 0.01,
        "{min_x}..{max_x}"
    );
    assert!(max_x - min_x > 20.0, "patrulla de verdad");
}

#[test]
fn touching_beetle_kills() {
    let mut w = world(BEETLE_ROOM);
    let mut killed = false;
    for _ in 0..SIM_HZ * 4 {
        step(&mut w, &mv(1));
        if died(&mut w) {
            killed = true;
            break;
        }
    }
    assert!(killed);
}

#[test]
fn striking_beetle_stuns_it_and_makes_it_safe() {
    let mut w = world(BEETLE_ROOM);
    // Golpear a la derecha en cuanto esté a tiro, y seguir andando encima.
    let mut struck = false;
    for _ in 0..SIM_HZ * 3 {
        let near = (w.objects[0].center.0 - w.player.center().0).abs() < 14.0;
        let inp = if near && !struck {
            strike((1, 0))
        } else {
            mv(1)
        };
        step(&mut w, &inp);
        struck |= w.objects[0].stun > 0;
        assert!(!died(&mut w), "golpear al enemigo a tiempo salva");
        if struck {
            break;
        }
    }
    assert!(struck, "el escarabajo debería quedar aturdido");
    assert!(!w.objects[0].hostile());
    let t = w.tuning.clone();
    // El hit-stop congela a los enemigos, así que cuenta aparte.
    run(
        &mut w,
        secs_to_ticks(t.beetle_stun) + secs_to_ticks(t.hitstop) + 2,
        NONE,
    );
    assert!(w.objects[0].hostile(), "se recupera");
}

#[test]
fn bat_is_downed_by_strike_and_returns_on_restart() {
    let mut w = world(
        "
#########
#.......#
#...P...#
#.......#
#...w...#
#.......#
#########
",
    );
    w.objects[0].range = 0.0; // quieto, para el test
    step(&mut w, &strike((0, 1)));
    assert!(w.objects[0].dead);
    assert!(!w.objects[0].hostile());
    assert!(w.events.iter().any(|e| matches!(
        e,
        SimEvent::EnemyDown {
            kind: StrikeKind::Bat,
            ..
        }
    )));
    assert_eq!(w.player.strikes, 1, "recarga");
    w.restart();
    assert!(!w.objects[0].dead);
}

#[test]
fn fire_burns_the_club() {
    let mut w = world(
        "
#########
#.......#
#P..fff.#
#########
",
    );
    run(&mut w, SIM_HZ / 4, NONE);
    assert_eq!(w.player.strikes, 1);
    let mut burned = false;
    for _ in 0..SIM_HZ {
        step(&mut w, &mv(1));
        if w.player.burning {
            burned = true;
            break;
        }
    }
    assert!(burned);
    assert!(w.events.iter().any(|e| matches!(e, SimEvent::Burn { .. })));
    assert_eq!(
        w.player.strikes, 0,
        "sin golpes dentro del fuego, aun en el suelo"
    );
    run(&mut w, 4, NONE);
    assert_eq!(w.player.strikes, 0);
    // Al salir del fuego y tocar suelo, recarga.
    run(&mut w, SIM_HZ, mv(1));
    assert!(!w.player.burning);
    assert_eq!(w.player.strikes, 1);
}

#[test]
fn exits_chain_rooms_and_complete_chapter() {
    let a = TileMap::parse(
        "
######
#...E#
#P..E#
######
",
    )
    .unwrap();
    let b = TileMap::parse(
        "
######
#E...#
#E..P#
######
",
    )
    .unwrap();
    let mut w = World::with_rooms(vec![a, b], Tuning::default());
    run(&mut w, SIM_HZ, mv(1));
    assert_eq!(w.room, 1);
    assert!(
        w.events
            .iter()
            .any(|e| matches!(e, SimEvent::RoomEnter { room: 1 }))
    );
    w.events.clear();
    run(&mut w, SIM_HZ, mv(-1));
    assert!(
        w.events
            .iter()
            .any(|e| matches!(e, SimEvent::ChapterComplete))
    );
    assert_eq!(w.room, 0, "vuelve a empezar");
}

#[test]
fn death_restores_room_objects() {
    let mut w = world(
        "
#########
#.......#
#...P...#
#.......#
#...x...#
#.......#
##^^^^^##
",
    );
    step(&mut w, &strike((0, 1)));
    assert!(!w.objects[0].active(), "frágil rota");
    // Sin más golpes cae hasta los pinchos.
    let mut killed = false;
    for _ in 0..SIM_HZ * 3 {
        step(&mut w, &NONE);
        if died(&mut w) {
            killed = true;
            break;
        }
    }
    assert!(killed);
    assert!(w.objects[0].active(), "la frágil vuelve tras morir");
}
