"""The outlines in client/src/games/smallworld/shapes.rs, as JSON.

    python3 -I export_shapes.py <shapes.rs> <shapes.json>"""
import json, re, sys

s = open(sys.argv[1]).read()
sizes = {m.group(4): (int(m.group(2)), int(m.group(3)))
         for m in re.finditer(r'static (\w+): Board = Board \{ width: (\d+), height: (\d+), shapes: (\w+) \}', s)}
out = {}
for m in re.finditer(r'const (\w+): &\[Shape\] = &\[(.*?)\n\];', s, re.S):
    shapes = [{'at': [int(a), int(b)], 'points': [list(map(float, p.split(','))) for p in pts.split()]}
              for a, b, pts in re.findall(r'Shape \{ at: \((-?\d+), (-?\d+)\), points: "([^"]+)" \}', m.group(2))]
    out[m.group(1)] = {'size': sizes[m.group(1)], 'shapes': shapes}
json.dump(out, open(sys.argv[2], 'w'))
