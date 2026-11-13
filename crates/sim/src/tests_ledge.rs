use super::*;

/// Pared de 4 tiles (32 px), más alta que un salto (~28 px): sin agarrarse
/// no se puede subir.
const WALL: &str = "
##########
#........#
#........#
#........#
#....#####
#....#####
#....#####
#P...#####
##########
";
const LEDGE_Y: i32 = 4 * TILE;

fn world() -> World {
    World::new(TileMap::parse(WALL).unwrap(), Tuning::default())
}

fn inp(x: i8, y: i8, jump: bool) -> Input {
    Input {
        x,
        y,
        jump_held: jump,
        jump_pressed: jump,
        aim_x: x,
        aim_y: y,
        ..Default::default()
    }
}

fn run(w: &mut World, ticks: u32, i: Input) {
    for _ in 0..ticks {
        step(w, &i);
    }
}

/// Anda hasta la pared y salta manteniendo hacia ella.
fn jump_at_wall(w: &mut World, x: i8) {
    run(w, SIM_HZ, inp(1, 0, false));
    step(w, &inp(x, 0, true));
    for _ in 0..SIM_HZ {
        step(
            w,
            &Input {
                jump_held: true,
                ..inp(x, 0, false)
            },
        );
        if w.player.hanging != 0 {
            break;
        }
    }
}

#[test]
fn grabs_ledge_when_holding_toward_the_wall() {
    let mut w = world();
    jump_at_wall(&mut w, 1);
    assert_eq!(w.player.hanging, 1, "debería colgarse");
    assert_eq!(w.player.pos.1, LEDGE_Y - 1, "manos justo encima del borde");
    let pos = w.player.pos;
    run(&mut w, SIM_HZ / 2, inp(1, 0, false));
    assert_eq!(w.player.pos, pos, "colgado no se mueve");
}

#[test]
fn no_grab_without_pushing_toward_the_wall() {
    let mut w = world();
    jump_at_wall(&mut w, 0);
    assert_eq!(w.player.hanging, 0);
}

#[test]
fn jump_from_ledge_climbs_on_top() {
    let mut w = world();
    jump_at_wall(&mut w, 1);
    step(&mut w, &inp(1, 0, true));
    run(&mut w, SIM_HZ, inp(1, 0, false));
    assert!(w.player.on_ground);
    assert_eq!(
        w.player.pos.1 + Player::H,
        LEDGE_Y,
        "de pie encima de la cornisa"
    );
}

#[test]
fn down_drops_from_ledge() {
    let mut w = world();
    jump_at_wall(&mut w, 1);
    step(&mut w, &inp(0, 1, false));
    assert_eq!(w.player.hanging, 0);
    run(&mut w, SIM_HZ, inp(0, 0, false));
    assert!(w.player.on_ground);
    assert_eq!(
        w.player.pos.1 + Player::H,
        8 * TILE,
        "vuelve al suelo de abajo"
    );
}

#[test]
fn ledge_grab_can_be_disabled() {
    let mut w = world();
    w.tuning.ledge_grab = 0.0;
    jump_at_wall(&mut w, 1);
    assert_eq!(w.player.hanging, 0);
}
