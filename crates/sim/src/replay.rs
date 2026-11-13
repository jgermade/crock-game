//! Grabación de entradas por tick. Con la simulación determinista, una
//! partida se reproduce exactamente a partir de la sala, las constantes y
//! esta lista.
//!
//! Formato de texto: una línea por tick con 7 enteros separados por
//! espacios: `x y jump_held jump_pressed strike_pressed aim_x aim_y`.
//! Las líneas que empiezan por `#` son comentarios.

use crate::Input;

pub fn encode(inputs: &[Input]) -> String {
    let mut s = String::with_capacity(inputs.len() * 16);
    s.push_str("# cROCK replay v1: x y jump_held jump_pressed strike_pressed aim_x aim_y\n");
    for i in inputs {
        s.push_str(&format!(
            "{} {} {} {} {} {} {}\n",
            i.x,
            i.y,
            i.jump_held as u8,
            i.jump_pressed as u8,
            i.strike_pressed as u8,
            i.aim_x,
            i.aim_y
        ));
    }
    s
}

pub fn decode(src: &str) -> Result<Vec<Input>, String> {
    let mut out = Vec::new();
    for (n, line) in src.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let v: Vec<i8> = line
            .split_whitespace()
            .map(|t| t.parse::<i8>())
            .collect::<Result<_, _>>()
            .map_err(|e| format!("línea {}: {e}", n + 1))?;
        let [x, y, jh, jp, sp, ax, ay] = v[..] else {
            return Err(format!("línea {}: se esperaban 7 valores", n + 1));
        };
        out.push(Input {
            x,
            y,
            jump_held: jh != 0,
            jump_pressed: jp != 0,
            strike_pressed: sp != 0,
            aim_x: ax,
            aim_y: ay,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let inputs = vec![
            Input {
                x: 1,
                y: -1,
                jump_held: true,
                jump_pressed: true,
                strike_pressed: false,
                aim_x: 1,
                aim_y: -1,
            },
            Input {
                x: 0,
                y: 1,
                jump_held: false,
                jump_pressed: false,
                strike_pressed: true,
                aim_x: -1,
                aim_y: 1,
            },
        ];
        assert_eq!(decode(&encode(&inputs)).unwrap(), inputs);
        assert!(decode("1 2 3").is_err());
    }
}
