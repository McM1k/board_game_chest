"""Straightens a picture of a board onto its frame, then moves the traced
outlines onto the white lines painted between regions: a smooth field of
local shifts, found near each control point of a grid, applied to every
outline point (shared points move together, so neighbours stay
neighbours). Points on the board's edge only slide along it."""
import json, sys
import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage as ndi
from scipy.interpolate import RegularGridInterpolator
sys.path.insert(0, __import__('os').path.dirname(__file__))
from overlay import homography, apply
from register import whiteness, border_points


def straighten(img, corners, W, H, out_w):
    """The board inside its frame, in board units scaled to out_w across."""
    s = out_w / W; oh = round(H * s)
    Hm = homography([(0, 0), (W, 0), (W, H), (0, H)], corners)
    ys, xs = np.mgrid[0:oh, 0:out_w]
    q = apply(Hm, np.c_[((xs + 0.5) / s).ravel(), ((ys + 0.5) / s).ravel()])
    a = np.asarray(img.convert('RGB')).astype(float)
    ch = [ndi.map_coordinates(a[..., k], [q[:, 1], q[:, 0]], order=3, mode='nearest').reshape(oh, out_w) for k in range(3)]
    return Image.fromarray(np.clip(np.stack(ch, -1), 0, 255).astype(np.uint8))


def field_at(D, gx, gy, p):
    q = np.c_[p[:, 1], p[:, 0]]
    return np.c_[RegularGridInterpolator((gy, gx), D[..., 0], bounds_error=False, fill_value=None)(q),
                 RegularGridInterpolator((gy, gx), D[..., 1], bounds_error=False, fill_value=None)(q)]


def solve(flat, shapes, W, H, spacing, radius, stages):
    """Shifts (in board units) that put the borders on the white lines."""
    s = flat.width / W
    white = whiteness(flat)
    pts = border_points(shapes, W, H, step=2.0)
    gx = np.linspace(0, W, max(3, round(W / spacing) + 1)); gy = np.linspace(0, H, max(3, round(H / spacing) + 1))
    D = np.zeros((len(gy), len(gx), 2))
    near = {}
    for j, y in enumerate(gy):
        for i, x in enumerate(gx):
            d = np.hypot(pts[:, 0] - x, pts[:, 1] - y)
            idx = np.nonzero(d < radius)[0]
            near[(j, i)] = (idx, 1 - d[idx] / radius)
    for sigma, reach, step in stages:
        fld = ndi.gaussian_filter(white, sigma * s)
        def score(idx, w):
            q = (pts[idx] + field_at(D, gx, gy, pts[idx])) * s
            return (ndi.map_coordinates(fld, [q[:, 1], q[:, 0]], order=1, mode='constant') * w).sum() / w.sum()
        for sweep in range(3):
            for (j, i), (idx, w) in near.items():
                if len(idx) < 20:
                    continue
                c0 = D[j, i].copy(); best = (score(idx, w), c0.copy())
                for dx in np.arange(-reach, reach + 1e-9, step):
                    for dy in np.arange(-reach, reach + 1e-9, step):
                        D[j, i] = c0 + (dx, dy)
                        v = score(idx, w)
                        if v > best[0]:
                            best = (v, D[j, i].copy())
                D[j, i] = best[1]
            for (j, i), (idx, w) in near.items():
                if len(idx) < 20:
                    nb = [D[jj, ii] for jj, ii in [(j - 1, i), (j + 1, i), (j, i - 1), (j, i + 1)] if 0 <= jj < len(gy) and 0 <= ii < len(gx)]
                    D[j, i] = np.mean(nb, axis=0)
            D = np.stack([ndi.median_filter(D[..., k], size=3, mode='nearest') * 0.3 + D[..., k] * 0.7 for k in range(2)], -1)
        q = (pts + field_at(D, gx, gy, pts)) * s
        v = ndi.map_coordinates(ndi.gaussian_filter(white, 1.2 * s), [q[:, 1], q[:, 0]], order=1, mode='constant').mean()
        q0 = pts * s
        v0 = ndi.map_coordinates(ndi.gaussian_filter(white, 1.2 * s), [q0[:, 1], q0[:, 0]], order=1, mode='constant').mean()
        print(f'  sigma {sigma}: borders on the lines {v0:.3f} -> {v:.3f}')
    return D, gx, gy


def global_fit(flat, shapes, W, H):
    """Scale and offset in x and y that best put the borders on the lines."""
    s = flat.width / W
    white = whiteness(flat)
    pts = border_points(shapes, W, H, step=3.0)
    best = (-1, (1, 1, 0, 0))
    for sigma, ranges in ((8, (0.12, 0.02, 80, 8)), (4, (0.03, 0.005, 16, 2)), (2, (0.008, 0.002, 4, 0.5))):
        fld = ndi.gaussian_filter(white, sigma * s)
        sx0, sy0, tx0, ty0 = best[1]
        ds, ss, dt, st = ranges
        for sx in np.arange(sx0 - ds, sx0 + ds + 1e-9, ss):
            for sy in np.arange(sy0 - ds, sy0 + ds + 1e-9, ss):
                for tx in np.arange(tx0 - dt, tx0 + dt + 1e-9, st):
                    for ty in np.arange(ty0 - dt, ty0 + dt + 1e-9, st):
                        q = np.c_[pts[:, 0] * sx + tx, pts[:, 1] * sy + ty] * s
                        v = ndi.map_coordinates(fld, [q[:, 1], q[:, 0]], order=1, mode='constant').mean()
                        if v > best[0]:
                            best = (v, (sx, sy, tx, ty))
        best = (-1, best[1])
    print('  global', tuple(round(float(v), 4) for v in best[1]))
    sx, sy, tx, ty = best[1]
    out = []
    for sh in shapes:
        pts2 = [[x * sx + tx, y * sy + ty] for x, y in sh['points']]
        out.append({'at': [sh['at'][0] * sx + tx, sh['at'][1] * sy + ty], 'points': pts2})
    return out


def moved(shapes, D, gx, gy, W, H):
    out = []
    for sh in shapes:
        p = np.array(sh['points'], float)
        q = p + field_at(D, gx, gy, p)
        for k in range(len(p)):
            for axis, lim in ((0, W), (1, H)):
                if p[k, axis] <= 0.5 or p[k, axis] >= lim - 0.5:
                    q[k, axis] = p[k, axis]
        q[:, 0] = q[:, 0].clip(0, W); q[:, 1] = q[:, 1].clip(0, H)
        at = np.array([sh['at']], float); at = at + field_at(D, gx, gy, at)
        out.append({'at': [int(round(at[0, 0])), int(round(at[0, 1]))], 'points': [[int(round(x)), int(round(y))] for x, y in q]})
    return out


def check(flat, shapes, W, out):
    s = flat.width / W
    im = flat.copy(); d = ImageDraw.Draw(im)
    for sh in shapes:
        pts = [(x * s, y * s) for x, y in sh['points']]
        d.line(pts + [pts[0]], fill=(255, 0, 255), width=2)
    im.save(out)


if __name__ == '__main__':
    allshapes = json.load(open(sys.argv[1])); key = sys.argv[2]
    W, H = allshapes[key]['size']; shapes = allshapes[key]['shapes']
    img = Image.open(sys.argv[3])
    corners = [tuple(map(float, c.split(','))) for c in sys.argv[4:8]]
    out = sys.argv[8]; out_w = int(sys.argv[9])
    flat = straighten(img, corners, W, H, out_w)
    flat.save(out)
    check(flat, shapes, W, out.replace('.png', '-before.png'))
    shapes = global_fit(flat, shapes, W, H)
    check(flat, shapes, W, out.replace('.png', '-global.png'))
    D, gx, gy = solve(flat, shapes, W, H, spacing=80, radius=130, stages=((6, 40, 4), (3, 8, 1), (1.5, 2, 0.5)))
    new = moved(shapes, D, gx, gy, W, H)
    check(flat, new, W, out.replace('.png', '-after.png'))
    json.dump({'key': key, 'size': [W, H], 'corners': [list(c) for c in corners], 'shapes': new}, open(out.replace('.png', '.json'), 'w'))
