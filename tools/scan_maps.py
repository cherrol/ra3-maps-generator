# -*- coding: utf-8 -*-
"""Scan all custom maps: dims, wayspoints, sides players -> template shortlist."""
import sys, os, glob, traceback
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ra3map import *

MAPS_DIR = r'C:\Users\cherrol\AppData\Roaming\Red Alert 3\Maps'

def scan(folder):
    mappath = None
    for f in os.listdir(folder):
        if f.lower().endswith('.map'):
            mappath = os.path.join(folder, f)
            break
    if not mappath:
        return None
    m = MapFile.load(mappath)
    info = {'folder': os.path.basename(folder), 'map': os.path.basename(mappath), 'size': os.path.getsize(mappath)}
    hm = m.get('HeightMapData')
    if hm:
        h = parse_heightmap(hm)
        info['dims'] = f"{h['width']}x{h['height']}"
        info['border'] = h['border_width']
        info['playable'] = h['width'] - 2 * h['border_width']
        info['zmin'] = min(h['elevations']); info['zmax'] = max(h['elevations'])
    sl = m.get('SidesList')
    if sl:
        s = parse_sides(sl, m.table, has_asset_list=m.get('AssetList') is not None)
        names = []
        for p in s['players']:
            d = props_dict(p['props'])
            names.append(d.get('playerName', ''))
        info['players'] = names
        # waypoints
    try:
        objs = parse_objects(m.get('ObjectsList'), m.table)
        wps = []
        for o in objs:
            if 'Waypoint' in o['type_name']:
                d = props_dict(o['props'])
                wps.append(d.get('waypointName', ''))
        info['waypoints'] = wps
        info['n_obj'] = len(objs)
        from collections import Counter
        info['objtypes'] = Counter(o['type_name'] for o in objs).most_common(6)
    except Exception as e:
        info['objerr'] = str(e)
    info['chunks'] = [c.name for c in m.chunks]
    return info

def main():
    out = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'map_scan.txt'), 'w', encoding='utf-8')
    folders = sorted(d for d in glob.glob(os.path.join(MAPS_DIR, '*')) if os.path.isdir(d))
    for i, folder in enumerate(folders):
        try:
            info = scan(folder)
            if info is None:
                print(f'[{i}] {os.path.basename(folder)}: NO .map', file=out, flush=True)
                continue
            names = info.get('players', [])
            play = [n for n in names if n.startswith('Player_')]
            wps = info.get('waypoints', [])
            starts = [w for w in wps if 'Start' in w]
            print(f"[{i}] {info['folder']} | {info['dims']} play={info.get('playable')} z={info.get('zmin')}-{info.get('zmax')} | sides={len(names)} mpplayers={play} | waypoints({len(wps)})={starts} | import={[w for w in wps if 'import' in w]} | n_obj={info.get('n_obj')} | {info.get('objtypes')}", file=out, flush=True)
        except Exception as e:
            print(f'[{i}] {os.path.basename(folder)}: ERROR {e}', file=out, flush=True)
    out.close()
    print('done')

if __name__ == '__main__':
    main()
