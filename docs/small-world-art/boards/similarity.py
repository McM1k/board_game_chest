"""The uniform scale and offset (board units) that best put a board's
borders on its picture's white lines, searched exhaustively, coarse to fine."""
import json, sys
from pathlib import Path
import numpy as np
from PIL import Image
from scipy import ndimage as ndi
sys.path.insert(0, str(Path(__file__).parent))
from register import whiteness, border_points

def search(flat, shapes, W, H, s_range=(0.8, 1.2), t_range=120, aniso=0.0):
    s_img = flat.width / W
    white = whiteness(flat)
    pts = border_points(shapes, W, H, step=3.0)
    best = None
    plan = [(8, s_range, 0.01, t_range, 4), (4, None, 0.002, 6, 1), (2, None, 0.0005, 2, 0.25)]
    for sigma, sr, ss, tr, ts in plan:
        fld = ndi.gaussian_filter(white, sigma * s_img)
        if best is None:
            S = np.arange(sr[0], sr[1] + 1e-9, ss); TX = TY = np.arange(-tr, tr + 1e-9, ts); A = [0.0]
        else:
            _, (s0, a0, tx0, ty0) = best
            S = np.arange(s0 - 5 * ss, s0 + 5 * ss + 1e-9, ss); TX = np.arange(tx0 - tr, tx0 + tr + 1e-9, ts); TY = np.arange(ty0 - tr, ty0 + tr + 1e-9, ts)
            A = np.arange(a0 - aniso / 4, a0 + aniso / 4 + 1e-9, max(aniso / 8, 1e-9)) if aniso else [0.0]
            best = None
        for s in S:
            for a in A:
                sx, sy = s * (1 + a), s * (1 - a)
                # All offsets at once: sample the field shifted by each (tx, ty).
                base = np.c_[pts[:, 0] * sx, pts[:, 1] * sy] * s_img
                for tx in TX:
                    xs = base[:, 0] + tx * s_img
                    for ty in TY:
                        v = ndi.map_coordinates(fld, [base[:, 1] + ty * s_img, xs], order=1, mode='constant').mean()
                        if best is None or v > best[0]:
                            best = (v, (float(s), float(a), float(tx), float(ty)))
        print(f'  sigma {sigma}: {best[0]:.3f} at scale {best[1][0]:.4f} skew {best[1][1]:.4f} offset {best[1][2]:.2f},{best[1][3]:.2f}')
    return best[1]

if __name__ == '__main__':
    key = sys.argv[2]
    shapes = json.loads(Path(sys.argv[1]).read_text())[key]
    flat = Image.open(sys.argv[3])
    print(search(flat, shapes['shapes'], *shapes['size']))
