"""The General and Financier tokens, as Game Park's online version draws
them: the round token's picture, cut out on a transparent background.

usage: python3 -I tokens.py <images> <out>

<images> is Game Park's `app/src/images` folder (only `characters` is read:
`general.jpg` and `financier.jpg`, 150 x 150, the token on white), and <out>
is `client/src/games/wonderful/art`, which receives `tokens/general.webp`
and `tokens/financier.webp`.
"""
import os
import sys

from PIL import Image, ImageDraw

images, out = sys.argv[1:3]

SIZE = 64  # twice the largest token on screen
SCALE = 4  # the circle is drawn this much bigger, then shrunk, for a smooth edge


def main():
    os.makedirs(os.path.join(out, 'tokens'), exist_ok=True)
    big = SIZE * SCALE
    mask = Image.new('L', (big, big), 0)
    ImageDraw.Draw(mask).ellipse((0, 0, big - 1, big - 1), fill=255)
    mask = mask.resize((SIZE, SIZE), Image.LANCZOS)
    total = 0
    for name in ('general', 'financier'):
        token = Image.open(os.path.join(images, 'characters', name + '.jpg')).convert('RGB')
        token = token.resize((SIZE, SIZE), Image.LANCZOS).convert('RGBA')
        token.putalpha(mask)
        path = os.path.join(out, 'tokens', name + '.webp')
        token.save(path, 'WEBP', quality=80, method=6)
        total += os.path.getsize(path)
    print(f'2 tokens, {total} bytes')


if __name__ == '__main__':
    main()
