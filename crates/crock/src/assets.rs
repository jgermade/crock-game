//! Localización de assets y recarga en caliente de constantes.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crock_sim::{TileMap, Tuning};

/// Busca la carpeta `assets`: `$CROCK_ASSETS`, `./assets`, junto al
/// ejecutable, o la del repositorio (útil con `cargo run`).
pub fn assets_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("CROCK_ASSETS") {
        return p.into();
    }
    let mut candidates = vec![PathBuf::from("assets")];
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        candidates.push(dir.join("assets"));
    }
    candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets"));
    candidates
        .into_iter()
        .find(|p| p.is_dir())
        .unwrap_or_else(|| "assets".into())
}

/// Un capítulo: nombre y salas en orden.
pub struct Chapter {
    pub name: String,
    pub rooms: Vec<PathBuf>,
}

/// Capítulos disponibles: los que listan `levels/capitulo*.txt` y, al
/// final, las salas de pruebas sueltas.
pub fn chapters(dir: &Path) -> Vec<Chapter> {
    let levels = dir.join("levels");
    let mut lists: Vec<PathBuf> = std::fs::read_dir(&levels)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.extension().is_some_and(|e| e == "txt")
                        && p.file_name()
                            .is_some_and(|n| n.to_string_lossy().starts_with("capitulo"))
                })
                .collect()
        })
        .unwrap_or_default();
    lists.sort();
    let mut out = Vec::new();
    for list in lists {
        let Ok(src) = std::fs::read_to_string(&list) else {
            continue;
        };
        let rooms: Vec<PathBuf> = src
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| levels.join(l))
            .collect();
        let stem = list.file_stem().unwrap().to_string_lossy();
        let name = stem.replacen("capitulo", "Capítulo ", 1);
        out.push(Chapter { name, rooms });
    }
    for sandbox in ["golpe", "movimiento", "cornisas"] {
        out.push(Chapter {
            name: format!("Pruebas: {sandbox}"),
            rooms: vec![levels.join(format!("{sandbox}.tmj"))],
        });
    }
    out
}

/// Carga las salas de un capítulo. Avisos por la consola.
pub fn load_chapter(ch: &Chapter) -> Result<Vec<TileMap>, String> {
    let mut rooms = Vec::new();
    for path in &ch.rooms {
        let (map, warnings) = crock_sim::tiled::load_file(path)?;
        for w in warnings {
            eprintln!("[crock] {w}");
        }
        rooms.push(map);
    }
    if rooms.is_empty() {
        return Err(format!("{}: sin salas", ch.name));
    }
    Ok(rooms)
}

/// Vigila `tuning.cfg` comprobando su fecha de modificación cada poco.
/// Un `stat` cada 250 ms es despreciable y no necesita hilo propio.
pub struct TuningWatcher {
    path: PathBuf,
    mtime: Option<SystemTime>,
    next_check_ns: u64,
}

impl TuningWatcher {
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join("tuning.cfg"),
            mtime: None,
            next_check_ns: 0,
        }
    }

    /// Devuelve nuevas constantes si el fichero ha cambiado (o en la primera llamada).
    pub fn poll(&mut self, now_ns: u64) -> Option<(Tuning, Vec<String>)> {
        if now_ns < self.next_check_ns {
            return None;
        }
        self.next_check_ns = now_ns + 250_000_000;
        let mtime = std::fs::metadata(&self.path)
            .and_then(|m| m.modified())
            .ok();
        if mtime.is_none() || mtime == self.mtime {
            return None;
        }
        self.mtime = mtime;
        let src = std::fs::read_to_string(&self.path).ok()?;
        Some(Tuning::parse(&src))
    }
}
