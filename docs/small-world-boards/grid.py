"""Crop an image region and draw a labelled coordinate grid (in source pixels)."""
import sys
from PIL import Image, ImageDraw
src, out, x0, y0, x1, y1 = sys.argv[1], sys.argv[2], *map(int, sys.argv[3:7])
step = int(sys.argv[7]) if len(sys.argv) > 7 else 50
scale = float(sys.argv[8]) if len(sys.argv) > 8 else 1.0
im = Image.open(src).convert('RGB').crop((x0, y0, x1, y1))
im = im.resize((int(im.size[0]*scale), int(im.size[1]*scale)), Image.LANCZOS)
d = ImageDraw.Draw(im)
for x in range((x0//step+1)*step, x1, step):
    X = (x-x0)*scale
    d.line([(X,0),(X,im.size[1])], fill=(255,0,0) if x%(step*2)==0 else (255,140,0), width=1)
    d.text((X+2,2), str(x), fill=(255,255,0), stroke_width=2, stroke_fill=(0,0,0))
for y in range((y0//step+1)*step, y1, step):
    Y = (y-y0)*scale
    d.line([(0,Y),(im.size[0],Y)], fill=(255,0,0) if y%(step*2)==0 else (255,140,0), width=1)
    d.text((2,Y+2), str(y), fill=(255,255,0), stroke_width=2, stroke_fill=(0,0,0))
im.save(out)
