use crate::strike::StrikeKind;

/// Tamaño de una celda en píxeles de juego.
pub const TILE: i32 = 8;

/// Hacia dónde apuntan unos pinchos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir4 {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Empty,
    Solid,
    /// Pinchos (no sólidos). Matan al tocarlos moviéndose contra ellos.
    Spike(Dir4),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneKind {
    /// Fuego: quema la porra (deja sin golpes) mientras se esté dentro.
    Fire,
    /// Salida de la sala: pasa a la siguiente.
    Exit,
}

/// Rectángulo de efecto en píxeles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Zone {
    pub kind: ZoneKind,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Zone {
    pub fn overlaps(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        x < self.x + self.w && self.x < x + w && y < self.y + self.h && self.y < y + h
    }
}

/// Definición de un objeto golpeable o enemigo tal como viene del mapa.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObjectDef {
    pub kind: StrikeKind,
    /// Centro de la celda/rectángulo del editor, en píxeles.
    pub center: (f32, f32),
    /// Recorrido a cada lado (px), para enemigos que patrullan.
    pub range: f32,
    /// Sentido inicial (-1 o 1).
    pub dir: i8,
}

impl ObjectDef {
    pub fn new(kind: StrikeKind, center: (f32, f32)) -> Self {
        Self {
            kind,
            center,
            range: 3.0 * TILE as f32,
            dir: 1,
        }
    }
}

/// Una sala: rejilla de tiles, punto de aparición, objetos y zonas.
///
/// Se carga desde Tiled (ver `tiled.rs`) o, en tests, desde texto ASCII:
///
/// - `#` sólido; `.` o espacio: vacío
/// - `^ v < >` pinchos apuntando arriba, abajo, izquierda, derecha
/// - `P` punto de aparición del jugador (sus pies, centrados en la celda)
/// - `o` roca, `x` roca frágil, `b` hueso colgante, `m` musgo (golpeables)
/// - `e` escarabajo, `w` murciélago (enemigos golpeables)
/// - `f` fuego, `E` salida (zonas de una celda)
#[derive(Clone, Debug)]
pub struct TileMap {
    pub name: String,
    pub w: i32,
    pub h: i32,
    tiles: Vec<Tile>,
    /// Punto de aparición en píxeles (centro inferior de la caja del jugador).
    pub spawn: (i32, i32),
    pub objects: Vec<ObjectDef>,
    pub zones: Vec<Zone>,
    /// Capas visuales (sólo para dibujar): índice+1 del tile en el tileset
    /// `cueva`, 0 = vacío. De abajo arriba.
    pub gfx: Vec<Vec<u16>>,
}

/// Índices del tileset `cueva` (ver `tools/gen_art.py`).
pub mod gfx {
    pub const STONE: u16 = 0;
    pub const STONE_TOP: u16 = 1;
    pub const STONE_DARK: u16 = 2;
    pub const SPIKE_UP: u16 = 3;
    pub const SPIKE_DOWN: u16 = 4;
    pub const SPIKE_LEFT: u16 = 5;
    pub const SPIKE_RIGHT: u16 = 6;
}

impl TileMap {
    /// Sala vacía de `w × h` celdas.
    pub fn new(name: &str, w: i32, h: i32) -> Self {
        Self {
            name: name.to_string(),
            w,
            h,
            tiles: vec![Tile::Empty; (w.max(0) * h.max(0)) as usize],
            spawn: (0, 0),
            objects: Vec::new(),
            zones: Vec::new(),
            gfx: Vec::new(),
        }
    }

    /// Capa visual automática a partir de la colisión (para salas ASCII o
    /// mapas sin capa visual): piedra, piedra con borde arriba, interior
    /// oscuro y pinchos.
    pub fn auto_gfx(&self) -> Vec<u16> {
        let mut out = vec![0u16; (self.w * self.h) as usize];
        let solid = |x, y| self.tile(x, y) == Tile::Solid && y >= 0 && y < self.h;
        for ty in 0..self.h {
            for tx in 0..self.w {
                let id = match self.tile(tx, ty) {
                    Tile::Empty => continue,
                    Tile::Spike(Dir4::Up) => gfx::SPIKE_UP,
                    Tile::Spike(Dir4::Down) => gfx::SPIKE_DOWN,
                    Tile::Spike(Dir4::Left) => gfx::SPIKE_LEFT,
                    Tile::Spike(Dir4::Right) => gfx::SPIKE_RIGHT,
                    Tile::Solid => {
                        let open_above = ty > 0 && !solid(tx, ty - 1);
                        let buried = (-1..=1).all(|dy| {
                            (-1..=1).all(|dx| {
                                let (x, y) = (tx + dx, ty + dy);
                                x < 0 || x >= self.w || y < 0 || y >= self.h || solid(x, y)
                            })
                        });
                        if open_above {
                            gfx::STONE_TOP
                        } else if buried {
                            gfx::STONE_DARK
                        } else {
                            gfx::STONE
                        }
                    }
                };
                out[(ty * self.w + tx) as usize] = id + 1;
            }
        }
        out
    }

    pub fn set(&mut self, tx: i32, ty: i32, t: Tile) {
        if tx >= 0 && ty >= 0 && tx < self.w && ty < self.h {
            self.tiles[(ty * self.w + tx) as usize] = t;
        }
    }

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
        let mut m = Self::new("ascii", w, rows.len() as i32);
        let mut spawn = None;
        for (ty, row) in rows.iter().enumerate() {
            for (tx, c) in row.chars().enumerate() {
                let (tx, ty) = (tx as i32, ty as i32);
                let cell_center = ((tx * TILE + TILE / 2) as f32, (ty * TILE + TILE / 2) as f32);
                let zone = |kind| Zone {
                    kind,
                    x: tx * TILE,
                    y: ty * TILE,
                    w: TILE,
                    h: TILE,
                };
                match c {
                    '#' => m.set(tx, ty, Tile::Solid),
                    '^' => m.set(tx, ty, Tile::Spike(Dir4::Up)),
                    'v' => m.set(tx, ty, Tile::Spike(Dir4::Down)),
                    '<' => m.set(tx, ty, Tile::Spike(Dir4::Left)),
                    '>' => m.set(tx, ty, Tile::Spike(Dir4::Right)),
                    '.' | ' ' => {}
                    'P' => spawn = Some((tx * TILE + TILE / 2, (ty + 1) * TILE)),
                    'f' => m.zones.push(zone(ZoneKind::Fire)),
                    'E' => m.zones.push(zone(ZoneKind::Exit)),
                    c => match StrikeKind::from_char(c) {
                        Some(kind) => m.objects.push(ObjectDef::new(kind, cell_center)),
                        None => return Err(format!("carácter desconocido '{c}' en ({tx},{ty})")),
                    },
                }
            }
        }
        m.spawn = spawn.ok_or("el mapa no tiene punto de aparición 'P'")?;
        m.gfx = vec![m.auto_gfx()];
        Ok(m)
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

    fn cells(x: i32, y: i32, w: i32, h: i32) -> impl Iterator<Item = (i32, i32)> {
        let tx0 = x.div_euclid(TILE);
        let ty0 = y.div_euclid(TILE);
        let tx1 = (x + w - 1).div_euclid(TILE);
        let ty1 = (y + h - 1).div_euclid(TILE);
        (ty0..=ty1).flat_map(move |ty| (tx0..=tx1).map(move |tx| (tx, ty)))
    }

    /// ¿Algún tile sólido solapa el rectángulo `[x, x+w) × [y, y+h)` en píxeles?
    pub fn solid_in_rect(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        if w <= 0 || h <= 0 {
            return false;
        }
        Self::cells(x, y, w, h).any(|(tx, ty)| self.tile(tx, ty) == Tile::Solid)
    }

    /// Caja letal de unos pinchos: la mitad de la celda pegada a su base.
    pub fn spike_box(tx: i32, ty: i32, d: Dir4) -> (i32, i32, i32, i32) {
        let (x, y, half) = (tx * TILE, ty * TILE, TILE / 2);
        match d {
            Dir4::Up => (x, y + half, TILE, half),
            Dir4::Down => (x, y, TILE, half),
            Dir4::Left => (x + half, y, half, TILE),
            Dir4::Right => (x, y, half, TILE),
        }
    }

    /// ¿El rectángulo toca unos pinchos moviéndose contra ellos? Como en
    /// Celeste, rozar unos pinchos alejándose de ellos no mata.
    pub fn spike_hit(&self, x: i32, y: i32, w: i32, h: i32, vel: (f32, f32)) -> bool {
        Self::cells(x, y, w, h).any(|(tx, ty)| {
            let Tile::Spike(d) = self.tile(tx, ty) else {
                return false;
            };
            let (sx, sy, sw, sh) = Self::spike_box(tx, ty, d);
            let overlap = x < sx + sw && sx < x + w && y < sy + sh && sy < y + h;
            let against = match d {
                Dir4::Up => vel.1 >= 0.0,
                Dir4::Down => vel.1 <= 0.0,
                Dir4::Left => vel.0 >= 0.0,
                Dir4::Right => vel.0 <= 0.0,
            };
            overlap && against
        })
    }
}
