# -*- coding: utf-8 -*-
"""Terrain generator (recipe driven).

Reads maps/<id>/config.py and writes into assets/<map name>/source/:
  terrain.npz            高程原始值 (uint16) + 陆地掩码
  terrain_preview.png    高程/地貌预览（含出生点标注）

用法：  python tools/terrain_gen.py [--map <map_id>]
"""
import json, os, sys, math, argparse
import numpy as np
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)
import project as proj

_ap = argparse.ArgumentParser(add_help=False)
_ap.add_argument('--map', dest='map_id', default=None, help='maps/<id>/ 配方')
_args, _unknown = _ap.parse_known_args()

P = proj.current(_args.map_id)
cfg = P.cfg
WS = proj.ROOT
MAPNAME = P.map_name
ASSETS = P.assets
SRCDIR = P.source
os.makedirs(SRCDIR, exist_ok=True)

GRID = cfg.GRID
BORDER = cfg.BORDER
PLAY = GRID - 2 * BORDER
CELL = cfg.CELL
Z_SCALE = cfg.Z_SCALE
LAND_Z = cfg.LAND_Z
WATER_Z = cfg.WATER_Z
RAW_LAND = int(round(LAND_Z / Z_SCALE))
RAW_WATER = int(round(WATER_Z / Z_SCALE))
SLOPE_IMPASSABLE = cfg.SLOPE_IMPASSABLE
PEAK_IMPASSABLE = cfg.PEAK_IMPASSABLE

GEOJSONS = [(P.data_file(f), prov, amp) for f, prov, amp in getattr(cfg, 'GEOJSONS', [])]
if not GEOJSONS:
    GEOJSONS = [(P.data_file(getattr(cfg, 'GEOJSON', 'main_boundary.json')), '', 1.0)]
OUTSIDE_AMP = float(getattr(cfg, 'OUTSIDE_AMP', 0.70))

LON0, LON1 = cfg.LON0, cfg.LON1
LAT0, LAT1 = cfg.LAT0, cfg.LAT1
V_STRETCH = cfg.V_STRETCH

BASES = cfg.BASES
DEFAULT_AMP = getattr(cfg, 'DEFAULT_AMP', 0.35)

# 地形塑形参数：配方可覆盖，缺省用这里的默认值
TERRAIN_DEFAULTS = {
    'noise_mount': (26, 5, 12345),
    'noise_detail': (7, 4, 4242),
    'coast_cells': 46.0,
    'river_cells': 34.0,
    'mount_gain': (14.0, 190.0),
    'hills_gain': (9.0, 24.0),
    'outside': (26.0, 18.0, 300.0, 90.0),
    'sea': (18.0, 78.0, 90.0, 6.0),
    'river': (3, 30.0, 2.6, 160.0, 4.0),
    'smooth': (2, 2),
    'pad': (12.0, 18.0, 252.0),
    'res_pad': (8.0, 12.0),          # 资源/要点小平台 (半径, 过渡)
    'sea_split_y': 0.62,
    'sea_split_x': 0.60,
    'sea_split_y2': 0.50,
    'z_clip': (60.0, 420.0),
    'wall': (2.0, 330.0),
    'lat_bias': 0.62,          # 省域竖直居中偏置（越大南侧海面越多）
}
import layer_height
layer_height.apply(cfg)
T = dict(TERRAIN_DEFAULTS)
T.update(getattr(cfg, 'TERRAIN', {}) or {})
if getattr(cfg, 'COAST', None):
    T['coast'] = list(cfg.COAST)          # 真实海岸线多边形（用于海陆判定）

BASE_ORE_OFFSETS = tuple(getattr(cfg, 'BASE_ORE_OFFSETS', ((-16, -11), (17, -9), (-2, 18))))
BASE_ORE_NEAR = tuple(getattr(cfg, 'BASE_ORE_NEAR', ()) or ())
BASE_ORE_FAR = tuple(getattr(cfg, 'BASE_ORE_FAR', ()) or ())
ORE_NEAR_MAX_DIST = float(getattr(cfg, 'ORE_NEAR_MAX_DIST', 100.0))
# ---- 矿脉新布局参数（每名玩家 4 个绕圈 + 中路争夺 4~6 个）----
BASE_ORE_DIRS = tuple(getattr(cfg, 'BASE_ORE_DIRS', ((0.7071, -0.7071), (-0.7071, -0.7071),
                                                      (0.7071, 0.7071), (-0.7071, 0.7071))))
ORE_NEAR_R = float(getattr(cfg, 'ORE_NEAR_R', 38.0))
ORE_MID_R = float(getattr(cfg, 'ORE_MID_R', 78.0))
CENTER_ORE_COUNT = int(getattr(cfg, 'CENTER_ORE_COUNT', 6))
CENTER_ORE_R = float(getattr(cfg, 'CENTER_ORE_R', 92.0))
CENTER_ORE_PHASE = float(getattr(cfg, 'CENTER_ORE_PHASE', 22.0))
ORE_FOOT_MIN = float(getattr(cfg, 'ORE_FOOT_MIN', 1.0))
ORE_FOOT_MAX = float(getattr(cfg, 'ORE_FOOT_MAX', 5.0))
ORE_MOUTH_CLEAR = float(getattr(cfg, 'ORE_MOUTH_CLEAR', 22.0))
CENTER_ORE = list(getattr(cfg, 'CENTER_ORE', []) or [])
FORD_ORE = list(getattr(cfg, 'FORD_ORE', []) or [])
FORD_ORE_OFFSET = tuple(getattr(cfg, 'FORD_ORE_OFFSET', (6, 6)))
DERRICK_POS = list(getattr(cfg, 'DERRICK_POS', []) or [])
BASE_DERRICK_OFFSETS = tuple(getattr(cfg, 'BASE_DERRICK_OFFSETS', ((27, 15), (-28, 12))))
DERRICKS_PER_BASE = int(getattr(cfg, 'DERRICKS_PER_BASE', 2))
ORE_BASE_CLEARANCE = float(getattr(cfg, 'ORE_BASE_CLEARANCE', 30.0))
ORE_MIN_SPACING = float(getattr(cfg, 'ORE_MIN_SPACING', 16.0))
RESOURCE_PAD_CLEARANCE = float(getattr(cfg, 'RESOURCE_PAD_CLEARANCE', 26.0))
RESOURCE_MIN_SPACING = float(getattr(cfg, 'RESOURCE_MIN_SPACING', 14.0))
POST_POS = list(getattr(cfg, 'POST_POS', []) or [])
GARAGE_POS = getattr(cfg, 'GARAGE_POS', None)
OUTER_ORE = list(getattr(cfg, 'OUTER_ORE', []) or [])
OUTER_DERRICK = list(getattr(cfg, 'OUTER_DERRICK', []) or [])
CITY_SEATS = dict(getattr(cfg, 'CITY_SEATS', {}) or {})
CITY_POINTS = [tuple(p) for p in (getattr(cfg, 'CITY_POINTS', None) or ())]
CITY_AMP_AT = [tuple(p) for p in (getattr(cfg, 'CITY_AMP_AT', None) or ())]
BASE_CITY_CLEARANCE = float(getattr(cfg, 'BASE_CITY_CLEARANCE', 30.0))
CITY_ORE_COUNT = int(getattr(cfg, 'CITY_ORE_COUNT', 28))
CITY_ORE_OFFSET = float(getattr(cfg, 'CITY_ORE_OFFSET', 14.0))
CITY_DERRICK_OFFSET = float(getattr(cfg, 'CITY_DERRICK_OFFSET', 11.0))
ORE_CLEAR_RADIUS = float(getattr(cfg, 'ORE_CLEAR_RADIUS', 18.0))
BRIDGES = list(getattr(cfg, 'BRIDGES', []) or [])
BRIDGE_TYPE = getattr(cfg, 'BRIDGE_TYPE', 'Bridge2a')
AIRPORT_POS = list(getattr(cfg, 'AIRPORT_POS', []) or [])
HOSPITAL_POS = list(getattr(cfg, 'HOSPITAL_POS', []) or [])
ACADEMY_POS = list(getattr(cfg, 'ACADEMY_POS', []) or [])

BASES = cfg.BASES
RIVERS = cfg.RIVERS
CITY_AMP = dict(getattr(cfg, 'CITY_AMP', {}) or {})

# ------------------------------------------------------------------ helpers
def load_geojson():
    cities = []
    for path, province, amp in GEOJSONS:
        gj = json.load(open(path, encoding='utf-8'))
        for f in gj['features']:
            g = f['geometry']
            name = f['properties'].get('name', '')
            rings = []
            if g['type'] == 'Polygon':
                rings = [g['coordinates'][0]]
            elif g['type'] == 'MultiPolygon':
                rings = [p[0] for p in g['coordinates']]
            cities.append((name, rings, province, amp))
    return cities


class Geo:
    def __init__(self):
        self.cities = load_geojson()
        allpts = [p for _, rings, _, _ in self.cities for r in rings for p in r]
        xs = [p[0] for p in allpts]; ys = [p[1] for p in allpts]
        self.lon0, self.lon1 = LON0, LON1
        self.lat0, self.lat1 = LAT0, LAT1
        # cell coords inside the full 640 grid; province centred horizontally in play area,
        # bottom of province sits above the southern sea margin
        self.sx = PLAY / (self.lon1 - self.lon0)
        self.sy = self.sx * V_STRETCH
        prov_h = (self.lat1 - self.lat0) * self.sy
        self.oy = BORDER + (PLAY - prov_h) * T['lat_bias']   # bias -> more sea below
        self.ox = BORDER

    def to_cell(self, lon, lat):
        x = self.ox + (lon - self.lon0) * self.sx
        # ⚠ 轴向（T1.8.7 实测锚定，勿再改）：**数据第 0 行 = 最北**，行号越大越往南，
        #   与 v11 蓝图 / terrain.npz 的 raw / dem.npz **同帧**。
        #   证据（work/diag_geo_frame.py，三个独立锚点）：
        #     ① 南岸三个市中心按本式换算后距海 2~47 格；
        #        旧式 (lat-lat0) 换算（行 0 = 南）把它们放到内陆 500~630 格处；
        #     ② make_mask() 与地形 land 的 IoU：本式 0.82 / 旧式 0.63；
        #     ③ dem.npz 同帧取海拔：西北高地 545m > 东北低地 40m。
        #   旧公式让 geo 帧与 raw 帧南北相反 -> 边界线、城镇、油田、河流净距全部与地形颠倒。
        y = self.oy + (self.lat1 - lat) * self.sy
        return x, y

    def make_mask(self, size=GRID):
        """Rasterise province polygons -> boolean land mask (size x size, y down)."""
        img = Image.new('L', (size, size), 0)
        d = ImageDraw.Draw(img)
        for name, rings, prov, amp in self.cities:
            for r in rings:
                pts = [self.to_cell(lon, lat) for lon, lat in r]
                d.polygon(pts, fill=255)
        m = np.array(img) > 0
        return m

    def city_index_map(self, size=GRID):
        """For each cell: index of the city polygon containing it (-1 = outside)."""
        idx = np.full((size, size), -1, dtype=np.int16)
        for i, (name, rings, prov, amp) in enumerate(self.cities):
            img = Image.new('L', (size, size), 0)
            d = ImageDraw.Draw(img)
            for r in rings:
                d.polygon([self.to_cell(lon, lat) for lon, lat in r], fill=255)
            m = np.array(img) > 0
            idx[m & (idx < 0)] = i
        return idx


def polyline_mask(geo, pts, size=GRID, width=1.0):
    img = Image.new('L', (size, size), 0)
    d = ImageDraw.Draw(img)
    xy = [geo.to_cell(lon, lat) for lon, lat in pts]
    d.line(xy, fill=255, width=max(1, int(width)), joint='curve')
    return np.array(img) > 0


def distance_transform(mask):
    """Approximate Euclidean distance (in cells) from True cells, chamfer 2-pass."""
    INF = 1e9
    h, w = mask.shape
    d = np.where(mask, 0.0, INF)
    # forward
    for y in range(h):
        row = d[y]
        if y > 0:
            prev = d[y - 1]
            row[:] = np.minimum(row, prev + 1.0)
            row[1:] = np.minimum(row[1:], prev[:-1] + 1.4142)
            row[:-1] = np.minimum(row[:-1], prev[1:] + 1.4142)
        for x in range(1, w):
            row[x] = min(row[x], row[x - 1] + 1.0)
    # backward
    for y in range(h - 1, -1, -1):
        row = d[y]
        if y < h - 1:
            nxt = d[y + 1]
            row[:] = np.minimum(row, nxt + 1.0)
            row[1:] = np.minimum(row[1:], nxt[:-1] + 1.4142)
            row[:-1] = np.minimum(row[:-1], nxt[1:] + 1.4142)
        for x in range(w - 2, -1, -1):
            row[x] = min(row[x], row[x + 1] + 1.0)
    return d


def smooth(a, radius=1, passes=2):
    for _ in range(passes):
        p = np.pad(a, radius, mode='edge')
        acc = np.zeros_like(a, dtype=np.float64)
        n = 0
        for dy in range(-radius, radius + 1):
            for dx in range(-radius, radius + 1):
                acc += p[radius + dy:radius + dy + a.shape[0], radius + dx:radius + dx + a.shape[1]]
                n += 1
        a = acc / n
    return a


def value_noise(shape, cells, seed):
    rng = np.random.RandomState(seed)
    gh = int(math.ceil(shape[0] / cells)) + 2
    gw = int(math.ceil(shape[1] / cells)) + 2
    g = rng.rand(gh, gw)
    ys = np.arange(shape[0]) / cells
    xs = np.arange(shape[1]) / cells
    y0 = np.floor(ys).astype(int); x0 = np.floor(xs).astype(int)
    fy = (ys - y0)[:, None]; fx = (xs - x0)[None, :]
    fy = fy * fy * (3 - 2 * fy); fx = fx * fx * (3 - 2 * fx)
    g00 = g[np.ix_(y0, x0)]; g01 = g[np.ix_(y0, x0 + 1)]
    g10 = g[np.ix_(y0 + 1, x0)]; g11 = g[np.ix_(y0 + 1, x0 + 1)]
    top = g00 * (1 - fx) + g01 * fx
    bot = g10 * (1 - fx) + g11 * fx
    return top * (1 - fy) + bot * fy


def fbm(shape, base_cells, octaves, seed, gain=0.5):
    out = np.zeros(shape, dtype=np.float64)
    amp = 1.0; total = 0.0; cells = base_cells
    for o in range(octaves):
        out += amp * value_noise(shape, cells, seed + o * 977)
        total += amp
        amp *= gain
        cells = max(2.0, cells * 0.5)
    return out / total


# ------------------------------------------------------------------ terrain
def _city_at(idx, x, y):
    """这个格子落在哪一块里。贴边时向周围看三格。"""
    h, w = idx.shape
    ix, iy = int(round(x)), int(round(y))
    for rad in range(0, 4):
        for dy in range(-rad, rad + 1):
            for dx in range(-rad, rad + 1):
                if rad and max(abs(dx), abs(dy)) != rad:
                    continue
                yy, xx = iy + dy, ix + dx
                if 0 <= yy < h and 0 <= xx < w and int(idx[yy, xx]) >= 0:
                    return int(idx[yy, xx])
    return -1


def _seat_lookup(geo):
    """配方给出的中心点落在哪一块上。返回 {块名: (cx, cy)}。"""
    cached = getattr(geo, '_seat_lookup', None)
    if cached is not None:
        return cached
    pts = [(float(lon), float(lat)) for lon, lat in CITY_POINTS]
    if not pts:
        pts = [(float(lon), float(lat)) for lon, lat in CITY_SEATS.values()]
    idx = geo.city_index_map()
    found = {}
    for lon, lat in pts:
        x, y = geo.to_cell(lon, lat)
        i = _city_at(idx, x, y)
        if i < 0:
            continue
        found[geo.cities[i][0]] = (x, y)
    geo._seat_lookup = found
    return found


def _amp_lookup(geo):
    """按中心点覆盖这一块的山地强度。"""
    cached = getattr(geo, '_amp_lookup', None)
    if cached is not None:
        return cached
    idx = geo.city_index_map()
    found = {}
    for lon, lat, amp in CITY_AMP_AT:
        x, y = geo.to_cell(float(lon), float(lat))
        i = _city_at(idx, x, y)
        if i >= 0:
            found[geo.cities[i][0]] = float(amp)
    geo._amp_lookup = found
    return found


def city_centers(geo, inside_only=True):
    """城市中心 [(块名, cx, cy)]。配方点用给出的坐标，其余用多边形质心。

    inside_only=True 时丢掉落在可玩区之外的。
    """
    lo, hi = BORDER + 4, BORDER + PLAY - 4
    seats = _seat_lookup(geo)
    out = []
    for name, rings, prov, _amp in geo.cities:
        if name in seats:
            cx, cy = seats[name]
        elif name in CITY_SEATS:
            cx, cy = geo.to_cell(*CITY_SEATS[name])
        else:
            best = None
            for r in rings:
                if best is None or len(r) > len(best):
                    best = r
            if not best:
                continue
            lon = sum(p[0] for p in best) / len(best)
            lat = sum(p[1] for p in best) / len(best)
            cx, cy = geo.to_cell(lon, lat)
        if inside_only and not (lo <= cx <= hi and lo <= cy <= hi):
            continue
        out.append((name, cx, cy))
    return out


def city_ore_points(geo, count, margin=34.0):
    """城市矿脉候选点：先用配方点中的块，再按间隔取其余块，凑够 count 个。

    目标点必须离可玩区边缘有 margin 格，否则会被挤到边缘挡墙上。
    """
    lo, hi = BORDER + margin, BORDER + PLAY - margin
    centers = [c for c in city_centers(geo) if lo <= c[1] <= hi and lo <= c[2] <= hi]
    seated = set(_seat_lookup(geo))
    gx = [c for c in centers if c[0] in seated]
    other = [c for c in centers if c[0] not in seated]
    need = max(0, count - len(gx))
    picked = list(gx)
    if need > 0 and other:
        step = max(1, len(other) // need)
        picked += other[::step][:need]
    return [(cx, cy) for _n, cx, cy in picked]


def snap_diag(a):
    """矿脉朝向只能取 45/135/225/315（RA3 矿脉模型只支持斜向）。"""
    return round((a - math.pi / 4) / (math.pi / 2)) * (math.pi / 2) + math.pi / 4


def ore_slots(geo, as_cells=True, base_cells=None):
    """矿脉槽位：每个基地旁 2 个。其余矿另按玩家总数撒到陆地和水面。"""
    slots = []
    order = []
    dirs = list(BASE_ORE_DIRS)
    for i, (lon, lat, name, team, kind) in enumerate(BASES):
        if base_cells and name in base_cells:
            cx, cy = base_cells[name]
        else:
            cx, cy = geo.to_cell(lon, lat)
        near_d = [dirs[0], dirs[2]] if (i % 2 == 0) else [dirs[1], dirs[3]]
        _near_r = float(T.get('ore_near_slot_r', 46.0))
        for dx, dy in near_d:
            slots.append((cx + dx * _near_r, cy + dy * _near_r))
            order.append((-dx, -dy))
    return slots, order


def derrick_slots(geo, base_cells=None):
    """每个出生点 2 个油井：放在对应城市中心或周边。"""
    import math as _m
    slots = []
    for lon, lat, name, team, kind in BASES:
        seat = CITY_SEATS.get(name + '市')
        cx, cy = geo.to_cell(*seat) if seat else (base_cells[name] if base_cells and name in base_cells else geo.to_cell(lon, lat))
        for i in range(DERRICKS_PER_BASE):
            ang = (i * 2.0 + 0.6) * _m.pi / max(1, DERRICKS_PER_BASE)
            off = CITY_DERRICK_OFFSET
            slots.append((cx + _m.cos(ang) * off, cy + _m.sin(ang) * off))
    return slots


def resolve_base_cells(geo, land, limit=240, allow_water=False):
    """把出生点吸附到省内陆地上：尽量避开城市中心，并且不能压在河道上（会把河堵死）。"""
    seats = [(cx, cy) for _n, cx, cy in city_centers(geo)]
    wet = np.zeros((GRID, GRID), dtype=bool)          # 会成水的河道
    for _n, pts, bed in RIVERS:
        if bed < WATER_Z:
            wet |= polyline_mask(geo, pts, width=T['river'][0])
    d_riv = distance_transform(wet)
    riv_clear = float(T.get('base_river_clearance', 52.0))
    resolved = {}
    _explicit = list(T.get('base_cells') or [])       # 直接用格坐标指定出生点（优先）
    for _bi, (lon, lat, name, team, kind) in enumerate(BASES):
        if _bi < len(_explicit):
            cx, cy = float(_explicit[_bi][0]), float(_explicit[_bi][1])
        else:
            cx, cy = geo.to_cell(lon, lat)
        ix, iy = int(round(cx)), int(round(cy))

        def _far_from_city(x, y):
            return all(math.hypot(x - sx, y - sy) >= BASE_CITY_CLEARANCE for sx, sy in seats)

        def _search(pred):
            best, bd = None, 1e18
            for dy in range(-limit, limit + 1):
                for dx in range(-limit, limit + 1):
                    x, y = ix + dx, iy + dy
                    if 0 <= x < GRID and 0 <= y < GRID and _land_ok[y, x] and pred(x, y):
                        d = dx * dx + dy * dy
                        if d < bd:
                            bd, best = d, (x, y)
            return best

        _yy2, _xx2 = np.mgrid[0:GRID, 0:GRID]
        _wor2 = np.where(land, _xx2, GRID + 999).min(axis=1)
        _has2 = (_wor2 < GRID)[:, None]
        _wor_all2 = float(_wor2.min())
        _strip2 = max(0.0, _wor_all2 + float(T.get('west_land_margin', 70.0)))
        _wor2 = np.where(_wor2 < GRID, _wor2, GRID)[:, None]
        _cx2 = float(T.get('sw_corner_x', 0.45)) * GRID
        _cy2 = float(T.get('sw_corner_y', 0.36)) * GRID
        sw_plain = ((_xx2 < _strip2) | ((_xx2 < _wor2) & _has2)
                    | ((_xx2 < _cx2) & (_yy2 < _cy2)))
        _land_ok = land | sw_plain      # 左下角连片陆地也算可出生陆地
        if allow_water:
            # 允许水域出生点（水上基地）：只要在"任何合法地形集合"里即可，不再强制陆地
            _land_ok = _land_ok | (~land)
        ok = (0 <= ix < GRID and 0 <= iy < GRID and _land_ok[iy, ix]
              and _far_from_city(ix, iy) and d_riv[iy, ix] >= riv_clear)
        if ok:
            resolved[name] = (cx, cy)
            continue
        # 依次放宽：避开城市+避开河 -> 只避开河 -> 只要求陆地
        best = (_search(lambda x, y: _far_from_city(x, y) and d_riv[y, x] >= riv_clear)
                or _search(lambda x, y: d_riv[y, x] >= riv_clear)
                or _search(lambda x, y: True))
        if best is None:
            raise SystemExit('出生点 %s 附近找不到陆地' % name)
        print('  [修正] 出生点 %s 避开城市/河道，吸附到 (%d,%d)，离河道 %.0f 格' % (
            name, best[0], best[1], d_riv[best[1], best[0]]))
        resolved[name] = (float(best[0]), float(best[1]))
    return resolved


def snap_to_mask(cx, cy, mask, max_search=90):
    """把点吸附到 mask 内最近的一格（用于"不铺平平台"时把出生点挪到平整地面上）。"""
    ix, iy = int(round(cx)), int(round(cy))
    H, W = mask.shape
    if 0 <= ix < W and 0 <= iy < H and mask[iy, ix]:
        return (ix, iy)
    for r in range(1, max_search + 1):
        for dy in range(-r, r + 1):
            for dx in range(-r, r + 1):
                if max(abs(dx), abs(dy)) != r:
                    continue
                x, y = ix + dx, iy + dy
                if 0 <= x < W and 0 <= y < H and mask[y, x]:
                    return (x, y)
    return None


def land_layer_index(z, T=None):
    """陆地层号。水域是 -1，其余格子取最近的可建层。"""
    T = T if T is not None else {}
    levels = sorted(float(v) for v in (T.get('levels') or [LAND_Z]))
    lv = np.asarray(levels, dtype=np.float64)
    idx = np.argmin(np.abs(z[..., None] - lv[None, None, :]), axis=-1).astype(np.int16)
    idx[z < WATER_Z] = np.int16(-1)
    return idx


def near_ore_surface_ok(z, bx, by, x, y, T=None, layers=None):
    """基地旁的矿落在哪一层。

    基地在陆地层：只能是同一陆地层，或近矿范围内的水域。
    基地在水域：必须在水域。
    """
    T = T if T is not None else {}
    H, W = z.shape
    ix, iy = int(round(float(bx))), int(round(float(by)))
    jx, jy = int(round(float(x))), int(round(float(y)))
    if not (0 <= ix < W and 0 <= iy < H and 0 <= jx < W and 0 <= jy < H):
        return False
    if layers is None:
        layers = land_layer_index(z, T)
    if z[iy, ix] < WATER_Z:
        return bool(z[jy, jx] < WATER_Z)
    if z[jy, jx] < WATER_Z:
        near_max = float(T.get('ore_near_max_r', 62.0))
        return math.hypot(float(x) - float(bx), float(y) - float(by)) <= near_max + 0.5
    return int(layers[jy, jx]) >= 0 and int(layers[jy, jx]) == int(layers[iy, ix])


def resolve_slots(geo, z, land, base_cells, verbose=True, base_extra_snap=None,
                  ore_open_rank=False, ramps=None):
    """按构建顺序落资源：近矿、近处油井、外围矿、城市油田和房子，
    然后哨站和精兵学院、车库医院机场、船厂。

    约束：落在省内陆地上、非水域、坡度平缓；离所有出生点保持最小距离；
    彼此保持最小间距。返回 dict(name -> [(cx, cy), ...])。
    """
    H, W = z.shape
    gy, gx = np.gradient(z)
    slope = np.sqrt(gx ** 2 + gy ** 2)
    _yy, _xx = np.mgrid[0:H, 0:W]
    d_edge = np.minimum.reduce([_xx - BORDER, _yy - BORDER,
                                (GRID - 1 - BORDER) - _xx, (GRID - 1 - BORDER) - _yy]).astype(np.float64)
    ok = land & (z > WATER_Z + 1.0) & (slope < SLOPE_IMPASSABLE) & (d_edge > 6.0)
    # 矿脉必须落在真正平整的地方：坡度很小，且离山脚（陡坡/山体）有一段距离
    flat_slope = float(T.get('ore_flat_slope', 4.0))
    mount_clear = max(float(T.get('ore_mount_clearance', 24.0)), ORE_CLEAR_RADIUS + 4.0)
    lev = list(T.get('levels') or [])
    if lev:
        # 分层地形：把"层边界"当作山脚/陡坎（层间缓坡带很宽，用坡度判会误伤大量平地）
        lvlidx = land_layer_index(z, T)
        steep = ((lvlidx != np.roll(lvlidx, 1, 0)) | (lvlidx != np.roll(lvlidx, -1, 0))
                 | (lvlidx != np.roll(lvlidx, 1, 1)) | (lvlidx != np.roll(lvlidx, -1, 1)))
    else:
        lvlidx = np.full(z.shape, -1, dtype=np.int16)
        steep = slope > float(T.get('ore_steep_slope', 10.0))
    d_steep = distance_transform(steep)
    # 矿口净空：沿某个斜向 ORE_MOUTH_CLEAR 格内层号一致（矿口朝外可建造）
    DIAG = ((1, 1), (1, -1), (-1, 1), (-1, -1))
    open_m = []
    for _dx, _dy in DIAG:
        _m = np.ones(z.shape, dtype=bool)
        for _k in range(1, int(ORE_MOUTH_CLEAR) + 1):
            # roll 的正向语义是 out[i]=arr[i-s]，取负才是 +d 方向
            _m &= (lvlidx == np.roll(lvlidx, (-_dy * _k, -_dx * _k), axis=(0, 1)))
        _m &= (d_edge > ORE_MOUTH_CLEAR + 4.0)
        open_m.append(_m)
    open_any = open_m[0] | open_m[1] | open_m[2] | open_m[3]
    # 矿位还要离河道有净距，免得矿场平台把河堵死/被河切掉
    riv_w = float(T['river'][0])
    riv_bed = float(T['river'][4])
    chan = np.zeros_like(land)
    for _n, _pts, _b in RIVERS:
        chan |= distance_transform(polyline_mask(geo, _pts, width=riv_w)) < riv_bed + 2.0
    d_riv = distance_transform(chan)
    ok_ore = ok & (slope < flat_slope) & (d_steep >= mount_clear) \
        & (d_riv >= float(T.get('ore_river_clearance', 22.0)))
    if ok_ore.sum() < 200:                       # 太平的地图兜底：放宽到"较平"
        ok_ore = ok & (slope < flat_slope * 2.0) & (d_steep >= mount_clear * 0.5)
    # 贴山脚：矿心到层边界 1~ORE_FOOT_MAX 格，且矿口方向有净空
    ok_foot = (ok & (slope < flat_slope) & (d_riv >= float(T.get('ore_river_clearance', 22.0)))
               & (d_steep >= ORE_FOOT_MIN) & (d_steep <= ORE_FOOT_MAX) & open_any)
    # 中间档：放宽到更宽的坡脚范围（仍贴山脚、仍要求矿口净空）
    ok_foot_loose = (ok & (slope < flat_slope * 2.0)
                     & (d_steep >= max(3.0, ORE_FOOT_MIN - 3.0)) & (d_steep <= ORE_FOOT_MAX + 12.0)
                     & open_any & (d_riv >= float(T.get('ore_river_clearance', 22.0)) * 0.8))
    if verbose:
        print('  [矿位] 平整且离山脚>=%.0f格: %d 格；贴山脚(%d~%d格)+矿口有净空: %d 格' % (
            mount_clear, int(ok_ore.sum()), int(ORE_FOOT_MIN), int(ORE_FOOT_MAX), int(ok_foot.sum())))
    base_list = list(base_cells.values())
    seats_all = city_centers(geo)

    # 矿优先级最高：其他资源与“任何已放置点（含矿）”的间距至少是矿区建造范围 + 余量，
    # 这样矿的建造范围内不会出现油井/车库/机场/医院/精兵学院/雷达等任何资源。
    other_spacing = max(RESOURCE_MIN_SPACING, ORE_CLEAR_RADIUS + 2.0)

    def resolve(cx, cy, base_clear, spacing, placed, max_search=70, mask=None):
        okm = ok if mask is None else mask
        ix, iy = int(round(cx)), int(round(cy))
        best, bd = None, 1e18
        for r in range(0, max_search + 1):
            for dy in range(-r, r + 1):
                for dx in range(-r, r + 1):
                    if max(abs(dx), abs(dy)) != r:
                        continue
                    x, y = ix + dx, iy + dy
                    if not (0 <= x < W and 0 <= y < H) or not okm[y, x]:
                        continue
                    if any(math.hypot(x - bx, y - by) < base_clear for bx, by in base_list):
                        continue
                    if any(math.hypot(x - px, y - py) < spacing for px, py in placed):
                        continue
                    d = dx * dx + dy * dy
                    if d < bd:
                        bd, best = d, (x, y)
            if best is not None:
                break
        if best is None:                      # 放不下就返回 None。不再把间距乘 0.92，也不换更宽的掩码。
            if verbose:
                print('  [资源] (%d,%d) 附近找不到满足间距的位置' % (ix, iy))
            return None
        if verbose and (abs(best[0] - ix) > 2 or abs(best[1] - iy) > 2):
            print('  [资源] (%d,%d) -> (%d,%d)' % (ix, iy, best[0], best[1]))
        placed.append(best)
        return best

    placed = []
    # 矿脉：强制落在平整、且离山脚有距离的平地（ok_ore 掩码）。
    # 若严格掩码内找不到落点，逐级放宽（平整+净距 -> 干燥陆地 -> 任意陆地），
    # 绝不允许退回水里或河上。
    def _ray_score(ix, iy, dx, dy, reach=26):
        """沿 (dx,dy) 方向的可建造净空得分：层号一致 / 水面(岸边矿可建在水上) / 不越界，
        遇到陡坎或层界就停。分数越高 = 这个方向越"开阔"。"""
        s = 0.0
        z0 = float(z[iy, ix])
        for k in range(1, reach + 1):
            x, y = ix + dx * k, iy + dy * k
            if not (0 <= x < W and 0 <= y < H):
                break
            zk = float(z[y, x])
            if steep[y, x]:
                break                                   # 撞到陡坎，方向到此为止
            if abs(zk - z0) < 1.5:
                s += 1.0                                # 同层（陆上开阔）
            elif zk < WATER_Z and z0 < WATER_Z:
                s += 1.0                                # 同在水面
            else:
                s += 0.25                               # 跨层/跨水陆：勉强算一点净空
        return s

    def mouth_angle(ix, iy, toward=None):
        """矿口方向（4 个斜向之一）：
           - 给了 `toward=(bx,by)`（基地旁的矿）：**朝基地**（矿场靠基地那一侧摆放）；
           - 否则（层界上 / 水陆交汇处的矿）：**朝最开阔的方向**，而不是朝交汇处。
        """
        if toward is not None:
            tx, ty = float(toward[0]) - ix, float(toward[1]) - iy
            n = math.hypot(tx, ty) or 1.0
            tx, ty = tx / n, ty / n
            cand = [(tx * dx + ty * dy, i) for i, (dx, dy) in enumerate(DIAG)]
            cand.sort(reverse=True)
            dx, dy = DIAG[cand[0][1]]
            return math.atan2(dy, dx)
        # ---- 朝开阔处：优先"层内净空成立"的斜向，再按射线得分排序
        scored = []
        for i, (dx, dy) in enumerate(DIAG):
            s = _ray_score(ix, iy, dx, dy)
            if open_m[i][iy, ix]:
                s += 6.0                                # 该斜向 22 格内层号一致 = 最理想
            scored.append((s, -i, i))
        scored.sort(reverse=True)
        dx, dy = DIAG[scored[0][2]]
        return math.atan2(dy, dx)

    ore, ore_ang = [], []
    _fallback = []
    _slots, _order = ore_slots(geo, base_cells=base_cells)    # 近矿（每基地前 2 个）要落在基地建造范围内 -> 就坐在基地大平原上，不要求"贴山脚"，
    # 并用"基地建造范围"作硬约束（实测社区图：出生点到最近矿脉 p50=331 / p90=432 单位）
    build_range = float(T.get('base_build_range', 45.0))
    base_xy = list(base_cells.values())
    # 新规则：不放山脚、不放水陆交界；并要求周围有足够开阔空间
    _foot_clear = float(T.get('ore_foot_clear', 10.0))     # 离陡坎/山脚至少这么多格
    _wat_clear = float(T.get('ore_water_clear', 9.0))      # 离水域至少这么多格
    _open_min = float(T.get('ore_open_min', 14.0))         # 周围可通行半径下限
    # 到"不可放矿处"的距离 = 开阔度。注意 distance_transform(mask) 返回"到最近 True 的距离"，
    # 所以要用 distance_transform(~ok) —— 在 ok 内部才是"离边界有多远"（旧代码用 ok 恒为 0）。
    d_imp = distance_transform(~ok)
    d_water = distance_transform(z < WATER_Z)   # 离水域的距离（不在水陆交界放矿）
    ok_near = (ok & (slope < flat_slope) & (d_imp >= _open_min)
               & (d_steep >= _foot_clear) & (d_water >= _wat_clear)
               & (d_imp >= float(T.get('ore_clear_want', 6.0)))
               & (d_riv >= float(T.get('ore_river_clearance', 22.0))))

    ok_near_relax = (ok & (slope < flat_slope * 3.0) & (d_riv >= 12.0)
                     & (d_imp >= _open_min * 0.7) & (d_steep >= _foot_clear * 0.6))
    # ---- 「往开阔地方摆」：开阔度 = 到 不可通行 / 陡坎 / 水 / 层边界 的最小距离（格）。
    #      先算层边界距离（量化后相邻格层号不同处），再取四者的最小值当分数。
    _open_want = float(T.get('ore_open_want', 18.0))       # 期望的开阔半径（格）
    _flat_soft = float(T.get('ore_flat_soft', 3.0))        # 开阔地内部的坡度上限
    # ---- 水域出生点（如水上基地）：允许把它的近矿直接放在水里（用户要求"至少 2 矿"）
    _water_base = None
    for _c in base_cells.values():
        _ix0, _iy0 = int(round(_c[0])), int(round(_c[1]))
        if 0 <= _ix0 < W and 0 <= _iy0 < H and z[_iy0, _ix0] < WATER_Z:
            _water_base = (_ix0, _iy0)
            break
    _water_ok = np.zeros(z.shape, dtype=bool)
    if _water_base is not None and bool(T.get('ore_in_water', True)):
        _wl = distance_transform(z >= WATER_Z)
        _water_ok = (z < WATER_Z) & (_wl >= float(T.get('ore_water_edge_min', 4.0)))
    _lvl_i0 = np.zeros_like(z, dtype=np.int16)
    for _lv0 in sorted([float(v) for v in (T.get('levels') or [LAND_Z])]):
        _lvl_i0 = _lvl_i0 + (z >= _lv0 - 1.0).astype(np.int16)
    _edge0 = np.zeros_like(ok, dtype=bool)
    _edge0[:, 1:] |= (_lvl_i0[:, 1:] != _lvl_i0[:, :-1])
    _edge0[1:, :] |= (_lvl_i0[1:, :] != _lvl_i0[:-1, :])
    d_edge0 = distance_transform(_edge0)
    # ---- 「离水陆交汇处多远」：水陆相邻的格子算交汇带
    _sedge = np.zeros_like(ok, dtype=bool)
    _w0 = (z < WATER_Z)
    for _dy2 in (-1, 0, 1):
        for _dx2 in (-1, 0, 1):
            _sedge |= ((~_w0) & np.roll(np.roll(_w0, _dy2, 0), _dx2, 1))
    d_shore0 = distance_transform(_sedge)
    # 矿位必须**离层界 / 离水陆交汇处都够远**（用户要求：别贴着不同层陆地或岸边）
    _edge_clear = float(T.get('ore_edge_clear', 16.0))
    _shore_clear = float(T.get('ore_shore_clear', 12.0))
    _open_score = np.minimum(np.minimum(d_imp, d_steep), np.minimum(d_water, d_edge0))
    # 矿场 + 矿车通道需要一小块完整平地：要求矿位周围 N 格都在可建地内
    _ore_clear_want = float(T.get('ore_clear_want', 6.0))
    _d_ok = d_imp                                       # 在 ok 内部 = 离可建地边界多远
    _border_clear = float(T.get('ore_border_clear', 30.0))
    _big_open = (ok & (slope < _flat_soft) & (d_imp >= _open_want)
                 & (d_steep >= _open_want) & (d_water >= _wat_clear)
                 & (_d_ok >= _ore_clear_want)
                 & (d_edge >= _border_clear)             # 离可玩边界够远，矿场不压进图边
                 & (d_edge0 >= _edge_clear)              # 离层界够远
                 & (d_shore0 >= _shore_clear)            # 离水陆交汇够远
                 & (d_riv >= float(T.get('ore_river_clearance', 22.0))))
    # 放宽档：把"离层界/离岸"的要求减半，保证偏远区域也能放下
    _road_open = (ok & (slope < _flat_soft * 1.6) & (d_imp >= _open_want * 0.6)
                  & (d_steep >= _open_want * 0.35) & (d_water >= _wat_clear)
                  & (_d_ok >= _ore_clear_want * 0.6)
                  & (d_edge0 >= _edge_clear * 0.6)
                  & (d_shore0 >= _shore_clear * 0.6)
                  & (d_riv >= float(T.get('ore_river_clearance', 22.0)) * 0.6))
    if verbose:
        print('  [矿位] 开阔地(开阔度>=%.0f, 离层界>=%.0f, 离岸>=%.0f): %d 格；放宽档 %d 格'
              % (_open_want, _edge_clear, _shore_clear,
                 int(_big_open.sum()), int(_road_open.sum())))
    # 散矿落水是配方开关。开阔度仍然只用 _big_open，水面是另一张掩码，不放宽陆地条件。
    _scatter_water = np.zeros(z.shape, dtype=bool)
    if bool(T.get('ore_scatter_water')):
        _wl_sc = distance_transform(z >= WATER_Z)
        _scatter_water = ((z < WATER_Z)
                          & (_wl_sc >= float(T.get('ore_scatter_shore', 32.0)))
                          & (d_edge >= _border_clear))
    _scatter_mask = _big_open | _scatter_water
    if verbose and bool(T.get('ore_scatter_water')):
        print('  [矿位] 外围矿可落水：开阔水面 %d 格' % int(_scatter_water.sum()))

    def resolve_open(cx, cy, base_clear, spacing, placed, max_search, mask, sector=None,
                     min_base_dist=None):
        """在 (cx,cy) 周围 max_search 内挑一个合法格。
           sector=(bx, by, r_lo, r_hi, ang_lo, ang_hi)：限制在"以基地为中心的扇区+半径环"里，
           让同一基地的 4 个矿天然分处 4 个斜向、彼此不重叠（否则 46 格间距会互相挡掉）。
           min_base_dist=(bx, by, d)：还要离该基地 >= d 格（**别贴基地、避免卡住建造场**）。
           排序：开阔度最大 -> 离目标格最近 -> 离环中心半径最近。"""
        ix, iy = int(round(cx)), int(round(cy))
        best, bk = None, None
        for r in range(0, max_search + 1):
            for dy in range(-r, r + 1):
                for dx in range(-r, r + 1):
                    if max(abs(dx), abs(dy)) != r:
                        continue
                    x, y = ix + dx, iy + dy
                    if not (0 <= x < W and 0 <= y < H) or not mask[y, x]:
                        continue
                    if any(math.hypot(x - bx, y - by) < base_clear for bx, by in base_list):
                        continue
                    if min_base_dist is not None:
                        _mbx, _mby, _mbd = min_base_dist
                        if math.hypot(x - _mbx, y - _mby) < _mbd:
                            continue
                    if any(math.hypot(x - px, y - py) < spacing for px, py in placed):
                        continue
                    if sector is not None:
                        bx0, by0, r_lo, r_hi, a_lo, a_hi = sector
                        dd = math.hypot(x - bx0, y - by0)
                        if not (r_lo <= dd <= r_hi):
                            continue
                        aa = math.atan2(y - by0, x - bx0)
                        if not (a_lo <= aa <= a_hi):
                            continue
                    key = (float(_open_score[y, x]), -math.hypot(dx, dy))
                    if bk is None or key > bk:
                        bk, best = key, (x, y)
            if best is not None and r >= 4:
                break
        if best is None:
            return None
        placed.append(best)
        return best

    # 水上基地的近矿只落在水里。陆上基地找不到同层陆地时，可以落到近矿环里的水面。
    _wl_near = distance_transform(z >= WATER_Z).astype(np.float64)
    _sea_gy, _sea_gx = np.gradient(_wl_near)
    _ramp_box = ramp_cover_mask(z.shape, ramps or [])

    def _take_near_water(bx, by, placed_):
        """近矿环上挑一格水面：离岸、离图边、不压斜坡，也不进别的基地的近矿环。偏向海的一侧。"""
        r_lo = max(float(T.get('ore_near_min_r', 44.0)), 46.0)
        r_hi = min(float(T.get('ore_near_max_r', 62.0)) - 4.0, 58.0)
        shore_need = float(T.get('ore_shore_clear', 20.0))
        border_need = float(T.get('ore_border_clear', 30.0))
        gap = float(T.get('ore_near_max_r', 62.0)) + 1.0
        ix0, iy0 = int(round(bx)), int(round(by))
        sx = float(_sea_gx[iy0, ix0])
        sy = float(_sea_gy[iy0, ix0])
        sn = math.hypot(sx, sy) or 1.0
        sx, sy = sx / sn, sy / sn
        best = None
        for rr in range(int(math.ceil(r_lo)), int(math.floor(r_hi)) + 1):
            steps = max(36, rr * 3)
            for k in range(steps):
                ang = 2.0 * math.pi * k / steps
                x = int(round(bx + math.cos(ang) * rr))
                y = int(round(by + math.sin(ang) * rr))
                if not (0 <= x < W and 0 <= y < H):
                    continue
                if z[y, x] >= WATER_Z or _wl_near[y, x] < shore_need:
                    continue
                if d_edge[y, x] < border_need or _ramp_box[y, x]:
                    continue
                if any(math.hypot(x - ox, y - oy) < gap
                       for ox, oy in base_xy
                       if math.hypot(ox - bx, oy - by) > 1.0):
                    continue
                if any(math.hypot(x - px, y - py) < float(ORE_MIN_SPACING)
                       for px, py in placed_):
                    continue
                dot = math.cos(ang) * sx + math.sin(ang) * sy
                key = (dot, float(_wl_near[y, x]), -abs(rr - 52))
                if best is None or key > best[0]:
                    best = (key, (x, y))
        if best is None:
            return None
        placed_.append(best[1])
        return best[1]

    _nslot = 2                                   # 每个基地只留这一对近矿槽
    for _oi, (cx, cy) in enumerate(_slots):
        is_base_ore = _oi < _nslot * len(BASES)
        near = is_base_ore and (_oi % _nslot) < int(T.get('ore_near_max', 2))
        # 陆上基地的近矿先找同层陆地，找不到再找近矿环里的水。水上基地的近矿只走水面。
        _scatter_on = (not near) and bool(T.get('ore_scatter_water'))
        cell = None
        _locked = False
        _wet_near = None
        if near and is_base_ore:
            bx, by = base_xy[(_oi // _nslot) % len(base_xy)]
            _ixb, _iyb = int(round(bx)), int(round(by))
            if 0 <= _ixb < W and 0 <= _iyb < H and z[_iyb, _ixb] < WATER_Z:
                _locked = True
                cell = _take_near_water(bx, by, placed)
                if cell is not None and verbose:
                    print('    [矿位] 近矿落在水下 (%d,%d) 距基地 %.0f 离岸 %.0f'
                          % (cell[0], cell[1], math.hypot(cell[0] - bx, cell[1] - by),
                             float(_wl_near[cell[1], cell[0]])))
        if (not _locked) and (bool(T.get('ore_open_rank')) or bool(ore_open_rank)):
            # ---- 新口径：往开阔地方摆；基地的 4 个矿按"斜向扇区 + 半径环"分头找，
            #      天然分处 4 个方向、彼此不重叠（不会因为 46 格间距互相挡掉）。
            #      **基地旁只留 2 个**（用户要求）：近矿 0/1；远矿 2/3 至少离基地
            #      `ore_far_min_dist`，并朝地图中部方向拉一点（"往中间/下一层摆"）。
            _sector = None
            _far = False
            if is_base_ore:
                bx, by = base_xy[(_oi // _nslot) % len(base_xy)]
                _k = _oi % _nslot
                _dirs = ((-1, -1), (1, 1), (-1, 1), (1, -1))       # 4 个象限，按索引轮换
                _dxq, _dyq = _dirs[(_k + (_oi // _nslot)) % 4]
                _base_ang = math.atan2(_dyq, _dxq)
                _near_max = int(T.get('ore_near_max', 2))
                _far = (_k >= _near_max)
                if _far:
                    # 远矿：先把目标点朝地图中部收缩，再按"离基地 >= ore_far_min_dist"找
                    _pull = float(T.get('ore_far_pull_mid', 0.45))
                    cx = cx * (1.0 - _pull) + (GRID / 2.0) * _pull
                    cy = cy * (1.0 - _pull) + (GRID / 2.0) * _pull
                    _dmin = float(T.get('ore_far_min_dist', 75.0))
                    _dmax = float(T.get('ore_far_max_dist', 0.0))
                    if _dmax <= _dmin:
                        _dmax = max(_dmin + 60.0, 170.0)
                    if _scatter_on:
                        # 同一基地的两个远矿分在两圈上，避免叠在一起。
                        if (_k % 2) == 0:
                            _r_lo, _r_hi = _dmin, min(_dmax, _dmin + 70.0)
                        else:
                            _r_lo = min(_dmax - 40.0, _dmin + 80.0)
                            _r_hi = _dmax
                            if _r_lo < _dmin:
                                _r_lo = _dmin
                    else:
                        _r_lo, _r_hi = _dmin, _dmax
                    _sector = (bx, by, _r_lo, _r_hi,
                               _base_ang - math.pi / 3.2, _base_ang + math.pi / 3.2)
                else:
                    # 近矿：区间 [ore_near_min_r, ore_near_max_r]。
                    # **下界很重要** —— 贴着基地摆会占掉建造场的位置（基地"卡住"矿场）。
                    _r0 = float(T.get('ore_near_min_r', 44.0))
                    _r1 = float(T.get('ore_near_max_r', 62.0))
                    _sector = (bx, by, _r0, _r1,
                               _base_ang - math.pi / 3.6, _base_ang + math.pi / 3.6)
            if _scatter_on and _far:
                _srch_main = 280
            elif _far:
                _srch_main = 180
            elif _scatter_on:
                _srch_main = 160
            else:
                _srch_main = 90
            _msk_main = _scatter_mask if _scatter_on else _big_open
            if near and not _scatter_on:
                _bxi, _byi = int(round(bx)), int(round(by))
                _bl = int(lvlidx[_byi, _bxi]) if (0 <= _bxi < W and 0 <= _byi < H) else -1
                if _bl >= 0 and z[_byi, _bxi] >= WATER_Z:
                    _msk_main = _big_open & (lvlidx == _bl) & (z >= WATER_Z)
                    _wet_near = ((z < WATER_Z)
                                 & (_wl_near >= float(T.get('ore_shore_clear', 12.0)))
                                 & (d_edge >= _border_clear)
                                 & ~_ramp_box)
            _sp_main = (float(T.get('ore_scatter_spacing', ORE_MIN_SPACING))
                        if _scatter_on else float(ORE_MIN_SPACING))
            # 外围矿要离开所有基地的近矿环，避免被算进别人的两处近矿。
            _clear_main = ((float(T.get('ore_near_max_r', 62.0)) + 20.0)
                           if _scatter_on else float(ORE_BASE_CLEARANCE))
            c2 = resolve_open(cx, cy, _clear_main, _sp_main, placed,
                              int(_srch_main), _msk_main, sector=_sector)
            if c2 is not None:
                cell = c2
            # 远矿放宽：仍找不到就退到更宽的环。散矿落水时不再收回到近矿环，也不退回陆地。
            if cell is None and _far and _scatter_on:
                _bx2, _by2 = base_xy[(_oi // _nslot) % len(base_xy)]
                _fb = float(T.get('ore_near_max_r', 62.0)) + 8.0
                _dmax2 = float(T.get('ore_far_max_dist', 0.0))
                if _dmax2 <= _fb:
                    _dmax2 = max(_fb + 60.0, 200.0)
                _sector2 = (_bx2, _by2, _fb, _dmax2, -math.pi, math.pi)
                cell = resolve_open(cx, cy, _clear_main, _sp_main, placed,
                                    280, _scatter_mask, sector=_sector2)
            elif cell is None and _far:
                _bx2, _by2 = base_xy[(_oi // _nslot) % len(base_xy)]
                _sector2 = (_bx2, _by2, float(T.get('ore_far_fallback', 55.0)),
                            200.0, -math.pi, math.pi)
                cell = resolve_open(cx, cy, ORE_BASE_CLEARANCE, ORE_MIN_SPACING, placed,
                                    200, _big_open, sector=_sector2)
            # 外围槽若靠近水上基地，仍可落到那片水面。近矿不走这里。
            _mind = (bx, by, float(T.get('ore_near_min_r', 44.0))) if is_base_ore else None
            if cell is None and not _scatter_on and not near and _water_ok.any() and _water_base is not None:
                _wbx, _wby = _water_base
                if math.hypot(cx - _wbx, cy - _wby) <= float(T.get('ore_mid_max', 120.0)):
                    c2 = resolve_open(cx, cy, 0.0, ORE_MIN_SPACING, placed, 90, _water_ok,
                                      min_base_dist=_mind)
                    if c2 is not None:
                        cell = c2
                        print('    [矿位] 水上基地附近的矿落在水域 (%d,%d)' % (c2[0], c2[1]))
            if cell is None and near and _wet_near is not None:
                # 近矿不落到别的陆地层。同层放不下，再在近矿环里找水。
                _ring = (bx, by,
                         float(T.get('ore_near_min_r', 44.0)),
                         float(T.get('ore_near_max_r', 62.0)),
                         -math.pi, math.pi)
                c3 = resolve_open(bx, by, ORE_BASE_CLEARANCE, ORE_MIN_SPACING, placed,
                                  80, _msk_main, sector=_ring)
                if c3 is None:
                    c3 = resolve_open(bx, by, ORE_BASE_CLEARANCE, ORE_MIN_SPACING, placed,
                                      80, _wet_near, sector=_ring)
                    if c3 is not None and verbose:
                        print('    [矿位] 陆上基地近矿落在范围内水域 (%d,%d) 距基地 %.0f'
                              % (c3[0], c3[1], math.hypot(c3[0] - bx, c3[1] - by)))
                if c3 is not None:
                    cell = c3
            elif cell is None and not _scatter_on:
                # 限制半径内找不到 -> 全图挑一个最开阔的合法点（绝不放坡上/水里）
                c3 = resolve_open(cx, cy, ORE_BASE_CLEARANCE, ORE_MIN_SPACING, placed,
                                  520, _big_open, min_base_dist=_mind)
                if c3 is not None:
                    cell = c3
        elif (not _locked) and is_base_ore:
            # 每名玩家的矿都限制在基地附近。近矿只留在和基地同一陆地层。
            bx, by = base_xy[(_oi // _nslot) % len(base_xy)]
            cap = build_range if near else float(T.get('ore_mid_max', 120.0))
            _mask_n = ok_near
            if near:
                _bxi, _byi = int(round(bx)), int(round(by))
                if (0 <= _bxi < W and 0 <= _byi < H and z[_byi, _bxi] >= WATER_Z
                        and int(lvlidx[_byi, _bxi]) >= 0):
                    _mask_n = ok_near & (lvlidx == int(lvlidx[_byi, _bxi]))
            for mask in (_mask_n,):
                c2 = resolve(cx, cy, ORE_BASE_CLEARANCE, ORE_MIN_SPACING, placed,
                             max_search=int(cap), mask=mask)
                if c2 is None:
                    continue
                if math.hypot(c2[0] - bx, c2[1] - by) <= cap:
                    cell = c2
                    break
                placed.pop()
            if cell is None:
                cell = resolve(bx, by, ORE_BASE_CLEARANCE, ORE_MIN_SPACING, placed,
                               max_search=max(24, int(cap)), mask=_mask_n)
        elif not _locked:
            for mask, search in ((ok_ore, 120), (ok_near, 90)):
                cell = resolve(cx, cy, ORE_BASE_CLEARANCE, ORE_MIN_SPACING, placed,
                               max_search=search, mask=mask)
                if cell is None:
                    continue
                ix, iy = int(round(cx)), int(round(cy))
                if (cell[0], cell[1]) != (ix, iy) or (0 <= ix < W and 0 <= iy < H and mask[iy, ix]):
                    break
                placed.pop()
        if cell is None and not _scatter_on and not _locked and not near:
            cell = resolve(cx, cy, ORE_BASE_CLEARANCE, ORE_MIN_SPACING, placed,
                           max_search=60, mask=ok_near)
        if cell is None:
            if verbose:
                print('  [矿位] 槽位 %d 放不下，跳过' % _oi)
            continue
        ore.append(cell)
        # ---- 矿口朝向（用户规则）：
        #      · 基地旁的矿（每基地前 2 个）-> **朝基地**
        #      · 层界上 / 水陆交汇处的矿 -> **朝开阔的那一侧**（不朝交汇处）
        _toward = None
        if is_base_ore and (_oi % _nslot) < int(T.get('ore_near_max', 2)):
            _toward = base_xy[(_oi // _nslot) % len(base_xy)]
        _a = mouth_angle(int(round(cell[0])), int(round(cell[1])), toward=_toward)
        ore_ang.append(_a)
        if verbose and _toward is not None:
            print('    [矿口] 基地旁矿 (%d,%d) 朝基地 (%.0f,%.0f)'
                  % (cell[0], cell[1], _toward[0], _toward[1]))
    # ---- 通用保底：**每个出生点附近至少 2 个矿**。
    #      陆上基地先补同层陆地，再补近矿范围内的水面；水上基地只补水面。
    _min_ore = int(T.get('base_min_ore', 2))
    _ore_rng = float(T.get('base_ore_range', 45.0))
    for _bn, _c in base_cells.items():
        _ix, _iy = int(round(_c[0])), int(round(_c[1]))
        if not (0 <= _ix < W and 0 <= _iy < H):
            continue
        _on_water = bool(z[_iy, _ix] < WATER_Z)
        _have = sum(1 for o in ore if math.hypot(o[0] - _ix, o[1] - _iy) <= _ore_rng)
        _need = _min_ore - _have
        if _need <= 0:
            continue
        # 候选：水上基地 -> 水面；陆上基地 -> 同层陆地，其次近矿范围内的水面
        _d_w = distance_transform(z < WATER_Z)
        _d_l = distance_transform(z >= WATER_Z)
        if _on_water:
            _masks = [(z < WATER_Z) & (_d_l >= 2.0)]
        else:
            _bl = int(lvlidx[_iy, _ix])
            _near_max = float(T.get('ore_near_max_r', 62.0))
            _yy, _xx = np.mgrid[0:H, 0:W]
            _dd = np.hypot(_xx - _ix, _yy - _iy)
            _same = ((lvlidx == _bl) & (z >= WATER_Z) & (_bl >= 0)
                     & (_d_w >= float(T.get('ore_water_clear', 4.0))))
            _wet = ((z < WATER_Z) & (_d_l >= float(T.get('ore_shore_clear', 12.0)))
                    & (_dd <= _near_max))
            _masks = [_same, _wet] if _bl >= 0 else [
                (z >= WATER_Z) & (_d_w >= float(T.get('ore_water_clear', 4.0)))]
        _added = 0
        for _k in range(_need):
            _c2 = None
            for _cand in _masks:
                _c2 = resolve_open(_ix, _iy, ORE_BASE_CLEARANCE, ORE_MIN_SPACING, placed,
                                   int(_ore_rng) + 25, _cand,
                                   min_base_dist=(_ix, _iy, float(T.get('ore_near_min_r', 44.0))))
                if _c2 is not None:
                    break
            if _c2 is None:
                break
            ore.append(_c2)
            ore_ang.append(math.pi / 4)
            _added += 1
        if verbose and _added:
            print('    [矿位] %s 保底补 %d 个矿（原有 %d 个，水面基地=%s）'
                  % (_bn, _added, _have, _on_water))
    # 近矿之后、外围矿之前：每个出生点一处油井。后面的矿和城市油井让开这些落点。
    # ---- 油井、哨站和中部建筑落在平整处，并避开层界、岸线、斜坡、不可通行。
    _flat_tol = float(T.get('build_flat_slope', 2.5))          # 允许的自然坡度上限
    _build_ok = (ok & (slope < _flat_tol)
                 & (d_steep >= float(T.get('build_foot_clear', 8.0)))
                 & (d_water >= float(T.get('build_water_clear', 4.0))))
    _lvl_i = np.zeros_like(z, dtype=np.int16)
    for _lv in sorted([float(v) for v in (T.get('levels') or [LAND_Z])]):
        _lvl_i = _lvl_i + (z >= _lv - 1.0).astype(np.int16)
    _edge = np.zeros_like(_build_ok, dtype=bool)
    _edge[:, 1:] |= (_lvl_i[:, 1:] != _lvl_i[:, :-1])
    _edge[1:, :] |= (_lvl_i[1:, :] != _lvl_i[:-1, :])
    _w0 = z < WATER_Z
    _shore = np.zeros(z.shape, dtype=bool)
    for _dy in (-1, 0, 1):
        for _dx in (-1, 0, 1):
            _shore |= _w0 != np.roll(np.roll(_w0, _dy, 0), _dx, 1)
    # 斜坡按挖出来的坡面算，不只看轴对齐框。建筑中心离开坡面 build_foot_clear 格，
    # 和离开层界同一距离，整座建筑不压在坡上。
    _ramp_body = ramp_body_mask(z.shape, ramps or [], z)
    _foot = float(T.get('build_foot_clear', 8.0))
    _on_ramp = (distance_transform(_ramp_body) < _foot) if _ramp_body.any() else np.zeros(z.shape, dtype=bool)
    _impass = (slope > SLOPE_IMPASSABLE) | (z > WATER_Z + PEAK_IMPASSABLE)
    _avoid = _edge | _shore | _on_ramp | _impass
    _build_ok &= (distance_transform(_edge) >= 3.0) & ~_avoid
    # 中立资源落水也是配方开关。开阔水面可以用，岸线、斜坡和不可通行仍然避开。
    _neutral_ok = _build_ok
    _open_water = ((z < WATER_Z) & ~_avoid
                   & (d_edge >= float(T.get('ore_border_clear', 30.0))))
    if bool(T.get('neutral_in_water')):
        _sea_pad = (_open_water
                    & (distance_transform(z >= WATER_Z) >= float(T.get('neutral_water_edge', 12.0))))
        _neutral_ok = _build_ok | _sea_pad
        if verbose:
            print('  [资源] 中立设施可落水：开阔水面 %d 格' % int(_sea_pad.sum()))

    derricks = []
    for idx, (lon, lat, name, team, kind) in enumerate(BASES):
        bx, by = base_cells[name]
        cell = resolve(bx, by, RESOURCE_PAD_CLEARANCE, other_spacing, placed, mask=_neutral_ok)
        # 水上出生点：L1~L3 找出来的往往是很远的陆上位置（>70 格），
        # 这里**优先在水面就近放**，放不下才用陆上那个。
        if z[int(round(by)), int(round(bx))] < WATER_Z:
            _sea_ok = (z < WATER_Z) & ~_avoid
            # 注意：resolve 是"从中心按环向外扫"，中心就是基地 -> 永远先撞到基地自己那格，
            # 又被 base_list 的净空挡掉 -> 必然返回 None。所以要**从离基地 25 格的方向点起搜**。
            _wc = None
            for _band in ((0.0, 90.0), (0.0, 140.0)):
                for _ang in [(idx * 1.7 + k * 0.9) for k in range(8)]:
                    _sx = bx + math.cos(_ang) * 25.0
                    _sy = by + math.sin(_ang) * 25.0
                    for _sp in (other_spacing,):
                        _try = resolve(_sx, _sy, 0.0, _sp, placed,
                                       max_search=int(_band[1]), mask=_sea_ok)
                        if _try is not None:
                            _wc = _try
                            break
                    if _wc is not None:
                        break
                if _wc is not None:
                    break
            if _wc is not None:
                _dw = math.hypot(_wc[0] - bx, _wc[1] - by)
                _dl = math.hypot(cell[0] - bx, cell[1] - by) if cell is not None else 1e9
                if _dw < _dl:
                    cell = _wc
                    if verbose:
                        print('    [油井] %s 是水上基地，油井落水面 (%d,%d) 距基地 %.0f 格'
                              % (name, _wc[0], _wc[1], _dw))
        if cell is None:
            if verbose:
                print('    [油井] 出生点 %s 附近放不下，跳过' % name)
            continue
        derricks.append(cell)
    _spawn_n = len(derricks)
    if verbose:
        print('  [油井] 出生点油井 %d 个' % _spawn_n)
    # 近矿和近处油井已经放好。其余矿按玩家总数撒开，陆地和水面各一半。
    _n_outer = int(T.get('ore_outer_per_player', 4)) * len(base_list)
    _ramp_ore = ramp_cover_mask(z.shape, ramps or [])
    _land_outer = _big_open & ~_ramp_ore
    _water_outer = ((z < WATER_Z) & ~_ramp_ore
                    & (distance_transform(z >= WATER_Z) >= 4.0)
                    & (d_edge >= float(T.get('ore_border_clear', 30.0))))
    _away = float(T.get('ore_near_max_r', 62.0)) + 20.0
    _sp_sc = float(T.get('ore_scatter_spacing', ORE_MIN_SPACING))
    _step = 28

    def _fill_outer(mask, n):
        added = 0
        while added < n:
            best, best_d = None, _sp_sc - 1e-6
            for _y in range(BORDER + 4, H - BORDER - 4, _step):
                row = mask[_y]
                for _x in range(BORDER + 4, W - BORDER - 4, _step):
                    if not row[_x]:
                        continue
                    if any(math.hypot(_x - bx, _y - by) < _away for bx, by in base_list):
                        continue
                    md = 1e9
                    _blocked = False
                    for p in placed:
                        if math.hypot(_x - p[0], _y - p[1]) < other_spacing:
                            _blocked = True
                            break
                    if _blocked:
                        continue
                    for p in ore:
                        d0 = math.hypot(_x - p[0], _y - p[1])
                        if d0 < _sp_sc:
                            _blocked = True
                            break
                        if d0 < md:
                            md = d0
                    if _blocked:
                        continue
                    if md > best_d:
                        best_d, best = md, (_x, _y)
            if best is None:
                break
            ore.append(best)
            placed.append(best)
            ore_ang.append(math.pi / 4)
            added += 1
        return added

    if base_list and _n_outer > 0 and not T.get('ore_bands'):
        _wq = _n_outer // 2
        _lq = _n_outer - _wq
        _aw = _fill_outer(_water_outer, _wq)
        _al = _fill_outer(_land_outer, _lq + (_wq - _aw))
        _left = _n_outer - _aw - _al
        if _left > 0:
            _aw += _fill_outer(_water_outer, _left)
        if verbose:
            print('  [矿位] 外围矿 %d 个（水面 %d、陆地 %d，目标 %d）'
                  % (_aw + _al, _aw, _al, _n_outer))
    if verbose and _fallback:
        print('  [矿位] %d 个矿只能退回目标格（周围没有满足间距的落点）' % len(_fallback))
    # ---- 油井：只放在配方点落进去的块上。自治州、自治县和落在水上的不算。
    #      CITY_CLUSTER 打开时，纹理圆相接的块合成一片：单独一片一口油井加一栋小房子，
    #      多座挨着则两栋房子在两侧、两口油井在中间。没开的图仍是每座一块一口油井。
    _seat_names = set(_seat_lookup(geo))
    houses = []
    house_kinds = []
    _cityd = []
    for _n, tx, ty in (seats_all or []):
        if _n not in _seat_names:
            continue
        if '自治州' in _n or '自治县' in _n:
            continue
        _ix, _iy = int(round(tx)), int(round(ty))
        if not (0 <= _ix < GRID and 0 <= _iy < GRID):
            continue
        if z[_iy, _ix] < WATER_Z:
            if verbose:
                print('    [油井] 地级市 %s 在本图是水域，跳过' % _n)
            continue
        _cityd.append((_n, tx, ty))
    if bool(getattr(cfg, 'CITY_CLUSTER', False)):
        _radius = float(getattr(cfg, 'CITY_TEX_R', 30.0))
        _touch = _radius * 2.0 + 0.5
        _inner = 16.0
        _parent = list(range(len(_cityd)))

        def _find(i, _parent=_parent):
            while _parent[i] != i:
                _parent[i] = _parent[_parent[i]]
                i = _parent[i]
            return i

        for _i in range(len(_cityd)):
            for _j in range(_i + 1, len(_cityd)):
                if math.hypot(_cityd[_i][1] - _cityd[_j][1],
                              _cityd[_i][2] - _cityd[_j][2]) <= _touch:
                    _a, _b = _find(_i), _find(_j)
                    if _a != _b:
                        _parent[_a] = _b
        _groups = {}
        for _i, _item in enumerate(_cityd):
            _groups.setdefault(_find(_i), []).append(_item)
        # 城市圆比资源间距小。油井和房子仍让开矿的建造范围（与后面的矿区清理同一距离），
        # 不要求隔开整整一份资源间距，否则矿落在市区里时整片城市都放不下。
        _city_pts = []

        def _in_union(x, y, centers):
            return any(math.hypot(x - cx, y - cy) <= _radius for cx, cy in centers)

        def _pull(x, y, centers):
            if _in_union(x, y, centers):
                return x, y
            cx, cy = min(centers, key=lambda c: math.hypot(x - c[0], y - c[1]))
            dist = math.hypot(x - cx, y - cy) or 1.0
            keep = max(0.0, _radius - 2.0)
            return cx + (x - cx) / dist * keep, cy + (y - cy) / dist * keep

        def _place(tx, ty, centers, mask):
            ix, iy = int(round(tx)), int(round(ty))
            best, bd = None, 1e18
            reach = int(math.ceil(_radius * 2.0))
            for r in range(0, reach + 1):
                for dy in range(-r, r + 1):
                    for dx in range(-r, r + 1):
                        if max(abs(dx), abs(dy)) != r:
                            continue
                        x, y = ix + dx, iy + dy
                        if not (0 <= x < W and 0 <= y < H) or not mask[y, x]:
                            continue
                        if not _in_union(x, y, centers):
                            continue
                        if any(math.hypot(x - bx, y - by) < RESOURCE_PAD_CLEARANCE
                               for bx, by in base_list):
                            continue
                        if any(math.hypot(x - px, y - py) < float(ORE_CLEAR_RADIUS) + 3.0
                               for px, py in placed):
                            continue
                        if any(math.hypot(x - px, y - py) < _inner
                               for px, py in _city_pts):
                            continue
                        d2 = dx * dx + dy * dy
                        if d2 < bd:
                            bd, best = d2, (x, y)
                if best is not None:
                    break
            if best is not None:
                _city_pts.append(best)
            return best

        def _attempt(specs, centers, mask):
            snap = list(_city_pts)
            got = []
            for x, y, kind in specs:
                cell = _place(x, y, centers, mask)
                if cell is None:
                    _city_pts[:] = snap
                    return None
                got.append((kind, cell))
            return got

        def _partial(specs, centers, mask):
            got = []
            for x, y, kind in specs:
                cell = _place(x, y, centers, mask)
                if cell is not None:
                    got.append((kind, cell))
            return got

        def _run(specs_kind, payload, centers, mask):
            if specs_kind == 'single':
                hx, hy = payload
                got = None
                specs = None
                for ang in (math.pi / 2.0, 0.0, math.pi / 4.0, -math.pi / 4.0):
                    c, s = math.cos(ang), math.sin(ang)
                    off = _radius * 0.45
                    specs = ((hx - c * off, hy - s * off, 'h'),
                             (hx + c * off, hy + s * off, 'd'))
                    got = _attempt(specs, centers, mask)
                    if got is not None:
                        break
                if got is None:
                    got = _partial(specs, centers, mask)
                return got
            return _attempt(payload, centers, mask) or _partial(payload, centers, mask)

        for _group in sorted(_groups.values(), key=lambda g: (g[0][1], g[0][2])):
            _centers = [(g[1], g[2]) for g in _group]
            _names = '、'.join(g[0] for g in _group)
            if len(_group) == 1:
                _kind, _payload = 'single', _centers[0]
            else:
                _bi = _bj = 0
                _bd = -1.0
                for _i in range(len(_group)):
                    for _j in range(_i + 1, len(_group)):
                        _d = math.hypot(_centers[_i][0] - _centers[_j][0],
                                        _centers[_i][1] - _centers[_j][1])
                        if _d > _bd:
                            _bi, _bj, _bd = _i, _j, _d
                ax, ay = _centers[_bi]
                bx, by = _centers[_bj]
                _dx, _dy = bx - ax, by - ay
                _ln = math.hypot(_dx, _dy) or 1.0
                ux, uy = _dx / _ln, _dy / _ln
                px, py = -uy, ux
                _out = _radius * 0.55
                ha = _pull(ax - ux * _out, ay - uy * _out, _centers)
                hb = _pull(bx + ux * _out, by + uy * _out, _centers)
                # 油井落在两端城市之间，并错开到轴线两侧。
                _span = _bd
                d1 = _pull(ax + ux * _span * 0.35 + px * 4.0,
                           ay + uy * _span * 0.35 + py * 4.0, _centers)
                d2 = _pull(ax + ux * _span * 0.65 - px * 4.0,
                           ay + uy * _span * 0.65 - py * 4.0, _centers)
                if math.hypot(d1[0] - d2[0], d1[1] - d2[1]) < _inner:
                    d2 = _pull(ax + ux * _span * 0.65 + px * 4.0,
                               ay + uy * _span * 0.65 + py * 4.0, _centers)
                _kind = 'multi'
                _payload = (ha + ('h',), hb + ('h',), d1 + ('d',), d2 + ('d',))
            _got = _run(_kind, _payload, _centers, _build_ok)
            _nh = _nd = 0
            for _kind, _cell in (_got or []):
                if _kind == 'h':
                    houses.append(_cell)
                    house_kinds.append(len(house_kinds) % 2)
                    _nh += 1
                else:
                    derricks.append(_cell)
                    _nd += 1
            if verbose:
                print('  [城市] %s（%d 座）房子 %d、油井 %d' % (_names, len(_group), _nh, _nd))
        placed.extend(_city_pts)
    else:
        for _n, tx, ty in _cityd:
            cell = resolve(tx, ty, RESOURCE_PAD_CLEARANCE, other_spacing, placed, mask=_neutral_ok)
            if cell is None and verbose:
                print('    [油井] 城市 %s 附近放不下，跳过' % _n)
            if cell is not None:
                derricks.append(cell)
        if verbose:
            print('  [油井] 城市油井 %d 个（陆地内城市 %d 个）' % (len(derricks) - _spawn_n, len(_cityd)))
    _city_derrick_n = len(derricks) - _spawn_n
    if verbose:
        _spawn_n = len(derricks) - _city_derrick_n
        if bool(getattr(cfg, 'CITY_CLUSTER', False)):
            print('  [油井] 合计 %d 个 = 城市区域 %d + 出生点 %d'
                  % (len(derricks), _city_derrick_n, _spawn_n))
        else:
            print('  [油井] 合计 %d 个 = 城市 %d + 出生点 %d'
                  % (len(derricks), len(_cityd), _spawn_n))
    for lon, lat in OUTER_DERRICK:              # 省外散布油井，落在配方点附近
        _c = resolve(*geo.to_cell(lon, lat), RESOURCE_PAD_CLEARANCE,
                     other_spacing, placed, mask=_neutral_ok)
        if _c is not None:
            derricks.append(_c)
    for _pt in (T.get('sea_derrick_cells') or ()):
        _c = resolve(float(_pt[0]), float(_pt[1]), RESOURCE_PAD_CLEARANCE,
                     other_spacing, placed, mask=_neutral_ok)
        if _c is not None:
            derricks.append(_c)
            if verbose:
                print('    [油井] 水域油井 (%d,%d)' % (_c[0], _c[1]))

    _gap = float(T.get('building_gap', 36.0))

    def _meeting():
        teams = {}
        for _b in BASES:
            if _b[2] not in base_cells:
                continue
            teams.setdefault(_b[3], []).append(base_cells[_b[2]])
        cents = [(sum(p[0] for p in pts) / len(pts), sum(p[1] for p in pts) / len(pts))
                 for pts in teams.values() if pts]
        if len(cents) >= 2:
            pair, bd = (0, 1), -1.0
            for i in range(len(cents)):
                for j in range(i + 1, len(cents)):
                    d = math.hypot(cents[i][0] - cents[j][0], cents[i][1] - cents[j][1])
                    if d > bd:
                        pair, bd = (i, j), d
            a, b = cents[pair[0]], cents[pair[1]]
            cx, cy = (a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5
            ax, ay = b[0] - a[0], b[1] - a[1]
        else:
            pts = list(base_cells.values()) or [(GRID * 0.5, GRID * 0.5)]
            cx = sum(p[0] for p in pts) / len(pts)
            cy = sum(p[1] for p in pts) / len(pts)
            ax, ay = 1.0, 0.0
        nrm = math.hypot(ax, ay) or 1.0
        return cx, cy, ax / nrm, ay / nrm

    _mx, _my, _ux, _uy = _meeting()
    _vx, _vy = -_uy, _ux
    _sep = max(_gap, 56.0)

    def _put(tx, ty, mask, search=160):
        return resolve(float(tx), float(ty), RESOURCE_PAD_CLEARANCE, _gap, placed,
                       max_search=search, mask=mask)

    def _pair(dx, dy, dist, mask):
        got = []
        for s in (-1.0, 1.0):
            cell = _put(_mx + dx * dist * s, _my + dy * dist * s, mask)
            if cell is not None:
                got.append(cell)
        return got

    posts = _pair(_ux, _uy, _sep, _build_ok)
    academies = _pair(_vx, _vy, _sep, _build_ok)
    _ring = _sep + _gap + 48.0
    garage, hospitals, airports = [], [], []
    for _ki, _bucket in enumerate((garage, hospitals, airports)):
        _ang0 = _ki * math.pi / 3.0
        for _s in (0.0, 1.0):
            _ang = _ang0 + _s * math.pi
            cell = _put(_mx + math.cos(_ang) * _ring, _my + math.sin(_ang) * _ring, _build_ok)
            if cell is not None:
                _bucket.append(cell)
    _inside = d_edge >= 0.0
    _water_frac = float(((z < WATER_Z) & _inside).sum()) / max(1, int(_inside.sum()))
    shipyards = []
    if _water_frac >= float(T.get('shipyard_water_frac', 0.20)):
        cell = _put(_mx, _my, _open_water, search=520)
        if cell is not None:
            shipyards.append(cell)
            if verbose:
                print('  [船厂] 水域 %.0f%%，放在交汇点附近的水面 (%d,%d)'
                      % (100.0 * _water_frac, cell[0], cell[1]))
        elif verbose:
            print('  [船厂] 水域 %.0f%%，交汇点附近没有可放的开阔水面' % (100.0 * _water_frac))
    elif verbose:
        print('  [船厂] 水域 %.0f%%，不到 20%%，不放' % (100.0 * _water_frac))

    # ---- 矿位定稿后**重算全部矿口朝向**（位置可能被后面的保底/清洗挪动过，
    #      原来在循环里逐個算的角度会与新位置不匹配）。
    #      规则：基地旁（<= ore_near_max_r）朝基地；其余（层界/岸边/中路）朝最开阔处。
    def _final_angles(ore_list, base_cells_):
        _r_max = float(T.get('ore_near_max_r', 62.0))
        angs = []
        for o in ore_list:
            ox, oy = int(round(o[0])), int(round(o[1]))
            # 最近的基地
            bn, bd, bc0 = None, 1e18, None
            for nm, c in (base_cells_ or {}).items():
                d = math.hypot(ox - c[0], oy - c[1])
                if d < bd:
                    bn, bd, bc0 = nm, d, c
            if bc0 is not None and bd <= _r_max:
                angs.append(mouth_angle(ox, oy, toward=(float(bc0[0]), float(bc0[1]))))
            else:
                angs.append(mouth_angle(ox, oy))
        return angs

    ore_ang = _final_angles(ore, base_cells)
    if verbose:
        _nb = sum(1 for o in ore
                  if min(math.hypot(o[0] - c[0], o[1] - c[1])
                         for c in (base_cells or {}).values()) <= float(T.get('ore_near_max_r', 62.0)))
        print('  [矿口] 基地旁 %d 个朝基地、其余 %d 个朝开阔处' % (_nb, len(ore) - _nb))
    if verbose:
        print('  [中部] 哨站 %d、学院 %d、车库 %d、医院 %d、机场 %d、船厂 %d'
              % (len(posts), len(academies), len(garage), len(hospitals),
                 len(airports), len(shipyards)))
    return dict(ore=ore, derricks=derricks, posts=posts, garage=garage,
                airports=airports, hospitals=hospitals, academies=academies,
                shipyards=shipyards,
                ore_angles=ore_ang, houses=houses, house_kinds=house_kinds)


PAD_MAX_DELTA = float(getattr(cfg, 'PAD_MAX_DELTA', 4.0))   # 资源平台只抹平 <= 该幅度的小起伏，保留陡坎


def flatten_pads(z, pads, min_z, xx, yy, verbose=True):
    """按“平台簇”统一高度铺平。

    pads: [(cx, cy, radius, blend, max_z)]；互相影响（影响半径相交）的平台并成一簇，
    整簇用同一高度（各中心高度的中位数），因此簇内处处等高、不会互相啃出斜坡/台阶。
    """
    n = len(pads)
    if not n:
        return z
    parent = list(range(n))

    def find(a):
        while parent[a] != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    def union(a, b):
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[rb] = ra

    for i in range(n):
        xi, yi, ri, bi, _mz, mi, *_ = pads[i]
        for j in range(i + 1, n):
            xj, yj, rj, bj, _mz2, mj, *_ = pads[j]
            d = math.hypot(xi - xj, yi - yj)
            if mi or mj:
                # 只要一方是基地平台：只按“平坦核心”相交才合并（基地与自己的矿场连片，
                # 但两个相距较远的队友基地不会并成巨型平地）
                if d <= ri + rj:
                    union(i, j)
            else:
                # 资源点之间：按影响区相交即合并（相邻资源同高，避免过渡带互相啃出斜坡）
                if d <= (ri + bi) + (rj + bj):
                    union(i, j)

    groups = {}
    for i in range(n):
        groups.setdefault(find(i), []).append(i)

    merged = 0
    for root, idxs in groups.items():
        hs = []
        for i in idxs:
            cx, cy, radius, blend, max_z, is_base = pads[i]
            h = float(z[int(round(cy)), int(round(cx))])
            hs.append(min(max(h, min_z), max_z))
        h = float(np.median(hs))
        if len(idxs) > 1:
            merged += 1
        for i in idxs:
            cx, cy, radius, blend, max_z, is_base = pads[i]
            d = np.sqrt((xx - cx) ** 2 + (yy - cy) ** 2)
            w = np.clip((blend - (d - radius)) / blend, 0.0, 1.0)
            znew = z * (1.0 - w) + h * w
            # 资源平台限制改动幅度：只抹平 <= pad_delta 的小起伏，
            # 身后的陡坎/山体（落差远大于此）原样保留 —— 矿才真的"贴山脚"
            if PAD_MAX_DELTA < 1e8:
                keep = np.abs(z - h) > PAD_MAX_DELTA
                z = np.where(keep & (not is_base), z, znew)
            else:
                z = znew
    if verbose:
        print('  [平台] %d 块平台（%d 个簇，其中 %d 个是多平台合并的平地）'
              % (n, len(groups), merged))
    return z


def main_boundary_mask(geo, size):
    """主区外轮廓。配方里强度 >= 1 的那一份才画，邻区只补陆地，不画进这条线。"""
    gx = [c for c in geo.cities if float(c[3]) >= 1.0]
    if not gx:
        gx = list(geo.cities)
    img = Image.new('L', (size, size), 0)
    d = ImageDraw.Draw(img)
    for name, rings, prov, amp in gx:
        for r in rings:
            d.polygon([geo.to_cell(lon, lat) for lon, lat in r], fill=255)
    m = np.array(img) > 0
    if not m.any():
        return np.zeros((size, size), dtype=bool)
    er = m.copy()
    er[1:, :] &= m[:-1, :]
    er[:-1, :] &= m[1:, :]
    er[:, 1:] &= m[:, :-1]
    er[:, :-1] &= m[:, 1:]
    edge = m & ~er
    # 加粗一格，缩略图/雷达上更醒目
    dil = edge.copy()
    dil[1:, :] |= edge[:-1, :]
    dil[:-1, :] |= edge[1:, :]
    dil[:, 1:] |= edge[:, :-1]
    dil[:, :-1] |= edge[:, 1:]
    return dil


def opening(mask, radius):
    """形态学开运算（先腐蚀后膨胀）：去掉小于 radius 的孤立小块，保留大面积块。"""
    m = mask.copy()
    for _ in range(max(1, int(radius))):              # 腐蚀
        m = m & np.roll(m, 1, 0) & np.roll(m, -1, 0) & np.roll(m, 1, 1) & np.roll(m, -1, 1)
    for _ in range(max(1, int(radius))):              # 膨胀
        m = m | np.roll(m, 1, 0) | np.roll(m, -1, 0) | np.roll(m, 1, 1) | np.roll(m, -1, 1)
    return m


def keep_largest_components(mask, n_keep):
    """只保留面积最大的 n_keep 个连通块，其余置 False（4 邻接，纯 numpy+deque）。"""
    from collections import deque
    H, W = mask.shape
    lab = np.zeros((H, W), dtype=np.int32)
    sizes = [0]
    cur = 0
    ys, xs = np.nonzero(mask)
    for y0, x0 in zip(ys.tolist(), xs.tolist()):
        if lab[y0, x0]:
            continue
        cur += 1
        lab[y0, x0] = cur
        q = deque(((y0, x0),))
        cnt = 0
        while q:
            cy, cx = q.popleft()
            cnt += 1
            if cy > 0 and mask[cy - 1, cx] and lab[cy - 1, cx] == 0:
                lab[cy - 1, cx] = cur; q.append((cy - 1, cx))
            if cy + 1 < H and mask[cy + 1, cx] and lab[cy + 1, cx] == 0:
                lab[cy + 1, cx] = cur; q.append((cy + 1, cx))
            if cx > 0 and mask[cy, cx - 1] and lab[cy, cx - 1] == 0:
                lab[cy, cx - 1] = cur; q.append((cy, cx - 1))
            if cx + 1 < W and mask[cy, cx + 1] and lab[cy, cx + 1] == 0:
                lab[cy, cx + 1] = cur; q.append((cy, cx + 1))
        sizes.append(cnt)
    if cur <= n_keep:
        return mask.copy()
    order = np.argsort(np.array(sizes[1:]))[::-1][:n_keep] + 1
    return np.isin(lab, order)


def dilate_cross(mask, r):
    """十字结构元膨胀 r 次（r=0 返回原图）。"""
    m = mask.copy()
    for _ in range(max(0, int(r))):
        d = m.copy()
        d[1:, :] |= m[:-1, :]
        d[:-1, :] |= m[1:, :]
        d[:, 1:] |= m[:, :-1]
        d[:, :-1] |= m[:, 1:]
        m = d
    return m


def erode_cross(mask, r):
    """十字结构元腐蚀 r 次（r=0 返回原图）。"""
    return ~dilate_cross(~mask, max(0, int(r)))


def level_join(zq, lv, radius, min_block=400, low_levels=None, land=None,
               band_index=2, rounds=8, mode='expand', verbose=True):
    """把「二层高地之间散落的一层地面」整片并成二层，让二层高地连成一片。

    做法（有界闭运算 + 保护带）：
      1) 记 A = 高度 >= 一层(lv[1]) 的陆地格子（一层/二层/三层/高山）。
      2) 候选 = 膨胀(A, radius) ∩ 一层(lv[band_index]) ∩ 距平原/海 >= radius
         —— 保护带保证不向低地一侧外扩，只抹平高地内部的缝隙。
      3) 逐个候选连通片：只有它把 >= 2 个面积 >= min_block 的「一层地块」连起来时才整片并成
         二层（单块面积不受限 —— 这就是「整块抹平」口径）。

    band_index=2 表示把 lv[2]（二层 350）当作连片目标层、lv[1]（一层 280）当作待填的缝隙层。
    返回 (新的高度场, 被并成二层的掩码)。
    """
    from collections import deque
    r = max(1, int(radius))
    lv = [float(v) for v in lv]
    if len(lv) < 2:
        return zq, np.zeros(zq.shape, dtype=bool)
    lo = float(lv[0])
    band = float(lv[min(max(1, int(band_index)), len(lv) - 1)])   # 默认二层：把一层缝隙并成二层
    if low_levels is None:
        low_levels = (lo,)
    low = np.zeros(zq.shape, dtype=bool)
    for v in low_levels:
        low |= (zq <= float(v) + 1.0)                 # 平原（含比平原更低的水下地形）

    # ---- 保护带：到低地的 4 邻接距离（只算一次）
    H, W = zq.shape
    dist = np.full(H * W, -1, dtype=np.int32)
    q = deque()
    for p in np.flatnonzero(low.ravel()):
        dist[p] = 0
        q.append(int(p))
    while q:
        p = q.popleft()
        d = dist[p]
        y, x = divmod(p, W)
        if y > 0 and dist[p - W] < 0:
            dist[p - W] = d + 1; q.append(p - W)
        if y + 1 < H and dist[p + W] < 0:
            dist[p + W] = d + 1; q.append(p + W)
        if x > 0 and dist[p - 1] < 0:
            dist[p - 1] = d + 1; q.append(p - 1)
        if x + 1 < W and dist[p + 1] < 0:
            dist[p + 1] = d + 1; q.append(p + 1)
    dist = dist.reshape(H, W)

    m350 = (np.abs(zq - band) < 1.0)                  # 二层地块
    gap = float(lv[max(0, min(max(1, int(band_index)) - 1, len(lv) - 1))])   # 缝隙层 = 一层
    gapm = (np.abs(zq - gap) < 1.0) & (dist >= r)     # 一层地面 + 保护带
    if land is not None:
        gapm = gapm & land                            # 水下不算
        m350 = m350 & land
    mb = int(min_block)

    def label4(mask):
        """4 邻接连通标记 + 面积表。"""
        lab = np.zeros((H, W), dtype=np.int32)
        sz = [0]
        n = 0
        for y0, x0 in zip(*[v.tolist() for v in np.nonzero(mask)]):
            if lab[y0, x0]:
                continue
            n += 1
            lab[y0, x0] = n
            qq = deque(((y0, x0),))
            cnt = 0
            while qq:
                cy, cx = qq.popleft()
                cnt += 1
                for ny, nx in ((cy - 1, cx), (cy + 1, cx), (cy, cx - 1), (cy, cx + 1)):
                    if 0 <= ny < H and 0 <= nx < W and mask[ny, nx] and lab[ny, nx] == 0:
                        lab[ny, nx] = n
                        qq.append((ny, nx))
            sz.append(cnt)
        return lab, sz, n

    def one_round(cur_mask):
        """一轮连片。两种检测口径：
           expand  = 二层掩码外扩 radius 格以内的一层地面（预览图用的口径，会把边界外推）
           closing = 闭运算露出来的缝隙（只填两块之间窄缝，不外扩）
           采纳条件相同：该新增片必须把 >=2 个面积 >= min_block 的二层地块连起来。"""
        if mode == 'closing':
            added = erode_cross(dilate_cross(cur_mask, r), r) & ~cur_mask & gapm
        else:
            added = dilate_cross(cur_mask, r) & ~cur_mask & gapm
        if not added.any():
            return None, 0, 0
        lab, sz, n = label4(cur_mask)
        big = np.zeros(n + 1, dtype=bool)
        for i in range(1, n + 1):
            big[i] = sz[i] >= mb
        labc, _, nc = label4(added)
        fillm = np.zeros((H, W), dtype=bool)
        used = 0
        for k in range(1, nc + 1):
            cell = (labc == k)
            nb = dilate_cross(cell, 1) & cur_mask
            ids = set(int(v) for v in np.unique(lab[nb]) if v > 0 and big[int(v)])
            if len(ids) >= 2:
                fillm |= cell
                used += 1
        return fillm, used, nc

    # ---- 迭代：填好的缝隙并入掩码后再来一轮，缝隙逐轮变窄，直到连成一片或到轮数上限
    rounds = max(1, int(rounds))
    fill = np.zeros((H, W), dtype=bool)
    total_used = 0
    total_cand = 0
    round_cap = rounds
    for it in range(round_cap):
        m = m350 | fill
        f, used, nc = one_round(m)
        total_used += used
        total_cand += nc
        if f is None or not f.any():
            break
        if (fill | f).sum() == fill.sum():
            break
        fill |= f
    zq = np.where(fill, band, zq)
    if verbose:
        lab, sz, n = label4(m350 | fill)
        big = sum(1 for v in sz[1:] if v >= mb)
        print('  [连片] 半径 %d、最多 %d 轮：一层缝隙并成二层 %d 格（%.2f%% 全图）；'
              '二层地块 %d 块（>=%d 格 %d 块），合并缝 %d 处'
              % (r, rounds, int(fill.sum()), 100.0 * fill.mean(), n, mb, big, total_used))
    return zq, fill


def box_sum(mask, r):
    """(2r+1)² 窗口内的和（积分图，快）。"""
    c = np.cumsum(np.cumsum(mask.astype(np.int32), axis=0), axis=1)
    c = np.pad(c, ((1, 0), (1, 0)))
    H, W = mask.shape
    y0 = np.clip(np.arange(H) - r, 0, H); y1 = np.clip(np.arange(H) + r + 1, 0, H)
    x0 = np.clip(np.arange(W) - r, 0, W); x1 = np.clip(np.arange(W) + r + 1, 0, W)
    return c[np.ix_(y1, x1)] - c[np.ix_(y0, x1)] - c[np.ix_(y1, x0)] + c[np.ix_(y0, x0)]


def layer_majority(lab, r, passes, levels, fixed=None):
    """层号图的多数投票平滑：窗口内出现最多的层号胜出（平票保持原层）。"""
    vals = [float(v) for v in levels] + [0.0]
    orig = lab.copy()
    for _ in range(max(0, int(passes))):
        counts = np.stack([box_sum(lab == v, r) for v in vals])
        best = np.argmax(counts, axis=0)
        cur = np.zeros(lab.shape, dtype=np.int32)
        for i, v in enumerate(vals):
            cur = np.where(lab == v, i, cur)
        bc = np.take_along_axis(counts, best[None], axis=0)[0]
        cc = np.take_along_axis(counts, cur[None], axis=0)[0]
        best = np.where(bc == cc, cur, best)
        lab = np.array(vals)[best]
        if fixed is not None:
            lab = np.where(fixed, orig, lab)
    return lab


def comp_label(mask):
    """4 邻接连通标记：返回 (标签图, 面积表, 块数)。面积表下标 0 占位。"""
    from collections import deque
    H, W = mask.shape
    lab = np.zeros((H, W), dtype=np.int32)
    sizes = [0]
    n = 0
    ys, xs = np.nonzero(mask)
    for y0, x0 in zip(ys.tolist(), xs.tolist()):
        if lab[y0, x0]:
            continue
        n += 1
        lab[y0, x0] = n
        q = deque(((y0, x0),))
        cnt = 0
        while q:
            cy, cx = q.popleft()
            cnt += 1
            for ny, nx in ((cy - 1, cx), (cy + 1, cx), (cy, cx - 1), (cy, cx + 1)):
                if 0 <= ny < H and 0 <= nx < W and mask[ny, nx] and lab[ny, nx] == 0:
                    lab[ny, nx] = n
                    q.append((ny, nx))
        sizes.append(cnt)
    return lab, sizes, n


def layer_build(lab, levels, smooth_r=0, smooth_passes=0, min_area=None,
                close_r=None, prune2_area=None, close2_r=None,
                final_maj_r=0, sea_close=0, sea_min=0, verbose=False):
    """按「边界平滑 + 删孤立块 + 连片」三件套重塑层号图（= 预览图 v11 的生成口径）。

      1) 边界平滑：层号多数投票（窗口 (2*smooth_r+1)²，smooth_passes 轮）
      2) 删孤立块：某层面积 < min_area[层] 的连通块整体降一级
      3) 连片：某层闭运算（半径 close_r[层]），只吃相邻中间层，不动海
      4) 第二遍删孤立块（连片后残留的小块）：prune2_area[层]
      5) 第二遍连片：close2_r[层]
      6) 收尾平滑：再多数投票 final_maj_r 一轮（把连片产生的方形边界磨圆）
      7) 海域：面积 < sea_min 的海上闭运算缺口填成 levels[1]

    lab 的层号即高度值；levels 为升序层高列表。返回 (新层号图, 明细)。
    """
    lab = lab.copy()
    lv = [float(v) for v in levels]
    order = sorted(lv, reverse=True)
    info = {}
    sea = (lab <= 0)

    def prune(area_map, key):
        if not area_map:
            return
        for v in order:
            thr = int(area_map.get(v, 0) or 0)
            if thr <= 0:
                continue
            m = (lab == v)
            if not m.any():
                continue
            _lab, sizes, n = comp_label(m)
            small = np.zeros(n + 1, dtype=bool)
            for i in range(1, n + 1):
                small[i] = sizes[i] < thr
            drop = small[_lab] & m
            if drop.any():
                i = order.index(v)
                lab[drop] = order[i + 1] if i + 1 < len(order) else 0.0
                info[key + '_%d' % int(v)] = int(drop.sum())

    def join(radius_map, key):
        if not radius_map:
            return
        for v in sorted(radius_map):
            r = int(radius_map[v])
            if r <= 0:
                continue
            m = (lab == v)
            if not m.any():
                continue
            new = erode_cross(dilate_cross(m, r), r) & ~m
            if not new.any():
                continue
            i = lv.index(float(v)) if float(v) in lv else -1
            ok = np.zeros(lab.shape, dtype=bool)
            if i > 0:
                ok |= (lab == lv[i - 1])
            if 0 <= i < len(lv) - 1:
                ok |= (lab == lv[i + 1])
            take = new & ok
            lab[take] = v
            info[key + '_%d' % int(v)] = int(take.sum())

    if smooth_r > 0 and smooth_passes > 0:
        lab = layer_majority(lab, int(smooth_r), int(smooth_passes), lv, fixed=sea)
        info['smooth1'] = 'r%d x%d' % (smooth_r, smooth_passes)
    prune(min_area, 'drop1')
    join(close_r, 'join1')
    prune(prune2_area, 'drop2')
    join(close2_r, 'join2')
    if final_maj_r > 0:
        lab = layer_majority(lab, int(final_maj_r), 1, lv, fixed=sea)
        info['smooth2'] = 'r%d x1' % final_maj_r
    if sea_close > 0:
        m = (lab <= 0)
        new = erode_cross(dilate_cross(m, int(sea_close)), int(sea_close)) & ~m
        if new.any():
            _lab, sizes, n = comp_label(new)
            small = np.zeros(n + 1, dtype=bool)
            for i in range(1, n + 1):
                small[i] = sizes[i] < int(sea_min)
            take = small[_lab] & new
            lab[take] = lv[1] if len(lv) > 1 else lv[0]
            info['sea_filled'] = int(take.sum())
    if verbose:
        print('  [层规则] ' + '  '.join('%s=%s' % (k, v) for k, v in info.items()))
    return lab, info


def carve_layer_ramps(z, base_cells, verbose=True, T=None):
    """在**层与层之间**挖可通行斜坡（spec §19.37）。

    规则：
      - 每条「相邻层交界」（212↔280、280↔350、350↔420）的每一个**独立交界片**至少 1 个斜坡；
        交界片越大 -> 开越多（`ramp_per_px` 格面积 1 个，上限 `ramp_max_per_patch`）。
      - 斜坡位置：候选 = 该交界片内「坡度最缓、离层边界最远」的点；
        评分偏向**地图中部（两组交汇点）**与**最近的出生点** —— 斜坡朝向中部与玩家方向。
      - 方向：从交界处朝**外**（往低层一侧）延伸 `ramp_len` 格，做成 5~10 单位/格的缓坡（可通行）。

    返回 (新的高度场, 斜坡中心列表 [(x, y, 上层层高)])。
    """
    T = T if T is not None else {}
    if not T.get('level_ramps', True):
        return z, []
    levels = [float(v) for v in (T.get('levels') or [])]
    if len(levels) < 2:
        return z, []
    lv_all = [LAND_Z + 2.0] + [v for v in levels if v > LAND_Z + 2.0]
    lv_all = sorted(set(round(v, 1) for v in lv_all))
    H, W = z.shape
    land = z >= WATER_Z
    gy, gx = np.gradient(z)
    slope = np.sqrt(gx ** 2 + gy ** 2)
    if verbose:
        print('  [斜坡] 层表 %s' % lv_all)

    lvl_i = np.full(z.shape, -1, dtype=np.int8)
    for i, v in enumerate(lv_all):
        lvl_i[np.abs(z - v) < 1.5] = i
    # 层交界带：用「窗口内层号极差」判定 —— 平滑会把硬坎软化成过渡带，
    # 只比较相邻两会漏掉几乎全部交界（实测从 11302 格掉到 11 格）。
    # 注意：必须先把 -1（陡坎/海面/高山等无层号的格子）填成邻近的有效层号，
    #       否则窗口极值会被 -1 污染（dilate 取到 -1、erode 取到最大层），span 恒为负。
    _li = np.where(lvl_i >= 0, lvl_i, np.int16(-1)).astype(np.int16)
    _last = _li.copy()
    for _ in range(6):                                  # 迭代把 -1 填成最近的有效层号
        _d = dilate_cross(_last, 1)
        _last = np.where(_last >= 0, _last, np.where(_d >= 0, _d, np.int16(-1)))
        if (_last < 0).sum() == 0:
            break
    _fill = np.where(_last >= 0, _last, np.int16(0))
    _hi = dilate_cross(_fill, 2)
    _lo = erode_cross(_fill, 2)
    span = (_hi - _lo)                      # 窗口内最高层 - 最低层
    bnd = (lvl_i >= 0) & (span >= 1) & land

    ramp_len = float(T.get('ramp_len', 14.0))
    grade = float(T.get('ramp_grade', 6.0))             # 目标坡度（世界单位/格）：<10 才可通行
    clear = float(T.get('ramp_clear', 2.0))
    per_px = float(T.get('ramp_per_px', 6000.0))
    max_per = int(T.get('ramp_max_per_patch', 6))
    min_cells = int(T.get('ramp_min_patch', 40))
    valid = lvl_i >= 0
    d_edge = distance_transform(~valid)                 # 层内部离交界多远（越大越"中心"）
    cx0, cy0 = GRID / 2.0, GRID / 2.0
    base_pts = list(base_cells.values()) if base_cells else [(cx0, cy0)]
    ramps = []
    used = []                                           # 已开斜坡中心，避免重叠

    def patch_id(mask):
        """给交界片编号（4 邻接）。"""
        lab, _sizes, n = comp_label(mask)
        return lab, n

    for i in range(len(lv_all) - 1):
        lo, hi = lv_all[i], lv_all[i + 1]
        # 交界带 = 低层靠近高层的格子与高层靠近低层的格子
        m_lo = (np.abs(z - lo) < 1.5)
        m_hi = (np.abs(z - hi) < 1.5)
        touch = np.zeros(z.shape, dtype=bool)
        for _ in range(2):
            touch |= (m_lo & bnd) | (m_hi & bnd)
        band = np.zeros(z.shape, dtype=bool)
        d_b = distance_transform(bnd)
        band = (d_b <= 2.0) & (m_lo | m_hi) & land
        if not band.any():
            continue
        lab, n = patch_id(band)
        _nbig = sum(1 for _k in range(1, n + 1) if int((lab == _k).sum()) >= min_cells)
        if verbose:
            print('    [斜坡] %4.0f -> %4.0f：交界带 %d 格，%d 片，其中 >=%d 格 %d 片'
                  % (lo, hi, int(band.sum()), n, min_cells, _nbig))
        for k in range(1, n + 1):
            cells = (lab == k)
            ncell = int(cells.sum())
            if ncell < min_cells:
                continue
            want = int(min(max_per, max(1, round(ncell / per_px))))
            ys, xs = np.nonzero(cells)
            # 候选点：交界片内、坡度缓、离层边界远
            score = d_edge[ys, xs] - 0.5 * slope[ys, xs]
            order = np.argsort(-score)
            got = 0
            for idx in order[:2000]:
                px, py = int(xs[idx]), int(ys[idx])
                if any(math.hypot(px - ux, py - uy) < ramp_len * 2.2 for ux, uy in used):
                    continue
                # 方向：朝**外**（低层一侧）延伸；同时用"沿该方向走到最低点所需格数"定坡长，
                # 保证坡度 <= grade（可通行）。
                best = None
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    last_lo, t_lo = None, 0
                    for t in range(1, int(ramp_len) + 3):
                        xx, yy = px + dx * t, py + dy * t
                        if not (0 <= xx < W and 0 <= yy < H) or not land[yy, xx]:
                            break
                        if abs(z[yy, xx] - lo) < 1.5:
                            last_lo, t_lo = (xx, yy), t
                        elif z[yy, xx] > lo + 1.5 and t_lo:
                            break
                    if last_lo is None or t_lo < 2:
                        continue
                    if best is None or t_lo < best[3]:
                        best = (dx, dy, last_lo, t_lo)
                if best is None:
                    continue
                dx, dy, (endx, endy), t_end = best
                # 坡长 = 落差 / 目标坡度（不足则取可用长度）
                need = int(math.ceil((hi - lo) / max(1.0, grade)))
                L = int(min(max(need, 3), t_end + 2, int(ramp_len) + 2))
                # 评分加成：离地图中部近 / 离最近出生点近（越靠中部与玩家越好）
                d_mid = math.hypot(px - cx0, py - cy0)
                d_ply = min(math.hypot(px - bx, py - by) for bx, by in base_pts)
                bias = (float(T.get('ramp_bias_mid', 1.0)) * d_mid
                        + float(T.get('ramp_bias_player', 0.5)) * d_ply)
                if got >= want and bias > float(T.get('ramp_bias_cap', 260.0)):
                    continue
                # 挖斜坡：从交界处朝外，高度线性降到 low；坡顶/坡底各抹 2 格做圆滑衔接
                prof = []
                for t in range(L + 1):
                    xx, yy = px + dx * t, py + dy * t
                    if not (0 <= xx < W and 0 <= yy < H):
                        break
                    prof.append((xx, yy, hi + (lo - hi) * (t / max(1.0, float(L)))))
                if len(prof) < 3:
                    continue
                # 坡底再往前抹 2 格到 low（避免与低层之间留硬台阶）
                _ex, _ey = px + dx * (L + 1), py + dy * (L + 1)
                for _t2 in (1, 2):
                    xx2, yy2 = px + dx * (L + _t2), py + dy * (L + _t2)
                    if 0 <= xx2 < W and 0 <= yy2 < H:
                        prof.append((xx2, yy2, lo))
                # 坡顶往回抹 2 格（用 high 层高度，避免交界处出现尖角）
                for _t3 in (1, 2):
                    xx3, yy3 = px - dx * _t3, py - dy * _t3
                    if 0 <= xx3 < W and 0 <= yy3 < H:
                        prof.append((xx3, yy3, hi))
                halfw = max(1, int(T.get('ramp_half_width', 5.0)))
                for (xx, yy, hh) in prof:
                    for oy in range(-halfw, halfw + 1):
                        for ox in range(-halfw, halfw + 1):
                            if ox * ox + oy * oy > halfw * halfw:
                                continue
                            x2, y2 = xx + ox, yy + oy
                            if 0 <= x2 < W and 0 <= y2 < H and land[y2, x2]:
                                z[y2, x2] = hh
                used.append((px, py))
                ramps.append((px, py, hi))
                got += 1
                if got >= want and ncell < per_px * 2:
                    break
            if verbose and got:
                print('    [斜坡] %4.0f -> %4.0f 的第 %d 片（%d 格）开了 %d 个斜坡'
                      % (lo, hi, k, ncell, got))
    if verbose:
        print('  [斜坡] 共开 %d 个层间斜坡（ramp_len=%.0f, half_width=%.0f, 目标坡度 %.0f）'
              % (len(ramps), ramp_len, float(T.get('ramp_half_width', 5.0)), grade))
    # ---- 收尾：双向配对限坡 —— 把每对相邻格的高差压到 <= smax，
    #      保证层与层之间不会残留走不上去的硬台阶。
    smax = float(T.get('ramp_slope_max', 9.0))
    for _ in range(int(T.get('ramp_relax', 10))):
        old = z.copy()
        for _ax in (0, 1):
            nxt = np.roll(z, -1, axis=_ax)
            d = z - nxt
            m = d > smax                                  # 前高后低 -> 削前
            z = np.where(m, (z + nxt + smax) * 0.5, z)
            nxt2 = np.roll(z, -1, axis=_ax)
            nxt3 = np.roll(z, 1, axis=_ax)
            d2 = nxt2 - z                                 # 后高前低 -> 削后
            m2 = d2 > smax
            z = np.where(m2, np.roll((z + nxt2 - smax) * 0.5, -1, axis=_ax), z)
        if np.abs(z - old).max() < 1e-9:
            break
    z = np.where(land, z, np.where(z < WATER_Z, float(T.get('sea_flat', LAND_Z - 20.0)), z))
    return z, ramps


def _bilinear(z, xs, ys):
    """双线性采样（xs/ys 为浮点格坐标数组）。"""
    H, W = z.shape
    x0 = np.clip(np.floor(xs).astype(np.int32), 0, W - 2)
    y0 = np.clip(np.floor(ys).astype(np.int32), 0, H - 2)
    fx = np.clip(xs - x0, 0.0, 1.0)
    fy = np.clip(ys - y0, 0.0, 1.0)
    a = z[y0, x0]
    b = z[y0, x0 + 1]
    c = z[y0 + 1, x0]
    d = z[y0 + 1, x0 + 1]
    return ((a * (1 - fx) + b * fx) * (1 - fy) + (c * (1 - fx) + d * fx) * fy)


def monotone_profile(win, L, grade):
    """把 win[0]=high ... win[-1]=low 整理成单调、且坡度不超过 grade 的剖面。"""
    win = np.asarray(win, dtype=np.float64)
    if win.size == 0:
        return win
    if win.size > 1:
        for _ in range(3):
            win[1:] = np.minimum(win[1:], win[:-1] - grade * 0.5)
            win[:-1] = np.maximum(win[:-1], win[1:] + grade * 0.5)
    return win


def quantize_strict_layers(z, T=None, verbose=True):
    """把地形压成**严格分层**：每个格子归到最近的层高，层与层之间是硬坎。

    这样"不同层之间的边界"默认**不可通行**（硬坎坡度极大），只有后面显式挖的斜坡能上下。
    """
    T = T if T is not None else {}
    if layer_height.enabled(T):
        return _quantize_standard(z, T, verbose)
    levels = sorted(set([round(float(LAND_Z + 2.0), 1)]
                        + [round(float(v), 1) for v in (T.get('levels') or []) if v > LAND_Z + 2.0]))
    H, W = z.shape
    sea_z = float(T.get('sea_flat', LAND_Z - 20.0))
    zz = z.copy()
    zz = np.where(zz < WATER_Z, sea_z, zz)              # 海/水一律拉平到海底高度
    vals = np.array(levels, dtype=np.float64)
    # 每格归到最近的合法层高
    idx = np.abs(zz[..., None] - vals[None, None, :]).argmin(axis=2)
    lab = vals[idx]
    lab = np.where(zz < WATER_Z, sea_z, lab)
    if verbose:
        print('  [层规] 压成严格分层：%s'
              % '  '.join('%.0f=%.1f%%' % (v, 100.0 * (lab == v).mean())
                          for v in (sea_z,) + tuple(levels)))
    return lab


def _quantize_standard(z, T, verbose):
    """通用层高：陆地层是平面，水域和五层山峰各是一段高度。"""
    levels = layer_height.LEVELS
    water_lo, water_hi = (float(v) for v in T.get('water_range', layer_height.WATER_RANGE))
    peak_lo, peak_hi = (float(v) for v in T.get('peak_range', layer_height.PEAK_RANGE))
    water = z < WATER_Z
    peak = (~water) & (z >= layer_height.PEAK_FROM)
    vals = np.array(levels, dtype=np.float64)
    idx = np.abs(z[..., None] - vals[None, None, :]).argmin(axis=2)
    lab = vals[idx]
    if peak.any():
        t = np.clip((z - layer_height.PEAK_FROM) / max(1.0, 1000.0 - layer_height.PEAK_FROM), 0.0, 1.0)
        lab = np.where(peak, peak_lo + t * (peak_hi - peak_lo), lab)
    dist = distance_transform(~water)
    shelf = float(layer_height.SHORE_SHELF)
    span = float(layer_height.DEEP_SPAN)
    bed = water_hi - np.clip((dist - shelf) / span, 0.0, 1.0) * (water_hi - water_lo)
    lab = np.where(water, bed, lab)
    if verbose:
        parts = ['水域 %.0f–%.0f=%.1f%%' % (water_lo, water_hi, 100.0 * water.mean())]
        parts += ['%.0f=%.1f%%' % (v, 100.0 * (np.abs(lab - v) < 1.5).mean()) for v in levels]
        parts.append('五层 %.0f–%.0f=%.1f%%' % (peak_lo, peak_hi, 100.0 * peak.mean()))
        print('  [层规] 通用层高：' + '  '.join(parts))
    return lab


def ramp_uphill(z, rs):
    """沿坡单位向量，从低层指向高层。

    自动斜坡用规划时的上坡方向。手工框仍用矩形内低层、高层的质心，
    不用框心梯度（框心经常是平的，会默认朝东）。
    """
    pair = rs.get('pair') or rs.get('layers') or (212.0, 280.0)
    lo_v, hi_v = float(min(pair)), float(max(pair))
    if rs.get('ux') is not None and rs.get('uy') is not None:
        ux, uy = float(rs['ux']), float(rs['uy'])
        nrm = math.hypot(ux, uy) or 1.0
        return ux / nrm, uy / nrm, lo_v, hi_v
    cx, cy = float(rs['x']), float(rs['y'])
    bw = float(rs.get('w', 56)) * 0.5
    bh = float(rs.get('h', 56)) * 0.5
    H, W = z.shape
    y0, y1 = max(0, int(math.floor(cy - bh))), min(H, int(math.ceil(cy + bh)) + 1)
    x0, x1 = max(0, int(math.floor(cx - bw))), min(W, int(math.ceil(cx + bw)) + 1)
    sub = z[y0:y1, x0:x1]
    mlo = np.abs(sub - lo_v) < 1.5
    mhi = np.abs(sub - hi_v) < 1.5
    dx = dy = 0.0
    if mlo.any() and mhi.any():
        ylo, xlo = np.nonzero(mlo)
        yhi, xhi = np.nonzero(mhi)
        dx = float((xhi + x0).mean() - (xlo + x0).mean())
        dy = float((yhi + y0).mean() - (ylo + y0).mean())
    if math.hypot(dx, dy) < 1e-6:
        gy, gx = np.gradient(z)
        box = np.zeros(z.shape, dtype=bool)
        box[y0:y1, x0:x1] = True
        dx = float(gx[box].mean()) if box.any() else 1.0
        dy = float(gy[box].mean()) if box.any() else 0.0
    nrm = math.hypot(dx, dy) or 1.0
    return dx / nrm, dy / nrm, lo_v, hi_v


def _layer_touch(a, b):
    """a 侧贴着 b 的格子。"""
    t = np.zeros(a.shape, dtype=bool)
    t[:, 1:] |= a[:, 1:] & b[:, :-1]
    t[:, :-1] |= a[:, :-1] & b[:, 1:]
    t[1:, :] |= a[1:, :] & b[:-1, :]
    t[:-1, :] |= a[:-1, :] & b[1:, :]
    return t


def _ramp_ii(a):
    ii = np.zeros((a.shape[0] + 1, a.shape[1] + 1), dtype=np.float64)
    ii[1:, 1:] = np.cumsum(np.cumsum(a, axis=0), axis=1)
    return ii


def _ramp_win(ii, ys, xs, r):
    h = ii.shape[0] - 1
    w = ii.shape[1] - 1
    y0 = np.clip(ys - r, 0, h)
    x0 = np.clip(xs - r, 0, w)
    y1 = np.clip(ys + r + 1, 0, h)
    x1 = np.clip(xs + r + 1, 0, w)
    return ii[y1, x1] - ii[y0, x1] - ii[y1, x0] + ii[y0, x0]


def _high_side_axes(hi_c, ys, xs):
    """交界格指向高层一侧的单位法线。"""
    sx = np.zeros(hi_c.shape, dtype=np.float64)
    sy = np.zeros(hi_c.shape, dtype=np.float64)
    nb = np.zeros(hi_c.shape, dtype=bool)
    nb[:, :-1] = hi_c[:, 1:]
    sx[:, :-1] += nb[:, :-1]
    nb[:, :] = False
    nb[:, 1:] = hi_c[:, :-1]
    sx[:, 1:] -= nb[:, 1:]
    nb[:, :] = False
    nb[:-1, :] = hi_c[1:, :]
    sy[:-1, :] += nb[:-1, :]
    nb[:, :] = False
    nb[1:, :] = hi_c[:-1, :]
    sy[1:, :] -= nb[1:, :]
    dx = sx[ys, xs]
    dy = sy[ys, xs]
    nrm = np.maximum(np.hypot(dx, dy), 1e-6)
    return dx / nrm, dy / nrm


def _smooth_axes(ux, uy, xs, ys, radius=14.0):
    """把单格法线沿交界抹平，得到这段边界的垂线。交界太长时保持单格法线。"""
    ux = np.asarray(ux, dtype=np.float64)
    uy = np.asarray(uy, dtype=np.float64)
    n = int(ux.shape[0])
    if n == 0 or n > 3000:
        return ux, uy
    xs = np.asarray(xs, dtype=np.float64)
    ys = np.asarray(ys, dtype=np.float64)
    w = np.clip(radius - np.hypot(xs[:, None] - xs[None, :], ys[:, None] - ys[None, :]), 0.0, None)
    su = w @ ux
    sv = w @ uy
    nrm = np.hypot(su, sv)
    bad = nrm < 1e-3
    nrm = np.maximum(nrm, 1e-6)
    out_u = np.where(bad, ux, su / nrm)
    out_v = np.where(bad, uy, sv / nrm)
    flip = (out_u * ux + out_v * uy) < 0.0
    out_u = np.where(flip, -out_u, out_u)
    out_v = np.where(flip, -out_v, out_v)
    return out_u, out_v


def plan_auto_ramps(z, targets, bases=None, width=28.0, length=28.0, verbose=True):
    """在层与层、水与陆的交界上自动开口。

    每一对贴在一起的大连片至少一条斜坡。没写 ramp_max_per_edge 时最多 3 条；
    写了就按那个上限，尽量让不同位置的玩家能过层、能到资源。
    开口尽量落在玩家通往中心或资源的直线上，上坡优先朝向这些目标。
    ramp_marks 把附近的坡挪到划线上，方向跟线一致；附近没有就补一条。
    配方 ramp_layout=face 时只连相邻层，坡与坡隔开，凸出朝凸出、左侧边界垂直朝第一层。
    配方 ramp_balance 时上坡尽量垂直于层界，坡面落在两侧的格子尽量接近；
    两端踩不上台面时退回原来的朝向。没开的图不走这条。
    边长 20~40 格。其余交界保持硬坎。
    """
    H, W = z.shape
    sea = float(T.get('sea_flat', LAND_Z - 20.0))
    bands = [(sea, z < WATER_Z)]
    for _lv in layer_height.play_levels(T):
        bands.append((float(_lv), np.abs(z - float(_lv)) < 1.5))
    labeled = []
    for val, mask in bands:
        lab, sizes, n = comp_label(mask)
        labeled.append((val, lab, sizes, mask))
    links = []
    face = T.get('ramp_layout') == 'face'
    balance = bool(T.get('ramp_balance'))
    if face:
        # 一条坡只跨相邻两层。一层直接接到三层会同时跨过三个层，不开。
        if len(labeled) > 1:
            links.append((0, 1))
        for j in range(1, len(labeled) - 1):
            links.append((j, j + 1))
    else:
        for j in range(1, len(labeled)):
            links.append((0, j))                       # 水对每一层陆地
        for j in range(1, len(labeled)):
            for k in range(j + 1, len(labeled)):
                links.append((j, k))                   # 实际贴在一起的陆地，含隔层相接
    targets = [(float(p[0]), float(p[1])) for p in (targets or [])]
    if not targets:
        targets = [(W / 2.0, H / 2.0)]
    bases = [(float(p[0]), float(p[1])) for p in (bases or [])]
    goals = targets
    specs = []
    taken = []
    span = float(max(20.0, min(40.0, length)))
    wide = float(max(20.0, min(40.0, width)))
    ramp_gap = float(T.get('ramp_gap', 120.0)) if face else 28.0

    def _on(mask, y, x):
        for oy, ox in ((0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)):
            y2, x2 = y + oy, x + ox
            if 0 <= x2 < W and 0 <= y2 < H and mask[y2, x2]:
                return True
        return False

    def room(y, x, ux, uy, lo_m, hi_m, lo_reach, hi_reach, strict=False):
        step = 2 if strict else 3

        def hit(mask, yy, xx):
            if not (0 <= xx < W and 0 <= yy < H):
                return False
            if strict:
                return bool(mask[yy, xx])
            return _on(mask, yy, xx)

        for s in range(4, int(lo_reach) + 1, step):
            if not hit(lo_m, int(round(y - uy * s)), int(round(x - ux * s))):
                return False
        for s in range(4, int(hi_reach) + 1, step):
            if not hit(hi_m, int(round(y + uy * s)), int(round(x + ux * s))):
                return False
        return True

    def _probe_reach(span_i, bias=2.0):
        # 坡心沿上坡偏 bias 格（缺省 2）。验收从坡心再探 4 格，两侧台面要盖住这段。
        half_u = max(10.0, min(20.0, float(span_i) * 0.5))
        hi_reach = int(math.ceil(float(bias) + half_u + 4.0)) + 2
        lo_reach = int(math.ceil(max(4.0, half_u + 4.0 - float(bias)))) + 2
        return lo_reach, hi_reach

    def _body_clear(y, x, ux, uy, lo_m, hi_m, span_i, wide_i, bias=2.0):
        """坡身不吃第三层。验收在坡端外再探 4 格，探到的格子必须整格落在对应台面上。"""
        half_u = max(10.0, min(20.0, float(span_i) * 0.5))
        cx = float(x) + float(ux) * float(bias)
        cy = float(y) + float(uy) * float(bias)
        bw = float(span_i) * 0.5
        bh = float(wide_i) * 0.5
        x0 = max(0, int(math.floor(cx - bw)))
        y0 = max(0, int(math.floor(cy - bh)))
        x1 = min(W, int(math.ceil(cx + bw)) + 1)
        y1 = min(H, int(math.ceil(cy + bh)) + 1)
        if x0 >= x1 or y0 >= y1:
            return False
        if not (lo_m[y0:y1, x0:x1] | hi_m[y0:y1, x0:x1]).all():
            return False

        def _pad(sign, mask):
            # 半格步进，把验收双线性会碰到的两个格子都算进去。
            # 12.5 的坡端在框内会被挖成台面；框外 13.5~16.5 必须已经是台面。
            for s in range(int(math.floor(half_u)) + 2, int(math.floor(half_u)) + 6):
                px = cx + ux * sign * (s - 0.5)
                py = cy + uy * sign * (s - 0.5)
                for iy in (int(math.floor(py)), int(math.ceil(py))):
                    for ix in (int(math.floor(px)), int(math.ceil(px))):
                        if not (0 <= ix < W and 0 <= iy < H and mask[iy, ix]):
                            return False
            return True

        return _pad(-1.0, lo_m) and _pad(1.0, hi_m)

    def _ramp_share(cx, cy, ux, uy, half_u, bw, bh, lo_m, hi_m):
        """挖坡占地里，低层、高层、总格数。占地是沿坡矩形并上轴对齐框。"""
        half_v = 0.0
        for sx, sy in ((bw, bh), (bw, -bh), (-bw, bh), (-bw, -bh)):
            half_v = max(half_v, abs(-sx * uy + sy * ux))
        rad = int(math.ceil(max(half_u, half_v, bw, bh))) + 2
        x0 = max(0, int(math.floor(cx - rad)))
        y0 = max(0, int(math.floor(cy - rad)))
        x1 = min(W, int(math.ceil(cx + rad)) + 1)
        y1 = min(H, int(math.ceil(cy + rad)) + 1)
        if x0 >= x1 or y0 >= y1:
            return 0, 0, 0
        yy, xx = np.mgrid[y0:y1, x0:x1]
        rx = xx - cx
        ry = yy - cy
        u = rx * ux + ry * uy
        v = -rx * uy + ry * ux
        inside = ((np.abs(u) <= half_u) & (np.abs(v) <= half_v)
                  | ((np.abs(rx) <= bw) & (np.abs(ry) <= bh)))
        lo_n = int((inside & lo_m[y0:y1, x0:x1]).sum())
        hi_n = int((inside & hi_m[y0:y1, x0:x1]).sum())
        return lo_n, hi_n, int(inside.sum())

    def _balanced_pose(y, x, ux, uy, lo_m, hi_m, span_i, wide_i):
        """法线附近选摆法。两侧格子差在 15% 以内就保持最接近垂直的角度。"""
        half_u = max(10.0, min(20.0, float(span_i) * 0.5))
        bw = float(span_i) * 0.5
        bh = float(wide_i) * 0.5
        best = None
        base = math.atan2(float(uy), float(ux))
        for dang in (0.0, 8.0, -8.0, 16.0, -16.0):
            ang = base + math.radians(dang)
            uxi, uyi = math.cos(ang), math.sin(ang)
            if uxi * float(ux) + uyi * float(uy) < 0.0:
                uxi, uyi = -uxi, -uyi
            angle_best = None
            for bias in (0.0, 0.5, 1.0, 2.0):
                lo_need, hi_need = _probe_reach(span_i, bias)
                if not room(y, x, uxi, uyi, lo_m, hi_m, lo_need, hi_need, strict=face):
                    continue
                if face and not _body_clear(
                        y, x, uxi, uyi, lo_m, hi_m, span_i, wide_i, bias=bias):
                    continue
                cx = float(x) + uxi * bias
                cy = float(y) + uyi * bias
                lo_n, hi_n, inside = _ramp_share(
                    cx, cy, uxi, uyi, half_u, bw, bh, lo_m, hi_m)
                if inside < 1:
                    continue
                imb = abs(lo_n - hi_n) / float(inside)
                other = inside - lo_n - hi_n
                if other > 0.15 * inside:
                    imb = max(imb, other / float(inside))
                cand = (imb, float(bias))
                if angle_best is None or cand < angle_best[0]:
                    angle_best = (cand, uxi, uyi, float(bias), (lo_n, hi_n))
            if angle_best is None:
                continue
            imb = angle_best[0][0]
            packed = (angle_best[1], angle_best[2], angle_best[3], angle_best[4])
            if imb <= 0.15:
                return ((0, abs(dang), imb),) + packed
            score = (1, imb, abs(dang))
            if best is None or score < best[0]:
                best = (score,) + packed
        return best

    def _place(y, x, ux, uy, lo_v, hi_v, nedge, span_i, wide_i, bias=2.0, share=None):
        cx = float(x) + float(ux) * float(bias)
        cy = float(y) + float(uy) * float(bias)
        specs.append({
            'x': round(cx, 1), 'y': round(cy, 1),
            'w': span_i, 'h': wide_i,
            'layers': [lo_v, hi_v], 'pair': [lo_v, hi_v],
            'auto': True, 'span': span_i, 'width': wide_i,
            'ux': round(float(ux), 4), 'uy': round(float(uy), 4),
        })
        taken.append((cx, cy))
        if verbose and share is not None:
            ang = math.degrees(math.atan2(float(uy), float(ux)))
            print('  [斜坡] %.0f→%.0f @(%d,%d) 朝 %.0f° 低%d 高%d 交界 %d 格'
                  % (lo_v, hi_v, round(cx), round(cy), ang, share[0], share[1], nedge))
        elif verbose:
            ang = math.degrees(math.atan2(float(uy), float(ux)))
            print('  [斜坡] %.0f→%.0f @(%d,%d) 朝 %.0f° 交界 %d 格'
                  % (lo_v, hi_v, round(cx), round(cy), ang, nedge))

    for ia, ib in links:
        lo_v, lo_lab, lo_sz, lo_m = labeled[ia]
        hi_v, hi_lab, hi_sz, hi_m = labeled[ib]
        for ca in range(1, len(lo_sz)):
            if lo_sz[ca] < 200:
                continue
            for cb in range(1, len(hi_sz)):
                if hi_sz[cb] < 200:
                    continue
                edge = _layer_touch(lo_lab == ca, hi_lab == cb)
                nedge = int(edge.sum())
                if nedge < 40:
                    continue
                # smoothstep 峰值约 1.5*落差/边长。没写 ramp_size 时邻层 28、隔层 40。
                # 写了就把边长交给配方；隔层落差仍加长到沿坡能过验收的最短。
                delta = abs(float(hi_v) - float(lo_v))
                span_i, wide_i = span, wide
                if delta > 90.0 and T.get('ramp_size') is None:
                    span_i, wide_i = 40.0, 40.0
                elif delta > 90.0:
                    need = 1.5 * delta / 6.0
                    span_i = float(min(40.0, max(span, math.ceil(need))))
                    if span_i > 34.0:
                        span_i = max(span_i, 36.0)
                    wide_i = wide
                ys, xs = np.nonzero(edge)
                hi_c = hi_lab == cb
                ux, uy = _high_side_axes(hi_c, ys, xs)
                ux0, uy0 = ux, uy
                convex = np.zeros(len(xs), dtype=bool)
                left = np.zeros(len(xs), dtype=bool)
                if face and len(xs):
                    ys_h, xs_h = np.nonzero(hi_c)
                    hx = float(xs_h.mean()) if xs_h.size else float(xs.mean())
                    yy, xx = np.mgrid[0:H, 0:W]
                    hi_f = hi_c.astype(np.float64)
                    mass = _ramp_win(_ramp_ii(hi_f), ys, xs, 16)
                    mx = _ramp_win(_ramp_ii(hi_f * xx), ys, xs, 16) / np.maximum(mass, 1.0)
                    my = _ramp_win(_ramp_ii(hi_f * yy), ys, xs, 16) / np.maximum(mass, 1.0)
                    frac = mass / float((16 * 2 + 1) ** 2)
                    convex = (frac < 0.40) & (frac > 0.03)
                    vx, vy = mx - xs, my - ys
                    vn = np.maximum(np.hypot(vx, vy), 1e-6)
                    ux = np.where(convex, vx / vn, ux)
                    uy = np.where(convex, vy / vn, uy)
                    # 层块左侧、边界大致南北向：垂直边界开口，下坡朝第一层。
                    l1 = labeled[1][3]
                    l1x = np.nonzero(l1)[1]
                    l1cx = float(l1x.mean()) if l1x.size else hx
                    left = (xs < hx - 30.0) & (np.abs(ux0) >= np.abs(uy0))
                    up_x = np.where(l1cx < xs, 1.0, -1.0)
                    ux = np.where(left, up_x, ux)
                    uy = np.where(left, 0.0, uy)
                if balance:
                    # 角度让位给层界法线。凸出和左侧仍只影响开口挑在哪一段。
                    ux, uy = ux0, uy0
                    ux_s, uy_s = _smooth_axes(
                        ux0, uy0, xs, ys, radius=max(8.0, float(wide_i) * 0.5))
                else:
                    ux_s, uy_s = ux, uy
                best_align = np.full(len(xs), -1.0)
                for tx, ty in targets:
                    vx = tx - xs
                    vy = ty - ys
                    dist = np.maximum(np.hypot(vx, vy), 1.0)
                    align = (vx * ux + vy * uy) / dist
                    best_align = np.maximum(best_align, align / (1.0 + dist / 500.0))
                useful = np.zeros(len(xs), dtype=np.float64)
                near = 0
                if bases:
                    cx0, cy0 = targets[0]
                    straight_c = np.array([math.hypot(bx - cx0, by - cy0) for bx, by in bases])
                    via_c = np.stack([
                        np.hypot(xs - bx, ys - by) + np.hypot(xs - cx0, ys - cy0)
                        for bx, by in bases
                    ])
                    extra_c = via_c - straight_c[:, None]
                    near = int(np.sum(extra_c.min(axis=1) < 90.0))
                    for bi, (bx, by) in enumerate(bases):
                        for gx, gy in goals:
                            straight = math.hypot(bx - gx, by - gy)
                            if straight < 40.0:
                                continue
                            extra = (np.hypot(xs - bx, ys - by)
                                     + np.hypot(xs - gx, ys - gy) - straight)
                            useful = np.maximum(
                                useful, np.where(extra < 180.0, 1.0 / (1.0 + extra / 40.0), 0.0))
                cap = int(T.get('ramp_max_per_edge', 3))
                n_ramp = 1
                if cap >= 5:
                    if nedge >= 500 or near >= 2:
                        n_ramp = 2
                    if nedge >= 900 or near >= 3:
                        n_ramp = 3
                    if nedge >= 1300 or near >= 4:
                        n_ramp = 4
                    if nedge >= 1700 or near >= 5:
                        n_ramp = 5
                else:
                    if nedge >= 700 or near >= 2:
                        n_ramp = 2
                    if nedge >= 1600 or near >= 4:
                        n_ramp = 3
                n_ramp = min(n_ramp, max(1, cap))
                prefer = np.maximum(best_align, 0.0) * (0.35 + useful) + useful
                if face:
                    prefer = prefer + np.where(convex, 3.0, 0.0) + np.where(left, 1.5, 0.0)
                local = []

                def _try(order, align_floor, sep_local, lo_reach, hi_reach, need, base_clear):
                    uneven = []
                    checked = 0

                    def _clear(x, y):
                        if any(math.hypot(x - bx, y - by) < base_clear for bx, by in bases):
                            return False
                        if any(math.hypot(x - px, y - py) < sep_local for px, py, *_rest in local):
                            return False
                        if any(math.hypot(x - px, y - py) < ramp_gap for px, py in taken):
                            return False
                        return True

                    def _fits(y, x, uxi, uyi):
                        if not room(y, x, uxi, uyi, lo_m, hi_m, lo_reach, hi_reach, strict=face):
                            return False
                        if face and not _body_clear(y, x, uxi, uyi, lo_m, hi_m, span_i, wide_i):
                            return False
                        return True

                    for k in order:
                        if len(local) >= need and not balance:
                            return
                        if balance and (len(local) >= need or checked >= 200):
                            break
                        if best_align[k] < align_floor:
                            continue
                        y, x = int(ys[k]), int(xs[k])
                        if not _clear(x, y):
                            continue

                        if balance:
                            checked += 1
                            pose = _balanced_pose(
                                y, x, float(ux_s[k]), float(uy_s[k]),
                                lo_m, hi_m, span_i, wide_i)
                            if pose is not None:
                                score, uxi, uyi, bias, share = pose
                                item = (float(x), float(y), uxi, uyi, bias, share)
                                if score[0] == 0:
                                    local.append(item)
                                else:
                                    uneven.append((score, item))
                                continue
                            uxi, uyi = float(ux0[k]), float(uy0[k])
                            if _fits(y, x, uxi, uyi):
                                uneven.append(((2, 0.0, 0.0),
                                               (float(x), float(y), uxi, uyi, 2.0, None)))
                            continue

                        uxi, uyi = float(ux[k]), float(uy[k])
                        if T.get('ramp_axis_snap') and not face:
                            ax, ay = abs(uxi), abs(uyi)
                            if ax >= ay and ay < 0.45 * max(ax, 1e-6):
                                uxi, uyi = math.copysign(1.0, uxi), 0.0
                            elif ay > ax and ax < 0.45 * max(ay, 1e-6):
                                uxi, uyi = 0.0, math.copysign(1.0, uyi)
                        if not _fits(y, x, uxi, uyi):
                            uxi, uyi = float(ux0[k]), float(uy0[k])
                            if not _fits(y, x, uxi, uyi):
                                continue
                        local.append((float(x), float(y), uxi, uyi, 2.0, None))
                    if balance and len(local) < need:
                        uneven.sort()
                        for _score, item in uneven:
                            if len(local) >= need:
                                break
                            if not _clear(item[0], item[1]):
                                continue
                            local.append(item)

                order_pref = np.argsort(-prefer)
                sep_far = ramp_gap if face else 150.0
                sep_near = ramp_gap if face else 90.0
                if face:
                    lo_need, hi_need = _probe_reach(span_i)
                    _try(order_pref, -1.0, ramp_gap, lo_need, hi_need, n_ramp, 48.0)
                    if len(local) < 1:
                        _try(order_pref, -1.0, ramp_gap, lo_need, hi_need, 1, 0.0)
                elif span_i >= 36.0:
                    # 坡心沿上坡挪了 2 格，验收还要再探 4 格，所以两侧都要有 28 格同层。
                    _try(order_pref, 0.02, max(120.0, sep_far), 28, 28, n_ramp, 48.0)
                    if len(local) < 1:
                        _try(order_pref, -1.0, max(60.0, sep_near) if face else 60.0, 28, 28, 1, 0.0)
                else:
                    _try(order_pref, 0.02, sep_far, 22, 26, n_ramp, 80.0)
                    if len(local) < n_ramp:
                        _try(order_pref, 0.02, sep_near, 16, 20, n_ramp, 48.0)
                    if len(local) < 1:
                        _try(np.argsort(-(useful + best_align * 0.05)), -1.0, sep_near if face else 0.0,
                             12, 16, 1, 0.0)
                    if len(local) < 1 and len(xs) and not face:
                        k = int(np.argmax(useful + np.maximum(best_align, 0.0)))
                        local.append((float(xs[k]), float(ys[k]), float(ux[k]), float(uy[k]), 2.0, None))
                for x, y, uxi, uyi, bias, share in local:
                    _place(y, x, uxi, uyi, lo_v, hi_v, nedge, span_i, wide_i,
                           bias=bias, share=share)
    raw_marks = [tuple(m) for m in (T.get('ramp_marks') or ())]
    marks = [(float(m[0]), float(m[1]), float(m[2]), len(m) >= 4 and bool(m[3]))
             for m in raw_marks]
    pinned = []

    def _span_mark(mx, my, ang):
        """这条划线允许跨层时，附近如果低层和高侧隔着一级以上，一条坡直接连过去。

        没标跨层的划线仍只连相邻层。
        """
        rad = 48
        x0 = max(0, int(mx) - rad)
        x1 = min(W, int(mx) + rad + 1)
        y0 = max(0, int(my) - rad)
        y1 = min(H, int(my) + rad + 1)
        near = []
        for i, (val, _lab, _sizes, mask) in enumerate(labeled):
            if i == 0:
                continue
            sub = mask[y0:y1, x0:x1]
            if int(sub.sum()) < 30:
                continue
            ys, xs = np.nonzero(sub)
            dmin = float(np.min(np.hypot(xs + x0 - mx, ys + y0 - my)))
            if dmin <= rad:
                near.append(i)
        if len(near) < 2 or near[-1] - near[0] < 2:
            return False
        lo_i, hi_i = near[0], near[-1]
        lo_v, _lo_lab, _lo_sz, lo_m = labeled[lo_i]
        hi_v, _hi_lab, _hi_sz, hi_m = labeled[hi_i]
        ys_l, xs_l = np.nonzero(lo_m[y0:y1, x0:x1])
        ys_h, xs_h = np.nonzero(hi_m[y0:y1, x0:x1])
        lx = float((xs_l + x0).mean())
        ly = float((ys_l + y0).mean())
        hx = float((xs_h + x0).mean())
        hy = float((ys_h + y0).mean())
        vx, vy = hx - lx, hy - ly
        nrm = math.hypot(vx, vy) or 1.0
        ux, uy = vx / nrm, vy / nrm
        # 笔画方向能分别到达高低层时，沿笔画，不沿质心。
        base = math.radians(ang)
        for sign in (1.0, -1.0):
            tx, ty = math.cos(base) * sign, math.sin(base) * sign
            def _hit(mask, dx, dy):
                for s in range(6, rad + 1, 2):
                    xx = int(round(mx + dx * s))
                    yy = int(round(my + dy * s))
                    if 0 <= xx < W and 0 <= yy < H and mask[yy, xx]:
                        return True
                return False
            if _hit(hi_m, tx, ty) and _hit(lo_m, -tx, -ty):
                ux, uy = tx, ty
                break
        # 沿笔画找台面内缘。窗口里别处靠得更近的层块不能把坡心拽离这条线。
        def _at(mask, dist):
            xx = int(round(mx + ux * dist))
            yy = int(round(my + uy * dist))
            return 0 <= xx < W and 0 <= yy < H and bool(mask[yy, xx])

        def _solid(mask, sign):
            for s in range(0, rad + 1):
                if all(_at(mask, sign * (s + k)) for k in range(6)):
                    return float(s)
            return None

        s_hi = _solid(hi_m, 1.0)
        s_lo = _solid(lo_m, -1.0)
        if s_hi is None or s_lo is None:
            return False
        reach = (s_hi + s_lo) * 0.5
        mid_t = (s_hi - s_lo) * 0.5
        delta = abs(float(hi_v) - float(lo_v))
        need = 1.5 * delta / 6.0
        span_i = float(min(40.0, max(span, math.ceil(max(need, 2.0 * reach)))))
        wide_i = wide
        half = max(10.0, min(20.0, span_i * 0.5))
        # 边长最多 40。缝比这还宽，两端踩不上台面。
        if half + 1.0 < reach:
            return False
        mid_x = mx + ux * mid_t
        mid_y = my + uy * mid_t

        def _on(mask, dist):
            xx = int(round(mid_x + ux * dist))
            yy = int(round(mid_y + uy * dist))
            return 0 <= xx < W and 0 <= yy < H and bool(mask[yy, xx])

        def _run(mask, sign):
            # 坡的尽头和再往外探的 4 格都要在台面上，不能只探到远处、中间还隔着一层。
            for extra in range(-1, 5):
                if not _on(mask, sign * (half + float(extra))):
                    return False
            return True

        nudge = 0.0
        for _ in range(12):
            lo_ok, hi_ok = _run(lo_m, -1.0), _run(hi_m, 1.0)
            if lo_ok and hi_ok:
                break
            if not lo_ok:
                nudge -= 2.0
            if not hi_ok:
                nudge += 2.0
            if abs(nudge) < 1.0:
                return False
            mid_x = mx + ux * (mid_t + nudge)
            mid_y = my + uy * (mid_t + nudge)
        else:
            if not (_run(lo_m, -1.0) and _run(hi_m, 1.0)):
                return False
        # _place 会再沿上坡挪 2 格，这里先退回来。
        site_x = int(round(mid_x - ux * 2.0))
        site_y = int(round(mid_y - uy * 2.0))
        cx = float(site_x) + float(ux) * 2.0
        cy = float(site_y) + float(uy) * 2.0
        kept = []
        for s in specs:
            if math.hypot(s['x'] - cx, s['y'] - cy) < 90.0:
                continue
            kept.append(s)
        specs[:] = kept
        taken[:] = [(float(s['x']), float(s['y'])) for s in specs]
        _place(site_y, site_x, ux, uy, lo_v, hi_v, 0, span_i, wide_i)
        pinned.append((cx, cy))
        if verbose:
            print('  [斜坡] 划线 (%.0f,%.0f) 跨层 %.0f→%.0f -> (%.0f,%.0f)'
                  % (mx, my, lo_v, hi_v, cx, cy))
        return True

    for mx, my, ang, span_ok in marks:
        if span_ok and T.get('ramp_mark_span') and _span_mark(mx, my, ang):
            continue
        best = None
        for ia, ib in links:
            lo_v, lo_lab, lo_sz, _lo_all = labeled[ia]
            hi_v, hi_lab, hi_sz, _hi_all = labeled[ib]
            delta = abs(float(hi_v) - float(lo_v))
            span_i, wide_i = span, wide
            if delta > 90.0 and T.get('ramp_size') is None:
                span_i, wide_i = 40.0, 40.0
            elif delta > 90.0:
                need = 1.5 * delta / 6.0
                span_i = float(min(40.0, max(span, math.ceil(need))))
                wide_i = wide
            for ca in range(1, len(lo_sz)):
                if lo_sz[ca] < 200:
                    continue
                for cb in range(1, len(hi_sz)):
                    if hi_sz[cb] < 200:
                        continue
                    edge = _layer_touch(lo_lab == ca, hi_lab == cb)
                    if int(edge.sum()) < 40:
                        continue
                    ys, xs = np.nonzero(edge)
                    dmin = float(np.min(np.hypot(xs - mx, ys - my)))
                    if dmin <= 42.0 and (best is None or dmin < best[0]):
                        best = (dmin, lo_v, hi_v, lo_lab == ca, hi_lab == cb, span_i, wide_i, ys, xs)
        if best is None:
            if verbose:
                print('  [斜坡] 划线 (%.0f,%.0f) 附近没有能开口的交界' % (mx, my))
            continue
        _dmin, lo_v, hi_v, lo_m, hi_m, span_i, wide_i, ys, xs = best
        d = np.hypot(xs - mx, ys - my)
        order = np.argsort(d)
        order = [int(k) for k in order if d[k] <= 42.0][:60]
        lo_need, hi_need = _probe_reach(span_i)
        base = math.radians(ang)
        site = None
        bias = 2.0
        share = None
        if balance and order:
            ux_e, uy_e = _high_side_axes(hi_m, ys, xs)
            ux_e, uy_e = _smooth_axes(
                ux_e, uy_e, xs, ys, radius=max(8.0, float(wide_i) * 0.5))
            d0 = float(d[order[0]])
            near_found = []
            any_found = []
            for k in order:
                y, x = int(ys[k]), int(xs[k])
                pose = _balanced_pose(
                    y, x, float(ux_e[k]), float(uy_e[k]), lo_m, hi_m, span_i, wide_i)
                if pose is None:
                    continue
                score, uxi, uyi, bias_i, share_i = pose
                item = (score, float(d[k]), (y, x, uxi, uyi, bias_i, share_i))
                any_found.append(item)
                if float(d[k]) <= d0 + 8.0:
                    near_found.append(item)
            pool = near_found or any_found
            if pool:
                pool.sort()
                y, x, uxi, uyi, bias, share = pool[0][2]
                site = (y, x, uxi, uyi)
        if site is None:
            for dang in (0.0, 8.0, -8.0, 16.0, -16.0, 24.0, -24.0):
                ux0 = math.cos(base + math.radians(dang))
                uy0 = math.sin(base + math.radians(dang))
                for k in order:
                    y, x = int(ys[k]), int(xs[k])
                    for sign in (1.0, -1.0):
                        uxi, uyi = ux0 * sign, uy0 * sign
                        hit = False
                        for s in (6, 10, 14):
                            yy = int(round(y + uyi * s))
                            xx = int(round(x + uxi * s))
                            if 0 <= xx < W and 0 <= yy < H and hi_m[yy, xx]:
                                hit = True
                                break
                        if not hit:
                            continue
                        if not room(y, x, uxi, uyi, lo_m, hi_m, lo_need, hi_need, strict=face):
                            continue
                        if face and not _body_clear(y, x, uxi, uyi, lo_m, hi_m, span_i, wide_i):
                            continue
                        site = (y, x, uxi, uyi)
                        bias = 2.0
                        share = None
                        break
                    if site is not None:
                        break
                if site is not None:
                    break
        if site is None:
            if verbose:
                print('  [斜坡] 划线 (%.0f,%.0f) 方向对不上台面，未改' % (mx, my))
            continue
        y, x, uxi, uyi = site
        cx = float(x) + float(uxi) * float(bias)
        cy = float(y) + float(uyi) * float(bias)
        kept = []
        for s in specs:
            same = (abs(min(s['layers']) - min(lo_v, hi_v)) < 2.0
                    and abs(max(s['layers']) - max(lo_v, hi_v)) < 2.0)
            near_pin = any(math.hypot(s['x'] - px, s['y'] - py) < 28.0 for px, py in pinned)
            if same and (not near_pin) and math.hypot(s['x'] - cx, s['y'] - cy) < 110.0:
                continue
            kept.append(s)
        specs[:] = kept
        taken[:] = [(float(s['x']), float(s['y'])) for s in specs]
        _place(y, x, uxi, uyi, lo_v, hi_v, 0, span_i, wide_i, bias=bias, share=share)
        pinned.append((cx, cy))
        if verbose:
            print('  [斜坡] 划线 (%.0f,%.0f) -> (%.0f,%.0f) 朝 %.0f°'
                  % (mx, my, cx, cy, math.degrees(math.atan2(uyi, uxi))))
    cap = int(T.get('ramp_max_per_edge', 3))
    if verbose:
        print('  [斜坡] 自动开口 %d 条（每段交界最多 %d 条，边长 20~40）' % (len(specs), cap))
    if balance and verbose and specs:
        diffs = []
        for s in specs:
            pair = s.get('pair') or s.get('layers') or (0.0, 0.0)
            lo_v, hi_v = float(min(pair)), float(max(pair))
            lo_m = hi_m = None
            for val, _lab, _sz, mask in labeled:
                if lo_m is None and abs(float(val) - lo_v) < 1.5:
                    lo_m = mask
                if hi_m is None and abs(float(val) - hi_v) < 1.5:
                    hi_m = mask
            if lo_m is None or hi_m is None:
                continue
            half_u = max(10.0, min(20.0, float(s.get('span', s.get('w', 28))) * 0.5))
            lo_n, hi_n, inside = _ramp_share(
                float(s['x']), float(s['y']), float(s['ux']), float(s['uy']),
                half_u, float(s['w']) * 0.5, float(s['h']) * 0.5, lo_m, hi_m)
            if inside < 1:
                continue
            diffs.append((abs(lo_n - hi_n) / float(inside), lo_n, hi_n,
                          float(s['x']), float(s['y'])))
        if diffs:
            mid = sorted(d[0] for d in diffs)[len(diffs) // 2]
            worst = max(diffs)
            print('  [斜坡] 两侧相差 中位 %.0f%% 最大 %.0f%% @(%d,%d) 低%d 高%d'
                  % (100 * mid, 100 * worst[0], round(worst[3]), round(worst[4]),
                     worst[1], worst[2]))
    return specs


def ramp_body_mask(shape, ramp_specs, z=None):
    """挖出来的斜坡占地：沿坡矩形，再并上轴对齐框。范围和 carve_marked_ramps 一致。"""
    H, W = shape
    cover = np.zeros(shape, dtype=bool)
    if not ramp_specs:
        return cover
    yy, xx = np.mgrid[0:H, 0:W].astype(np.float64)
    for rs in ramp_specs:
        if rs.get('ux') is not None and rs.get('uy') is not None:
            ux, uy = float(rs['ux']), float(rs['uy'])
            nrm = math.hypot(ux, uy) or 1.0
            ux, uy = ux / nrm, uy / nrm
        elif z is not None:
            ux, uy, _lo, _hi = ramp_uphill(z, rs)
        else:
            ux, uy = 1.0, 0.0
        if rs.get('auto'):
            half_u = max(10.0, min(20.0, float(rs.get('span', rs.get('w', 32))) * 0.5))
        else:
            hw = max(16.0, float(rs.get('w', 56)) * 0.75)
            hh = max(16.0, float(rs.get('h', 56)) * 0.75)
            half_u = max(6.0, min(hw, hh) * 0.5)
            if z is not None:
                _up = ramp_uphill(z, rs)
                lo_v, hi_v = _up[2], _up[3]
                step = float(T.get('ramp_max_step', 2.5))
                half_u = max(half_u, 0.75 * abs(hi_v - lo_v) / max(step, 0.5))
        bw = float(rs.get('w', 56)) * 0.5
        bh = float(rs.get('h', 56)) * 0.5
        half_v = 0.0
        for sx, sy in ((bw, bh), (bw, -bh), (-bw, bh), (-bw, -bh)):
            half_v = max(half_v, abs(-sx * uy + sy * ux))
        half_v = max(4.0, half_v)
        cx, cy = float(rs['x']), float(rs['y'])
        rx = xx - cx
        ry = yy - cy
        u = rx * ux + ry * uy
        v = -rx * uy + ry * ux
        cover |= ((np.abs(u) <= half_u) & (np.abs(v) <= half_v)) | (
            (np.abs(rx) <= bw) & (np.abs(ry) <= bh))
    return cover


def ramp_cover_mask(shape, ramp_specs, pad=1.0):
    """自动斜坡的矩形占地。水陆硬坎在这里留出通道。"""
    H, W = shape
    cover = np.zeros(shape, dtype=bool)
    if not ramp_specs:
        return cover
    yy, xx = np.mgrid[0:H, 0:W]
    for rs in ramp_specs:
        bw = float(rs.get('w', 32)) * 0.5 + pad
        bh = float(rs.get('h', 32)) * 0.5 + pad
        cover |= (np.abs(xx - float(rs['x'])) <= bw) & (np.abs(yy - float(rs['y'])) <= bh)
    return cover


def shore_block_mask(z, ramp_specs):
    """水陆交界的陆地格不可通行，斜坡占地除外。landing_coasts 圈内的岸整段可登陆。"""
    water = z < WATER_Z
    land = ~water
    shore = np.zeros(z.shape, dtype=bool)
    shore[:, 1:] |= land[:, 1:] & water[:, :-1]
    shore[:, :-1] |= land[:, :-1] & water[:, 1:]
    shore[1:, :] |= land[1:, :] & water[:-1, :]
    shore[:-1, :] |= land[:-1, :] & water[1:, :]
    shore &= ~ramp_cover_mask(z.shape, ramp_specs)
    coasts = list(T.get('landing_coasts') or ())
    if coasts:
        yy, xx = np.mgrid[0:z.shape[0], 0:z.shape[1]]
        inside = np.zeros(z.shape, dtype=bool)
        for c in coasts:
            inside |= ((xx - float(c[0])) ** 2 + (yy - float(c[1])) ** 2) <= float(c[2]) ** 2
        shore &= ~inside
    return shore


def grade_landing_shores(z, circles, ramp_specs=None, reach=12.0, verbose=True):
    """圈内海岸整段可登陆：贴岸水面从海床抬到仍低于水面判定的高度，陆地台面不动。"""
    circles = list(circles or ())
    if not circles:
        return z
    water = z < WATER_Z
    land = ~water
    dist = distance_transform(land)
    H, W = z.shape
    yy, xx = np.mgrid[0:H, 0:W]
    inside = np.zeros(z.shape, dtype=bool)
    for c in circles:
        inside |= ((xx - float(c[0])) ** 2 + (yy - float(c[1])) ** 2) <= float(c[2]) ** 2
    cover = ramp_cover_mask(z.shape, ramp_specs or [], pad=1.0)
    band = water & inside & (dist >= 1.0) & (dist <= float(reach)) & ~cover
    z = z.copy()
    if band.any():
        t = np.clip((float(reach) - dist) / max(float(reach) - 1.0, 1.0), 0.0, 1.0)
        s = t * t * (3.0 - 2.0 * t)
        sea = float(T.get('sea_flat', LAND_Z - 20.0))
        shore_top = min(WATER_Z - 8.0, sea + 70.0)
        z[band] = np.maximum(z[band], sea + (shore_top - sea) * s[band])
    shore = np.zeros(z.shape, dtype=bool)
    shore[:, 1:] |= land[:, 1:] & water[:, :-1]
    shore[:, :-1] |= land[:, :-1] & water[:, 1:]
    shore[1:, :] |= land[1:, :] & water[:-1, :]
    shore[:-1, :] |= land[:-1, :] & water[1:, :]
    if verbose:
        print('  [登陆] 圈内可上岸岸线 %d 格，贴岸水面抬高 %d 格'
              % (int((shore & inside).sum()), int(band.sum())))
    return z


def carve_marked_ramps(z, ramp_specs, T=None, verbose=True):
    """在用户标注的矩形处挖层间斜坡；其余层界保持硬坎（不可通行）。

    做法：矩形内沿"高->低"方向的长度做**双线性（smoothstep）高度场**，
    直接压成从高层到低层的平缓坡（用双线性而非在硬坎上叠加，是为了不被坎的陡度拖累）。
    结束后对整幅地形做一次限坡（只削不改层高）。
    """
    T = T if T is not None else {}
    if not ramp_specs:
        return z, 0
    H, W = z.shape
    n_done = 0
    for rs in ramp_specs:
        cx, cy = float(rs['x']), float(rs['y'])
        # 用标注框尺寸定坡面尺度（略放大，保证跨越层界）
        hw = max(16.0, float(rs.get('w', 56)) * 0.75)
        hh = max(16.0, float(rs.get('h', 56)) * 0.75)
        ux, uy, lo_v, hi_v = ramp_uphill(z, rs)
        if abs(hi_v - lo_v) < 5:
            continue
        # ---- 剖面：纯 smoothstep（低层台面 -> 坡道 -> 高层台面）。
        #      实测教训（T1.9.0 诊断 work/check_ramps.py）：旧版"从地形采样剖面 + 高斯权重
        #      混合"会在坡道两端留下 30~90 的接缝落差、沿坡单格落差最高 56 —— 因为高斯权重
        #      在坡道端点只有 ~0.6，混进了 40% 的旧硬坎。改为：**车道内权重恒 1**（剖面精确生效），
        #      只在车道外做羽毛过渡；剖面两端截断处保持台面高度 -> 端缝为 0。
        half_u = max(6.0, min(hw, hh) * 0.5)         # 坡道半长（沿坡方向）
        # 限坡：沿坡最大单格落差 <= ramp_max_step（smoothstep 峰值坡度 = 1.5*Δ/L）。
        #   不够长就往两侧台面延伸引道（顺坡线加长，不会在别处开新口子）。
        #   自动开口把边长限制在 20~40，不再为了 2.5 把坡拉出这个范围。
        _step = float(T.get('ramp_max_step', 2.5))
        if rs.get('auto'):
            half_u = max(10.0, min(20.0, float(rs.get('span', 32)) * 0.5))
        else:
            half_u = max(half_u, 0.75 * abs(hi_v - lo_v) / max(_step, 0.5))
        # 横向铺满标注框，框内车道两侧的硬坎也走同一条剖面。沿坡长度保持上面的限坡值，不再加长。
        _bw = float(rs.get('w', 56)) * 0.5
        _bh = float(rs.get('h', 56)) * 0.5
        _half_v_box = 0.0
        for _sx, _sy in ((_bw, _bh), (_bw, -_bh), (-_bw, _bh), (-_bw, -_bh)):
            _half_v_box = max(_half_v_box, abs(-_sx * uy + _sy * ux))
        half_v = max(4.0, _half_v_box)
        L = 2.0 * half_u
        ss = np.arange(0, L + 1, dtype=np.float64)
        t01 = ss / L
        prof = lo_v + (hi_v - lo_v) * (3.0 * t01 ** 2 - 2.0 * t01 ** 3)   # 单调、端点导数 0
        # 沿坡最大单格落差 = 1.5*(hi-lo)/L，hw/hh>=16 -> 远低于 SLOPE_MAX
        # 车道内权重 1。默认不再做羽化：4 格羽化会把约 70 的层差抹进坡度约 48 的裙边。
        feather = float(T.get('ramp_feather', 0.0))
        xx, yy = np.meshgrid(np.arange(W, dtype=np.float64), np.arange(H, dtype=np.float64))
        rx = xx - cx
        ry = yy - cy
        u = rx * ux + ry * uy                        # 沿坡坐标：-half_u=低层端, +half_u=高层端
        v = -rx * uy + ry * ux                       # 横向坐标
        dist_out = np.maximum(np.maximum(np.abs(u) - half_u, np.abs(v) - half_v), 0.0)
        # 剖面长度不动。标注框里剖面两端剩下的硬坎改成同一条剖面的台面，不再留在框内。
        in_box = (np.abs(rx) <= _bw) & (np.abs(ry) <= _bh)
        if feather <= 0.0:
            wgt = np.where((dist_out <= 0.0) | in_box, 1.0, 0.0)
        else:
            wgt = np.maximum(np.clip(1.0 - dist_out / feather, 0.0, 1.0), in_box.astype(np.float64))
        # 目标高度场：车道内按剖面；两端之外截断在台面高度（低层端 lo_v、高层端 hi_v）
        uu = np.clip(u + half_u, 0.0, L)
        i0 = np.clip(np.floor(uu).astype(np.int32), 0, max(0, prof.size - 2))
        frac = np.clip(uu - i0, 0.0, 1.0)
        hh_field = prof[i0] * (1.0 - frac) + prof[np.minimum(i0 + 1, prof.size - 1)] * frac
        z = z * (1.0 - wgt) + hh_field * wgt
        n_done += 1
    # 注意：**不做全图限坡**。全图限坡会把坡面之外的层界也削成缓坡，
    # 等于到处都是通路，违反"只有斜坡能跨层"的规则（实测违规通路 65k 格 -> 9k 格）。
    if verbose:
        print('  [斜坡] 按标注挖了 %d 条层间斜坡（其余层界保持硬坎不可通行）' % n_done)
    return z, n_done


def ensure_base_min_ore(z, base_cells, ore_list, min_ore=2, rng=45.0,
                        spacing=None, T=None, verbose=True, ramps=None, avoid=None):
    """保证**每个出生点周围 rng 格内至少有 min_ore 个矿**（用户硬要求）。

    陆上基地先补同层陆地，再补近矿范围内的水面；水上基地只补水面。
    返回 (新的矿位列表, 补了几个)。
    """
    T = T if T is not None else {}
    if spacing is None:
        spacing = float(getattr(cfg, 'ORE_MIN_SPACING', 46.0))
    H, W = z.shape
    placed = [(float(o[0]), float(o[1])) for o in ore_list]
    for _ap in (avoid or ()):
        placed.append((float(_ap[0]), float(_ap[1])))
    out = list(ore_list)
    d_edge_j, d_shore_j = ore_open_maps(z, T)
    _ecl = float(T.get('ore_edge_clear', 16.0))
    _scl = float(T.get('ore_shore_clear', 12.0))
    d_w = distance_transform(z < WATER_Z)
    d_l = distance_transform(z >= WATER_Z)
    layers = land_layer_index(z, T)
    _mouth = None
    added = 0
    for bn, c in base_cells.items():
        ix, iy = int(round(c[0])), int(round(c[1]))
        if not (0 <= ix < W and 0 <= iy < H):
            continue
        have = sum(1 for o in out if math.hypot(o[0] - ix, o[1] - iy) <= rng)
        if have >= min_ore:
            continue
        on_water = bool(z[iy, ix] < WATER_Z)
        _yyb, _xxb = np.mgrid[0:H, 0:W]
        _db = np.minimum.reduce([_xxb - BORDER, _yyb - BORDER,
                                 (GRID - 1 - BORDER) - _xxb,
                                 (GRID - 1 - BORDER) - _yyb])
        _gate = ((_db >= float(T.get('ore_border_clear', 30.0)))
                 & (d_edge_j >= _ecl) & (d_shore_j >= _scl))
        if on_water:
            _masks = [(z < WATER_Z) & (d_l >= 2.0) & _gate]
        else:
            if _mouth is None:
                _mouth = mouth_open_map(z, T, ramps)
            _bl = int(layers[iy, ix])
            _near_max = float(T.get('ore_near_max_r', 62.0))
            _dd = np.hypot(_xxb - ix, _yyb - iy)
            _same = ((layers == _bl) & (z >= WATER_Z) & (_bl >= 0)
                     & (d_w >= 4.0) & _gate & _mouth)
            _wet = ((z < WATER_Z) & (d_l >= _scl) & (_dd <= _near_max) & _gate)
            _masks = [_same, _wet] if _bl >= 0 else [
                (z >= WATER_Z) & (d_w >= 4.0) & _gate & _mouth]
        # ⚠ 保底补的矿**也必须离基地足够远**（否则会贴到基地上、把建造场卡住，
        #   实测出现过 0~4 格的情况）。用与近矿槽位一致的最小半径。
        _min_r = float(T.get('ore_near_min_r', 44.0))

        def _pick(cand, _ix=ix, _iy=iy, _min_r=_min_r):
            best, bk = None, None
            R = int(rng) + 30
            for r in range(0, R + 1):
                for dy in range(-r, r + 1):
                    for dx in range(-r, r + 1):
                        if max(abs(dx), abs(dy)) != r:
                            continue
                        x, y = _ix + dx, _iy + dy
                        if not (0 <= x < W and 0 <= y < H) or not cand[y, x]:
                            continue
                        if math.hypot(dx, dy) < _min_r:
                            continue                       # 别贴基地
                        if any(math.hypot(x - px, y - py) < spacing for px, py in placed):
                            continue
                        key = -math.hypot(dx, dy)
                        if bk is None or key > bk:
                            bk, best = key, (x, y)
                if best is not None:
                    break
            return best

        got = 0
        for _ in range(min_ore - have):
            best = None
            for cand in _masks:
                best = _pick(cand)
                if best is not None:
                    break
            if best is None:
                break
            placed.append((float(best[0]), float(best[1])))
            out.append((float(best[0]), float(best[1])))
            got += 1
            added += 1
        if verbose and got:
            print('    [矿位] %s 保底补 %d 个矿（原有 %d，水面基地=%s）'
                  % (bn, got, have, on_water))
    return out, added


def ore_open_maps(z, T=None):
    """算出"离层界多远 / 离水陆交汇多远"两张距离图（矿位判据共用）。"""
    T = T if T is not None else {}
    levels = sorted([float(v) for v in (T.get('levels') or [LAND_Z])])
    lvl = np.zeros(z.shape, dtype=np.int16)
    for v in levels:
        lvl = lvl + (z >= v - 1.0).astype(np.int16)
    edge = np.zeros(z.shape, dtype=bool)
    edge[:, 1:] |= (lvl[:, 1:] != lvl[:, :-1])
    edge[1:, :] |= (lvl[1:, :] != lvl[:-1, :])
    w0 = (z < WATER_Z)
    sedge = np.zeros(z.shape, dtype=bool)
    for _dy in (-1, 0, 1):
        for _dx in (-1, 0, 1):
            sedge |= ((~w0) & np.roll(np.roll(w0, _dy, 0), _dx, 1))
    return distance_transform(edge), distance_transform(sedge)


def enforce_ore_base_clearance(z, base_cells, ore_list, min_r=44.0, spacing=None,
                               T=None, verbose=True, ramps=None, avoid=None):
    """最终清洗：把**贴在基地上（< min_r 格）或贴着层界/岸边的矿**重新落位。

    基地旁那 2 个矿的槽位本身已经保证 44~62 格，但后续的补矿/兜底仍可能贴到基地上
    （实测出现过 0~4 格，会把建造场卡死）；另外"清洗挪位"自己也可能挪到层界/岸边附近。
    所以这里统一用同一套判据（基地距离 + 层界距离 + 岸边距离 + 邻矿间距）重放。
    """
    T = T if T is not None else {}
    if spacing is None:
        spacing = float(getattr(cfg, 'ORE_MIN_SPACING', 46.0))
    H, W = z.shape
    d_edge, d_shore = ore_open_maps(z, T)
    _ecl = float(T.get('ore_edge_clear', 16.0))
    _scl = float(T.get('ore_shore_clear', 12.0))
    _mopen = mouth_open_map(z, T, ramps)   # 挪位目标必须有开阔的矿口朝向
    layers = land_layer_index(z, T)
    out = []
    moved = 0
    for o in ore_list:
        ox, oy = float(o[0]), float(o[1])
        bn = min(base_cells.items(),
                 key=lambda kv: (kv[1][0] - ox) ** 2 + (kv[1][1] - oy) ** 2)
        _bx, _by = float(bn[1][0]), float(bn[1][1])
        d = math.hypot(ox - _bx, oy - _by)
        _ix, _iy = int(round(ox)), int(round(oy))
        _near_max = float(T.get('ore_near_max_r', 62.0))
        _is_near = d <= _near_max
        _bad_edge = d_edge[_iy, _ix] < _ecl
        _bad_shore = (z[_iy, _ix] >= WATER_Z) and (d_shore[_iy, _ix] < _scl)
        _bad_surface = _is_near and not near_ore_surface_ok(z, _bx, _by, ox, oy, T, layers)
        if d >= min_r and not _bad_edge and not _bad_shore and not _bad_surface:
            out.append((ox, oy))
            continue
        # 需要挪走：沿原矿相对最近基地的方向找合法格。
        # 已经在近矿环外的矿不能被拉进环里，否则会把基地旁那 2 个矿挤掉。
        _outer = d > _near_max
        _ang = math.atan2(oy - _by, ox - _bx)
        if not (d > 1e-6):
            _ang = 0.0
        placed = [(float(p[0]), float(p[1])) for p in out] + \
                 [(float(p[0]), float(p[1])) for p in ore_list if p is not o]
        for _ap in (avoid or ()):
            placed.append((float(_ap[0]), float(_ap[1])))
        best = None
        _onwater = z[int(_by), int(_bx)] < WATER_Z
        _rr0 = int(math.floor(_near_max)) + 1 if _outer else int(min_r)
        _rr1 = (max(int(min_r) + 61, int(math.ceil(d)) + 48) if _outer
                else int(min_r) + 61)
        for _rr in range(_rr0, _rr1):
            _cands = []
            for _da in (0.0, 0.3, -0.3, 0.6, -0.6, 0.9, -0.9, 1.2, -1.2):
                _x = int(round(_bx + math.cos(_ang + _da) * _rr))
                _y = int(round(_by + math.sin(_ang + _da) * _rr))
                if not (0 <= _x < W and 0 <= _y < H):
                    continue
                if not _outer:
                    if not near_ore_surface_ok(z, _bx, _by, _x, _y, T, layers):
                        continue
                    if (z[_y, _x] >= WATER_Z) and not _mopen[_y, _x]:
                        continue
                else:
                    if (z[_y, _x] < WATER_Z) != _onwater:
                        continue                               # 外围矿保持原来的水陆
                    if not _onwater and not _mopen[_y, _x]:
                        continue                               # 要有开阔的矿口朝向
                if d_edge[_y, _x] < _ecl or d_shore[_y, _x] < _scl:
                    continue                                   # 离层界/岸边够远
                if _outer and any(
                        math.hypot(_x - float(bc[0]), _y - float(bc[1])) <= _near_max
                        for bc in base_cells.values()):
                    continue                                   # 外围矿留在所有近矿环外
                if any(math.hypot(_x - px, _y - py) < spacing for px, py in placed):
                    continue
                _cands.append((abs(_da), _x, _y))
            if _cands:
                _cands.sort()
                best = (_cands[0][1], _cands[0][2])
                break
        if best is None:
            if d < min_r or _bad_surface:
                if verbose:
                    print('    [矿位] (%d,%d) 距%s仅%.0f格或落层不对，挪不开，跳过'
                          % (ox, oy, bn[0], d))
                continue                               # 贴在基地上、或近矿落层不对的矿不能留下
            out.append((ox, oy))
            continue
        out.append((float(best[0]), float(best[1])))
        moved += 1
        if verbose:
            _why = []
            if d < min_r:
                _why.append('距%s仅%.0f格' % (bn[0], d))
            if _bad_edge:
                _why.append('离层界%.0f格' % d_edge[_iy, _ix])
            if _bad_shore:
                _why.append('离岸%.0f格' % d_shore[_iy, _ix])
            if _bad_surface:
                _why.append('不在基地允许的层')
            print('    [矿位] (%d,%d) %s -> 挪到 (%d,%d)'
                  % (ox, oy, '、'.join(_why), best[0], best[1]))
    if verbose and moved:
        print('  [矿位] 清洗：%d 个不合规的矿已挪走' % moved)
    return out, moved


def _mouth_blocked_map(z, T=None, ramps=None):
    """矿口射线判障图（四周带 padding，防 roll 回绕）：层界/岸线/斜坡/不可通行 = True。

    开阔水面不算障碍。斜坡本身能走，但矿口不要朝它。
    """
    T = T if T is not None else {}
    r1 = int(T.get('ore_mouth_r1', 20))
    H, W = z.shape
    levels = sorted([float(v) for v in (T.get('levels') or [LAND_Z])])
    lvl = np.zeros(z.shape, dtype=np.int16)
    for v in levels:
        lvl = lvl + (z >= v - 1.0).astype(np.int16)
    gy, gx = np.gradient(z)
    slope = np.sqrt(gx * gx + gy * gy)
    blocked = (slope > SLOPE_IMPASSABLE) | (z > WATER_Z + PEAK_IMPASSABLE)
    bnd = np.zeros(z.shape, dtype=bool)                    # 层界
    bnd[:, 1:] |= (lvl[:, 1:] != lvl[:, :-1])
    bnd[1:, :] |= (lvl[1:, :] != lvl[:-1, :])
    blocked |= bnd
    w0 = z < WATER_Z                                        # 岸线：水陆交界两侧
    sedge = np.zeros(z.shape, dtype=bool)
    for _dy in (-1, 0, 1):
        for _dx in (-1, 0, 1):
            sedge |= w0 != np.roll(np.roll(w0, _dy, 0), _dx, 1)
    blocked |= sedge
    blocked |= ramp_cover_mask((H, W), ramps or [])        # 斜坡占地
    # 可玩区外缘 3 格是图边禁建带。矿口射线伸进这里，矿场就摆不下去。
    _yyb, _xxb = np.mgrid[0:H, 0:W]
    _db = np.minimum.reduce([_xxb - BORDER, _yyb - BORDER,
                             (GRID - 1 - BORDER) - _xxb,
                             (GRID - 1 - BORDER) - _yyb])
    blocked |= _db < 3.0
    pad = r1 + 2
    cp = np.ones((H + 2 * pad, W + 2 * pad), dtype=bool)
    cp[pad:pad + H, pad:pad + W] = blocked
    return cp, pad


def mouth_open_map(z, T=None, ramps=None):
    """每格：是否存在一条斜向射线（r0..r1 格）不撞 层界/岸线/斜坡/不可通行。
    用于矿位候选过滤 —— 保证"矿口前方开阔"是可落位的属性，而不是事后祈祷。"""
    T = T if T is not None else {}
    r0 = int(T.get('ore_mouth_r0', 8))
    r1 = int(T.get('ore_mouth_r1', 20))
    cp, pad = _mouth_blocked_map(z, T, ramps)
    clear = ~cp
    ok_any = np.zeros_like(clear)
    H, W = z.shape
    for dx, dy in ((1, 1), (1, -1), (-1, 1), (-1, -1)):
        run = np.ones_like(clear)
        for r in range(r0, r1 + 1):
            run &= np.roll(np.roll(clear, -dy * r, 0), -dx * r, 1)
        ok_any |= run
    return ok_any[pad:pad + H, pad:pad + W]


def _mouth_ray_clear(cp, pad, x, y, a, r0, r1):
    """矿口射线（角度 a，r0..r1 格）是否全程无障。x/y 为未加 pad 的格坐标。"""
    dx, dy = math.cos(a), math.sin(a)
    for r in range(r0, r1 + 1):
        ix = int(round(x + dx * r)) + pad
        iy = int(round(y + dy * r)) + pad
        if cp[iy, ix]:
            return False
    return True


def enforce_ore_mouth_open(z, base_cells, ore_list, T=None, verbose=True, ramps=None, avoid=None):
    """矿口朝向收尾：保证每个矿的朝向射线前方开阔（不撞 层界/岸线/斜坡/不可通行）。

    规则：
    - 基地旁（<= ore_near_max_r）矿口**朝基地**（优先级最高）；
      例外豁免：**水上基地**的近矿隔着水面朝基地是设计本意，不处理。
    - 其余矿朝"最开阔斜向"；若当前朝向在 r1 格内撞障，或矿口半圆跨上层界/岸线/斜坡，
      或矿心离可玩边界不够，先试同点的其他斜向，不行就在附近挪位。
      仍放不下就跳过该矿，不把不合规的位置留下来。
    返回 (矿位列表, 矿口角度列表) —— 角度在这里**最终定稿**。
    """
    T = T if T is not None else {}
    H, W = z.shape
    r0 = int(T.get('ore_mouth_r0', 8))
    r1 = int(T.get('ore_mouth_r1', 20))
    near_r = float(T.get('ore_near_max_r', 62.0))
    min_r = float(T.get('ore_near_min_r', 44.0))
    spacing = float(getattr(cfg, 'ORE_MIN_SPACING', 46.0))
    cp, pad = _mouth_blocked_map(z, T, ramps)
    layers = land_layer_index(z, T)
    d_land = distance_transform(z >= WATER_Z)      # 水面矿要求离岸 >=4 格
    _yyb, _xxb = np.mgrid[0:H, 0:W]
    d_border = np.minimum.reduce([_xxb - BORDER, _yyb - BORDER,
                                  (GRID - 1 - BORDER) - _xxb,
                                  (GRID - 1 - BORDER) - _yyb]).astype(np.float64)
    _border_clear = float(T.get('ore_border_clear', 30.0))
    d_edge_j, d_shore_j = ore_open_maps(z, T)
    _ecl = float(T.get('ore_edge_clear', 16.0))
    _scl = float(T.get('ore_shore_clear', 12.0))

    def away_from_junction(x, y):
        """近矿也离开层界和岸线，距离用配方里的 ore_edge_clear / ore_shore_clear。"""
        return bool(d_edge_j[y, x] >= _ecl and d_shore_j[y, x] >= _scl)
    gy, gx = np.gradient(z)
    slope = np.sqrt(gx * gx + gy * gy)
    DIAG = ((1, 1), (1, -1), (-1, 1), (-1, -1))

    # ---- 矿场地基（v11 不压平，靠选址）：
    #      矿场摆在矿点上，建造范围是绕矿点的整圆盘，不是矿口前方的扇面。
    #      扇面会把矿点侧后方的层坎漏掉（实测坎在 15~19 格，核心盘只查到 12）。
    #      圆盘半径 = max(ORE_CLEAR_RADIUS, ore_mouth_r1)，高差 <=12。矿心还要离开可玩边界。
    _build_r = max(float(ORE_CLEAR_RADIUS), float(r1))
    _pr = int(math.ceil(_build_r))
    _oy, _ox = np.mgrid[-_pr:_pr + 1, -_pr:_pr + 1]
    _od = np.sqrt(_ox ** 2 + _oy ** 2).astype(np.float64)
    _disc = _od <= _build_r

    def pad_flat(x, y, a):
        if not (0 <= x < W and 0 <= y < H):
            return False
        if d_border[y, x] < _border_clear:
            return False                                  # 离图边太近，矿场压进禁建带
        if z[y, x] < WATER_Z:
            return True                                   # 水面矿：水面恒平
        if x < _pr or y < _pr or x >= W - _pr or y >= H - _pr:
            return False
        win = z[y - _pr:y + _pr + 1, x - _pr:x + _pr + 1]
        return float(win[_disc].max() - win[_disc].min()) <= 12.0

    def nearest_base(x, y):
        return min(base_cells.items(),
                   key=lambda kv: (kv[1][0] - x) ** 2 + (kv[1][1] - y) ** 2)

    def ranked_open_angles(x, y):
        """4 个斜向按开阔度从高到低。打平时朝离开最近基地的一侧，避免外矿全朝一个方向。"""
        _bn, bc = nearest_base(x, y)
        ax, ay = float(x) - float(bc[0]), float(y) - float(bc[1])
        nrm = math.hypot(ax, ay) or 1.0
        ax, ay = ax / nrm, ay / nrm
        scored = []
        for dx, dy in DIAG:
            a = math.atan2(dy, dx)
            s = 0.0
            for r in range(r0, r1 + 1):
                ix = int(round(x + dx * r)) + pad
                iy = int(round(y + dy * r)) + pad
                if cp[iy, ix]:
                    break
                s += 1.0
            if _mouth_ray_clear(cp, pad, x, y, a, r0, r1):
                s += 100.0
            scored.append((s, ax * dx + ay * dy, a))
        scored.sort(reverse=True)
        return [a for _s, _away, a in scored]

    def best_open_angle(x, y):
        """4 斜向里选最开阔的（射线得分：全程畅通 r1 格的优先）。"""
        return ranked_open_angles(x, y)[0]

    def face_base_angle(x, y, bx, by):
        a_src = math.atan2(by - y, bx - x)
        bi = max(range(4), key=lambda i: math.cos(a_src) * DIAG[i][0] + math.sin(a_src) * DIAG[i][1])
        return math.atan2(DIAG[bi][1], DIAG[bi][0])

    def find_ring(bx, by, base_water, placed, prefer_ang, allow_water=True):
        """在基地 44~62 格环上找一格：朝基地，且矿场放得下。

        陆上基地先只找同层陆地；allow_water 为真时才接受近矿环里的水面。
        水上基地的近矿只能落在水里。
        """
        for rr in range(int(math.ceil(min_r)), int(math.floor(near_r)) + 1):
            steps = max(36, int(rr * 3))
            for k in range(steps):
                ang = prefer_ang + 2.0 * math.pi * k / steps
                x2 = int(round(bx + math.cos(ang) * rr))
                y2 = int(round(by + math.sin(ang) * rr))
                if not (0 <= x2 < W and 0 <= y2 < H):
                    continue
                if d_border[y2, x2] < _border_clear:
                    continue
                if not away_from_junction(x2, y2):
                    continue
                wet = bool(z[y2, x2] < WATER_Z)
                if wet and not allow_water:
                    continue
                if not near_ore_surface_ok(z, bx, by, x2, y2, T, layers):
                    continue
                if wet:
                    if d_land[y2, x2] < 4.0:
                        continue
                elif slope[y2, x2] >= 2.5:
                    continue
                if any(math.hypot(x2 - px, y2 - py) < spacing for px, py in placed):
                    continue
                a2 = face_base_angle(x2, y2, bx, by)
                if not pad_flat(x2, y2, a2):
                    continue
                if (not wet) and not _mouth_ray_clear(cp, pad, x2, y2, a2, r0, r1):
                    continue
                return (x2, y2, a2)
        return None

    def take_ring(bx, by, base_water, placed, prefer):
        if base_water:
            return find_ring(bx, by, base_water, placed, prefer, True)
        spot = find_ring(bx, by, base_water, placed, prefer, False)
        if spot is None:
            spot = find_ring(bx, by, base_water, placed, prefer, True)
        return spot

    out, angs = [], []
    moved = rotated = 0
    for o in ore_list:
        ox, oy = float(o[0]), float(o[1])
        ix, iy = int(round(ox)), int(round(oy))
        bn, bc = nearest_base(ox, oy)
        bd = math.hypot(ox - bc[0], oy - bc[1])
        base_water = bool(z[int(round(bc[1])), int(round(bc[0]))] < WATER_Z)
        is_near = bd <= near_r
        a = (math.atan2(bc[1] - oy, bc[0] - ox) if is_near else best_open_angle(ix, iy))
        # 朝基地的角量化到最近斜向（与游戏里矿场的 4 向摆法一致）
        if is_near:
            best_i = max(range(4),
                         key=lambda i: math.cos(a) * DIAG[i][0] + math.sin(a) * DIAG[i][1])
            a = math.atan2(DIAG[best_i][1], DIAG[best_i][0])
        # 通过条件：朝向射线畅通（水上基地近矿过水面豁免）+ 地基平整。
        # 贴在基地上的矿即使垫子是平的也不能留（建造场会被矿场卡住）。
        _ray_ok = _mouth_ray_clear(cp, pad, ix, iy, a, r0, r1)
        _pad_ok = pad_flat(ix, iy, a)
        _junc_ok = (not is_near) or away_from_junction(ix, iy)
        _too_close = is_near and bd < min_r - 0.5
        _ore_wet = bool(0 <= ix < W and 0 <= iy < H and z[iy, ix] < WATER_Z)
        _surf_ok = (not is_near) or near_ore_surface_ok(z, bc[0], bc[1], ox, oy, T, layers)
        if (not _too_close) and _ray_ok and _pad_ok and _junc_ok and _surf_ok:
            out.append((ox, oy)); angs.append(a)
            continue
        # 近矿落在水上、朝基地穿过水面：水面不算撞障。矿心仍然离开层界和岸线。
        if (not _too_close) and is_near and _ore_wet and _pad_ok and _junc_ok and _surf_ok:
            out.append((ox, oy)); angs.append(a)
            continue
        # ① 同点换向（非近矿）：射线畅通且地基平整，优先更开阔的斜向
        fixed = False
        if not is_near:
            for a2 in ranked_open_angles(ix, iy):
                if _mouth_ray_clear(cp, pad, ix, iy, a2, r0, r1) and pad_flat(ix, iy, a2):
                    out.append((ox, oy)); angs.append(a2)
                    rotated += 1
                    fixed = True
                    break
        if fixed:
            continue
        # ② 挪位。近矿只在 44~62 环上重找（仍朝基地）；远矿不准掉进这个环。
        placed = out + [(float(p[0]), float(p[1])) for p in ore_list if p is not o]
        for _ap in (avoid or ()):
            placed.append((float(_ap[0]), float(_ap[1])))
        best = None
        if is_near:
            prefer = math.atan2(oy - bc[1], ox - bc[0])
            best = take_ring(bc[0], bc[1], base_water, placed, prefer)
        else:
            onwater = bool(z[iy, ix] < WATER_Z)
            tries = [(onwater, 100)]
            if base_water and not onwater:
                tries.append((True, 100))
            bk = None
            for want_water, R in tries:
                for rr in range(1, R + 1):
                    for ddy in range(-rr, rr + 1):
                        for ddx in range(-rr, rr + 1):
                            if max(abs(ddx), abs(ddy)) != rr:
                                continue
                            x2, y2 = ix + ddx, iy + ddy
                            if not (0 <= x2 < W and 0 <= y2 < H):
                                continue
                            if d_border[y2, x2] < _border_clear:
                                continue
                            if (z[y2, x2] < WATER_Z) != want_water:
                                continue
                            if want_water:
                                if d_land[y2, x2] < 4.0:
                                    continue
                            elif slope[y2, x2] >= 2.5:
                                continue
                            d2 = math.hypot(x2 - bc[0], y2 - bc[1])
                            if d2 <= near_r:
                                continue                    # 别掉进近矿环，否则会多出一个不朝基地的近矿
                            if any(math.hypot(x2 - px, y2 - py) < spacing for px, py in placed):
                                continue
                            pref = best_open_angle(x2, y2)
                            cand_ang = [pref]
                            for dx, dy in DIAG:
                                a_try = math.atan2(dy, dx)
                                if abs((a_try - pref + math.pi) % (2 * math.pi) - math.pi) > 0.2:
                                    cand_ang.append(a_try)
                            chosen = None
                            for a2 in cand_ang:
                                if not pad_flat(x2, y2, a2):
                                    continue
                                if not _mouth_ray_clear(cp, pad, x2, y2, a2, r0, r1):
                                    continue
                                chosen = a2
                                break
                            if chosen is None:
                                continue
                            if bk is None or -rr > bk:
                                bk, best = -rr, (x2, y2, chosen)
                    if best is not None:
                        break
                if best is not None:
                    break
        if best is None:
            if verbose:
                print('    [矿口] (%d,%d) 离边或朝向层界/水岸，附近放不下，跳过' % (ix, iy))
            continue
        out.append((float(best[0]), float(best[1])))
        angs.append(best[2])
        moved += 1
        if verbose:
            print('    [矿口] (%d,%d) 撞障/地基不平 -> 挪到 (%d,%d)' % (ix, iy, best[0], best[1]))
    # 近矿若被跳过，在 44~62 环上补回，仍然朝基地。
    for bn, bcell in base_cells.items():
        bx, by = float(bcell[0]), float(bcell[1])
        base_water = bool(z[int(round(by)), int(round(bx))] < WATER_Z)
        have = sum(1 for o in out if min_r - 0.5 <= math.hypot(o[0] - bx, o[1] - by) <= near_r + 0.5)
        # 近矿环外、还没离开一整段间距的矿，会把环里剩下的平地挡住。先把它们推远。
        if have < 2:
            _far0 = int(near_r + spacing) + 2
            for idx in range(len(out)):
                ox, oy = float(out[idx][0]), float(out[idx][1])
                d0 = math.hypot(ox - bx, oy - by)
                if not (near_r < d0 < _far0 - 0.5):
                    continue
                if nearest_base(ox, oy)[0] != bn:
                    continue
                ix0, iy0 = int(round(ox)), int(round(oy))
                ang0 = math.atan2(oy - by, ox - bx)
                onwater = bool(z[iy0, ix0] < WATER_Z)
                best = None
                for rr in range(_far0, _far0 + 80):
                    for da in (0.0, 0.5, -0.5, 1.0, -1.0, 1.6, -1.6, 2.2, -2.2):
                        x2 = int(round(bx + math.cos(ang0 + da) * rr))
                        y2 = int(round(by + math.sin(ang0 + da) * rr))
                        if not (0 <= x2 < W and 0 <= y2 < H):
                            continue
                        if d_border[y2, x2] < _border_clear:
                            continue
                        if (z[y2, x2] < WATER_Z) != onwater:
                            continue
                        if onwater:
                            if d_land[y2, x2] < 4.0:
                                continue
                        elif slope[y2, x2] >= 2.5:
                            continue
                        if any(math.hypot(x2 - float(bc[0]), y2 - float(bc[1])) <= near_r
                               for bc in base_cells.values()):
                            continue
                        if any(math.hypot(x2 - p[0], y2 - p[1]) < spacing
                               for j, p in enumerate(out) if j != idx):
                            continue
                        a2 = best_open_angle(x2, y2)
                        if not pad_flat(x2, y2, a2):
                            continue
                        if not onwater and not _mouth_ray_clear(cp, pad, x2, y2, a2, r0, r1):
                            continue
                        best = (x2, y2, a2)
                        break
                    if best is not None:
                        break
                if best is None:
                    continue
                out[idx] = (float(best[0]), float(best[1]))
                angs[idx] = best[2]
                moved += 1
                if verbose:
                    print('    [矿口] %s 环外矿 (%d,%d) 推到 (%d,%d)，给近矿环让位'
                          % (bn, ix0, iy0, best[0], best[1]))
            have = sum(1 for o in out
                       if min_r - 0.5 <= math.hypot(o[0] - bx, o[1] - by) <= near_r + 0.5)
        for _k in range(2 - have):
            placed = list(out)
            for _ap in (avoid or ()):
                placed.append((float(_ap[0]), float(_ap[1])))
            spot = take_ring(bx, by, base_water, placed, _k * 1.7)
            if spot is None:
                if verbose:
                    print('    [矿口] %s 近矿环上补不出第 %d 个' % (bn, have + _k + 1))
                break
            out.append((float(spot[0]), float(spot[1])))
            angs.append(spot[2])
            moved += 1
            if verbose:
                print('    [矿口] %s 近矿环补 (%d,%d)' % (bn, spot[0], spot[1]))
    if verbose and (moved or rotated):
        print('  [矿口] 朝向修正：换向 %d 个、挪位 %d 个' % (rotated, moved))
    return out, angs


def apply_ore_place_at(z, base_cells, ore_list, angles, T=None, verbose=True):
    """配方 ore_place_at 把个别矿挪到指定格。没写这项的图保持原落点。

    每条是 (from_x, from_y, to_x, to_y)，from 是生成落点，容差 6 格。
    挪到水上的近矿仍朝最近的基地。
    """
    spec = list((T or {}).get('ore_place_at') or ())
    if not spec or not ore_list:
        return ore_list, angles
    ores = [tuple(o) for o in ore_list]
    angs = list(angles) if angles else [0.0] * len(ores)
    DIAG = ((1, 1), (1, -1), (-1, 1), (-1, -1))
    H, W = z.shape
    for item in spec:
        fx, fy, tx, ty = (float(item[0]), float(item[1]), float(item[2]), float(item[3]))
        ix, iy = int(round(tx)), int(round(ty))
        if not (0 <= ix < W and 0 <= iy < H):
            if verbose:
                print('  [矿位] 指定落点 (%d,%d) 超出地图' % (ix, iy))
            continue
        best_i, best_d = None, 1e9
        for i, o in enumerate(ores):
            d = math.hypot(float(o[0]) - fx, float(o[1]) - fy)
            if d < best_d:
                best_d, best_i = d, i
        if best_i is None or best_d > 6.0:
            if verbose:
                print('  [矿位] 指定落点 (%.0f,%.0f) 附近没有矿' % (fx, fy))
            continue
        ores[best_i] = (float(ix), float(iy))
        if base_cells:
            bc = min(base_cells.values(),
                     key=lambda c: (float(c[0]) - ix) ** 2 + (float(c[1]) - iy) ** 2)
            a = math.atan2(float(bc[1]) - iy, float(bc[0]) - ix)
            bi = max(range(4), key=lambda k: math.cos(a) * DIAG[k][0] + math.sin(a) * DIAG[k][1])
            if best_i < len(angs):
                angs[best_i] = math.atan2(DIAG[bi][1], DIAG[bi][0])
        if verbose:
            wet = '水面' if z[iy, ix] < WATER_Z else '陆地'
            print('  [矿位] (%.0f,%.0f) 按配方挪到 (%d,%d) %s' % (fx, fy, ix, iy, wet))
    return ores, angs


def apply_derrick_place_at(derricks, T=None, verbose=True):
    """配方 derrick_place_at 把个别油井挪到指定格。没写这项的图保持原落点。

    每条是 (from_x, from_y, to_x, to_y)，from 是生成落点，容差 6 格。
    """
    spec = list((T or {}).get('derrick_place_at') or ())
    if not spec or not derricks:
        return list(derricks)
    out = [tuple(c) for c in derricks]
    for item in spec:
        fx, fy, tx, ty = (float(item[0]), float(item[1]), float(item[2]), float(item[3]))
        best_i, best_d = None, 1e9
        for i, c in enumerate(out):
            d = math.hypot(float(c[0]) - fx, float(c[1]) - fy)
            if d < best_d:
                best_d, best_i = d, i
        if best_i is None or best_d > 6.0:
            if verbose:
                print('  [油井] 指定落点 (%.0f,%.0f) 附近没有油井' % (fx, fy))
            continue
        out[best_i] = (float(int(round(tx))), float(int(round(ty))))
        if verbose:
            print('  [油井] (%.0f,%.0f) 按配方挪到 (%d,%d)' % (fx, fy, int(round(tx)), int(round(ty))))
    return out


def apply_ore_face_at(ore_list, angles, T=None, verbose=True):
    """配方 ore_face_at 指定个别矿的斜向。没写这项的图保持算出的朝向。

    每条是 (x, y, dx, dy)，dx/dy 为内部帧的四个斜向之一（+y 向南）。
    目标格 3 格内没有矿时，可再写 (from_x, from_y)，把那一颗矿放回目标格。
    返回 (矿位, 角度)。
    """
    spec = list((T or {}).get('ore_face_at') or ())
    if not spec or not angles:
        return list(ore_list), angles
    ores = [tuple(map(float, o)) for o in ore_list]
    angs = list(angles)
    for item in spec:
        x, y, dx, dy = (float(item[0]), float(item[1]), float(item[2]), float(item[3]))
        best_i, best_d = None, 1e9
        for i, o in enumerate(ores):
            d = math.hypot(o[0] - x, o[1] - y)
            if d < best_d:
                best_d, best_i = d, i
        if best_i is None or best_d > 3.0:
            src = None
            if len(item) >= 6:
                sx, sy = float(item[4]), float(item[5])
                src_d = 1e9
                for i, o in enumerate(ores):
                    d = math.hypot(o[0] - sx, o[1] - sy)
                    if d < src_d:
                        src_d, src = d, i
                if src is None or src_d > 3.0:
                    src = None
            if src is None:
                if verbose:
                    print('  [矿口] 指定朝向 (%.0f,%.0f) 附近没有矿' % (x, y))
                continue
            if verbose:
                print('  [矿口] (%.0f,%.0f) 放回 (%.0f,%.0f)'
                      % (ores[src][0], ores[src][1], x, y))
            ores[src] = (x, y)
            best_i = src
        angs[best_i] = math.atan2(dy, dx)
        if verbose:
            print('  [矿口] (%.0f,%.0f) 按配方朝向 (%d,%d)'
                  % (ores[best_i][0], ores[best_i][1], int(dx), int(dy)))
    return ores, angs


def trim_outer_ore_quota(ore_list, angles, base_cells, T=None, verbose=True):
    """外围矿收到每名玩家 ore_outer_per_player 个。近矿环里挤出来的先去掉。"""
    T = T if T is not None else {}
    near_r = float(T.get('ore_near_max_r', 62.0))
    bases = [c for c in (base_cells or {}).values() if c is not None]
    if not bases:
        return ore_list, angles
    want = int(T.get('ore_outer_per_player', 4)) * len(bases)
    outer = []
    for i, o in enumerate(ore_list):
        d = min(math.hypot(o[0] - c[0], o[1] - c[1]) for c in bases)
        if d > near_r:
            outer.append((d, i))
    extra = len(outer) - want
    if extra <= 0:
        return ore_list, angles
    outer.sort()
    drop = set(i for _d, i in outer[:extra])
    kept_o, kept_a = [], []
    for i, o in enumerate(ore_list):
        if i in drop:
            continue
        kept_o.append(o)
        if angles and i < len(angles):
            kept_a.append(angles[i])
    if verbose:
        print('  [矿位] 外围矿裁到 %d 个，去掉 %d 个刚挤出近矿环的' % (want, extra))
    return kept_o, kept_a


def _walk_mask(z, ramps):
    """陆上可走格：坡度封死、岸线封死和高山之外。斜坡开口留在岸线豁免里。"""
    gy, gx = np.gradient(z)
    slope = np.sqrt(gx * gx + gy * gy)
    shore = shore_block_mask(z, ramps or [])
    yy, xx = np.mgrid[0:z.shape[0], 0:z.shape[1]]
    inset = np.minimum.reduce([
        xx - BORDER, yy - BORDER,
        GRID - 1 - BORDER - xx, GRID - 1 - BORDER - yy])
    return ((z >= WATER_Z) & (slope <= SLOPE_IMPASSABLE)
            & ~shore & (z <= WATER_Z + PEAK_IMPASSABLE) & (inset >= 1.0))


def _bfs_from(walk, sx, sy):
    """从一格做 8 邻接距离。到不了的格子是 -1。"""
    from collections import deque
    H, W = walk.shape
    sx, sy = int(sx), int(sy)
    dist = np.full((H, W), -1, np.int32)
    parent = np.full((H, W), -1, np.int32)
    if not (0 <= sx < W and 0 <= sy < H and walk[sy, sx]):
        found = None
        for rad in range(1, 16):
            for dy in range(-rad, rad + 1):
                for dx in range(-rad, rad + 1):
                    if max(abs(dx), abs(dy)) != rad:
                        continue
                    x, y = sx + dx, sy + dy
                    if 0 <= x < W and 0 <= y < H and walk[y, x]:
                        found = (x, y)
                        break
                if found:
                    break
            if found:
                break
        if found is None:
            return dist, parent
        sx, sy = found
    q = deque([(sx, sy)])
    dist[sy, sx] = 0
    while q:
        x, y = q.popleft()
        base = int(dist[y, x]) + 1
        for dy in (-1, 0, 1):
            ny = y + dy
            if ny < 0 or ny >= H:
                continue
            row = walk[ny]
            for dx in (-1, 0, 1):
                if dx == 0 and dy == 0:
                    continue
                nx = x + dx
                if nx < 0 or nx >= W or not row[nx] or dist[ny, nx] >= 0:
                    continue
                dist[ny, nx] = base
                parent[ny, nx] = y * W + x
                q.append((nx, ny))
    return dist, parent


def _disc_flat(z, x, y, radius, limit=12.0):
    H, W = z.shape
    r = int(math.ceil(radius))
    if x < r or y < r or x >= W - r or y >= H - r:
        return False
    win = z[y - r:y + r + 1, x - r:x + r + 1]
    yy, xx = np.ogrid[-r:r + 1, -r:r + 1]
    vals = win[(yy * yy + xx * xx) <= radius * radius]
    if vals.size == 0:
        return False
    return float(vals.max() - vals.min()) <= limit


def apply_ore_bands(z, base_cells, ore_list, angles, T=None, ramps=None, avoid=None, verbose=True):
    """近矿 2 个、中矿 2 个标给出生点，远矿 2 个不标。

    没开 ore_bands 的图不进这里。近矿沿用已经落好的那两个。中矿落在该基地
    走得到的陆地上，并离开所有近矿环。远矿在更外一圈；可玩水域达到 20% 时改放水面。
    """
    T = T if T is not None else {}
    if not T.get('ore_bands'):
        return ore_list, angles, None, None, None
    H, W = z.shape
    bases = [(n, (float(c[0]), float(c[1]))) for n, c in base_cells.items()]
    if not bases:
        return ore_list, angles, None, None, None
    near_r = float(T.get('ore_near_max_r', 62.0))
    mid_lo = near_r + float(T.get('ore_mid_gap', 20.0))
    mid_hi = float(T.get('ore_far_min_dist', 160.0))
    if mid_hi <= mid_lo + 10.0:
        mid_hi = mid_lo + 80.0
    far_lo = mid_hi
    spacing = float(getattr(cfg, 'ORE_MIN_SPACING', 46.0))
    if spacing < 46.0:
        spacing = 46.0
    build_r = max(float(ORE_CLEAR_RADIUS), float(T.get('ore_mouth_r1', 20)))
    gy, gx = np.gradient(z)
    slope = np.sqrt(gx * gx + gy * gy)
    d_layer, d_shore = ore_open_maps(z, T)
    yy, xx = np.mgrid[0:H, 0:W]
    border = np.minimum.reduce([
        xx - BORDER, yy - BORDER,
        GRID - 1 - BORDER - xx, GRID - 1 - BORDER - yy])
    steep = ((slope > SLOPE_IMPASSABLE) | (z > WATER_Z + PEAK_IMPASSABLE)
             | shore_block_mask(z, ramps or []))
    d_bad = distance_transform(steep)
    mouth = mouth_open_map(z, T, ramps)
    land_ok = ((z >= WATER_Z) & (slope < 2.5) & (d_bad >= build_r + 1.0)
               & (d_layer >= build_r + 1.0) & (d_shore >= float(T.get('ore_shore_clear', 12.0)))
               & (border >= float(T.get('ore_border_clear', 30.0))) & mouth)
    inside = border >= 0
    water_frac = float(((z < WATER_Z) & inside).sum()) / max(1, int(inside.sum()))
    far_on_water = water_frac >= 0.20
    if far_on_water:
        d_land = distance_transform(z >= WATER_Z)
        water_ok = ((z < WATER_Z) & (d_land >= build_r + 1.0)
                    & (border >= float(T.get('ore_border_clear', 30.0))))
    else:
        water_ok = np.zeros(z.shape, dtype=bool)
    walk = _walk_mask(z, ramps)
    claimed = set()
    near_o, near_a, near_own = [], [], []
    for bi, (_name, c) in enumerate(bases):
        ranked = []
        for i, o in enumerate(ore_list):
            if i in claimed:
                continue
            d = math.hypot(float(o[0]) - c[0], float(o[1]) - c[1])
            if d <= near_r:
                ranked.append((d, i))
        ranked.sort()
        for _d, i in ranked[:2]:
            claimed.add(i)
            near_o.append((float(ore_list[i][0]), float(ore_list[i][1])))
            near_a.append(float(angles[i]) if angles and i < len(angles) else 0.0)
            near_own.append(bi)
    placed = list(near_o)
    for p in (avoid or ()):
        placed.append((float(p[0]), float(p[1])))

    def _blocked(x, y, extra=0.0):
        need = spacing if extra <= 0 else extra
        return any(math.hypot(x - px, y - py) < need for px, py in placed)

    def _away_bases(x, y):
        return all(math.hypot(x - c[0], y - c[1]) > near_r + 0.5 for _n, c in bases)

    def _pick(mask, score_at, limit=240):
        step = 4
        best = None
        ys = range(BORDER + 4, H - BORDER - 4, step)
        xs = range(BORDER + 4, W - BORDER - 4, step)
        cands = []
        for y in ys:
            row = mask[y]
            for x in xs:
                if not row[x] or not _away_bases(x, y) or _blocked(x, y):
                    continue
                sc = score_at(x, y)
                if sc > -1e8:
                    cands.append((sc, x, y))
        cands.sort(reverse=True)
        for _s, x, y in cands[:limit]:
            if z[y, x] >= WATER_Z and not _disc_flat(z, x, y, build_r):
                continue
            return (x, y)
        return best

    DIAG = ((1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0))
    cp, pad = _mouth_blocked_map(z, T, ramps)
    r0 = int(T.get('ore_mouth_r0', 8))
    r1 = int(T.get('ore_mouth_r1', 20))

    def _angle(x, y):
        for dx, dy in DIAG:
            a = math.atan2(dy, dx)
            if _mouth_ray_clear(cp, pad, x, y, a, r0, r1):
                return a
        return math.atan2(1.0, 1.0)

    mid_o, mid_a, mid_own = [], [], []
    far_o, far_a = [], []
    for bi, (name, c) in enumerate(bases):
        dist, parent = _bfs_from(walk, c[0], c[1])
        others = [b[1] for j, b in enumerate(bases) if j != bi]
        mx = sum(p[0] for p in others) / len(others)
        my = sum(p[1] for p in others) / len(others)
        out_ang = math.atan2(c[1] - my, c[0] - mx)
        for k, center in enumerate((out_ang - 0.7, out_ang + 0.7)):
            def _score(x, y, _c=c, _center=center, _dist=dist):
                eu = math.hypot(x - _c[0], y - _c[1])
                if not (mid_lo <= eu <= mid_hi):
                    return -1e9
                ang = math.atan2(y - _c[1], x - _c[0])
                dang = (ang - _center + math.pi) % (2 * math.pi) - math.pi
                if abs(dang) > 0.6:
                    return -1e9
                pd = int(_dist[y, x])
                if pd < 0:
                    return -1e9
                target = (mid_lo + mid_hi) * 0.5
                return -abs(eu - target) - 0.04 * pd

            cell = _pick(land_ok, _score)
            if cell is None:
                def _score2(x, y, _c=c, _dist=dist):
                    eu = math.hypot(x - _c[0], y - _c[1])
                    if not (mid_lo <= eu <= mid_hi + 80.0):
                        return -1e9
                    pd = int(_dist[y, x])
                    if pd < 0:
                        return -1e9
                    return -abs(eu - mid_lo - 30.0) - 0.04 * pd
                cell = _pick(land_ok, _score2)
            if cell is None:
                if verbose:
                    print('  [矿带] %s 中矿 %d 放不下' % (name, k + 1))
                continue
            x, y = cell
            mid_o.append((float(x), float(y)))
            mid_a.append(_angle(x, y))
            mid_own.append(bi)
            placed.append((float(x), float(y)))

    n_far = 2 * len(bases)
    far_mask = water_ok if far_on_water else land_ok
    cx0, cy0 = (W - 1) * 0.5, (H - 1) * 0.5
    for _k in range(n_far):
        def _far_score(x, y):
            db = min(math.hypot(x - c[0], y - c[1]) for _n, c in bases)
            if db < far_lo:
                return -1e9
            spread = min((math.hypot(x - p[0], y - p[1]) for p in far_o), default=200.0)
            return spread + 0.1 * db - 0.02 * math.hypot(x - cx0, y - cy0)

        cell = _pick(far_mask, _far_score, limit=400)
        if cell is None and not far_on_water:
            def _far_loose(x, y):
                db = min(math.hypot(x - c[0], y - c[1]) for _n, c in bases)
                if db < mid_lo:
                    return -1e9
                spread = min((math.hypot(x - p[0], y - p[1]) for p in far_o), default=200.0)
                return spread + 0.05 * db
            cell = _pick(far_mask, _far_loose, limit=400)
        if cell is None:
            if verbose:
                print('  [矿带] 远矿放不下（已有 %d / %d）' % (len(far_o), n_far))
            break
        x, y = cell
        far_o.append((float(x), float(y)))
        far_a.append(_angle(x, y) if z[y, x] >= WATER_Z else math.atan2(1.0, 1.0))
        placed.append((float(x), float(y)))

    ore = near_o + mid_o + far_o
    ang = near_a + mid_a + far_a
    kind = ([0] * len(near_o)) + ([1] * len(mid_o)) + ([2] * len(far_o))
    owner = list(near_own) + list(mid_own) + ([-1] * len(far_o))
    if verbose:
        print('  [矿带] 近矿 %d、中矿 %d、远矿 %d（%s）' % (
            len(near_o), len(mid_o), len(far_o),
            '远矿在水面' if far_on_water else '远矿在陆地'))
    return ore, ang, kind, owner


def trim_near_ore_quota(z, base_cells, ore_list, T=None, keep=2, verbose=True, ramps=None, avoid=None):
    """近矿超员裁剪：每基地 ore_near_max_r 内**恰好 keep 个**（保底补矿 + 清洗挪位可能叠加出 3 个）。
    超出的矿挪到近矿环之外一整段间距（否则会贴着环、把还能放的近矿挡住），挪不动才删除。"""
    T = T if T is not None else {}
    H, W = z.shape
    near_r = float(T.get('ore_near_max_r', 62.0))
    spacing = float(getattr(cfg, 'ORE_MIN_SPACING', 46.0))
    d_edge, d_shore = ore_open_maps(z, T)
    _ecl = float(T.get('ore_edge_clear', 16.0))
    _scl = float(T.get('ore_shore_clear', 12.0))
    mopen = mouth_open_map(z, T, ramps)
    gy, gx = np.gradient(z)
    slope = np.sqrt(gx * gx + gy * gy)
    out = list(ore_list)
    for bn, c in base_cells.items():
        bx, by = float(c[0]), float(c[1])
        near = sorted((o for o in out
                       if math.hypot(o[0] - bx, o[1] - by) <= near_r),
                      key=lambda o: math.hypot(o[0] - bx, o[1] - by))
        for extra in near[keep:]:
            ix, iy = int(round(extra[0])), int(round(extra[1]))
            onwater = bool(z[iy, ix] < WATER_Z)
            ang0 = math.atan2(iy - by, ix - bx)
            best = None
            # 至少离开近矿环一整段矿间距，避免贴在环外把第二个近矿挡住。
            _far0 = int(near_r + spacing) + 2
            _border_clear = float(T.get('ore_border_clear', 30.0))
            for rr in range(_far0, _far0 + 70):
                cands = []
                for da in (0.0, 0.4, -0.4, 0.8, -0.8, 1.2, -1.2, 1.6, -1.6):
                    x2 = int(round(bx + math.cos(ang0 + da) * rr))
                    y2 = int(round(by + math.sin(ang0 + da) * rr))
                    if not (0 <= x2 < W and 0 <= y2 < H):
                        continue
                    if min(x2 - BORDER, y2 - BORDER,
                           GRID - 1 - BORDER - x2, GRID - 1 - BORDER - y2) < _border_clear:
                        continue
                    if (z[y2, x2] < WATER_Z) != onwater:
                        continue
                    if not onwater and slope[y2, x2] >= 2.5:
                        continue
                    if d_edge[y2, x2] < _ecl or d_shore[y2, x2] < _scl:
                        continue
                    if not mopen[y2, x2]:
                        continue
                    if any(math.hypot(x2 - float(c2[0]), y2 - float(c2[1])) <= near_r
                           for c2 in base_cells.values()):
                        continue
                    if any(math.hypot(x2 - p[0], y2 - p[1]) < spacing
                           for p in out if p is not extra):
                        continue
                    if any(math.hypot(x2 - float(a[0]), y2 - float(a[1])) < spacing
                           for a in (avoid or ())):
                        continue
                    cands.append((abs(da), x2, y2))
                if cands:
                    cands.sort()
                    best = (cands[0][1], cands[0][2])
                    break
            if best is not None:
                out[out.index(extra)] = (float(best[0]), float(best[1]))
                if verbose:
                    print('    [矿位] %s 近矿超员：(%d,%d) 挪到远矿环 (%d,%d)'
                          % (bn, ix, iy, best[0], best[1]))
            else:
                out.remove(extra)
                if verbose:
                    print('    [矿位] %s 近矿超员：(%d,%d) 挪不动，删除' % (bn, ix, iy))
    return out


def connect_water_bases(z, sea_pts, land, T=None, verbose=True):
    """给落在水域的出生点铺一条**登陆坡道**：从落点朝最近陆地做一条缓坡堤道。

    sea_pts: [(名字, (x, y))]；land: 陆地掩码。返回 (新高度场, 铺了几条)。
    """
    T = T if T is not None else {}
    H, W = z.shape
    grade = float(T.get('ramp_grade', 6.0))
    halfw = max(1, int(T.get('ramp_half_width', 5.0)))
    d_land = distance_transform(land)
    n_done = 0
    for nm, (cx, cy) in sea_pts:
        ix, iy = int(round(cx)), int(round(cy))
        # 在 160 格内找最近的陆地格
        best, bd = None, 1e18
        R = 160
        y0, y1 = max(0, iy - R), min(H, iy + R + 1)
        x0, x1 = max(0, ix - R), min(W, ix + R + 1)
        sub = land[y0:y1, x0:x1]
        ys, xs = np.nonzero(sub)
        if len(ys) == 0:
            if verbose:
                print('    [登陆] %s 附近 160 格内没有陆地，跳过' % nm)
            continue
        dd = (ys + y0 - iy) ** 2 + (xs + x0 - ix) ** 2
        k = int(np.argmin(dd))
        tx, ty = int(xs[k] + x0), int(ys[k] + y0)
        d = math.hypot(tx - ix, ty - iy)
        if d > 160:
            continue
        # 目标高度 = 登陆点所在层高；起点高度 = 水面（200，出水一点）
        z_hi = z[ty, tx]
        z_lo = float(T.get('sea_flat', LAND_Z - 20.0))      # 水上端用海底高度，别把落点抬到水面
        L = max(4, int(math.ceil(abs(z_hi - z_lo) / max(1.0, grade))))
        L = max(L, int(d))
        ux, uy = (tx - ix) / max(1e-6, d), (ty - iy) / max(1e-6, d)
        _skip = max(3, halfw)                               # 出生点附近留缺口，别盖住落点
        for t in range(0, L + 1):
            xx, yy = ix + ux * t, iy + uy * t
            px, py = int(round(xx)), int(round(yy))
            if not (0 <= px < W and 0 <= py < H):
                continue
            if t <= _skip:                                  # 出生点周围不动
                continue
            hh = z_lo + (z_hi - z_lo) * (t / float(L))
            for oy in range(-halfw, halfw + 1):
                for ox in range(-halfw, halfw + 1):
                    if ox * ox + oy * oy > halfw * halfw:
                        continue
                    x2, y2 = px + ox, py + oy
                    if 0 <= x2 < W and 0 <= y2 < H and z[y2, x2] < WATER_Z:
                        if math.hypot(x2 - ix, y2 - iy) <= _skip:
                            continue                        # 落点附近保持水下
                        z[y2, x2] = max(z[y2, x2], hh)      # 只在原水面处填堤道
        n_done += 1
        if verbose:
            print('    [登陆] %s (%d,%d) -> 陆地 (%d,%d) 距离 %.0f 格，堤道 %d 格'
                  % (nm, ix, iy, tx, ty, d, L))
    return z, n_done


def compute_bridges(geo, z, r_bed, bank_ramp):
    """在指定河流的指定位置架桥：桥心吸附到最近的深水格，两岸各放一个桥段。

    返回 (cells, angles)：cells 为桥段格坐标，angles 为桥轴朝向（弧度）。
    """
    river_by_name = {name: pts for name, pts, _bed in RIVERS}
    cells, angles = [], []
    for spec in BRIDGES:
        name, frac = spec[0], float(spec[1])
        pts = river_by_name.get(name)
        if not pts or len(pts) < 2:
            continue
        n = len(pts) - 1
        x = max(0.0, min(1.0, frac)) * n
        i = min(int(x), n - 1)
        u = x - i
        lon = pts[i][0] * (1 - u) + pts[i + 1][0] * u
        lat = pts[i][1] * (1 - u) + pts[i + 1][1] * u
        dlon = pts[i + 1][0] - pts[i][0]
        dlat = pts[i + 1][1] - pts[i][1]
        # 河流在“格坐标”里的方向（数据第 0 行 = 最北，y 随纬度递减，故纬度取负）
        vx, vy = dlon * geo.sx, -dlat * geo.sy
        norm = math.hypot(vx, vy) or 1.0
        vx, vy = vx / norm, vy / norm
        ax, ay = -vy, vx                       # 桥轴 = 垂直于河流
        cx, cy = geo.to_cell(lon, lat)
        # 桥心吸附到附近最深的河道格（避免落在被压平的岸/平台上）
        best, bz, R = None, WATER_Z, 34
        xi, yi = int(round(cx)), int(round(cy))
        for dy in range(-R, R + 1):
            y = yi + dy
            if not (0 <= y < GRID):
                continue
            row = z[y]
            for dx in range(-R, R + 1):
                xx = xi + dx
                if 0 <= xx < GRID and row[xx] < bz:
                    bz, best = row[xx], (xx, y)
        if best is None:
            print('  [桥梁] %s @%.2f 附近没有水面，跳过' % (name, frac))
            continue
        cx, cy = float(best[0]), float(best[1])
        ang = math.atan2(ay, ax)
        # ---- 桥段必须**压在水面上**：模型长约 490 世界单位（官方图 2 段、间距 490~590），
        #      所以按 bridge_seg_step（约 24.5 格 = 245 单位）为步长，从水面中心沿桥轴铺开，
        #      水面多宽就铺多少段 —— 保证真正跨过水域，而不是两端各搁在岸上。
        seg_step = max(6.0, float(T.get('bridge_seg_step', 24.5)))

        def _is_water(x, y):
            ix2, iy2 = int(round(x)), int(round(y))
            return (0 <= ix2 < GRID and 0 <= iy2 < GRID) and z[iy2, ix2] < WATER_Z

        half = 0.0
        for s in (1.0, -1.0):
            k = 0.0
            while k < 80.0:
                if not _is_water(cx + ax * k * s, cy + ay * k * s):
                    break
                k += 1.0
            half = max(half, k)
        # 只给"细长水域"（河道）架桥：水面太宽说明是海/大湾，不架桥
        if half * 2.0 > float(T.get('bridge_max_water', 46.0)):
            print('  [桥梁] %s @%.2f 水面宽 %.0f 格（海/大湾）-> 不架桥' % (name, frac, half * 2.0))
            continue
        # 向两岸各多铺 margin 格，保证桥段真正"盖过"河面（而不是刚好卡在水边）
        half += float(T.get('bridge_margin_cells', 7.0))
        n_seg = max(1, int(math.ceil((half * 2.0) / seg_step)))
        span_cells = half * 2.0
        for si in range(n_seg):
            t = (-half + span_cells * (si + 0.5) / n_seg) if n_seg > 1 else 0.0
            cells.append((cx + ax * t, cy + ay * t))
            angles.append(ang)
        print('  [桥梁] %s @%.2f 桥心(%d,%d) 水面宽 %.0f 格 -> 桥段 %d 个（步长 %.1f 格）'
              % (name, frac, round(cx), round(cy), span_cells, n_seg, seg_step))
    return cells, angles


def _runs_along_rows(mask):
    """每个格子所在的横向连续段长度。mask 为 False 的格子记 0。"""
    h, w = mask.shape
    out = np.zeros((h, w), dtype=np.int32)
    pad = np.zeros((h, w + 2), dtype=np.int8)
    pad[:, 1:-1] = mask.astype(np.int8)
    diff = np.diff(pad, axis=1)
    for y in range(h):
        starts = np.flatnonzero(diff[y] == 1)
        ends = np.flatnonzero(diff[y] == -1)
        for s, e in zip(starts, ends):
            out[y, s:e] = e - s
    return out


def seat_same_layer(z, base_cells, design=None, min_span=80, edge_margin=40,
                    search=240, verbose=True):
    """陆地出生点留在当前层：离可玩边界至少 edge_margin，且穿过该格的
    同行或同列同层连续段不少于 min_span。水上出生点不动。
    已经满足的落点保持不动。否则在设计格附近的同层里挑最近的合格格。
    """
    design = design or {}
    levels = layer_height.play_levels(T)
    H, W = z.shape
    yy, xx = np.mgrid[0:H, 0:W]
    d_edge = np.minimum.reduce([
        xx - BORDER, yy - BORDER,
        (GRID - 1 - BORDER) - xx, (GRID - 1 - BORDER) - yy,
    ]).astype(np.float64)
    play = ((xx >= BORDER) & (yy >= BORDER)
            & (xx <= GRID - 1 - BORDER) & (yy <= GRID - 1 - BORDER))
    cache = {}

    def runs(lv):
        if lv not in cache:
            m = (np.abs(z - lv) < 1.5) & play
            cache[lv] = (_runs_along_rows(m), _runs_along_rows(m.T).T)
        return cache[lv]

    out = dict(base_cells)
    for name, (bx, by) in list(base_cells.items()):
        ix, iy = int(round(bx)), int(round(by))
        if not (0 <= ix < W and 0 <= iy < H) or float(z[iy, ix]) < WATER_Z:
            continue
        lv = min(levels, key=lambda v: abs(float(z[iy, ix]) - v))
        if abs(float(z[iy, ix]) - lv) > 1.5:
            continue
        rx, ry = runs(lv)
        if (d_edge[iy, ix] >= edge_margin
                and max(int(rx[iy, ix]), int(ry[iy, ix])) >= min_span):
            continue
        ox, oy = design.get(name, (float(bx), float(by)))
        ox, oy = float(ox), float(oy)
        y0 = max(0, int(math.floor(oy)) - search)
        y1 = min(H, int(math.ceil(oy)) + search + 1)
        x0 = max(0, int(math.floor(ox)) - search)
        x1 = min(W, int(math.ceil(ox)) + search + 1)
        sub_e = d_edge[y0:y1, x0:x1]
        sub_rx = rx[y0:y1, x0:x1]
        sub_ry = ry[y0:y1, x0:x1]
        ok = (sub_e >= edge_margin) & ((sub_rx >= min_span) | (sub_ry >= min_span))
        ys, xs = np.nonzero(ok)
        if len(ys) == 0:
            if verbose:
                print('  [场地] %s 同层 %.0f 附近没有跨度>=%d 且离边>=%d 的格，保留 (%d,%d)'
                      % (name, lv, min_span, edge_margin, ix, iy))
            continue
        dist = (xs + x0 - ox) ** 2 + (ys + y0 - oy) ** 2
        k = int(np.argmin(dist))
        nx, ny = int(xs[k] + x0), int(ys[k] + y0)
        out[name] = (float(nx), float(ny))
        if verbose:
            print('  [场地] %s 同层换座 (%d,%d)->(%d,%d) 层 %.0f 离边 %.0f 跨度 %d/%d'
                  % (name, ix, iy, nx, ny, lv, float(d_edge[ny, nx]),
                     int(rx[ny, nx]), int(ry[ny, nx])))
    return out


def _procedural_height(geo, land, verbose=True):
    city_idx = geo.city_index_map()
    inside = land.astype(np.float64)

    # 主区各地块可单独覆盖山地强度，邻区用这一份边界自己的强度。
    amp = np.full((GRID, GRID), OUTSIDE_AMP, dtype=np.float64)
    amp_at = _amp_lookup(geo)
    for i, (name, _rings, prov, pamp) in enumerate(geo.cities):
        amp[city_idx == i] = amp_at.get(name, CITY_AMP.get(name, pamp))

    # ---- distance fields
    d_land = distance_transform(land)         # distance outside land (sea depth)
    d_sea = distance_transform(~land)         # distance inside land from coast
    # border wall: distance to playable border
    yy, xx = np.mgrid[0:GRID, 0:GRID]
    d_edge = np.minimum.reduce([xx - BORDER, yy - BORDER, (GRID - 1 - BORDER) - xx, (GRID - 1 - BORDER) - yy]).astype(np.float64)

    # ---- base height field
    n_mount = fbm((GRID, GRID), *T['noise_mount'])      # broad mountain mass
    n_detail = fbm((GRID, GRID), *T['noise_detail'])    # hills / roughness

    # coastal lowland factor: near coast -> low
    coast_factor = np.clip(d_sea / T['coast_cells'], 0.0, 1.0)
    # river valleys: low near rivers
    river_min = np.full((GRID, GRID), 1e9)
    for name, pts, bed in RIVERS:
        m = polyline_mask(geo, pts, width=2)
        d = distance_transform(m)
        river_min = np.minimum(river_min, d)
    river_factor = np.clip(river_min / T['river_cells'], 0.0, 1.0)

    # mountainous interior: karst peaks + plateau edge, low plains in the SE
    mg_base, mg_amp = T['mount_gain']
    hg_base, hg_amp = T['hills_gain']
    mount = np.clip((n_mount - 0.30) / 0.55, 0.0, 1.0) ** 1.3          # 0 = plain, 1 = mountain
    hills = (n_detail - 0.5) * 2.0                                     # -1 .. 1
    relief = (mg_base + amp * mg_amp) * mount * coast_factor * river_factor

    # ---- 真实地势（高德地形图/公开 DEM）作为蓝图：有 dem.npz 就用它驱动起伏，
    #      没有就退回程序化噪声。dem 的行列与网格一致（第 0 行 = 最南），单位米。
    # 真实地势蓝图存在**配方目录**（跨版本共用；放版本目录里一升版本就找不到了）
    _dem_path = ''
    for _cand in (os.path.join(os.path.dirname(os.path.abspath(cfg.__file__)), 'dem.npz'),
                  os.path.join(SRCDIR, 'dem.npz')):
        if os.path.isfile(_cand):
            _dem_path = _cand
            break
    if _dem_path:
        try:
            _dem = np.load(_dem_path)['dem'].astype(np.float64)
            if _dem.shape == (GRID, GRID):
                # 海拔 -> 0..1 起伏：60m 以下算平原，750m 以上拉满（配合层高 210/280/350）
                _rel = np.clip((_dem - 60.0) / 690.0, 0.0, 1.0) ** 1.15
                relief = _rel * (mg_base + mg_amp) * coast_factor * river_factor
                print('  [地势] 使用真实 DEM：高程 %.0f..%.0f m，起伏 >0.35 占 %.1f%%' % (
                    float(_dem.min()), float(_dem.max()), 100.0 * (_rel > 0.35).mean()))
        except Exception as _e:
            print('  [地势] DEM 读取失败，改用程序化噪声：%s' % str(_e)[:60])

    # ---- 蓝图二选一：**v11 手改分层图**（spec §19.34）或程序化/DEM 高度模型。
    #      v11 模式的高度场覆盖在下面 z 算出来之后执行（见"v11 分层模式"）。
    if bool(T.get('use_v11_layers')):
        _v11p = os.path.join(os.path.dirname(os.path.abspath(cfg.__file__)), 'v11_layers.npz')
        if not os.path.isfile(_v11p):
            print('  [地势] 找不到 v11_layers.npz，退回程序化高度模型')

    # ---- 只保留东西两侧的山脉，中间全部铺平
    #      形态学开运算去掉小山体 -> 再按东西两条竖带保留山系；keep_soft 让边界平滑过渡
    relief_min = float(T.get('mountain_relief', 60.0))
    mount_radius = int(T.get('mountain_radius', 14))
    w0, w1 = T.get('mount_bands', (0.34, 0.66))
    yy_m, xx_m = np.mgrid[0:GRID, 0:GRID]
    side = (xx_m < w0 * GRID) | (xx_m > w1 * GRID)          # 西侧 / 东侧
    mountain_mask = relief > relief_min
    big = opening(mountain_mask, mount_radius) & side
    # 只保留面积最大的 N 片山体（其余零散小山全部铺平）
    keep_n = int(T.get('mount_keep', 3))
    big = keep_largest_components(big, keep_n)
    if verbose:
        print('  [山体] 保留 %d 片最大山体 %d 格（西 %d / 东 %d），铺平其余 %d 格' % (
            keep_n, int(big.sum()), int((big & (xx_m < w0 * GRID)).sum()),
            int((big & (xx_m > w1 * GRID)).sum()), int((mountain_mask & ~big).sum())))
    keep_soft = smooth(big.astype(np.float64), 3, 2)
    relief = relief * keep_soft

    # ---- 主区之外：以东经 sea_cut_lon 为界，该经线以东、且在各列陆地以南的算海
    #      以西仍按陆地补全。
    yy, xx = np.mgrid[0:GRID, 0:GRID]
    coast = T.get('coast') or []
    if coast:
        # 海岸线多边形并上主区就是陆地，其余是海。
        _img = Image.new('L', (GRID, GRID), 0)
        ImageDraw.Draw(_img).polygon([geo.to_cell(lon, lat) for lon, lat in coast], fill=255)
        land_all = land | (np.array(_img) > 0)
        sea = ~land_all
        # 南边做成大片水域：主区最南端以南按列全部算海，西南角的区外陆地也变成水。
        _ly = np.where(land, yy, GRID)
        _south = _ly.min(axis=0)                      # 每列最南端的主区陆地（行号最小）
        _south = np.where(_south < GRID, _south, 0)[None, :]
        sea = sea | (yy < _south - int(T.get('sea_south_margin', 3)))
        # 左下角（省外西南）改为平原陆地：给 Player8 当出生地
        # 左下角改成与主区连片的西侧陆地：每行取主区陆地最西列，其以西都算陆地
        # （原来是矩形，会像一块方地浮在海里，陆地与水域不连贯）
        _wor = np.where(land, xx, GRID + 999).min(axis=1)
        _hasrow = (_wor < GRID)[:, None]                 # 只在该行有主区陆地时才向西延伸
        _wor = np.where(_wor < GRID, _wor, GRID)[:, None]
        # 西侧整条边也做陆地：主区最西列再往西 margin 格，所有行都算陆地
        _wor_all = float(np.where(land, xx, GRID + 999).min())
        _strip = max(0.0, _wor_all + float(T.get('west_land_margin', 70.0)))
        _cx2 = float(T.get('sw_corner_x', 0.45)) * GRID
        _cy2 = float(T.get('sw_corner_y', 0.36)) * GRID
        # 西侧整条边 + 与主区连片的西侧 + 西南角到地图边界
        sw_plain = ((xx < _strip) | ((xx < _wor) & _hasrow)
                    | ((xx < _cx2) & (yy < _cy2)))
        sea = sea & (~sw_plain)
    else:
        land_y = np.where(land, yy, -1)
        south_edge = land_y.max(axis=0)                      # 每列最南的陆地格
        no_land_col = south_edge < 0
        south_edge = np.where(no_land_col, GRID - 1, south_edge)[None, :]
        sea_cut_lon = float(T.get('sea_cut_lon', LON0))
        sea_cut = geo.to_cell(sea_cut_lon, LAT0)[0]
        border = T.get('sea_border') or []
        if border:
            bl = np.array(sorted(((p[1], p[0]) for p in border)))       # (lat, lon) 升序
            lat_row = LAT1 - (np.arange(GRID) - geo.oy) / geo.sy        # 每行对应的纬度（行号大=南）
            lon_edge = np.interp(lat_row, bl[:, 0], bl[:, 1])           # 每行的边界经度（端点外取端点值）
            x_edge = geo.ox + (lon_edge - LON0) * geo.sx
            east_of_border = xx > x_edge[:, None]
        else:
            east_of_border = xx >= sea_cut
        sea = (~land) & (yy > south_edge) & east_of_border
        # 左下角改成与主区连片的西侧陆地：每行取主区陆地最西列，其以西都算陆地
        # （原来是矩形，会像一块方地浮在海里，陆地与水域不连贯）
        _wor = np.where(land, xx, GRID + 999).min(axis=1)
        _wor_all = float(_wor.min())
        _strip = max(0.0, _wor_all + float(T.get('west_land_margin', 70.0)))
        _wb = float(T.get('west_band', 0.46)) * GRID                  # 西侧整条：不要任何水域
        _sb = T.get('sw_band', (0.46, 0.40))
        _sbx, _sby = float(_sb[0]) * GRID, float(_sb[1]) * GRID        # 西南角：不要任何水域
        _hasrow = (_wor < GRID)[:, None]
        sw_plain = ((xx < _wb) | (xx < _strip) | ((xx < _wor) & _hasrow)
                    | ((xx < _sbx) & (yy < _sby)))
        sea = sea & (~sw_plain)
    land_or_sea = (~land) & (~sea)                        # 主区之外但仍是陆地
    if verbose:
        print('  [海陆] 海域 %d 格 (%.1f%%)，省外陆地 %d 格' % (
            int(sea.sum()), 100.0 * sea.mean(), int(land_or_sea.sum())))

    # ---- 基础高度场：大部分为平地/缓坡，山地为渐变坡
    mg_base, mg_amp = T['mount_gain']
    hg_base, hg_amp = T['hills_gain']
    mount = np.clip((n_mount - 0.30) / 0.55, 0.0, 1.0) ** 1.3          # 0 = plain, 1 = mountain
    hills = (n_detail - 0.5) * 2.0                                     # -1 .. 1
    # 注意：必须用上面经"山体掩码"过滤后的 relief，不能重新内联原表达式，
    # 否则 keep_largest_components / 开运算全成死代码，全图都会铺满噪声起伏（小山遍地）。
    z = LAND_Z + 4.0 + relief + hills * (hg_base + amp * hg_amp)
    return z, {
        'd_land': d_land,
        'sea': sea,
        'sw_plain': sw_plain,
        'land_or_sea': land_or_sea,
        'amp': amp,
        'mount': mount,
        'n_detail': n_detail,
        'hills': hills,
    }


def build_terrain(verbose=True):
    geo = Geo()
    land = geo.make_mask()
    _v11_path = os.path.join(os.path.dirname(os.path.abspath(cfg.__file__)), "v11_layers.npz")
    _v11_direct = (bool(T.get("use_v11_layers")) and bool(T.get("v11_plain", True))
                   and os.path.isfile(_v11_path))
    if _v11_direct:
        z = np.zeros((GRID, GRID), dtype=np.float64)
        _proc = None
        if verbose:
            print('  [地势] v11 分层图已在，跳过程序化高度')
    else:
        z, _proc = _procedural_height(geo, land, verbose)

    # ---- v11 分层模式：整幅高度场直接取 v11 图层（跳过程序化起伏/海深/省外补全）
    if bool(T.get('use_v11_layers')):
        _v11p = os.path.join(os.path.dirname(os.path.abspath(cfg.__file__)), 'v11_layers.npz')
        if os.path.isfile(_v11p):
            _d = np.load(_v11p)
            _z = _d['base_z'].astype(np.float64) * float(_d['z_scale'])
            # 翻转已在导出阶段做掉（v11_layers.npz 里的 flip_v=1），这里不再翻，
            # 保证"数据坐标 = v11 图的坐标"，圈/出生点按图坐标直接落。
            if _z.shape == z.shape:
                z = _z.copy()
                _sea_note = float(T.get('sea_flat', LAND_Z - 20.0))
                print('  [地势] 使用 v11 手改分层图（海 %.0f / 平 212 / 一层 280 / 二层 350 / 三层 420 / 高山 480）'
                      % _sea_note)
                if 'layer' in _d.files:
                    _lay = _d['layer']
                    print('        ' + '  '.join('%d=%.1f%%' % (
                        _lv, 100.0 * (_lay == _lv).mean()) for _lv in (0, 210, 280, 350, 420, 480)))
                if bool(T.get('v11_plain', True)):
                    # ---- v11 蓝图模式：这张图就是最终地形。
                    #      只做「浅平滑 + 海域拉平」，不再叠加 DEM 起伏、边界海陆、河道、平台、
                    #      层规则等改写（那些会把 v11 的分层改掉）。
                    #      出生点/资源位置照常在**这张图**上解算（因为后面直接返回，要在这里补）。
                    #      v11_baked：高度场已经是定稿（斜坡也在里面）。没开的图仍重切层、重挖坡。
                    _zr_orig = z.copy()                              # 原始蓝图（判定"设计上是否在水里"）
                    if bool(T.get('v11_baked')):
                        _rsp = json.loads(str(_d['ramp_json'])) if 'ramp_json' in _d.files else []
                        if verbose:
                            print('  [地势] 使用定稿高度（含斜坡 %d 条），不再重切层' % len(_rsp))
                    else:
                        for _ in range(max(0, int(T.get('v11_smooth', 1)))):
                            z = smooth(z, 1, 1)
                        z = np.where(z < WATER_Z, float(T.get('sea_flat', LAND_Z - 20.0)), z)
                    _land_v = z >= WATER_Z
                    if not bool(T.get('v11_baked')):
                        # ---- 严格分层：层与层、水与陆默认不可通行。斜坡自动开在交界上，
                        #      朝向地图中心或资源，边长 20~40。不再使用手工矩形。
                        z = quantize_strict_layers(z, T=T, verbose=verbose)
                        _aim = [(GRID / 2.0, GRID / 2.0)]
                        for _cell in list(T.get('base_cells') or []):
                            _aim.append((float(_cell[0]), float(_cell[1])))
                        _bc_design = {}
                        for _i, _cell in enumerate(_aim[1:]):
                            if _i < len(BASES):
                                _bc_design[BASES[_i][2]] = _cell
                        if _bc_design:
                            _oslots, _ = ore_slots(geo, base_cells=_bc_design)
                            for _p in list(_oslots)[::3]:
                                _aim.append((float(_p[0]), float(_p[1])))
                        _bases_aim = []
                        for _cell in list(T.get('base_cells') or []):
                            _bases_aim.append((float(_cell[0]), float(_cell[1])))
                        _rsz = float(T.get('ramp_size', 28.0))
                        _rsp = plan_auto_ramps(z, _aim, bases=_bases_aim,
                                               width=_rsz, length=_rsz, verbose=verbose)
                        _coasts = list(T.get('landing_coasts') or ())
                        if _coasts:
                            _kept = []
                            _ndrop = 0
                            for _s in _rsp:
                                _layers = _s.get('layers') or (999.0,)
                                _sea_ramp = min(float(v) for v in _layers) < WATER_Z
                                _x, _y = float(_s['x']), float(_s['y'])
                                _in = _sea_ramp and any(
                                    math.hypot(_x - float(c[0]), _y - float(c[1])) <= float(c[2])
                                    for c in _coasts)
                                if _in:
                                    _ndrop += 1
                                else:
                                    _kept.append(_s)
                            if _ndrop and verbose:
                                print('  [登陆] 圈内海岸整段可上，去掉圈内水陆斜坡 %d 条' % _ndrop)
                            _rsp = _kept
                        z, _nramp = carve_marked_ramps(z, _rsp, T=T, verbose=verbose)
                    # 斜坡和登陆岸在出生点之前。登陆圈抬高贴岸水面，出生点再落到这张高度上。
                    if T.get('landing_coasts'):
                        z = grade_landing_shores(z, T.get('landing_coasts'), _rsp, verbose=verbose)
                    raw = np.clip(np.round(z / Z_SCALE), 0, 65535).astype(np.uint16)
                    _land_v = z >= WATER_Z
                    if layer_height.enabled(T):
                        _wlo, _whi = layer_height.WATER_RANGE
                        _plo, _phi = layer_height.PEAK_RANGE
                        _bits = ['水域 %.0f–%.0f=%.1f%%' % (
                            _wlo, _whi, 100.0 * (z < WATER_Z).mean())]
                        _bits += ['%d=%.1f%%' % (int(_lv), 100.0 * (np.abs(z - _lv) < 1.5).mean())
                                  for _lv in layer_height.LEVELS]
                        _bits.append('五层 %.0f–%.0f=%.1f%%' % (
                            _plo, _phi, 100.0 * ((z >= _plo) & (z <= _phi + 1.0)).mean()))
                        print('  [v11] 直接采用该图作为最终地形（陆地 %.1f%%）：%s'
                              % (100.0 * _land_v.mean(), '  '.join(_bits)))
                    else:
                        print('  [v11] 直接采用该图作为最终地形（陆地 %.1f%%）：%s'
                              % (100.0 * _land_v.mean(),
                                 '  '.join('%d=%.1f%%' % (_lv, 100.0 * (np.abs(z - _lv) < 1.5).mean())
                                           for _lv in (float(T.get('sea_flat', LAND_Z - 20.0)),
                                                       212, 280, 350, 420, 480))))
                    # ---- 在**定稿地形**上解算出生点与资源点（不再铺平任何平台）
                    #      可落格：**一/二/三/四层（212/280/350/420）∪ 水域**；
                    #      只排除高山(480+) 与层间陡坎。要求落点周围有足够净空，
                    #      相当于天然自带平整场地，不必改地形。
                    _lv_ok = np.zeros(z.shape, dtype=bool)
                    for _lv in layer_height.play_levels(T):
                        _lv_ok |= (np.abs(z - _lv) < 1.5)
                    _lv_ok |= (z < WATER_Z)                          # 水域也可落
                    _clear_want = float(T.get('base_clear_want', 8.0))
                    _d_room = distance_transform(~_lv_ok)            # 在该集合内 -> 离边界多远
                    _ok = _lv_ok & (_d_room >= _clear_want)
                    _bs = _lv_ok
                    _bc = resolve_base_cells(geo, _lv_ok, allow_water=True)
                    for _bn, (_bx, _by) in list(_bc.items()):
                        _iy, _ix = int(round(_by)), int(round(_bx))
                        if (0 <= _ix < GRID and 0 <= _iy < GRID and _ok[_iy, _ix]):
                            continue
                        _c = snap_to_mask(_bx, _by, _ok, 200)
                        if _c is not None:
                            print('  [出生点] %s 吸附到「可落层/水域且周围 %.0f 格净空」(%d,%d)->(%d,%d) z=%.0f 净空 %.0f'
                                  % (_bn, _clear_want, round(_bx), round(_by), _c[0], _c[1],
                                     z[_c[1], _c[0]], _d_room[_c[1], _c[0]]))
                            _bc[_bn] = (float(_c[0]), float(_c[1]))
                        else:
                            _c = snap_to_mask(_bx, _by, _lv_ok, 260)
                            if _c is not None:
                                print('  [出生点] %s 净空不足，退到该集合任意处 (%d,%d)->(%d,%d) z=%.0f'
                                      % (_bn, round(_bx), round(_by), _c[0], _c[1],
                                         z[_c[1], _c[0]]))
                                _bc[_bn] = (float(_c[0]), float(_c[1]))
                            else:
                                print('  [出生点] %s 附近没有可落格（保留原坐标）' % _bn)
                    # ---- 落点收尾：水上出生点保持在水域（用户要求：水上基地 + 允许水里放矿，
                    #      所以不强行上陆）；陆上落点若不在可落层则就近吸附。
                    _land_ok = _ok & land & (z >= WATER_Z)
                    for _bn, (_bx, _by) in list(_bc.items()):
                        _iy, _ix = int(round(_by)), int(round(_bx))
                        if not (0 <= _ix < GRID and 0 <= _iy < GRID):
                            continue
                        if z[_iy, _ix] < WATER_Z:
                            continue                     # 水上基地：保留
                        _c = snap_to_mask(_bx, _by, _land_ok, 60)
                        if _c is not None and (_c[0], _c[1]) != (_ix, _iy):
                            print('  [出生点] %s 就近吸附到平整可建处 (%d,%d)->(%d,%d) z=%.0f'
                                  % (_bn, _ix, _iy, _c[0], _c[1], z[_c[1], _c[0]]))
                            _bc[_bn] = (float(_c[0]), float(_c[1]))
                    _design = {}
                    for _i, _cell in enumerate(list(T.get('base_cells') or [])):
                        if _i < len(BASES):
                            _design[BASES[_i][2]] = (float(_cell[0]), float(_cell[1]))
                    _bc = seat_same_layer(z, _bc, _design, verbose=verbose)
                    # 水陆交界只在自动斜坡处可通行，不再从水上出生点另铺一条堤道。
                    _sea_pt = [(nm, c) for nm, c in _bc.items()
                               if z[int(c[1]), int(c[0])] < WATER_Z]
                    if _sea_pt and verbose:
                        print('  [登陆] %d 个水上出生点走自动水陆斜坡，岸线其余位置不可通行'
                              % len(_sea_pt))
                    # ---- 最小建造区：出生点所在**层内**补一小块平整面（默认半径 24 格），
                    #      保证能放下建造场 + 几个建筑；不跨层、不改动其它层的地形。
                    _br = float(T.get('base_min_flat_radius', 24.0))
                    if _br > 0:
                        for _bn, (_bx, _by) in _bc.items():
                            _ix, _iy = int(round(_bx)), int(round(_by))
                            _zv = z[_iy, _ix]
                            if _zv < WATER_Z:
                                # 水上出生点：铺**湖底平台**（保持在水面以下，不填成陆地）
                                _lv = float(T.get('sea_flat', LAND_Z - 20.0))
                                _keep = np.zeros(z.shape, dtype=bool)
                                _keep[_iy, _ix] = True
                            else:
                                _lv = min(layer_height.play_levels(T),
                                          key=lambda v: abs(v - _zv))
                                if abs(_zv - _lv) > 40.0:
                                    continue                    # 不是标准层，跳过
                            _R = int(math.ceil(_br)) + 2
                            _y0, _y1 = max(0, _iy - _R), min(GRID, _iy + _R + 1)
                            _x0, _x1 = max(0, _ix - _R), min(GRID, _ix + _R + 1)
                            _yy, _xx2 = np.mgrid[_y0:_y1, _x0:_x1]
                            _dd = np.sqrt((_yy - _iy) ** 2 + (_xx2 - _ix) ** 2)
                            _w = np.clip((_br - _dd) / 4.0, 0.0, 1.0)
                            _sub = z[_y0:_y1, _x0:_x1]
                            if _zv < WATER_Z:
                                _w = _w * (_sub < WATER_Z)
                            else:
                                _w = _w * (np.abs(_sub - _lv) < 1.5)
                            z[_y0:_y1, _x0:_x1] = _sub * (1.0 - _w) + _lv * _w
                        raw = np.clip(np.round(z / Z_SCALE), 0, 65535).astype(np.uint16)
                        print('  [场地] 8 个出生点各补一块半径 %.0f 格的最小建造面（陆上取层高、水上铺湖底）'
                              % _br)
                    # ---- 收尾纠正：配置坐标在**原始蓝图**里就是水域的出生点，
                    #      最终地形里必须也低于水面（前面几步的填平/堤道可能把它抬成陆地）。
                    _zfinal = raw.astype(np.float64) * Z_SCALE
                    _d_shore2 = distance_transform(_zfinal >= WATER_Z)
                    _wc = float(T.get('base_water_clear', 14.0))
                    _wet_ok2 = (_zfinal < WATER_Z - 1.0) & (_d_shore2 >= _wc)
                    for _bi, (_clon, _clat, _bn, _bt2, _bk) in enumerate(BASES):
                        _ex = list(T.get('base_cells') or [])
                        if _bi >= len(_ex):
                            continue
                        _cx0, _cy0 = float(_ex[_bi][0]), float(_ex[_bi][1])
                        _ix0, _iy0 = int(round(_cx0)), int(round(_cy0))
                        if not (0 <= _ix0 < GRID and 0 <= _iy0 < GRID):
                            continue
                        if _zr_orig is not None and _zr_orig[_iy0, _ix0] >= WATER_Z:
                            continue                     # 原始蓝图里就是陆地 -> 不用管
                        _cur = _bc.get(_bn)
                        if _cur is not None and _wet_ok2[int(round(_cur[1])), int(round(_cur[0]))]:
                            continue
                        _c = snap_to_mask(_cx0, _cy0, _wet_ok2, 90)
                        if _c is not None:
                            print('  [场地] %s 纠正为水下落点 (%s) -> (%d,%d) 离岸 %.0f 格'
                                  % (_bn, _cur, _c[0], _c[1], _d_shore2[_c[1], _c[0]]))
                            _bc[_bn] = (float(_c[0]), float(_c[1]))
                    _sea_pt3 = [(nm, c) for nm, c in _bc.items()
                                if (raw.astype(np.float64) * Z_SCALE)[int(c[1]), int(c[0])] < WATER_Z]
                    if _sea_pt3 and verbose:
                        if T.get('landing_coasts'):
                            print('  [登陆] 水上出生点 %d 个；圈内海岸整段可上，其余岸线只留斜坡'
                                  % len(_sea_pt3))
                        else:
                            print('  [登陆] 水上出生点 %d 个，岸线只留自动斜坡' % len(_sea_pt3))
                    # ---- 落点全部定稿后，再解算资源（矿/油井/要点），保证"每基地 45 格内 >=2 矿"
                    #      是按**最终**出生点坐标算的。
                    _slots_v = resolve_slots(geo, raw.astype(np.float64) * Z_SCALE, _bs, _bc,
                                             verbose=verbose, ore_open_rank=True, ramps=_rsp)
                    # ---- 绝对保底：按最终落点再核一遍"每基地 45 格内 >=2 矿"，不足就在附近补。
                    _zF = raw.astype(np.float64) * Z_SCALE
                    _ORE_F, _nadd = ensure_base_min_ore(
                        _zF, _bc, list(_slots_v.get('ore') or []),
                        min_ore=int(T.get('base_min_ore', 2)),
                        rng=float(T.get('base_ore_range', 66.0)),
                        spacing=float(getattr(cfg, 'ORE_MIN_SPACING', 46.0)),
                        T=T, verbose=verbose, ramps=_rsp,
                        avoid=list(_slots_v.get('derricks') or []))
                    _slots_v['ore'] = _ORE_F
                    # ---- 最终清洗：任何贴基地的矿都挪走（避免建造场被卡住）
                    _ORE_G, _nmv = enforce_ore_base_clearance(
                        _zF, _bc, list(_slots_v.get('ore') or []),
                        min_r=float(T.get('ore_near_min_r', 44.0)),
                        spacing=float(getattr(cfg, 'ORE_MIN_SPACING', 46.0)),
                        T=T, verbose=verbose, ramps=_rsp,
                        avoid=list(_slots_v.get('derricks') or []))
                    _slots_v['ore'] = _ORE_G
                    # ---- 近矿超员裁剪：保底补矿 + 清洗挪位可能叠加出 3 个（规则：恰好 2 个）
                    _ORE_H = trim_near_ore_quota(_zF, _bc, list(_slots_v.get('ore') or []),
                                                 T=T, keep=2, verbose=verbose, ramps=_rsp,
                                                 avoid=list(_slots_v.get('derricks') or []))
                    _slots_v['ore'] = _ORE_H
                    # ---- 矿口朝向定稿：前方必须开阔（不撞 层界/岸线/斜坡/不可通行），
                    #      位置定稿后统一重算角度（此前的 ore_angles 已随挪位过时）
                    _ORE_I, _ANG_I = enforce_ore_mouth_open(
                        _zF, _bc, list(_slots_v.get('ore') or []), T=T, verbose=verbose,
                        ramps=_rsp, avoid=list(_slots_v.get('derricks') or []))
                    _ORE_I, _ANG_I = apply_ore_place_at(
                        _zF, _bc, _ORE_I, _ANG_I, T=T, verbose=verbose)
                    _ORE_I, _ANG_I = trim_outer_ore_quota(
                        _ORE_I, _ANG_I, _bc, T=T, verbose=verbose)
                    _ORE_I, _ANG_I = apply_ore_face_at(_ORE_I, _ANG_I, T, verbose=verbose)
                    if T.get('ore_bands'):
                        _avoid = []
                        for _key in ('derricks', 'houses', 'posts', 'garage',
                                     'airports', 'hospitals', 'academies', 'shipyards'):
                            _avoid.extend(_slots_v.get(_key) or [])
                        _ORE_I, _ANG_I, _kind, _own = apply_ore_bands(
                            _zF, _bc, _ORE_I, _ANG_I, T, _rsp, _avoid, verbose=verbose)
                        _slots_v['ore_kind'] = _kind
                        _slots_v['ore_owner'] = _own
                    _slots_v['derricks'] = apply_derrick_place_at(
                        list(_slots_v.get('derricks') or []), T, verbose=verbose)
                    _slots_v['ore'] = _ORE_I
                    _slots_v['ore_angles'] = _ANG_I
                    _slots_v['ramps'] = _rsp
                    return (geo, raw, _land_v, np.zeros((GRID, GRID), dtype=np.float64),
                            np.zeros((GRID, GRID), dtype=np.float64), _bc, _slots_v, [], [])

    # 海面（主区之外、该列陆地以南）
    if _proc is None:
        raise SystemExit('v11 蓝图没有替换高度场')
    d_land = _proc['d_land']
    sea = _proc['sea']
    sw_plain = _proc['sw_plain']
    land_or_sea = _proc['land_or_sea']
    amp = _proc['amp']
    mount = _proc['mount']
    n_detail = _proc['n_detail']
    hills = _proc['hills']
    sea_shallow, sea_depth_z, sea_span, sea_detail = T['sea']
    sea_depth = np.clip(d_land / sea_span, 0.0, 1.0)
    # 新规则：所有海域拉平（水面 200，海底恒定 -> 一整片平坦水域）
    z_sea = np.full(z.shape, float(T.get('sea_flat', LAND_Z - 20.0)))
    z = np.where(sea, z_sea, z)
    # 主区之外的陆地：平滑的山地梯度
    z = np.where(land_or_sea & (~sw_plain), LAND_Z + 60.0 + amp * 150.0 * mount + n_detail * 40.0, z)
    # 西南境外：平原（不再是 270+ 的山地、也不是水）
    z = np.where(sw_plain & (~land), LAND_Z + 4.0 + hills * (hg_base + amp * hg_amp), z)
    # 所有陆地不得低于水面（海岸线内）
    z = np.maximum(z, np.where(land, WATER_Z + 4.0, z))

    # ---- smooth first, then carve rivers so the trenches survive
    z = smooth(z, T['smooth'][0], T['smooth'][1])

    # ---- carve river valleys（加宽河道 + 平缓可建造的河岸 + 缓坡接回地形）
    r_width, r_span, r_taper, r_relief, r_bed = T['river']
    bank_ramp, bank_flat, bank_out = T.get('river_bank', (8.0, 10.0, 14.0))
    bank_level = WATER_Z + float(T.get('bank_above', 22.0))
    # 先把所有河道合并成一个掩码：岸坡/平岸绝不能盖住任何一条河的水面，
    # 否则支流接干流的地方会被"岸"填平而断流（水系就连不起来了）。
    rd, rbed = {}, {}
    channel = np.zeros((GRID, GRID), dtype=bool)
    wet_channel = np.zeros((GRID, GRID), dtype=bool)
    for name, pts, bed in RIVERS:
        d = distance_transform(polyline_mask(geo, pts, width=r_width))
        rd[name] = d
        rbed[name] = bed
        channel |= d < r_bed
        if bed < WATER_Z:
            wet_channel |= d < r_bed
    for name in rd:                                                              # 河谷
        d = rd[name]
        t = np.clip(d / r_span, 0.0, 1.0)
        target = rbed[name] + (1.0 - np.exp(-r_taper * t)) / (1.0 - math.exp(-r_taper)) * r_relief
        z = np.where(d < r_span + 2.0, np.minimum(z, target), z)
    for name in rd:                                                              # 岸坡 / 平岸 / 缓坡（避开河道与海面）
        d = rd[name]
        w = np.clip((d - r_bed) / bank_ramp, 0.0, 1.0)
        m1 = (d >= r_bed) & (d < r_bed + bank_ramp) & ~channel & ~sea
        z = np.where(m1, (WATER_Z + 1.0) * (1 - w) + bank_level * w, z)
        m2 = (d >= r_bed + bank_ramp) & (d < r_bed + bank_ramp + bank_flat) & ~channel & ~sea
        z = np.where(m2, bank_level, z)
        w2 = np.clip((d - (r_bed + bank_ramp + bank_flat)) / bank_out, 0.0, 1.0)
        m3 = (d >= r_bed + bank_ramp + bank_flat) & (d < r_bed + bank_ramp + bank_flat + bank_out) & ~channel & ~sea
        z = np.where(m3, bank_level * (1 - w2) + z * w2, z)
    # gentle clean-up that keeps the river beds
    z = smooth(z, 2, 2)
    for name in rd:                                                              # 最后统一重铺所有河道
        d = rd[name]
        floor = rbed[name] + (d / max(r_bed, 1.0)) * 8.0
        if rbed[name] < WATER_Z:                       # 成水的河：压到水面以下
            z = np.where(d < r_bed, np.minimum(z, floor), z)
        else:                                          # 干谷：只下切到河床，绝不低于水面，也不抬高别人的水面
            m = (d < r_bed) & ~wet_channel
            if rbed[name] >= WATER_Z:
                z = np.where(m, np.minimum(np.maximum(z, WATER_Z + 2.0), floor), z)
    for name in rd:
        d = rd[name]
        m2 = (d >= r_bed + bank_ramp) & (d < r_bed + bank_ramp + bank_flat) & ~channel & ~sea
        z = np.where(m2, np.minimum(z, bank_level + 1.0), z)

    # ---- 高度分层：陆地高度一律量化到设计层（平原 210 / 一层 280 / 二层 320），最高 320。
    #      层间做缓坡过渡，保证每一层都能通过部队（坡度 < SLOPE_IMPASSABLE）。
    levels = list(T.get('levels') or [])
    if levels:
        lv = np.asarray(sorted(levels), dtype=np.float64)
        landm = z >= WATER_Z
        # 先粗平滑再量化：让同一层的区域更连片（否则会碎成小斑块，缓坡占比过高）
        zs = np.where(landm, z, lv[0])
        for _ in range(max(0, int(T.get('level_presmooth', 3)))):
            zs = smooth(zs, 1, 1)
        zq = lv[np.argmin(np.abs(zs[..., None] - lv[None, None, :]), axis=-1)]
        # ---- 层规则：边界平滑 + 删孤立块 + 连片（规则见 spec §19.10，参数由 v11 逆向拟合）
        if T.get('level_rule'):
            if T.get('level_rule_verbose', True):
                print('  [层规则] 平滑 r%s x%s / 删块 %s / 连片 %s / 再删 %s / 收尾 r%s'
                      % (T.get('level_rule_smooth_r'), T.get('level_rule_smooth_passes'),
                         T.get('level_rule_min_area'), T.get('level_rule_close_r'),
                         T.get('level_rule_prune2_area'), T.get('level_rule_final_maj_r')))
            zq, _ruleinfo = layer_build(zq, lv,
                                        smooth_r=int(T.get('level_rule_smooth_r', 0) or 0),
                                        smooth_passes=int(T.get('level_rule_smooth_passes', 0) or 0),
                                        min_area=T.get('level_rule_min_area'),
                                        close_r=T.get('level_rule_close_r'),
                                        prune2_area=T.get('level_rule_prune2_area'),
                                        close2_r=T.get('level_rule_close2_r'),
                                        final_maj_r=int(T.get('level_rule_final_maj_r', 0) or 0),
                                        sea_close=int(T.get('level_rule_sea_close', 0) or 0),
                                        sea_min=int(T.get('level_rule_sea_min', 0) or 0),
                                        verbose=bool(T.get('level_rule_verbose', True)))
            zq = np.where(landm, zq, lv[0])
        # ---- 二层连片：把二层高地之间散落的一层地面整片并成二层（规则见 spec §19.10）
        _jr = int(T.get('level_join_radius', 0) or 0)
        if _jr > 0:
            if T.get('level_join_verbose', True):
                print('  [连片] 半径 %d 格，最小地块 %d 格，口径 %s'
                      % (_jr, int(T.get('level_join_min_block', 400)),
                         T.get('level_join_mode', 'expand')))
            zq, _joinmask = level_join(zq, lv, _jr,
                                       min_block=int(T.get('level_join_min_block', 400)),
                                       low_levels=T.get('level_join_low') or (float(lv[0]),),
                                       land=landm,
                                       rounds=int(T.get('level_join_rounds', 8)),
                                       mode=str(T.get('level_join_mode', 'expand')),
                                       verbose=bool(T.get('level_join_verbose', True)))
        else:
            _joinmask = None
        # 层间缓坡：保证每层都能通过部队（坡度 < SLOPE_IMPASSABLE）
        base = zq
        for _ in range(max(1, int(T.get('level_ramp', 2)))):
            base = smooth(base, 1, 1)
        z = np.where(landm, np.clip(base, lv[0], lv[-1]), z)

        # ---- 平台陡坎 + 少量斜坡口：只在"口子"附近做缓坡，其余边界保持陡坎（不可通行）
        _opens = int(T.get('ramp_openings', 2))
        _halfw = float(T.get('ramp_half_width', 14.0))
        _rlen = float(T.get('ramp_len', 24.0))
        _rampmask = np.zeros_like(landm, dtype=bool)
        if _opens > 0:
            _yo, _xo = np.mgrid[0:GRID, 0:GRID]
            _lvl = np.zeros((GRID, GRID), dtype=np.int8)
            for _i in range(1, len(lv)):
                _lvl = _lvl + (z >= lv[_i] - 1.0).astype(np.int8)
            _bnd = np.zeros_like(landm, dtype=bool)
            _bnd[:, 1:] |= (_lvl[:, 1:] != _lvl[:, :-1])
            _bnd[1:, :] |= (_lvl[1:, :] != _lvl[:-1, :])
            _bnd &= landm & (z <= lv[-1] + 1.0)
            _by, _bx = np.nonzero(_bnd)
            if len(_by):
                _flat = smooth((z <= lv[0] + 1.0).astype(np.float64), 8, 2)   # 外侧越平越适合开口
                _ord = np.argsort(-_flat[_by, _bx])
                _picked = []
                for _k in _ord:
                    _py, _px = int(_by[_k]), int(_bx[_k])
                    if all((_py - _q[0]) ** 2 + (_px - _q[1]) ** 2 > (6 * _halfw) ** 2 for _q in _picked):
                        _picked.append((_py, _px))
                    if len(_picked) >= _opens:
                        break
                for _py, _px in _picked:
                    _rampmask |= (((_yo - _py) ** 2 + (_xo - _px) ** 2) < (_halfw * 2 + _rlen) ** 2)
                print('  [斜坡口] 开口 %d 处（每处半径 %.0f 格）' % (len(_picked), _halfw))

        smax = float(T.get('slope_max', 10.0)) / 1.5
        for _ in range(max(0, int(T.get('slope_relax', 20)))):
            zs = z
            for _ax, _sh in ((0, 1), (0, -1), (1, 1), (1, -1)):
                zs = np.minimum(zs, np.roll(zs, _sh, axis=_ax) + smax)
            z = np.where(landm & _rampmask, np.clip(zs, lv[0], lv[-1]), z)
        # （全域坡度限制器已按新规则移除：平台边界保持陡坎，只有斜坡口是缓坡）
        # ---- 清除零散小山：只保留面积最大的 N 片山体。
        #      判据用"一层(280)及以上"作山体核心（不能用 z>平原+20，否则缓坡会把
        #      全图连成一大片，等于没清）；山体的缓坡 halo 算山体的一部分，
        #      其余隆起连同缓坡一律压回平原层。
        lvl1 = float(lv[1]) if len(lv) > 1 else float(lv[0])
        keep_n = int(T.get('mount_keep', 3))
        core = (z >= lvl1 - 5.0) & landm
        keep = keep_largest_components(core, keep_n)
        keepz = smooth(keep.astype(np.float64), int(T.get('mount_halo', 10)), 3) > 0.02
        drop = landm & (~keepz) & (z > float(lv[0]) + 2.0)
        if drop.any():
            wd = smooth(drop.astype(np.float64), 6, 2)
            z = np.where(landm, z * (1.0 - wd) + float(lv[0]) * wd, z)
            if verbose:
                print('  [小山] 压平零散隆起 %d 格（保留 %d 片大山，核心 %d 格，含缓坡 %d 格）' % (
                    int(drop.sum()), keep_n, int(keep.sum()), int(keepz.sum())))

        if verbose:
            exact = np.min(np.abs(z[..., None] - lv[None, None, :]), axis=-1) < 1.0
            gy2, gx2 = np.gradient(z)
            sl = np.sqrt(gx2 ** 2 + gy2 ** 2)
            print('  [高度层] %s；平台层占陆地 %.0f%%，层间缓坡 %.0f%%，坡度>%.0f 占 %.1f%%' % (
                '/'.join('%.0f' % v for v in lv), 100.0 * (exact & landm).sum() / max(1, landm.sum()),
                100.0 * ((~exact) & landm).sum() / max(1, landm.sum()),
                SLOPE_IMPASSABLE, 100.0 * (sl[landm] > SLOPE_IMPASSABLE).mean()))

    # ---- 城市平台：城市建在平地上，顺带让"城市中心/周边的矿脉"有平整落点
    cp = T.get('city_pad')
    if cp and T.get('levels'):
        _cr, _cb = float(cp[0]), float(cp[1])
        _cpads = [(cx, cy, _cr, _cb, 320.0, False)
                  for _n, cx, cy in city_centers(geo, inside_only=True)]
        z = flatten_pads(z, _cpads, T['levels'][0], xx, yy, verbose=False)
        if verbose:
            print('  [城市平台] %d 个（半径 %.0f / 过渡 %.0f 格）' % (len(_cpads), _cr, _cb))

    # ---- outer wall only at the outermost cells, and only where the background is land
    wall_cells, wall_h = T['wall']
    wall = (d_edge < wall_cells) & (z > WATER_Z)
    z = np.where(wall, np.maximum(z, LAND_Z + wall_h), z)

    # ---- basements / flat base plateaus and resource pads
    base_cells = resolve_base_cells(geo, land)

    # ---- 资源/要点位置先解算，再和出生点一起按“平台簇”统一高度铺平
    #      同一个簇（互相影响）用同一高度 -> 基地与周边矿场/油井连成一整块平地
    res_radius, res_blend = T.get('res_pad', (13.0, 11.0))
    pad_radius, pad_blend, pad_max = T['pad']
    slots = resolve_slots(geo, z, land, base_cells, verbose=verbose)
    # 只取坐标类槽位（ore/derricks/posts/garage/airports/hospitals/academies）；
    # ore_angles 是角度数组，不能当坐标用
    zones = [c for k, group in slots.items() if k != 'ore_angles' for c in group]

    pads = [(cx, cy, pad_radius, pad_blend, pad_max, True) for cx, cy in base_cells.values()]
    pads += [(cx, cy, res_radius, res_blend, pad_max, False) for cx, cy in zones]
    z = flatten_pads(z, pads, LAND_Z + 2.0, xx, yy, verbose=verbose)

    # keep pads above water (only the pads, never the river beds)
    pad_mask = np.zeros_like(z, dtype=bool)
    for name, (cx, cy) in base_cells.items():
        pad_mask |= (np.sqrt((xx - cx) ** 2 + (yy - cy) ** 2) < pad_radius + 2.0)
    for cx, cy in zones:
        pad_mask |= (np.sqrt((xx - cx) ** 2 + (yy - cy) ** 2) < res_radius + 2.0)
    z = np.where(pad_mask & land, np.maximum(z, LAND_Z - 2.0), z)

    # 配方指定的出生点旁，把高山做成更高一层的可通行平台。
    _p4 = T.get('p4_plateau') or []
    if _p4:
        _pr, _pb, _pl = (float(_p4[0]), float(_p4[1]), float(_p4[2]))
        _want = T.get('plateau_base') or ''
        for _bn, (_bx, _by) in base_cells.items():
            if _bn != _want:
                continue
            # 平台中心：从出生点往 DEM 更高的方向偏一点（贴着那座高山）
            _cy2 = float(np.clip(_by + _pr * 0.45, BORDER + 8, BORDER + PLAY - 8))
            _cx2 = float(np.clip(_bx + _pr * 0.25, BORDER + 8, BORDER + PLAY - 8))
            _dd = np.sqrt((xx - _cx2) ** 2 + (yy - _cy2) ** 2)
            _w = np.clip((_pr - _dd) / max(_pb, 1.0), 0.0, 1.0)
            z = np.where(land, z * (1.0 - _w) + _pl * _w, z)
            print('  [高台] %s 旁建成 %.0f 层平台（半径 %.0f / 过渡 %.0f）' % (_bn, _pl, _pr, _pb))

    # ---- 平台之后再补一次河道：防止基地/矿脉平台把河堵死（水系必须连通）
    #      基地平台核心优先（不被河切开），其余河道一律恢复水面
    base_core = np.zeros_like(z, dtype=bool)
    for name, (cx, cy) in base_cells.items():
        base_core |= (np.sqrt((xx - cx) ** 2 + (yy - cy) ** 2) < pad_radius - 4.0)
    for name in rd:
        d = rd[name]
        carve = d < r_bed
        floor = rbed[name] + (d / max(r_bed, 1.0)) * 8.0
        if rbed[name] < WATER_Z:                       # 成水的河：平台之后恢复水面
            z = np.where(carve & ~base_core, np.minimum(z, floor), z)
            z = np.where(carve & ~base_core, np.minimum(z, WATER_Z - 6.0), z)
        else:                                          # 干谷：保持可通行，不抬高别的水面
            m = carve & ~base_core & ~wet_channel
            z = np.where(m, np.minimum(np.maximum(z, WATER_Z + 2.0), floor), z)

    # ---- 矿场摆放位「强制平整」：矿口方向的半圆盘一律压平到矿位高度。
    #      身后的山脚陡坎保留（仍满足"贴山脚、矿口朝外"），但矿口一侧保证可放矿场。
    #      过渡宽度按"每格落差 <= slope_max"自动摊开（落差越大过渡越宽），既保平又不造陡坡。
    flat_r = float(T.get('ore_mouth_flat', 20.0))
    soft = max(0.5, float(T.get('ore_mouth_flat_soft', 4.0)))
    slope_max = float(T.get('slope_max', 10.0))
    _ore = slots.get('ore') or []
    _angs = slots.get('ore_angles') or [0.0] * len(_ore)
    if _ore:
        R = int(math.ceil(flat_r + 40.0))
        core_r = flat_r * 0.8
        # 目标高度取「平台铺平之后」该矿核心区的中位数：
        # 同一个平台簇里的几块矿会得到同一个高度，避免相邻矿核心重叠时互相覆写成不同高度。
        _h0 = []
        for (cx, cy) in _ore:
            ix, iy = int(round(cx)), int(round(cy))
            r = int(core_r)
            y0, y1 = max(0, iy - r), min(GRID, iy + r + 1)
            x0, x1 = max(0, ix - r), min(GRID, ix + r + 1)
            _h0.append(float(np.median(z[y0:y1, x0:x1])))
        # 跑多遍：相邻矿脉的过渡带可能盖到别人的核心，后续遍重新压实
        for _pass in range(6):
          for _oi, ((cx, cy), ang) in enumerate(zip(_ore, _angs)):
            ix, iy = int(round(cx)), int(round(cy))
            h = _h0[_oi]
            mx, my = math.cos(ang), math.sin(ang)
            y0, y1 = max(0, iy - R), min(GRID, iy + R + 1)
            x0, x1 = max(0, ix - R), min(GRID, ix + R + 1)
            yy2, xx2 = np.mgrid[y0:y1, x0:x1]
            dx2, dy2 = xx2 - cx, yy2 - cy
            d2 = np.sqrt(dx2 ** 2 + dy2 ** 2)
            sub = z[y0:y1, x0:x1]
            need = np.maximum(soft, np.abs(sub - h) / max(slope_max / 1.5, 1e-6))   # 摊到多少格（留对角余量）
            w = np.clip((flat_r + need - d2) / need, 0.0, 1.0)
            w = np.where(d2 <= flat_r, 1.0, w)                          # 半径内一律压平（强制）
            # 只保留矿口锥形范围（±cone）：矿场就摆在这个扇面里；
            # 用整半平面会把 90° 方向的崖壁也算进来（实测边缘 1 格会留下 7 单位误差）
            cos_cone = math.cos(math.radians(float(T.get('ore_mouth_cone', 75.0))))
            w = np.where((dx2 * mx + dy2 * my) >= cos_cone * d2, w, 0.0)
            if w.max() <= 0:
                continue
            z[y0:y1, x0:x1] = sub * (1.0 - w) + h * w

        # 最终一遍：只压「核心半径」、不给过渡带 —— 迭代多遍也治不了"相邻矿脉按固定顺序
        # 反复覆写别人核心"的问题（后遍历的矿总能把别人的核心改成自己的高度），
        # 所以最后用核心掩码把每个矿的核心重新钉死。
        for _oi, ((cx, cy), ang) in enumerate(zip(_ore, _angs)):
            ix, iy = int(round(cx)), int(round(cy))
            h = _h0[_oi]
            mx, my = math.cos(ang), math.sin(ang)
            y0, y1 = max(0, iy - R), min(GRID, iy + R + 1)
            x0, x1 = max(0, ix - R), min(GRID, ix + R + 1)
            yy2, xx2 = np.mgrid[y0:y1, x0:x1]
            dx2, dy2 = xx2 - cx, yy2 - cy
            d2 = np.sqrt(dx2 ** 2 + dy2 ** 2)
            cos_cone = math.cos(math.radians(float(T.get('ore_mouth_cone', 75.0))))
            w = np.where((d2 <= core_r) & ((dx2 * mx + dy2 * my) >= cos_cone * d2), 1.0, 0.0)
            if w.max() <= 0:
                continue
            sub = z[y0:y1, x0:x1]
            z[y0:y1, x0:x1] = sub * (1.0 - w) + h * w

    # ---- 桥梁：桥段按官方间距（约 490 世界单位）跨在两岸之间，桥下保留水面
    b_cells, b_ang = compute_bridges(geo, z, r_bed, bank_ramp)
    cw_half = int(T.get('bridge_causeway', 0))
    if cw_half > 0:                                    # 可选：桥下做一条过河堤道（会截断水面）
        cw_z = bank_level - float(T.get('bridge_drop', 2.0))
        for i in range(0, len(b_cells), 2):
            (x1, y1), (x2, y2) = b_cells[i], b_cells[i + 1]
            steps = int(math.hypot(x2 - x1, y2 - y1)) + 1
            tsm = np.zeros_like(z, dtype=bool)
            for s in range(steps + 1):
                u = s / max(1, steps)
                px, py = x1 + (x2 - x1) * u, y1 + (y2 - y1) * u
                tsm |= (np.abs(xx - px) <= cw_half) & (np.abs(yy - py) <= cw_half)
            z = np.where(tsm & (z < cw_z), cw_z, z)
    if verbose and b_cells:
        print('  [桥梁] %d 座桥（桥段间距约 %d 单位）' % (
            len(b_cells) // 2, int(2 * float(T.get('bridge_span_cells', 24.5)) * CELL)))

    # ---- 高山层（不可通行）：DEM 高海拔区做成连片山体。
    #      放在**函数末尾**（此时 base_cells / slots 都已就绪），才能把出生点与矿区平台抠掉，
    #      否则高山会从基地中间穿过（实测 P1~P4 的 80x80 出现 76~156 落差）
    #      v11 分层模式下 v11 图里已经有 480 高山层，故跳过（避免用 DEM 高山覆盖蓝图）。
    try:
        if bool(T.get('v11_skip_peak')) and bool(T.get('use_v11_layers')):
            raise RuntimeError('__skip_v11__')
        _pk_from = float(T.get('peak_from_m', 850.0))
        _pk_lo, _pk_hi = T.get('peak_level', (480.0, 1000.0))
        _dp = os.path.join(os.path.dirname(os.path.abspath(cfg.__file__)), 'dem.npz')
        if os.path.isfile(_dp):
            _d = np.load(_dp)['dem'].astype(np.float64)
            if _d.shape == (GRID, GRID):
                _lm2 = z >= WATER_Z
                _peaks = (_d > _pk_from) & _lm2
                _peaks = keep_largest_components(opening(_peaks, 3), 4)          # 连片高山
                _padk = np.zeros_like(_peaks, dtype=bool)
                for _bn, (_bx, _by) in base_cells.items():
                    _padk |= ((xx - _bx) ** 2 + (yy - _by) ** 2) < (float(T['pad'][0]) + 10.0) ** 2
                for _ox, _oy in (slots.get('ore') or []):
                    _padk |= ((xx - _ox) ** 2 + (yy - _oy) ** 2) < (float(T['res_pad'][0]) + 8.0) ** 2
                _peaks &= ~_padk
                if _peaks.any():
                    _t = np.clip((_d - _pk_from) / max(1.0, float(_d.max()) - _pk_from), 0.0, 1.0)
                    z = np.where(_peaks, _pk_lo + _t * (_pk_hi - _pk_lo), z)
                    print('  [高山] %d 格（DEM > %.0f m）做成 %.0f~%.0f 不可通行山体（已避开平台）'
                          % (int(_peaks.sum()), _pk_from, _pk_lo, _pk_hi))
    except Exception as _e:
        print('  [高山] 跳过：%s' % str(_e)[:60])

    lo, hi = T['z_clip']
    z = np.clip(z, lo, LAND_Z + hi)
    # ---- 西侧/西南角的海湾按周边地形填成陆地（用户指定：圈出的那片水不要）
    #      v11 分层模式下这片海是蓝图的一部分，跳过填湾（否则会把 v11 的海改成陆地）
    _skip_fill = bool(T.get('v11_skip_water')) and bool(T.get('use_v11_layers'))
    _slx = float(T.get('sw_fill_x', 0.30)) * GRID
    _sly = float(T.get('sw_fill_y', 0.78)) * GRID
    if not _skip_fill:
        _swf = (xx < _slx) & (yy < _sly)
        if _swf.any():
            _fill = np.where(z >= WATER_Z, z, np.nan)
            _base_h = float(np.nanmedian(_fill[_swf])) if np.isfinite(_fill[_swf]).any() else (LAND_Z + 4.0)
            _nmask = _swf & (z < WATER_Z)
            z = np.where(_nmask, _base_h, z)
            print('  [填湾] 西侧海湾 %d 格 -> 陆地（高度 %.0f，取周边陆地中位）' % (int(_nmask.sum()), _base_h))

    # ---- 所有水域最终拉平：水面以下一律同一个高度（海面/河面是一整片平坦水域）
    z = np.where(z < WATER_Z, float(T.get('sea_flat', LAND_Z - 20.0)), z)
    if T.get('ore_bands') and not slots.get('ore_kind'):
        _avoid = []
        for _key in ('derricks', 'houses', 'posts', 'garage',
                     'airports', 'hospitals', 'academies', 'shipyards'):
            _avoid.extend(slots.get(_key) or [])
        _ore, _ang, _kind, _own = apply_ore_bands(
            z, base_cells, list(slots.get('ore') or []),
            list(slots.get('ore_angles') or []), T,
            list(slots.get('ramps') or []), _avoid, verbose=verbose)
        slots['ore'] = _ore
        slots['ore_angles'] = _ang
        slots['ore_kind'] = _kind
        slots['ore_owner'] = _own
    raw = np.clip(np.round(z / Z_SCALE), 0, 65535).astype(np.uint16)
    return geo, raw, land, amp, d_edge, base_cells, slots, b_cells, b_ang


def texture_layout(raw, land, d_edge):
    """Return per-cell tile values (uint16) and impassability bits."""
    z = raw.astype(np.float64) * Z_SCALE
    gy, gx = np.gradient(z)
    slope = np.sqrt(gx ** 2 + gy ** 2)          # world z units per cell
    impassable = (slope > SLOPE_IMPASSABLE) | (d_edge < 3.0)
    return z, slope, impassable


def main():
    geo, raw, land, amp, d_edge, base_cells, slots, _b_cells, _b_ang = build_terrain()
    z = raw.astype(np.float64) * Z_SCALE
    print('z range %.1f .. %.1f   water %.1f' % (z.min(), z.max(), WATER_Z))
    print('cells below water: %d  (%.1f%%)' % ((z < WATER_Z).sum(), 100.0 * (z < WATER_Z).mean()))
    print('bases:')
    for name, (cx, cy) in base_cells.items():
        print('   %-4s cell (%.0f,%.0f) world (%.0f,%.0f) z=%.1f' % (
            name, cx, cy, (cx - BORDER) * CELL, (cy - BORDER) * CELL, z[int(round(cy)), int(round(cx))]))

    if _b_cells:
        print('  [桥梁] %d 座桥 / %d 个桥段' % (len(_b_cells) // 2, len(_b_cells)))
    names = list(base_cells.keys())
    np.savez_compressed(os.path.join(SRCDIR, 'terrain.npz'), raw=raw, land=land,
                        base_names=np.array('|'.join(names)),
                        base_cells=np.array([base_cells[n] for n in names], dtype=np.float64),
                        ore_cells=np.array(slots.get('ore') or [], dtype=np.float64),
                        ore_angles=np.array(slots.get('ore_angles') or [], dtype=np.float64),
                        derrick_cells=np.array(slots.get('derricks') or [], dtype=np.float64),
                        house_cells=np.array(slots.get('houses') or [], dtype=np.float64).reshape(-1, 2),
                        house_kinds=np.array(slots.get('house_kinds') or [], dtype=np.int16),
                        post_cells=np.array(slots.get('posts') or [], dtype=np.float64),
                        garage_cells=np.array(slots.get('garage') or [], dtype=np.float64),
                        airport_cells=np.array(slots.get('airports', []), dtype=np.float64),
                        hospital_cells=np.array(slots.get('hospitals', []), dtype=np.float64),
                        academy_cells=np.array(slots.get('academies', []), dtype=np.float64),
                        shipyard_cells=np.array(slots.get('shipyards') or [], dtype=np.float64).reshape(-1, 2),
                        boundary=main_boundary_mask(geo, GRID),
                        bridge_cells=np.array(_b_cells, dtype=np.float64).reshape(-1, 2),
                        bridge_angles=np.array(_b_ang, dtype=np.float64),
                        ramp_json=np.array(json.dumps(slots.get('ramps') or [])),
                        ore_kind=np.asarray(slots.get('ore_kind') if slots.get('ore_kind') is not None else [], dtype=np.int8),
                        ore_owner=np.asarray(slots.get('ore_owner') if slots.get('ore_owner') is not None else [], dtype=np.int16))

    # ---- preview (hypsometric colours)
    col = np.zeros((GRID, GRID, 3), dtype=np.uint8)
    zz = z
    deep = zz < WATER_Z - 40
    shal = (zz >= WATER_Z - 40) & (zz < WATER_Z)
    col[deep] = (18, 56, 120)
    col[shal] = (40, 96, 170)
    beach = (zz >= WATER_Z) & (zz < WATER_Z + 4)
    col[beach] = (214, 200, 150)
    landm = zz >= WATER_Z + 4
    lv = np.clip((zz - (WATER_Z + 4)) / 330.0, 0.0, 1.0) ** 0.85
    idx = np.clip((lv * 255).astype(int), 0, 254)
    # colour ramp: green lowland -> olive -> brown -> grey rock -> white peak
    ramp = np.zeros((256, 3), dtype=np.uint8)
    for i in range(256):
        f = i / 255.0
        if f < 0.42:
            g = f / 0.42
            ramp[i] = (58 + 52 * g, 122 + 44 * g, 58 - 12 * g)
        elif f < 0.66:
            g = (f - 0.42) / 0.24
            ramp[i] = (110 + 50 * g, 166 - 16 * g, 46 - 18 * g)
        elif f < 0.85:
            g = (f - 0.66) / 0.19
            ramp[i] = (160 + 30 * g, 150 - 20 * g, 28 + 22 * g)
        else:
            g = (f - 0.85) / 0.15
            ramp[i] = (190 + 55 * g, 130 + 80 * g, 50 + 130 * g)
    col[landm] = ramp[idx[landm]]
    # ⚠ 轴向（T1.8.8 实机锚定）：显示方向 = 内部帧（行 0 = 北、海在下半），**不 flipud**；
    #   出生点标注直接用内部帧行号 cy。与其它预览图（preview/build_view/passability）同向。
    pim = Image.fromarray(col)
    d = ImageDraw.Draw(pim)
    for name, (cx, cy) in base_cells.items():
        r = 8
        d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(255, 255, 0), outline=(0, 0, 0))
        d.text((cx + 9, cy - 5), name, fill=(255, 255, 255))
    pim.save(os.path.join(SRCDIR, 'terrain_preview.png'))
    print('saved source/terrain.npz + source/terrain_preview.png')


if __name__ == '__main__':
    main()
