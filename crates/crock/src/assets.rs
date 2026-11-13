//! Localización de assets y recarga en caliente de constantes.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crock_sim::{TileMap, Tuning};

pub const FALLBACK_LEVEL: &str = include_str!("../../../assets/levels/test.txt");

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

pub fn load_level(dir: &Path) -> TileMap {
    let path = dir.join("levels/test.txt");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|_| FALLBACK_LEVEL.to_string());
    TileMap::parse(&src).unwrap_or_else(|e| {
        eprintln!(
            "[crock] error en {}: {e}; uso el nivel embebido",
            path.display()
        );
        TileMap::parse(FALLBACK_LEVEL).expect("nivel embebido válido")
    })
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
