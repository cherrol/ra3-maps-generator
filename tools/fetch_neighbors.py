# -*- coding: utf-8 -*-
"""抓取邻省（云南/贵州/湖南/广东）地级市边界，用于补全广西省外的地形。"""
import json
import os
import urllib.request

DEST = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'maps', 'bagui_dadi', 'data')
PROVINCES = [
    ('530000', 'yunnan_boundary.json', '云南省'),
    ('520000', 'guizhou_boundary.json', '贵州省'),
    ('430000', 'hunan_boundary.json', '湖南省'),
    ('440000', 'guangdong_boundary.json', '广东省'),
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
