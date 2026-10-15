# -*- coding: utf-8 -*-
"""扫描 .map 里所有对象，找出位置越界 / NaN / 超大的（游戏报 Drawable/Object position too large）。"""
import os, sys, math
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ra3map import MapFile, parse_objects, props_dict, parse_heightmap
import project as proj

P = proj.current()
m = MapFile.load(P.asset(P.map_name + '.map'))
hm = parse_heightmap(m.get('HeightMapData'))
W, H = hm['width'], hm['height']
print('地图 %dx%d，可玩区 %d..%d' % (W, H, 0, (W - 2 * P.BORDER) * P.CELL))
limit = (W) * P.CELL   # 全图范围（含边框）

objs = parse_objects(m.get('ObjectsList'), m.table)
print('对象总数:', len(objs))
bad = []
for i, o in enumerate(objs):
    x, y, zz = o['position']
    fn = (math.isnan(x) or math.isnan(y) or math.isnan(zz))
    big = (abs(x) > limit or abs(y) > limit)
    if fn or big:
        d = props_dict(o['props'])
        bad.append((i, o['type_name'], x, y, zz, d.get('uniqueID', ''), d.get('originalOwner', '')))
print('越界/非法对象: %d' % len(bad))
for b in bad:
    print('   idx%d  %-28s (%10.1f, %10.1f, %8.1f)  id=%s owner=%s' % b)
# 也打印最外圈的对象
far = sorted(objs, key=lambda o: -max(abs(o['position'][0]), abs(o['position'][1])))[:6]
print('最远的 6 个对象:')
for o in far:
    print('   %-28s (%.1f, %.1f, %.1f)' % (o['type_name'], *o['position']))
