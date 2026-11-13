#!/usr/bin/env python3
"""Convierte una sala en ASCII a un mapa de Tiled (.tmj).

    python3 tools/ascii2tmj.py sala.txt assets/levels/sala.tmj ["Nombre"]

Mismos caracteres que `TileMap::parse` (ver crates/sim/src/map.rs). Genera
una capa `tiles` con autotiling del tileset `cueva`, una capa `deco` con
estalactitas y piedrecitas, y una capa de objetos con objetos-tile del
tileset `objetos` (heredan su clase del tile, así que se ven en el editor).
A partir de ahí, el .tmj es la fuente: se edita en Tiled.
"""

import json
import sys

TS = 24  # píxeles por tile en el editor (el arte va a 3 px por unidad de juego)
CUEVA_FIRST = 1
OBJ_FIRST = 101
# Índices en los tilesets (ver tools/gen_art.py).
STONE, STONE_TOP, STONE_DARK = 0, 1, 2
# Variantes para que no se note la repetición (elegidas por hash de la celda).
STONE_VARIANTS = (0, 11, 12, 0, 11, 12, 10)  # 10: piedra con musgo
TOP_VARIANTS = (1, 14)
DARK_VARIANTS = (2, 13)
SPIKES = {"^": 3, "v": 4, "<": 5, ">": 6}
STALACTITE, TUFT, PEBBLES = 7, 8, 9
OBJECTS = {"P": 0, "o": 1, "x": 2, "b": 3, "m": 4, "e": 5, "w": 6, "f": 7, "E": 8}


def parse(src):
    rows = [r.rstrip("\r") for r in src.splitlines() if r.strip() and not r.startswith(";")]
    w = max(len(r) for r in rows)
    return [r.ljust(w, ".") for r in rows]


def convert(rows, name):
    h, w = len(rows), len(rows[0])

    def solid(x, y):
        if x < 0 or x >= w:
            return True
        if y < 0 or y >= h:
            return False
        return rows[y][x] == "#"

    def hashed(x, y):
        v = (x * 73856093) ^ (y * 19349663)
        return (v ^ (v >> 13)) % 100

    tiles = [0] * (w * h)
    deco = [0] * (w * h)
    objects = []
    for y in range(h):
        for x in range(w):
            c = rows[y][x]
            i = y * w + x
            if c == "#":
                open_above = y > 0 and not solid(x, y - 1)
                buried = all(
                    x + dx < 0 or x + dx >= w or y + dy < 0 or y + dy >= h or solid(x + dx, y + dy)
                    for dy in (-1, 0, 1)
                    for dx in (-1, 0, 1)
                )
                pick = lambda opts: opts[hashed(x, y) % len(opts)]
                t = pick(TOP_VARIANTS) if open_above else pick(DARK_VARIANTS) if buried else pick(STONE_VARIANTS)
                tiles[i] = CUEVA_FIRST + t
            elif c in SPIKES:
                tiles[i] = CUEVA_FIRST + SPIKES[c]
            else:
                if c in OBJECTS:
                    objects.append((c, x, y))
                elif c not in ". ":
                    raise SystemExit(f"carácter desconocido '{c}' en ({x},{y})")
                # Decoración en huecos vacíos.
                if c == "." and y > 0 and solid(x, y - 1) and y + 1 < h and rows[y + 1][x] == "." and hashed(x, y) < 22:
                    deco[i] = CUEVA_FIRST + STALACTITE
                elif c == "." and solid(x, y + 1) and y + 1 < h and hashed(x, y) < 26:
                    deco[i] = CUEVA_FIRST + (PEBBLES if hashed(x, y) < 7 else TUFT)

    objs = []
    for n, (c, x, y) in enumerate(objects, start=1):
        objs.append(
            {
                "id": n,
                "gid": OBJ_FIRST + OBJECTS[c],
                "name": "",
                "type": "",
                "x": x * TS,
                "y": (y + 1) * TS,  # los objetos-tile tienen el origen abajo
                "width": TS,
                "height": TS,
                "rotation": 0,
                "visible": True,
            }
        )

    def layer(lid, lname, data):
        return {
            "id": lid,
            "name": lname,
            "type": "tilelayer",
            "x": 0,
            "y": 0,
            "width": w,
            "height": h,
            "opacity": 1,
            "visible": True,
            "data": data,
        }

    return {
        "type": "map",
        "version": "1.10",
        "tiledversion": "1.10.2",
        "orientation": "orthogonal",
        "renderorder": "right-down",
        "width": w,
        "height": h,
        "tilewidth": TS,
        "tileheight": TS,
        "infinite": False,
        "compressionlevel": -1,
        "nextlayerid": 4,
        "nextobjectid": len(objs) + 1,
        "properties": [{"name": "name", "type": "string", "value": name}],
        "tilesets": [
            {"firstgid": CUEVA_FIRST, "source": "cueva.tsj"},
            {"firstgid": OBJ_FIRST, "source": "objetos.tsj"},
        ],
        "layers": [
            layer(1, "deco", deco),
            layer(2, "tiles", tiles),
            {
                "id": 3,
                "name": "objetos",
                "type": "objectgroup",
                "draworder": "topdown",
                "x": 0,
                "y": 0,
                "opacity": 1,
                "visible": True,
                "objects": objs,
            },
        ],
    }


def dump(m):
    """JSON legible, con cada fila de tiles en su línea (diffs útiles)."""
    w = m["width"]
    pretty = {}
    for n, layer in enumerate(m["layers"]):
        if layer["type"] == "tilelayer":
            data = layer["data"]
            rows = [data[i : i + w] for i in range(0, len(data), w)]
            pretty[n] = "[\n" + ",\n".join("    " + ",".join(str(v) for v in r) for r in rows) + "\n   ]"
            layer["data"] = f"__DATA_{n}__"
    text = json.dumps(m, indent=1, ensure_ascii=False)
    for n, p in pretty.items():
        text = text.replace(f'"__DATA_{n}__"', p)
    return text + "\n"


def main():
    if len(sys.argv) < 3:
        raise SystemExit(__doc__)
    src = open(sys.argv[1]).read()
    name = sys.argv[3] if len(sys.argv) > 3 else sys.argv[2].rsplit("/", 1)[-1].removesuffix(".tmj")
    with open(sys.argv[2], "w", encoding="utf-8") as f:
        f.write(dump(convert(parse(src), name)))


if __name__ == "__main__":
    main()
