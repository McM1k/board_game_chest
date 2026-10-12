"""Cut each card's illustration out of its printed face, as Game Park's online
version has it, into a 2:1 band clear of the printed costs and icons.

usage: python3 -I cards.py <images> <cards.rs> <out>

<images> is Game Park's `app/src/images` folder (only `cards/developments/en`
is read), <cards.rs> is `games/wonderful/src/cards.rs`, whose designs say
how many cost spaces each card prints, and <out> receives `<slug>.webp`, one
per design (`client/src/games/wonderful/art/cards`).

`focus.json`, next to this script, frames the cards that the middle of the
free area frames badly, chosen by eye from contact sheets. Under "at", a
card's band moves up or down: 0 puts it at the top of the free area, 1 at
the bottom, 0.5 (the default) in the middle. A card with a second cost
column has its band under that column, as wide as the card allows, or, if
it is listed under "beside_second_column" (or there is no room under it),
beside both columns over their whole height, narrower.
"""
import json
import os
import re
import sys

from PIL import Image

images, cards_rs, out = sys.argv[1:4]
here = os.path.dirname(os.path.abspath(__file__))
focus = json.load(open(os.path.join(here, 'focus.json')))

# Game Park's file names, where the card's name doesn't give them.
ALIASES = {"Pandora's Box": 'PandoraBox'}

# The printed face is 650 x 1000. The free area is under the title bar
# (y 125) and above the construction bonus and recycling icons (y 725),
# right of the cost column (x 128). A card printing more than nine cost
# spaces has a second column beside the first, x 128 to 245, from y 140,
# one space every 95 pixels.
TOP, BOTTOM, RIGHT = 125, 725, 650
COLUMN_1, COLUMN_2 = 128, 245
WIDTH, QUALITY = 480, 70


def designs():
    """(name, number of cost spaces) for every design in the catalogue."""
    src = open(cards_rs).read()
    found = []
    pattern = r'c\("([^"]+)", \w+, \[([\d, ]+)\], (?:\[[\d, ;]+\]), \w, \d+\)([^\n]*)'
    for m in re.finditer(pattern, src):
        name, cost, rest = m.groups()
        spaces = sum(int(x) for x in cost.split(','))
        k = re.search(r'\.krystallium\((\d+)\)', rest)
        chars = re.search(r'\.chars\((\d+), (\d+)\)', rest)
        spaces += int(k.group(1)) if k else 0
        spaces += int(chars.group(1)) + int(chars.group(2)) if chars else 0
        found.append((name, spaces))
    return found


def slug(name):
    """As `look::art_slug`: "Pandora's Box" is "pandoras-box"."""
    return re.sub(r'[^a-z0-9]+', '-', name.lower().replace("'", '')).strip('-')


def free_area(spaces, beside):
    if spaces <= 9:
        return (COLUMN_1, TOP, RIGHT, BOTTOM)
    second_column_end = 160 + 95 * (spaces - 9)
    if not beside and BOTTOM - second_column_end >= (RIGHT - COLUMN_1) // 2:
        return (COLUMN_1, second_column_end, RIGHT, BOTTOM)
    return (COLUMN_2, TOP, RIGHT, BOTTOM)


def band(spaces, at, beside):
    """The 2:1 box to cut: as wide as the free area, `at` of the way down it."""
    x0, y0, x1, y1 = free_area(spaces, beside)
    w = x1 - x0
    h = w // 2
    top = y0 + round(at * (y1 - y0 - h))
    return (x0, top, x1, top + h)


def main():
    files = {}
    folder = os.path.join(images, 'cards', 'developments', 'en')
    for f in os.listdir(folder):
        m = re.match(r'\w+?_\d+_EN_(\w+)\.jpg$', f)
        if m:
            files[m.group(1).lower()] = os.path.join(folder, f)
    os.makedirs(out, exist_ok=True)
    all_designs = designs()
    assert len(all_designs) == 116, f'{len(all_designs)} designs found in {cards_rs}'
    at, beside = focus['at'], set(focus['beside_second_column'])
    unknown = (set(at) | beside) - {name for name, _ in all_designs}
    assert not unknown, f'focus.json names no card: {unknown}'
    total = 0
    for name, spaces in all_designs:
        key = ALIASES.get(name, re.sub(r'[^A-Za-z0-9]', '', name)).lower()
        face = Image.open(files[key]).convert('RGB')
        assert face.size == (650, 1000), (name, face.size)
        crop = face.crop(band(spaces, at.get(name, 0.5), name in beside))
        crop = crop.resize((WIDTH, WIDTH // 2), Image.LANCZOS)
        path = os.path.join(out, slug(name) + '.webp')
        crop.save(path, 'WEBP', quality=QUALITY, method=6)
        total += os.path.getsize(path)
    print(f'{len(all_designs)} cards, {total // 1024} KB')


if __name__ == '__main__':
    main()
