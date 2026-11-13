"""Crock, dibujado a mano: celdas de 32x32 px de arte, pies en la fila 31.

Cavernícola cabezón y bonachón en vista 3/4 mirando a la derecha: melena
oscura y despeinada que le cae por la espalda, barba larga en punta, ropa de
pelo deshilachada, brazos fuertes con puño y pies grandes descalzos.

Cada frame se compone por capas (melena de atrás, torso, brazos, piernas,
cabeza, barba). Las siluetas y las caras van píxel a píxel; los mechones se
rellenan con  (tonos por columna con una onda suave).

Lo usa tools/gen_art.py; ejecutado solo guarda una vista ampliada:
    python3 tools/crock_art.py vista.png
"""
import sys
from PIL import Image

P = {
    ".": None,
    "k": (20, 14, 12),      # contorno
    "1": (36, 24, 19),      # pelo muy oscuro
    "2": (56, 38, 28),      # pelo oscuro
    "3": (82, 56, 39),      # pelo medio
    "4": (110, 78, 53),     # pelo claro
    "5": (142, 104, 70),    # pelo brillo
    "q": (88, 46, 30),      # contorno interior de la piel
    "S": (152, 92, 58),     # piel sombra
    "s": (196, 130, 86),    # piel
    "a": (228, 172, 122),   # piel luz
    "r": (206, 112, 86),    # nariz / mejillas
    "e": (236, 228, 214),   # ojo
    "p": (20, 14, 12),      # pupila
}


def paint(spec):
    """spec: {fila: [(columna_inicial, "píxeles"), ...]}  ('.' no pinta)."""
    g = [["."] * 32 for _ in range(32)]
    for y, segs in spec.items():
        for x0, px in segs:
            for i, c in enumerate(px):
                if c == "_" and 0 <= x0 + i < 32:
                    g[y][x0 + i] = "."
                elif c != "." and 0 <= x0 + i < 32:
                    g[y][x0 + i] = c
    return ["".join(r) for r in g]


HEAD = {
    1: [(11, "k"), (15, "k"), (19, "k")],
    2: [(10, "k1"), (13, "k1kk"), (18, "k1k")],
    3: [(8, "kk1122k21121k")],
    4: [(7, "k122334433221k")],
    5: [(6, "k12334454433321k")],
    6: [(5, "k1233445544433321k")],
    7: [(5, "k12334s3s3ss3s3s3k")],
    8: [(5, "k1233saaaaaaaaassq")],
    9: [(5, "k1233ss11aaa111ssq")],
    10: [(5, "k1233Ssepsssepssq")],
    11: [(5, "k1233Srssssssaarrq")],
    12: [(5, "k1223SsqsssqsraarrSq")],
    13: [(5, "k1223SSsqqqssSrrrrq")],
    14: [(5, "k1223334434343SSqqk")],
}

def strands(extents, pattern, shift_every=3, edge="k"):
    """Relleno de pelo por mechones verticales dentro de (izq, der) por fila.

    Cada columna toma un tono del patrón; cada `shift_every` filas el patrón
    se desplaza una columna (onda suave). Bordes con `edge`.
    """
    out = {}
    for y, (l, r) in extents.items():
        row = []
        for x in range(l, r + 1):
            if x in (l, r) and edge:
                row.append(edge)
            else:
                row.append(pattern[(x + y // shift_every) % len(pattern)])
        out[y] = [(l, "".join(row))]
    return out


def jag(spec, y, cols, ch="."):
    """Borra píxeles de la fila `y` (para puntas deshilachadas)."""
    spec.setdefault(y, []).extend((c, "_") for c in cols)


BACK_HAIR = strands(
    {14: (4, 9), 15: (3, 9), 16: (3, 9), 17: (3, 9), 18: (3, 9), 19: (3, 9), 20: (3, 9),
     21: (3, 9), 22: (4, 9), 23: (4, 8)},
    "1221122211",
)
BACK_HAIR[24] = [(4, "k1kk")]
BACK_HAIR[25] = [(5, "k")]

TORSO = strands(
    {15: (8, 19), 16: (7, 19), 17: (7, 19), 18: (7, 19), 19: (7, 19), 20: (6, 19),
     21: (6, 19), 22: (6, 20), 23: (6, 20), 24: (6, 20)},
    "2233232332",
)
TORSO[24] = [(6, "k12k1232k23k1k")]
TORSO[25] = [(7, "kk.k1k.k1k.kk")]

BEARD = strands(
    {14: (10, 17), 15: (10, 20), 16: (10, 20), 17: (11, 19), 18: (11, 19), 19: (12, 18),
     20: (12, 18), 21: (13, 17)},
    "3443453443",
    shift_every=4,
    edge="2",
)
BEARD[14] = [(10, "23443434")]
BEARD[20] = [(12, "k34543k")]
BEARD[21] = [(13, "k343k")]
BEARD[22] = [(14, "k3k")]
BEARD[23] = [(15, "k")]

FLYAWAY = {
    3: [(21, "k")],
    4: [(6, "k")],
    5: [(22, "k")],
    6: [(4, "k")],
    8: [(3, "kk"), (23, "k")],
    9: [(4, "k")],
    11: [(3, "kk")],
    13: [(3, "kk")],
}

FRONT_ARM = {
    15: [(19, "qqqq")],
    16: [(18, "qsaasq")],
    17: [(18, "qSsaasq")],
    18: [(19, "qSssaq"), (25, "qqq")],
    19: [(19, "qSSssqsaaq")],
    20: [(20, "qSssssSsq")],
    21: [(21, "qqqqqqqq")],
}

BACK_ARM = {
    16: [(2, "qqq")],
    17: [(1, "qsaSq")],
    18: [(1, "qsaSq")],
    19: [(2, "qsSsqq")],
    20: [(3, "qSsssq")],
    21: [(4, "qqqqq")],
}

LEGS_IDLE = {
    26: [(9, "qSq"), (15, "qSq")],
    27: [(9, "qsSq"), (15, "qsSq")],
    28: [(9, "qsSq"), (15, "qsSq")],
    29: [(8, "qsssSq"), (14, "qssSaq")],
    30: [(7, "qsssssSq"), (14, "qsssssaq")],
    31: [(7, "qqqqqqqq"), (14, "qqqqqqqq")],
}


def frame(*parts):
    spec = {}
    for p in parts:
        for y, segs in p.items():
            spec.setdefault(y, []).extend(segs)
    return paint(spec)


LEGS_RUN1 = {
    26: [(8, "qSq"), (16, "qSq")],
    27: [(7, "qsSq"), (16, "qsSq")],
    28: [(6, "qsSq"), (17, "qsSq")],
    29: [(4, "qsssSq"), (17, "qssSaq")],
    30: [(4, "qqqqqq"), (17, "qsssssaq")],
    31: [(17, "qqqqqqqq")],
}
LEGS_RUN2 = {
    26: [(10, "qSq"), (14, "qSq")],
    27: [(8, "qqsSq"), (14, "qsSq")],
    28: [(7, "qsssSq"), (14, "qsSq")],
    29: [(7, "qqqqqq"), (13, "qssSaq")],
    30: [(13, "qsssssaq")],
    31: [(13, "qqqqqqqq")],
}
LEGS_JUMP = {
    26: [(9, "qSq"), (15, "qSq")],
    27: [(9, "qsSqq"), (15, "qsSqq")],
    28: [(8, "qsssSaq"), (14, "qsssSaq")],
    29: [(8, "qqqqqqq"), (14, "qqqqqqq")],
}
LEGS_FALL = {
    26: [(9, "qSq"), (15, "qSq")],
    27: [(9, "qsSq"), (15, "qsSq")],
    28: [(9, "qsSq"), (15, "qsSq")],
    29: [(9, "qsSq"), (15, "qsSq")],
    30: [(9, "qsaq"), (15, "qsaq")],
    31: [(9, "qqqq"), (15, "qqqq")],
}

FRONT_ARM_UP = {
    10: [(23, "qqq")],
    11: [(22, "qsaaq")],
    12: [(22, "qsasq")],
    13: [(21, "qssSq")],
    14: [(20, "qssSq")],
    15: [(19, "qsasq")],
    16: [(18, "qsaSq")],
    17: [(18, "qSsSq")],
    18: [(19, "qqqq")],
}
FRONT_ARM_DOWN = {
    15: [(19, "qqqq")],
    16: [(18, "qsaasq")],
    17: [(18, "qSsasq")],
    18: [(19, "qSsaq")],
    19: [(19, "qSsSq")],
    20: [(19, "qsaaq")],
    21: [(19, "qSSq")],
    22: [(20, "qq")],
}
BACK_ARM_UP = {
    9: [(1, "qqq")],
    10: [(0, "qsaq")],
    11: [(0, "qsSq")],
    12: [(1, "qsSq")],
    13: [(2, "qsSq")],
    14: [(3, "qsq")],
}

BACK_HAIR_RUN = strands(
    {14: (4, 9), 15: (3, 9), 16: (3, 9), 17: (2, 9), 18: (2, 9), 19: (1, 9), 20: (1, 9),
     21: (1, 9), 22: (2, 9), 23: (2, 8)},
    "1221122211",
)
BACK_HAIR_RUN[24] = [(2, "k1kk")]
BACK_HAIR_FALL = strands(
    {14: (4, 9), 15: (3, 9), 16: (3, 9), 17: (3, 9), 18: (2, 9), 19: (2, 9)},
    "1221122211",
)
BACK_HAIR_FALL[20] = [(2, "k1k2k")]
BACK_HAIR_FALL[17].append((1, "k"))

IDLE = frame(FLYAWAY, BACK_HAIR, TORSO, BACK_ARM, LEGS_IDLE, HEAD, BEARD, FRONT_ARM)
RUN1 = frame(FLYAWAY, BACK_HAIR_RUN, TORSO, LEGS_RUN1, HEAD, BEARD, FRONT_ARM)
RUN2 = frame(FLYAWAY, BACK_HAIR_RUN, TORSO, LEGS_RUN2, HEAD, BEARD, FRONT_ARM_DOWN)
JUMP = frame(FLYAWAY, BACK_HAIR, TORSO, BACK_ARM, LEGS_JUMP, HEAD, BEARD, FRONT_ARM_UP)
FALL = frame(FLYAWAY, BACK_HAIR_FALL, BACK_ARM_UP, TORSO, LEGS_FALL, HEAD, BEARD, FRONT_ARM_UP)
FRAMES = [IDLE, RUN1, RUN2, JUMP, FALL]


def img(rows):
    im = Image.new("RGBA", (32, 32), (0, 0, 0, 0))
    for y, r in enumerate(rows):
        for x, c in enumerate(r):
            if P[c]:
                im.putpixel((x, y), P[c] + (255,))
    return im


if __name__ == "__main__":
    W = 48 * len(FRAMES) + 16
    sheet = Image.new("RGBA", (W, 48), (150, 150, 150, 255))
    for i, f in enumerate(FRAMES):
        sheet.alpha_composite(img(f), (16 + i * 48, 8))
    sheet.resize((W * 10, 480), Image.NEAREST).save(sys.argv[1])
