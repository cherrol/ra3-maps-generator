# -*- coding: utf-8 -*-
"""按地图取景框抓「真实高程」与「高德地形地势图」，生成 dem.npz 供 terrain_gen 使用。

高程源：AWS Terrain Tiles (terrarium PNG)
    elev = (R * 256 + G + B / 256) - 32768     （米）
蓝图参考：高德瓦片 style=7（矢量）/ style=8（地形）拼接图，存到 source/amap_terrain.png

dem.npz 的行列与地图网格一致：**第 0 行 = 最南**（游戏网格原点在左下角）。
"""
import argparse
import io
import math
import os
import sys
import urllib.request

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import project as proj          # noqa: E402
import terrain_gen as bt        # noqa: E402

UA = {'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)'}


def fetch(url, tries=3):
    last = None
    for _ in range(tries):
        try:
            return urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=25).read()
        except Exception as e:
            last = e
    raise RuntimeError('%s -> %s' % (url, last))


def lonlat_to_px(lon, lat, z):
    """经纬度 -> 全球像素坐标（Web Mercator；每张瓦片 256 px）。支持 numpy 数组。"""
    n = 256.0 * (2.0 ** z)
    lon = np.asarray(lon, dtype=np.float64)
    lat = np.asarray(lat, dtype=np.float64)
    x = (lon + 180.0) / 360.0 * n
    la = np.radians(lat)
    y = (1.0 - np.log(np.tan(la) + 1.0 / np.cos(la)) / np.pi) / 2.0 * n
    return (float(x) if x.ndim == 0 else x), (float(y) if y.ndim == 0 else y)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--map', dest='map_id', default=None)
    ap.add_argument('--zoom', type=int, default=8)
    ap.add_argument('--amap-style', type=int, default=8, help='高德瓦片 style（7 矢量 / 8 地形 / 6 卫星）')
    a = ap.parse_args()
    P = proj.current(a.map_id)
    geo = bt.Geo()
    z = a.zoom

    # ---- 取景框四角 -> 经纬度（用地图自己的映射，保证与网格严格对齐）
    def cell_to_lonlat(cx, cy):
        lon = geo.lon0 + (cx - geo.ox) / geo.sx
        lat = geo.lat1 - (cy - geo.oy) / geo.sy          # 行号大 = 南（数据第 0 行 = 最北）
        return lon, lat

    corners = [cell_to_lonlat(0, 0), cell_to_lonlat(bt.GRID, bt.GRID)]
    lon0, lat0 = corners[0]
    lon1, lat1 = corners[1]
    print('取景框 lon %.4f..%.4f  lat %.4f..%.4f' % (lon0, lon1, lat0, lat1))

    px0, py1 = lonlat_to_px(lon0, lat0, z)               # 左下（南） -> 像素 y 大
    px1, py0 = lonlat_to_px(lon1, lat1, z)
    tx0, tx1 = int(px0 // 256), int(px1 // 256)
    ty0, ty1 = int(py0 // 256), int(py1 // 256)
    print('DEM 瓦片 z=%d  x %d..%d  y %d..%d（共 %d 张）' % (
        z, tx0, tx1, ty0, ty1, (tx1 - tx0 + 1) * (ty1 - ty0 + 1)))

    W = (tx1 - tx0 + 1) * 256
    H = (ty1 - ty0 + 1) * 256
    dem = np.zeros((H, W), dtype=np.float32)
    amap = Image.new('RGB', (W, H), (0, 0, 0))
    got = 0
    for ty in range(ty0, ty1 + 1):
        for tx in range(tx0, tx1 + 1):
            try:
                raw = fetch('https://s3.amazonaws.com/elevation-tiles-prod/terrarium/%d/%d/%d.png' % (z, tx, ty))
                im = np.array(Image.open(io.BytesIO(raw)).convert('RGB'), dtype=np.float32)
                e = (im[:, :, 0] * 256.0 + im[:, :, 1] + im[:, :, 2] / 256.0) - 32768.0
                dem[(ty - ty0) * 256:(ty - ty0 + 1) * 256,
                    (tx - tx0) * 256:(tx - tx0 + 1) * 256] = e
                got += 1
            except Exception as ex:
                print('  !! DEM 瓦片 %d/%d/%d 失败: %s' % (z, tx, ty, str(ex)[:60]))
            try:
                raw = fetch('https://wprd0%d.is.autonavi.com/appmaptile?x=%d&y=%d&z=%d&lang=zh_cn&size=1&scl=1&style=%d'
                            % (1 + (tx + ty) % 4, tx, ty, z, a.amap_style))
                amap.paste(Image.open(io.BytesIO(raw)).convert('RGB'),
                           ((tx - tx0) * 256, (ty - ty0) * 256))
            except Exception:
                pass
        print('  行 %d/%d 完成（已取 %d 张 DEM）' % (ty - ty0 + 1, ty1 - ty0 + 1, got))

    # ---- 逐格采样（网格行 0 = 最北）
    yy, xx = np.mgrid[0:bt.GRID, 0:bt.GRID]
    lon = geo.lon0 + (xx - geo.ox) / geo.sx
    lat = geo.lat1 - (yy - geo.oy) / geo.sy
    pxx, pyy = lonlat_to_px(lon, lat, z)
    ix = np.clip((pxx - tx0 * 256).astype(np.int32), 0, W - 1)
    iy = np.clip((pyy - ty0 * 256).astype(np.int32), 0, H - 1)
    grid_dem = dem[iy, ix]
    out = os.path.join(os.path.dirname(os.path.abspath(bt.cfg.__file__)), 'dem.npz')
    os.makedirs(os.path.dirname(out), exist_ok=True)
    np.savez_compressed(out, dem=grid_dem, zoom=z)
    print('DEM 已写入 %s  高程 %.0f..%.0f m (陆地中位 %.0f m)' % (
        out, float(grid_dem.min()), float(grid_dem.max()), float(np.median(grid_dem))))

    amap_img = P.source_file('amap_terrain.png')
    amap.save(amap_img)
    print('高德地形蓝图已存 source/amap_terrain.png (%dx%d)' % amap.size)
    # 同时存一份对齐到网格的蓝图（便于对照）；网格行 0 = 最北，直接存即为北朝上
    arr = np.array(amap.resize((W, H)))
    ref = arr[iy, ix]
    Image.fromarray(ref).save(P.source_file('amap_terrain_aligned.png'))
    print('对齐到网格的蓝图 -> source/amap_terrain_aligned.png')


if __name__ == '__main__':
    main()
