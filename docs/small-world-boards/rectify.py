"""Warp a photo of a board to a flat rectangle from its four corners
(TL TR BR BL as x,y in the photo) to an output of W x H."""
import sys
import numpy as np
from PIL import Image
src, out = sys.argv[1], sys.argv[2]
pts = [tuple(map(float, p.split(','))) for p in sys.argv[3:7]]
W, H = int(sys.argv[7]), int(sys.argv[8])
dst = [(0, 0), (W, 0), (W, H), (0, H)]
A, b = [], []
for (x, y), (u, v) in zip(dst, pts):
    A.append([x, y, 1, 0, 0, 0, -u * x, -u * y]); b.append(u)
    A.append([0, 0, 0, x, y, 1, -v * x, -v * y]); b.append(v)
coef = np.linalg.solve(np.array(A), np.array(b))
im = Image.open(src).convert('RGB')
im.transform((W, H), Image.PERSPECTIVE, tuple(coef), Image.BICUBIC).save(out)
