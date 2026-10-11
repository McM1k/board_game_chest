"""Seeded watershed: every region floods from its seeds, floods meet on the
white border lines (and on hand-drawn cuts). Coordinates are source pixels,
limited to the board's frame. Writes labels, an overlay and the adjacency."""
import sys, json
import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage as ndi
from flood import flood
src, lines_npy, seeds_json, out = sys.argv[1:5]
ox, oy, x1, y1 = map(int, sys.argv[5:9])
cuts = json.load(open(sys.argv[9])) if len(sys.argv) > 9 else []
thin = np.load(lines_npy).astype(bool)[oy:y1, ox:x1]
h, w = thin.shape
cutimg = Image.fromarray(np.zeros((h, w), np.uint8))
dc = ImageDraw.Draw(cutimg)
for line in cuts:
    dc.line([(x - ox, y - oy) for x, y in line], fill=1, width=4)
wall = thin | np.asarray(cutimg).astype(bool)
dist = ndi.distance_transform_edt(~wall)
cost = np.clip(255 - 20 * dist, 0, 255).astype(np.uint8)
regions = json.load(open(seeds_json))
markers = np.zeros((h, w), np.int16)
for k, r in enumerate(regions, 1):
    for (x, y) in r['seeds']:
        markers[max(0, y - oy - 3):y - oy + 4, max(0, x - ox - 3):x - ox + 4] = k
lab = flood(cost, markers).astype(np.int16)
np.save(out + '.npy', lab)
img = np.asarray(Image.open(src).convert('RGB').crop((ox, oy, x1, y1))).astype(np.float32)
rng = np.random.default_rng(7)
cols = rng.integers(30, 255, size=(len(regions) + 1, 3))
over = 0.55 * img + 0.45 * cols[lab]
edge = (np.diff(lab, axis=0, prepend=lab[:1]) != 0) | (np.diff(lab, axis=1, prepend=lab[:, :1]) != 0)
edge = ndi.binary_dilation(edge)
over[edge] = (0, 0, 0)
vis = Image.fromarray(over.astype(np.uint8))
d = ImageDraw.Draw(vis)
for k, r in enumerate(regions, 1):
    x, y = r['seeds'][0]
    d.text((x - ox - 10, y - oy - 8), f"{r['id']}", fill=(255, 255, 255), stroke_width=3, stroke_fill=(0, 0, 0))
scale = float(sys.argv[10]) if len(sys.argv) > 10 else 0.55
vis.resize((int(w * scale), int(h * scale)), Image.LANCZOS).save(out)
pairs = {}
for a, b in [(lab[:, :-1], lab[:, 1:]), (lab[:-1, :], lab[1:, :])]:
    m = a != b
    for p, q in zip(a[m], b[m]):
        key = (int(min(p, q)), int(max(p, q)))
        pairs[key] = pairs.get(key, 0) + 1
adj = {f"{regions[p-1]['id']}-{regions[q-1]['id']}": n for (p, q), n in sorted(pairs.items())}
json.dump(adj, open(out + '.adj.json', 'w'), indent=0)
print(len(regions), 'regions;', len(adj), 'touching pairs')
