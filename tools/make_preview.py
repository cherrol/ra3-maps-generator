# -*- coding: utf-8 -*-
"""Render a presentation preview of the finished map (Chinese labels, legend)."""
import os, sys, math
import numpy as np
from PIL import Image, ImageDraw, ImageFont
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
import map_gen as mg
PREVIEW = mg.PREVIEW

WS = bt.WS
MAPDIR = bt.ASSETS
SRCDIR = bt.SRCDIR
MAP = os.path.join(MAPDIR, bt.MAPNAME + '.map')
FONT_CANDIDATES = [r'C:\Windows\Fonts\msyhbd.ttc', r'C:\Windows\Fonts\msyh.ttc',
                   r'C:\Windows\Fonts\simhei.ttf']


def font(size):
    for p in FONT_CANDIDATES:
        if os.path.exists(p):
            try:
                return ImageFont.truetype(p, size)
            except Exception:
                pass
    return ImageFont.load_default()


def main():
    d = np.load(os.path.join(SRCDIR, 'terrain.npz'))
    z = d['raw'].astype(np.float64) * bt.Z_SCALE
    gx_boundary = d['gx_boundary'] if 'gx_boundary' in d else None
    m = MapFile.load(MAP)
    hm = parse_heightmap(m.get('HeightMapData'))
    bt2 = parse_blendtiles(m.get('BlendTileData'), hm['width'], hm['height'], m.table)
    # ⚠ 轴向（T1.8.8 实机锚定，以此为准）：
    #   · 内部帧（terrain.npz 的 raw / v11 蓝图 / 用户标注图 / geo 派生物）：**第 0 行 = 最北**
    #   · 游戏内实际显示：**北在屏幕上、海（南）在屏幕下** = 内部帧的方向。
    #     （.map 高度场数组第 0 行 = 世界 y=0 = 屏幕**下**方；T1.8.3 时代"游戏里北在下、
    #       海在上"的说法是错的——错把高度场数组行序当成了屏幕方向。）
    #   · 本预览与游戏内一致 = 北朝上 => 底图直接用 raw（内部帧），**不 flipud**；
    #     边界掩码 / 城市（内部帧）直接画；物件世界 y 换算回内部帧行号
    #     `row = (GRID-1-BORDER) - wy/10`；.map 的 BlendTileData 是游戏帧，要 flipud。
    gx_boundary = gx_boundary if gx_boundary is not None else None
    # .map 里的 BlendTileData 是游戏帧（行 0 = 南），翻回内部帧
    imp = np.flipud(np.array(bt2['impassability'], dtype=bool))
    water = z < bt.WATER_Z

    S = 2
    img = np.zeros((bt.GRID, bt.GRID, 3), dtype=np.uint8)
    img[water] = (37, 92, 158)
    deep = z < bt.WATER_Z - 45
    img[deep] = (22, 60, 122)
    land = ~water
    lv = np.clip((z - bt.WATER_Z) / 320.0, 0, 1) ** 0.85
    idx = np.clip((lv * 255).astype(int), 0, 254)
    ramp = np.zeros((256, 3), dtype=np.uint8)
    for i in range(256):
        f = i / 255.0
        if f < 0.42:
            g = f / 0.42; ramp[i] = (62 + 48 * g, 126 + 44 * g, 62 - 12 * g)
        elif f < 0.66:
            g = (f - 0.42) / 0.24; ramp[i] = (110 + 52 * g, 170 - 18 * g, 50 - 20 * g)
        elif f < 0.85:
            g = (f - 0.66) / 0.19; ramp[i] = (162 + 28 * g, 152 - 22 * g, 30 + 24 * g)
        else:
            g = (f - 0.85) / 0.15; ramp[i] = (190 + 58 * g, 130 + 85 * g, 54 + 135 * g)
    img[land] = ramp[idx[land]]
    # hill-shade for readability
    gy, gx = np.gradient(z)
    shade = np.clip(1.0 - 0.030 * (gx + gy), 0.72, 1.22)
    img = np.clip(img.astype(np.float64) * shade[..., None], 0, 255).astype(np.uint8)
    # impassable hatch
    img[imp & land] = (img[imp & land].astype(np.float64) * 0.62 +
                       np.array([92, 70, 52]) * 0.38).astype(np.uint8)

    # ⚠ 预览底图 = 内部帧（行 0 = 北），不再 flipud。
    im = Image.fromarray(img).resize((bt.GRID * S, bt.GRID * S), Image.LANCZOS)
    dr = ImageDraw.Draw(im, 'RGBA')
    # 高亮主省省界
    if gx_boundary is not None and gx_boundary.any():
        bimg = Image.fromarray(gx_boundary.astype(np.uint8) * 255).resize(
            (bt.GRID * S, bt.GRID * S), Image.NEAREST)
        bmask = np.array(bimg) > 96
        arrv = np.array(im)
        arrv[bmask] = (255, 70, 40)
        im = Image.fromarray(arrv)
        dr = ImageDraw.Draw(im, 'RGBA')
    f_base = font(26)
    f_small = font(20)
    f_title = font(46)
    f_leg = font(22)

    objs = parse_objects(m.get('ObjectsList'), m.table)
    colours = {'OreNode': (255, 214, 0), 'OilDerrick': (255, 92, 0),
               'ST_Building01': (190, 225, 255), 'ST_Building02': (140, 185, 255),
               'ObservationPostTechStructure': (0, 235, 235), 'GarageTechStructure': (235, 0, 235),
               'SovietConstructionYard': (240, 40, 40), 'AlliedConstructionYard': (60, 90, 255),
               'JapanConstructionYard': (232, 150, 32)}
    # 物件世界 y -> 内部帧行号（cell_to_world 的逆）：row = (GRID-1-BORDER) - wy/10
    OY = (bt.GRID - 1 - bt.BORDER) * S

    def obj_xy(o):
        return ((o['position'][0] / 10.0 + bt.BORDER) * S,
                OY - o['position'][1] / 10.0 * S)

    # 桥梁：画成醒目的绿色粗线段（预览里不画的话根本看不出有没有桥）
    for o in objs:
        tn = o['type_name']
        if 'Bridge' not in tn:
            continue
        cx, cy = obj_xy(o)
        a = float(o.get('angle') or 0.0)
        # 桥段是"半座桥"，沿桥向两侧各画一段，两段拼成完整桥
        L = 26 * S
        dx, dy = math.cos(a) * L * 0.5, -math.sin(a) * L * 0.5
        dr.line([cx - dx, cy - dy, cx + dx, cy + dy], fill=(60, 255, 120), width=9)
        dr.ellipse([cx - 4, cy - 4, cx + 4, cy + 4], fill=(255, 255, 255), outline=(20, 60, 20))
    # 城市名标注：标在城市房屋旁边（每座城市 1~2 个房子）
    # ⚠ city_centers 与底图同为**内部帧**（第 0 行 = 最北），直接画、不翻 y
    for cn, ccx, ccy in [(n, cx, cy) for n, cx, cy in bt.city_centers(bt.Geo()) if n in bt.CITY_SEATS]:
        cx = ccx * S
        cy = ccy * S
        dr.text((cx + 14, cy + 4), cn, font=f_small, fill=(255, 245, 200),
                stroke_width=3, stroke_fill=(0, 0, 0))
    # 每类独立资源编号标注（矿1..矿N / 油井1..N / 医院1..N …），方便后续按编号调整
    res_kinds = [('OreNode', '矿', (255, 214, 0)), ('OilDerrick', '油井', (255, 92, 0)),
                 ('ObservationPostTechStructure', '哨', (0, 235, 235)),
                 ('GarageTechStructure', '车库', (235, 0, 235)),
                 ('AirportTechStructure', '机场', (255, 255, 255)),
                 ('VeterancyTechStructure', '学院', (180, 255, 120)),
                 ('HospitalTechStructure', '医院', (255, 160, 200))]
    for tn, pref, col in res_kinds:
        objs_k = [o for o in objs if o['type_name'] == tn]
        objs_k.sort(key=lambda o: (-o['position'][1], o['position'][0]))     # 由北到南（世界 y 大 = 北）
        for i, o in enumerate(objs_k, 1):
            cx, cy = obj_xy(o)
            txt = '%s%d' % (pref, i)
            dr.text((cx + 8, cy - 6), txt, font=f_small, fill=col,
                    stroke_width=2, stroke_fill=(0, 0, 0))
    # 桥梁也编号
    brs = [o for o in objs if 'Bridge' in o['type_name']]
    brs.sort(key=lambda o: (-o['position'][1], o['position'][0]))
    for i in range(0, len(brs) - 1, 2):
        cx, cy = obj_xy(brs[i])
        dr.text((cx + 8, cy - 6), '桥%d' % (i // 2 + 1), font=f_small, fill=(60, 255, 120),
                stroke_width=2, stroke_fill=(0, 0, 0))
    for o in objs:
        cx, cy = obj_xy(o)
        tn = o['type_name']
        if tn in colours:
            c = colours[tn]
            r = 7 if tn == 'OreNode' else 9
            dr.ellipse([cx - r, cy - r, cx + r, cy + r], fill=c, outline=(20, 20, 20), width=2)
    # 斜坡编号与验收序号一致，并画出上坡方向，方便按编号改角度。
    if 'ramp_json' in d.files:
        import json
        _ramps = json.loads(str(d['ramp_json']))
        for i, rs in enumerate(_ramps, 1):
            cx = float(rs['x']) * S
            cy = float(rs['y']) * S
            ux = float(rs.get('ux') or 0.0)
            uy = float(rs.get('uy') or 0.0)
            nrm = math.hypot(ux, uy) or 1.0
            ux, uy = ux / nrm, uy / nrm
            half = float(rs.get('span', rs.get('w', 25))) * 0.5 * S
            wide = max(4.0, float(rs.get('width', rs.get('h', 25))) * 0.22 * S)
            corners = []
            for su, sv in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
                corners.append((cx + ux * su * half - uy * sv * wide,
                                cy + uy * su * half + ux * sv * wide))
            dr.polygon(corners, outline=(80, 200, 255))
            dr.line([cx - ux * half, cy - uy * half, cx + ux * half, cy + uy * half],
                    fill=(190, 235, 255), width=3)
            dr.text((cx + 6, cy - 16), str(i), font=f_small, fill=(255, 255, 255),
                    stroke_width=3, stroke_fill=(0, 36, 72))
    labels = PREVIEW['labels']
    for o in objs:
        if 'Waypoint' not in o['type_name']:
            continue
        d2 = props_dict(o['props'])
        nm = d2.get('waypointName')
        if nm not in labels:
            continue
        txt, team = labels[nm]
        cx, cy = obj_xy(o)
        col = (90, 160, 255) if team in ('W', 'N') else (255, 120, 90)
        dr.ellipse([cx - 13, cy - 13, cx + 13, cy + 13], outline=(255, 255, 255), width=4)
        dr.ellipse([cx - 11, cy - 11, cx + 11, cy + 11], outline=col, width=4)
        w = dr.textlength(txt, font=f_base)
        if cx + 26 + w > im.size[0] - 8:      # flip label to the left near the right edge
            dr.rectangle([cx - 26 - w, cy - 20, cx - 16, cy + 14], fill=(0, 0, 0, 170))
            dr.text((cx - 21 - w, cy - 18), txt, font=f_base, fill=(255, 255, 255))
        else:
            dr.rectangle([cx + 16, cy - 20, cx + 26 + w, cy + 14], fill=(0, 0, 0, 170))
            dr.text((cx + 21, cy - 18), txt, font=f_base, fill=(255, 255, 255))

    # legend + title
    W, H = im.size
    band = 150
    out = Image.new('RGB', (W, H + band), (16, 18, 22))
    out.paste(im, (0, band))
    dr = ImageDraw.Draw(out)
    dr.text((24, 18), PREVIEW['title'], font=f_title, fill=(255, 226, 120))
    dr.text((24, 80), PREVIEW['subtitle'], font=f_base, fill=(210, 220, 235))
    # ⚠ 朝向标注：本预览与**游戏内实际显示一致**（T1.8.8 实机确认）——
    #   北在屏幕上方、海（北部湾，南）在下方，即常规地图朝向。
    _w = dr.textlength(PREVIEW['subtitle'], font=f_base)
    dr.text((30 + _w, 80), '（本图与游戏内一致：北朝上、海在下方）',
            font=f_base, fill=(255, 170, 90))
    ly = H + 18
    items = list(PREVIEW.get('legend') or ()) or [
        ((255, 214, 0), '矿脉 Ore'), ((255, 92, 0), '油井'), ((0, 235, 235), '观察哨'),
        ((235, 0, 235), '车库'), ((240, 40, 40), '固定AI基地(苏)'), ((60, 90, 255), '固定AI基地(盟)'),
        ((32, 90, 160), '海面/河流'), ((80, 200, 255), '斜坡(编号)'),
        ((120, 96, 74), '不可通行(陡崖/山地)')]
    x = 24
    for c, t in items:
        dr.rectangle([x, ly + 4, x + 22, ly + 26], fill=c)
        dr.text((x + 30, ly + 3), t, font=f_leg, fill=(225, 230, 240))
        x += 40 + dr.textlength(t, font=f_leg) + 34
        if x > W - 260:
            x = 24; ly += 34
    if PREVIEW['footer']:
        dr.text((24, ly + 40), PREVIEW['footer'], font=f_leg, fill=(180, 195, 215))
    out.save(os.path.join(MAPDIR, 'preview.png'))
    print('saved', os.path.join(MAPDIR, 'preview.png'), out.size)


if __name__ == '__main__':
    main()
