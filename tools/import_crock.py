#!/usr/bin/env python3
"""Importa la hoja de sprites de Crock (assets/src/crock_hoja.png) al juego.

La hoja original (pixel art de unos 27x41 px por figura sobre fondo plano)
se usa a su resolución nativa: el juego dibuja el arte a 3 px por unidad.
El script quita el fondo, separa las figuras, elige las de cada animación, las
alinea (pies abajo y cuerpo centrado, usando el pelo y la barba como ancla) y
escribe assets/gfx/crock.png con celdas de CELL x CELL.

    python3 tools/import_crock.py [--preview vista.png]

Orden de las celdas (lo que usa crates/crock/src/render.rs):
"""

import json
import os
import sys

from PIL import Image

CELL = 56
# Figuras de la hoja, en orden de lectura (ver --list).
ANIMS = [
    ("idle", [0, 1, 2, 3, 4]),
    ("run", [6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17]),
    ("jump", [19]),
    ("fall", [20, 21]),
    ("strike_side", [28]),
    ("strike_up", [26]),
    ("strike_down", [30]),
    ("land", [23]),
    ("hang", ["hang"]),  # se construye a partir de la figura 19 (ver make_hang)
]
HAIR = {(0x3C, 0x2A, 0x23), (0x4E, 0x39, 0x2B), (0x2B, 0x1E, 0x1D)}

__doc__ += "".join(f"\n    {name}: {len(ix)} celda(s)" for name, ix in ANIMS) + "\n"


def figures(im):
    """Quita el fondo (color de la esquina) y devuelve las cajas de las figuras."""
    im = im.convert("RGBA")
    W, H = im.size
    px = im.load()
    bg = px[0, 0][:3]
    for y in range(H):
        for x in range(W):
            if px[x, y][:3] == bg:
                px[x, y] = (0, 0, 0, 0)
    R = 2  # une mechones sueltos a su figura
    seen = [[False] * W for _ in range(H)]
    boxes = []
    for y in range(H):
        for x in range(W):
            if px[x, y][3] == 0 or seen[y][x]:
                continue
            stack, seen[y][x] = [(x, y)], True
            x0 = x1 = x
            y0 = y1 = y
            n = 0
            while stack:
                cx, cy = stack.pop()
                n += 1
                x0, x1, y0, y1 = min(x0, cx), max(x1, cx), min(y0, cy), max(y1, cy)
                for dy in range(-R, R + 1):
                    for dx in range(-R, R + 1):
                        nx, ny = cx + dx, cy + dy
                        if 0 <= nx < W and 0 <= ny < H and px[nx, ny][3] and not seen[ny][nx]:
                            seen[ny][nx] = True
                            stack.append((nx, ny))
            if n > 30:
                boxes.append((x0, y0, x1 + 1, y1 + 1))
    boxes.sort(key=lambda b: (round((b[1] + b[3]) / 2 / 50), b[0]))
    return im, boxes


SKIN_DARK = (0x83, 0x53, 0x44)


SKIN = (0xD1, 0x8E, 0x59)
SKIN_LIGHT = (0xE3, 0x9E, 0x65)
SKIN_SHADE = (0xA4, 0x6A, 0x4C)
SKIN_DARK = (0x83, 0x53, 0x44)
OUTLINE = (0x0B, 0x09, 0x14)


def make_hang(fig):
    """Colgado de una cornisa, a partir de la figura 19 (en el aire, piernas
    colgando): se quita el brazo que sube detrás de la cabeza y se dibujan
    los brazos hacia delante y arriba, con los puños agarrando el borde."""
    W, H = fig.size
    pad = 4  # sitio a la derecha para los puños
    out = Image.new("RGBA", (W + pad, H), (0, 0, 0, 0))
    out.alpha_composite(fig, (0, 0))
    px = out.load()
    W += pad
    # 1) Fuera el brazo de atrás: se vacía la franja izquierda superior y se
    #    rehace el borde de la melena con el pelo de su derecha.
    for y in range(0, 27):
        for x in range(0, 9):
            px[x, y] = (0, 0, 0, 0)
    # También la mano de delante original (arriba a la derecha), que se redibuja.
    for y in range(0, 7):
        for x in range(17, W):
            c = px[x, y]
            if c[3] and c[:3] not in HAIR:
                px[x, y] = (0, 0, 0, 0)
    for y in range(4, 27):
        row = [x for x in range(9, W) if px[x, y][3] and px[x, y][:3] in HAIR]
        if not row:
            continue
        x0 = row[0]
        # Melena redondeada arriba y con mechones irregulares en el borde.
        edge = (13 - y if y <= 9 else 5) - (1 if y > 9 and y % 5 in (1, 2) else 0)
        for x in range(max(edge, 2), x0):
            src = px[min(x0 + (x0 - x) % 3, W - 1), y]
            px[x, y] = src if src[3] and src[:3] in HAIR else (0x3C, 0x2A, 0x23, 255)
        px[max(edge, 2) - 1, y] = OUTLINE + (255,)

    def thick_line(x0, y0, x1, y1, colors, width):
        n = max(abs(x1 - x0), abs(y1 - y0))
        for k in range(n + 1):
            t = k / n
            cx, cy = x0 + (x1 - x0) * t, y0 + (y1 - y0) * t
            for d in range(width):
                # perpendicular aproximada (el brazo va hacia arriba-derecha)
                x, y = round(cx + d * 0.7), round(cy + d * 0.7)
                if 0 <= x < W and 0 <= y < H:
                    px[x, y] = colors[min(d, len(colors) - 1)] + (255,)

    def outline_around(color_set):
        add = []
        for y in range(H):
            for x in range(W):
                if px[x, y][3]:
                    continue
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    nx, ny = x + dx, y + dy
                    if 0 <= nx < W and 0 <= ny < H and px[nx, ny][3] and px[nx, ny][:3] in color_set:
                        add.append((x, y))
                        break
        for x, y in add:
            px[x, y] = OUTLINE + (255,)

    # 2) Brazo de atrás (más oscuro), algo por detrás y a la izquierda.
    thick_line(18, 19, 22, 5, [SKIN_SHADE, SKIN_DARK, SKIN_DARK], 3)
    for x, y in ((21, 2), (22, 2), (23, 2), (21, 3), (22, 3), (23, 3), (24, 3), (22, 4), (23, 4)):
        px[x, y] = SKIN_DARK + (255,)
    # 3) Brazo de delante: del hombro (junto a la barba) al borde, con luz.
    thick_line(21, 20, 26, 5, [SKIN_LIGHT, SKIN, SKIN_SHADE], 3)
    for x, y in ((25, 1), (26, 1), (27, 1), (24, 2), (25, 2), (26, 2), (27, 2), (28, 2),
                 (24, 3), (25, 3), (26, 3), (27, 3), (28, 3), (25, 4), (26, 4), (27, 4)):
        px[x, y] = SKIN + (255,)
    for x, y in ((25, 1), (26, 1), (24, 2)):
        px[x, y] = SKIN_LIGHT + (255,)
    for x, y in ((27, 4), (28, 3), (26, 4)):
        px[x, y] = SKIN_SHADE + (255,)
    outline_around({SKIN, SKIN_LIGHT, SKIN_SHADE, SKIN_DARK})
    return out


def place(fig):
    """Coloca la figura en una celda: pies en la última fila y el pelo
    (cabeza, barba y melena) centrado en horizontal."""
    px = fig.load()
    xs = [x for y in range(fig.height) for x in range(fig.width) if px[x, y][:3] in HAIR and px[x, y][3]]
    xs.sort()
    anchor = xs[len(xs) // 2] if xs else fig.width // 2
    cell = Image.new("RGBA", (CELL, CELL), (0, 0, 0, 0))
    cell.alpha_composite(fig, (CELL // 2 - anchor, CELL - fig.height))
    return cell


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    src, boxes = figures(Image.open(os.path.join(root, "assets/src/crock_hoja.png")))
    if "--list" in sys.argv:
        for i, b in enumerate(boxes):
            print(i, b)
        return
    def figure(i):
        if i == "hang":
            return make_hang(src.crop(boxes[19]))
        return src.crop(boxes[i])

    cells = [place(figure(i)) for _, ix in ANIMS for i in ix]
    sheet = Image.new("RGBA", (CELL * len(cells), CELL), (0, 0, 0, 0))
    for n, c in enumerate(cells):
        sheet.alpha_composite(c, (n * CELL, 0))
    sheet.save(os.path.join(root, "assets/gfx/crock.png"))
    first = 0
    for name, ix in ANIMS:
        print(f"{name}: celdas {first}..{first + len(ix) - 1}")
        first += len(ix)
    if "--preview" in sys.argv:
        out = sys.argv[sys.argv.index("--preview") + 1]
        prev = Image.new("RGBA", sheet.size, (60, 52, 70, 255))
        prev.alpha_composite(sheet)
        prev.resize((sheet.width * 3, sheet.height * 3), Image.NEAREST).save(out)


if __name__ == "__main__":
    main()
