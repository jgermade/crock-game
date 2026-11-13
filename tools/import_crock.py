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
    cells = [place(src.crop(boxes[i])) for _, ix in ANIMS for i in ix]
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
