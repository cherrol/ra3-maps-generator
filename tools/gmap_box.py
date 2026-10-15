# -*- coding: utf-8 -*-
"""只出图：抓谷歌地形图，按广西四向最外缘裁切，导出预览（不改任何地图数据）。

输出（写到 maps/bagui_dadi/）：
  gmap_box.png         按真实经纬度比例裁切（宽 7.62° x 高 5.51°）
  gmap_box_square.png  拉伸成正方形（模拟地图 V_STRETCH 后的观感）
"""
import io
import math
import os
import sys
import urllib.request

import numpy as np
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import terrain_gen as bt          # noqa: E402

# 广西四向最外缘（上一轮从省界线量出来的）
LON_W, LON_E = 104.4422, 112.0657
LAT_S, LAT_N = 20.8850, 26.3913
Z = int(os.environ.get('GMAP_Z', '8'))
STYLE = os.environ.get('GMAP_STYLE', 'p')          # p=地形+标注, t=纯地形, s=卫星
UA = {'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)'}


def px(lon, lat, z):
    n = 256.0 * (2.0 ** z)
    x = (lon + 180.0) / 360.0 * n
    la = math.radians(lat)
    y = (1.0 - math.log(math.tan(la) + 1.0 / math.cos(la)) / math.pi) / 2.0 * n
    return x, y


def fetch(url, tries=3):
    last = None
    for _ in range(tries):
        try:
            return urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=25).read()
        except Exception as e:
            last = e
    raise RuntimeError(str(last)[:80])


def main():
    x0, y1 = px(LON_W, LAT_S, Z)
    x1, y0 = px(LON_E, LAT_N, Z)
    tx0, tx1 = int(x0 // 256), int(x1 // 256)
    ty0, ty1 = int(y0 // 256), int(y1 // 256)
    W, H = (tx1 - tx0 + 1) * 256, (ty1 - ty0 + 1) * 256
    canvas = Image.new('RGB', (W, H), (0, 0, 0))
    got = 0
    for ty in range(ty0, ty1 + 1):
        for tx in range(tx0, tx1 + 1):
            for host in (1, 2, 3, 4):
                try:
                    raw = fetch('https://mt%d.google.com/vt/lyrs=%s&hl=zh-CN&x=%d&y=%d&z=%d'
                                % (host, STYLE, tx, ty, Z))
                    canvas.paste(Image.open(io.BytesIO(raw)).convert('RGB'),
                                 ((tx - tx0) * 256, (ty - ty0) * 256))
                    got += 1
                    break
                except Exception:
                    continue
        print('  行 %d/%d 完成' % (ty - ty0 + 1, ty1 - ty0 + 1))

    crop = canvas.crop((int(x0 - tx0 * 256), int(y0 - ty0 * 256),
                        int(x1 - tx0 * 256), int(y1 - ty0 * 256)))
    out = os.path.join(os.path.dirname(os.path.abspath(bt.cfg.__file__)), 'gmap_box.png')
    crop.save(out)
    print('抓到 %d 张瓦片（z=%d lyrs=%s）' % (got, Z, STYLE))
    print('  真实比例裁切 %dx%d -> %s' % (crop.size[0], crop.size[1], out))

    # 正方形（模拟 V_STRETCH：地图是 1000x1000）
    side = 900
    sq = crop.resize((side, side), Image.LANCZOS)
    d = ImageDraw.Draw(sq)
    d.rectangle([0, 0, side - 1, side - 1], outline=(255, 60, 40), width=4)
    d.text((10, 8), 'Guangxi bbox -> square %dx%d  (Google terrain z=%d lyrs=%s)'
           % (side, side, Z, STYLE), fill=(255, 255, 255))
    out2 = os.path.join(os.path.dirname(os.path.abspath(bt.cfg.__file__)), 'gmap_box_square.png')
    sq.save(out2)
    print('  正方形(拉伸)  %dx%d -> %s' % (side, side, out2))
    print()
    print('提示：地图可玩区 800~1000 格。按 1000 格算，1 格 ≈ %.4f° 经 / %.4f° 纬' % (
        (LON_E - LON_W) / 1000.0, (LAT_N - LAT_S) / 1000.0))
    print('     真实比例下 宽/高 = %.2f（正方形地图必然要纵向拉伸 %.2f 倍）' % (
        (LON_E - LON_W) / (LAT_N - LAT_S) * 0.78, (LAT_N - LAT_S) / ((LON_E - LON_W) * 0.78) * 1000 / 1000))


if __name__ == '__main__':
    main()
