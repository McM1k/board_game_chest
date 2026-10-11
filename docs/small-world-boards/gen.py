"""Writes the Small World boards as Rust: topology for the rules crate,
outlines for the client."""
import json, sys

TERRAIN = {'farm': 'Farmland', 'forest': 'Forest', 'hill': 'Hill', 'swamp': 'Swamp',
           'mountain': 'Mountain', 'sea': 'Sea', 'lake': 'Lake'}
TURNS = {2: 10, 3: 10, 4: 9, 5: 8}
core_out, client_out = sys.argv[1], sys.argv[2]

maps = {n: json.load(open(f'map{n}.json')) for n in (2, 3, 4, 5)}

core = []
for n, m in maps.items():
    regs = sorted(m['regions'], key=lambda r: int(r['id']))
    assert [int(r['id']) for r in regs] == list(range(1, len(regs) + 1))
    core.append(f"/// The {n}-player board: {len(regs)} regions, {TURNS[n]} turns.")
    core.append(f"const MAP_{n}: &[Region] = &[")
    for i, r in enumerate(regs):
        flags = []
        for f, name in (('mine', 'MINE'), ('magic', 'MAGIC'), ('cavern', 'CAVERN')):
            if f in r['features']:
                flags.append(name)
        if r['lt']:
            flags.append('TRIBE')
        if r['edge']:
            flags.append('EDGE')
        adj = ', '.join(str(int(a) - 1) for a in r['adj'])
        core.append(f"    Region::new({TERRAIN[r['terrain']]}, {' | '.join(flags) or '0'}, &[{adj}]), // {i}")
    core.append("];")
    core.append("")
open(core_out, 'w').write('\n'.join(core))

client = []
for n, m in maps.items():
    regs = sorted(m['regions'], key=lambda r: int(r['id']))
    h = round(m['height'])
    client.append(f"/// The {n}-player board, {1000} by {h}.")
    client.append(f"const SHAPES_{n}: &[Shape] = &[")
    for i, r in enumerate(regs):
        pts = ' '.join(f"{round(x)},{round(y)}" for x, y in r['outline'])
        ax, ay = r['anchor']
        client.append(f"    Shape {{ at: ({round(ax)}, {round(ay)}), points: \"{pts}\" }}, // {i}")
    client.append("];")
    client.append("")
open(client_out, 'w').write('\n'.join(client))
print('ok')
