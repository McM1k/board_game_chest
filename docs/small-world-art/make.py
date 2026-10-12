"""Makes Small World's pictures (client/src/games/smallworld/art) from Days
of Wonder's rule sheets, exported with `pdftohtml -xml` (see
docs/small-world.md, "The pictures").

    python3 -I make.py <exports> <art>

<exports>/<sheet>/doc-<page>_<n>.png are the exported images; picks.json
says which one is each race's and each power's. A race gets a round
portrait: the base game's are printed round on white, so the ring's outer
edge is found where the white starts; the expansions' are the medallion on
their banners, the same circle on every banner of a sheet
(medallions.json: centre and radius, as fractions of the banner's width
and height). A power gets its badge, the white around it made transparent.
"""
import json, sys
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw

here = Path(__file__).parent
exports, art = Path(sys.argv[1]), Path(sys.argv[2])
picks = json.loads((here / 'picks.json').read_text())
medallions = json.loads((here / 'medallions.json').read_text())
SIZE = 128


def ring_on_white(im):
    """The circle where the white background starts, all around but where
    the figure steps out of the ring."""
    white = np.asarray(im, dtype=int).min(axis=2) > 225
    H, W = white.shape
    t = np.linspace(0, 2 * np.pi, 240, endpoint=False)

    def score(cx, cy, r):
        def at(rr):
            xs = np.clip(np.round(cx + rr * np.cos(t)).astype(int), 0, W - 1)
            ys = np.clip(np.round(cy + rr * np.sin(t)).astype(int), 0, H - 1)
            return white[ys, xs]
        return np.mean(at(r + 2) & ~at(r - 2))

    m = min(W, H)
    grid = [(cx, cy, r) for r in np.arange(0.38 * m, 0.52 * m, 1.0)
            for cx in np.arange(0.3 * W, 0.7 * W, 1.0) for cy in np.arange(0.3 * H, 0.75 * H, 1.0)]
    cx0, cy0, r0 = max(grid, key=lambda c: score(*c))
    fine = [(cx, cy, r) for r in np.arange(r0 - 1, r0 + 1, 0.25)
            for cx in np.arange(cx0 - 1, cx0 + 1, 0.25) for cy in np.arange(cy0 - 1, cy0 + 1, 0.25)]
    return max(fine, key=lambda c: score(*c))


def round_crop(im, cx, cy, r):
    face = im.crop(tuple(int(round(v)) for v in (cx - r, cy - r, cx + r, cy + r))).resize((SIZE, SIZE), Image.LANCZOS)
    mask = Image.new('L', (SIZE * 4, SIZE * 4), 0)
    ImageDraw.Draw(mask).ellipse((2, 2, SIZE * 4 - 3, SIZE * 4 - 3), fill=255)
    face.putalpha(mask.resize((SIZE, SIZE), Image.LANCZOS))
    return face


def clear_white(im, tol=38):
    """The near-white around a badge, flooded from the edges, made transparent."""
    white = np.asarray(im, dtype=int).min(axis=2) > 255 - tol
    H, W = white.shape
    seen = np.zeros((H, W), bool)
    stack = [(y, x) for x in range(W) for y in (0, H - 1)] + [(y, x) for y in range(H) for x in (0, W - 1)]
    while stack:
        y, x = stack.pop()
        if 0 <= y < H and 0 <= x < W and white[y, x] and not seen[y, x]:
            seen[y, x] = True
            stack += [(y - 1, x), (y + 1, x), (y, x - 1), (y, x + 1)]
    out = im.copy()
    out.putalpha(Image.fromarray(np.where(seen, 0, 255).astype(np.uint8)))
    return out


def load(sheet, ref):
    return Image.open(exports / sheet / f'doc-{ref}.png').convert('RGB')


(art / 'races').mkdir(parents=True, exist_ok=True)
(art / 'powers').mkdir(parents=True, exist_ok=True)
for race, (sheet, ref) in picks['races'].items():
    im = load(sheet, ref)
    if sheet in medallions:
        fx, fy, fr = medallions[sheet]
        face = round_crop(im, fx * im.width, fy * im.height, fr * im.height)
    else:
        cx, cy, r = ring_on_white(im)
        face = round_crop(im, cx, cy, r * 0.97)
    face.save(art / 'races' / f'{race.lower()}.webp', quality=82, method=6)
for power, (sheet, ref) in picks['powers'].items():
    im = clear_white(load(sheet, ref))
    im.thumbnail((SIZE, SIZE), Image.LANCZOS)
    im.save(art / 'powers' / f'{power.lower()}.webp', quality=82, method=6)
print(len(picks['races']), 'races,', len(picks['powers']), 'powers')
