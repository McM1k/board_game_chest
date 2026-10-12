"""Draws a board's outlines over an image through a homography from board
units to image pixels."""
import json, sys
import numpy as np
from PIL import Image, ImageDraw

def homography(src, dst):
    A, b = [], []
    for (x, y), (u, v) in zip(src, dst):
        A.append([x, y, 1, 0, 0, 0, -u * x, -u * y]); b.append(u)
        A.append([0, 0, 0, x, y, 1, -v * x, -v * y]); b.append(v)
    h = np.linalg.solve(np.array(A, float), np.array(b, float))
    return np.append(h, 1).reshape(3, 3)

def apply(H, pts):
    p = np.c_[np.asarray(pts, float), np.ones(len(pts))] @ H.T
    return p[:, :2] / p[:, 2:3]

def draw(img, shapes, H, out, width=3, scale=1.0):
    im = img.copy().convert('RGB')
    d = ImageDraw.Draw(im)
    for sh in shapes:
        pts = apply(H, sh['points'])
        d.line([tuple(p) for p in pts] + [tuple(pts[0])], fill=(255, 0, 255), width=width)
    if scale != 1.0:
        im = im.resize((int(im.width * scale), int(im.height * scale)), Image.LANCZOS)
    im.save(out)

if __name__ == '__main__':
    shapes = json.load(open(sys.argv[1]))[sys.argv[2]]
    W, Hh = shapes['size']
    img = Image.open(sys.argv[3])
    corners = [tuple(map(float, c.split(','))) for c in sys.argv[4:8]]
    H = homography([(0, 0), (W, 0), (W, Hh), (0, Hh)], corners)
    draw(img, shapes['shapes'], H, sys.argv[8], scale=float(sys.argv[9]) if len(sys.argv) > 9 else 1.0)
