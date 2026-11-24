# Sprites de Crock con Meshy

Prompts para generar los sprites de Crock en pixel art. Cada prompt de
animación es completo: se puede pegar tal cual.

**Imagen de referencia:** `assets/src/crock_hoja.png` (súbela como referencia
de estilo y personaje si Meshy lo permite).

## Especificaciones comunes

| Parámetro | Valor |
|---|---|
| Estilo | Pixel art 16 bits, contorno oscuro de 1 px, sin antialiasing ni degradados |
| Vista | Lateral, mirando a la **derecha** (el juego voltea el sprite) |
| Tamaño del personaje | ~41 px de alto (o 2× / 4× exactos: 82 / 164 px) |
| Celda | 56 × 56 px por frame (o 112 / 224 a 2× / 4×) |
| Colocación | Pies en la última fila de la celda, cuerpo centrado, igual en todos los frames |
| Fondo | Transparente (o un color plano único) |
| Disposición | Una fila horizontal por animación, frames separados por igual |
| Frames | 6–12 por animación (ver cada una); fluido, con smear frames en los golpes |
| Paleta | Marrones del pelo, piel cálida, piel oscura de la túnica y madera de la porra |

**Negative prompt (si lo admite):**

```
3D render, realistic, smooth shading, gradients, anti-aliasing, blur, soft edges, text, watermark, background scenery, multiple characters, cropped feet, inconsistent size between frames, front view, perspective
```

## Fluidez

- Entre **6 y 12 frames** por animación, con **smear frames** (porra y pelo
  estirados) en los golpes y **follow-through** del pelo y la barba.
- Ritmo en el juego: ciclos a **12–15 fps** (reposo, carrera), golpes y
  saltos a **20–24 fps**.
- Los golpes son instantáneos en el juego (no hay espera para no añadir
  latencia): la **preparación es un solo frame** y el **impacto llega en el
  frame 4**. Lo que sigue (seguimiento y recuperación) puede durar lo que
  quieras y es lo que da la fluidez.
- Si Meshy no aguanta tantos frames en una imagen, pide la animación en dos
  tiras (frames 1–5 y 6–10) con el mismo prompt, añadiendo `first half` o
  `second half`.

---

## Personaje (prompt base)

```
Pixel art sprite, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair down his back and a long full beard covering his chest, big nose, warm tanned skin, sleeveless shaggy brown fur tunic to the knees with a ragged hem, barefoot, holding a short thick wooden club in his right hand. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background.
```

---

## Animaciones

### 1. strike_side — golpe lateral (10 frames)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 10-frame smooth animation in one horizontal row: frame 1 quick club wind-up behind the shoulder, frames 2-3 fast horizontal swing forward at chest height with smear frames, frame 4 impact pose with the club fully extended forward, frames 5-7 follow-through with hair and beard trailing, frames 8-10 recover to neutral. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size and feet position in every frame.
```

### 2. strike_up — golpe hacia arriba (10 frames)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 10-frame smooth animation in one horizontal row: frame 1 slight crouch with the club low, frames 2-3 explosive upward smash with smear frames, frame 4 impact pose with both arms fully extended overhead and the club pointing straight up, frames 5-7 hold and settle with hair bouncing, frames 8-10 recover to neutral. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size and feet position in every frame.
```

### 3. strike_down — golpe hacia abajo, en el aire (10 frames)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 10-frame smooth animation in one horizontal row, in mid-air: frame 1 club raised overhead, frames 2-3 body curling as the club comes down with smear frames, frame 4 impact pose with the club straight down below his feet and knees tucked up, frames 5-7 recoil with hair flying up, frames 8-10 recover to a neutral mid-air pose. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size in every frame.
```

### 4. strike_diag_up — golpe diagonal hacia arriba (10 frames)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 10-frame smooth animation in one horizontal row: frame 1 club low behind the hip, frames 2-3 diagonal upward swing forward at 45 degrees with smear frames, frame 4 impact pose with the club pointing up and forward, frames 5-7 follow-through, frames 8-10 recover to neutral. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size and feet position in every frame.
```

### 5. strike_diag_down — golpe diagonal hacia abajo (10 frames)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 10-frame smooth animation in one horizontal row, in mid-air: frame 1 club raised above the shoulder, frames 2-3 diagonal downward swing forward at 45 degrees with smear frames, frame 4 impact pose with the club pointing down and forward below his feet, frames 5-7 follow-through, frames 8-10 recover to a neutral mid-air pose. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size in every frame.
```

### 6. idle — reposo (12 frames, bucle)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 12-frame smooth seamless idle loop in one horizontal row: slow gentle breathing with the chest and shoulders rising and falling, club resting on his shoulder, subtle sway of hair and beard, a blink in frames 9-10. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size and feet position in every frame.
```

### 7. run — carrera (12 frames, bucle)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 12-frame smooth seamless run cycle in one horizontal row: bouncy cartoon steps with contact, down, passing and up poses for each leg, body bobbing up and down, hair and beard flowing behind with follow-through, club swinging slightly in one hand. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size in every frame, feet on the same ground line.
```

### 8. jump — salto (6 frames)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 6-frame smooth jump animation in one horizontal row: anticipation crouch, takeoff stretch, rising with knees coming up, arms raised, hair lagging behind, reaching the apex. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size in every frame.
```

### 9. fall — caída (6 frames, bucle)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 6-frame smooth seamless falling loop in one horizontal row: arms flailing upward, hair and beard blown upward and fluttering, legs dangling and kicking slightly. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size in every frame.
```

### 10. land — aterrizaje (6 frames)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 6-frame smooth landing animation in one horizontal row: feet touching down, heavy squash into a deep crouch with hair bouncing down, stretch back up, settling to standing. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size and feet position in every frame.
```

### 11. hang — colgado de una cornisa (8 frames, bucle)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 8-frame smooth seamless loop in one horizontal row: hanging from a ledge edge on his right with one hand gripping above his head, club in the other hand, legs dangling with a slow pendulum swing, hair swaying. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size in every frame.
```

### 12. death — muerte (10 frames)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 10-frame smooth comic death animation in one horizontal row: hit flash pose, knocked back, flying backwards with limbs and hair flailing, hitting the ground, small bounce, lying flat on his back with dizzy eyes. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced.
```

---

## Entrega

1. Un PNG por animación, con el nombre de la sección: `strike_side.png`, `run.png`…
2. Guárdalos en `assets/src/crock/`.
3. Si salen a 2× o 4×, mejor: se reducen exactos con vecino más próximo.
4. `tools/import_crock.py` los recorta, los alinea (pies abajo, cuerpo centrado),
   reduce la paleta y escribe `assets/gfx/crock.png`.
