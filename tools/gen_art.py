#!/usr/bin/env python3
"""Genera el pixel art de cROCK (a 24 px por tile) y los tilesets de Tiled.

La física trabaja a 8 unidades por tile; el arte va al doble de resolución
(cada unidad de juego son 3×3 píxeles de arte; Crock viene aparte, de su
hoja de sprites: tools/import_crock.py). Ejecutar desde la raíz:

    python3 tools/gen_art.py [--preview salida.png]

Salida:
    assets/gfx/cueva.png       tiles 16x16 (colisión + decoración)
    assets/gfx/objetos.png     objetos 16x16 (golpeables, enemigos, zonas)
    assets/levels/cueva.tsj, assets/levels/objetos.tsj

Piedra, rocas, pinchos, musgo y fuego se generan geométricamente (con
sombreado y contorno); Crock, el hueso y el murciélago están dibujados a
mano como matrices de caracteres.
"""

import json
import math
import os
import random
import sys

from PIL import Image

T = 24  # píxeles de arte por tile (3 por unidad de juego)
K = T / 16  # las formas se diseñan en coordenadas de 16 y se muestrean a T

PALETTE = {
    ".": None,
    "k": (22, 18, 26),  # contorno
    # piedra (de oscuro a claro)
    "0": (34, 28, 38),
    "1": (52, 44, 54),
    "2": (74, 64, 70),
    "3": (98, 86, 86),
    "4": (128, 112, 104),
    "5": (160, 142, 124),
    # musgo
    "g": (58, 104, 50),
    "G": (96, 150, 64),
    "Y": (150, 196, 92),
    # hueso
    "b": (236, 228, 204),
    "B": (188, 174, 144),
    "n": (130, 116, 92),
    # roca frágil
    "t": (196, 168, 124),
    "T": (226, 204, 160),
    "u": (140, 112, 78),
    # escarabajo
    "c": (44, 70, 124),
    "C": (84, 128, 196),
    "D": (150, 196, 240),
    # murciélago
    "p": (72, 48, 92),
    "P": (116, 82, 142),
    "Q": (156, 120, 182),
    "r": (236, 70, 58),
    # fuego
    "R": (168, 44, 34),
    "o": (236, 120, 36),
    "y": (255, 214, 92),
    "W": (255, 248, 210),
    # Crock: piel y melena
    "s": (226, 168, 112),
    "S": (178, 118, 74),
    "H": (58, 30, 20),  # pelo: sombra
    "M": (96, 54, 30),  # pelo: medio
    "L": (138, 84, 44),  # pelo: luz
    "e": (250, 244, 230),  # blanco del ojo
    "a": (250, 212, 162),  # piel con luz
    "q": (128, 70, 44),  # contorno de la piel
    "l": (178, 116, 60),  # pelo: brillo
    "A": (214, 158, 92),  # pelo: brillo fuerte
    "O": (40, 22, 18),  # contorno del pelo
    # porra
    "w": (128, 86, 46),
    "x": (176, 124, 72),
    "X": (210, 164, 106),
    # salida
    "z": (250, 234, 160),
    "Z": (206, 178, 96),
}


class Canvas:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.px = [["."] * w for _ in range(h)]

    def set(self, x, y, c):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = c

    def get(self, x, y):
        if 0 <= x < self.w and 0 <= y < self.h:
            return self.px[y][x]
        return "."

    def outline(self, c="k"):
        """Contorno exterior de 1 px alrededor de lo opaco."""
        add = []
        for y in range(self.h):
            for x in range(self.w):
                if self.px[y][x] != ".":
                    continue
                if any(self.get(x + dx, y + dy) not in (".", c) for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))):
                    add.append((x, y))
        for x, y in add:
            self.px[y][x] = c
        return self


def u(v):
    """Centro del píxel `v` de salida en coordenadas de diseño (16)."""
    return (v + 0.5) / K


def dot(c, x, y, ch):
    """Pinta el píxel de diseño (x, y) como su bloque escalado."""
    for yy in range(int(y * K), int((y + 1) * K)):
        for xx in range(int(x * K), int((x + 1) * K)):
            c.set(xx, yy, ch)


def scaled(src):
    """Escala un sprite dibujado a 16 px a T px (vecino más próximo)."""
    c = Canvas(T, T)
    for y in range(T):
        for x in range(T):
            c.set(x, y, src.get(int(x / K), int(y / K)))
    return c


def from_rows(rows, w, h):
    c = Canvas(w, h)
    for y, row in enumerate(rows):
        for x, ch in enumerate(row.ljust(w, ".")[:w]):
            c.px[y][x] = ch
    return c


# --- Piedra: celdas de Voronoi con luz arriba-izquierda ----------------------


def stone(seed, dark=False):
    rnd = random.Random(seed)
    pts = [(rnd.uniform(0, T), rnd.uniform(0, T)) for _ in range(5)]
    c = Canvas(T, T)

    def nearest(x, y):
        best = None
        for i, (px, py) in enumerate(pts):
            for ox in (-T, 0, T):
                for oy in (-T, 0, T):
                    d = (x - px - ox) ** 2 + (y - py - oy) ** 2
                    if best is None or d < best[0]:
                        best = (d, i)
        return best[1]

    cell = [[nearest(x + 0.5, y + 0.5) for x in range(T)] for y in range(T)]
    ramp = "0112" if dark else "1234"
    for y in range(T):
        for x in range(T):
            i = cell[y][x]
            edge_tl = cell[(y - 1) % T][x] != i or cell[y][(x - 1) % T] != i
            edge_br = cell[(y + 1) % T][x] != i or cell[y][(x + 1) % T] != i
            base = 2 if (i % 3) else 1
            if edge_br:
                v = 0
            elif edge_tl:
                v = 3
            else:
                v = base
                if rnd.random() < 0.06:
                    v = min(3, v + 1)
            c.set(x, y, ramp[v])
    return c


def stone_top(seed):
    c = stone(seed)
    rnd = random.Random(seed + 99)
    for x in range(T):
        depth = int((3 + (1 if rnd.random() < 0.35 else 0) + (1 if rnd.random() < 0.15 else 0)) * K)
        for y in range(depth):
            c.set(x, y, "Y" if y == 0 and rnd.random() < 0.5 else "G" if y < 2 else "g")
        c.set(x, depth, "1")
    return c


def fangs():
    """Pinchos hacia arriba: dos colmillos de hueso en la mitad inferior."""
    c = Canvas(T, T)
    for cx in (4, 12):
        for y in range(T):
            if u(y) < 5:
                continue
            half = (u(y) - 5) * 4 / 10.5
            for x in range(T):
                dx = u(x) - cx
                if abs(dx) <= half + 0.25:
                    c.set(x, y, "b" if dx < -0.5 else "B" if dx < 1.2 else "n")
    for x in range(T):
        c.set(x, T - 1, "n")
    c.outline()
    return c


def rotate(c, times):
    for _ in range(times % 4):
        n = Canvas(c.h, c.w)
        for y in range(c.h):
            for x in range(c.w):
                n.set(c.h - 1 - y, x, c.px[y][x])
        c = n
    return c


def stalactite():
    c = Canvas(T, T)
    for y in range(T):
        if u(y) >= 11:
            continue
        half = 5.5 * (1 - u(y) / 11)
        for x in range(T):
            dx = u(x) - 8
            if abs(dx) <= half:
                c.set(x, y, "4" if dx < -1.5 else "3" if dx < 1 else "2")
    c.outline()
    return c


def pebbles():
    c = Canvas(T, T)
    for cx, cy, r in ((4, 14, 2.2), (10, 14.5, 1.6), (13, 13.8, 1.2)):
        for y in range(T):
            for x in range(T):
                d = math.hypot(u(x) - cx, u(y) - cy)
                if d <= r:
                    c.set(x, y, "4" if (u(x) - cx) + (u(y) - cy) < -0.5 else "3")
    c.outline()
    return c


# --- Objetos -------------------------------------------------------------------

BAYER = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]


def ball(ramp, r=6.5, cx=8, cy=8.5, squash=1.0):
    """Esfera con luz arriba-izquierda y tramado ordenado."""
    c = Canvas(T, T)
    lx, ly, lz = -0.55, -0.65, 0.5
    for y in range(T):
        for x in range(T):
            nx = (u(x) - cx) / r
            ny = (u(y) - cy) / (r * squash)
            d2 = nx * nx + ny * ny
            if d2 > 1:
                continue
            nz = math.sqrt(1 - d2)
            light = max(0.0, nx * lx + ny * ly + nz * lz)
            v = light * (len(ramp) - 1) + (BAYER[y % 4][x % 4] / 16 - 0.5) * 0.9
            c.set(x, y, ramp[max(0, min(len(ramp) - 1, round(v)))])
    c.outline()
    return c


def fragile():
    c = ball("utTT", r=6.5)
    for x, y in ((8, 3), (8, 4), (7, 5), (7, 6), (8, 7), (9, 8), (9, 9), (8, 10), (7, 11), (5, 7), (4, 8), (10, 6), (11, 5), (12, 5)):
        if c.get(int(x * K), int(y * K)) not in (".", "k"):
            dot(c, x, y, "k")
    return c


BONE = [
    "......kk.kk.....",
    ".....kbbkbbk....",
    ".....kbBbbBk....",
    "......kbbBk.....",
    ".......kbBk.....",
    ".......kbBk.....",
    ".......kbBk.....",
    ".......kbBk.....",
    ".......kbBk.....",
    ".......kbBk.....",
    ".......kbBk.....",
    "......kbbBk.....",
    ".....kbBbbBk....",
    ".....kbbkbBk....",
    "......kk.kk.....",
    "................",
]


def moss():
    c = Canvas(T, T)
    rnd = random.Random(7)
    for y in range(T):
        for x in range(T):
            nx, ny = (u(x) - 8) / 7.5, (u(y) - 10) / 5
            if nx * nx + ny * ny <= 1 + rnd.uniform(-0.15, 0.1):
                shade = -ny * 1.2 - nx * 0.4 + rnd.uniform(-0.4, 0.4)
                c.set(x, y, "Y" if shade > 0.8 else "G" if shade > -0.2 else "g")
    for x in (3, 7, 10, 13):  # hojitas
        dot(c, x, 4, "G")
        dot(c, x, 3, "Y")
    c.outline()
    return c


def beetle():
    c = Canvas(T, T)
    # caparazón: media elipse
    for y in range(T):
        for x in range(T):
            nx, ny = (u(x) - 7) / 6.5, (u(y) - 12) / 7
            if ny <= 0.15 and nx * nx + ny * ny <= 1:
                light = -nx * 0.5 - ny * 0.9
                v = light + (BAYER[y % 4][x % 4] / 16 - 0.5) * 0.5
                c.set(x, y, "D" if v > 0.75 else "C" if v > 0.2 else "c")
    for y in range(5, 13):  # línea de los élitros
        if c.get(int(7 * K), int(y * K)) != ".":
            dot(c, 7, y, "c")
    for x, y in ((13, 9), (14, 9), (13, 10), (14, 10), (14, 11), (13, 11)):  # cabeza
        dot(c, x, y, "c")
    dot(c, 14, 9, "r")
    dot(c, 15, 11, "B")  # mandíbula
    for x in (3, 6, 9, 12):  # patas
        dot(c, x, 14, "k")
        dot(c, x - 1, 15, "k")
    c.outline()
    return c


BAT = [
    "................",
    "................",
    "......k..k......",
    "k....kPkkPk....k",
    "pk...kPPPPk...kp",
    "Ppk.kPrPPrPk.kpP",
    "PPpkpPPPPPPpkpPP",
    "QPPppPPkkPPppPPQ",
    ".QPPpPPPPPPpPPQ.",
    "..QP.pPPPPp.PQ..",
    "...Q..pPPp..Q...",
    ".......kk.......",
    "................",
    "................",
    "................",
    "................",
]


def fire():
    c = Canvas(T, T)
    rnd = random.Random(3)
    for y in range(T):
        for x in range(T):
            # tres lenguas de fuego
            h = max(
                11 * math.exp(-((u(x) - 4) ** 2) / 6),
                14 * math.exp(-((u(x) - 9) ** 2) / 7),
                9 * math.exp(-((u(x) - 13) ** 2) / 4),
            )
            top = 16 - h
            if u(y) >= top:
                k = (u(y) - top) / max(h, 1) + rnd.uniform(-0.12, 0.12)
                c.set(x, y, "W" if k > 0.85 and y < T - 1 else "y" if k > 0.55 else "o" if k > 0.25 else "R")
    for x in range(T):
        c.set(x, T - 1, "R")
    c.outline()
    return c


def exit_arrow():
    c = Canvas(T, T)
    for y in range(T):
        for x in range(T):
            body = 2 <= u(x) <= 9 and 5 <= u(y) <= 11
            head = u(x) >= 8.5 and abs(u(y) - 8) <= (14.5 - u(x))
            if body or head:
                c.set(x, y, "z" if u(y) < 8 else "Z")
    c.outline()
    return c


# --- Crock: viene de su hoja de sprites (tools/import_crock.py) -------------


PORRA = [
    "..........kkkk..",
    ".........kxXXxk.",
    "kkkkkkkkkxXXxxxk",
    "kwwwwwwwwxxxxxwk",
    "kkkkkkkkkwxxxwwk",
    ".........kwwwwk.",
    "..........kkkk..",
]


# --- Hojas ------------------------------------------------------------------


def check(c):
    for row in c.px:
        for ch in row:
            assert ch in PALETTE, f"color desconocido '{ch}'"


def sheet(canvases):
    w = sum(c.w for c in canvases)
    h = max(c.h for c in canvases)
    img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    x0 = 0
    for c in canvases:
        if getattr(c, "image", None) is not None:
            img.alpha_composite(c.image, (x0, 0))
            x0 += c.w
            continue
        check(c)
        for y in range(c.h):
            for x in range(c.w):
                col = PALETTE[c.px[y][x]]
                if col is not None:
                    img.putpixel((x0 + x, y), col + (255,))
        x0 += c.w
    return img


def cueva():
    up = fangs()
    return [
        ("solid", stone(1)),
        ("solid", stone_top(2)),
        ("solid", stone(3, dark=True)),
        ("spike_up", up),
        ("spike_down", rotate(up, 2)),
        ("spike_left", rotate(up, 3)),
        ("spike_right", rotate(up, 1)),
        ("deco", stalactite()),
        ("deco", pebbles()),
    ]


def objetos():
    spawn_small = Canvas(T, T)
    # Cabeza de Crock (de la hoja importada) como icono de aparición.
    crock_png = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "assets/gfx/crock.png")
    if os.path.exists(crock_png):
        spawn_small.image = Image.open(crock_png).crop((16, 14, 16 + T, 14 + T))
    return [
        ("spawn", spawn_small),
        ("rock", ball("12345", r=6.8)),
        ("fragile", fragile()),
        ("bone", scaled(from_rows(BONE, 16, 16))),
        ("moss", moss()),
        ("beetle", beetle()),
        ("bat", scaled(from_rows(BAT, 16, 16))),
        ("fire", fire()),
        ("exit", exit_arrow()),
    ]


def tileset(name, image, tiles, count):
    return {
        "type": "tileset",
        "version": "1.10",
        "tiledversion": "1.10.2",
        "name": name,
        "tilewidth": T,
        "tileheight": T,
        "tilecount": count,
        "columns": count,
        "margin": 0,
        "spacing": 0,
        "image": image,
        "imagewidth": T * count,
        "imageheight": T,
        "tiles": tiles,
    }


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    gfx = os.path.join(root, "assets", "gfx")
    levels = os.path.join(root, "assets", "levels")
    os.makedirs(gfx, exist_ok=True)

    cu, ob = cueva(), objetos()
    porra = from_rows(PORRA, 16, 7)
    sheets = {
        "cueva": sheet([c for _, c in cu]),
        "objetos": sheet([c for _, c in ob]),
    }
    for name, img in sheets.items():
        img.save(os.path.join(gfx, f"{name}.png"))

    cueva_tiles = [{"id": i, "properties": [{"name": "kind", "type": "string", "value": k}]} for i, (k, _) in enumerate(cu)]
    objetos_tiles = [{"id": i, "type": k} for i, (k, _) in enumerate(ob)]
    for name, tiles, count in (("cueva", cueva_tiles, len(cu)), ("objetos", objetos_tiles, len(ob))):
        with open(os.path.join(levels, f"{name}.tsj"), "w") as f:
            json.dump(tileset(name, f"../gfx/{name}.png", tiles, count), f, indent=1)
            f.write("\n")

    if "--preview" in sys.argv:
        out = sys.argv[sys.argv.index("--preview") + 1]
        parts = list(sheets.values())
        W = max(p.width for p in parts) + 8
        H = sum(p.height + 6 for p in parts) + 4
        prev = Image.new("RGBA", (W, H), (36, 30, 42, 255))
        y = 4
        for p in parts:
            prev.alpha_composite(p, (4, y))
            y += p.height + 6
        prev.resize((W * 6, H * 6), Image.NEAREST).save(out)
    print("arte generado")


if __name__ == "__main__":
    main()
