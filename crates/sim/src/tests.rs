use super::*;

const FLAT: &str = "
##########
#........#
#........#
#........#
#........#
#........#
#..P.....#
##########
";

fn world(src: &str) -> World {
    World::new(TileMap::parse(src).unwrap(), Tuning::default())
}

fn run(w: &mut World, ticks: u32, input: Input) {
    for _ in 0..ticks {
        step(w, &input);
    }
}

fn settle(w: &mut World) {
    run(w, SIM_HZ, Input::default());
    assert!(w.player.on_ground, "el jugador debería estar en el suelo");
}

const NONE: Input = Input {
    x: 0,
    y: 0,
    jump_held: false,
    jump_pressed: false,
    strike_pressed: false,
    aim_x: 0,
    aim_y: 0,
};
const JUMP: Input = Input {
    x: 0,
    y: 0,
    jump_held: true,
    jump_pressed: true,
    ..NONE
};
const HOLD: Input = Input {
    x: 0,
    y: 0,
    jump_held: true,
    jump_pressed: false,
    ..NONE
};

#[test]
fn shipped_test_level_parses() {
    let m = crate::test_util::level("movimiento");
    assert_eq!((m.w, m.h), (44, 24));
}

#[test]
fn falls_and_lands_on_ground() {
    let mut w = world(FLAT);
    let floor_y = 7 * TILE;
    settle(&mut w);
    assert_eq!(w.player.pos.1 + Player::H, floor_y);
    assert_eq!(w.player.vel.1, 0.0);
}

#[test]
fn walls_stop_horizontal_motion() {
    let mut w = world(FLAT);
    settle(&mut w);
    run(
        &mut w,
        SIM_HZ * 2,
        Input {
            x: 1,
            ..Default::default()
        },
    );
    assert_eq!(
        w.player.pos.0 + Player::W,
        9 * TILE,
        "pegado a la pared derecha"
    );
    run(
        &mut w,
        SIM_HZ * 2,
        Input {
            x: -1,
            ..Default::default()
        },
    );
    assert_eq!(w.player.pos.0, TILE, "pegado a la pared izquierda");
}

#[test]
fn reaches_run_speed() {
    let mut w = world(
        "
##############################
#............................#
#P...........................#
##############################
",
    );
    settle(&mut w);
    run(
        &mut w,
        SIM_HZ / 4,
        Input {
            x: 1,
            ..Default::default()
        },
    );
    assert_eq!(w.player.vel.0, w.tuning.run_speed);
}

/// Altura de salto en píxeles con un perfil de botón dado.
fn jump_height(hold_ticks: u32) -> i32 {
    let mut w = world(FLAT);
    settle(&mut w);
    let ground = w.player.pos.1;
    step(&mut w, &JUMP);
    let mut top = w.player.pos.1;
    for i in 0..SIM_HZ {
        let inp = if i < hold_ticks {
            HOLD
        } else {
            Input::default()
        };
        step(&mut w, &inp);
        top = top.min(w.player.pos.1);
    }
    assert!(w.player.on_ground);
    ground - top
}

#[test]
fn variable_jump_height() {
    let tap = jump_height(0);
    let full = jump_height(SIM_HZ);
    assert!(tap >= 3, "un toque debe levantar algo ({tap})");
    assert!(
        full > tap * 2,
        "mantener debe saltar bastante más ({full} vs {tap})"
    );
    // Unos 3 tiles de salto completo con los valores por defecto.
    assert!((20..=32).contains(&full), "salto completo = {full} px");
}

#[test]
fn coyote_time_allows_late_jump() {
    let src = "
##########
#........#
#........#
#........#
#........#
#P.......#
###......#
#........#
#........#
#........#
##########
";
    // Saltar dentro y fuera de la ventana de coyote tras dejar el borde.
    for (late, should_jump) in [
        (1, true),
        (secs_to_ticks(0.08), true),
        (secs_to_ticks(0.15), false),
    ] {
        let mut w = world(src);
        settle(&mut w);
        let mut off_ticks = 0;
        let mut jumped = false;
        for _ in 0..SIM_HZ * 2 {
            let off = !w.player.on_ground && w.player.vel.1 >= 0.0;
            if off {
                off_ticks += 1;
            }
            let press = off_ticks == late;
            step(
                &mut w,
                &Input {
                    x: 1,
                    y: 0,
                    jump_held: press,
                    jump_pressed: press,
                    ..NONE
                },
            );
            if press {
                jumped = w.player.vel.1 < 0.0;
                break;
            }
        }
        assert_eq!(
            jumped, should_jump,
            "salto {late} ticks después de dejar el borde"
        );
    }
}

#[test]
fn jump_buffer_triggers_on_landing() {
    for (early, should_jump) in [(secs_to_ticks(0.05), true), (secs_to_ticks(0.2), false)] {
        let mut w = world(FLAT);
        settle(&mut w);
        step(&mut w, &JUMP);
        // Esperar a estar cayendo cerca del suelo, contando ticks hasta aterrizar.
        let mut probe = w.clone();
        let mut to_land = 0;
        while !probe.player.on_ground || probe.player.vel.1 < 0.0 || to_land == 0 {
            step(&mut probe, &Input::default());
            to_land += 1;
        }
        // Repetir pulsando `early` ticks antes de aterrizar.
        for i in 0..to_land {
            let press = i + early == to_land;
            step(
                &mut w,
                &Input {
                    x: 0,
                    y: 0,
                    jump_held: press,
                    jump_pressed: press,
                    ..NONE
                },
            );
        }
        // Un tick más: el salto en buffer se ejecuta al tocar suelo.
        step(&mut w, &Input::default());
        assert_eq!(
            w.player.vel.1 < 0.0,
            should_jump,
            "pulsado {early} ticks antes de aterrizar"
        );
    }
}

#[test]
fn ceiling_bonk_cancels_jump() {
    let mut w = world(
        "
##########
#........#
#........#
#........#
#..P.....#
##########
",
    );
    settle(&mut w);
    step(&mut w, &JUMP);
    let mut top = w.player.pos.1;
    while w.player.pos.1 > TILE && w.player.vel.1 < 0.0 {
        step(&mut w, &HOLD);
        top = top.min(w.player.pos.1);
    }
    assert_eq!(top, TILE, "toca el techo");
    // El choque llega en cuanto el resto sub-píxel intenta avanzar 1 px más
    // (1-2 ticks); después cae aunque se siga manteniendo el botón.
    run(&mut w, 3, HOLD);
    assert!(w.player.vel.1 >= 0.0);
    assert_eq!(w.player.var_jump, 0);
}

#[test]
fn falling_out_restarts() {
    let mut w = world(
        "
#.....#
#.P...#
#.###.#
#.....#
",
    );
    run(&mut w, SIM_HZ / 2, Input::default());
    run(
        &mut w,
        SIM_HZ * 2,
        Input {
            x: 1,
            ..Default::default()
        },
    );
    // Ha caído por el hueco de la derecha y ha reaparecido.
    let spawn = Player::spawn(w.map.spawn);
    assert!(w.player.pos.1 <= spawn.pos.1 + 2 * TILE);
}

#[test]
fn deterministic() {
    let script: Vec<Input> = (0..SIM_HZ * 10)
        .map(|i| {
            let x = [1, 1, 0, -1, 1, 0][(i / 37) as usize % 6];
            let jump = (i / 23) % 5 == 0;
            Input {
                x,
                y: 0,
                jump_held: jump,
                jump_pressed: jump && i % 23 == 0,
                ..NONE
            }
        })
        .collect();
    let level = "movimiento";
    let mut a = World::new(crate::test_util::level(level), Tuning::default());
    let mut b = World::new(crate::test_util::level(level), Tuning::default());
    for inp in &script {
        step(&mut a, inp);
        step(&mut b, inp);
    }
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn tuning_parse() {
    let (t, warn) = Tuning::parse("gravity = 1000 # comentario\nfoo = 1\njump_speed = x\n\n");
    assert_eq!(t.gravity, 1000.0);
    assert_eq!(t.jump_speed, Tuning::default().jump_speed);
    assert_eq!(warn.len(), 2);
}

#[test]
fn interpolation_bounds() {
    let mut w = world(FLAT);
    run(&mut w, 10, Input::default());
    let p = &w.player;
    assert_eq!(p.lerp_pos(0.0), p.prev);
    let (x, y) = p.lerp_pos(1.0);
    assert!((x - p.exact_pos().0).abs() < 1e-4 && (y - p.exact_pos().1).abs() < 1e-4);
}
