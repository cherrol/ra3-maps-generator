# -*- coding: utf-8 -*-
"""抓取四周邻区的地级边界，用来补全主区以外的地形。"""
import json
import os
import urllib.request

_root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'maps')
_mid = open(os.path.join(_root, '.default'), encoding='utf-8').read().strip()
DEST = os.path.join(_root, _mid, 'data')
PROVINCES = [
    ('530000', 'fill_w.json', 'w'),
    ('520000', 'fill_n.json', 'n'),
    ('430000', 'fill_ne.json', 'ne'),
    ('440000', 'fill_e.json', 'e'),
]
os.makedirs(DEST, exist_ok=True)

for adcode, fname, cname in PROVINCES:
    url = 'https://geo.datav.aliyun.com/areas_v3/bound/%s_full.json' % adcode
    out = os.path.join(DEST, fname)
    if os.path.isfile(out) and os.path.getsize(out) > 1000:
        print('已有 %s (%s)' % (fname, cname))
        continue
    for attempt in range(4):
        try:
            req = urllib.request.Request(url, headers={'User-Agent': 'dsh'})
            data = urllib.request.urlopen(req, timeout=60).read()
            open(out, 'wb').write(data)
            gj = json.loads(data.decode('utf-8'))
            print('下载 %s -> %s  (%d 个行政区, %d 字节)' % (
                cname, fname, len(gj.get('features', [])), len(data)))
            break
        except Exception as e:
            if attempt == 3:
                print('失败 %s: %s' % (cname, e))
