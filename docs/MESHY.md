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
| Paleta | Marrones del pelo, piel cálida, piel oscura de la túnica y madera de la porra |

**Negative prompt (si lo admite):**

```
3D render, realistic, smooth shading, gradients, anti-aliasing, blur, soft edges, text, watermark, background scenery, multiple characters, cropped feet, inconsistent size between frames, front view, perspective
```

---

## Personaje (prompt base)

```
Pixel art sprite, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair down his back and a long full beard covering his chest, big nose, warm tanned skin, sleeveless shaggy brown fur tunic to the knees with a ragged hem, barefoot, holding a short thick wooden club in his right hand. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background.
```

---

## Animaciones

### 1. strike_side — golpe lateral (6 frames)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 6-frame animation in one horizontal row: club wind-up behind the shoulder, fast powerful horizontal swing forward at chest height, impact pose with the club fully extended forward, follow-through, recover. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size and feet position in every frame.
```

### 2. strike_up — golpe hacia arriba (5 frames)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 5-frame animation in one horizontal row: slight crouch with the club low, explosive upward smash, impact pose with both arms fully extended overhead and the club pointing straight up, hold, recover. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size and feet position in every frame.
```

### 3. strike_down — golpe hacia abajo, en el aire (5 frames)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 5-frame animation in one horizontal row, in mid-air: club raised overhead, body curling, club smashed straight down below his feet with knees tucked up, impact pose, recover. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size in every frame.
```

### 4. strike_diag_up — golpe diagonal hacia arriba (5 frames)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 5-frame animation in one horizontal row: club low behind the hip, diagonal upward swing forward at 45 degrees, impact pose with the club pointing up and forward, follow-through, recover. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size and feet position in every frame.
```

### 5. strike_diag_down — golpe diagonal hacia abajo (5 frames)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 5-frame animation in one horizontal row, in mid-air: club raised above the shoulder, diagonal downward swing forward at 45 degrees, impact pose with the club pointing down and forward below his feet, follow-through, recover. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size in every frame.
```

### 6. idle — reposo (4 frames, bucle)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 4-frame seamless idle loop in one horizontal row: gentle breathing, club resting on his shoulder, slight sway, one frame with a blink. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size and feet position in every frame.
```

### 7. run — carrera (8 frames, bucle)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 8-frame seamless run cycle in one horizontal row: bouncy cartoon steps, hair and beard flowing behind, club held low in one hand. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size in every frame, feet on the same ground line.
```

### 8. jump — salto (2 frames)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 2-frame jump animation in one horizontal row: takeoff with knees bending up, rising with knees up and arms raised. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size in every frame.
```

### 9. fall — caída (2 frames, bucle)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 2-frame falling loop in one horizontal row: arms flailing upward, hair and beard blown upward, legs dangling. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size in every frame.
```

### 10. land — aterrizaje (2 frames)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 2-frame landing animation in one horizontal row: heavy squash into a crouch, then recovering to standing. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size and feet position in every frame.
```

### 11. hang — colgado de una cornisa (2 frames, bucle)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 2-frame loop in one horizontal row: hanging from a ledge edge on his right with one hand gripping above his head, club in the other hand, legs dangling, slight swing. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced, same size in every frame.
```

### 12. death — muerte (4 frames)

```
Pixel art sprite sheet, side view, 2D platformer character facing right: a friendly chunky caveman with very long messy dark-brown hair and a long full beard, big nose, tanned skin, shaggy brown fur tunic, barefoot, holding a short thick wooden club. 4-frame comic death animation in one horizontal row: hit and knocked back, flying backwards, falling, lying flat on his back with dizzy eyes. 16-bit style, about 40 pixels tall, crisp 1-pixel dark outline, limited palette, no anti-aliasing, no gradients, transparent background, frames evenly spaced.
```

---

## Entrega

1. Un PNG por animación, con el nombre de la sección: `strike_side.png`, `run.png`…
2. Guárdalos en `assets/src/crock/`.
3. Si salen a 2× o 4×, mejor: se reducen exactos con vecino más próximo.
4. `tools/import_crock.py` los recorta, los alinea (pies abajo, cuerpo centrado),
   reduce la paleta y escribe `assets/gfx/crock.png`.
