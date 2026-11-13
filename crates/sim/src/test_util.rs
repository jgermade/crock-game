/// Carga una sala de `assets/levels` para los tests.
pub fn level(name: &str) -> crate::TileMap {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/levels")
        .join(format!("{name}.tmj"));
    let (map, warnings) = crate::tiled::load_file(&path).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    map
}
