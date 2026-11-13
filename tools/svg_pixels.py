"""Rasteriza SVG de pixel art vectorizado (paths con M/H/V/Z, evenodd) a PNG 1:1.

    python3 tools/svg_pixels.py entrada.svg salida.png

Respeta el `fill` heredado de los grupos. Cada unidad del viewBox es un píxel.
"""
import re, sys
from PIL import Image

src = open(sys.argv[1]).read()
vb = list(map(int, re.search(r'viewBox="([\d\s.]+)"', src).group(1).split()))
W, H = vb[2], vb[3]
img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
px = img.load()
import xml.etree.ElementTree as ET
root = ET.fromstring(src)
def walk(el, fill):
    fill = el.get("fill", fill)
    tag = el.tag.split("}")[-1]
    if tag == "path":
        yield fill, el.get("d")
    for ch in el:
        yield from walk(ch, fill)
def parse_color(c):
    c = c.lstrip("#")
    if len(c) == 3:
        c = "".join(ch * 2 for ch in c)
    return tuple(int(c[i:i + 2], 16) for i in (0, 2, 4)) + (255,)
for fill, d in walk(root, "#000000"):
    if not fill or fill == "none":
        continue
    color = parse_color(fill)
    edges = []  # aristas verticales (x, y0, y1)
    x = y = sx = sy = 0
    for cmd, args in re.findall(r'([MmHhVvZzLl])([^MmHhVvZzLl]*)', d):
        nums = list(map(float, re.findall(r'-?\d*\.?\d+', args)))
        if cmd in "Mm":
            if cmd == "m":
                x, y = x + nums[0], y + nums[1]
            else:
                x, y = nums[0], nums[1]
            sx, sy = x, y
        elif cmd in "Hh":
            for n in nums:
                x = x + n if cmd == "h" else n
        elif cmd in "Vv":
            for n in nums:
                ny = y + n if cmd == "v" else n
                edges.append((x, min(y, ny), max(y, ny)))
                y = ny
        elif cmd in "Zz":
            if x == sx and y != sy:
                edges.append((x, min(y, sy), max(y, sy)))
            x, y = sx, sy
        elif cmd in "Ll":
            raise SystemExit("L no soportado")
    rows = {}
    for ex, y0, y1 in edges:
        for yy in range(int(y0), int(y1)):
            rows.setdefault(yy, []).append(ex)
    for yy, xs in rows.items():
        xs.sort()
        for a, b in zip(xs[0::2], xs[1::2]):
            for xx in range(int(a), int(b)):
                if 0 <= xx < W and 0 <= yy < H:
                    px[xx, yy] = color
img.save(sys.argv[2])
print(W, H)
