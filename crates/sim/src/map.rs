use crate::strike::StrikeKind;

/// Tamaño de una celda en píxeles de juego.
pub const TILE: i32 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Empty,
    Solid,
}

/// Rejilla de tiles. Se construye desde texto ASCII (provisional, hasta la
/// Fase 3 con Tiled/LDtk):
///
/// - `#` sólido
/// - `.` o espacio: vacío
/// - `P` punto de aparición del jugador (sus pies, centrados en la celda)
/// - `o` roca, `x` roca frágil, `b` hueso colgante, `m` musgo (golpeables)
#[derive(Clone, Debug)]
pub struct TileMap {
    pub w: i32,
    pub h: i32,
    tiles: Vec<Tile>,
    /// Punto de aparición en píxeles (centro inferior de la caja del jugador).
    pub spawn: (i32, i32),
    /// Objetos golpeables iniciales: tipo y centro en píxeles.
    pub objects: Vec<(StrikeKind, (i32, i32))>,
}

impl TileMap {
    pub fn parse(src: &str) -> Result<Self, String> {
        let rows: Vec<&str> = src
            .lines()
            .map(|l| l.trim_end_matches('\r'))
            .filter(|l| !l.trim().is_empty() && !l.starts_with(';'))
            .collect();
        if rows.is_empty() {
            return Err("mapa vacío".into());
        }
        let w = rows.iter().map(|r| r.chars().count()).max().unwrap() as i32;
        let h = rows.len() as i32;
        let mut tiles = vec![Tile::Empty; (w * h) as usize];
        let mut spawn = None;
        let mut objects = Vec::new();
        for (ty, row) in rows.iter().enumerate() {
            for (tx, c) in row.chars().enumerate() {
                let i = ty * w as usize + tx;
                match c {
                    '#' => tiles[i] = Tile::Solid,
                    '.' | ' ' => {}
                    'P' => {
                        spawn = Some((tx as i32 * TILE + TILE / 2, (ty as i32 + 1) * TILE));
                    }
                    c if StrikeKind::from_char(c).is_some() => {
                        let kind = StrikeKind::from_char(c).unwrap();
                        let center = (tx as i32 * TILE + TILE / 2, ty as i32 * TILE + TILE / 2);
                        objects.push((kind, center));
                    }
                    other => return Err(format!("carácter desconocido '{other}' en ({tx},{ty})")),
                }
            }
        }
        let spawn = spawn.ok_or("el mapa no tiene punto de aparición 'P'")?;
        Ok(Self {
            w,
            h,
            tiles,
            spawn,
            objects,
        })
    }

    pub fn width_px(&self) -> i32 {
        self.w * TILE
    }

    pub fn height_px(&self) -> i32 {
        self.h * TILE
    }

    /// Tile en coordenadas de celda. Fuera del mapa: sólido por los lados,
    /// vacío por arriba y por abajo (para poder caer al vacío).
    pub fn tile(&self, tx: i32, ty: i32) -> Tile {
        if tx < 0 || tx >= self.w {
            return Tile::Solid;
        }
        if ty < 0 || ty >= self.h {
            return Tile::Empty;
        }
        self.tiles[(ty * self.w + tx) as usize]
    }

    /// ¿Algún tile sólido solapa el rectángulo `[x, x+w) × [y, y+h)` en píxeles?
    pub fn solid_in_rect(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        if w <= 0 || h <= 0 {
            return false;
        }
        let tx0 = x.div_euclid(TILE);
        let ty0 = y.div_euclid(TILE);
        let tx1 = (x + w - 1).div_euclid(TILE);
        let ty1 = (y + h - 1).div_euclid(TILE);
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                if self.tile(tx, ty) == Tile::Solid {
                    return true;
                }
            }
        }
        false
    }
}
