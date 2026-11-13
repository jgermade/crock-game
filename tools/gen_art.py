#!/usr/bin/env python3
"""Genera el pixel art de cROCK (a 16 px por tile) y los tilesets de Tiled.

La física trabaja a 8 unidades por tile; el arte va al doble de resolución
(cada unidad de juego son 2×2 píxeles de arte). Ejecutar desde la raíz:

    python3 tools/gen_art.py [--preview salida.png]

Salida:
    assets/gfx/cueva.png       tiles 16x16 (colisión + decoración)
    assets/gfx/objetos.png     objetos 16x16 (golpeables, enemigos, zonas)
    assets/gfx/crock.png       Crock, celdas 24x24 (pies en la fila 23, centrado)
    assets/gfx/porra.png       la porra, 16x7 (empuñadura a la izquierda)
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

T = 16  # píxeles de arte por tile

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
    "H": (52, 32, 24),  # pelo oscuro
    "M": (86, 56, 36),  # pelo medio
    "L": (126, 86, 54),  # pelo claro (brillos)
    "e": (250, 244, 230),  # blanco del ojo
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
        depth = 3 + (1 if rnd.random() < 0.35 else 0) + (1 if rnd.random() < 0.15 else 0)
        for y in range(depth):
            c.set(x, y, "Y" if y == 0 and rnd.random() < 0.5 else "G" if y < 2 else "g")
        c.set(x, depth, "1")
    return c


def fangs():
    """Pinchos hacia arriba: dos colmillos de hueso en la mitad inferior."""
    c = Canvas(T, T)
    for cx in (4, 12):
        for y in range(5, T):
            half = (y - 5) * 4 / 10.5
            for x in range(T):
                dx = x + 0.5 - cx
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
    for y in range(11):
        half = 5.5 * (1 - y / 11)
        for x in range(T):
            dx = x + 0.5 - 8
            if abs(dx) <= half:
                c.set(x, y, "4" if dx < -1.5 else "3" if dx < 1 else "2")
    c.outline()
    return c


def pebbles():
    c = Canvas(T, T)
    for cx, cy, r in ((4, 14, 2.2), (10, 14.5, 1.6), (13, 13.8, 1.2)):
        for y in range(T):
            for x in range(T):
                d = math.hypot(x + 0.5 - cx, y + 0.5 - cy)
                if d <= r:
                    c.set(x, y, "4" if (x + 0.5 - cx) + (y + 0.5 - cy) < -0.5 else "3")
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
            nx = (x + 0.5 - cx) / r
            ny = (y + 0.5 - cy) / (r * squash)
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
        if c.get(x, y) not in (".", "k"):
            c.set(x, y, "k")
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
            nx, ny = (x + 0.5 - 8) / 7.5, (y + 0.5 - 10) / 5
            if nx * nx + ny * ny <= 1 + rnd.uniform(-0.15, 0.1):
                shade = -ny * 1.2 - nx * 0.4 + rnd.uniform(-0.4, 0.4)
                c.set(x, y, "Y" if shade > 0.8 else "G" if shade > -0.2 else "g")
    for x in (3, 7, 10, 13):  # hojitas
        c.set(x, 4, "G")
        c.set(x, 3, "Y")
    c.outline()
    return c


def beetle():
    c = Canvas(T, T)
    # caparazón: media elipse
    for y in range(T):
        for x in range(T):
            nx, ny = (x + 0.5 - 7) / 6.5, (y + 0.5 - 12) / 7
            if ny <= 0.15 and nx * nx + ny * ny <= 1:
                light = -nx * 0.5 - ny * 0.9
                v = light + (BAYER[y % 4][x % 4] / 16 - 0.5) * 0.5
                c.set(x, y, "D" if v > 0.75 else "C" if v > 0.2 else "c")
    for y in range(5, 13):  # línea de los élitros
        if c.get(7, y) != ".":
            c.set(7, y, "c")
    for x, y in ((13, 9), (14, 9), (13, 10), (14, 10), (14, 11), (13, 11)):  # cabeza
        c.set(x, y, "c")
    c.set(14, 9, "r")
    c.set(15, 11, "B")  # mandíbula
    for x in (3, 6, 9, 12):  # patas
        c.set(x, 14, "k")
        c.set(x - 1, 15, "k")
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
                11 * math.exp(-((x + 0.5 - 4) ** 2) / 6),
                14 * math.exp(-((x + 0.5 - 9) ** 2) / 7),
                9 * math.exp(-((x + 0.5 - 13) ** 2) / 4),
            )
            top = T - h
            if y >= top:
                k = (y - top) / max(h, 1) + rnd.uniform(-0.12, 0.12)
                c.set(x, y, "W" if k > 0.85 and y < T - 1 else "y" if k > 0.55 else "o" if k > 0.25 else "R")
    for x in range(T):
        c.set(x, T - 1, "R")
    c.outline()
    return c


def exit_arrow():
    c = Canvas(T, T)
    for y in range(T):
        for x in range(T):
            body = 2 <= x <= 8 and 5 <= y <= 10
            head = x >= 8 and abs(y + 0.5 - 8) <= (14.5 - x)
            if body or head:
                c.set(x, y, "z" if y < 8 else "Z")
    c.outline()
    return c


# --- Crock -----------------------------------------------------------------
#
# Cavernícola delgado con una melena larga que le cae como un manto y le
# cubre el cuerpo; asoman la cara, los brazos y los pies. Mira a la derecha.
# Celdas de 24x24, pies en la fila 23; la caja de colisión (16x22 px de arte)
# ocupa las columnas 4..19.

STRANDS = "HMMLMHMM"  # patrón de mechones por columna


def crock(pose):
    c = Canvas(24, 24)
    # Melena: silueta por filas (izquierda, derecha). Estrecha.
    sway = {"idle": 0, "run1": -1, "run2": -2, "jump": 0, "fall": 0}[pose]
    bottom_base = {"idle": 20, "run1": 19, "run2": 19, "jump": 20, "fall": 18}[pose]
    rnd = random.Random(11)
    tips = [rnd.choice((0, 0, 1, 1, 2)) for _ in range(24)]
    for y in range(2, 22):
        if y < 4:
            l, r = 10 - (y - 2), 13 + (y - 2)
        elif y < 9:
            l, r = 7, 16
        else:
            # cae hacia atrás según la pose; se abre un poco abajo
            spread = (y - 9) // 4
            l = 7 - spread + (sway if y > 13 else 0)
            r = 16 if y < 16 else 15
        if pose == "fall" and y > 12:
            # al caer el pelo se levanta y se abre hacia los lados
            l -= 1
            r += 1 if y > 15 else 0
        for x in range(l, r + 1):
            bottom = bottom_base - tips[x] + (1 if x < 9 and pose.startswith("run") else 0)
            if y > bottom:
                continue
            wave = (y // 3 + (1 if x > 11 else 0)) % 2
            ch = STRANDS[(x + wave) % len(STRANDS)]
            if y < 5 and x in (9, 10):
                ch = "L"  # brillo en la coronilla
            c.set(x, y, ch)
    # Cara: hueco a la derecha, con flequillo encima.
    for y in range(6, 11):
        for x in range(13, 17):
            c.set(x, y, "s")
    for x in range(12, 17):
        c.set(x, 5, "M" if x % 2 else "H")  # flequillo
    c.set(16, 6, "M")
    c.set(14, 7, "e")
    c.set(15, 7, "k")  # ojo
    c.set(16, 8, "S")  # nariz
    c.set(17, 8, "s")
    for x in (13, 14, 15):
        c.set(x, 10, "M")  # barba que se funde con la melena
    c.set(16, 10, "S")
    # Brazos (sólo el de delante se ve casi siempre).
    arms = {
        "idle": [(16, 13), (17, 13), (17, 14), (18, 14), (18, 15)],
        "run1": [(16, 12), (17, 12), (18, 12), (19, 11)],
        "run2": [(16, 14), (17, 14), (17, 15), (18, 16)],
        "jump": [(16, 11), (17, 10), (18, 9), (18, 8)],
        "fall": [(16, 12), (17, 11), (18, 10), (19, 10)],
    }[pose]
    for x, y in arms:
        c.set(x, y, "s")
    hx, hy = arms[-1]
    c.set(hx, hy, "S")  # mano
    if pose == "fall":
        for x, y in ((6, 12), (5, 11), (4, 10)):  # brazo de atrás, arriba
            c.set(x, y, "s")
    # Piernas y pies descalzos.
    feet = {
        "idle": [(9, 21), (9, 22), (8, 23), (9, 23), (10, 23), (13, 21), (13, 22), (13, 23), (14, 23), (15, 23)],
        "run1": [(8, 21), (7, 22), (6, 22), (6, 23), (7, 23), (14, 20), (15, 21), (15, 22), (16, 22), (17, 22)],
        "run2": [(11, 21), (11, 22), (10, 23), (11, 23), (12, 23), (13, 23)],
        "jump": [(9, 21), (10, 22), (11, 22), (13, 21), (14, 22), (15, 22)],
        "fall": [(9, 19), (9, 20), (8, 21), (9, 21), (14, 19), (14, 20), (14, 21), (15, 21)],
    }[pose]
    for x, y in feet:
        c.set(x, y, "s" if c.get(x, y + 1) in (".",) or y == 23 else "S")
    c.outline()
    return c


FRAMES = ["idle", "run1", "run2", "jump", "fall"]

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
    spawn = crock("idle")
    spawn_small = Canvas(T, T)
    for y in range(T):  # cabeza y hombros de Crock, recortados
        for x in range(T):
            spawn_small.set(x, y, spawn.get(x + 4, y + 3))
    return [
        ("spawn", spawn_small),
        ("rock", ball("12345", r=6.8)),
        ("fragile", fragile()),
        ("bone", from_rows(BONE, T, T)),
        ("moss", moss()),
        ("beetle", beetle()),
        ("bat", from_rows(BAT, T, T)),
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
    frames = [crock(p) for p in FRAMES]
    porra = from_rows(PORRA, 16, 7)
    sheets = {
        "cueva": sheet([c for _, c in cu]),
        "objetos": sheet([c for _, c in ob]),
        "crock": sheet(frames),
        "porra": sheet([porra]),
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
