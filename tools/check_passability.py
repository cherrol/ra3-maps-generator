# -*- coding: utf-8 -*-
"""Check slope-based passability / connectivity between the 8 bases; render a preview."""
import os, sys, math, json
import numpy as np
from PIL import Image
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import terrain_gen as bt

WS = bt.WS
SLOPE_LIMIT = 26.0    # world-z units per cell (1 cell = 10 world units)


def main():
    d = np.load(os.path.join(bt.SRCDIR, 'terrain.npz'))
    raw = d['raw']
    z = raw.astype(np.float64) * bt.Z_SCALE
    gy, gx = np.gradient(z)
    slope = np.sqrt(gx ** 2 + gy ** 2)
    yy, xx = np.mgrid[0:bt.GRID, 0:bt.GRID]
    d_edge = np.minimum.reduce([xx - bt.BORDER, yy - bt.BORDER,
                                (bt.GRID - 1 - bt.BORDER) - xx, (bt.GRID - 1 - bt.BORDER) - yy]).astype(np.float64)
    land_ok = z > bt.WATER_Z + 1.0
    _ramps = json.loads(str(d['ramp_json'])) if 'ramp_json' in d.files else []
    passable = land_ok & (slope < SLOPE_LIMIT) & (d_edge > 1) & ~bt.shore_block_mask(z, _ramps)

    geo = bt.Geo()
    # 出生点坐标：优先用 terrain.npz 里保存的（已按省界/河道/城市吸附过的）格坐标；
    # 没有才退回经纬度换算，并夹进可玩区 —— 否则像百色这种偏北的纬度经 V_STRETCH 拉伸后
    # 会算出 y>GRID，直接 IndexError（自 v11 之前就存在的坑）。
    # npz 里的名字是生成那一版时的 BASES 名称。改名后按顺序对齐，
    # 不要用名字查找，也不要退回 BASES 里的旧经纬度。
    saved_cells = []
    try:
        saved_cells = [(float(c[0]), float(c[1])) for c in d['base_cells']]
    except Exception:
        saved_cells = []
    lim = bt.GRID - 1
    bases = {}
    for i, (lon, lat, name, _, _) in enumerate(bt.BASES):
        if i < len(saved_cells):
            cx, cy = saved_cells[i]
        else:
            cx, cy = geo.to_cell(lon, lat)
        bases[name] = (max(0.0, min(lim, cx)), max(0.0, min(lim, cy)))

    # label connected components (4-neighbour flood fill)
    lab = np.zeros(passable.shape, dtype=np.int32)
    cur = 0
    from collections import deque
    for sy in range(bt.GRID):
        for sx in range(bt.GRID):
            if passable[sy, sx] and lab[sy, sx] == 0:
                cur += 1
                q = deque([(sy, sx)])
                lab[sy, sx] = cur
                while q:
                    y, x = q.popleft()
                    for ny, nx in ((y-1, x), (y+1, x), (y, x-1), (y, x+1)):
                        if 0 <= ny < bt.GRID and 0 <= nx < bt.GRID and passable[ny, nx] and lab[ny, nx] == 0:
                            lab[ny, nx] = cur
                            q.append((ny, nx))
    sizes = np.bincount(lab.ravel())
    cand = [i for i in range(1, cur + 1)]
    main_lab = max(cand, key=lambda i: sizes[i]) if cand else 0
    order = np.argsort(-sizes)
    print('passable cells: %d (%.1f%%)  components: %d' % (passable.sum(), 100*passable.mean(), cur))
    print('largest components:', [(int(i), int(sizes[i])) for i in order[:6] if i != 0])
    print('bases:')
    ok_all = True
    for name, (cx, cy) in bases.items():
        iy, ix = int(round(cy)), int(round(cx))
        on_water = z[iy, ix] < bt.WATER_Z + 1.0
        l = lab[iy, ix]
        inmain = (l == main_lab)
        # 水上基地不在陆路连通区里，这是设计结果，不记为断连
        if on_water:
            print('   %-4s cell(%3d,%3d) z=%6.1f slope=%5.1f WATER（不要求陆路连通）' % (
                name, ix, iy, z[iy, ix], slope[iy, ix]))
            continue
        ok_all &= inmain
        print('   %-4s cell(%3d,%3d) z=%6.1f slope=%5.1f label=%d %s' % (
            name, ix, iy, z[iy, ix], slope[iy, ix], l, 'OK' if inmain else '<<< ISOLATED'))
    print('ALL LAND BASES CONNECTED:', ok_all)
    if not ok_all:
        raise SystemExit('有陆地出生点不在主连通区')

    # preview
    img = np.zeros((bt.GRID, bt.GRID, 3), dtype=np.uint8)
    img[..., 0] = np.where(z < bt.WATER_Z, 30, 60)
    img[..., 1] = np.where(z < bt.WATER_Z, 70, 110)
    img[..., 2] = np.where(z < bt.WATER_Z, 140, 60)
    img[passable] = (120, 200, 120)
    img[passable & (lab != main_lab)] = (230, 120, 60)
    # ⚠ 轴向（T1.8.8 实机锚定）：显示方向 = 内部帧（行 0 = 北），**不 flipud**，
    #   出生点标注直接用内部帧行号 cy。
    im = Image.fromarray(img)
    from PIL import ImageDraw
    dd = ImageDraw.Draw(im)
    for name, (cx, cy) in bases.items():
        dd.ellipse([cx-6, cy-6, cx+6, cy+6], fill=(255, 255, 0), outline=(0, 0, 0))
    im.save(os.path.join(bt.SRCDIR, 'passability_preview.png'))
    print('saved source/passability_preview.png')


if __name__ == '__main__':
    main()
