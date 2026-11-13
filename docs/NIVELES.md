# Hacer salas con Tiled

Las salas son mapas de [Tiled](https://www.mapeditor.org/) en formato JSON
(`.tmj`) dentro de `assets/levels/`. Un capítulo es una lista de salas en
`assets/levels/capituloN.txt`, una por línea y en orden; al tocar la salida de
una sala se pasa a la siguiente.

## Convenciones

- **Tiles de 24×24 px** (el arte va a 3 px por unidad de física; el juego
  escala las coordenadas solo; también acepta mapas de 8 o 16 px). Mapa **no infinito**, capas de tiles
  guardadas en **CSV** (Mapa → Propiedades → Formato de capa de tiles).
- Tilesets externos: `cueva.tsj` (terreno) y `objetos.tsj` (objetos).
- Propiedad de mapa `name` (texto): nombre que se ve en el HUD.

### Capas de tiles (tileset `cueva`)

Cada tile tiene la propiedad `kind`: `solid`, `spike_up`, `spike_down`,
`spike_left`, `spike_right` o `deco`. Cualquier capa de tiles cuenta para la
colisión salvo las que empiezan por `deco` (sólo se dibujan).

Los pinchos no son sólidos: matan al tocarlos **moviéndose contra ellos** (unos
que apuntan arriba no matan si se sube a través de ellos). Su zona letal es la
mitad de la celda pegada a la base.

### Capa de objetos (tileset `objetos`)

Inserta los objetos como **objetos-tile** desde el tileset `objetos`: heredan
su clase del tile y se ven en el editor. También vale un rectángulo con la
clase escrita a mano.

| Clase | Qué es | Propiedades |
|---|---|---|
| `spawn` | Aparición de Crock (sus pies, centrados en el objeto). Obligatorio. | |
| `rock` | Roca: impulso normal, recarga el golpe. | |
| `fragile` | Roca frágil: se rompe y reaparece. | |
| `bone` | Hueso colgante: impulso fuerte, no recarga. | |
| `moss` | Musgo: impulso flojo, recarga. | |
| `beetle` | Escarabajo: patrulla su plataforma; golpearlo lo aturde. | `dir` (-1/1) |
| `bat` | Murciélago: vaivén horizontal; golpearlo lo derriba. | `range` (tiles, 3), `dir` |
| `fire` | Zona de fuego: quema la porra (sin golpes dentro). Redimensionable. | |
| `exit` | Salida de la sala. Redimensionable. | |

## Comprobar que una sala se puede pasar

```bash
cargo run --release -p crock-solver -- assets/levels/mi-sala.tmj --out=ruta.rep
cargo run --release -p crock -- --chapter=1 --room=3 --replay=ruta.rep
```

El solver es una búsqueda en haz sobre la simulación: encuentra *una* ruta
(no necesariamente la prevista ni la más bonita) con las constantes de
`tuning.cfg`. Si dice «sin salida» o «no encontrada», la sala probablemente no
se puede pasar; prueba con `--beam=2000 --secs=40` antes de rediseñarla.

Las rutas de `assets/replays/` se comprueban en los tests: tras editar una sala
o cambiar constantes, regenéralas con:

```bash
tools/solve_all.sh
```

## Prototipar en ASCII

`tools/ascii2tmj.py` convierte una sala en texto (mismos caracteres que los
tests: `#`, `^ v < >`, `P`, `o x b m e w`, `f`, `E`) a `.tmj` con autotiling y
decoración (matas, estalactitas y una capa `deco_rocas` con rocas grandes
sobre las caras de las paredes). Útil para bocetar rápido; después el `.tmj`
es la fuente. En Tiled, las rocas son piezas de varios tiles del tileset
`cueva` (2×2, 1×3, 1×2 y 1×1): se pintan en una capa cuyo nombre empiece por
`deco`, así que no afectan a la colisión.

## Arte

- `tools/gen_art.py` genera el terreno, los objetos y el fondo de cueva
  (`cueva.png`, `objetos.png`, `fondo_*.png`) y los `.tsj`. Si editas esos PNG a mano, no vuelvas a
  ejecutarlo (o lleva los cambios al script).
- Crock sale de su hoja de sprites, `assets/src/crock_hoja.png` (rasterizada
  del SVG original con `tools/svg_pixels.py`). `tools/import_crock.py` la
  recorta, alinea y escribe `assets/gfx/crock.png`; las animaciones que se
  usan están al principio del script.
