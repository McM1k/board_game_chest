"""Barrage's pieces and tokens (client/src/games/barrage/art/pieces) from
the rulebook's component pages (2 and 3), see docs/barrage.md, "The
pictures".

    python3 -I pieces.py <rules.pdf> <work> <art>

The pieces are drawn in the PDF as vectors, so Ghostscript renders the two
pages with the photos and the text left out (-dFILTERIMAGE -dFILTERTEXT) at
600 dpi, on transparent ground. Each piece is cut from its box below, and
the white page around it made transparent (flooded from the box's edges).
The Credit coin is a photo: it is taken with its soft mask (pdfimages).
"""
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

pdf, work, art = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
SIZE = 128  # the longer side, about twice the largest piece on screen

# Boxes at 600 dpi: page (1 for the rulebook's page 2, 2 for page 3), left,
# top, width, height. The four companies' pieces are printed in a row:
# France's white, Germany's black, Italy's green, the USA's red.
COLUMNS = {'france': 3215, 'germany': 3505, 'italy': 3800, 'usa': 4092}
ROWS = {'base': (905, 250), 'elevation': (1665, 205), 'conduit': (2315, 260),
        'powerhouse': (3128, 240), 'engineer': (5888, 305)}
BOXES = {f'{c}-{k}': (2, x, y, 290, h) for c, x in COLUMNS.items() for k, (y, h) in ROWS.items()}
BOXES.update({
    # The neutral Bases and Elevations are drawn three in a heap: the back
    # right one is taken, its left half (hidden by the front one) mirrored
    # from its right half.
    'neutral-base': (1, 3006, 4407, 222, 160),
    'neutral-elevation': (1, 3002, 5124, 222, 190),
    'drop': (1, 880, 5940, 170, 210),
    'excavator': (1, 2395, 5998, 133, 166),
    'mixer': (1, 3151, 5998, 91, 166),
})


MIRRORED = {'neutral-base', 'neutral-elevation'}


def cut(page, x, y, w, h, mirror=False):
    a = np.asarray(page.crop((x, y, x + w, y + h))).copy()
    if mirror:
        a[:, : w // 2] = a[:, ::-1][:, : w // 2]
    light = (a[:, :, :3].min(axis=2) >= 238) | (a[:, :, 3] == 0)
    lab, _ = ndimage.label(light)
    edge = set(np.unique(np.concatenate([lab[0], lab[-1], lab[:, 0], lab[:, -1]]))) - {0}
    solid = ~np.isin(lab, list(edge))
    # Only the piece itself, not a neighbour reaching into the box.
    parts, n = ndimage.label(solid)
    if n > 1:
        sizes = ndimage.sum(solid, parts, range(1, n + 1))
        solid = parts == 1 + int(np.argmax(sizes))
    a[:, :, 3] = np.where(solid, a[:, :, 3], 0)
    im = Image.fromarray(a)
    return im.crop(im.getbbox())


def shrink(im):
    im = im.copy()
    im.thumbnail((SIZE, SIZE), Image.LANCZOS)
    return im


def main():
    work.mkdir(parents=True, exist_ok=True)
    out = art / 'pieces'
    out.mkdir(parents=True, exist_ok=True)
    subprocess.run(['gs', '-q', '-dNOPAUSE', '-dBATCH', '-dSAFER', '-sDEVICE=pngalpha', '-r600',
                    '-dFILTERIMAGE', '-dFILTERTEXT', '-dFirstPage=2', '-dLastPage=3',
                    f'-sOutputFile={work}/vector-%d.png', str(pdf)], check=True)
    pages = {n: Image.open(work / f'vector-{n}.png').convert('RGBA') for n in (1, 2)}
    total = 0
    for name, (n, x, y, w, h) in BOXES.items():
        path = out / f'{name}.webp'
        shrink(cut(pages[n], x, y, w, h, name in MIRRORED)).save(path, 'WEBP', quality=85, method=6)
        total += path.stat().st_size
    # The 1-Credit coin: image 171 of the rulebook's page 2, its mask next.
    subprocess.run(['pdfimages', '-f', '2', '-l', '2', '-png', str(pdf), f'{work}/p2'], check=True)
    coin = Image.open(work / 'p2-171.png').convert('RGB')
    mask = Image.open(work / 'p2-172.png').convert('L').resize(coin.size, Image.LANCZOS)
    coin.putalpha(mask)
    coin = coin.crop(coin.getbbox())
    path = out / 'credit.webp'
    shrink(coin).save(path, 'WEBP', quality=85, method=6)
    total += path.stat().st_size
    print(f'{len(BOXES) + 1} pieces, {total} bytes')


if __name__ == '__main__':
    main()
