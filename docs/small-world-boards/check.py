"""Draw the region outlines of a label map over the photo, with ids and a grid,
for one window (source coordinates)."""
import sys, json
import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage as ndi
src, lab_npy, seeds_json, out = sys.argv[1:5]
ox, oy = map(int, sys.argv[5:7])
x0, y0, x1, y1 = map(int, sys.argv[7:11])
scale = float(sys.argv[11]) if len(sys.argv) > 11 else 1.0
lab = np.load(lab_npy)
regions = json.load(open(seeds_json))
img = np.asarray(Image.open(src).convert('RGB')).copy()
full = np.zeros(img.shape[:2], np.int32)
full[oy:oy + lab.shape[0], ox:ox + lab.shape[1]] = lab
edge = (np.diff(full, axis=0, prepend=full[:1]) != 0) | (np.diff(full, axis=1, prepend=full[:, :1]) != 0)
img[ndi.binary_dilation(edge)] = (255, 0, 0)
vis = Image.fromarray(img[y0:y1, x0:x1])
vis = vis.resize((int(vis.size[0] * scale), int(vis.size[1] * scale)), Image.LANCZOS)
d = ImageDraw.Draw(vis)
for gx in range((x0 // 100 + 1) * 100, x1, 100):
    d.text(((gx - x0) * scale + 2, 2), str(gx), fill=(255, 255, 0), stroke_width=2, stroke_fill=(0, 0, 0))
    d.line([((gx - x0) * scale, 0), ((gx - x0) * scale, 8)], fill=(255, 255, 0), width=2)
for gy in range((y0 // 100 + 1) * 100, y1, 100):
    d.text((2, (gy - y0) * scale + 2), str(gy), fill=(255, 255, 0), stroke_width=2, stroke_fill=(0, 0, 0))
    d.line([(0, (gy - y0) * scale), (8, (gy - y0) * scale)], fill=(255, 255, 0), width=2)
for r in regions:
    for (x, y) in r['seeds']:
        if x0 <= x < x1 and y0 <= y < y1:
            d.text(((x - x0) * scale - 6, (y - y0) * scale - 8), r['id'], fill=(255, 255, 255), stroke_width=3, stroke_fill=(0, 0, 0))
vis.save(out)
