"""Every board's picture in board units, and its outlines moved onto the
picture's white lines.

    python3 -I boards.py <work> [KEY...]

<work>/boards/<id>.jpg are the pictures boards.json names (BoardGameGeek
image ids), <work>/map/shapes.json the outlines (export_shapes.py). Writes
<work>/map/out/<KEY>.png (the picture), <KEY>.json (the moved outlines) and
<KEY>-before.png / -after.png to check them by eye."""
import json, sys
from pathlib import Path
import numpy as np
from PIL import Image
from scipy import ndimage as ndi
sys.path.insert(0, str(Path(__file__).parent))
from overlay import homography, apply
from register import fit
from refit import solve, moved, check
from similarity import search

root = Path(sys.argv[1]); only = sys.argv[2:]
conf = json.loads((Path(__file__).parent / 'boards.json').read_text())
allshapes = json.loads((root / 'map/shapes.json').read_text())
out = root / 'map/out'; out.mkdir(parents=True, exist_ok=True)


def corners_for(spec, shapes, W, H):
    """The picture's corners of the board as traced: rough corners, then the
    scale and offset that best put the borders on the white lines, then
    (for a photograph at an angle) the perspective fitted from there."""
    img = Image.open(root / 'boards' / f"{spec['src']}.jpg").convert('RGB')
    c = np.array(spec['corners'], float)
    a, _ = straighten(img, c, W, H, 800)
    sc, sk, tx, ty = search(Image.fromarray(a.astype(np.uint8)), shapes, W, H, s_range=spec.get('scale', (0.8, 1.25)), t_range=spec.get('offset', 140), aniso=0.08)
    sx, sy = sc * (1 + sk), sc * (1 - sk)
    H0 = homography([(0, 0), (W, 0), (W, H), (0, H)], c)
    c = apply(H0, [(tx, ty), (W * sx + tx, ty), (W * sx + tx, H * sy + ty), (tx, H * sy + ty)])
    if spec.get('perspective'):
        c, score = fit(img, shapes, W, H, c, sigmas=(4, 2))
        print('  perspective fit', round(score, 3))
    return img, np.array(c, float)


def straighten(img, corners, W, H, out_w):
    """The board in board units, and where the picture had it at all."""
    s = out_w / W; oh = round(H * s)
    Hm = homography([(0, 0), (W, 0), (W, H), (0, H)], corners)
    ys, xs = np.mgrid[0:oh, 0:out_w]
    q = apply(Hm, np.c_[((xs + 0.5) / s).ravel(), ((ys + 0.5) / s).ravel()])
    a = np.asarray(img).astype(float)
    ch = [ndi.map_coordinates(a[..., k], [q[:, 1], q[:, 0]], order=3, mode='nearest').reshape(oh, out_w) for k in range(3)]
    inside = ((q[:, 0] >= 0) & (q[:, 0] < img.width - 1) & (q[:, 1] >= 0) & (q[:, 1] < img.height - 1)).reshape(oh, out_w)
    return np.clip(np.stack(ch, -1), 0, 255), inside


for key, spec in conf.items():
    if only and key not in only:
        continue
    print(key)
    W, H = allshapes[key]['size']; shapes = allshapes[key]['shapes']
    img, c = corners_for(spec, shapes, W, H)
    a, inside = straighten(img, c, W, H, spec['width'])
    if 'fallback' in spec:
        fimg, fc = corners_for(spec['fallback'], shapes, W, H)
        b, _ = straighten(fimg, fc, W, H, spec['width'])
        # Blend the fallback in where the main picture runs out.
        m = ndi.gaussian_filter(inside.astype(float), 4)[..., None]
        a = a * m + b * (1 - m)
    flat = Image.fromarray(a.astype(np.uint8))
    flat.save(out / f'{key}.png')
    check(flat, shapes, W, str(out / f'{key}-before.png'))
    D, gx, gy = solve(flat, shapes, W, H, spacing=90, radius=130, stages=((5, 16, 2), (2.5, 5, 1), (1.4, 1.5, 0.5)))
    new = moved(shapes, D, gx, gy, W, H)
    check(flat, new, W, str(out / f'{key}-after.png'))
    (out / f'{key}.json').write_text(json.dumps({'size': [W, H], 'corners': c.tolist(), 'shapes': new}))
