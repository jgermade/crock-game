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

fn ticks(s: f32) -> u32 {
    secs_to_ticks(s)
}

/// Jugador en el aire con un objeto `obj` justo debajo.
fn above(obj: char) -> World {
    world(&format!(
        "
#########
#.......#
#.......#
#...P...#
#.......#
#...{obj}...#
#.......#
#.......#
#########
"
    ))
}

#[test]
fn strike_down_launches_straight_up() {
    let mut w = above('o');
    step(&mut w, &strike((0, 1)));
    let t = w.tuning.clone();
    assert_eq!(w.player.vel, (0.0, -t.strike_speed));
    assert!(w.events.iter().any(|e| matches!(
        e,
        SimEvent::Strike {
            kind: StrikeKind::Rock,
            ..
        }
    )));
    assert_eq!(w.player.strikes, 1, "la roca recarga el golpe");
}

#[test]
fn hitstop_freezes_then_floats() {
    let mut w = above('o');
    step(&mut w, &strike((0, 1)));
    let t = w.tuning.clone();
    let frozen = w.player.exact_pos();
    assert_eq!(w.hitstop, ticks(t.hitstop));
    run(&mut w, ticks(t.hitstop), NONE);
    assert_eq!(w.player.exact_pos(), frozen, "inmóvil durante el hit-stop");
    assert!(w.shake > 0);
    // Durante el float no hay gravedad: velocidad exacta del golpe.
    run(&mut w, ticks(t.strike_float) - 1, NONE);
    assert_eq!(w.player.vel.1, -t.strike_speed);
    assert!(w.player.exact_pos().1 < frozen.1);
    // Después, la gravedad vuelve.
    run(&mut w, 2, NONE);
    assert!(w.player.vel.1 > -t.strike_speed);
}

#[test]
fn diagonal_impulse_is_exactly_opposite() {
    let mut w = world(
        "
#########
#.......#
#...P...#
#.......#
#....o..#
#.......#
#########
",
    );
    step(&mut w, &strike((1, 1)));
    let s = w.tuning.strike_speed / 2f32.sqrt();
    let (vx, vy) = w.player.vel;
    assert!(
        (vx + s).abs() < 1e-3 && (vy + s).abs() < 1e-3,
        "vel = {vx},{vy}"
    );
}

#[test]
fn whiff_costs_nothing_but_cooldown() {
    let mut w = above('o');
    let before = w.player.vel;
    step(&mut w, &strike((0, -1))); // hacia arriba: no hay nada
    let t = w.tuning.clone();
    run(&mut w, ticks(t.strike_window), NONE);
    assert!(w.events.iter().any(|e| matches!(e, SimEvent::Whiff)));
    assert_eq!(w.player.strikes, 1);
    assert!(w.player.vel.1 > before.1, "sigue cayendo, sin impulso");
    // Durante el enfriamiento, un golpe nuevo hacia la roca queda en buffer
    // y entra en cuanto se puede (si sigue a tiro).
    assert!(w.player.strike_cooldown > 0);
}

#[test]
fn no_direction_strikes_where_facing() {
    let mut w = world(
        "
#########
#.......#
#..P.o..#
#########
",
    );
    run(&mut w, 30, NONE);
    step(&mut w, &strike((0, 0)));
    assert!(
        w.player.vel.0 < 0.0,
        "golpea a la derecha, sale a la izquierda"
    );
}

#[test]
fn bone_is_stronger_and_does_not_refill() {
    let mut w = above('b');
    step(&mut w, &strike((0, 1)));
    let t = w.tuning.clone();
    assert_eq!(w.player.vel.1, -t.strike_speed * t.bone_mult);
    assert_eq!(w.player.strikes, 0);
    // Sin golpes no se puede volver a golpear hasta tocar suelo.
    run(&mut w, ticks(t.hitstop + t.object_cooldown) + 2, NONE);
    let vel = w.player.vel;
    step(&mut w, &strike((0, 1)));
    assert_eq!(w.player.strike_window, 0);
    assert_ne!(w.player.vel.1, -t.strike_speed * t.bone_mult, "{vel:?}");
}

#[test]
fn ground_refills_strikes() {
    let mut w = above('b');
    step(&mut w, &strike((0, 1)));
    assert_eq!(w.player.strikes, 0);
    run(&mut w, SIM_HZ * 2, NONE);
    assert!(w.player.on_ground);
    assert_eq!(w.player.strikes, 1);
}

#[test]
fn fragile_breaks_and_respawns() {
    let mut w = above('x');
    step(&mut w, &strike((0, 1)));
    let t = w.tuning.clone();
    assert!(w.events.iter().any(|e| matches!(e, SimEvent::Break { .. })));
    assert!(!w.objects[0].active());
    run(&mut w, ticks(t.fragile_respawn) - 2, NONE);
    assert!(!w.objects[0].active());
    run(&mut w, 2, NONE);
    assert!(w.objects[0].active());
}

#[test]
fn same_object_has_cooldown() {
    let mut w = above('o');
    step(&mut w, &strike((0, 1)));
    let t = w.tuning.clone();
    assert!(!w.objects[0].active());
    run(&mut w, ticks(t.object_cooldown), NONE);
    assert!(w.objects[0].active());
}

#[test]
fn active_window_catches_object_entering_range() {
    let mut w = above('o');
    // Más arriba y cayendo rápido: al pulsar, la roca aún no está a tiro.
    w.player.pos.1 = 12;
    w.player.vel = (0.0, 150.0);
    let hb = w.player.strike_box((0, 1), &w.tuning);
    assert!(hb.3 < w.objects[0].bounds().1, "fuera de alcance al pulsar");
    step(&mut w, &strike((0, 1)));
    assert!(w.player.strike_window > 0);
    let mut hit_at = None;
    for i in 0..ticks(w.tuning.strike_window) {
        if w.player.vel.1 < 0.0 {
            hit_at = Some(i);
            break;
        }
        step(&mut w, &NONE);
    }
    assert!(hit_at.is_some(), "la ventana activa debe conectar");
}

#[test]
fn strike_pressed_during_hitstop_is_buffered_for_chaining() {
    // Roca abajo y otra a la derecha a la altura a la que se llega subiendo.
    let mut w = world(
        "
###########
#.........#
#.....o...#
#...P.....#
#.........#
#...o.....#
#.........#
###########
",
    );
    step(&mut w, &strike((0, 1)));
    assert!(w.hitstop > 0);
    // Pulsa golpe a la derecha en mitad del hit-stop.
    step(&mut w, &strike((1, 0)));
    let t = w.tuning.clone();
    let mut chained = false;
    for _ in 0..ticks(t.hitstop + t.strike_float + t.strike_window) + 2 {
        step(&mut w, &NONE);
        if w.player.vel.0 < 0.0 {
            chained = true;
            break;
        }
    }
    assert!(chained, "el segundo golpe debe encadenarse");
}

#[test]
fn steer_option_adds_stick_push() {
    let mut w = above('o');
    w.tuning.strike_steer = 50.0;
    step(&mut w, &strike((0, 1)));
    // Apuntando abajo, el empuje del stick (abajo) resta a la subida.
    assert_eq!(w.player.vel.1, -w.tuning.strike_speed + 50.0);
}

#[test]
fn restart_restores_objects() {
    let mut w = above('x');
    step(&mut w, &strike((0, 1)));
    assert!(!w.objects[0].active());
    w.restart();
    assert!(w.objects[0].active());
    assert_eq!(w.hitstop, 0);
}

#[test]
fn shipped_strike_level_parses() {
    let m = TileMap::parse(include_str!("../../../assets/levels/golpe.txt")).unwrap();
    assert_eq!((m.w, m.h), (40, 23));
    for k in [
        StrikeKind::Rock,
        StrikeKind::Fragile,
        StrikeKind::Bone,
        StrikeKind::Moss,
    ] {
        assert!(m.objects.iter().any(|(o, _)| *o == k), "falta {k:?}");
    }
}

#[test]
fn deterministic_with_strikes() {
    let level = include_str!("../../../assets/levels/golpe.txt");
    let mut a = world(level);
    let mut b = world(level);
    let dirs = [(0, 1), (1, 0), (-1, -1), (1, 1), (0, -1), (-1, 0)];
    for i in 0..SIM_HZ * 15 {
        let d = dirs[(i / 50) as usize % dirs.len()];
        let inp = Input {
            x: d.0,
            y: d.1,
            jump_held: (i / 30) % 4 == 0,
            jump_pressed: i % 120 == 0,
            strike_pressed: i % 41 == 0,
            aim_x: d.0,
            aim_y: d.1,
        };
        step(&mut a, &inp);
        step(&mut b, &inp);
    }
    assert_eq!(a.state_hash(), b.state_hash());
}
