"""The Empires' pictures, as Game Park's online version has them: a wide
banner cut from each Empire board's illustration, and the Empire's round
crest.

usage: python3 -I empires.py <images> <cards.rs> <out>

<images> is Game Park's `app/src/images` folder (only `empires` is read:
`<name>-artwork.jpg`, 1920 x 1080, and `<name>-avatar.png`, 100 x 100),
<cards.rs> is `games/wonderful/src/cards.rs`, for the Empires' names, and
<out> is `client/src/games/wonderful/art`, which receives
`empires/<slug>.webp` and `crests/<slug>.webp`.
"""
import os
import re
import sys

from PIL import Image

images, cards_rs, out = sys.argv[1:4]

BANNER = (1200, 400)  # 3:1, from the middle of the picture
CREST = 96


def slug(name):
    """As `look::art_slug`: "Noram States" is "noram-states"."""
    return re.sub(r'[^a-z0-9]+', '-', name.lower().replace("'", '')).strip('-')


def main():
    names = re.findall(r'empire\("([^"]+)"', open(cards_rs).read())
    assert len(names) == 7, names
    for sub in ('empires', 'crests'):
        os.makedirs(os.path.join(out, sub), exist_ok=True)
    total = 0
    for name in names:
        s = slug(name)
        art = Image.open(os.path.join(images, 'empires', s + '-artwork.jpg')).convert('RGB')
        w, h = art.size
        band_h = w * BANNER[1] // BANNER[0]
        top = (h - band_h) // 2
        banner = art.crop((0, top, w, top + band_h)).resize(BANNER, Image.LANCZOS)
        path = os.path.join(out, 'empires', s + '.webp')
        banner.save(path, 'WEBP', quality=65, method=6)
        total += os.path.getsize(path)
        crest = Image.open(os.path.join(images, 'empires', s + '-avatar.png')).convert('RGBA')
        crest = crest.resize((CREST, CREST), Image.LANCZOS)
        path = os.path.join(out, 'crests', s + '.webp')
        crest.save(path, 'WEBP', quality=80, method=6)
        total += os.path.getsize(path)
    print(f'{len(names)} Empires, {total // 1024} KB')


if __name__ == '__main__':
    main()
