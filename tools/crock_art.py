"""Crock, dibujado a mano: celdas de 32x32 px de arte, pies en la fila 31.

Cavernícola delgado con una melena larga que le cae como un manto; asoman la
cara, los brazos y los pies. Mira a la derecha. Frames: quieto, carrera 1,
carrera 2, salto y caída. Lo usa tools/gen_art.py; ejecutado solo, guarda
una vista ampliada:  python3 tools/crock_art.py vista.png
"""
import sys
from PIL import Image

P = {
    ".": None,
    "O": (34, 19, 16),     # contorno del pelo
    "H": (68, 37, 24),     # pelo sombra
    "M": (108, 61, 34),    # pelo medio
    "L": (148, 90, 47),    # pelo luz
    "A": (196, 138, 76),   # pelo brillo
    "q": (104, 54, 36),    # contorno piel
    "S": (184, 110, 68),   # piel sombra
    "s": (226, 160, 106),  # piel
    "a": (248, 202, 150),  # piel luz
    "k": (22, 16, 20),     # pupila
    "e": (240, 234, 222),  # blanco del ojo
}

IDLE = [
#    01234567890123456789012345678901
    "................................",  # 0
    "................................",  # 1
    "................................",  # 2
    ".............OOOOOO.............",  # 3
    "...........OOAAALMMOO...........",  # 4
    "..........OAALLLMMMMMO..........",  # 5
    ".........OALLMMMMMMMMHO.........",  # 6
    ".........OLLMMMHMMMMMMHO........",  # 7
    "........OLLMMMHMMMMMHMMMO.......",  # 8
    "........OLMMMHMMMMHMMMHMMOO.....",  # 9
    "........OLMMHMMMMHMHMHMHMHO.....",  # 10
    "........OLMMHMMMMHSHsHsHsSq.....",  # 11
    "........OAMMHLMMMHsSSSSSsaq.....",  # 12
    "........OLMMHLMMMHsseksssaq.....",  # 13
    "........OLMMHLMMMHSssssSsaaq....",  # 14
    "........OLMMHLMMMMHssssSaaaaq...",  # 15
    "........OAMHMLMMMMHSsqqqSaaSq...",  # 16
    "........OLMHMLMMMHMHSssssSqq....",  # 17
    "........OLMHMLMMHMMHMHMHMO......",  # 18
    "........OLMHMMLMHMMLMHMHMOqqqq..",  # 19
    "........OLMHMMLMHMMLMMHMMqsaasq.",  # 20
    "........OLMHMMLMHMMLMMHMHqsassq.",  # 21
    "........OLHMMMLMHMMMLMHMHOqSSq..",  # 22
    "........OLHMMMLMHMMMLMHMHO.qq...",  # 23
    "........OMHMMMLHHMMMLMHHHO......",  # 24
    "........OMHMMHLHHMMHLHHHO.......",  # 25
    "........OHHMHHMHOHMHMHHHO.......",  # 26
    ".........OHHOHHOqOHHOHHO........",  # 27
    "..........OO.OOqSqOO.OO.........",  # 28
    "..............qsSq.qsSq.........",  # 29
    ".............qsssaqqsssaq.......",  # 30
    ".............qqqqqq.qqqqqq......",  # 31
]



HEAD = IDLE[:18]

RUN1 = HEAD + [
    "........OLMHMLMMHMMHMHMHMOqqq...",  # 18
    "........OLMHMMLMHMMLMHMHMqsaasq.",  # 19
    ".......OLMHMMLMHMMLMMHMMMqsassq.",  # 20
    ".......OLMHMMLMHMMLMMHMHOqSSq...",  # 21
    ".......OLHMMMLMHMMMLMHMHO.qq....",  # 22
    "......OLHMMMLMHMMMLMHMHO........",  # 23
    "......OMHMMMLHHMMMLMHHHO........",  # 24
    ".....OMHMMHLHHMMHLHHHHO.........",  # 25
    ".....OHHMHHMHOHMHMHHHO..........",  # 26
    ".....OHOHHOHHOqHOHHHO...........",  # 27
    "......O.OO.OqSSqOqSqO...........",  # 28
    "..........qsSq...qsSq...........",  # 29
    ".........qsssq...qsssaq.........",  # 30
    ".........qqqq....qqqqqqq........",  # 31
]

RUN2 = HEAD + [
    "........OLMHMLMMHMMHMHMHMO......",  # 18
    "........OLMHMMLMHMMLMHMHMO......",  # 19
    ".......OLMHMMLMHMMLMMHMMMqqq....",  # 20
    "......OLMHMMLMHMMLMMHMHMqsasq...",  # 21
    "......OLHMMMLMHMMMLMHMHMqsssq...",  # 22
    ".....OLHMMMLMHMMMLMHMHO.qqq.....",  # 23
    ".....OMHMMMLHHMMMLMHHHO.........",  # 24
    "....OMHMMHLHHMMHLHHHHO..........",  # 25
    "....OHHMHHMHOHMHMHHHO...........",  # 26
    "....OHOHHOHHOHHHOHO.............",  # 27
    ".....O.OO.OOOqSqO...............",  # 28
    "..............qsSqq.............",  # 29
    ".............qsssaaq............",  # 30
    ".............qqqqqqq............",  # 31
]

JUMP = HEAD + [
    "........OLMHMLMMHMMHMHMHMOqqq...",  # 18
    "........OLMHMMLMHMMLMHMHMqsaasq.",  # 19
    "........OLMHMMLMHMMLMMHMMqsassq.",  # 20
    "........OLMHMMLMHMMLMMHMHOqSSq..",  # 21
    "........OLHMMMLMHMMMLMHMHO.qq...",  # 22
    "........OLHMMMLMHMMMLMHMHO......",  # 23
    "........OMHMMMLHHMMMLMHHHO......",  # 24
    "........OMHMMHLHHMMHLHHHHO......",  # 25
    "........OHHMHHMHHMHMHHHHHO......",  # 26
    ".........OHHMHHMHHOHHMHHO.......",  # 27
    ".........OHOHHOHHOHHOHHO........",  # 28
    "..........O.OqsSqO.qsSqO........",  # 29
    "............qsssaq.qsssaq.......",  # 30
    "............qqqqqq.qqqqqq.......",  # 31
]

FALL = HEAD + [
    "........OLMHMLMMHMMHMHMHMO...qq.",  # 18
    "........OLMHMMLMHMMLMHMHMO..qsaq",  # 19
    "........OLMHMMLMHMMLMMHMMOqqsasq",  # 20
    "........OLMHMMLMHMMLMMHMHqssSqq.",  # 21
    ".......OLHMMMLMHMMMLMHMHMOqq....",  # 22
    "......OLHMMMLMHMMMLMHMHMHO......",  # 23
    ".....OMHMMMLHHMMMLMHHHMHHO......",  # 24
    "....OMHHMMHLHHMMHLHHHMHHHO......",  # 25
    "...OHHOMHHMHOHMHMHHHOHHHO.......",  # 26
    "...OO.OOHOHHOqOHHOHOOOO.........",  # 27
    "........OO.OqSqOOqSqO...........",  # 28
    "...........qsSq..qsSq...........",  # 29
    "...........qssaq.qssaq..........",  # 30
    "...........qqqqq.qqqqq..........",  # 31
]

# Brazo de atrás levantado al caer: sale de la melena hacia arriba-atrás.
FALL = [list(r) for r in FALL]
arm = [(8, 15, "s"), (8, 16, "S"), (7, 14, "s"), (7, 15, "S"), (6, 13, "s"), (6, 14, "S"), (5, 12, "s"),
       (5, 13, "S"), (4, 11, "s"), (4, 12, "S"), (3, 10, "s"), (4, 10, "a"), (3, 9, "s"), (4, 9, "s"), (3, 11, "S")]
for x, y, c in arm:
    FALL[y][x] = c
for x, y, _ in arm:
    for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
        if FALL[y + dy][x + dx] == ".":
            FALL[y + dy][x + dx] = "q"
FALL = ["".join(r) for r in FALL]

FRAMES = [IDLE, RUN1, RUN2, JUMP, FALL]

def img(rows):
    im = Image.new("RGBA", (32, 32), (0, 0, 0, 0))
    for y, r in enumerate(rows):
        assert len(r) == 32, (y, len(r), r)
        for x, c in enumerate(r):
            if P[c]:
                im.putpixel((x, y), P[c] + (255,))
    return im


if __name__ == "__main__":
    frames = [img(f) for f in FRAMES]
    W = 32 * len(frames) + 16 * (len(frames) + 1)
    sheet = Image.new("RGBA", (W, 48), (40, 34, 46, 255))
    for i, f in enumerate(frames):
        sheet.alpha_composite(f, (16 + i * 48, 8))
    sheet.resize((W * 10, 480), Image.NEAREST).save(sys.argv[1])
