"""Turn a label map into a planar map: shared borders simplified once, so
neighbouring outlines match exactly. Writes JSON: per region its outline
(in board units, width 1000), a label anchor, its neighbours, and whether it
touches the board's edge."""
import sys, json
import numpy as np
from scipy import ndimage as ndi

lab_npy, seeds_json, out_json = sys.argv[1:4]
minborder = int(sys.argv[4]) if len(sys.argv) > 4 else 50
step = int(sys.argv[5]) if len(sys.argv) > 5 else 4
drop = set(tuple(p.split('-')) for p in (sys.argv[6].split(',') if len(sys.argv) > 6 and sys.argv[6] else []))
regions = json.load(open(seeds_json))
lab = np.load(lab_npy).astype(np.int32)
H, W = lab.shape
# majority-downsample to a coarse grid so outlines are smooth and small
h, w = H // step, W // step
small = np.zeros((h, w), np.int32)
blocks = lab[:h * step, :w * step].reshape(h, step, w, step).transpose(0, 2, 1, 3).reshape(h, w, step * step)
for i in range(h):
    for j in range(w):
        small[i, j] = np.bincount(blocks[i, j]).argmax()
n = len(regions)
# grid of labels with a border of 0 (outside the board)
g = np.zeros((h + 2, w + 2), np.int32)
g[1:-1, 1:-1] = small
# corners are lattice points (y, x) in 0..h+2, 0..w+2; a corner is a junction
# when the four cells around it hold 3+ distinct labels, or 2 labels on a
# diagonal, or it's where a region border meets the outside.
def cells(y, x):
    return (g[y - 1, x - 1], g[y - 1, x], g[y, x - 1], g[y, x])
# directed boundary edges: for each region, walk its boundary keeping the
# region on the left. Edge between cell (r,c) and its neighbour.
from collections import defaultdict
edges = defaultdict(list)  # region -> list of (start corner, end corner)
for y in range(h + 2):
    for x in range(w + 2):
        a = g[y, x]
        if a == 0:
            continue
        # up neighbour: edge along top side, from (y,x+1) to (y,x) keeps cell on left when going west
        if y == 0 or g[y - 1, x] != a:
            edges[a].append(((y, x + 1), (y, x)))
        if y == h + 1 or g[y + 1, x] != a:
            edges[a].append(((y + 1, x), (y + 1, x + 1)))
        if x == 0 or g[y, x - 1] != a:
            edges[a].append(((y, x), (y + 1, x)))
        if x == w + 1 or g[y, x + 1] != a:
            edges[a].append(((y + 1, x + 1), (y, x + 1)))
def is_junction(c):
    y, x = c
    if y <= 0 or x <= 0 or y >= h + 2 or x >= w + 2:
        return True
    q = cells(y, x)
    s = set(q)
    if len(s) >= 3:
        return True
    if len(s) == 2 and q[0] == q[3] and q[1] == q[2] and q[0] != q[1]:
        return True
    return False
def other_side(a, e):
    (y0, x0), (y1, x1) = e
    # the cell on the right of the directed edge
    if y0 == y1:  # horizontal
        if x1 < x0:  # going west, region below? we defined top side going west: region at (y0, x1), other at (y0-1, x1)
            return g[y0 - 1, x1] if y0 - 1 >= 0 else 0
        return g[y0, x0] if y0 < h + 2 else 0
    else:
        if y1 > y0:  # going south on west side: region at (y0, x0), other at (y0, x0-1)
            return g[y0, x0 - 1] if x0 - 1 >= 0 else 0
        return g[y1, x0] if x0 < w + 2 else 0
def rings(a):
    nxt = {}
    for s, e in edges[a]:
        nxt.setdefault(s, []).append(e)
    used = set()
    out = []
    for s0, ends in list(nxt.items()):
        for e0 in ends:
            if (s0, e0) in used:
                continue
            ring = [s0]
            cur, e = s0, e0
            while True:
                used.add((cur, e))
                ring.append(e)
                if e == s0:
                    break
                cand = [c for c in nxt[e] if (e, c) not in used]
                # prefer turning left consistently is not needed for simple shapes
                cur, e = e, cand[0]
            out.append(ring[:-1])
    return out
def dp(pts, eps):
    if len(pts) < 3:
        return pts
    a, b = np.array(pts[0], float), np.array(pts[-1], float)
    ab = b - a
    L = np.hypot(*ab)
    best, idx = -1, 0
    for i in range(1, len(pts) - 1):
        p = np.array(pts[i], float)
        d = abs(ab[0] * (a[1] - p[1]) - ab[1] * (a[0] - p[0])) / L if L else np.hypot(*(p - a))
        if d > best:
            best, idx = d, i
    if best <= eps:
        return [pts[0], pts[-1]]
    return dp(pts[:idx + 1], eps)[:-1] + dp(pts[idx:], eps)
chains = {}
def simplify_chain(pts):
    key = (pts[0], pts[-1], len(pts))
    rkey = (pts[-1], pts[0], len(pts))
    if rkey in chains:
        return list(reversed(chains[rkey]))
    if key not in chains:
        chains[key] = dp(pts, 1.1)
    return chains[key]
scale = 1000.0 / w
result = []
pairs = {}
for k, r in enumerate(regions, 1):
    polys = []
    for ring in rings(k):
        js = [i for i, c in enumerate(ring) if is_junction(c)]
        if not js:
            js = [0]
        pts = []
        for t in range(len(js)):
            i0 = js[t]
            i1 = js[(t + 1) % len(js)] if t + 1 < len(js) else js[0] + len(ring)
            seg = [ring[i % len(ring)] for i in range(i0, i1 + 1)]
            pts += simplify_chain(seg)[:-1]
        polys.append([(round((x - 1) * scale, 1), round((y - 1) * scale, 1)) for (y, x) in pts])
    # keep the biggest ring as the outline (holes are not expected)
    polys.sort(key=lambda p: -abs(sum(p[i][0] * p[i - 1][1] - p[i - 1][0] * p[i][1] for i in range(len(p)))))
    mask = small == k
    dt = ndi.distance_transform_edt(np.pad(mask, 1))[1:-1, 1:-1]
    ay, ax = np.unravel_index(dt.argmax(), dt.shape)
    edge = bool(mask[0].any() or mask[-1].any() or mask[:, 0].any() or mask[:, -1].any())
    result.append({**{k2: v for k2, v in r.items() if k2 != 'seeds'}, 'outline': polys[0], 'holes': polys[1:], 'anchor': [round((ax + .5) * scale, 1), round((ay + .5) * scale, 1)], 'edge': edge, 'rings': len(polys)})
for a_, b_ in [(small[:, :-1], small[:, 1:]), (small[:-1, :], small[1:, :])]:
    m = a_ != b_
    for p, q in zip(a_[m], b_[m]):
        key = (int(min(p, q)), int(max(p, q)))
        pairs[key] = pairs.get(key, 0) + 1
adj = {}
for (p, q), cnt in pairs.items():
    ip, iq = regions[p - 1]['id'], regions[q - 1]['id']
    if cnt * step < minborder or (ip, iq) in drop or (iq, ip) in drop:
        print('dropped', ip, iq, cnt * step)
        continue
    adj.setdefault(ip, []).append(iq)
    adj.setdefault(iq, []).append(ip)
for r in result:
    r['adj'] = sorted(adj.get(r['id'], []), key=lambda s: int(s))
json.dump({'width': 1000, 'height': round(h * scale, 1), 'regions': result}, open(out_json, 'w'), indent=1)
print(len(result), 'regions; height', round(h * scale, 1), '; multi-ring:', [r['id'] for r in result if r['rings'] > 1])
