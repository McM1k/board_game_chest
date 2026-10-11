"""Zoom around four approximate corners (x,y pairs) with a fine grid."""
import sys
from PIL import Image, ImageDraw
src, out = sys.argv[1], sys.argv[2]
pts = [tuple(map(int, p.split(','))) for p in sys.argv[3:7]]
R = int(sys.argv[7]) if len(sys.argv) > 7 else 60
Z = 3
im = Image.open(src).convert('RGB')
tiles = []
for (cx, cy) in pts:
    t = im.crop((cx - R, cy - R, cx + R, cy + R)).resize((2 * R * Z, 2 * R * Z), Image.NEAREST)
    d = ImageDraw.Draw(t)
    for v in range(-R, R + 1, 10):
        c = (255, 0, 0) if v % 50 == 0 else (255, 160, 0)
        d.line([((v + R) * Z, 0), ((v + R) * Z, 2 * R * Z)], fill=c)
        d.line([(0, (v + R) * Z), (2 * R * Z, (v + R) * Z)], fill=c)
        if v % 50 == 0:
            d.text(((v + R) * Z + 2, 2), str(cx + v), fill=(255, 255, 0), stroke_width=2, stroke_fill=(0, 0, 0))
            d.text((2, (v + R) * Z + 2), str(cy + v), fill=(255, 255, 0), stroke_width=2, stroke_fill=(0, 0, 0))
    tiles.append(t)
W = 2 * R * Z
o = Image.new('RGB', (2 * W, 2 * W))
for i, t in enumerate(tiles):
    o.paste(t, ((i % 2) * W, (i // 2) * W))
o.save(out)
