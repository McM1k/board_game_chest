"""The round tracker, as Game Park's online version draws it: a ring of
arrows around the round number, green on the side used in rounds 1 and 3
(hands pass to the next seat), pink on the other (rounds 2 and 4, to the
previous seat).

usage: python3 -I round.py <images> <out>

<images> is Game Park's `app/src/images` folder (only
`round-tracker-recto.png` and `round-tracker-verso.png` are read, 200 x 200),
and <out> is `client/src/games/wonderful/art`, which receives
`round/next.webp` and `round/previous.webp`.
"""
import os
import sys

from PIL import Image

images, out = sys.argv[1:3]

SIZE = 112  # twice the ring on screen


def main():
    os.makedirs(os.path.join(out, 'round'), exist_ok=True)
    total = 0
    for side, name in (('recto', 'next'), ('verso', 'previous')):
        ring = Image.open(os.path.join(images, f'round-tracker-{side}.png')).convert('RGBA')
        ring = ring.resize((SIZE, SIZE), Image.LANCZOS)
        path = os.path.join(out, 'round', name + '.webp')
        ring.save(path, 'WEBP', quality=85, method=6)
        total += os.path.getsize(path)
    print(f'2 rings, {total} bytes')


if __name__ == '__main__':
    main()
