# -*- coding: utf-8 -*-
"""Deep inspect a RA3 map: sides, teams, objects (waypoints), mp positions, textures."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ra3map import *

def main(path):
    m = MapFile.load(path)
    hm = parse_heightmap(m.get('HeightMapData'))
    print(f"HeightMap: {hm['width']}x{hm['height']} border_w={hm['border_width']} borders={hm['borders']} playable={hm['width']-2*hm['border_width']}")
    print("elev min/max:", min(hm['elevations']), max(hm['elevations']))

    bt = parse_blendtiles(m.get('BlendTileData'), hm['width'], hm['height'], m.table)
    print(f"BlendTileData v{bt['version']} consumed={bt['consumed']} size={len(m.get('BlendTileData').payload)} tiles={bt['num_tiles']}")
    print("  textures:", [(t['name'], t['cell_start'], t['cell_size']) for t in bt['textures']])
    print("  magic1=%08x magic2=%08x blends=%d cliffs=%d" % (bt['magic1'], bt['magic2'], bt['blends_count'], bt['cliff_count']))
    print("  tile value histogram (top):", sorted({t: bt['tiles'].count(t) for t in set(bt['tiles'])}.items(), key=lambda kv: -kv[1])[:8])

    sl = parse_sides(m.get('SidesList'), m.table, has_asset_list=m.get('AssetList') is not None)
    print(f"SidesList v{sl['version']} num_players={sl['num_players']} consumed={sl['consumed']}/{len(m.get('SidesList').payload)}")
    for i, p in enumerate(sl['players']):
        d = props_dict(p['props'])
        print(f"  player[{i}] {d}  buildlist={len(p['build_list'])}")

    teams = parse_teams(m.get('Teams'), m.table)
    print(f"Teams: {len(teams)}")
    for t in teams:
        print("   ", props_dict(t))

    objs = parse_objects(m.get('ObjectsList'), m.table)
    print(f"Objects: {len(objs)}")
    from collections import Counter
    print("  type histogram:", Counter(o['type_name'] for o in objs).most_common(20))
    print("  waypoints:")
    for o in objs:
        if 'Waypoint' in o['type_name']:
            print("   ", o['position'], {k: v for k, t, v in o['props']})

    mp = parse_mp_positions(m.get('MPPositionList'), m.table)
    print("MPPositionList:", mp)

    print("WaypointsList paths:", parse_waypointslist(m.get('WaypointsList')))

if __name__ == '__main__':
    main(sys.argv[1])
