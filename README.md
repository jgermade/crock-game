# cROCK

Platformer 2D de precisión: un troglodita con una porra que, en lugar de doble
salto, **golpea objetos para impulsarse**. Ver [docs/ROADMAP.md](docs/ROADMAP.md).

Estado: **Fase 0 (esqueleto) y Fase 1 (movimiento base)**.

## Estructura

| Ruta | Qué es |
|---|---|
| `crates/sim` | Simulación determinista, sin SDL ni reloj: `step(&mut World, &Input)` a 240 Hz fijos. Tests incluidos. |
| `crates/crock` | Capa de plataforma SDL3: ventana, bucle, entrada (teclado + mando), render interpolado, métricas. |
| `assets/tuning.cfg` | Constantes de movimiento; **se recargan en caliente** al guardar. |
| `assets/levels/test.txt` | Sala de prueba en ASCII (`#` sólido, `P` aparición). F5 la recarga. |

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
(exclusiva). Para pruebas automáticas: `--frames=N`, `--autoplay`,
`--screenshot=out.bmp`.

```bash
cargo test --workspace
```

## Controles

| Acción | Teclado | Mando |
|---|---|---|
| Mover | Flechas / WASD | Cruceta / stick izquierdo (8 direcciones) |
| Saltar (mantener = más alto) | Espacio / C / Z / K | A (botón sur) |
| Reiniciar | R | |
| Overlay de métricas | F1 | |
| Modo de presentación (vsync → adaptativo → inmediato) | F2 | |
| Limitador de fps (no, 60, 120, 144, 240, 360) | F3 | |
| Retardo antes de leer la entrada (0–8 ms) | F4 | |
| Recargar nivel | F5 | |
| Ventana → sin bordes → exclusiva | F11 / Alt+Enter | |

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
