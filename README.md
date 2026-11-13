# cROCK

Platformer 2D de precisión: un troglodita con una porra que, en lugar de doble
salto, **golpea objetos para impulsarse**. Ver [docs/ROADMAP.md](docs/ROADMAP.md).

Estado: Fases 0 y 1 hechas; **Fase 2 (prototipo del golpe)** lista para probar y decidir si el golpe encadenado divierte.

## Estructura

| Ruta | Qué es |
|---|---|
| `crates/sim` | Simulación determinista, sin SDL ni reloj: `step(&mut World, &Input)` a 240 Hz fijos. Tests incluidos. |
| `crates/crock` | Capa de plataforma SDL3: ventana, bucle, entrada (teclado + mando), render interpolado, métricas. |
| `assets/tuning.cfg` | Constantes de movimiento; **se recargan en caliente** al guardar. |
| `assets/levels/*.txt` | Salas en ASCII: `golpe` (Fase 2) y `movimiento` (Fase 1). F5 recarga, F6 cambia. |
| `assets/replays/` | Partidas grabadas (entradas por tick) que se reproducen exactamente. |

## Compilar y jugar

SDL3 se compila desde fuente y se enlaza en estático (hace falta `cmake` y un
compilador de C). En Linux, además, las cabeceras de X11/Wayland/audio:

```bash
sudo apt install cmake libx11-dev libxext-dev libxcursor-dev libxi-dev libxrandr-dev libxss-dev libxfixes-dev libxtst-dev libwayland-dev libxkbcommon-dev libegl-dev libasound2-dev libpulse-dev libudev-dev libdbus-1-dev
```

```bash
cargo run --release -p crock
```

Opciones: `--vsync=on|adaptive|off`, `--fps=N` (limitador), `--fullscreen`
(exclusiva), `--level=golpe|movimiento`. Grabar y reproducir partidas:
`--record=partida.rep` (guarda al salir, desde el último reinicio) y
`--replay=partida.rep`. Para pruebas automáticas: `--frames=N`, `--autoplay`,
`--screenshot=out.bmp`.

```bash
cargo run --release -p crock -- --replay=assets/replays/cruce-golpe.rep
```

Una repetición sólo es válida con la misma sala y las mismas constantes con
las que se grabó.

```bash
cargo test --workspace
```

## Controles

| Acción | Teclado | Mando |
|---|---|---|
| Mover | Flechas / WASD | Cruceta / stick izquierdo (8 direcciones) |
| Saltar (mantener = más alto) | Espacio / C / Z / K | A (botón sur) |
| Golpear con la porra (en la dirección pulsada) | X / J / Shift izq. | X (botón oeste) / RB |
| Reiniciar | R | |
| Overlay de métricas | F1 | |
| Modo de presentación (vsync → adaptativo → inmediato) | F2 | |
| Limitador de fps (no, 60, 120, 144, 240, 360) | F3 | |
| Retardo antes de leer la entrada (0–8 ms) | F4 | |
| Recargar sala / cambiar de sala | F5 / F6 | |
| Ventana → sin bordes → exclusiva | F11 / Alt+Enter | |

## El golpe (Fase 2)

Golpear un objeto lanza al troglodita en la dirección **opuesta** a la del
golpe, a velocidad fija, durante un instante sin gravedad; luego la física
normal. La dirección (8) se fija en el instante de pulsar; sin dirección, golpea
hacia donde mira. Si al pulsar no hay nada a tiro, el golpe sigue activo
`strike_window` segundos y conecta si algo entra en la caja; si no, falla y hay
un breve enfriamiento. Un golpe pulsado durante el hit-stop o el enfriamiento
queda en buffer, así que se puede encadenar sin esperar al frame exacto.

| Objeto | Mapa | Impulso | Recarga el golpe |
|---|---|---|---|
| Roca | `o` | normal | sí |
| Roca frágil (se rompe y reaparece) | `x` | normal | sí |
| Hueso colgante | `b` | fuerte (×1,35) | **no** |
| Musgo | `m` | flojo (×0,7) | sí |

Hay 1 golpe por recarga; tocar suelo también recarga. Con el overlay (F1) se ve
la caja de golpe mientras está activa. Para probar el **empuje adicional del
stick** (pregunta abierta del roadmap) basta con poner `strike_steer` a un valor
positivo en `tuning.cfg` y guardar.

## Latencia: qué hace el bucle

1. Espera (si hay limitador o retardo) con `sleep` + *spin-wait* en los últimos 2 ms.
2. Lee **todos** los eventos justo antes de simular. Las pulsaciones se
   guardan como flancos, así que una pulsación corta entre ticks nunca se pierde.
3. Simula los ticks de 1/240 s pendientes (acumulador, tope de 250 ms).
4. Dibuja interpolando entre los dos últimos ticks y presenta.

El overlay muestra el intervalo entre presents, el trabajo de CPU del frame,
ticks por frame y la latencia **pulsación de salto → retorno de present**
(medida con el timestamp del evento de SDL; no incluye el escaneo del monitor).
El retardo de F4 sirve para leer la entrada más tarde con vsync activo: si el
trabajo del frame cabe en lo que queda, la entrada llega más fresca.
