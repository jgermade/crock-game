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
    # terreno (paleta de la escena de referencia)
    "6": (22, 19, 30),  # grieta
    "7": (42, 41, 59),  # piedra fría
    "8": (53, 59, 81),  # piedra fría, luz
    "9": (74, 62, 69),  # piedra cálida, sombra
    "!": (95, 81, 81),  # piedra cálida
    "@": (108, 93, 93),  # piedra cálida, luz
    "$": (121, 106, 102),  # piedra cálida, brillo
    "%": (58, 94, 60),  # hierba sombra
    "&": (78, 118, 60),  # hierba
    "*": (98, 138, 67),  # hierba luz
    "+": (131, 185, 68),  # hierba brillo
    "=": (73, 51, 37),  # raíz
    "?": (113, 85, 60),  # raíz luz
    "<": (53, 66, 41),  # musgo sombra
    ">": (82, 92, 52),  # musgo
    "^": (183, 177, 157),  # estalagmita luz
    "~": (150, 140, 129),  # estalagmita
    ":": (110, 100, 94),  # estalagmita sombra
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


def cobbles(seed, ramp, n=6, crack="6"):
    """Empedrado: celdas de Voronoi (con vuelta, para que case al repetirse)
    separadas por grietas, cada piedra con luz arriba-izquierda."""
    rnd = random.Random(seed)
    pts = [(rnd.uniform(0, T), rnd.uniform(0, T), rnd.randint(0, 1)) for _ in range(n)]
    c = Canvas(T, T)
    for y in range(T):
        for x in range(T):
            ds = []
            for i, (px_, py_, _) in enumerate(pts):
                for ox in (-T, 0, T):
                    for oy in (-T, 0, T):
                        ds.append((math.hypot(x + 0.5 - px_ - ox, y + 0.5 - py_ - oy), i, px_ + ox, py_ + oy))
            ds.sort()
            (d1, i, cx, cy), (d2, _, _, _) = ds[0], ds[1]
            if d2 - d1 < 1.1:
                c.set(x, y, crack)
                continue
            # Luz arriba-izquierda dentro de la piedra.
            light = (-(x + 0.5 - cx) - (y + 0.5 - cy)) / (d2 + d1 + 1e-3) * 1.8
            edge = d2 - d1 < 2.4
            v = 1 + pts[i][2] * 0.5 + light + (BAYER[y % 4][x % 4] / 16 - 0.5) * 0.4
            if edge and light < 0:
                v -= 1
            c.set(x, y, ramp[max(0, min(len(ramp) - 1, round(v)))])
    return c


def stone(seed, dark=False):
    if dark:
        return cobbles(seed, "778", n=4)
    return cobbles(seed, "9!@$", n=5)


def stone_moss(seed):
    """Piedra con manchas de musgo (para paredes y laterales)."""
    c = stone(seed)
    rnd = random.Random(seed + 7)
    for _ in range(3):
        cx, cy, r = rnd.uniform(2, T - 2), rnd.uniform(2, T - 2), rnd.uniform(2.5, 5)
        for y in range(T):
            for x in range(T):
                d = math.hypot(x + 0.5 - cx, (y + 0.5 - cy) * 1.4)
                if d < r and c.get(x, y) != "6" and rnd.random() < 0.9 - d / r * 0.4:
                    c.set(x, y, ">" if (x - cx) + (y - cy) < 0 else "<")
    return c


def stone_top(seed):
    """Borde de suelo: hierba gruesa con goterones de musgo y raíces."""
    c = stone(seed)
    rnd = random.Random(seed + 99)
    depth = []
    x = 0
    while x < T:
        d = 7 + (1 if rnd.random() < 0.4 else 0)
        if rnd.random() < 0.3:
            # Goterón de musgo de 2-3 columnas.
            extra = rnd.randint(3, 6)
            for k in range(rnd.randint(2, 3)):
                depth.append(d + extra - (1 if k != 1 else 0))
                x += 1
            continue
        depth.append(d)
        x += 1
    depth = depth[:T]
    for x in range(T):
        for y in range(depth[x]):
            if y == 0:
                ch = "+" if rnd.random() < 0.7 else "*"
            elif y < 3:
                ch = "*" if rnd.random() < 0.75 else "+"
            elif y < 6:
                ch = "&"
            else:
                ch = "%" if y < depth[x] - 1 or depth[x] > 8 else "&"
            c.set(x, y, ch)
        c.set(x, depth[x], "6")
    # Raíces que se meten entre las piedras.
    for _ in range(2):
        x, y = rnd.uniform(3, T - 3), 7.0
        for k in range(rnd.randint(6, 11)):
            c.set(int(x), int(y), "?" if k % 3 == 0 else "=")
            x += rnd.choice((-1, 0, 0, 1))
            y += 1
    return c


def fangs():
    """Pinchos hacia arriba: estalagmitas de piedra pálida."""
    c = Canvas(T, T)
    for cx, hgt, wd in ((4.5, 15, 3.6), (12, 18, 4.2), (19.5, 14, 3.4)):
        for y in range(T):
            t = (T - 1 - y) / hgt  # 0 en la base, 1 en la punta
            if t > 1:
                continue
            half = wd * (1 - t) ** 0.9
            for x in range(T):
                dx = x + 0.5 - cx
                if abs(dx) <= half:
                    rel = dx / max(half, 0.5)
                    c.set(x, y, "^" if rel < -0.3 else "~" if rel < 0.35 else ":")
    for x in range(T):
        c.set(x, T - 1, ":")
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
        t = y / 16
        if t > 1:
            continue
        half = 6 * (1 - t) ** 0.9
        for x in range(T):
            dx = x + 0.5 - 12
            if abs(dx) <= half:
                c.set(x, y, "@" if dx < -half * 0.3 else "!" if dx < half * 0.4 else "9")
    c.outline()
    return c


def tuft(seed):
    """Mata de hierba alta sobre el suelo."""
    c = Canvas(T, T)
    rnd = random.Random(seed)
    for _ in range(rnd.randint(11, 15)):
        x = rnd.uniform(5, T - 5)
        h = rnd.uniform(6, 16)
        lean = rnd.uniform(-0.5, 0.5)
        shade = rnd.choice("*&+&%")
        for k in range(int(h)):
            xx = int(x + lean * k * (k / h))
            ch = "+" if k > h - 3 and shade in "*+" else shade
            c.set(xx, T - 1 - k, ch)
            if k < h * 0.5:
                c.set(xx + 1, T - 1 - k, "%" if shade != "%" else "&")
    return c


def pebbles():
    c = Canvas(T, T)
    for cx, cy, r in ((6, T - 2.5, 3.0), (14, T - 2, 2.2), (19, T - 2.8, 1.8)):
        for y in range(T):
            for x in range(T):
                d = math.hypot(x + 0.5 - cx, y + 0.5 - cy)
                if d <= r:
                    c.set(x, y, "@" if (x + 0.5 - cx) + (y + 0.5 - cy) < -0.5 else "9")
    c.outline()
    return c


# --- Rocas grandes de pared (piezas de varios tiles) -----------------------


def boulders(wt, ht, seed):
    """Pila de rocas redondeadas de wt x ht tiles: fondo de grietas oscuras,
    rocas en gris cálido con luz arriba-izquierda, grietas y musgo encima."""
    W, H = wt * T, ht * T
    c = Canvas(W, H)
    rnd = random.Random(seed)
    for y in range(H):
        for x in range(W):
            c.set(x, y, "6")
    # Rocas: una o dos grandes y alguna pequeña, que se solapan.
    rocks = []
    if wt == 2:
        # Una roca grande que domina y alguna pequeña en una esquina.
        rocks.append((W * rnd.uniform(0.46, 0.56), H * rnd.uniform(0.5, 0.58), W * 0.53, H * 0.5))
        if rnd.random() < 0.7:
            rocks.append((W * rnd.choice((0.18, 0.82)), H * 0.14, W * 0.2, H * 0.17))
    else:
        # Columna estrecha: rocas a todo el ancho, una alta o varias apiladas.
        n = ht if rnd.random() < 0.5 else max(1, ht - 1)
        cuts = sorted(rnd.uniform(0.25, 0.75) for _ in range(n - 1))
        edges = [0.0] + cuts + [1.0] if n > 1 else [0.0, 1.0]
        for a, b in zip(edges, edges[1:]):
            rocks.append((W * rnd.uniform(0.47, 0.53), H * (a + b) / 2, W * 0.56, H * (b - a) / 2 + 1.5))
    ramp = "9!@$"
    for k, (cx, cy, rx, ry) in enumerate(rocks):
        bumps = [rnd.uniform(0, 6.28) for _ in range(3)]
        for y in range(H):
            for x in range(W):
                dx, dy = (x + 0.5 - cx) / rx, (y + 0.5 - cy) / ry
                ang = math.atan2(dy, dx)
                # Borde irregular (superelipse con bultos).
                edge = 1 + 0.06 * math.sin(ang * 3 + bumps[0]) + 0.04 * math.sin(ang * 5 + bumps[1])
                r = (abs(dx) ** 2.4 + abs(dy) ** 2.4) ** (1 / 2.4)
                if r > edge:
                    continue
                if r > edge - 1.3 / min(rx, ry):
                    c.set(x, y, "6")  # contorno oscuro
                    continue
                nz = max(0.0, 1 - r * r) ** 0.5
                light = -dx * 0.55 - dy * 0.75 + nz * 0.6
                v = 1.2 + light * 1.6 + (BAYER[y % 4][x % 4] / 16 - 0.5) * 0.45
                c.set(x, y, ramp[max(0, min(3, round(v)))])
        # Grietas cortas.
        for _ in range(2):
            x, y = cx + rnd.uniform(-rx, rx) * 0.4, cy + rnd.uniform(-ry, ry) * 0.3
            for _ in range(int(min(rx, ry) * 0.5)):
                if c.get(int(x), int(y)) in ramp:
                    c.set(int(x), int(y), "9" if c.get(int(x), int(y)) != "9" else "6")
                x += rnd.choice((-1, 0, 1))
                y += 1
        # Musgo por arriba, con algún hilo colgando.
        for x in range(W):
            for y in range(H):
                ch = c.get(x, y)
                if ch in ramp and c.get(x, y - 1) == "6":
                    if math.sin(x * 0.45 + k * 2 + bumps[2]) > 0.15:
                        depth = rnd.randint(1, 3)
                        for d in range(depth):
                            if c.get(x, y + d) in ramp:
                                c.set(x, y + d, ">" if d == 0 else "<")
                        if rnd.random() < 0.12:
                            for d in range(depth, depth + rnd.randint(2, 5)):
                                if c.get(x, y + d) in ramp:
                                    c.set(x, y + d, "<")
                    break
    return c


def slices(c, wt, ht):
    """Corta una pieza de varios tiles en tiles (por filas)."""
    out = []
    for ty in range(ht):
        for tx in range(wt):
            t = Canvas(T, T)
            for y in range(T):
                for x in range(T):
                    t.set(x, y, c.get(tx * T + x, ty * T + y))
            out.append(t)
    return out


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
        ("deco", tuft(5)),
        ("deco", pebbles()),
        ("solid", stone_moss(4)),
        ("solid", stone(11)),
        ("solid", stone(21)),
        ("solid", stone(22, dark=True)),
        ("solid", stone_top(14)),
    ] + [
        # Rocas de pared (decoración; ver tools/ascii2tmj.py):
        # 15-18 y 19-22: 2x2; 23-25 y 26-28: 1x3; 29-30 y 31-32: 1x2; 33 y 34: 1x1.
        ("deco", t)
        for wt, ht, seed in ((2, 2, 1), (2, 2, 2), (1, 3, 3), (1, 3, 4), (1, 2, 5), (1, 2, 6), (1, 1, 7), (1, 1, 8))
        for t in slices(boulders(wt, ht, seed), wt, ht)
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


# --- Fondo de cueva con parallax (paleta de la escena de referencia) ---------
#
# Tres tiras que se repiten en horizontal; el juego las desplaza a distinta
# velocidad según la cámara. Su "suelo" queda a BG_BASE px del borde inferior.

BG = {
    "night": (22, 19, 30),  # #16131e
    "base": (42, 41, 59),  # #2a293b
    "rim": (53, 59, 81),  # #353b51
    "light": (72, 78, 102),  # #484e66
    "glow": (88, 92, 112),
    "mid": (34, 33, 49),
    "midlit": (48, 52, 72),
    "near": (16, 13, 22),
}
BG_H = 420
BG_BASE = 100


def _wrapdx(x, cx, w):
    d = abs(x - cx)
    return min(d, w - d)


def _sdx(x, cx, w):
    """Distancia con signo y vuelta horizontal."""
    d = x - cx
    if d > w / 2:
        d -= w
    elif d < -w / 2:
        d += w
    return d


class Spike:
    """Estalactita/estalagmita orgánica: perfil abultado, punta roma y
    bordes ondulados."""

    def __init__(self, rnd, w, lmin, lmax, wmin, wmax):
        self.x = rnd.uniform(0, w)
        self.len = rnd.uniform(lmin, lmax)
        self.wd = rnd.uniform(wmin, wmax)
        self.ph = rnd.uniform(0, 6.28)
        self.bulge = rnd.uniform(0.45, 0.8)

    def half(self, t):
        """Media anchura a la fracción `t` (0 = base, 1 = punta)."""
        if t < 0 or t > 1:
            return -1
        wob = 1 + 0.12 * math.sin(t * 11 + self.ph) + 0.06 * math.sin(t * 29 + self.ph * 2)
        return self.wd / 2 * (1 - t) ** self.bulge * wob + (1.2 if t < 0.97 else 0)


def _noise2(x, y, w, seed):
    """Ruido suave periódico en x (suma de senos)."""
    k = 2 * math.pi / w
    return (
        math.sin(x * k * 3 + seed) * math.cos(y / 37 + seed * 0.7)
        + 0.5 * math.sin(x * k * 7 + y / 23 + seed * 1.3)
        + 0.25 * math.sin(x * k * 13 - y / 11 + seed * 2.1)
    ) / 1.75


def bg_far(w=480):
    rnd = random.Random(101)
    img = Image.new("RGBA", (w, BG_H), BG["base"] + (255,))
    px = img.load()
    ground = BG_H - BG_BASE - 24
    ceiling = [40 + 12 * math.sin(x * 2 * math.pi / w * 3) + 5 * math.sin(x / 7.3) for x in range(w)]
    hang = [Spike(rnd, w, 25, 90, 10, 26) for _ in range(12)]
    rise = [Spike(rnd, w, 30, 90, 26, 54) for _ in range(6)]
    for y in range(BG_H):
        for x in range(w):
            # Cámara del fondo iluminada: centro alto, bordes rotos por ruido.
            vy = 1 - ((y - 215) / 150) ** 2
            k = 2 * math.pi / w
            v = vy + 0.16 * math.sin(x * k * 2 + 1.0) + 0.09 * math.sin(x * k * 5 + y / 41) + 0.04 * math.sin(x * k * 11)
            t = BAYER[y % 4][x % 4] / 16 * 0.1
            col = BG["base"]
            if v > 0.78 + t:
                col = BG["glow"]
            elif v > 0.48 + t:
                col = BG["light"]
            elif v > 0.28 + t:
                col = BG["rim"]
            for sp in hang:
                if abs(_sdx(x, sp.x, w)) < sp.half((y - ceiling[x]) / sp.len):
                    col = BG["base"]
            for sp in rise:
                if abs(_sdx(x, sp.x, w)) < sp.half((ground - y) / sp.len):
                    col = BG["rim"] if col in (BG["light"], BG["glow"]) else BG["base"]
            if y > ground + 8 * math.sin(x * 2 * math.pi / w * 4) + 3 * math.sin(x / 9):
                col = BG["base"]
            if y < ceiling[x]:
                col = BG["night"]
            px[x, y] = col + (255,)
    return img


def bg_mid(w=560):
    rnd = random.Random(202)
    img = Image.new("RGBA", (w, BG_H), (0, 0, 0, 0))
    px = img.load()
    ground = BG_H - BG_BASE
    top = [18 + 8 * math.sin(x * 2 * math.pi / w * 5) + 3 * math.sin(x / 6) for x in range(w)]
    cols = [Spike(rnd, w, 60, 150, 30, 60) for _ in range(5)]
    hang = [Spike(rnd, w, 30, 150, 12, 32) for _ in range(15)]
    for y in range(BG_H):
        for x in range(w):
            col = None
            for sp in cols:
                d = _sdx(x, sp.x, w)
                h = sp.half((ground - y) / sp.len)
                if abs(d) < h:
                    col = BG["midlit"] if d < -h + 2.5 else BG["mid"]
            for sp in hang:
                d = _sdx(x, sp.x, w)
                h = sp.half((y - top[x]) / sp.len)
                if abs(d) < h:
                    col = BG["midlit"] if d < -h + 2 else BG["mid"]
            if y < top[x]:
                col = BG["mid"]
            if y > ground + 5 * math.sin(x * 2 * math.pi / w * 6) + 2 * math.sin(x / 5):
                col = BG["mid"]
            if col:
                px[x, y] = col + (255,)
    return img


def bg_near(w=640):
    rnd = random.Random(303)
    img = Image.new("RGBA", (w, BG_H), (0, 0, 0, 0))
    px = img.load()
    ground = BG_H - BG_BASE + 14
    near = BG["near"] + (255,)

    def put(x, y):
        if 0 <= y < BG_H:
            px[int(x) % w, int(y)] = near

    for x in range(w):  # suelo ondulado
        top = ground + 5 * math.sin(x / 21) + 3 * math.sin(x / 7)
        for y in range(int(top), BG_H):
            put(x, y)
    for _ in range(14):  # rocas redondeadas
        cx, r = rnd.uniform(0, w), rnd.uniform(8, 22)
        for y in range(int(ground - r), ground + 2):
            for x in range(int(cx - r * 1.4), int(cx + r * 1.4)):
                if ((x - cx) / 1.4) ** 2 + (y - ground) ** 2 <= r * r:
                    put(x, y)
    for _ in range(26):  # helechos: frondas curvas
        bx = rnd.uniform(0, w)
        for f in range(rnd.randint(4, 7)):
            ang = math.radians(rnd.uniform(-75, 75) - 90)
            ln = rnd.uniform(14, 34)
            curl = rnd.uniform(-0.04, 0.04)
            x, y = bx, ground
            for k in range(int(ln)):
                ang += curl
                x += math.cos(ang)
                y += math.sin(ang)
                put(x, y)
                put(x + 1, y)
                leaf = 3 * (1 - k / ln) + 1
                if k % 2 == 1:  # foliolos
                    for r in range(1, int(leaf) + 1):
                        put(x + math.cos(ang + 1.3) * r, y + math.sin(ang + 1.3) * r)
                        put(x + math.cos(ang - 1.3) * r, y + math.sin(ang - 1.3) * r)
    for _ in range(90):  # briznas de hierba
        bx, ln = rnd.uniform(0, w), rnd.randint(4, 12)
        lean = rnd.uniform(-0.5, 0.5)
        for k in range(ln):
            put(bx + lean * k, ground - k)
    return img


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
    for name, fn in (("fondo_lejano", bg_far), ("fondo_medio", bg_mid), ("fondo_cerca", bg_near)):
        fn().save(os.path.join(gfx, f"{name}.png"))

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
