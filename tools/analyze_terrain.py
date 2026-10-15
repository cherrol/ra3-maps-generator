# -*- coding: utf-8 -*-
"""Analyze a map's terrain/water setup: elevation stats, water settings, textures."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ra3map import *
from collections import Counter

def analyze(path, label=None):
    m = MapFile.load(path)
    label = label or os.path.basename(path)
    print('=' * 70)
    print('MAP', label)
    hm = parse_heightmap(m.get('HeightMapData'))
    e = hm['elevations']
    print(f"dims {hm['width']}x{hm['height']} border {hm['border_width']} bounds={hm['borders']}")
    print(f"elev raw: min={min(e)} max={max(e)}  (z: {min(e)*0.0390625:.1f} .. {max(e)*0.0390625:.1f})")
    print("elev histogram (raw//256):", sorted(Counter(v // 256 for v in e).items())[:20])
    print("elev histogram (raw//1024):", sorted(Counter(v // 1024 for v in e).items())[:20])
    gws = m.get('GlobalWaterSettings')
    if gws:
        r = R(gws.payload)
        print(f"GlobalWaterSettings v{gws.version}: refl={r.u8()} pad={r.d[r.p:r.p+3].hex()} planeZ={r.f32()}")
    wi = m.get('WorldInfo')
    if wi:
        r = R(wi.payload); props = read_props(r, m.table)
        for k, t, v in props:
            if k in ('mapName', 'mapDescription', 'weather', 'cameraGroundMinHeight', 'cameraGroundMaxHeight', 'isScenarioMultiplayer'):
                print(f"  WorldInfo {k} = {v!r}")
    bt = parse_blendtiles(m.get('BlendTileData'), hm['width'], hm['height'], m.table)
    print(f"BlendTileData v{bt['version']} textures={len(bt['textures'])}: {[t['name'] for t in bt['textures']][:40]}")
    if bt['textures']:
        print("   cell_size:", Counter(t['cell_size'] for t in bt['textures']))
    for cn in ('StandingWaterAreas', 'RiverAreas', 'StandingWaveAreas', 'TriggerAreas'):
        c = m.get(cn)
        if c:
            cnt, = struct.unpack_from('<I', c.payload, 0) if len(c.payload) >= 4 else (None,)
            print(f"  {cn} v{c.version} count={cnt} size={len(c.payload)}")
    return m, hm, bt

if __name__ == '__main__':
    for p in sys.argv[1:]:
        try:
            analyze(p)
        except Exception as ex:
            print('ERR', p, ex)
