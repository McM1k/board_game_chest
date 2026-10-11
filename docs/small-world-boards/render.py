"""Draw an exported map with flat terrain colours to eyeball it."""
import sys, json
from PIL import Image, ImageDraw
m = json.load(open(sys.argv[1]))
out = sys.argv[2]
S = 0.9
col = {'farm': (232, 196, 90), 'forest': (40, 110, 50), 'hill': (140, 200, 90), 'swamp': (120, 95, 70),
       'mountain': (170, 170, 180), 'sea': (70, 140, 210), 'lake': (90, 170, 230), None: (220, 120, 220)}
im = Image.new('RGB', (int(1000 * S), int(m['height'] * S)), (30, 30, 30))
d = ImageDraw.Draw(im)
for r in m['regions']:
    for ring in [r['outline']] + r.get('holes', []):
        d.polygon([(x * S, y * S) for x, y in ring], fill=col.get(r.get('terrain')), outline=(255, 255, 255))
for r in m['regions']:
    ax, ay = r['anchor']
    tag = r['id'] + ''.join(t[0].upper() for t in r.get('features', [])) + ('*' if r.get('lt') else '')
    d.text((ax * S - 8, ay * S - 6), tag, fill=(0, 0, 0), stroke_width=2, stroke_fill=(255, 255, 255))
im.save(out)
