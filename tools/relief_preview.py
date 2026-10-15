# -*- coding: utf-8 -*-
"""按配方取景框抓真实高程，切成游戏高度层，写出北朝上的预览。不放资源。

步骤对每张图相同。缺省阈值写在 DEFAULT_PREVIEW，来自广东预览的调整。
单张图用 config.PREVIEW 覆盖缺的项；要留一块宽水域凸起时写 config.COAST_KEEP。

只留三张图，不写中间版本：

    source.png          真实高程地形
    preview.png         游戏高度层
    preview--line.png   同上，画出省界

    python -X utf8 tools/relief_preview.py --map <id>
"""
import argparse
import io
import os
import sys
import time
import urllib.request

import numpy as np
from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)
import project as proj

UA = {'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)'}

# 连片和海岸的缺省。数字来自广东预览的调整，别的图不写 PREVIEW 时沿用。
DEFAULT_PREVIEW = {
    'min_width': 30,
    'min_area': 50,
    'adjacent': 80,
    'coast_radius': 32.0,
    'coast_area': 40,
    'land_neck': 20.0,
    'water_neck': 8.0,
    'soften_amp': 14.0,
    'offshore_m': 5.0,
    'deep_m': -30.0,
    'boundary_label': '边界',
}
# 配方没写 LAYER_M 时，预览用这组海拔分层。不写回地图。
DEFAULT_LAYER_M = (
    (200.0, 212.0),
    (500.0, 280.0),
    (1500.0, 350.0),
    (4000.0, 420.0),
)
LAYER_COLORS = (
    (118, 168, 78),
    (168, 164, 70),
    (176, 132, 68),
    (146, 96, 62),
    (120, 84, 56),
)
LAYER_NAMES = ('一层', '二层', '三层', '四层', '五层', '六层', '七层', '八层')
PEAK_COLOR = (108, 104, 100)


def preview_recipe(cfg):
    """缺省配方盖上这张图自己的 PREVIEW。"""
    recipe = dict(DEFAULT_PREVIEW)
    recipe.update(getattr(cfg, 'PREVIEW', None) or {})
    return recipe


def layer_bands(cfg):
    bands = tuple(getattr(cfg, 'LAYER_M', ()) or ())
    return bands or DEFAULT_LAYER_M


def _land_z(cfg):
    return float(getattr(cfg, 'LAND_Z', 210.0))


def _first_land_level(cfg):
    layers = layer_bands(cfg)
    if layers:
        return float(layers[0][1])
    return _land_z(cfg)


def level_style(cfg):
    """图例颜色和名字跟配方层高走，不绑某一张图的高度数。"""
    sea = float(getattr(cfg, 'SEA_Z', 190.0))
    water = float(cfg.WATER_Z)
    peak = float(getattr(cfg, 'PEAK_Z', 480.0))
    colors = {sea: (18, 58, 112), water: (36, 108, 176)}
    labels = [(sea, '深海'), (water, '海')]
    for i, (_limit, level) in enumerate(layer_bands(cfg)):
        level = float(level)
        colors[level] = LAYER_COLORS[min(i, len(LAYER_COLORS) - 1)]
        labels.append((level, LAYER_NAMES[min(i, len(LAYER_NAMES) - 1)]))
    if all(abs(peak - level) > 0.5 for level, _name in labels):
        i = len(labels) - 2
        colors[peak] = PEAK_COLOR
        labels.append((peak, LAYER_NAMES[min(max(i, 0), len(LAYER_NAMES) - 1)]))
    return colors, labels


def fetch(url, tries=5):
    last = None
    for i in range(tries):
        try:
            return urllib.request.urlopen(
                urllib.request.Request(url, headers=UA), timeout=40).read()
        except Exception as e:
            last = e
            time.sleep(1.5 * (i + 1))
    raise RuntimeError('%s -> %s' % (url, last))


def lonlat_to_px(lon, lat, z):
    n = 256.0 * (2.0 ** z)
    lon = np.asarray(lon, dtype=np.float64)
    lat = np.asarray(lat, dtype=np.float64)
    x = (lon + 180.0) / 360.0 * n
    la = np.radians(lat)
    y = (1.0 - np.log(np.tan(la) + 1.0 / np.cos(la)) / np.pi) / 2.0 * n
    return x, y


PREVIEW_MIN = 600
PREVIEW_MAX = 800
PREVIEW_PAD = 24


def _preview_hw(cfg):
    w = int(getattr(cfg, 'PREVIEW_W', 0) or 0)
    h = int(getattr(cfg, 'PREVIEW_H', 0) or 0)
    if w > 0 and h > 0:
        return h, w
    return cfg.GRID, cfg.GRID


def land_bounds(P, cfg):
    """强度为 1 的陆地外接经纬度。"""
    import json
    lons, lats = [], []
    for fname, _prov, amp in cfg.GEOJSONS:
        if float(amp) < 1.0:
            continue
        gj = json.load(open(P.data_file(fname), encoding='utf-8'))
        for feat in gj['features']:
            g = feat['geometry']
            polys = g['coordinates'] if g['type'] == 'MultiPolygon' else [g['coordinates']]
            for poly in polys:
                for c in poly[0]:
                    lons.append(c[0])
                    lats.append(c[1])
    if not lons:
        return cfg.LON0, cfg.LON1, cfg.LAT0, cfg.LAT1
    return min(lons), max(lons), min(lats), max(lats)


def apply_land_view(P, cfg, pad=PREVIEW_PAD):
    """预览画幅包住陆地，宽高各自落在 600–800。只改本次内存里的取景，不写配方。"""
    lon0, lon1, lat0, lat1 = land_bounds(P, cfg)
    vx = max(lon1 - lon0, 1e-6)
    vy = max((lat1 - lat0) * float(cfg.V_STRETCH), 1e-6)
    scale = (PREVIEW_MAX - 2 * pad) / max(vx, vy)
    lw = scale * vx
    lh = scale * vy
    w = int(np.clip(int(np.rint(lw + 2 * pad)), PREVIEW_MIN, PREVIEW_MAX))
    h = int(np.clip(int(np.rint(lh + 2 * pad)), PREVIEW_MIN, PREVIEW_MAX))
    cfg.LON0, cfg.LON1, cfg.LAT0, cfg.LAT1 = lon0, lon1, lat0, lat1
    cfg.PREVIEW_W = w
    cfg.PREVIEW_H = h
    cfg.PREVIEW_PAD = pad
    print('预览画幅 %d×%d  陆地 lon %.2f..%.2f  lat %.2f..%.2f' % (
        w, h, lon0, lon1, lat0, lat1))
    return w, h


def frame(cfg):
    w = int(getattr(cfg, 'PREVIEW_W', 0) or 0)
    h = int(getattr(cfg, 'PREVIEW_H', 0) or 0)
    if w > 0 and h > 0:
        pad = float(getattr(cfg, 'PREVIEW_PAD', PREVIEW_PAD))
        lon_span = cfg.LON1 - cfg.LON0
        lat_span = cfg.LAT1 - cfg.LAT0
        vx = lon_span
        vy = lat_span * float(cfg.V_STRETCH)
        scale = (PREVIEW_MAX - 2 * pad) / max(vx, vy)
        lw = scale * vx
        lh = scale * vy
        ox = (w - lw) / 2.0
        oy = (h - lh) / 2.0
        return ox, oy, lw / lon_span, lh / lat_span
    play = cfg.GRID - 2 * cfg.BORDER
    sx = play / (cfg.LON1 - cfg.LON0)
    sy = sx * cfg.V_STRETCH
    lat_bias = float((getattr(cfg, 'TERRAIN', {}) or {}).get('lat_bias', 0.5))
    prov_h = (cfg.LAT1 - cfg.LAT0) * sy
    oy = cfg.BORDER + (play - prov_h) * lat_bias
    ox = float(cfg.BORDER)
    return ox, oy, sx, sy


def to_cell(lon, lat, cfg, fr):
    ox, oy, sx, sy = fr
    x = ox + (lon - cfg.LON0) * sx
    y = oy + (cfg.LAT1 - lat) * sy
    return x, y


def land_mask(P, cfg, fr):
    import json
    h, w = _preview_hw(cfg)
    img = Image.new('L', (w, h), 0)
    draw = ImageDraw.Draw(img)
    for fname, _prov, amp in cfg.GEOJSONS:
        if float(amp) < 1.0:
            continue
        gj = json.load(open(P.data_file(fname), encoding='utf-8'))
        for feat in gj['features']:
            g = feat['geometry']
            polys = g['coordinates'] if g['type'] == 'MultiPolygon' else [g['coordinates']]
            for poly in polys:
                pts = [to_cell(c[0], c[1], cfg, fr) for c in poly[0]]
                if len(pts) >= 3:
                    draw.polygon(pts, fill=255)
    return np.array(img) > 0


def fetch_dem(cfg, fr, zoom):
    ox, oy, sx, sy = fr

    def cell_to_lonlat(cx, cy):
        lon = cfg.LON0 + (cx - ox) / sx
        lat = cfg.LAT1 - (cy - oy) / sy
        return lon, lat

    h, w = _preview_hw(cfg)
    lon_a, lat_n = cell_to_lonlat(0, 0)
    lon_b, lat_s = cell_to_lonlat(w - 1, h - 1)
    lon0, lon1 = min(lon_a, lon_b), max(lon_a, lon_b)
    lat0, lat1 = min(lat_s, lat_n), max(lat_s, lat_n)
    px0, py1 = lonlat_to_px(lon0, lat0, zoom)
    px1, py0 = lonlat_to_px(lon1, lat1, zoom)
    tx0, tx1 = int(np.floor(px0 / 256)), int(np.floor(px1 / 256))
    ty0, ty1 = int(np.floor(py0 / 256)), int(np.floor(py1 / 256))
    print('DEM z=%d  x %d..%d  y %d..%d（%d 张）' % (
        zoom, tx0, tx1, ty0, ty1, (tx1 - tx0 + 1) * (ty1 - ty0 + 1)))
    W = (tx1 - tx0 + 1) * 256
    H = (ty1 - ty0 + 1) * 256
    dem = np.zeros((H, W), dtype=np.float32)
    for ty in range(ty0, ty1 + 1):
        for tx in range(tx0, tx1 + 1):
            raw = fetch('https://s3.amazonaws.com/elevation-tiles-prod/terrarium/%d/%d/%d.png'
                        % (zoom, tx, ty))
            im = np.array(Image.open(io.BytesIO(raw)).convert('RGB'), dtype=np.float32)
            e = (im[:, :, 0] * 256.0 + im[:, :, 1] + im[:, :, 2] / 256.0) - 32768.0
            dem[(ty - ty0) * 256:(ty - ty0 + 1) * 256,
                (tx - tx0) * 256:(tx - tx0 + 1) * 256] = e
        print('  行 %d/%d' % (ty - ty0 + 1, ty1 - ty0 + 1))
    yy, xx = np.mgrid[0:h, 0:w]
    lon = cfg.LON0 + (xx - ox) / sx
    lat = cfg.LAT1 - (yy - oy) / sy
    pxx, pyy = lonlat_to_px(lon, lat, zoom)
    ix = np.clip((pxx - tx0 * 256).astype(np.int32), 0, W - 1)
    iy = np.clip((pyy - ty0 * 256).astype(np.int32), 0, H - 1)
    return dem[iy, ix]


def quantize(dem, land, cfg):
    sea = float(getattr(cfg, 'SEA_Z', 190.0))
    water = float(cfg.WATER_Z)
    peak = float(getattr(cfg, 'PEAK_Z', 480.0))
    recipe = preview_recipe(cfg)
    z = np.full(dem.shape, peak, dtype=np.float32)
    for limit, level in reversed(layer_bands(cfg)):
        z[dem < float(limit)] = level
    # 0 米起算进一层。海面以下，以及边界外的近海，仍是海。
    ocean = (dem < 0.0) | ((~land) & (dem < float(recipe['offshore_m'])))
    z[ocean] = water
    z[ocean & (dem < float(recipe['deep_m']))] = sea
    return z


def shade(rgb, dem):
    gy, gx = np.gradient(dem)
    light = np.clip(0.72 + 0.28 * np.tanh((-gx - gy) / 40.0), 0.45, 1.15)
    out = np.clip(rgb.astype(np.float32) * light[..., None], 0, 255).astype(np.uint8)
    return out


def _chamfer_from(sources):
    """到 sources 为真的格子的曼哈顿距离。"""
    h, w = sources.shape
    inf = 1e6
    row = np.where(sources, 0.0, inf)
    xi = np.arange(w, dtype=np.float64)
    left = np.minimum.accumulate(row - xi, axis=1) + xi
    right = np.minimum.accumulate((row + xi)[:, ::-1], axis=1)[:, ::-1] - xi
    horiz = np.minimum(left, right)
    yi = np.arange(h, dtype=np.float64)[:, None]
    up = np.minimum.accumulate(horiz - yi, axis=0) + yi
    down = np.minimum.accumulate((horiz + yi)[::-1], axis=0)[::-1] - yi
    return np.minimum(up, down)


def _label_components(z):
    from collections import deque
    h, w = z.shape
    lab = np.zeros((h, w), dtype=np.int32)
    areas = [0]
    levels = [0.0]
    n = 0
    ys, xs = np.nonzero(np.ones((h, w), dtype=bool))
    for y0, x0 in zip(ys.tolist(), xs.tolist()):
        if lab[y0, x0]:
            continue
        n += 1
        v = float(z[y0, x0])
        q = deque(((y0, x0),))
        lab[y0, x0] = n
        cnt = 0
        while q:
            y, x = q.popleft()
            cnt += 1
            for ny, nx in ((y - 1, x), (y + 1, x), (y, x - 1), (y, x + 1)):
                if 0 <= ny < h and 0 <= nx < w and lab[ny, nx] == 0 and z[ny, nx] == v:
                    lab[ny, nx] = n
                    q.append((ny, nx))
        areas.append(cnt)
        levels.append(v)
    return lab, areas, levels


def _component_width(lab):
    """每块最窄处的宽度（内接直径）。贴图边也算边界。"""
    h, w = lab.shape
    edge = np.zeros((h, w), dtype=bool)
    edge[0, :] = edge[-1, :] = edge[:, 0] = edge[:, -1] = True
    edge[1:, :] |= lab[1:, :] != lab[:-1, :]
    edge[:-1, :] |= lab[1:, :] != lab[:-1, :]
    edge[:, 1:] |= lab[:, 1:] != lab[:, :-1]
    edge[:, :-1] |= lab[:, 1:] != lab[:, :-1]
    dist = _chamfer_from(edge)
    max_r = np.zeros(int(lab.max()) + 1, dtype=np.float64)
    np.maximum.at(max_r, lab.ravel(), dist.ravel())
    return max_r * 2.0


def _contact_level(z, lab, ids):
    """这些块与相邻层的接触格数。返回 best_level[块号]。"""
    want = np.zeros(int(lab.max()) + 1, dtype=bool)
    want[ids] = True
    best_n = {}
    best_lv = {}
    pairs = (
        (lab[:, :-1], lab[:, 1:], z[:, 1:]),
        (lab[:, 1:], lab[:, :-1], z[:, :-1]),
        (lab[:-1, :], lab[1:, :], z[1:, :]),
        (lab[1:, :], lab[:-1, :], z[:-1, :]),
    )
    for a, b, zb in pairs:
        m = want[a] & (a != b)
        if not m.any():
            continue
        ai = a[m].astype(np.int32).ravel()
        lv = zb[m].astype(np.float32).ravel()
        key = np.stack([ai, lv.view(np.int32)], axis=1)
        uniq, counts = np.unique(key, axis=0, return_counts=True)
        total = {}
        for (i, bits), n in zip(uniq.tolist(), counts.tolist()):
            lvl = float(np.array([bits], dtype=np.int32).view(np.float32)[0])
            n = int(n)
            total[i] = total.get(i, 0) + n
            if n > best_n.get(i, 0):
                best_n[i] = n
                best_lv[i] = lvl
    return best_lv, best_n, total


def _merge_narrow(z, max_width, min_area=0):
    """宽度不足，或面积小于 min_area 的整块，并入接触面积更大的层。"""
    lab, areas, levels = _label_components(z)
    width = _component_width(lab)
    drop = [i for i in range(1, len(areas))
            if width[i] < max_width or (min_area > 0 and areas[i] < min_area)]
    if not drop:
        return z, 0
    best, _best_n, _total = _contact_level(z, lab, drop)
    repl = np.asarray(levels, dtype=np.float32)
    changed = 0
    for i in drop:
        lv = best.get(i)
        if lv is None or lv == levels[i]:
            continue
        repl[i] = lv
        changed += 1
    if changed:
        z = repl[lab]
    return z, changed


def _merge_three_sided(z, max_width):
    """从大层上伸出去、又被同一层三面包住的窄突出部，并入那一层。

    大层外圈不算突出。开口后盖不回来的部分才是伸出去的。
    外接触里某一层占多数，才算被这一层围住。
    """
    r = max_width / 2.0
    z = z.copy()
    changed = 0
    for v in [float(v) for v in np.unique(z)]:
        mask = np.abs(z - v) < 0.5
        if int(mask.sum()) < 4:
            continue
        outside = ~mask
        outside[0, :] = outside[-1, :] = outside[:, 0] = outside[:, -1] = True
        dist = _chamfer_from(outside)
        fat = mask & (dist * 2.0 >= max_width)
        if not fat.any():
            continue
        covered = mask & (_chamfer_from(fat) <= r)
        extra = mask & ~covered
        if not extra.any():
            continue
        lab = np.zeros(z.shape, dtype=np.int32)
        # 只给突出部编号，接触统计沿用整图的层号
        n = 0
        from collections import deque
        h, w = z.shape
        ys, xs = np.nonzero(extra)
        for y0, x0 in zip(ys.tolist(), xs.tolist()):
            if lab[y0, x0]:
                continue
            n += 1
            q = deque(((y0, x0),))
            lab[y0, x0] = n
            while q:
                y, x = q.popleft()
                for ny, nx in ((y - 1, x), (y + 1, x), (y, x - 1), (y, x + 1)):
                    if 0 <= ny < h and 0 <= nx < w and extra[ny, nx] and lab[ny, nx] == 0:
                        lab[ny, nx] = n
                        q.append((ny, nx))
        if n == 0:
            continue
        best, best_n, total = _contact_level(z, lab, list(range(1, n + 1)))
        new_lv = np.full(n + 1, np.nan, dtype=np.float32)
        for i, lv in best.items():
            # 外接触过半才算被同一层围住；两侧各贴一层的窄桥留下
            if total.get(i, 0) <= 0 or lv == v or best_n[i] * 2 <= total[i]:
                continue
            new_lv[i] = lv
        sel = new_lv[lab]
        take = np.isfinite(sel)
        changed += int(take.sum())
        if take.any():
            z = z.copy()
            z[take] = sel[take]
    return z, changed


def _adjacent_components(lab):
    """相连的两块（4 邻接）及其接触格数。"""
    adj = {}
    pairs = ((lab[:, :-1], lab[:, 1:]), (lab[:-1, :], lab[1:, :]))
    for a, b in pairs:
        m = (a != b) & (a > 0) & (b > 0)
        if not m.any():
            continue
        ai = a[m].astype(np.int32).ravel()
        bi = b[m].astype(np.int32).ravel()
        key = np.stack([np.minimum(ai, bi), np.maximum(ai, bi)], axis=1)
        uniq, counts = np.unique(key, axis=0, return_counts=True)
        for (i, j), n in zip(uniq.tolist(), counts.tolist()):
            adj[(int(i), int(j))] = adj.get((int(i), int(j)), 0) + int(n)
    return adj


def _decide_merge(area, level, by_level, min_area, lose_tie=False):
    """按相邻层规则决定这块并入哪一层。by_level 是相邻层号到那一侧面积。

    两层相接：小于 min_area 的并入旁边那层；两边都小则并入面积更大的。
    三层或更多相接：两个较小的并到这两个小层里面积较大的那层。
    """
    if area >= min_area or not by_level:
        return None
    if len(by_level) == 1:
        lv, other = next(iter(by_level.items()))
        if other < min_area:
            if area < other or (area == other and lose_tie):
                return lv
            return None
        return lv
    cands = [(area, level)]
    for lv, other in by_level.items():
        cands.append((other, lv))
    cands.sort()
    target = cands[1][1]
    if cands[0][1] == level and target != level:
        return target
    return None


def _merge_adjacent_small(z, min_area, locked=None):
    """相邻层的小块，按 _decide_merge 并入。locked 为真的格子不再改。"""
    lab, areas, levels = _label_components(z)
    adj = _adjacent_components(lab)
    neigh = {}
    for (i, j) in adj:
        neigh.setdefault(i, []).append(j)
        neigh.setdefault(j, []).append(i)
    moves = {}
    for i in range(1, len(areas)):
        if locked is not None and bool(locked[lab == i].any()):
            continue
        by = {}
        for j in neigh.get(i, []):
            if levels[j] == levels[i]:
                continue
            by.setdefault(levels[j], []).append(j)
        by_level = {lv: max(areas[j] for j in group) for lv, group in by.items()}
        lose_tie = False
        if len(by_level) == 1:
            other = next(iter(by_level.values()))
            group = next(iter(by.values()))
            lose_tie = areas[i] == other and i > min(group)
        lv = _decide_merge(areas[i], levels[i], by_level, min_area, lose_tie)
        if lv is not None:
            moves[i] = lv
    if not moves:
        return z, 0
    repl = np.asarray(levels, dtype=np.float32)
    for i, lv in moves.items():
        repl[i] = lv
    return repl[lab], len(moves)


def _opening_extra(mask, radius):
    """颈宽小于 2*radius、伸不出主体的凸出。"""
    if int(mask.sum()) < 4:
        return np.zeros(mask.shape, dtype=bool)
    outside = ~mask
    outside[0, :] = outside[-1, :] = outside[:, 0] = outside[:, -1] = True
    dist = _chamfer_from(outside)
    fat = mask & (dist * 2.0 >= radius * 2.0)
    if not fat.any():
        return np.zeros(mask.shape, dtype=bool)
    covered = mask & (_chamfer_from(fat) <= radius)
    return mask & ~covered


def _merge_surrounded_protrusions(z, min_area, locked=None):
    """三面被围的凸出也走相邻层并入规则，不一律并进围住它的那层。"""
    side = int(round(min_area ** 0.5))
    radius = side / 2.0
    extra = np.zeros(z.shape, dtype=bool)
    for v in np.unique(z):
        extra |= _opening_extra(np.abs(z - float(v)) < 0.5, radius)
    if locked is not None:
        extra &= ~locked
    if not extra.any():
        return z, 0
    sentinel = np.float32(-1.0)
    field = np.where(extra, z, sentinel).astype(np.float32)
    plab, pareas, plevels = _label_components(field)
    lab, areas, levels = _label_components(z)
    is_prot = np.array(
        [i > 0 and abs(plevels[i] - float(sentinel)) > 0.5 for i in range(len(pareas))],
        dtype=bool)
    stable = ~extra
    by = {}
    pairs = (
        (plab[:, :-1], lab[:, 1:], z[:, 1:], stable[:, 1:]),
        (plab[:, 1:], lab[:, :-1], z[:, :-1], stable[:, :-1]),
        (plab[:-1, :], lab[1:, :], z[1:, :], stable[1:, :]),
        (plab[1:, :], lab[:-1, :], z[:-1, :], stable[:-1, :]),
    )
    p_at_level = np.asarray(plevels, dtype=np.float32)
    for pa, nb, zb, st in pairs:
        m = is_prot[pa] & st & (np.abs(zb - p_at_level[pa]) >= 0.5)
        if not m.any():
            continue
        pid = pa[m].astype(np.int32).ravel()
        nid = nb[m].astype(np.int32).ravel()
        key = np.stack([pid, nid], axis=1)
        uniq = np.unique(key, axis=0)
        for p, n in uniq.tolist():
            lv = levels[n]
            if abs(lv - plevels[p]) < 0.5:
                continue
            slot = by.setdefault(int(p), {})
            slot[lv] = max(slot.get(lv, 0), areas[n])
    moves = {}
    for p, by_level in by.items():
        lv = _decide_merge(pareas[p], plevels[p], by_level, min_area, lose_tie=True)
        if lv is not None:
            moves[p] = lv
    if not moves:
        return z, 0
    repl = np.asarray(plevels, dtype=np.float32)
    for p, lv in moves.items():
        repl[p] = lv
    out = z.copy()
    take = np.isin(plab, np.fromiter(moves.keys(), dtype=np.int32))
    out[take] = repl[plab[take]]
    return out, len(moves)


def _blur(img, sigma):
    """可分离高斯。sigma 用格。"""
    sigma = float(sigma)
    r = max(1, int(round(sigma * 3)))
    x = np.arange(-r, r + 1, dtype=np.float32)
    k = np.exp(-0.5 * (x / sigma) ** 2)
    k /= k.sum()
    pad = np.pad(img.astype(np.float32), ((0, 0), (r, r)), mode='edge')
    tmp = np.empty(img.shape, dtype=np.float32)
    for i in range(img.shape[0]):
        tmp[i] = np.convolve(pad[i], k, mode='valid')
    pad = np.pad(tmp, ((r, r), (0, 0)), mode='edge')
    out = np.empty(img.shape, dtype=np.float32)
    for i in range(img.shape[1]):
        out[:, i] = np.convolve(pad[:, i], k, mode='valid')
    return out


def _soften_merged_edges(z, dem, amp=14.0, land_z=210.0):
    """并入结果保持不变，只把交界顺着地形起伏挪开一点，不修成圆弧。"""
    fine = _blur(dem, 5.0) - _blur(dem, 22.0)
    gy, gx = np.gradient(fine.astype(np.float64))
    mag = np.hypot(gx, gy)
    p = float(np.percentile(mag, 85))
    if p < 1e-6:
        return z.astype(np.float32, copy=False)
    scale = amp / p
    dx = _blur(np.clip(gx * scale, -amp, amp), 10.0)
    dy = _blur(np.clip(gy * scale, -amp, amp), 10.0)
    h, w = z.shape
    yy, xx = np.mgrid[0:h, 0:w]
    xs = np.clip(np.rint(xx + dx).astype(np.int32), 0, w - 1)
    ys = np.clip(np.rint(yy + dy).astype(np.int32), 0, h - 1)
    warped = z[ys, xs].astype(np.float32)
    out = z.astype(np.float32, copy=True)
    # 水陆交界留给海岸恢复，这里只挪陆地层界和深浅海交界
    same = (z < land_z) == (warped < land_z)
    out[same] = warped[same]
    return out


def _nearest_land_level(z, land_z=210.0, fallback=212.0):
    """每个格子到并入结果里最近陆地层的层号。"""
    land = z >= land_z
    best_d = np.full(z.shape, 1e9, dtype=np.float64)
    best = np.full(z.shape, fallback, dtype=np.float32)
    if not land.any():
        return best
    for lv in np.unique(z[land]):
        d = _chamfer_from(np.abs(z - lv) < 0.5)
        take = d < best_d
        best_d[take] = d[take]
        best[take] = np.float32(lv)
    return best


def _restore_coast(z, dem, land, cfg, sigma=2.0):
    """水陆交界用轻平滑后的初始地形，港湾和岬角留下。陆地的层号仍用并入结果。"""
    nat = quantize(_blur(dem, sigma), land, cfg)
    land_z = _land_z(cfg)
    fallback = _first_land_level(cfg)
    nat_water = nat < land_z
    out = z.astype(np.float32, copy=True)
    out[nat_water] = nat[nat_water]
    lab, areas, levels = _label_components(out)
    drop = np.zeros(len(areas), dtype=bool)
    water_ids = [i for i in range(1, len(areas)) if levels[i] < land_z]
    if water_ids:
        main = max(water_ids, key=lambda i: areas[i])
        for i in water_ids:
            if i != main and areas[i] < 120:
                drop[i] = True
    need = drop[lab] | ((out < land_z) & ~nat_water)
    if need.any():
        out[need] = _nearest_land_level(z, land_z, fallback)[need]
    return out


def _euclid_from(sources):
    """到 sources 为真的格子的欧氏距离。"""
    inf = 1e12
    g = np.where(sources, 0.0, inf)

    def dt1(f):
        n = f.shape[0]
        v = np.zeros(n, dtype=np.int32)
        z = np.empty(n + 1, dtype=np.float64)
        d = np.empty(n, dtype=np.float64)
        k = 0
        v[0] = 0
        z[0] = -np.inf
        z[1] = np.inf
        for q in range(1, n):
            s = ((f[q] + q * q) - (f[v[k]] + v[k] * v[k])) / (2.0 * q - 2.0 * v[k])
            while s <= z[k]:
                k -= 1
                s = ((f[q] + q * q) - (f[v[k]] + v[k] * v[k])) / (2.0 * q - 2.0 * v[k])
            k += 1
            v[k] = q
            z[k] = s
            z[k + 1] = np.inf
        k = 0
        for q in range(n):
            while z[k + 1] < q:
                k += 1
            dx = q - v[k]
            d[q] = dx * dx + f[v[k]]
        return d

    h, w = g.shape
    tmp = np.empty_like(g)
    for i in range(h):
        tmp[i] = dt1(g[i])
    out = np.empty_like(g)
    for j in range(w):
        out[:, j] = dt1(tmp[:, j])
    return np.sqrt(out)


def _disk_open(mask, radius):
    """去掉比 radius 更窄的凸出。"""
    fat = mask & (_euclid_from(~mask) >= radius)
    if not fat.any():
        return np.zeros(mask.shape, dtype=bool)
    return _euclid_from(fat) <= radius


def _disk_close(mask, radius):
    """填上比 radius 更窄的凹口。"""
    return ~_disk_open(~mask, radius)


def _only_small(mask, min_area):
    """只留下面积小于阈值的块。"""
    if not mask.any():
        return mask
    field = np.where(mask, 1.0, 0.0).astype(np.float32)
    lab, areas, levels = _label_components(field)
    small = np.zeros(mask.shape, dtype=bool)
    for i in range(1, len(areas)):
        if levels[i] > 0.5 and areas[i] < min_area:
            small[lab == i] = True
    return small


def _merge_coast(z, cfg, fr, radius=32.0, min_area=40 * 40):
    """海岸并入窄凸出和小凹口。面积达到阈值的岸线凸起留下。"""
    box = getattr(cfg, 'COAST_KEEP', None)
    land_z = _land_z(cfg)
    fallback = _first_land_level(cfg)
    z = z.astype(np.float32, copy=True)
    keep = np.zeros(z.shape, dtype=bool)
    if box:
        ox, oy, sx, sy = fr
        yy, xx = np.mgrid[0:z.shape[0], 0:z.shape[1]]
        lon = cfg.LON0 + (xx - ox) / sx
        lat = cfg.LAT1 - (yy - oy) / sy
        keep = (lon >= box[0]) & (lon <= box[1]) & (lat >= box[2]) & (lat <= box[3])
    land = z >= land_z
    opened = _disk_open(land, radius)
    closed = _disk_close(opened, radius * 0.45)
    cut = _only_small(land & ~closed & ~keep, min_area)
    fill = _only_small((~land) & closed & ~keep, min_area)
    n = int(cut.any()) + int(fill.any())
    water_z = float(cfg.WATER_Z)
    if cut.any():
        z[cut] = water_z
    if fill.any():
        z[fill] = _nearest_land_level(z, land_z, fallback)[fill]
    land = z >= land_z
    lab, areas, levels = _label_components(np.where(land, 1.0, 0.0).astype(np.float32))
    main = max(range(1, len(areas)), key=lambda i: areas[i] if levels[i] > 0.5 else -1)
    islands = 0
    for i in range(1, len(areas)):
        if i == main or levels[i] < 0.5 or areas[i] >= min_area:
            continue
        z[lab == i] = water_z
        islands += 1
    return z, n + islands


def _merge_land_necks(z, min_width=20.0, land_z=210.0):
    """陆地层里宽不到 min_width 的窄段并入距离最近的层。宽层的外缘不算窄段。"""
    z = z.astype(np.float32, copy=True)
    half = min_width / 2.0
    locked = np.zeros(z.shape, dtype=bool)
    total = 0
    for _ in range(8):
        levels = [float(v) for v in np.unique(z)]
        narrow = np.zeros(z.shape, dtype=bool)
        for lv in levels:
            if lv < land_z:
                continue
            mask = np.abs(z - lv) < 0.5
            if int(mask.sum()) < 4:
                continue
            dist = _chamfer_from(~mask)
            fat = mask & (dist >= half)
            if not fat.any():
                narrow |= mask & ~locked
                continue
            covered = _chamfer_from(fat) <= half
            narrow |= mask & ~covered & ~locked
        if not narrow.any():
            break
        best_d = np.full(z.shape, 1e9, dtype=np.float64)
        best = z.copy()
        for lv in levels:
            src = np.abs(z - lv) < 0.5
            if not src.any():
                continue
            d = _chamfer_from(src)
            take = narrow & ~src & (d < best_d)
            best_d[take] = d[take]
            best[take] = np.float32(lv)
        moved = narrow & (np.abs(best - z) >= 0.5)
        if not moved.any():
            break
        z[moved] = best[moved]
        locked |= moved
        total += int(moved.sum())
    return z, total


def _keep_water_bulge(z, dem, cfg, fr, neck=8.0, min_area=40 * 40):
    """保留框里按初始水域留宽的那一块凸起，更窄的小水域并进陆地。"""
    box = getattr(cfg, 'COAST_KEEP', None)
    if not box:
        return z, 0
    z = z.astype(np.float32, copy=True)
    land_z = _land_z(cfg)
    fallback = _first_land_level(cfg)
    deep_m = float(preview_recipe(cfg)['deep_m'])
    ox, oy, sx, sy = fr
    yy, xx = np.mgrid[0:z.shape[0], 0:z.shape[1]]
    lon = cfg.LON0 + (xx - ox) / sx
    lat = cfg.LAT1 - (yy - oy) / sy
    keep = (lon >= box[0]) & (lon <= box[1]) & (lat >= box[2]) & (lat <= box[3])
    sea = float(getattr(cfg, 'SEA_Z', 190.0))
    water_z = float(cfg.WATER_Z)
    nat = dem < 0.0
    dist = _euclid_from(~nat)
    fat = nat & (dist >= neck)
    wide = _euclid_from(fat) <= neck if fat.any() else np.zeros(z.shape, dtype=bool)
    paint = wide & keep
    before = z.copy()
    z[paint & (dem >= deep_m)] = water_z
    z[paint & (dem < deep_m)] = sea
    thin = (z < land_z) & keep & ~wide
    if thin.any():
        z[thin] = _nearest_land_level(z, land_z, fallback)[thin]
    water = z < land_z
    lab, areas, levels = _label_components(np.where(water, 1.0, 0.0).astype(np.float32))
    near = _nearest_land_level(z, land_z, fallback)
    for i in range(1, len(areas)):
        if levels[i] < 0.5 or areas[i] >= min_area:
            continue
        cells = lab == i
        if int((cells & keep).sum()) * 2 < areas[i]:
            continue
        z[cells] = near[cells]
    return z, int((np.abs(z - before) >= 0.5).sum())


def coarsen_layers(z, max_width=30.0, min_area=0):
    """宽度不足或面积太小的块并进接触更多的层；三面被围的突出部并进围住它的层。"""
    z = z.astype(np.float32, copy=True)
    for k in range(16):
        z, n_drop = _merge_narrow(z, max_width, min_area)
        z, n_side = _merge_three_sided(z, max_width)
        if min_area > 0:
            print('  [连片] 第 %d 轮  窄或小于 %d 格 %d 块  三面突出 %d 格' % (
                k + 1, min_area, n_drop, n_side))
        else:
            print('  [连片] 第 %d 轮  窄块 %d  三面突出 %d 格' % (k + 1, n_drop, n_side))
        if n_drop == 0 and n_side == 0:
            break
    lab, areas, levels = _label_components(z)
    width = _component_width(lab)
    by = {}
    for i in range(1, len(areas)):
        by.setdefault(levels[i], []).append((areas[i], width[i]))
    for v in sorted(by):
        bits = sorted(by[v], key=lambda t: -t[0])
        kept = sum(1 for a, w in bits if w >= max_width and (min_area <= 0 or a >= min_area))
        print('  [连片] %.0f  %d 块（留下 %d 块）  最大 %d 格' % (
            v, len(bits), kept, bits[0][0]))
    return z


def font(size):
    for p in (r'C:\Windows\Fonts\msyhbd.ttc', r'C:\Windows\Fonts\msyh.ttc'):
        if os.path.isfile(p):
            return ImageFont.truetype(p, size)
    return ImageFont.load_default()


def province_edge(land):
    """边界：陆地掩码内外不同的那一圈格子。"""
    edge = np.zeros(land.shape, dtype=bool)
    edge[1:, :] |= land[1:, :] != land[:-1, :]
    edge[:-1, :] |= land[1:, :] != land[:-1, :]
    edge[:, 1:] |= land[:, 1:] != land[:, :-1]
    edge[:, :-1] |= land[:, 1:] != land[:, :-1]
    thick = edge.copy()
    thick[1:, :] |= edge[:-1, :]
    thick[:-1, :] |= edge[1:, :]
    thick[:, 1:] |= edge[:, :-1]
    thick[:, :-1] |= edge[:, 1:]
    return thick


# 地形图的海拔色带，单位米。和游戏层高不是同一套数。
SOURCE_STOPS = (
    (0.0, (96, 158, 74)),
    (200.0, (150, 178, 82)),
    (500.0, (190, 174, 92)),
    (1000.0, (176, 140, 78)),
    (2000.0, (140, 100, 68)),
    (4000.0, (214, 210, 202)),
)
_STALE_PREVIEW = (
    'preview-v1.png', 'preview-v1--line.png',
    'preview-v2.png', 'preview-v2--line.png',
    'preview-v3.png', 'preview-v3--line.png',
    'layers.npz', 'layers_v1.npz', 'layers_v2.npz',
)


def _ramp_color(elev, stops):
    ms = np.array([m for m, _c in stops], dtype=np.float64)
    cs = np.array([c for _m, c in stops], dtype=np.float64)
    e = np.asarray(elev, dtype=np.float64)
    flat = np.clip(e, ms[0], ms[-1]).ravel()
    idx = np.clip(np.searchsorted(ms, flat, side='right') - 1, 0, len(ms) - 2)
    span = np.maximum(ms[idx + 1] - ms[idx], 1e-6)
    t = ((flat - ms[idx]) / span)[:, None]
    return (cs[idx] * (1.0 - t) + cs[idx + 1] * t).reshape(e.shape + (3,))


def source_rgb(dem, cfg):
    """真实高程上色。海按深度，陆地按海拔连续过渡，再加地形阴影。"""
    deep_m = min(float(preview_recipe(cfg)['deep_m']), -1.0)
    rgb = np.zeros(dem.shape + (3,), dtype=np.float64)
    water = dem < 0.0
    t = np.clip(dem / deep_m, 0.0, 1.0)
    shallow = np.array((36, 108, 176), dtype=np.float64)
    deep = np.array((18, 58, 112), dtype=np.float64)
    rgb[water] = shallow + (deep - shallow) * t[water, None]
    dry = ~water
    if dry.any():
        rgb[dry] = _ramp_color(np.maximum(dem[dry], 0.0), SOURCE_STOPS)
    return shade(np.clip(rgb, 0, 255).astype(np.uint8), dem)


def _present(rgb, title, note, legend_title, rows):
    # 北朝上：内部帧第 0 行已经是北，直接当图像上方。
    map_img = Image.fromarray(rgb)
    ih, iw = rgb.shape[:2]
    scale = 1000.0 / max(ih, iw)
    dw, dh = int(round(iw * scale)), int(round(ih * scale))
    map_img = map_img.resize((dw, dh), Image.BOX)
    canvas = Image.new('RGB', (dw + 220, dh + 108), (28, 32, 36))
    canvas.paste(map_img, (16, 56))
    draw = ImageDraw.Draw(canvas)
    draw.text((16, 12), title, fill=(236, 232, 220), font=font(28))
    draw.text((16, dh + 68), note, fill=(180, 176, 164), font=font(16))
    lx = dw + 36
    draw.text((lx, 64), legend_title, fill=(220, 216, 204), font=font(20))
    for i, (color, text) in enumerate(rows):
        y = 108 + i * 36
        draw.rectangle([lx, y, lx + 22, y + 22], fill=tuple(int(c) for c in color))
        draw.text((lx + 32, y - 2), text, fill=(210, 206, 194), font=font(18))
    return canvas


def render(z, dem, cfg, title, edge=None, note='北朝上 · 海在下方 · 暂无资源', boundary_label='边界'):
    colors, labels = level_style(cfg)
    rgb = np.zeros(z.shape + (3,), dtype=np.uint8)
    for level, color in colors.items():
        rgb[np.abs(z - level) < 0.5] = color
    rgb = shade(rgb, dem)
    if edge is not None:
        rgb[edge] = (230, 48, 36)
    rows = [((colors[level]), '%s  %.0f' % (name, level)) for level, name in labels]
    if edge is not None:
        rows.append(((230, 48, 36), boundary_label))
    return _present(rgb, title, note, '高度层', rows)


def render_source(dem, cfg, title):
    rgb = source_rgb(dem, cfg)
    hi = 0.0
    dry = dem >= 0.0
    if dry.any():
        hi = float(np.percentile(dem[dry], 99))
    rows = [((18, 58, 112), '海')]
    for meter, color in SOURCE_STOPS:
        if meter > hi + 80:
            break
        rows.append((color, '%.0f 米' % meter if meter else '海岸'))
    return _present(rgb, title, '北朝上 · 海在下方 · 真实高程', '高程', rows)


def _drop_stale_previews(directory):
    for name in _STALE_PREVIEW:
        path = os.path.join(directory, name)
        if os.path.isfile(path):
            os.remove(path)


def _rings_on_grid(pack, shape, cfg, fr):
    """手圈掩码对到这次的预览格子。画幅没变就直接用。"""
    layer2 = np.asarray(pack['layer2'], dtype=bool)
    layer3 = np.asarray(pack['layer3'], dtype=bool)
    same = (layer2.shape == shape and layer3.shape == shape
            and abs(float(pack['lon0']) - float(cfg.LON0)) < 1e-3
            and abs(float(pack['lat1']) - float(cfg.LAT1)) < 1e-3)
    if same:
        return layer2, layer3
    ox, oy, sx, sy = fr
    h, w = shape
    yy, xx = np.mgrid[0:h, 0:w]
    lon = cfg.LON0 + (xx - ox) / sx
    lat = cfg.LAT1 - (yy - oy) / sy
    ix = np.rint(float(pack['ox']) + (lon - float(pack['lon0'])) * float(pack['sx'])).astype(np.int32)
    iy = np.rint(float(pack['oy']) + (float(pack['lat1']) - lat) * float(pack['sy'])).astype(np.int32)
    sh, sw = layer2.shape
    ok = (ix >= 0) & (ix < sw) & (iy >= 0) & (iy < sh)
    m2 = np.zeros(shape, dtype=bool)
    m3 = np.zeros(shape, dtype=bool)
    m2[ok] = layer2[iy[ok], ix[ok]]
    m3[ok] = layer3[iy[ok], ix[ok]]
    return m2, m3


def paint_layer2_patch(z, cfg, fr, path):
    """圈内陆地改成二层。圈外不动，水域不动。"""
    levels = [float(level) for _limit, level in layer_bands(cfg)]
    if len(levels) < 2 or not os.path.isfile(path):
        return z
    pack = np.load(path)
    mark, _other = _rings_on_grid(pack, z.shape, cfg, fr)
    out = z.astype(np.float32, copy=True)
    land = out >= _land_z(cfg)
    out[land & mark] = levels[1]
    print('手圈并入二层 %d 格' % int((land & mark).sum()))
    return out


def paint_layer_rings(z, cfg, fr, path):
    """大圈是二层，圈里的小圈是三层。圈外陆地回到一层。水域不动。"""
    levels = [float(level) for _limit, level in layer_bands(cfg)]
    if len(levels) < 3 or not os.path.isfile(path):
        return z
    pack = np.load(path)
    layer2, layer3 = _rings_on_grid(pack, z.shape, cfg, fr)
    out = z.astype(np.float32, copy=True)
    land = out >= _land_z(cfg)
    out[land] = levels[0]
    out[land & layer2] = levels[1]
    out[land & layer3] = levels[2]
    print('手圈 二层 %d 格  三层 %d 格' % (
        int((land & layer2 & ~layer3).sum()), int((land & layer3).sum())))
    return out


def _write_final(z, dem, cfg, land, directory, note, boundary_label):
    """地形、游戏分层、带省界的分层。中间版本不留。"""
    _drop_stale_previews(directory)
    edge = province_edge(land)
    source = os.path.join(directory, 'source.png')
    plain = os.path.join(directory, 'preview.png')
    lined = os.path.join(directory, 'preview--line.png')
    render_source(dem, cfg, cfg.MAP_NAME).save(source)
    render(z, dem, cfg, cfg.MAP_NAME, note=note, boundary_label=boundary_label).save(plain)
    render(z, dem, cfg, cfg.MAP_NAME, edge=edge, note=note,
           boundary_label=boundary_label).save(lined)
    print('地形 -> %s' % source)
    print('游戏预览 -> %s' % plain)
    print('边界 -> %s' % lined)


def build_relief(P, cfg, zoom=8, refresh=False, final_path=None, dem=None):
    """切层、连片、相邻并入、海岸和窄颈。阈值读配方。

    画幅按陆地外接框定成 600–800 的矩形，不写回配方。
    final_path 有值时只写这一张最终图，不碰游戏用的 dem.npz。
    """
    apply_land_view(P, cfg)
    recipe = preview_recipe(cfg)
    width = float(recipe['min_width'])
    area_side = int(recipe['min_area'])
    adjacent = int(recipe['adjacent'])
    coast_side = int(recipe['coast_area'])
    land_z = _land_z(cfg)
    label = str(recipe['boundary_label'])
    fr = frame(cfg)
    print('取景 lon %.2f..%.2f  lat %.2f..%.2f' % (cfg.LON0, cfg.LON1, cfg.LAT0, cfg.LAT1))
    print('预览配方 宽%.0f 面积%d 相邻%d 海岸%d/半径%.0f 窄颈%.0f 水域%.0f' % (
        width, area_side, adjacent, coast_side, float(recipe['coast_radius']),
        float(recipe['land_neck']), float(recipe['water_neck'])))
    land = land_mask(P, cfg, fr)
    dem_path = os.path.join(P.dir, 'dem.npz')
    if dem is None:
        cached = None
        if os.path.isfile(dem_path) and not refresh:
            cached = np.load(dem_path)['dem'].astype(np.float32)
        if cached is not None and tuple(cached.shape) == tuple(land.shape):
            dem = cached
            print('沿用已有高程 %s' % dem_path)
        else:
            dem = fetch_dem(cfg, fr, zoom)
    else:
        dem = np.asarray(dem, dtype=np.float32)
        print('沿用传入高程 %s' % (dem.shape,))
    z = quantize(dem, land, cfg)
    print('高程 %.0f..%.0f m' % (float(dem.min()), float(dem.max())))
    _colors, labels = level_style(cfg)
    for level, name in labels:
        pct = 100.0 * (np.abs(z - level) < 0.5).mean()
        if pct >= 0.05:
            print('  %s %.0f  %.1f%%' % (name, level, pct))
    if not final_path:
        np.savez_compressed(dem_path, dem=dem.astype(np.float32), zoom=zoom)
    print('整理成连续大层，并掉窄于 %.0f 或小于 %d×%d 的块 …' % (width, area_side, area_side))
    z2 = coarsen_layers(z, max_width=width, min_area=area_side * area_side)
    print('相邻层和三面凸出里小于 %d×%d 的再并 …' % (adjacent, adjacent))
    z3 = z2
    locked = np.zeros(z3.shape, dtype=bool)
    adjacent_area = adjacent * adjacent
    for k in range(8):
        before = z3.copy()
        z3, n_adj = _merge_adjacent_small(z3, adjacent_area, locked)
        z3, n_pro = _merge_surrounded_protrusions(z3, adjacent_area, locked)
        locked |= z3 != before
        print('  [相邻] 第 %d 轮  整块 %d  三面凸出 %d' % (k + 1, n_adj, n_pro))
        if n_adj == 0 and n_pro == 0:
            break
    print('陆地层界顺着地形轻轻起伏 …')
    z3 = _soften_merged_edges(z3, dem, amp=float(recipe['soften_amp']), land_z=land_z)
    print('海岸贴回初始地形 …')
    z3 = _restore_coast(z3, dem, land, cfg)
    print('海岸并入凸出，保留框里的岸线不动 …')
    z3, n_coast = _merge_coast(
        z3, cfg, fr, radius=float(recipe['coast_radius']), min_area=coast_side * coast_side)
    print('  海岸并掉 %d 块' % n_coast)
    print('陆地层窄于 %.0f 格的并入最近层 …' % float(recipe['land_neck']))
    z3, n_neck = _merge_land_necks(z3, float(recipe['land_neck']), land_z)
    print('  窄处 %d 格' % n_neck)
    if getattr(cfg, 'COAST_KEEP', None):
        z3, n_keep = _keep_water_bulge(
            z3, dem, cfg, fr, neck=float(recipe['water_neck']), min_area=coast_side * coast_side)
        print('  保留框小水域 %d' % n_keep)
        note3 = '北朝上 · 海在下方 · 保留框内留一块宽水域 · 暂无资源'
    else:
        note3 = '北朝上 · 海在下方 · 暂无资源'
    ring_name = getattr(cfg, 'LAYER_RINGS', None)
    if ring_name:
        z3 = paint_layer_rings(z3, cfg, fr, P.data_file(ring_name))
    patch_name = getattr(cfg, 'LAYER2_PATCH', None)
    if patch_name:
        z3 = paint_layer2_patch(z3, cfg, fr, P.data_file(patch_name))
    if final_path:
        render(z3, dem, cfg, cfg.MAP_NAME, edge=province_edge(land), note=note3,
               boundary_label=label).save(final_path)
        print('最终预览 -> %s' % final_path)
    else:
        _write_final(z3, dem, cfg, land, P.dir, note3, label)
        np.savez_compressed(os.path.join(P.dir, 'layers_v3.npz'), z=z3.astype(np.float32))
    return z3


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--map', dest='map_id', required=True)
    ap.add_argument('--zoom', type=int, default=8)
    ap.add_argument('--refresh', action='store_true')
    ap.add_argument('--out', default='', help='只写最终预览到地图目录下的这个文件名')
    a = ap.parse_args()
    P = proj.current(a.map_id)
    final = os.path.join(P.dir, a.out) if a.out else None
    build_relief(P, P.cfg, zoom=a.zoom, refresh=a.refresh, final_path=final)


if __name__ == '__main__':
    main()
