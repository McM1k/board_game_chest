"""Fits a board's outlines to a picture of it: the homography (four
corners) that puts the shared borders between regions on the white lines
painted between them, from a rough guess of the corners."""
import json, sys
import numpy as np
from PIL import Image
from scipy import ndimage as ndi
from scipy.optimize import minimize
sys.path.insert(0, __import__('os').path.dirname(__file__))
from overlay import homography, apply, draw

def whiteness(img):
    """How much each pixel looks like the thin white lines painted between
    regions: white, unsaturated, and narrow (a white top-hat, so a table,
    a cloud or the sea's foam don't count)."""
    a = np.asarray(img.convert('RGB')).astype(float)
    mn, mx = a.min(axis=2), a.max(axis=2)
    w = np.clip((mn - 150) / 80, 0, 1) * np.clip(1 - (mx - mn) / 70, 0, 1)
    size = max(5, round(0.009 * max(img.width, img.height)))
    return np.clip(w - ndi.grey_opening(w, size=(size, size)), 0, 1)

def border_points(shapes, W, H, step=3.0, margin=6.0):
    """Points along the borders two regions share (the board's edge is no
    white line)."""
    seen = set(); pts = []
    for sh in shapes:
        p = sh['points']
        for i in range(len(p)):
            a, b = tuple(p[i]), tuple(p[(i + 1) % len(p)])
            key = tuple(sorted([a, b]))
            if key in seen:
                continue
            seen.add(key)
            n = max(1, int(np.hypot(b[0] - a[0], b[1] - a[1]) / step))
            for t in np.linspace(0, 1, n, endpoint=False):
                x, y = a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])
                if margin < x < W - margin and margin < y < H - margin:
                    pts.append((x, y))
    return np.array(pts)

def fit(img, shapes, W, H, corners, sigmas=(8, 4, 2)):
    white = whiteness(img)
    pts = border_points(shapes, W, H)
    rect = [(0, 0), (W, 0), (W, H), (0, H)]
    c = np.array(corners, float).ravel()
    for sg in sigmas:
        field = ndi.gaussian_filter(white, sg)
        def cost(v):
            Hm = homography(rect, v.reshape(4, 2))
            q = apply(Hm, pts)
            vals = ndi.map_coordinates(field, [q[:, 1], q[:, 0]], order=1, mode='constant')
            return -vals.mean()
        r = minimize(cost, c, method='Powell', options={'xtol': 0.05, 'ftol': 1e-6, 'maxfev': 20000})
        c = r.x
        print(f'  sigma {sg}: score {-r.fun:.4f}')
    return c.reshape(4, 2), -r.fun

if __name__ == '__main__':
    shapes = json.load(open(sys.argv[1]))[sys.argv[2]]
    W, H = shapes['size']
    img = Image.open(sys.argv[3])
    corners = [tuple(map(float, c.split(','))) for c in sys.argv[4:8]]
    c, score = fit(img, shapes['shapes'], W, H, corners)
    print('corners', [tuple(round(v, 1) for v in p) for p in c], 'score', round(score, 4))
    Hm = homography([(0, 0), (W, 0), (W, H), (0, H)], c)
    draw(img, shapes['shapes'], Hm, sys.argv[8], scale=float(sys.argv[9]) if len(sys.argv) > 9 else 1.0)
    json.dump({'corners': [list(map(float, p)) for p in c], 'score': score}, open(sys.argv[8] + '.json', 'w'))
