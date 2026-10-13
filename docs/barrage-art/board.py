"""The printed map board (client/src/games/barrage/art/board.webp) from
BoardGameGeek image 5302816, a flat picture of the board with the twelve
basins outlined in white (see docs/barrage.md, "The pictures").

    python3 -I board.py <pic5302816.png> <art>

The board is cut out of the picture along its gold frame, and the white
outlines are painted over: every pure white pixel of a large white shape
(the outlines are 5 pixels wide, nothing else on the board is pure white
for long) is filled in from the pixels around it, ring after ring.
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

src, art = Path(sys.argv[1]), Path(sys.argv[2])
BOX = (8, 6, 667, 940)  # the gold frame: left, top, right, bottom

im = np.asarray(Image.open(src).convert('RGB').crop(BOX)).astype(float)
white = im.min(axis=2) >= 250
labels, n = ndimage.label(white, structure=np.ones((3, 3)))
sizes = ndimage.sum(white, labels, range(1, n + 1))
lines = np.isin(labels, 1 + np.flatnonzero(sizes > 400))
# The anti-aliased rim of each line too.
hole = ndimage.binary_dilation(lines, iterations=1) & (im.min(axis=2) >= 200)
hole |= lines

known = ~hole
out = im.copy()
out[hole] = 0
while not known.all():
    total = np.zeros_like(out)
    count = np.zeros(known.shape)
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            if dy == dx == 0:
                continue
            k = np.roll(np.roll(known, dy, 0), dx, 1)
            total += np.roll(np.roll(out * known[..., None], dy, 0), dx, 1)
            count += k
    edge = ~known & (count > 0)
    out[edge] = total[edge] / count[edge][:, None]
    known = known | edge

art.mkdir(parents=True, exist_ok=True)
Image.fromarray(out.round().astype(np.uint8)).save(art / 'board.webp', 'WEBP', quality=82, method=6)
print(f'{hole.sum()} pixels painted over, {(art / "board.webp").stat().st_size} bytes')
