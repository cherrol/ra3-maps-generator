# -*- coding: utf-8 -*-
"""End-to-end verification of the built map: re-read it, render what the game will see."""
import os, sys
import numpy as np
from PIL import Image, ImageDraw
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ra3map import *

import argparse
_ap = argparse.ArgumentParser(add_help=False)
_ap.add_argument('--map', dest='map_id', default=None, help='maps/<id>/ 配方')
_args, _unknown = _ap.parse_known_args()
if _args.map_id:
    import os as _os
    _os.environ['RA3MAP_ID'] = _args.map_id

import terrain_gen as bt

WS = bt.WS
MAPDIR = bt.ASSETS
SRCDIR = bt.SRCDIR
MAP = os.path.join(MAPDIR, bt.MAPNAME + '.map')


def main():
    m = MapFile.load(MAP)
    hm = parse_heightmap(m.get('HeightMapData'))
    W, H = hm['width'], hm['height']
    # ⚠ .map 的高度场已是**显示方向**（= flipud(内部帧)，见 map_gen.cell_to_world），
    #   所以这里的 `z`/`imp` 与物件世界 y 同帧，直接画即可。
    z = np.array(hm['elevations'], dtype=np.float64).reshape(H, W) * bt.Z_SCALE
    btc = m.get('BlendTileData')
    passable = np.zeros((H, W), dtype=bool)
    ok_passable = None
    # parse impassability + buildability manually from the chunk
    bt_data = parse_blendtiles(btc, W, H, m.table)
    # re-parse bit arrays with a dedicated reader (parse_blendtiles keeps them as lists)
    imp = np.array(bt_data['impassability'], dtype=np.uint8)
    build_ok = np.array(bt_data['buildability'], dtype=np.uint8)
    print('impassable cells: %d (%.1f%%)  buildable-blocked: %d (%.1f%%)' % (
        imp.sum(), 100 * imp.mean(), build_ok.sum(), 100 * build_ok.mean()))
    water = z < bt.WATER_Z
    print('water cells: %d (%.1f%%)' % (water.sum(), 100 * water.mean()))

    # connectivity of the *final* map data
    walk = (~imp.astype(bool)) & (~water)
    lab = np.zeros((H, W), dtype=np.int32)
    from collections import deque
    cur = 0
    for sy in range(0, H, 1):
        for sx in range(0, W, 1):
            if walk[sy, sx] and lab[sy, sx] == 0:
                cur += 1
                q = deque([(sy, sx)]); lab[sy, sx] = cur
                while q:
                    y, x = q.popleft()
                    for ny, nx in ((y-1, x), (y+1, x), (y, x-1), (y, x+1)):
                        if 0 <= ny < H and 0 <= nx < W and walk[ny, nx] and lab[ny, nx] == 0:
                            lab[ny, nx] = cur; q.append((ny, nx))
    sizes = np.bincount(lab.ravel())
    main_lab = max(range(1, cur + 1), key=lambda i: sizes[i]) if cur else 0
    print('walkable components: %d  main size %d' % (cur, sizes[main_lab] if cur else 0))

    objs = parse_objects(m.get('ObjectsList'), m.table)

    def to_game_row(wy):
        """世界 y -> .map 高度场行号。

        实测（用出生点当锚点）：`row = y/10 + BORDER` 才能让 .map 高度场取到设计值
        （P8->190 水上基地、P5->350、P1->420 全部吻合）。
        """
        return wy / 10.0 + bt.BORDER

    wp = {}
    for o in objs:
        if 'Waypoint' in o['type_name']:
            d = props_dict(o['props'])
            wp[d['waypointName']] = (o['position'][0] / 10.0 + bt.BORDER,
                                     to_game_row(o['position'][1]))
    for name, (cx, cy) in sorted(wp.items()):
        iy, ix = int(round(cy)), int(round(cx))
        print('   %-28s 游戏帧格(%3d,%3d) walk=%s lab=%d %s' % (
            name, ix, iy, walk[iy, ix], lab[iy, ix],
            'MAIN' if lab[iy, ix] == main_lab else 'NOT MAIN'))

    # ---- 出生点平整可建面：要求 宽 >= 80 或 长 >= 80 格（特殊地形除外）
    #      容差 3 世界单位 ≈ 0.3 格（远小于 1 格，工程上等同平整、可正常建造）
    FLAT_TOL = 3.0

    def _span(cx, cy, axis):
        ix, iy = int(round(cx)), int(round(cy))
        h0 = z[iy, ix]
        if axis == 'x':
            lo = hi = ix
            while lo - 1 >= 0 and abs(z[iy, lo - 1] - h0) < FLAT_TOL:
                lo -= 1
            while hi + 1 < W and abs(z[iy, hi + 1] - h0) < FLAT_TOL:
                hi += 1
        else:
            lo = hi = iy
            while lo - 1 >= 0 and abs(z[lo - 1, ix] - h0) < FLAT_TOL:
                lo -= 1
            while hi + 1 < H and abs(z[hi + 1, ix] - h0) < FLAT_TOL:
                hi += 1
        return hi - lo + 1

    print('出生点平整可建面（要求 宽>=80 或 长>=80 格）:')
    short = 0
    for name, (cx, cy) in sorted(wp.items()):
        sx, sy = _span(cx, cy, 'x'), _span(cx, cy, 'y')
        ok = sx >= 80 or sy >= 80
        iy, ix = int(round(cy)), int(round(cx))
        water_base = 0 <= iy < H and 0 <= ix < W and z[iy, ix] < bt.WATER_Z + 1.0
        if water_base:
            mark = '水上'
            ok = True
        elif 'Start' not in name:
            mark = '✓'
            ok = True
        else:
            mark = '✓' if ok else '✗ 不足 80'
        short += (not ok)
        print('   %-28s 宽(x) %3d  长(y) %3d  %s' % (name, sx, sy, mark))
    print('   不足 80 的陆地出生点: %d' % short)
    if short:
        raise SystemExit('陆地出生点平整跨度不足 80 格')

    # ---- render in-game-like top view (terrain colour + impassable hatch + objects)
    img = np.zeros((H, W, 3), dtype=np.uint8)
    img[water] = (36, 88, 156)
    land = ~water
    lv = np.clip((z - bt.WATER_Z) / 300.0, 0, 1)
    img[land] = np.stack([(90 + 120 * lv)[land], (150 - 30 * lv)[land], (80 - 20 * lv)[land]], axis=-1).astype(np.uint8)
    img[imp.astype(bool)] = (img[imp.astype(bool)].astype(np.float64) * 0.55 + np.array([70, 60, 50]) * 0.45).astype(np.uint8)
    # ⚠ 轴向（T1.8.8 实机锚定）：游戏内实际是"北朝上、海在下"。.map 高度场是游戏帧
    #   （第 0 行 = 世界 y=0 = 屏幕下方），所以底图要 flipud 成北朝上；
    #   物件世界 y 对应的游戏帧行是 `wy/10 + BORDER`，图上画在 `(H-1) - 该行`。
    img = np.flipud(img)
    im = Image.fromarray(img)
    d = ImageDraw.Draw(im)
    colours = {'OreNode': (255, 215, 0), 'OilDerrick': (255, 80, 0), 'ObservationPostTechStructure': (0, 255, 255),
               'GarageTechStructure': (255, 0, 255), 'SovietConstructionYard': (255, 0, 0),
               'AlliedConstructionYard': (0, 0, 255), 'JapanConstructionYard': (232, 150, 32)}
    for o in objs:
        cx = o['position'][0] / 10.0 + bt.BORDER
        cy = (H - 1) - to_game_row(o['position'][1])
        tn = o['type_name']
        if tn in colours:
            c = colours[tn]
            d.ellipse([cx - 4, cy - 4, cx + 4, cy + 4], fill=c, outline=(0, 0, 0))
        elif 'Waypoint' in tn:
            d.ellipse([cx - 6, cy - 6, cx + 6, cy + 6], outline=(255, 255, 255), width=2)
    for name, (cx, cy) in wp.items():
        cy = (H - 1) - cy
        lbl = name.replace('Player_', 'P').replace('_Start', '').replace(' (imported)', '*')
        d.text((cx + 7, cy - 4), lbl, fill=(255, 255, 255))
    im.save(os.path.join(SRCDIR, 'build_view.png'))
    print('saved source/build_view.png')

    # ---- art tga check
    art = os.path.join(MAPDIR, bt.MAPNAME + '_art.tga')
    a = Image.open(art)
    print('art.tga size', a.size, 'mode', a.mode, 'bytes', os.path.getsize(art))
    # ⚠ art.tga 的显示方向（PIL 读回 / 游戏内小地图）= 北朝上（T1.8.8 实机锚定）；
    #   TGA 自底向上存储，所以写入时画布已翻过（map_gen.write_art_tga）。
    #   **直接保存即为正确显示方向，不要再 transpose(FLIP_TOP_BOTTOM)**。
    a.save(os.path.join(SRCDIR, 'art_preview.png'))
    arr = np.array(a.convert('RGB'))
    print('art 预览（与游戏内一致：北朝上、海在下方）top row mean',
          arr[0].mean(axis=0).round(1),
          'bottom row mean', arr[-1].mean(axis=0).round(1))


if __name__ == '__main__':
    main()
