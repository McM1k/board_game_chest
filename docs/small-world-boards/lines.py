"""Thin white border lines: bright, unsaturated pixels that survive a white
top-hat (big white blobs such as snow are dropped), then only long pieces.
Coordinates of the output mask are those of the source image."""
import sys
import numpy as np
from PIL import Image
from scipy import ndimage as ndi
src, out = sys.argv[1], sys.argv[2]
width = int(sys.argv[3]) if len(sys.argv) > 3 else 9
minspan = int(sys.argv[4]) if len(sys.argv) > 4 else 70
bright = int(sys.argv[5]) if len(sys.argv) > 5 else 175
im = np.asarray(Image.open(src).convert('RGB')).astype(np.int16)
mn, mx = im.min(axis=2), im.max(axis=2)
white = (mn > bright) & (mx - mn < 60)
opened = ndi.binary_opening(white, structure=np.ones((width, width), bool))
thin = white & ~opened
lab, n = ndi.label(thin, structure=np.ones((3, 3)))
keep = np.zeros(n + 1, bool)
for i, sl in enumerate(ndi.find_objects(lab), 1):
    keep[i] = max(sl[0].stop - sl[0].start, sl[1].stop - sl[1].start) >= minspan
clean = keep[lab]
np.save(out + '.npy', clean)
vis = np.asarray(Image.open(src).convert('RGB')).copy()
vis[clean] = (255, 0, 255)
Image.fromarray(vis).save(out)
print(n, 'pieces,', keep.sum(), 'kept')
