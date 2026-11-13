//! Carga de salas desde Tiled (formato JSON: `.tmj` y tilesets `.tsj`).
//!
//! Convenciones (ver `docs/NIVELES.md`):
//!
//! - Tiles de 8, 16 o 24 px (el arte va a 3 px por unidad; las
//!   coordenadas se escalan a la unidad de la física, 8 px por tile), capas
//!   de tiles en CSV (no base64), mapa no infinito.
//! - Cada tile del tileset declara su `kind` (propiedad o clase): `solid`,
//!   `spike_up`, `spike_down`, `spike_left`, `spike_right`. Sin `kind`: sólido.
//!   Las capas cuyo nombre empieza por `deco` son sólo decorativas.
//! - Capa de objetos con objetos cuyo tipo/clase es `spawn`, `rock`,
//!   `fragile`, `bone`, `moss`, `beetle`, `bat`, `fire` o `exit`.
//!   Propiedades opcionales: `range` (tiles, enemigos) y `dir` (-1 o 1).

use serde::Deserialize;
use serde_json::Value;

use crate::{Dir4, ObjectDef, StrikeKind, TILE, Tile, TileMap, Zone, ZoneKind};

#[derive(Deserialize)]
struct RawMap {
    width: i32,
    height: i32,
    tilewidth: i32,
    tileheight: i32,
    #[serde(default)]
    infinite: bool,
    #[serde(default)]
    layers: Vec<RawLayer>,
    #[serde(default)]
    tilesets: Vec<RawTilesetRef>,
    #[serde(default)]
    properties: Vec<RawProp>,
}

#[derive(Deserialize)]
struct RawTilesetRef {
    firstgid: u32,
    source: Option<String>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    tiles: Vec<RawTile>,
}

#[derive(Deserialize)]
struct RawTileset {
    #[serde(default)]
    name: String,
    #[serde(default)]
    tiles: Vec<RawTile>,
}

#[derive(Deserialize)]
struct RawTile {
    id: u32,
    #[serde(rename = "type", alias = "class", default)]
    class: String,
    #[serde(default)]
    properties: Vec<RawProp>,
}

#[derive(Deserialize)]
struct RawProp {
    name: String,
    value: Value,
}

#[derive(Deserialize)]
struct RawLayer {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    name: String,
    data: Option<Value>,
    encoding: Option<String>,
    #[serde(default)]
    objects: Vec<RawObject>,
    #[serde(default)]
    layers: Vec<RawLayer>,
}

#[derive(Deserialize)]
struct RawObject {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    class: String,
    #[serde(default)]
    name: String,
    x: f32,
    y: f32,
    #[serde(default)]
    width: f32,
    #[serde(default)]
    height: f32,
    #[serde(default)]
    point: bool,
    gid: Option<u32>,
    #[serde(default)]
    properties: Vec<RawProp>,
}

fn prop<'a>(props: &'a [RawProp], name: &str) -> Option<&'a Value> {
    props.iter().find(|p| p.name == name).map(|p| &p.value)
}

/// Bits de volteo/rotación que Tiled guarda en la parte alta de cada gid.
const GID_MASK: u32 = 0x0FFF_FFFF;

fn tile_from_kind(kind: &str) -> Result<Tile, String> {
    Ok(match kind {
        "" | "solid" => Tile::Solid,
        "spike_up" => Tile::Spike(Dir4::Up),
        "spike_down" => Tile::Spike(Dir4::Down),
        "spike_left" => Tile::Spike(Dir4::Left),
        "spike_right" => Tile::Spike(Dir4::Right),
        "empty" | "deco" => Tile::Empty,
        other => return Err(format!("kind de tile desconocido '{other}'")),
    })
}

/// Carga una sala. `name` identifica la sala (normalmente el fichero) y
/// `read` lee ficheros relativos al mapa (para tilesets externos).
/// Devuelve la sala y una lista de avisos no fatales.
pub fn load_map(
    src: &str,
    name: &str,
    read: &dyn Fn(&str) -> Result<String, String>,
) -> Result<(TileMap, Vec<String>), String> {
    let raw: RawMap =
        serde_json::from_str(src).map_err(|e| format!("{name}: JSON inválido: {e}"))?;
    if raw.tilewidth != raw.tileheight || !matches!(raw.tilewidth, 8 | 16 | 24) {
        return Err(format!("{name}: los tiles deben ser de 8, 16 o 24 px"));
    }
    // Factor de editor -> unidades de física.
    let k = TILE as f32 / raw.tilewidth as f32;
    if raw.infinite {
        return Err(format!("{name}: los mapas infinitos no están soportados"));
    }
    let mut warnings = Vec::new();

    // gid -> Tile (colisión) y gid -> clase (objetos-tile), según los tilesets.
    // El tileset `cueva` además da los gráficos.
    let mut gid_tiles: Vec<(u32, Tile)> = Vec::new();
    let mut gid_class: Vec<(u32, String)> = Vec::new();
    let mut cueva_firstgid: Option<u32> = None;
    let mut firstgids: Vec<u32> = raw.tilesets.iter().map(|t| t.firstgid).collect();
    firstgids.sort_unstable();
    for ts in &raw.tilesets {
        let (ts_name, tiles) = match &ts.source {
            Some(src) => {
                let text = read(src)?;
                let t: RawTileset =
                    serde_json::from_str(&text).map_err(|e| format!("{src}: {e}"))?;
                (t.name, t.tiles)
            }
            None => (
                ts.name.clone(),
                ts.tiles
                    .iter()
                    .map(|t| RawTile {
                        id: t.id,
                        class: t.class.clone(),
                        properties: t
                            .properties
                            .iter()
                            .map(|p| RawProp {
                                name: p.name.clone(),
                                value: p.value.clone(),
                            })
                            .collect(),
                    })
                    .collect(),
            ),
        };
        let is_cueva = ts_name == "cueva"
            || ts
                .source
                .as_deref()
                .is_some_and(|s| s.ends_with("cueva.tsj"));
        if is_cueva {
            cueva_firstgid = Some(ts.firstgid);
        }
        for t in tiles {
            if !t.class.is_empty() {
                gid_class.push((ts.firstgid + t.id, t.class.to_lowercase()));
            }
            let kind = prop(&t.properties, "kind")
                .and_then(Value::as_str)
                .unwrap_or(&t.class);
            let lower = kind.to_lowercase();
            let is_object = matches!(lower.as_str(), "spawn" | "fire" | "exit")
                || StrikeKind::from_name(&lower).is_some();
            let tile = if is_object {
                // Tile de un tileset de objetos: no colisiona si se pinta.
                Tile::Empty
            } else {
                tile_from_kind(&lower).map_err(|e| format!("{name}: tile {}: {e}", t.id))?
            };
            gid_tiles.push((ts.firstgid + t.id, tile));
        }
    }
    // Índice visual en `cueva` (+1), o 0 si el gid es de otro tileset.
    let gfx_of = |gid: u32| -> u16 {
        let Some(first) = cueva_firstgid else {
            return 0;
        };
        let next = firstgids
            .iter()
            .copied()
            .find(|&g| g > first)
            .unwrap_or(u32::MAX);
        if gid >= first && gid < next {
            (gid - first + 1) as u16
        } else {
            0
        }
    };
    let lookup = |gid: u32| -> Tile {
        if gid == 0 {
            return Tile::Empty;
        }
        gid_tiles
            .iter()
            .find(|(g, _)| *g == gid)
            .map(|(_, t)| *t)
            .unwrap_or(Tile::Solid)
    };

    let map_name = prop(&raw.properties, "name")
        .and_then(Value::as_str)
        .unwrap_or(name);
    let mut m = TileMap::new(map_name, raw.width, raw.height);
    let mut spawn = None;

    // Aplanar grupos de capas.
    fn flatten<'a>(layers: &'a [RawLayer], out: &mut Vec<&'a RawLayer>) {
        for l in layers {
            if l.kind == "group" {
                flatten(&l.layers, out);
            } else {
                out.push(l);
            }
        }
    }
    let mut layers = Vec::new();
    flatten(&raw.layers, &mut layers);

    for layer in layers {
        match layer.kind.as_str() {
            "tilelayer" => {
                let deco = layer.name.to_lowercase().starts_with("deco");
                if layer.encoding.as_deref().is_some_and(|e| e != "csv") {
                    return Err(format!(
                        "{name}: capa '{}' en {:?}; guarda las capas en CSV",
                        layer.name, layer.encoding
                    ));
                }
                let data: Vec<u32> = match &layer.data {
                    Some(v) => serde_json::from_value(v.clone())
                        .map_err(|e| format!("{name}: capa '{}': {e}", layer.name))?,
                    None => continue,
                };
                if data.len() != (raw.width * raw.height) as usize {
                    return Err(format!(
                        "{name}: capa '{}' de tamaño incorrecto",
                        layer.name
                    ));
                }
                let mut visual = vec![0u16; data.len()];
                for (i, gid) in data.into_iter().enumerate() {
                    let gid = gid & GID_MASK;
                    visual[i] = gfx_of(gid);
                    let t = lookup(gid);
                    if !deco && t != Tile::Empty {
                        m.set(i as i32 % raw.width, i as i32 / raw.width, t);
                    }
                }
                m.gfx.push(visual);
            }
            "objectgroup" => {
                for o in &layer.objects {
                    let kind = if o.kind.is_empty() { &o.class } else { &o.kind };
                    let kind = if kind.is_empty() { &o.name } else { kind };
                    let mut kind = kind.to_lowercase();
                    if kind.is_empty()
                        && let Some(gid) = o.gid
                        && let Some((_, c)) = gid_class.iter().find(|(g, _)| *g == gid & GID_MASK)
                    {
                        // Objeto-tile sin clase propia: hereda la del tile.
                        kind = c.clone();
                    }
                    // Los objetos-tile tienen el origen abajo a la izquierda.
                    let (x, y) = if o.gid.is_some() {
                        (o.x, o.y - o.height)
                    } else {
                        (o.x, o.y)
                    };
                    let (w, h) = if o.point {
                        (0.0, 0.0)
                    } else {
                        (o.width, o.height)
                    };
                    let (x, y, w, h) = (x * k, y * k, w * k, h * k);
                    let center = (x + w / 2.0, y + h / 2.0);
                    match kind.as_str() {
                        "spawn" => {
                            spawn = Some(((x + w / 2.0).round() as i32, (y + h).round() as i32))
                        }
                        "fire" | "exit" => m.zones.push(Zone {
                            kind: if kind == "fire" {
                                ZoneKind::Fire
                            } else {
                                ZoneKind::Exit
                            },
                            x: x.round() as i32,
                            y: y.round() as i32,
                            w: (w.round() as i32).max(1),
                            h: (h.round() as i32).max(1),
                        }),
                        k => match StrikeKind::from_name(k) {
                            Some(sk) => {
                                let mut d = ObjectDef::new(sk, center);
                                if let Some(r) =
                                    prop(&o.properties, "range").and_then(Value::as_f64)
                                {
                                    d.range = r as f32 * TILE as f32;
                                }
                                if let Some(dir) =
                                    prop(&o.properties, "dir").and_then(Value::as_i64)
                                {
                                    d.dir = if dir < 0 { -1 } else { 1 };
                                }
                                m.objects.push(d);
                            }
                            None => warnings.push(format!(
                                "{name}: objeto de tipo desconocido '{k}' en ({}, {})",
                                o.x, o.y
                            )),
                        },
                    }
                }
            }
            _ => {}
        }
    }
    m.spawn = spawn.ok_or_else(|| format!("{name}: falta un objeto 'spawn'"))?;
    if m.gfx.iter().all(|l| l.iter().all(|&g| g == 0)) {
        // Sin gráficos del tileset `cueva`: generarlos desde la colisión.
        m.gfx = vec![m.auto_gfx()];
    }
    Ok((m, warnings))
}

/// Carga un `.tmj` desde disco, resolviendo los tilesets externos
/// relativos a su carpeta.
pub fn load_file(path: &std::path::Path) -> Result<(TileMap, Vec<String>), String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let dir = path.parent().unwrap_or(std::path::Path::new("."));
    let read = |rel: &str| {
        let p = dir.join(rel);
        std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))
    };
    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("sala");
    load_map(&src, name, &read)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_files(p: &str) -> Result<String, String> {
        Err(format!("no existe {p}"))
    }

    const MAP: &str = r#"{
      "type": "map", "width": 4, "height": 3, "tilewidth": 8, "tileheight": 8,
      "infinite": false, "properties": [{"name": "name", "type": "string", "value": "Prueba"}],
      "tilesets": [{"firstgid": 1, "source": "tiles.tsj"},
                   {"firstgid": 101, "name": "objetos", "tiles": [{"id": 1, "type": "moss"}]}],
      "layers": [
        {"type": "tilelayer", "name": "suelo", "width": 4, "height": 3,
         "data": [0,0,0,0, 0,0,0,2, 1,1,1,2147483649]},
        {"type": "tilelayer", "name": "deco", "width": 4, "height": 3,
         "data": [1,1,1,1, 1,1,1,1, 1,1,1,1]},
        {"type": "objectgroup", "name": "cosas", "objects": [
          {"id": 1, "type": "spawn", "x": 0, "y": 8, "width": 8, "height": 8},
          {"id": 2, "type": "bat", "x": 16, "y": 0, "width": 8, "height": 8,
           "properties": [{"name": "range", "type": "float", "value": 2}, {"name": "dir", "type": "int", "value": -1}]},
          {"id": 3, "class": "exit", "x": 24, "y": 0, "width": 8, "height": 16},
          {"id": 4, "type": "rock", "gid": 1, "x": 8, "y": 16, "width": 8, "height": 8},
          {"id": 5, "type": "dragon", "x": 0, "y": 0, "width": 8, "height": 8},
          {"id": 6, "gid": 102, "x": 0, "y": 24, "width": 8, "height": 8}
        ]}
      ]
    }"#;

    const TSJ: &str = r#"{"type": "tileset", "tilewidth": 8, "tileheight": 8, "tiles": [
        {"id": 0, "properties": [{"name": "kind", "type": "string", "value": "solid"}]},
        {"id": 1, "type": "spike_left"}
    ]}"#;

    #[test]
    fn loads_tiles_objects_and_zones() {
        let read = |p: &str| {
            if p == "tiles.tsj" {
                Ok(TSJ.to_string())
            } else {
                no_files(p)
            }
        };
        let (m, warns) = load_map(MAP, "prueba.tmj", &read).unwrap();
        assert_eq!(m.name, "Prueba");
        assert_eq!((m.w, m.h), (4, 3));
        assert_eq!(m.tile(0, 2), Tile::Solid);
        assert_eq!(m.tile(3, 2), Tile::Solid, "los bits de volteo se ignoran");
        assert_eq!(m.tile(3, 1), Tile::Spike(Dir4::Left));
        assert_eq!(m.tile(0, 0), Tile::Empty, "la capa deco no colisiona");
        assert_eq!(m.spawn, (4, 16));
        assert_eq!(m.objects.len(), 3);
        assert_eq!(
            m.objects[2].kind,
            StrikeKind::Moss,
            "hereda la clase del tile"
        );
        let bat = m.objects[0];
        assert_eq!(
            (bat.kind, bat.center, bat.range, bat.dir),
            (StrikeKind::Bat, (20.0, 4.0), 16.0, -1)
        );
        assert_eq!(
            m.objects[1].center,
            (12.0, 12.0),
            "objeto-tile con origen abajo"
        );
        assert_eq!(
            m.zones,
            vec![Zone {
                kind: ZoneKind::Exit,
                x: 24,
                y: 0,
                w: 8,
                h: 16
            }]
        );
        assert_eq!(warns.len(), 1, "{warns:?}");
    }

    #[test]
    fn rejects_bad_maps() {
        let read = |_: &str| Ok(TSJ.to_string());
        let wrong_size = MAP.replace("\"tilewidth\": 8", "\"tilewidth\": 12");
        assert!(load_map(&wrong_size, "x", &read).is_err());
        let no_spawn = MAP.replace("\"spawn\"", "\"moss\"");
        assert!(
            load_map(&no_spawn, "x", &read)
                .unwrap_err()
                .contains("spawn")
        );
        assert!(
            load_map(MAP, "x", &no_files).is_err(),
            "tileset externo ausente"
        );
    }

    #[test]
    fn double_resolution_maps_scale_to_physics_units() {
        let read = |p: &str| {
            if p == "tiles.tsj" {
                Ok(TSJ.to_string())
            } else {
                no_files(p)
            }
        };
        let doubled = MAP
            .replace(
                "\"tilewidth\": 8, \"tileheight\": 8",
                "\"tilewidth\": 16, \"tileheight\": 16",
            )
            .replace(
                "\"x\": 16, \"y\": 0, \"width\": 8, \"height\": 8",
                "\"x\": 32, \"y\": 0, \"width\": 16, \"height\": 16",
            )
            .replace(
                "\"x\": 0, \"y\": 8, \"width\": 8, \"height\": 8",
                "\"x\": 0, \"y\": 16, \"width\": 16, \"height\": 16",
            );
        let (a, _) = load_map(MAP, "a", &read).unwrap();
        let (b, _) = load_map(&doubled, "b", &read).unwrap();
        assert_eq!(a.spawn, b.spawn);
        assert_eq!(a.objects[0].center, b.objects[0].center);
    }
}
