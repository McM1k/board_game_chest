"""Writes the moved outlines (boards.py's <KEY>.json) back into shapes.rs.

    python3 -I write_shapes.py <shapes.rs> <out folder> KEY..."""
import json, re, sys
from pathlib import Path

p = Path(sys.argv[1]); s = p.read_text(); out = Path(sys.argv[2])
for key in sys.argv[3:]:
    data = json.loads((out / f'{key}.json').read_text())['shapes']
    m = re.search(rf'const {key}: &\[Shape\] = &\[\n(.*?)\n\];', s, re.S)
    assert len(m.group(1).split('\n')) == len(data), key
    lines = [f'    Shape {{ at: ({sh["at"][0]}, {sh["at"][1]}), points: "{" ".join(f"{x},{y}" for x, y in sh["points"])}" }},' for sh in data]
    s = s[:m.start(1)] + '\n'.join(lines) + s[m.end(1):]
p.write_text(s)
