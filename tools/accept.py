# -*- coding: utf-8 -*-
"""当前版本的构建验收（矿规则 / 矿场可建造 / 标注斜坡 / 轴向）。

默认检查配方当前版本的 assets 目录。单独查历史包时：

    TAG=T1.9.4 python tools/accept.py
    python tools/accept.py --only ore --only axis

作为 build_all 的一个阶段被调用时不解析命令行（避免吃掉 --only 等参数）。
任一硬规则不通过则以退出码 1 结束。
"""
import argparse
import json
import math
import os
import struct
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import project as proj
import terrain_gen as bt
from ra3map import MapFile, parse_heightmap

CHECKS = ('ore', 'buildable', 'ramps', 'axis')
RAMP_ALONG_MAX = 6.0
# 自动斜坡边长 20~40，70 的层差在这个长度上峰值大约 4，仍远低于不可通行的 24
RAMP_ALONG_SLACK = 0.05
RAMP_END_GAP_MAX = 12.0
RAMP_TRANS_MAX = 10.0


def _bundle(historical_tag=None):
    P = proj.current()
    tag = historical_tag or ''
    if tag and tag != P.version_string:
        name = P.base_name + tag
        base = os.path.join(P.assets_group, name)
    else:
        name = P.map_name
        base = P.assets
    if not os.path.isdir(base):
        raise SystemExit('找不到产物目录：%s' % base)
    src = os.path.join(base, 'source')
    npz = os.path.join(src, 'terrain.npz')
    mappath = os.path.join(base, name + '.map')
    if not os.path.isfile(npz):
        raise SystemExit('缺少 %s' % npz)
    if not os.path.isfile(mappath):
        raise SystemExit('缺少 %s' % mappath)
    return dict(P=P, name=name, base=base, src=src, npz=npz, mappath=mappath)


def _terrain(bundle):
    d = np.load(bundle['npz'])
    z = d['raw'].astype(np.float64) * bt.Z_SCALE
    ore = [tuple(map(float, c)) for c in d['ore_cells']]
    ang = [float(a) for a in d['ore_angles']]
    bases = [tuple(map(float, c)) for c in d['base_cells']]
    names = list(str(d['base_names']).split('|'))
    # 产物里的名字是生成时写入的。条数一致就按当前配方的名称显示，格子顺序不变。
    if len(bt.BASES) == len(bases):
        names = [b[2] for b in bt.BASES]
    gy, gx = np.gradient(z)
    slope = np.sqrt(gx ** 2 + gy ** 2)
    return d, z, ore, ang, bases, names, slope, gx, gy


def _impassable(bundle, shape):
    """BlendTileData 的不可通行位图，翻回内部帧（行 0 = 北）。"""
    h, w = shape
    m = MapFile.load(bundle['mappath'])
    data = m.get('BlendTileData').payload
    n = struct.unpack('<I', data[:4])[0]
    off = 4 + n * 2 * 4
    nbytes = (h * w + 7) // 8
    imp = np.unpackbits(np.frombuffer(data[off:off + nbytes], dtype=np.uint8),
                        count=h * w, bitorder='little').reshape(h, w).astype(bool)
    return np.flipud(imp), m


def check_ore(bundle):
    """近矿配额、中路矿、落点平整、矿口开阔。返回失败说明。"""
    d, z, ore, ang, bases, names, slope, _gx, _gy = _terrain(bundle)
    grid = bt.GRID
    water = z < bt.WATER_Z
    gy, gx = np.gradient(z)
    slope = np.sqrt(gx ** 2 + gy ** 2)
    levels = list(bt.layer_height.play_levels(bt.T))
    if not bt.layer_height.enabled(bt.T):
        levels = levels + [480.0]
    lv = np.full(z.shape, -1, np.int8)
    for i, v in enumerate(levels):
        lv[np.abs(z - v) < 1.5] = i
    bnd = np.zeros(z.shape, bool)
    shore = np.zeros(z.shape, bool)
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            sh = np.roll(np.roll(lv, dy, 0), dx, 1)
            bnd |= (lv >= 0) & (sh >= 0) & (lv != sh)
            shw = np.roll(np.roll(water, dy, 0), dx, 1)
            shore |= water != shw
    d_edge_m, d_shore_m = bt.ore_open_maps(z, bt.T)
    layers = bt.land_layer_index(z, bt.T)
    ecl = float(bt.T.get('ore_edge_clear', 16.0))
    scl = float(bt.T.get('ore_shore_clear', 12.0))
    ramps = json.loads(str(d['ramp_json'])) if 'ramp_json' in d.files else []
    ramp = bt.ramp_cover_mask(z.shape, ramps)
    imp = ((slope > bt.SLOPE_IMPASSABLE)
           | (z > bt.WATER_Z + bt.PEAK_IMPASSABLE))
    fails = []

    print('【矿规则】近矿恰好 2 个且朝基地；陆上同层或范围内水域，水上只在水域；外围矿水陆各半；落点平整；矿口开阔')
    for i, (nm, c) in enumerate(zip(names, bases), 1):
        ds = sorted((math.hypot(c[0] - o[0], c[1] - o[1]), j) for j, o in enumerate(ore))
        near = [(dist, j) for dist, j in ds if dist <= 62.0]
        n_near = len(near)
        if n_near != 2:
            fails.append('R1: %s 近矿 %d 个（要求恰好 2）' % (nm, n_near))
        for dist, j in near:
            o, a = ore[j], ang[j]
            dot = math.cos(a) * (c[0] - o[0]) + math.sin(a) * (c[1] - o[1])
            # 只有配方写了 ore_beside 的图，被点名的近矿可以贴着基地，并按 ore_face_at 朝外。
            pin = face_pin = None
            if bt.T.get('ore_beside'):
                for item in bt.T.get('ore_place_at') or ():
                    if math.hypot(o[0] - float(item[2]), o[1] - float(item[3])) <= 3.0:
                        pin = item
                        break
                for item in bt.T.get('ore_face_at') or ():
                    if math.hypot(o[0] - float(item[0]), o[1] - float(item[1])) <= 3.0:
                        face_pin = item
                        break
            ok_rng = (24.0 <= dist <= 62.0) if pin is not None else (43.5 <= dist <= 62.0)
            if face_pin is not None:
                want = math.atan2(float(face_pin[3]), float(face_pin[2]))
                ok_face = math.cos(a - want) > 0.5
            else:
                ok_face = dot > 0
            ix, iy = int(round(o[0])), int(round(o[1]))
            de = float(d_edge_m[iy, ix])
            dsh = float(d_shore_m[iy, ix])
            ok_j = de >= ecl and dsh >= scl
            ok_s = bt.near_ore_surface_ok(z, c[0], c[1], o[0], o[1], bt.T, layers)
            bix, biy = int(round(c[0])), int(round(c[1]))
            base_wet = bool(water[biy, bix])
            if not (ok_rng and ok_face):
                fails.append('R1: %s 近矿 (%d,%d) 距离 %.1f 朝向=%s' % (
                    nm, o[0], o[1], dist, ok_face))
            if not ok_j:
                fails.append('R1: %s 近矿 (%d,%d) 离层界 %.0f（要≥%.0f）离岸 %.0f（要≥%.0f）' % (
                    nm, o[0], o[1], de, ecl, dsh, scl))
            if not ok_s:
                if base_wet:
                    fails.append('R1: %s 近矿 (%d,%d) 不在水域' % (nm, o[0], o[1]))
                else:
                    fails.append('R1: %s 近矿 (%d,%d) 不在基地同层陆地，也不在范围内水域'
                                 % (nm, o[0], o[1]))
            face_word = '朝外' if face_pin is not None else '朝基地'
            print('   P%d %-6s 近矿 (%4d,%4d) 距离 %5.1f %s  %s %s  离层界 %4.0f 离岸 %4.0f %s  落层 %s' % (
                i, nm, o[0], o[1], dist, '✓' if ok_rng else '✗', face_word,
                '✓' if ok_face else '✗',
                de, dsh, '✓' if ok_j else '✗', '✓' if ok_s else '✗'))

    near_r = 62.0
    outer = []
    for o in ore:
        if min(math.hypot(o[0] - c[0], o[1] - c[1]) for c in bases) > near_r:
            outer.append(o)
    n_players = len(bases)
    want_outer = int(bt.T.get('ore_outer_per_player', 4)) * n_players
    n_water = sum(1 for o in outer if bool(water[int(round(o[1])), int(round(o[0]))]))
    n_land = len(outer) - n_water
    print('   外围矿 %d 个（陆地 %d、水面 %d，目标 %d）' % (len(outer), n_land, n_water, want_outer))
    if len(outer) != want_outer:
        fails.append('R3: 外围矿 %d 个，要求每名玩家 4 个、共 %d' % (len(outer), want_outer))
    yy_i, xx_i = np.mgrid[0:grid, 0:grid]
    inside = np.minimum.reduce([xx_i - bt.BORDER, yy_i - bt.BORDER,
                                bt.GRID - 1 - bt.BORDER - xx_i,
                                bt.GRID - 1 - bt.BORDER - yy_i]) >= 0
    water_frac = float((water & inside).sum()) / max(1, int(inside.sum()))
    if water_frac >= 0.20 and outer:
        lo = min(n_land, n_water) / float(len(outer))
        print('   外围矿水陆比例 %.0f%% / %.0f%%（水域 %.0f%%）' % (
            100.0 * n_land / len(outer), 100.0 * n_water / len(outer), 100.0 * water_frac))
        if lo < 0.35:
            fails.append('R3: 外围矿陆地 %d、水面 %d，没有均匀分开' % (n_land, n_water))

    for key, label in (('ore_cells', '矿'), ('derrick_cells', '油井'), ('post_cells', '哨站'),
                       ('garage_cells', '车库'), ('airport_cells', '机场'),
                       ('hospital_cells', '医院'), ('academy_cells', '学院'),
                       ('shipyard_cells', '船厂')):
        if key not in d:
            continue
        for c in d[key]:
            x, y = int(round(c[0])), int(round(c[1]))
            sl = float(slope[y, x])
            if sl >= 2.5 and not bool(water[y, x]):
                fails.append('R4a: %s (%d,%d) 坡度 %.1f 且不在水域' % (label, x, y, sl))
                print('   ✗ %s (%d,%d) 坡度 %.1f' % (label, x, y, sl))

    for key, label in (('derrick_cells', '油井'), ('post_cells', '哨站'),
                       ('garage_cells', '车库'), ('airport_cells', '机场'),
                       ('hospital_cells', '医院'), ('academy_cells', '学院'),
                       ('house_cells', '房子'), ('shipyard_cells', '船厂')):
        if key not in d.files or d[key].size == 0:
            continue
        for c in np.atleast_2d(d[key]):
            x, y = int(round(float(c[0]))), int(round(float(c[1])))
            if not (0 <= x < grid and 0 <= y < grid):
                why = '图外'
            elif bnd[y, x]:
                why = '层界'
            elif shore[y, x]:
                why = '岸线'
            elif ramp[y, x]:
                why = '斜坡'
            elif imp[y, x]:
                why = '不可通行'
            else:
                why = None
            if why:
                fails.append('R4c: %s (%d,%d) 落在%s' % (label, x, y, why))
                print('   ✗ %s (%d,%d) 落在%s' % (label, x, y, why))

    bad_mouth = 0
    for o, a in zip(ore, ang):
        ox, oy = o
        bi = min(range(len(bases)), key=lambda k: (bases[k][0] - ox) ** 2 + (bases[k][1] - oy) ** 2)
        bd = math.hypot(ox - bases[bi][0], oy - bases[bi][1])
        ore_wet = bool(water[int(round(oy)), int(round(ox))])
        if ore_wet and bd <= near_r:
            continue
        dx, dy = math.cos(a), math.sin(a)
        hit = None
        mouth_r = int(bt.T.get('ore_mouth_r1', 20))
        for r in range(8, mouth_r + 11):
            x, y = int(round(ox + dx * r)), int(round(oy + dy * r))
            if not (0 <= x < grid and 0 <= y < grid):
                hit = ('图边', r)
                break
            dcell = min(x - bt.BORDER, y - bt.BORDER,
                        bt.GRID - 1 - bt.BORDER - x, bt.GRID - 1 - bt.BORDER - y)
            if dcell < 3:
                hit = ('图边', r)
                break
            if bnd[y, x]:
                hit = ('层界', r)
                break
            if shore[y, x]:
                hit = ('岸线', r)
                break
            if ramp[y, x]:
                hit = ('斜坡', r)
                break
            if imp[y, x]:
                hit = ('不可通行', r)
                break
        if hit and hit[1] < mouth_r:
            bad_mouth += 1
            fails.append('R4b: 矿 (%d,%d) 朝向前方 %d 格撞上%s' % (o[0], o[1], hit[1], hit[0]))

    def _pts(key):
        if key not in d.files or d[key].size == 0:
            return []
        return [(float(c[0]), float(c[1])) for c in np.atleast_2d(d[key])]

    posts_c = _pts('post_cells')
    academies_c = _pts('academy_cells')
    garages_c = _pts('garage_cells')
    hospitals_c = _pts('hospital_cells')
    airports_c = _pts('airport_cells')
    yards_c = _pts('shipyard_cells')
    oils = _pts('derrick_cells')
    print('   哨站 %d、学院 %d、车库 %d、医院 %d、机场 %d、船厂 %d' % (
        len(posts_c), len(academies_c), len(garages_c), len(hospitals_c),
        len(airports_c), len(yards_c)))
    if len(posts_c) != 2:
        fails.append('R5: 哨站 %d 个，要求 2' % len(posts_c))
    if len(academies_c) != 2:
        fails.append('R5: 精兵学院 %d 个，要求 2' % len(academies_c))
    for label, cells in (('车库', garages_c), ('医院', hospitals_c), ('机场', airports_c)):
        if not (1 <= len(cells) <= 2):
            fails.append('R5: %s %d 个，要求 1~2' % (label, len(cells)))
        for c in cells:
            if bool(water[int(round(c[1])), int(round(c[0]))]):
                fails.append('R5: %s (%.0f,%.0f) 不在陆地' % (label, c[0], c[1]))
    if water_frac >= 0.20 and len(yards_c) != 1:
        fails.append('R5: 水域 %.0f%%，船厂 %d 个，要求 1 个' % (100.0 * water_frac, len(yards_c)))
    if water_frac < 0.20 and yards_c:
        fails.append('R5: 水域 %.0f%%，不放船厂' % (100.0 * water_frac,))
    for c in yards_c:
        if not bool(water[int(round(c[1])), int(round(c[0]))]):
            fails.append('R5: 船厂 (%.0f,%.0f) 不在水面' % (c[0], c[1]))
    buildings = posts_c + academies_c + garages_c + hospitals_c + airports_c + yards_c
    gap = float(bt.T.get('building_gap', 36.0))
    for i in range(len(buildings)):
        for j in range(i + 1, len(buildings)):
            dist = math.hypot(buildings[i][0] - buildings[j][0], buildings[i][1] - buildings[j][1])
            if dist < gap - 0.5:
                fails.append('R5: 两座建筑相距 %.0f 格' % dist)
    for c in bases:
        if not any(math.hypot(o[0] - c[0], o[1] - c[1]) <= 140.0 for o in oils):
            fails.append('R5: 基地 (%.0f,%.0f) 附近没有油井' % (c[0], c[1]))

    print('   矿口前方 <%d 格撞障碍：%d' % (int(bt.T.get('ore_mouth_r1', 20)), bad_mouth))
    print('   矿规则 %s' % ('通过' if not fails else '失败 %d 项' % len(fails)))
    return fails


def check_buildable(bundle):
    """矿场地基平整、不可通行豁免打在最终矿位、矿心离开可玩区边。"""
    _d, z, ore, ang, bases, names, _slope, _gx, _gy = _terrain(bundle)
    h, w = z.shape
    imp, _m = _impassable(bundle, (h, w))
    yy, xx = np.mgrid[0:h, 0:w]
    mouth_r = int(bt.T.get('ore_mouth_r1', 20))
    build_r = max(float(getattr(bt, 'ORE_CLEAR_RADIUS', 20.0)), float(mouth_r))
    border_clear = float(bt.T.get('ore_border_clear', 30.0))
    pr = int(math.ceil(build_r))
    oy, ox = np.mgrid[-pr:pr + 1, -pr:pr + 1]
    od = np.sqrt(ox ** 2 + oy ** 2)
    disc = od <= build_r
    fails = []
    print('【矿场可建造】绕矿点整圆盘 r%.0f 高差≤12、圆盘内不可通行为 0、离边≥%.0f' % (
        build_r, border_clear))
    for o, _a in zip(ore, ang):
        ix, iy = int(round(o[0])), int(round(o[1]))
        on_water = z[iy, ix] < bt.WATER_Z
        if on_water:
            zrange = 0.0
            ok = True
        elif ix < pr or iy < pr or ix >= w - pr or iy >= h - pr:
            zrange = 99.0
            ok = False
        else:
            win = z[iy - pr:iy + pr + 1, ix - pr:ix + pr + 1]
            zrange = float(win[disc].max() - win[disc].min())
            ok = zrange <= 12.0
        d2 = (xx - o[0]) ** 2 + (yy - o[1]) ** 2
        fi = float(imp[d2 <= build_r * build_r].mean())
        d_bor = min(o[0] - bt.BORDER, o[1] - bt.BORDER,
                    bt.GRID - 1 - bt.BORDER - o[0], bt.GRID - 1 - bt.BORDER - o[1])
        if d_bor < border_clear or not ok or fi > 0.0:
            why = '离图边 %.0f 格' % d_bor if d_bor < border_clear else '地基或不可通行'
            fails.append('矿 (%d,%d) 摆不下（%s）' % (ix, iy, why))
            print('   ✗ (%d,%d) 高差 %.1f 不可通行 %.1f%%  %s' % (
                ix, iy, zrange, 100 * fi, why))
    print('   摆不下的矿：%d / %d' % (len(fails), len(ore)))
    return fails


def check_ramps(bundle):
    """自动斜坡：沿坡单格落差、端缝、横向落差，车道要有土路纹理。"""
    _d, z, _ore, _ang, _bases, _names, slope, gx, gy = _terrain(bundle)
    h, w = z.shape
    if 'ramp_json' not in _d.files:
        return ['terrain.npz 没有自动斜坡']
    ramps = json.loads(str(_d['ramp_json']))
    if not ramps:
        return ['没有自动斜坡']
    _imp, m = _impassable(bundle, (h, w))
    payload = m.get('BlendTileData').payload
    n = struct.unpack('<I', payload[:4])[0]
    tiles = np.flipud(np.frombuffer(payload[4:4 + n * 2], dtype='<u2').reshape(h, w))
    blk_tex = tiles // 64
    tex_names = [e[0] for e in proj.current().cfg.TEX_SET]
    fails = []
    print('【斜坡】%d 条；沿坡≤%.1f 端缝≤%.0f 横向≤%.0f 单调，车道含 Dirt_Yucatan06' % (
        len(ramps), RAMP_ALONG_MAX, RAMP_END_GAP_MAX, RAMP_TRANS_MAX))
    for k, r in enumerate(ramps, 1):
        cx, cy = float(r['x']), float(r['y'])
        hw, hh = r['w'] / 2.0, r['h'] / 2.0
        lv = r.get('pair') or r['layers']
        ux, uy, _lo, _hi = bt.ramp_uphill(z, r)
        if r.get('auto'):
            half_u = max(10.0, min(20.0, float(r.get('span', r['w'])) * 0.5))
            half_v = max(4.0, min(12.0, float(r.get('width', r['h'])) * 0.35))
            end_pad = 4
        else:
            half_u = max(6.0, min(hw, hh) * 0.5)
            half_v = max(4.0, min(hw, hh) * 0.35)
            end_pad = 6
        ss = np.arange(-half_u - end_pad, half_u + end_pad + 1, dtype=np.float64)
        prof = bt._bilinear(z, cx + ux * ss, cy + uy * ss)
        along_max = float(np.abs(np.diff(prof)).max())
        end_gap = max(abs(float(prof[0] - prof[end_pad])),
                      abs(float(prof[-1] - prof[-1 - end_pad])))
        # 只量坡面内部。车道外再探 4 格会探出边长 25 的坡，把旁边的硬坎算成横向落差。
        half_box = max(4.0, 0.5 * min(float(r.get('w', 28)), float(r.get('h', 28))) - 1.0)
        tt = np.arange(-half_box, half_box + 1.0, dtype=np.float64)
        prof_t = bt._bilinear(z, cx - uy * tt, cy + ux * tt)
        trans_max = float(np.abs(np.diff(prof_t)).max())
        mono = int((np.diff(prof) < -1.0).sum())
        texs = set()
        for s2 in np.arange(-half_u, half_u + 1, 8):
            sx2 = int(round(cx + ux * s2))
            sy2 = int(round(cy + uy * s2))
            if 0 <= sy2 < h and 0 <= sx2 < w:
                texs.add(tex_names[int(blk_tex[sy2, sx2])])
        bad = []
        if along_max > RAMP_ALONG_MAX + RAMP_ALONG_SLACK:
            bad.append('沿坡 %.2f' % along_max)
        if end_gap > RAMP_END_GAP_MAX:
            bad.append('端缝 %.1f' % end_gap)
        if trans_max > RAMP_TRANS_MAX:
            bad.append('横向 %.1f' % trans_max)
        if mono:
            bad.append('单调违例 %d' % mono)
        if 'Dirt_Yucatan06' not in texs:
            bad.append('无土路纹理')
        mark = '✓' if not bad else '✗ ' + ','.join(bad)
        print('   #%02d (%4.0f,%4.0f) 沿坡 %4.1f 端缝 %4.1f 横向 %4.1f  %s' % (
            k, cx, cy, along_max, end_gap, trans_max, mark))
        if bad:
            fails.append('斜坡 #%d (%d,%d) %s' % (k, cx, cy, ','.join(bad)))
    print('   斜坡 %s' % ('通过' if not fails else '失败 %d 条' % len(fails)))
    return fails


def _sea_frac(rgb):
    sea = ((rgb[..., 2] > rgb[..., 0] + 30) & (rgb[..., 2] > rgb[..., 1] + 15)
           & (rgb[..., 2] > 90))
    mid = sea.shape[0] // 2
    return 100 * sea[:mid].mean(), 100 * sea[mid:].mean()


def check_axis(bundle):
    """高度场游戏帧海在上半；显示图北朝上、海在下半。"""
    from PIL import Image

    m = MapFile.load(bundle['mappath'])
    hm = parse_heightmap(m.get('HeightMapData'))
    z = np.array(hm['elevations'], dtype=np.uint16).reshape(hm['height'], hm['width'])
    z = z * bt.Z_SCALE
    mid = z.shape[0] // 2
    up = 100 * (z[:mid] < bt.WATER_Z).mean()
    dn = 100 * (z[mid:] < bt.WATER_Z).mean()
    fails = []
    inland = bool(getattr(bt.cfg, 'INLAND', False))
    print('【轴向】高度场（游戏帧）上半海 %.1f%% 下半 %.1f%%' % (up, dn))
    if inland and up < 0.05 and dn < 0.05:
        print('   内陆图，没有海可对朝向')
    elif not (up > dn):
        fails.append('高度场海不在上半（游戏帧应为海在屏幕下方）')
    images = [
        (os.path.join(bundle['base'], bundle['name'] + '_art.tga'), 'art.tga', 0.0),
        (os.path.join(bundle['src'], 'art_preview.png'), 'art_preview.png', 0.0),
        (os.path.join(bundle['src'], 'terrain_preview.png'), 'terrain_preview.png', 0.0),
        (os.path.join(bundle['src'], 'build_view.png'), 'build_view.png', 0.0),
        (os.path.join(bundle['src'], 'passability_preview.png'), 'passability_preview.png', 0.0),
        (os.path.join(bundle['base'], 'preview.png'), 'preview.png', 0.07),
    ]
    for path, tag, crop in images:
        if not os.path.isfile(path):
            fails.append('缺少 %s' % tag)
            print('   %-24s 缺失' % tag)
            continue
        a = np.asarray(Image.open(path).convert('RGB')).astype(int)
        if crop:
            a = a[int(a.shape[0] * crop):]
        u, d = _sea_frac(a)
        ok = d > u or (inland and u < 0.5 and d < 0.5)
        print('   %-24s 上半 %.1f%%  下半 %.1f%%  %s' % (tag, u, d, '北朝上' if ok else '反向'))
        if not ok:
            fails.append('%s 海不在下半' % tag)
    print('   轴向 %s' % ('通过' if not fails else '失败'))
    return fails


def main(only=None, historical_tag=None):
    """跑指定验收。only 为空则四项都跑。返回进程退出码（0 通过）。"""
    chosen = [c for c in (only or CHECKS) if c in CHECKS]
    if only and not chosen:
        raise SystemExit('未知验收项：%s（可选 %s）' % (','.join(only), ','.join(CHECKS)))
    bundle = _bundle(historical_tag)
    print('验收 %s' % bundle['name'])
    runners = {'ore': check_ore, 'buildable': check_buildable,
               'ramps': check_ramps, 'axis': check_axis}
    fails = []
    for name in chosen:
        print()
        fails.extend(runners[name](bundle))
    print()
    if fails:
        print('验收未通过，%d 项：' % len(fails))
        for item in fails:
            print('  -', item)
        return 1
    print('验收通过')
    return 0


if __name__ == '__main__':
    ap = argparse.ArgumentParser(description='验收当前（或 TAG= 指定的）地图产物')
    ap.add_argument('--map', dest='map_id', default=None)
    ap.add_argument('--only', action='append', default=[], choices=CHECKS)
    args, _unknown = ap.parse_known_args()
    if args.map_id:
        os.environ['RA3MAP_ID'] = args.map_id
        proj.current(args.map_id, refresh=True)
    sys.exit(main(only=args.only or None, historical_tag=os.environ.get('TAG')))
