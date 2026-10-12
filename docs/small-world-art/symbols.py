"""Makes the symbols printed on Small World's boards, which the map's key
shows over the printed board (client/src/games/smallworld/art/symbols).

    python3 -I symbols.py <scans> <art>

<scans>/<id>.jpg are two BoardGameGeek images: 4530241, the flat scan of
the 4-player board, and 3740949, Days of Wonder's picture of the Sky
Islands' three-island side. symbols.json says where each symbol is printed
(centres in the image's pixels) and how it is cut: `half` is half the
picture's side, `radius` a disc's, `square` half a square's side.

The 4-player board prints a mine, a magic source and a cavern seven times
each, and the Lost Tribes' square fourteen times. The copies of a symbol
are aligned on the first to a quarter pixel and their median taken, which
clears the scan's JPEG noise. A disc is cut at the outer edge of its ring
(a magic source's ring comes to a point on top, `tip`, which is kept); the
square with its shadow. The Beanstalk and the Stairway carry the same
shield, so each is cut round with its vine or its stairs around it.
"""
import json, sys
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw

here = Path(__file__).parent
scans, art = Path(sys.argv[1]), Path(sys.argv[2])
spec = json.loads((here / 'symbols.json').read_text())
SIZE = spec['size']
UP = 4  # working resolution, per scan pixel
REACH = 6  # how far a copy may move while aligning, in quarter pixels


def window(im, x, y, half):
    """The square of side 2 * half around (x, y), UP times larger."""
    n = round(2 * half * UP)
    data = (1 / UP, 0, x - half, 0, 1 / UP, y - half)
    return np.asarray(im.transform((n, n), Image.AFFINE, data, resample=Image.BICUBIC), dtype=float)


def stacked(im, at, half):
    """The median of the copies at `at`, each moved onto the first."""
    first = window(im, *at[0], half)
    n = first.shape[0]
    yy, xx = np.mgrid[0:n, 0:n] - n / 2
    middle = np.hypot(xx, yy) < 0.75 * n / 2
    copies = [first]
    moves = [(dx / UP, dy / UP) for dx in range(-REACH, REACH + 1) for dy in range(-REACH, REACH + 1)]
    for x, y in at[1:]:
        tries = [window(im, x + dx, y + dy, half) for dx, dy in moves]
        copies.append(min(tries, key=lambda w: ((w - first) ** 2).sum(axis=2)[middle].mean()))
    return np.median(copies, axis=0)


def outline(sym, half, cy):
    """The symbol's shape, drawn four times larger than the picture; `cy` is
    how far below the picture's centre the disc's centre is, in scan pixels."""
    big = SIZE * 4
    s = big / (2 * half)  # mask pixels per scan pixel
    m = Image.new('L', (big, big), 0)
    d = ImageDraw.Draw(m)
    cx, cy = big / 2, big / 2 + cy * s
    if 'square' in sym:
        q = sym['square'] * s
        d.rectangle((cx - q, cy - q, cx + q, cy + q), fill=255)
        return m
    r = sym['radius'] * s
    d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=255)
    if 'tip' in sym:
        # The point, joined to the circle along the two tangents.
        tx, ty = (v * s for v in sym['tip'])
        toward, spread = np.arctan2(ty, tx), np.arccos(r / np.hypot(tx, ty))
        sides = [(cx + r * np.cos(toward + k * spread), cy + r * np.sin(toward + k * spread)) for k in (-1, 1)]
        d.polygon([sides[0], (cx + tx, cy + ty), sides[1], (cx, cy)], fill=255)
    return m


(art / 'symbols').mkdir(parents=True, exist_ok=True)
images = {}
for name, sym in spec['symbols'].items():
    if sym['scan'] not in images:
        images[sym['scan']] = Image.open(scans / f"{sym['scan']}.jpg").convert('RGB')
    half = sym['half']
    # A drop's picture is centred between its point and its bottom.
    cy = -(sym['tip'][1] + sym['radius']) / 2 if 'tip' in sym else 0
    at = [(x, y - cy) for x, y in sym['at']]
    pixels = np.clip(stacked(images[sym['scan']], at, half), 0, 255).astype(np.uint8)
    picture = Image.fromarray(pixels).resize((SIZE, SIZE), Image.LANCZOS)
    picture.putalpha(outline(sym, half, cy).resize((SIZE, SIZE), Image.LANCZOS))
    picture.save(art / 'symbols' / f'{name}.webp', quality=85, method=6)
    print(name, len(at), 'copies')
