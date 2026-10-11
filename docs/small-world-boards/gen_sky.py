"""Sky Islands tables: topology for the rules crate, outlines for the client."""
import json
TERRAIN = {'farm': 'Farmland', 'forest': 'Forest', 'hill': 'Hill', 'swamp': 'Swamp',
           'mountain': 'Mountain', 'sea': 'Sea', 'lake': 'Lake'}
sides = {
    'THREE': ('map-sky-three.json', [(1, 3), (2, 8), (5, 9)], [[1, 2], [3, 4, 5, 6], [7, 8, 9]], 9, 6),
    'TWO': ('map-sky-two.json', [(2, 5), (4, 9)], [[1, 2, 3, 4], [5, 6, 7, 8, 9, 10]], 4, 10),
}
core, client = [], []
for name, (f, bridges, islands, beanstalk, stairway) in sides.items():
    m = json.load(open(f))
    regs = [r for r in m['regions'] if r['id'] != 'sky']
    regs.sort(key=lambda r: int(r['id']))
    adj = {int(r['id']): set(int(a) for a in r['adj'] if a != 'sky') for r in regs}
    for a, b in bridges:
        adj[a].add(b); adj[b].add(a)
    n = len(regs)
    core.append(f"/// The Sky Islands board, {n} regions on {len(islands)} islands (cloud bridges included).")
    core.append(f"const SKY_{name}_REGIONS: &[Region] = &[")
    for r in regs:
        i = int(r['id'])
        flags = [nm for f2, nm in (('mine', 'MINE'), ('magic', 'MAGIC'), ('cavern', 'CAVERN')) if f2 in r['features']]
        if r['lt']:
            flags.append('TRIBE')
        a = ', '.join(str(x - 1) for x in sorted(adj[i]))
        core.append(f"    Region::new({TERRAIN[r['terrain']]}, {' | '.join(flags) or '0'}, &[{a}]), // {i - 1}")
    core.append("];")
    isl = ', '.join('&[' + ', '.join(str(x - 1) for x in il) + ']' for il in islands)
    core.append(f"const SKY_{name}_ISLANDS: &[&[u8]] = &[{isl}];")
    core.append("")
    client.append(f"/// The Sky Islands board with {len(islands)} islands, 1000 by {round(m['height'])}.")
    client.append(f"const SKY_{name}: &[Shape] = &[")
    for r in regs:
        pts = ' '.join(f"{round(x)},{round(y)}" for x, y in r['outline'])
        ax, ay = r['anchor']
        client.append(f"    Shape {{ at: ({round(ax)}, {round(ay)}), points: \"{pts}\" }}, // {int(r['id']) - 1}")
    client.append("];")
    client.append("")
    print(name, n, 'height', m['height'], 'beanstalk', beanstalk - 1, 'stairway', stairway - 1)
open('core_sky.rs', 'w').write('\n'.join(core))
open('client_sky.rs', 'w').write('\n'.join(client))
