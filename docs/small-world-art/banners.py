"""Makes the races' banners (client/src/games/smallworld/art/banners).

    python3 -I banners.py <exports> <scans> <art>

The expansions' banners are the rule sheets' (<exports>, as for make.py),
the white around them made transparent. The base game's are cut out of
three flatbed scans on BoardGameGeek, images 466842, 466845 and 466846
(<scans>/<id>.jpg). banner-scans.json says roughly where each banner is
(found by matching a template of what all banners share, the expansion
banners averaged). From there, each banner is straightened at the angle
that makes its bottom edge sharpest (the top has the name's plank, whose
edges look just as straight), its top is one banner's height (540 pixels
in these scans) above, and the scanner's grey bed around it is made
transparent. A banner touching a neighbour is cut from it by keeping the
largest shape once thin bridges are opened.
"""
import json, sys
from pathlib import Path
import numpy as np
from PIL import Image
from scipy import ndimage as ndi

here = Path(__file__).parent
exports, scans, art = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
picks = json.loads((here / 'picks.json').read_text())['races']
spots = json.loads((here / 'banner-scans.json').read_text())
HEIGHT = 128
BED = (205, 200, 197)
TALL = 540
(art / 'banners').mkdir(parents=True, exist_ok=True)


def save(rgba, race):
    rgba = rgba.resize((round(HEIGHT * rgba.width / rgba.height), HEIGHT), Image.LANCZOS)
    rgba.save(art / 'banners' / f'{race.lower()}.webp', quality=85, method=6)


def bed(a):
    """The scanner's grey bed."""
    return ((a.max(axis=2) - a.min(axis=2)) < 26) & (a.max(axis=2) > 150)


def edge_rows(g, x0, x1):
    gy = np.abs(ndi.sobel(ndi.gaussian_filter(g, 1.2), 0))
    return gy[:, x0:x1].mean(axis=1)


def from_scan(race, spot):
    scan = Image.open(scans / f"{spot['scan']}.jpg").convert('RGB')
    w, cx, cy = spot['width'], spot['cx'], spot['cy']
    box = (int(cx - 0.62 * w), int(cy - 0.40 * w), int(cx + 0.62 * w), int(cy + 0.40 * w))
    region = Image.new('RGB', (box[2] - box[0], box[3] - box[1]), BED)
    inside = (max(0, box[0]), max(0, box[1]), min(scan.width, box[2]), min(scan.height, box[3]))
    region.paste(scan.crop(inside), (inside[0] - box[0], inside[1] - box[1]))
    RW, RH = region.size
    x0, x1 = int(RW * 0.18), int(RW * 0.5)
    lo, hi = RH // 2 + TALL // 2 - 40, RH // 2 + TALL // 2 + 40

    def turned(ang):
        return region.rotate(ang, resample=Image.BICUBIC, fillcolor=BED)

    def sharpness(ang):
        return edge_rows(np.asarray(turned(ang).convert('L')).astype(float), x0, x1)[lo:hi].max()

    # The scans lie nearly straight: wider angles only line up a neighbour's edges.
    ang = float(max(np.arange(-1.5, 1.501, 0.05), key=sharpness))
    rot = turned(ang)
    prof = edge_rows(np.asarray(rot.convert('L')).astype(float), x0, x1)
    bot = lo + int(np.argmax(prof[lo:hi])); top = bot - TALL
    fg = ~bed(np.asarray(rot).astype(int))
    band = np.zeros_like(fg); band[top:bot + 1] = fg[top:bot + 1]
    band = ndi.binary_opening(band, structure=np.ones((9, 9)))
    lab, n = ndi.label(band)
    main = lab == (1 + int(np.argmax(ndi.sum(band, lab, range(1, n + 1)))))
    main = ndi.binary_fill_holes(ndi.binary_closing(main, structure=np.ones((5, 5))))
    xs = np.nonzero(main.any(axis=0))[0]
    left, right = int(xs.min()), int(xs.max())
    crop = rot.crop((left, top, right + 1, bot + 1)).convert('RGBA')
    alpha = ndi.gaussian_filter(main[top:bot + 1, left:right + 1].astype(float), 0.8)
    crop.putalpha(Image.fromarray((alpha * 255).clip(0, 255).astype(np.uint8)))
    return crop


def from_sheet(sheet, ref):
    im = Image.open(exports / sheet / f'doc-{ref}.png').convert('RGB')
    white = np.asarray(im).astype(int).min(axis=2) > 225
    lab, n = ndi.label(white)
    edge = set(np.unique(np.concatenate([lab[0], lab[-1], lab[:, 0], lab[:, -1]]))) - {0}
    clear = np.isin(lab, list(edge))
    alpha = ndi.gaussian_filter(np.where(clear, 0.0, 1.0), 0.7)
    rgba = im.convert('RGBA'); rgba.putalpha(Image.fromarray((alpha * 255).clip(0, 255).astype(np.uint8)))
    return rgba


for race, (sheet, ref) in picks.items():
    save(from_scan(race, spots[race]) if race in spots else from_sheet(sheet, ref), race)
print(len(picks), 'banners')
