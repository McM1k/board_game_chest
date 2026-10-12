"""Makes the pictures of Small World's pieces, which the map draws over the
printed board (client/src/games/smallworld/art/pieces): the Troll's Lair,
the Fortress, the Encampment and so on, as the box has them.

    python3 -I pieces.py <exports> <scans> <art>

pieces.json says where each piece's picture comes from:

- `sheet`, `page`, `image`: a picture in one of Days of Wonder's rule
  sheets. <exports>/<sheet>/doc-<page>_<n>.png is the image as
  `pdftohtml -xml` exports it, and <exports>/<sheet>/page-<page>.svg the
  page as `pdftocairo -svg -f <page> -l <page>` draws it. The sheets clip
  each token's square image to the token's outline; the outline is read
  from the SVG's clip paths and mapped back onto the image's pixels.
- `scan`, `box`: a part of a BoardGameGeek image, <scans>/<id>.jpg, as
  (left, top, right, bottom) in its pixels, cut by each rule given:
  `octagon`, the box's octagon with corners cut this many pixels deep;
  `polygon`, an outline in the image's pixels (traced by hand); `white`,
  the white and light grey joining the box's edges (the background and
  the shadow a render casts on it) made transparent, this far from white.

Every picture is fitted, keeping its shape, in a SIZE x SIZE square with a
transparent outside, and saved as WebP.
"""
import base64, io, json, re, sys
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

here = Path(__file__).parent
exports, scans, art = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
spec = json.loads((here / 'pieces.json').read_text())
SIZE = spec['size']
UP = 4  # how much finer the masks are drawn than the image


def path_polygons(d, steps=16):
    """An SVG path's subpaths (M, L, C and Z only, absolute, as cairo writes
    them) as lists of points, the curves flattened."""
    polys, cur = [], []
    tokens = re.findall(r'[MLCZ]|-?[\d.]+(?:e-?\d+)?', d)
    k = 0
    while k < len(tokens):
        t = tokens[k]
        k += 1
        if t == 'M':
            if len(cur) > 2:
                polys.append(cur)
            cur = [(float(tokens[k]), float(tokens[k + 1]))]
            k += 2
        elif t == 'L':
            cur.append((float(tokens[k]), float(tokens[k + 1])))
            k += 2
        elif t == 'C':
            p0 = np.array(cur[-1])
            p1, p2, p3 = (np.array((float(tokens[k + 2 * j]), float(tokens[k + 2 * j + 1]))) for j in range(3))
            k += 6
            for s in np.linspace(0, 1, steps + 1)[1:]:
                q = (1 - s) ** 3 * p0 + 3 * (1 - s) ** 2 * s * p1 + 3 * (1 - s) * s ** 2 * p2 + s ** 3 * p3
                cur.append((float(q[0]), float(q[1])))
        elif t == 'Z':
            if len(cur) > 2:
                polys.append(cur)
            cur = cur[:1]
    if len(cur) > 2:
        polys.append(cur)
    return polys


def from_sheet(p):
    """The image `p['image']` of a rule sheet's page, with its outline."""
    png = exports / p['sheet'] / f"doc-{p['image']}.png"
    want = np.asarray(Image.open(png).convert('RGB'), dtype=int)
    svg = (exports / p['sheet'] / f"page-{p['page']}.svg").read_text()
    # The SVG's image with the same pixels as the export.
    found = None
    for m in re.finditer(r'<image id="([^"]+)" x="0" y="0" width="(\d+)" height="(\d+)"[^>]*xlink:href="data:image/png;base64,([^"]+)"', svg):
        if (int(m.group(3)), int(m.group(2))) != want.shape[:2]:
            continue
        im = Image.open(io.BytesIO(base64.b64decode(m.group(4)))).convert('RGB')
        if np.abs(np.asarray(im, dtype=int) - want).mean() < 2:
            found = m.group(1)
            break
    assert found, f'{png} is not on page {p["page"]}'
    # Where it is drawn, and the clip paths around it (innermost last).
    use = re.search(r'<use xlink:href="#' + found + r'" transform="matrix\(([^)]+)\)"/>', svg)
    a, b, c, d, e, f = (float(v) for v in use.group(1).split(','))
    before = svg[:use.start()]
    opened = []
    for g in re.finditer(r'<g clip-path="url\(#([^)]+)\)">|</g>', before):
        if g.group(1):
            opened.append(g.group(1))
        elif opened:
            opened.pop()
    h, w = want.shape[:2]
    inv = np.linalg.inv(np.array([[a, c, e], [b, d, f], [0, 0, 1]]))
    mask = Image.new('L', (w * UP, h * UP), 255)
    for clip in opened:
        body = re.search(r'<clipPath id="' + clip + r'">(.*?)</clipPath>', svg, re.S).group(1)
        layer = Image.new('L', mask.size, 0)
        draw = ImageDraw.Draw(layer)
        for path in re.findall(r'd="([^"]+)"', body):
            for poly in path_polygons(path):
                pts = np.c_[np.array(poly), np.ones(len(poly))] @ inv.T
                # Odd-even, as cairo writes the outline as one path.
                hole = Image.new('L', mask.size, 0)
                ImageDraw.Draw(hole).polygon([(x * UP, y * UP) for x, y, _ in pts], fill=255)
                layer = Image.fromarray(np.asarray(layer) ^ np.asarray(hole))
        mask = Image.fromarray(np.minimum(np.asarray(mask), np.asarray(layer)))
    im = Image.open(png).convert('RGB').resize(mask.size, Image.LANCZOS)
    im.putalpha(mask)
    return im


def from_scan(p):
    """A part of a scan, cut by every rule its entry gives."""
    x0, y0, x1, y1 = p['box']
    im = Image.open(scans / f"{p['scan']}.jpg").convert('RGB').crop((x0, y0, x1, y1))
    im = im.resize(((x1 - x0) * UP, (y1 - y0) * UP), Image.LANCZOS)
    mask = np.full(im.size[::-1], 255, dtype=np.uint8)

    def shape(points):
        m = Image.new('L', im.size, 0)
        ImageDraw.Draw(m).polygon([((x - x0) * UP, (y - y0) * UP) for x, y in points], fill=255)
        return np.asarray(m)

    if 'octagon' in p:
        c = p['octagon']
        mask = np.minimum(mask, shape([(x0 + c, y0), (x1 - c, y0), (x1, y0 + c), (x1, y1 - c),
                                       (x1 - c, y1), (x0 + c, y1), (x0, y1 - c), (x0, y0 + c)]))
    if 'polygon' in p:
        mask = np.minimum(mask, shape(p['polygon']))
    if 'white' in p:
        mask = np.minimum(mask, outside_white(im, p['white']))
    im.putalpha(Image.fromarray(mask).filter(ImageFilter.GaussianBlur(UP / 3)))
    return im


def outside_white(im, tol):
    """Opaque but where light grey or white pixels join the edges: the
    background and the soft shadow a render casts on it."""
    a = np.asarray(im, dtype=int)
    white = (a.min(axis=2) > 255 - tol) & (a.max(axis=2) - a.min(axis=2) < 16)
    seen = np.zeros_like(white)
    h, w = white.shape
    stack = [(y, x) for y in range(h) for x in (0, w - 1) if white[y, x]]
    stack += [(y, x) for x in range(w) for y in (0, h - 1) if white[y, x]]
    while stack:
        y, x = stack.pop()
        if seen[y, x] or not white[y, x]:
            continue
        seen[y, x] = True
        for yy, xx in ((y + 1, x), (y - 1, x), (y, x + 1), (y, x - 1)):
            if 0 <= yy < h and 0 <= xx < w and not seen[yy, xx]:
                stack.append((yy, xx))
    # Half a pixel inside the edge, to leave the halo out.
    return np.asarray(Image.fromarray(np.where(seen, 0, 255).astype(np.uint8)).filter(ImageFilter.MinFilter(UP + 1)))


def fitted(im):
    """In a SIZE square, as large as its shape allows, centred."""
    if im.getbbox() is not None:
        im = im.crop(Image.fromarray((np.asarray(im)[..., 3] > 8).astype(np.uint8) * 255).getbbox())
    w, h = im.size
    s = SIZE / max(w, h)
    im = im.resize((max(1, round(w * s)), max(1, round(h * s))), Image.LANCZOS)
    out = Image.new('RGBA', (SIZE, SIZE), (0, 0, 0, 0))
    out.alpha_composite(im, ((SIZE - im.size[0]) // 2, (SIZE - im.size[1]) // 2))
    return out


(art / 'pieces').mkdir(parents=True, exist_ok=True)
for name, p in spec['pieces'].items():
    im = from_sheet(p) if 'sheet' in p else from_scan(p)
    fitted(im).save(art / 'pieces' / f'{name}.webp', quality=88, method=6)
    print(name)
