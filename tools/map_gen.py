# -*- coding: utf-8 -*-
"""地图组装器（配方驱动）：把配方 + 生成好的地形组装成可加载的 RA3 .map。

骨架取自 templates/ 下的 8 人模板，保留其脚本/阵营/队伍/光照系统（房间 6 可选位 +
2 个固定 AI 位），替换高程、地表纹理、水面、对象布局与地图文本。

用法：  python tools/map_gen.py [--map <map_id>]
"""
import os, sys, math, struct, json, argparse
import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)
from ra3map import *
import project as proj

_ap = argparse.ArgumentParser(add_help=False)
_ap.add_argument('--map', dest='map_id', default=None, help='maps/<id>/ 配方')
_args, _unknown = _ap.parse_known_args()

P = proj.current(_args.map_id)
cfg = P.cfg
import layer_tex
layer_tex.apply_mountain_preset(cfg)
import terrain_gen as bt          # 共用同一份配方与路径解析

WS = proj.ROOT
TEMPLATE = P.template
MAPNAME = P.map_name
ASSETS = P.assets           # assets/<map name>/  = 发布包
SRCDIR = P.source           # assets/<map name>/source/ = 生成物与自检图
OUTDIR = ASSETS

# 地表纹理表与块尺寸（配方提供，见 maps/<id>/config.py）
TEX_SET = [tuple(e) for e in cfg.TEX_SET]
CELL_SIZE = cfg.CELL_SIZE
GRASS_NAMES = tuple(getattr(cfg, 'GRASS_TEXTURES', ()) or ())
WATER = dict(getattr(cfg, 'WATER', {}) or {})
MAP_STR_INTRO = P.render(getattr(cfg, 'MAP_STR_INTRO', ''))
MAP_STR_DEBUG = P.render(getattr(cfg, 'MAP_STR_DEBUG', ''))
MAP_STR_INFO = {k: P.render(v) for k, v in (getattr(cfg, 'MAP_STR_INFO', {}) or {}).items()}
WAYPOINT_LAYOUT = dict(getattr(cfg, 'WAYPOINT_LAYOUT', {}) or {})


def waypoint_plan():
    """出生点计划：human 依次 Player_1_Start…，固定位接续编号并加 "(imported)"。

    返回 [(waypoint 名, BASES 条目)]；配置里显式给出 WAYPOINT_LAYOUT 时以其为准。
    """
    bases = list(bt.BASES)
    plan = []
    if WAYPOINT_LAYOUT:
        for name, (lon, lat) in WAYPOINT_LAYOUT.items():
            match = min(bases, key=lambda b: (b[0] - lon) ** 2 + (b[1] - lat) ** 2)
            plan.append((name, match))
        return plan
    humans = [b for b in bases if b[4] == 'human']
    fixed = [b for b in bases if b[4] != 'human']
    for i, b in enumerate(humans, 1):
        plan.append(('Player_%d_Start' % i, b))
    for i, b in enumerate(fixed, len(humans) + 1):
        plan.append(('Player_%d_Start (imported)' % i, b))
    return plan
# 两个固定 AI 每张图一样：p7 苏联欧列格，p8 帝国晋三。不读配方。
_FIXED_YARDS = (
    ('SovietConstructionYard', 'Player_7_Start (imported)', 2, -3, 20),
    ('JapanConstructionYard', 'Player_8_Start (imported)', -2, 3, 20),
)
_FIXED_FACTION = {
    'PlyrCivilian': 'PlayerTemplate:Soviet',
    'PlyrCreeps': 'PlayerTemplate:Japan',
}
# 游戏数据里欧列格是 1SovietBalanced，晋三是 1JapanBalanced。
# 夏娜是 3SovietAirMarshall，贤治是 2JapanMechaWarfare，一个位子只能钉一位。
_FIXED_PERSONALITY = {
    'PlyrCivilian': 'AIPersonalityDefinition:1SovietBalanced',
    'PlyrCreeps': 'AIPersonalityDefinition:1JapanBalanced',
}
_FIXED_UNITS = {
    'PlyrCreeps': {'GameObject:AlliedEngineer': 'GameObject:JapanEngineer'},
}
BASE_ORE_OFFSETS = tuple(getattr(cfg, 'BASE_ORE_OFFSETS', ((-16, -11), (17, -9), (-2, 18))))
CENTER_ORE = list(getattr(cfg, 'CENTER_ORE', []) or [])
FORD_ORE = list(getattr(cfg, 'FORD_ORE', []) or [])
FORD_ORE_OFFSET = tuple(getattr(cfg, 'FORD_ORE_OFFSET', (6, 6)))
DERRICK_POS = list(getattr(cfg, 'DERRICK_POS', []) or [])
POST_POS = list(getattr(cfg, 'POST_POS', []) or [])
GARAGE_POS = getattr(cfg, 'GARAGE_POS', None)
BRIDGE_TYPE = getattr(cfg, 'BRIDGE_TYPE', 'Bridge2a')
DECOR_POS = list(getattr(cfg, 'DECOR_POS', []) or [])
DECOR_TYPES = tuple(getattr(cfg, 'DECOR_TYPES', ()) or ())
PAD_CLEARANCE = getattr(cfg, 'PAD_CLEARANCE', 16.0)
RESOURCE_PAD_CLEARANCE = getattr(cfg, 'RESOURCE_PAD_CLEARANCE', 15.0)
# 摆放互斥：基地与矿/资源点互不重叠
ORE_BASE_CLEARANCE = getattr(cfg, 'ORE_BASE_CLEARANCE', 30.0)
ORE_MIN_SPACING = getattr(cfg, 'ORE_MIN_SPACING', 16.0)
RESOURCE_MIN_SPACING = getattr(cfg, 'RESOURCE_MIN_SPACING', 14.0)
DECOR_RESOURCE_CLEARANCE = getattr(cfg, 'DECOR_RESOURCE_CLEARANCE', 10.0)
ORE_CLEAR_RADIUS = getattr(cfg, 'ORE_CLEAR_RADIUS', 18.0)
ORE_LEFT_RING = getattr(cfg, 'ORE_LEFT_RING', (150.0,))[0]
DERRICKS_PER_BASE = int(getattr(cfg, 'DERRICKS_PER_BASE', 2))
PREVIEW = dict(title=P.render(getattr(cfg, 'PREVIEW_TITLE', P.base_name)),
               subtitle=P.render(getattr(cfg, 'PREVIEW_SUBTITLE', '')),
               labels=dict(getattr(cfg, 'PREVIEW_LABELS', {}) or {}),
               footer=P.render(getattr(cfg, 'PREVIEW_FOOTER', '')),
               legend=tuple(getattr(cfg, 'PREVIEW_LEGEND_ITEMS', ()) or ()))

TEX_INDEX = {e[0]: i for i, e in enumerate(TEX_SET)}
CELL_SIZE = 4          # every texture uses 8x8-cell tiles (verified on real maps)


# ------------------------------------------------------------------ helpers
def clamp_cell(v):
    return int(max(0, min(bt.GRID - 1, round(v))))


def cell_to_world(cx, cy):
    """Heightmap cell（全 1040 网格，内部帧：行 0 = 最北）-> 世界坐标（游戏帧）。

    ⚠ 轴向最终结论（T1.8.8 实机锚定）：
      引擎把高度场第 0 行画在**屏幕最下方**（世界 y=0 = 南缘）。
      内部帧行 0 = 最北 -> 写 .map 时高度场 `flipud(raw)`、物件世界 y 用本函数翻 y。
      游戏内实际显示 = **北朝上、海（南）在下方**（T1.8.7 实机确认 3D 地形正确）。
      （T1.8.2~1.8.7 的注释称"游戏里海在上半"是错的——错把高度场数组行序当屏幕方向。）
    => 高度场写出、BlendTileData、物件世界 y 全部 flipud；art.tga 画布翻转后写出
       （TGA 自底向上存储会再翻一次，读回 = 北朝上，见 write_art_tga）。
    """
    return ((cx - bt.BORDER) * bt.CELL,
            ((bt.GRID - 1 - bt.BORDER) - cy) * bt.CELL)


def world_to_cell_y(wy):
    """世界 y -> 内部高度场行号（cell_to_world 的逆）。"""
    return (bt.GRID - 1 - bt.BORDER) - wy / bt.CELL


class Terrain:
    def __init__(self):
        npz = os.path.join(SRCDIR, 'terrain.npz')
        if not os.path.isfile(npz):
            raise SystemExit('缺少地形数据 %s —— 请先运行 terrain 阶段：\n'
                             '    python tools/build_all.py --map %s\n'
                             '（版本号递增后发布目录会变，地形需要为新版本重新生成）' % (npz, P.map_id))
        d = np.load(npz)
        self.raw = d['raw']
        self.land = d['land']
        names = str(d['base_names']) if 'base_names' in d else ''
        self.base_names = [n for n in names.split('|') if n]
        self.base_cells = d['base_cells'] if 'base_cells' in d else None
        # 资源点位置由 terrain_gen 解算并铺平，这里直接落位
        self.ore_cells = [tuple(c) for c in d['ore_cells']] if 'ore_cells' in d else []
        self.ore_angles = list(d['ore_angles']) if 'ore_angles' in d else []
        self.derrick_cells = [tuple(c) for c in d['derrick_cells']] if 'derrick_cells' in d else []
        _hc = d['house_cells'] if 'house_cells' in d.files else None
        self.house_cells = [tuple(c) for c in _hc] if _hc is not None and _hc.size else []
        _hk = d['house_kinds'] if 'house_kinds' in d.files else None
        self.house_kinds = [int(v) for v in _hk] if _hk is not None and _hk.size else []
        self.post_cells = [tuple(c) for c in d['post_cells']] if 'post_cells' in d else []
        self.garage_cells = [tuple(c) for c in d['garage_cells']] if 'garage_cells' in d else []
        self.airport_cells = [tuple(c) for c in d['airport_cells']] if 'airport_cells' in d else []
        self.hospital_cells = [tuple(c) for c in d['hospital_cells']] if 'hospital_cells' in d else []
        self.academy_cells = [tuple(c) for c in d['academy_cells']] if 'academy_cells' in d else []
        _sy = d['shipyard_cells'] if 'shipyard_cells' in d.files else None
        self.shipyard_cells = [tuple(c) for c in _sy] if _sy is not None and _sy.size else []
        self.bridge_cells = [tuple(c) for c in d['bridge_cells']] if 'bridge_cells' in d else []
        self.bridge_angles = list(d['bridge_angles']) if 'bridge_angles' in d else []
        self.ramps = json.loads(str(d['ramp_json'])) if 'ramp_json' in d.files else []
        self.z = self.raw.astype(np.float64) * bt.Z_SCALE
        self.geo = bt.Geo()
        gy, gx = np.gradient(self.z)
        self.slope = np.sqrt(gx ** 2 + gy ** 2)
        yy, xx = np.mgrid[0:bt.GRID, 0:bt.GRID]
        self.xx, self.yy = xx, yy
        self.d_edge = np.minimum.reduce([
            xx - bt.BORDER, yy - bt.BORDER,
            (bt.GRID - 1 - bt.BORDER) - xx, (bt.GRID - 1 - bt.BORDER) - yy]).astype(np.float64)
        self.passable = ((self.z > bt.WATER_Z + 1.0) & (self.slope < bt.SLOPE_IMPASSABLE)
                         & (self.land) & (self.d_edge > 1))
        # 出生点使用 terrain_gen 解析后的坐标。npz 按当时的 BASES 顺序保存；
        # 配方改了显示名之后仍按顺序对上当前名称，避免退回过时经纬度。
        cells = None if self.base_cells is None else list(self.base_cells)
        if cells is not None and len(cells) == len(bt.BASES):
            self.base = {b[2]: (float(c[0]), float(c[1])) for b, c in zip(bt.BASES, cells)}
        elif cells is not None:
            self.base = {str(n): (float(c[0]), float(c[1]))
                         for n, c in zip(self.base_names, cells)}
        else:
            self.base = {}

    def snap(self, cx, cy, radius=14):
        """Nearest passable cell to (cx, cy) within radius."""
        cx, cy = clamp_cell(cx), clamp_cell(cy)
        if self.passable[cy, cx]:
            return cx, cy
        best, bd = None, 1e18
        for dy in range(-radius, radius + 1):
            for dx in range(-radius, radius + 1):
                x, y = cx + dx, cy + dy
                if 0 <= x < bt.GRID and 0 <= y < bt.GRID and self.passable[y, x]:
                    dd = dx * dx + dy * dy
                    if dd < bd:
                        bd, best = dd, (x, y)
        return best if best else (cx, cy)


# ------------------------------------------------------------------ terrain chunks
def build_heightmap_payload(t):
    w = bytearray()
    w += struct.pack('<IIII', bt.GRID, bt.GRID, bt.BORDER, 1)
    w += struct.pack('<IIII', 0, 0, bt.PLAY, bt.PLAY)
    w += struct.pack('<I', bt.GRID * bt.GRID)
    # ⚠ 行序换算到游戏帧：高度场第 0 行 = 世界 y=0 = 屏幕下方（南）。
    #   内部帧行 0 = 北 -> 写出前 flipud。见 cell_to_world 的说明；
    #   BlendTileData / 物件世界 y 同步翻（art.tga 另有 TGA 自底向上的一次翻）。
    w += np.flipud(t.raw).astype('<u2').tobytes()
    return bytes(w)


def pick_texture(z, slope, land, water):
    """Texture name for a tile from height / slope / land-ness.

    注意：平原与**二三层高地一律草地**（不再按高度铺 Dirt/Elevation），
    城市范围另由 build_blendtiles_payload 铺水泥路。
    """
    if z < bt.WATER_Z - 30:
        return 'RA3_ShallowSeaFloor' if z < bt.WATER_Z - 60 else 'Reef_Yucatan01'
    if z < bt.WATER_Z:
        return 'Reef_Yucatan02'
    if z < bt.WATER_Z + 3.0:
        return 'Sand_Hawaii02' if land else 'Reef_Havana02'
    if slope > 34.0:
        return 'Rock_Yucatan02' if (z % 2 < 1) else 'Rock_Yucatan01'
    if slope > 22.0:
        return 'Rock_Yucatan03'
    # 注意：不要用 z < 水面+12 判沙滩 —— 平原就在 210~212，会把整片平原判成沙。
    # 沙滩改由调用方按「离水距离」判定。
    return None      # 其余（平原 + 一层/二层高地）一律草地，由噪声选品种


def build_blendtiles_payload(t, template_bt=None, seed=7, decorate=None):
    rng = np.random.RandomState(seed)
    z, slope, land = t.z, t.slope, t.land
    W = H = bt.GRID
    # ---- texture per 8x8 tile block
    chosen = np.zeros((H, W), dtype=np.uint16)      # tile *block* texture index
    grass_names = GRASS_NAMES or tuple(n for n, _, _ in TEX_SET if n.startswith('Grass'))
    grass_variants = [TEX_INDEX[n] for n in grass_names if n in TEX_INDEX]
    noise = bt.fbm((H, W), float(getattr(cfg, 'TEX_NOISE_CELLS', 9)), 2, 909)
    # ---- 按陆地层级分草皮家族（同层视觉一致、层间相近可分辨；配置 LAYER_TEX）
    _layer_tex = dict(getattr(cfg, 'LAYER_TEX', {}) or {})
    _layer_lv = sorted(_layer_tex.keys())
    _layer_variants = {lv: [TEX_INDEX[n] for n in names if n in TEX_INDEX]
                       for lv, names in _layer_tex.items()}

    _layer_strict = bool(getattr(cfg, 'LAYER_TEX_STRICT', False)) and bool(_layer_lv)
    _lv_arr = np.asarray(_layer_lv, dtype=np.float64) if _layer_lv else None

    def layer_grass(zz, noise_v, zblock=None):
        """该层纹理。严格模式按块内陆地的多数层，否则按块均高找最近层。"""
        if _layer_lv:
            if _layer_strict and zblock is not None:
                flat = np.asarray(zblock, dtype=np.float64).ravel()
                landv = flat[flat >= bt.WATER_Z]
                if landv.size:
                    nearest = np.abs(landv[:, None] - _lv_arr[None, :]).argmin(axis=1)
                    lv = _layer_lv[int(np.bincount(nearest, minlength=len(_layer_lv)).argmax())]
                else:
                    lv = min(_layer_lv, key=lambda v: abs(v - zz))
            else:
                lv = min(_layer_lv, key=lambda v: abs(v - zz))
            variants = _layer_variants.get(lv) or grass_variants
        else:
            variants = grass_variants
        k = int(noise_v * len(variants)) % len(variants)
        return variants[k]
    # ---- 城市一定范围内铺水泥路（SteelDeck=水泥/铺装面 + 碎石路做变化）
    city_r = float(getattr(cfg, 'CITY_TEX_R', 30.0))
    city_pts = [(cx, cy) for _n, cx, cy in bt.city_centers(t.geo)]
    city_mask = np.zeros((H, W), dtype=bool)
    for _cx, _cy in city_pts:
        city_mask |= ((t.xx - _cx) ** 2 + (t.yy - _cy) ** 2) < city_r * city_r
    _city_names = tuple(getattr(cfg, 'CITY_TEX', ('SteelDeck01', 'BB_Gravel01', 'Dirt_Yucatan06')))
    city_tex = [TEX_INDEX[n] for n in _city_names if n in TEX_INDEX] or grass_variants
    # 沙滩只铺水边的陆地。水下距离是 0，不能把整片海床铺成沙。
    d_water = bt.distance_transform(z < bt.WATER_Z)
    beach_r = float(getattr(cfg, 'BEACH_TEX_R', 6.0))
    _sea_bands = [(float(a), str(b)) for a, b in (getattr(cfg, 'SEA_TEX_BANDS', ()) or ())]
    d_land = bt.distance_transform(z >= bt.WATER_Z) if _sea_bands else None
    for by in range(0, H, 8):
        for bx in range(0, W, 8):
            zz = float(z[by:by+8, bx:bx+8].mean())
            ss = float(slope[by:by+8, bx:bx+8].mean())
            ll = bool(land[by:by+8, bx:bx+8].mean() > 0.5)
            if (not getattr(cfg, 'CITY_TEX_ROUND', False)
                    and city_mask[by, bx] and ll and zz > bt.WATER_Z + 3.0):
                k = int(noise[by, bx] * len(city_tex)) % len(city_tex)
                idx = city_tex[k]
                chosen[by:by+8, bx:bx+8] = idx
                continue
            if zz >= bt.WATER_Z and 0 < d_water[by, bx] <= beach_r:
                chosen[by:by+8, bx:bx+8] = TEX_INDEX.get('Sand_Hawaii02', grass_variants[0])
                continue
            if _sea_bands and zz < bt.WATER_Z:
                dist = float(d_land[by, bx])
                name = _sea_bands[-1][1]
                for lim, nm in _sea_bands:
                    if dist <= lim:
                        name = nm
                        break
                chosen[by:by+8, bx:bx+8] = TEX_INDEX.get(
                    name, TEX_INDEX.get('RA3_ShallowSeaFloor', grass_variants[0]))
                continue
            name = pick_texture(zz, ss, ll, zz < bt.WATER_Z)
            # 严格分层时，缓坡不再换成统一岩壁，块面跟着该层自己的纹理。
            if _layer_strict and zz >= bt.WATER_Z:
                name = None
            if name is None:
                idx = layer_grass(zz, noise[by, bx], z[by:by+8, bx:bx+8] if _layer_strict else None)
            else:
                idx = TEX_INDEX[name]
            chosen[by:by+8, bx:bx+8] = idx
    # 圆形刷：只铺圆内的陆地格。8×8 整块铺会把城刷成方的。
    if getattr(cfg, 'CITY_TEX_ROUND', False) and city_tex:
        paint = city_mask & np.asarray(land, dtype=bool) & (z > bt.WATER_Z + 3.0)
        if len(city_tex) == 1:
            chosen[paint] = city_tex[0]
        else:
            pick = np.asarray(city_tex, dtype=np.uint16)
            kk = (noise * len(city_tex)).astype(np.int32) % len(city_tex)
            chosen[paint] = pick[kk[paint]]
    # ---- 斜坡车道铺土路纹理（视觉标记"这里能上下层"；车道定义与 carve_marked_ramps 一致）
    _ramp_tex = TEX_INDEX.get('Dirt_Yucatan06')
    _ramp_tex2 = TEX_INDEX.get('Gravel_Yucatan01')
    t.ramp_tex_mask = np.zeros((H, W), dtype=bool)
    if _ramp_tex is not None:
        _lb_all = np.zeros((H // 8, W // 8), dtype=bool)
        _eb_all = np.zeros((H // 8, W // 8), dtype=bool)
        _ramps = list(getattr(t, 'ramps', None) or [])
        for rs in _ramps:
            cx, cy = float(rs['x']), float(rs['y'])
            hw = max(16.0, float(rs.get('w', 56)) * 0.75)
            hh = max(16.0, float(rs.get('h', 56)) * 0.75)
            ux, uy, _lo, _hi = bt.ramp_uphill(z, rs)
            if rs.get('auto'):
                half_u = max(10.0, min(20.0, float(rs.get('span', 28)) * 0.5))
                half_v = max(4.0, min(14.0, float(rs.get('width', 28)) * 0.35))
            else:
                # 手工框仍按目标坡度把土路拉到引道末端
                _lv = rs.get('pair') or rs.get('layers') or (212, 280)
                _dz = abs(float(max(_lv)) - float(min(_lv)))
                half_u = max(max(6.0, min(hw, hh) * 0.5), 0.75 * _dz / 2.5) + 2.0
                half_v = max(4.0, min(hw, hh) * 0.35) + 2.0
            uu = (t.xx - cx) * ux + (t.yy - cy) * uy
            vv = -(t.xx - cx) * uy + (t.yy - cy) * ux
            lane = (np.abs(uu) <= half_u) & (np.abs(vv) <= half_v)
            if not rs.get('auto'):
                lane = lane & (z > bt.WATER_Z + 2.0)
            edge = (np.abs(uu) <= half_u + 8.0) & (np.abs(vv) <= half_v + 8.0) \
                & (z > bt.WATER_Z + 2.0) & ~lane
            # 8x8 块级：块内车道占比 >=25% 换土路；边缘带 >=25% 换碎石过渡
            lb = lane.reshape(H // 8, 8, W // 8, 8).mean(axis=(1, 3)) >= 0.25
            eb = (edge.reshape(H // 8, 8, W // 8, 8).mean(axis=(1, 3)) >= 0.25) & ~lb
            _lb_all |= lb
            _eb_all |= eb
            chosen = np.where(np.kron(lb, np.ones((8, 8), dtype=bool)), _ramp_tex, chosen)
            if _ramp_tex2 is not None:
                chosen = np.where(np.kron(eb, np.ones((8, 8), dtype=bool)), _ramp_tex2, chosen)
        t.ramp_tex_mask = np.kron(_lb_all | _eb_all, np.ones((8, 8), dtype=bool))
    # 层纹理和斜坡车道到这里为止。装饰盖上来，再打包。
    if decorate is not None:
        chosen = decorate(t, chosen, z)
    # ---- per-cell tile values: block texture base + sub-tile offset (verified order)
    tiles = np.zeros((H, W), dtype=np.uint16)
    oy, ox = np.mgrid[0:H, 0:W]
    sub = (((oy % 8) // 2) * 4 + ((ox % 8) // 2)) * 4 + (oy % 2) * 2 + (ox % 2)
    tiles = (chosen.astype(np.uint32) * 64 + sub).astype(np.uint16)

    zeros16 = np.zeros((H, W), dtype=np.uint16)

    def pack_bits(mask):
        # 每行按字节对齐、LSB-first；np.packbits 行为一致且快得多
        return np.packbits(mask.astype(np.uint8), axis=1, bitorder='little').tobytes()

    impassable = ((slope > bt.SLOPE_IMPASSABLE)
                  | (z > bt.WATER_Z + bt.PEAK_IMPASSABLE)
                  | (~land & (z > bt.WATER_Z)) | (t.d_edge < 3.0)
                  | bt.shore_block_mask(z, getattr(t, 'ramps', None) or []))
    # keep base plateaus and resource pads fully open (they are flattened for base building)
    for name, (cx, cy) in t.base.items():
        d = np.sqrt((t.xx - cx) ** 2 + (t.yy - cy) ** 2)
        impassable &= ~(d < getattr(cfg, 'PAD_CLEARANCE', 26.0))
    # 矿脉豁免洞必须打在**最终落点**（t.ore_cells）上——⚠ 教训：曾用设计槽位
    # bt.ore_slots()，而最终矿位经挪移可偏出几十格，洞全打歪（矿场摆不下）。
    # 豁免洞与矿场建造圆盘同一半径。矿只落在整盘都平的格子上，所以不会在层坎上开口。
    _ore_cells = list(t.ore_cells) or [tuple(c) for c in bt.ore_slots(t.geo, base_cells=t.base)[0]]
    _br = max(float(getattr(cfg, 'ORE_CLEAR_RADIUS', 20.0)),
              float(bt.T.get('ore_mouth_r1', 20)))
    for (cx, cy) in _ore_cells:
        d = np.sqrt((t.xx - cx) ** 2 + (t.yy - cy) ** 2)
        impassable &= ~((d <= _br) & (t.d_edge >= 1.0))
    allzero = np.zeros((H, W), dtype=bool)

    # ⚠ 轴向：逐格数组必须与高度场同帧 -> 高度场 flipud，这里也 flipud。
    #   （曾因只翻高度场不翻这里，出现"游戏地形对、小地图/边界线/城市反"。）
    tiles = np.flipud(tiles)
    impassable = np.flipud(impassable)
    blocked = getattr(t, 'decor_block', None)
    if blocked is None or np.shape(blocked) != allzero.shape:
        blocked = allzero
    else:
        blocked = np.flipud(np.asarray(blocked, dtype=bool))

    w = bytearray()
    w += struct.pack('<I', W * H)
    w += tiles.astype('<u2').tobytes()
    w += zeros16.tobytes()          # Blends
    w += zeros16.tobytes()          # ThreeWayBlends
    w += zeros16.tobytes()          # CliffTextures
    w += pack_bits(impassable)      # Impassability            (v > 6)
    w += pack_bits(allzero)         # ImpassabilityToPlayers   (v >= 10)
    w += pack_bits(allzero)         # PassageWidths            (v >= 11)
    w += pack_bits(allzero)         # ExtraPassability         (v >= 15)
    w += pack_bits(np.ones((H, W), dtype=bool))   # Visibility  (v >= 17); all ones in real maps
    w += pack_bits(blocked)         # Buildability             (v >= 24); 0 = buildable
    w += pack_bits(allzero)         # ImpassabilityToAirUnits  (v >= 24)
    w += pack_bits(allzero)         # TiberiumGrowability      (v >= 24)
    w += bytes(H * W)               # DynamicShrubberyDensity   (v >= 25)
    w += struct.pack('<I', sum(CELL_SIZE * CELL_SIZE for _ in TEX_SET))   # TextureCellCount
    # Blend descriptions are unused (every cell's blend index is 0), but keeping the
    # template's table makes the chunk structurally identical to real WorldBuilder output.
    descs = (template_bt or {}).get('blend_descriptions', []) if template_bt else []
    w += struct.pack('<I', len(descs) + 1)      # BlendsCount raw (stored value = count + 1)
    w += struct.pack('<I', 1)                   # CliffMappingsCount raw
    w += struct.pack('<I', len(TEX_SET))
    for i, entry in enumerate(TEX_SET):
        name = entry[0]
        w += struct.pack('<IIII', i * CELL_SIZE * CELL_SIZE, CELL_SIZE * CELL_SIZE, CELL_SIZE, 0)
        nb = name.encode('ascii')
        w += struct.pack('<H', len(nb)) + nb
    w += struct.pack('<II', 0xCDCDCDCD, 0)   # magic1 (uninitialised marker in real maps), magic2
    for b in descs:
        w += struct.pack('<I', b['secondary_tile'])
        w += bytes(b['direction'][:4])
        w += struct.pack('<BB', b['flags'], b['two_sided'])
        w += struct.pack('<II', b['magic1'], b['magic2'])
    return bytes(w)


def build_water_payload():
    """One water polygon covering the whole map at height 200 (as in working naval maps)."""
    w = bytearray()
    w += struct.pack('<I', 1)
    w += struct.pack('<I', 1)                                    # UniqueID
    for s in ('New Water Area', ''):
        b = s.encode('ascii'); w += struct.pack('<H', len(b)) + b
    w += struct.pack('<f', float(WATER.get('uv_scroll', 0.06)))   # UvScrollSpeed
    w += struct.pack('<B', int(WATER.get('additive', 0)))        # UseAdditiveBlending
    for s in (WATER.get('bump', 'WaterRippleBump'), WATER.get('sky', 'SkyEnv')):
        b = s.encode('ascii'); w += struct.pack('<H', len(b)) + b
    side = bt.PLAY * bt.CELL
    # 引擎坐标上限：水面多边形点必须留在可玩区内（越界会触发 Drawable/Object position too large）
    pts = [(0.0, 0.0), (0.0, side), (side, side), (side, 0.0)]
    w += struct.pack('<I', len(pts))
    for x, y in pts:
        w += struct.pack('<ff', x, y)
    w += struct.pack('<I', int(WATER.get('height') or bt.WATER_Z))   # WaterHeight
    for s in (WATER.get('fx', 'FXOceanRA3'), WATER.get('depth_colors', 'LUTDepthTint.tga')):
        b = s.encode('ascii'); w += struct.pack('<H', len(b)) + b
    return bytes(w)


# ------------------------------------------------------------------ objects
def build_objects(t, template_objects):
    geo = t.geo
    objs = [dict(o) for o in template_objects]          # keep template order & properties

    def set_pos(o, lon, lat, jitter=0.0, snap=16, clear_pads=False):
        cx, cy = geo.to_cell(lon, lat)
        if jitter:
            cx += jitter * 3.2; cy += jitter * 2.7
        if clear_pads:
            for bx, by in (list(t.base.values()) or
                           [geo.to_cell(lon2, lat2) for lon2, lat2, _, _, _ in bt.BASES]):
                dx, dy = cx - bx, cy - by
                d = math.hypot(dx, dy)
                if d < RESOURCE_PAD_CLEARANCE:
                    f = ((RESOURCE_PAD_CLEARANCE + 0.5) / d) if d > 0.01 else 1.0
                    cx, cy = bx + dx * f, by + dy * f
        cx, cy = t.snap(cx, cy, snap)
        cx, cy = clamp_playable(cx, cy)
        o['position'] = (cell_to_world(cx, cy)[0], cell_to_world(cx, cy)[1], 0.0)

    def clamp_playable(cx, cy):
        """对象坐标强制钳进可玩区（越出可玩区会让游戏崩 Drawable/Object position too large）。"""
        lo = bt.BORDER + 2.0
        hi = bt.BORDER + bt.PLAY - 2.0
        return (min(max(cx, lo), hi), min(max(cy, lo), hi))

    def set_cell(o, cx, cy, snap=16):
        cx, cy = t.snap(cx, cy, snap)
        cx, cy = clamp_playable(cx, cy)
        o['position'] = (cell_to_world(cx, cy)[0], cell_to_world(cx, cy)[1], 0.0)

    # ---- 摆放互斥：矿脉/资源点不压基地，彼此也不重叠
    base_list = list(t.base.values()) or [geo.to_cell(lon, lat) for lon, lat, _, _, _ in bt.BASES]
    placed = []          # 已放置的资源点格坐标（矿/油井/观察哨/车库）

    def far_from_bases(cx, cy, clearance):
        return all(math.hypot(cx - bx, cy - by) >= clearance for bx, by in base_list)

    def far_from_placed(cx, cy, spacing):
        return all(math.hypot(cx - px, cy - py) >= spacing for px, py in placed)

    def place_resource(o, cx, cy, base_clear, spacing, max_search=45):
        """在 (cx,cy) 附近找一格：可通行、离所有出生点 >= base_clear、离已放置资源 >= spacing。"""
        cx, cy = clamp_cell(cx), clamp_cell(cy)
        best, bd = None, 1e18
        for r in range(0, max_search + 1):
            for dy in range(-r, r + 1):
                for dx in range(-r, r + 1):
                    if max(abs(dx), abs(dy)) != r:
                        continue
                    x, y = cx + dx, cy + dy
                    if not (0 <= x < bt.GRID and 0 <= y < bt.GRID):
                        continue
                    if not t.passable[y, x]:
                        continue
                    if not far_from_bases(x, y, base_clear):
                        continue
                    if not far_from_placed(x, y, spacing):
                        continue
                    d = dx * dx + dy * dy
                    if d < bd:
                        bd, best = d, (x, y)
            if best is not None:
                break
        if best is None:
            best = t.snap(cx, cy, max_search)
        placed.append(best)
        set_cell(o, best[0], best[1], 0)
        return best

    by_type = {}
    for o in objs:
        by_type.setdefault(o['type_name'], []).append(o)

    def ensure_count(type_name, wanted):
        """按需克隆模板对象（复制属性，重写 uniqueID），保证类型数量足够。"""
        have = by_type.get(type_name, [])
        if not have or len(have) >= wanted:
            return have
        proto = have[0]
        n = len(have)
        while len(have) < wanted:
            clone = dict(proto)
            clone['props'] = [[k, ty, v] for k, ty, v in proto['props']]
            for pr in clone['props']:
                if pr[0] == 'uniqueID':
                    pr[2] = '%s %d' % (type_name, 100 + n)
            have.append(clone)
            objs.append(clone)
            n += 1
        by_type[type_name] = have
        print('  [补齐] %s: %d -> %d 个' % (type_name, len(have) - (wanted - len(have)), wanted))
        return have

    # ---- waypoints: human slots first, then the fixed (imported) positions
    wp = {}
    for o in objs:
        if 'Waypoint' not in o['type_name']:
            continue
        d = props_dict(o['props'])
        wp[d.get('waypointName', '')] = o
    plan = waypoint_plan()
    wp_cell = {}
    faces = list(bt.T.get('base_face') or [])
    face_ang = {}
    for i, (name, (lon, lat, city, team, kind)) in enumerate(plan):
        o = wp.get(name)
        if o is None:
            raise SystemExit('模板缺少 waypoint: ' + name)
        # 用 terrain_gen 解析好的出生点格坐标（与铺平/矿脉一致）
        cell = t.base.get(city) or t.geo.to_cell(lon, lat)
        wp_cell[name] = cell
        set_cell(o, cell[0], cell[1], 0)
        if i < len(faces):
            fx, fy = float(faces[i][0]), float(faces[i][1])
            dxf = fx - float(cell[0])
            dyf = fy - float(cell[1])
            if dxf or dyf:
                # 内部帧 y 向南。角度翻到游戏帧，和矿口同一套。
                ang = math.atan2(-dyf, dxf)
                o['angle'] = ang
                face_ang[name] = ang
                print('  [出生] %s 朝向斜坡 (%.0f,%.0f)' % (city, fx, fy))

    # 模板 p8 的基地是盟军。每张图都改成帝国，再摆到固定位旁边。
    src, dst = 'AlliedConstructionYard', 'JapanConstructionYard'
    found = by_type.get(src, [])
    if not found:
        raise SystemExit('模板里没有要改成帝国的基地：%s' % src)
    for o in found:
        o['type_name'] = dst
        for pr in o['props']:
            if pr[0] == 'uniqueID' and isinstance(pr[2], str) and pr[2].startswith(src):
                pr[2] = dst + pr[2][len(src):]
    by_type[dst] = found
    del by_type[src]
    print('  [基地] %s -> %s' % (src, dst))

    # 每个固定 AI 的基地改一个脚本能找到的名字，科技脚本都要挂上。
    # 水上的另外强制造船厂和空军。
    sea_yards = []
    ai_at = {}
    if getattr(cfg, 'FORCE_SEA_AI', False):
        wz = float(getattr(cfg, 'WATER_Z', 200.0))
        for wp, base in waypoint_plan():
            if base[4] != 'ai':
                continue
            cell = t.base.get(base[2])
            if cell is None:
                continue
            zz = float(t.z[int(cell[1]), int(cell[0])])
            ai_at[wp] = (int(wp.split('_')[1]), base[2], zz, zz < wz)

    # the two AI construction yards sit right next to their fixed waypoints
    for tname, wp_name, dx, dy, snap in _FIXED_YARDS:
        placed_yards = by_type.get(tname, [])
        if not placed_yards:
            raise SystemExit('没有可摆放的基地：%s' % tname)
        for o in placed_yards:
            bx, by = wp_cell.get(wp_name) or geo.to_cell(*WAYPOINT_LAYOUT[wp_name])
            set_cell(o, bx + dx, by + dy, snap)
            if wp_name in face_ang:
                o['angle'] = face_ang[wp_name]
            hit = ai_at.get(wp_name)
            if not hit:
                continue
            slot, city, zz, water = hit
            uid = 'sea-mcv-%d' % slot
            owner = ''
            for pr in o['props']:
                if pr[0] == 'uniqueID':
                    pr[2] = uid
                elif pr[0] == 'originalOwner' and isinstance(pr[2], str):
                    owner = pr[2]
            if '/' not in owner:
                raise SystemExit('固定 AI 基地没有队伍：%s' % wp_name)
            sea_yards.append({
                'slot': slot, 'city': city, 'mcv': uid, 'z': zz, 'water': water,
                'player': owner.split('/')[0], 'team': owner,
            })
            where = '水上' if water else '陆地'
            print('  [科技] %s 在%s（高度 %.0f），基地 %s，队伍 %s' % (
                city, where, zz, uid, owner))

    # ---- resources: 3 ore nodes per base + contested buffer + river crossings
    #      矿脉与基地、与彼此都保持最小间距（配置 ORE_BASE_CLEARANCE / ORE_MIN_SPACING）
    ore_slots = list(t.ore_cells) or list(bt.ore_slots(geo, base_cells=t.base)[0])
    ore = ensure_count('OreNode', len(ore_slots))
    # 模板自带的矿物件往往比我们需要的多：多余的必须**删除**，
    # 否则会被"撒到外圈"，绕过所有间距/布局约束（实测多出 22 个、矿间只剩 9 格）
    if len(ore) > len(ore_slots):
        for _o in ore[len(ore_slots):]:
            objs.remove(_o)
        print('  [矿脉] 删除模板多余的 %d 个矿物件（保留 %d 个）' % (len(ore) - len(ore_slots), len(ore_slots)))
        ore = ore[:len(ore_slots)]
        by_type['OreNode'] = ore
    base_cells_list = [t.base.get(name) or geo.to_cell(lon, lat) for lon, lat, name, _, _ in bt.BASES]
    ore_per_base = max(1, len(bt.BASE_ORE_OFFSETS))   # 每个基地的矿位数（= BASE_ORE_OFFSETS 条数）

    DIRS8 = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)]

    def openness(cx, cy):
        """8 个方向的开阔度：各方向走到不可通行/明显上坡为止的格数。"""
        out = []
        for dx, dy in DIRS8:
            dist = 0.0
            for step in range(1, 26):
                x, y = cx + dx * step, cy + dy * step
                if not (0 <= x < bt.GRID and 0 <= y < bt.GRID) or not t.passable[int(y), int(x)]:
                    break
                if t.z[int(y), int(x)] - t.z[int(cy), int(cx)] > 8.0:
                    break
                dist = float(step)
            out.append(dist)
        return out

    def snap_diag(a):
        """矿脉朝向只能取 45/135/225/315 四个斜向（RA3 矿脉模型只支持斜向，不支持 0/90/180/270）。"""
        return round((a - math.pi / 4) / (math.pi / 2)) * (math.pi / 2) + math.pi / 4

    def game_diag(dx, dy):
        """内部帧方向（y 向南）转到游戏角度（y 向北）。位置已经翻过 y，角度不翻的话矿口会朝反。"""
        return snap_diag(math.atan2(-dy, dx))

    # 每个基地只让近矿环里最近的 2 个矿朝基地，其余保持地形阶段的开阔朝向。
    _near_r = float(bt.T.get('ore_near_max_r', 62.0))
    _buckets = {}
    for _i, (_cx, _cy) in enumerate(ore_slots):
        _bi, _bd = 0, 1e18
        for _j, _bc in enumerate(base_cells_list):
            if _bc is None:
                continue
            _d = math.hypot(_bc[0] - _cx, _bc[1] - _cy)
            if _d < _bd:
                _bi, _bd = _j, _d
        _buckets.setdefault(_bi, []).append((_bd, _i))
    _face_base = set()
    for _items in _buckets.values():
        _items.sort()
        _n = 0
        for _bd, _i in _items:
            if _bd <= _near_r and _n < 2:
                _face_base.add(_i)
                _n += 1

    for i, o in enumerate(ore):
        if i < len(ore_slots):
            cx, cy = ore_slots[i]
        else:      # leftover ore nodes -> spread on the outer ring
            ang = i * 2.399
            cx = bt.GRID / 2 + math.cos(ang) * ORE_LEFT_RING
            cy = bt.GRID / 2 + math.sin(ang) * ORE_LEFT_RING
        if t.ore_cells:
            set_cell(o, cx, cy, 0)            # 已由地形阶段解算并铺平
        else:
            place_resource(o, cx, cy, ORE_BASE_CLEARANCE, ORE_MIN_SPACING)
        # ---- 朝向：每个基地只让最近的 2 个矿朝基地。更远的矿用地形阶段的开阔朝向。
        _nb, _nd = None, 1e18
        for _bc in base_cells_list:
            _d = math.hypot(_bc[0] - cx, _bc[1] - cy)
            if _d < _nd:
                _nd, _nb = _d, _bc
        if i in _face_base and _nb is not None:
            o['angle'] = game_diag(_nb[0] - cx, _nb[1] - cy)               # 矿口朝基地（游戏坐标）
        elif i < len(t.ore_angles):
            o['angle'] = game_diag(math.cos(float(t.ore_angles[i])),
                                   math.sin(float(t.ore_angles[i])))       # 地形阶段角度，同样翻到游戏坐标
        else:
            opens = openness(cx, cy)
            k = opens.index(max(opens))
            dx, dy = DIRS8[k]
            o['angle'] = game_diag(dx, dy)                                 # 朝向最开阔方向
        # 矿是优先级最高的资源：把它登记进 placed，后续资源/装饰物才会主动让开矿区
        placed.append((round(cx, 3), round(cy, 3)))

    # ---- oil derricks: 每个玩家 2 个（贴着基地的建造范围内），剩余的放中路争夺区
    slots = list(t.derrick_cells) or list(bt.derrick_slots(geo, base_cells=t.base))
    derricks = ensure_count('OilDerrick', len(slots))
    for i, o in enumerate(derricks):
        if i < len(slots):
            cx, cy = slots[i]
            if t.derrick_cells:
                set_cell(o, cx, cy, 0)
            else:
                place_resource(o, cx, cy, RESOURCE_PAD_CLEARANCE, RESOURCE_MIN_SPACING)
        elif DERRICK_POS:
            lon, lat = DERRICK_POS[(i - len(slots)) % len(DERRICK_POS)]
            cx, cy = geo.to_cell(lon, lat)
            j = ((i - len(slots)) // len(DERRICK_POS)) * 1.3
            place_resource(o, cx + j * 3.2, cy + j * 2.7,
                           RESOURCE_PAD_CLEARANCE, RESOURCE_MIN_SPACING)
        else:
            print('  [油井] 多出来的油井没有经纬度兜底，留在模板原位')

    posts = by_type.get('ObservationPostTechStructure', [])
    p_pos = POST_POS
    drop_posts = []
    for i, o in enumerate(posts):
        if i < len(t.post_cells):
            set_cell(o, *t.post_cells[i], 0)
            continue
        # 模板自带的观察哨多于配置数量：多出来的直接从对象表里去掉
        # （例如只在地图中心放 1~2 个雷达/哨站）
        drop_posts.append(o)
    if drop_posts:
        for o in drop_posts:
            objs.remove(o)
            posts.remove(o)
        print('  [哨站] 移除多余的 %d 个观察哨（只保留地图中心的 %d 个）' % (
            len(drop_posts), len(t.post_cells)))

    # ---- 中路科技建筑：车库 / 机场 / 精兵学院 / 医院（名字经游戏数据核实）
    proto = by_type.get('GarageTechStructure', [])
    if not proto:
        print('  [警告] 模板里没有 GarageTechStructure，科技建筑跳过')
    else:
        # 车库：模板自带 1 个，克隆补到车库槽位数
        garages = ensure_count('GarageTechStructure', len(t.garage_cells))
        for i, o in enumerate(garages):
            if i < len(t.garage_cells):
                set_cell(o, *t.garage_cells[i], 0)
        # 机场/精兵学院/医院：克隆车库的属性、改成对应类型
        for tname, cells in (('AirportTechStructure', t.airport_cells),
                             ('VeterancyTechStructure', t.academy_cells),
                             ('HospitalTechStructure', t.hospital_cells)):
            for cx, cy in cells:
                o = dict(proto[0])
                o['type_name'] = tname
                o['props'] = [[k, ty, v] for k, ty, v in proto[0]['props']]
                for pr in o['props']:
                    if pr[0] == 'uniqueID':
                        pr[2] = '%s %d' % (tname, len(objs))
                objs.append(o)
                by_type.setdefault(tname, []).append(o)
                set_cell(o, cx, cy, 0)

        for cx, cy in t.shipyard_cells:
            o = dict(proto[0])
            o['type_name'] = 'AlliedNavalYard'
            o['props'] = [[k, ty, v] for k, ty, v in proto[0]['props']]
            for pr in o['props']:
                if pr[0] == 'uniqueID':
                    pr[2] = 'AlliedNavalYard %d' % len(objs)
                elif pr[0] == 'originalOwner':
                    pr[2] = 'PlyrCivilian/teamPlyrCivilian'
            objs.append(o)
            by_type.setdefault('AlliedNavalYard', []).append(o)
            set_cell(o, cx, cy, 0)
        if t.shipyard_cells:
            print('  [船厂] 放置 %d 个' % len(t.shipyard_cells))

        # ---- 桥梁：沿垂直河流方向放在两岸，让陆军能过河（对象名 Bridge2a，原版 RA3 桥）
        for i, (cx, cy) in enumerate(t.bridge_cells):
            o = dict(proto[0])
            o['type_name'] = BRIDGE_TYPE
            o['props'] = [[k, ty, v] for k, ty, v in proto[0]['props']]
            for pr in o['props']:
                if pr[0] == 'uniqueID':
                    pr[2] = '%s %d' % (BRIDGE_TYPE, i)
                elif pr[0] == 'originalOwner':
                    pr[2] = 'PlyrCivilian/teamPlyrCivilian'
                elif pr[0] == 'objectIndestructible':
                    pr[2] = 1
            o['angle'] = float(t.bridge_angles[i]) if i < len(t.bridge_angles) else 0.0
            objs.append(o)
            by_type.setdefault(BRIDGE_TYPE, []).append(o)
            set_cell(o, cx, cy, 0)
        if t.bridge_cells:
            print('  [桥梁] 放置 %d 个桥段 (%s)' % (len(t.bridge_cells), BRIDGE_TYPE))

    # 装饰在纹理之后另放，见 place_decorations。

    # ---- 城市房屋。CITY_CLUSTER 时用地形阶段算好的落点：单独一片一栋，挨着的一片两栋。
    #      没开这个开关的图仍是每座城市 1~2 栋。
    house_types = list(getattr(cfg, 'CITY_HOUSE_TYPES', ()) or ())
    _pl = (by_type.get(DECOR_TYPES[0]) if DECOR_TYPES else None) or by_type.get('EI_Structure_01') or []
    proto_h = _pl[0] if _pl else None
    seated = set(bt._seat_lookup(geo))
    city_list = [(n, cx, cy) for n, cx, cy in bt.city_centers(geo) if n in seated]
    _cluster_houses = bool(getattr(cfg, 'CITY_CLUSTER', False))
    if _cluster_houses and proto_h:
        small = list(getattr(cfg, 'CITY_HOUSE_SMALL', ()) or ()) or house_types[:2] or ['ST_Building01']
        made = 0
        for i, (cx, cy) in enumerate(t.house_cells):
            tname = small[(t.house_kinds[i] if i < len(t.house_kinds) else i) % len(small)]
            o = dict(proto_h)
            o['type_name'] = tname
            o['props'] = [[p[0], p[1], p[2]] for p in proto_h['props']]
            for pr in o['props']:
                if pr[0] == 'uniqueID':
                    pr[2] = '%s %d' % (tname, 600 + made)
            o['angle'] = 0.0
            set_cell(o, cx, cy, 0)
            objs.append(o)
            by_type.setdefault(tname, []).append(o)
            placed.append((round(float(cx), 3), round(float(cy), 3)))
            made += 1
        print('  [城市] 按城市区域放置 %d 栋小房子' % made)
    elif house_types and proto_h and city_list:
        made = 0
        for ci, (cn, ccx, ccy) in enumerate(city_list):
            for j in range(1 + (ci % 2)):                    # 1~2 个
                tname = house_types[(ci + j) % len(house_types)]
                a = math.radians(35.0 + 90.0 * j)
                cx = ccx + math.cos(a) * (9.0 + 7.0 * j)
                cy = ccy + math.sin(a) * (9.0 + 7.0 * j)
                o = dict(proto_h)
                o['type_name'] = tname
                o['props'] = [[p[0], p[1], p[2]] for p in proto_h['props']]
                for pr in o['props']:
                    if pr[0] == 'uniqueID':
                        pr[2] = '%s %d' % (tname, 600 + made)
                o['angle'] = 0.0
                set_cell(o, cx, cy, 0)
                objs.append(o)
                by_type.setdefault(tname, []).append(o)
                made += 1
        print('  [城市] 放置 %d 个城市房屋（%d 座城市，每城 1~2 个）' % (made, len(city_list)))

    # ---- 模板自带的遗留物体（示例建筑等）若压在矿区建造范围内，直接删除：
    #      矿是优先级最高的资源，任何东西都不能占矿区（实测模板里有 SovietConstructionYard 压矿）
    ore_xy = [(float(c[0]), float(c[1])) for c in (t.ore_cells or [])]
    if ore_xy:
        removed = 0
        for o in list(objs):
            tn = o['type_name']
            if tn == 'OreNode' or 'Waypoint' in tn:
                continue
            ox = o['position'][0] / 10.0 + bt.BORDER
            oy = world_to_cell_y(o['position'][1])
            if any(math.hypot(ox - gx, oy - gy) < ORE_CLEAR_RADIUS + 3.0 for gx, gy in ore_xy):
                objs.remove(o)
                removed += 1
                print('  [矿区清理] 删除压矿区的遗留物体 %s @格(%d,%d)' % (tn, round(ox), round(oy)))
        if removed:
            print('  [矿区清理] 共删除 %d 个模板遗留物体' % removed)
    # 配方点名的遗留房子。没写这项的图保持模板位置。
    # 每条 (类型, from_x, from_y, to_x, to_y)，from 是现在的落点，容差 6 格。
    for item in getattr(cfg, 'OBJECT_PLACE_AT', ()) or ():
        tname = item[0]
        fx, fy, tx, ty = (float(item[1]), float(item[2]), float(item[3]), float(item[4]))
        best, bd = None, 1e9
        for o in objs:
            if o['type_name'] != tname:
                continue
            ox = o['position'][0] / bt.CELL + bt.BORDER
            oy = world_to_cell_y(o['position'][1])
            d = math.hypot(ox - fx, oy - fy)
            if d < bd:
                bd, best = d, o
        if best is None or bd > 6.0:
            print('  [房子] %s 在 (%.0f,%.0f) 附近没有' % (tname, fx, fy))
            continue
        best['position'] = (cell_to_world(tx, ty)[0], cell_to_world(tx, ty)[1], 0.0)
        print('  [房子] %s (%.0f,%.0f) 挪到 (%d,%d)' % (
            tname, fx, fy, int(round(tx)), int(round(ty))))
    if sea_yards:
        live = set()
        for o in objs:
            for pr in o['props']:
                if pr[0] == 'uniqueID' and isinstance(pr[2], str):
                    live.add(pr[2])
        gone = [s for s in sea_yards if s['mcv'] not in live]
        if gone:
            raise SystemExit('固定 AI 的基地被清掉了：%s' % ', '.join(s['city'] for s in gone))
    return objs, sea_yards


def _drop_stale_assetlist(tmpl):
    """删掉从模板继承来的 AssetList。

    ⚠ 2026-10-16 实机定位（症状："地编里能看见树/石头，游戏里看不见，地编打开再保存进游戏又看见了"）：
    模板的 AssetList 只有 20 条，是为模板那 15 种对象准备的。我们在它基础上新加了对象
    （树/石头/园石…），但 AssetList 没跟着重建 —— 游戏按这份过期的清单去准备资源，
    清单外的对象类型就直接不生成，所以装饰物进不了游戏。
    地编（WorldBuilder）打开再保存时会按地图内容重建 AssetList（T2.1.9 变成 37 条），
    所以"地编存一次就好了"。

    本流水线不做资源预登记：直接删掉这个 chunk，游戏会退回到"按需加载"，
    所有对象类型都能正常生成。实测 A_TEST1_NOASSETLIST / A_TEST4_WBALL 都能看见装饰；
    地编自己的自动存档也从不写 AssetList。
    如将来要恢复，把它做成配方开关即可。
    """
    before = len(tmpl.chunks)
    tmpl.chunks = [c for c in tmpl.chunks if c.name != 'AssetList']
    if len(tmpl.chunks) != before:
        print('  [AssetList] 已删除模板继承的过期资源清单（否则新加的装饰不会被游戏加载）')


# ------------------------------------------------------------------ worldinfo / art
def build_worldinfo(template_chunk, table):
    r = R(template_chunk.payload)
    props = read_props(r, table)
    tex_tokens = []
    for name, token, nrm in TEX_SET:
        tex_tokens.append(token)
        tex_tokens.append(nrm)
    out = []
    for k, ty, v in props:
        if k == 'terrainTextureStrings':
            v = ';'.join(tex_tokens) + ';'
        elif k == 'mapName':
            v = ''                       # lobby shows the folder name
        elif k == 'mapDescription':
            v = ''
        elif k == 'weather':
            v = 0
        out.append([k, ty, v])
    return out


def write_art_tga(t, path, size=512):
    z = t.z
    land = t.land
    img = np.zeros((bt.GRID, bt.GRID, 3), dtype=np.uint8)
    deep = z < bt.WATER_Z - 40
    shal = (z >= bt.WATER_Z - 40) & (z < bt.WATER_Z)
    img[deep] = (22, 62, 122)
    img[shal] = (46, 104, 172)
    beach = (z >= bt.WATER_Z) & (z < bt.WATER_Z + 4)
    img[beach] = (216, 204, 152)
    landm = z >= bt.WATER_Z + 4
    lv = np.clip((z - (bt.WATER_Z + 4)) / 330.0, 0, 1) ** 0.85
    idx = np.clip((lv * 255).astype(int), 0, 254)
    ramp = np.zeros((256, 3), dtype=np.uint8)
    for i in range(256):
        f = i / 255.0
        if f < 0.42:
            g = f / 0.42; ramp[i] = (58 + 52 * g, 122 + 44 * g, 58 - 12 * g)
        elif f < 0.66:
            g = (f - 0.42) / 0.24; ramp[i] = (110 + 50 * g, 166 - 16 * g, 46 - 18 * g)
        elif f < 0.85:
            g = (f - 0.66) / 0.19; ramp[i] = (160 + 30 * g, 150 - 20 * g, 28 + 22 * g)
        else:
            g = (f - 0.85) / 0.15; ramp[i] = (190 + 55 * g, 130 + 80 * g, 50 + 130 * g)
    img[landm] = ramp[idx[landm]]
    # 装饰要出现在小地图上：层交界岩壁、水陆交界沙滩。
    if getattr(t, 'layer_edge', None) is not None and t.layer_edge.shape == img.shape[:2]:
        img[t.layer_edge] = (128, 120, 108)
    if getattr(t, 'shore_band', None) is not None and t.shore_band.shape == img.shape[:2]:
        img[t.shore_band] = (214, 198, 142)
    # 省外荒地压暗。art_keep_sea 时水面保持原来的蓝色，
    # 否则环岛的外海被压到阈值以下，小地图看不出海，轴向验收也会判反。
    outside = ~land
    if bool(bt.T.get('art_keep_sea')):
        outside = outside & (z >= bt.WATER_Z)
    img[outside] = (img[outside].astype(np.float64) * 0.72).astype(np.uint8)
    # ⚠ 轴向：数据第 0 行 = 最北，与 v11 蓝图/用户标注同向 —— **不再 flipud**
    im = Image.fromarray(img).resize((size, size), Image.LANCZOS)
    arr_img = np.array(im).copy()
    scale = size / float(bt.GRID)
    for x, y, name, _ang in list(getattr(t, 'scenery', None) or []):
        px = int(round(float(x) * scale))
        py = int(round(float(y) * scale))
        col = (34, 110, 46) if ('Tree' in name or 'Bush' in name) else (168, 164, 156)
        y0, y1 = max(0, py - 1), min(size, py + 2)
        x0, x1 = max(0, px - 1), min(size, px + 2)
        arr_img[y0:y1, x0:x1] = col
    # ⚠ art 与**游戏内实际地形朝向**必须同向（T1.8.8 实机锚定）：
    #   游戏内 3D 地形是正确的 = 北在屏幕上、海（南）在屏幕下。
    #   引擎把小地图图片的"图像上方"摆在屏幕上方 => PIL 读回的 art 必须**行 0 = 北**（海在下半）。
    #   TGA（origin=0）自底向上存储，PIL 读回 = flipud(存储序)
    #   => 存储序必须 = flipud(raw)，即**画布写出前要翻一次**。
    #   （T1.8.2 的"不翻"结论是错的：当时拿高度场**数组**行序当屏幕方向——
    #     其实高度场第 0 行 = 世界 y=0 = 屏幕**下**方，显示出来正好再翻一次。）
    arr = arr_img[::-1]
    header = bytes([0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    size & 0xFF, size >> 8, size & 0xFF, size >> 8, 24, 0])
    footer = b'\x00' * 8 + b'TRUEVISION-XFILE.\x00'
    data = arr[:, :, ::-1].tobytes()               # BGR
    open(path, 'wb').write(header + data + footer)


def build_map_str(path):
    """map.str：UTF-8 文本表（Intro:String / Debug:Spectate / Info:x_y），内容来自配方。

    电脑现金的说明也写在这里。显示动作只认表里的键。
    """
    from ai_money import map_str_entries
    txt = ['Intro:String', MAP_STR_INTRO, 'End', '',
           'Debug:Spectate', MAP_STR_DEBUG, 'End', '']
    for k, v in MAP_STR_INFO.items():
        txt += ['Info:' + k, '"%s"' % v, 'End', '']
    txt += map_str_entries()
    open(path, 'w', encoding='utf-8').write('\n'.join(txt))


EAST_ALLY_SCRIPTS = ('DPR Player 5', 'DPR Player 6', 'FAPR Player 5', 'FAPR Player 6')


def _u16_str(data, p):
    n = int.from_bytes(data[p:p + 2], 'little')
    return data[p + 2:p + 2 + n].decode('latin1'), p + 2 + n


def _skip_script_header(data, p, ver):
    _, p = _u16_str(data, p)
    for _ in range(3):
        _, p = _u16_str(data, p)
    p += 6
    if ver >= 2:
        p += 4
    if ver == 5:
        p += 5
    if ver >= 3:
        p += 7
        _, p = _u16_str(data, p)
    if ver >= 4:
        _, p = _u16_str(data, p)
    return p


def patch_east_bonus_alliance(mmap):
    """固定 AI 与东队人类结盟。

    模板里 DPR/FAPR Player N 的条件为真（该位是人类，或与人类友好）时执行
    「PRC Player N Enemy」，为假才执行 Friend。东队是 Player_5 / Player_6，
    把这两条脚本的真分支和假分支对调：人类在东侧时，额外 AI 走 Friend。
    西队 Player_1–4 仍走 Enemy。
    """
    chunk = mmap.get('PlayerScriptsList')
    data = bytearray(chunk.payload)
    table = mmap.table
    action_id = next(i for i, n in table.items() if n == 'ScriptAction')
    false_id = next(i for i, n in table.items() if n == 'ScriptActionFalse')
    swapped = []

    def children(p, end):
        out = []
        while p + 10 <= end:
            ni = int.from_bytes(data[p:p + 4], 'little')
            ver = int.from_bytes(data[p + 4:p + 6], 'little')
            sz = int.from_bytes(data[p + 6:p + 10], 'little')
            body = p + 10
            pend = body + sz
            if sz < 0 or pend > len(data):
                break
            out.append((p, ver, body, pend, table.get(ni, '')))
            p = pend
        return out

    def walk(p, end):
        for pos, ver, body, pend, nm in children(p, end):
            if nm == 'ScriptGroup':
                _, hp = _u16_str(data, body)
                walk(hp + 2, pend)
            elif nm == 'Script':
                name, _ = _u16_str(data, body)
                hp = _skip_script_header(data, body, ver)
                if name in EAST_ALLY_SCRIPTS:
                    kids = children(hp, pend)
                    acts = [k for k in kids if k[4] == 'ScriptAction']
                    falses = [k for k in kids if k[4] == 'ScriptActionFalse']
                    if len(acts) != 1 or len(falses) != 1:
                        raise SystemExit('脚本 %s 的真/假分支不是各一条' % name)
                    data[acts[0][0]:acts[0][0] + 4] = false_id.to_bytes(4, 'little')
                    data[falses[0][0]:falses[0][0] + 4] = action_id.to_bytes(4, 'little')
                    swapped.append(name)
                walk(hp, pend)

    p = 0
    end = len(data)
    while p + 10 <= end:
        ni = int.from_bytes(data[p:p + 4], 'little')
        sz = int.from_bytes(data[p + 6:p + 10], 'little')
        body = p + 10
        pend = body + sz
        if table.get(ni) == 'ScriptList':
            walk(body, pend)
        p = pend
    missing = [n for n in EAST_ALLY_SCRIPTS if n not in swapped]
    if missing:
        raise SystemExit('东队结盟脚本没有改全：%s' % ', '.join(missing))
    chunk.payload = bytes(data)
    print('  [外交] 固定 AI 与东队结盟：%s' % ', '.join(swapped))


def deactivate_named_scripts(mmap, names):
    """把指定脚本或脚本组的“启用”位清掉。不改名字，模板文件也不动。"""
    want = set(names)
    chunk = mmap.get('PlayerScriptsList')
    data = bytearray(chunk.payload)
    table = mmap.table
    hit = []

    def children(p, end):
        out = []
        while p + 10 <= end:
            ni = int.from_bytes(data[p:p + 4], 'little')
            ver = int.from_bytes(data[p + 4:p + 6], 'little')
            sz = int.from_bytes(data[p + 6:p + 10], 'little')
            body = p + 10
            pend = body + sz
            if sz < 0 or pend > len(data):
                break
            out.append((ver, body, pend, table.get(ni, '')))
            p = pend
        return out

    def walk(p, end):
        for ver, body, pend, nm in children(p, end):
            if nm == 'ScriptGroup':
                name, hp = _u16_str(data, body)
                if name in want and data[hp] != 0:
                    data[hp] = 0
                    hit.append(name)
                walk(hp + 2, pend)
            elif nm == 'Script':
                name, _ = _u16_str(data, body)
                hp = _skip_script_header(data, body, ver)
                # 三个注释字符串之后的第一个字节是启用位。
                active_at = body
                n = int.from_bytes(data[active_at:active_at + 2], 'little')
                active_at += 2 + n
                for _ in range(3):
                    n = int.from_bytes(data[active_at:active_at + 2], 'little')
                    active_at += 2 + n
                if name in want and data[active_at] != 0:
                    data[active_at] = 0
                    hit.append(name)
                walk(hp, pend)

    p = 0
    end = len(data)
    while p + 10 <= end:
        ni = int.from_bytes(data[p:p + 4], 'little')
        sz = int.from_bytes(data[p + 6:p + 10], 'little')
        body = p + 10
        pend = body + sz
        if table.get(ni) == 'ScriptList':
            walk(body, pend)
        p = pend
    missing = [n for n in names if n not in hit]
    if missing:
        raise SystemExit('要关掉的脚本没找到：%s' % ', '.join(missing))
    chunk.payload = bytes(data)
    print('  [脚本] 已关闭：%s' % ', '.join(hit))


def remove_named_scripts(mmap, names):
    """从脚本列表里删掉指定脚本或脚本组。模板文件不改。"""
    want = set(names)
    chunk = mmap.get('PlayerScriptsList')
    data = chunk.payload
    table = mmap.table
    found = []

    def rebuild(buf, start, end):
        out = bytearray()
        p = start
        while p + 10 <= end:
            ni = int.from_bytes(buf[p:p + 4], 'little')
            ver = int.from_bytes(buf[p + 4:p + 6], 'little')
            sz = int.from_bytes(buf[p + 6:p + 10], 'little')
            body = p + 10
            pend = body + sz
            if sz < 0 or pend > end:
                raise SystemExit('脚本块越界，不能删除')
            nm = table.get(ni, '')
            if nm == 'Script':
                name, _ = _u16_str(buf, body)
                if name in want:
                    found.append(name)
                    p = pend
                    continue
                child = _skip_script_header(buf, body, ver)
                new_body = buf[body:child] + rebuild(buf, child, pend)
            elif nm == 'ScriptGroup':
                name, after_name = _u16_str(buf, body)
                if name in want:
                    found.append(name)
                    p = pend
                    continue
                child = after_name + 2
                new_body = buf[body:child] + rebuild(buf, child, pend)
            elif nm in ('ScriptList', 'OrCondition'):
                new_body = rebuild(buf, body, pend)
            else:
                new_body = buf[body:pend]
            out += buf[p:p + 6]
            out += len(new_body).to_bytes(4, 'little')
            out += new_body
            p = pend
        if p < end:
            out += buf[p:end]
        return bytes(out)

    new_payload = rebuild(data, 0, len(data))
    missing = [n for n in names if n not in found]
    if missing:
        raise SystemExit('要移除的脚本没找到：%s' % ', '.join(missing))
    from nkl_scripts import _walk
    p = 0
    while p + 10 <= len(new_payload):
        ni = int.from_bytes(new_payload[p:p + 4], 'little')
        sz = int.from_bytes(new_payload[p + 6:p + 10], 'little')
        body = p + 10
        pend = body + sz
        if table.get(ni) == 'ScriptList' and sz:
            _walk(new_payload, body, pend, table, lambda *a: None)
        p = pend
    if p != len(new_payload):
        raise SystemExit('删脚本后列表末尾对不上')
    chunk.payload = new_payload
    print('  [脚本] 已移除：%s' % ', '.join(found))


def _write_sides(sides, key_ids, has_asset_list=True):
    w = W()
    if sides['version'] >= 6:
        w.u8(sides.get('unknown', 0))
    w.i32(sides['num_players'])
    for p in sides['players']:
        write_props(w, p['props'], key_ids)
        items = p['build_list']
        w.u32(len(items))
        for it in items:
            w.s16(it['building_name'])
            w.s16(it['template_name'])
            w.vec3(it['location'])
            w.f32(it['angle'])
            w.u8(it['initially_built'])
            if has_asset_list:
                w.u8(it['unknown1'])
            w.u32(it['num_rebuilds'])
            w.s16(it['script'])
            w.i32(it['health'])
            w.u8(it['whiner'])
            w.u8(it['unsellable'])
            w.u8(it['repairable'])
    return bytes(w.b)


def _write_teams(teams, key_ids):
    w = W()
    w.i32(len(teams))
    for props in teams:
        write_props(w, props, key_ids)
    return bytes(w.b)


def patch_locked_factions(mmap):
    """两个固定 AI 每张图都钉死。不改磁盘上的模板，也不看配方。

    p7 PlyrCivilian 是苏联欧列格，p8 PlyrCreeps 是帝国晋三。
    """
    locks = dict(_FIXED_FACTION)
    pers = dict(_FIXED_PERSONALITY)
    unit_retype = {k: dict(v) for k, v in _FIXED_UNITS.items()}
    key_ids = {name: idx for idx, name in mmap.table.items()}
    if locks or pers:
        chunk = mmap.get('SidesList')
        sides = parse_sides(chunk, mmap.table, has_asset_list=True)
        if sides['consumed'] != len(chunk.payload):
            raise SystemExit('SidesList 没读完，不能改阵营')
        if _write_sides(sides, key_ids) != chunk.payload:
            raise SystemExit('SidesList 回写和原文不一致，停止改阵营')
        if locks:
            hit = []
            for p in sides['players']:
                name = props_dict(p['props']).get('playerName')
                if name not in locks:
                    continue
                found = False
                for pr in p['props']:
                    if pr[0] == 'playerFaction':
                        pr[2] = locks[name]
                        found = True
                if not found:
                    raise SystemExit('玩家 %s 没有 playerFaction' % name)
                hit.append('%s -> %s' % (name, locks[name]))
            missing = [n for n in locks if not any(n in h for h in hit)]
            if missing:
                raise SystemExit('要锁阵营的玩家没找到：%s' % ', '.join(missing))
            print('  [阵营] %s' % ', '.join(hit))
        if pers:
            if 'aiPersonality' not in key_ids:
                raise SystemExit('模板没有 aiPersonality，不能钉指挥官')
            hit = []
            for p in sides['players']:
                name = props_dict(p['props']).get('playerName')
                if name not in pers:
                    continue
                found = False
                for pr in p['props']:
                    if pr[0] == 'aiPersonality':
                        pr[2] = pers[name]
                        found = True
                if not found:
                    p['props'].append(['aiPersonality', 5, pers[name]])
                hit.append('%s -> %s' % (name, pers[name]))
            missing = [n for n in pers if not any(n in h for h in hit)]
            if missing:
                raise SystemExit('要钉指挥官的玩家没找到：%s' % ', '.join(missing))
            print('  [性格] %s' % ', '.join(hit))
        chunk.payload = _write_sides(sides, key_ids)
    if unit_retype:
        chunk = mmap.get('Teams')
        teams = parse_teams(chunk, mmap.table)
        if _write_teams(teams, key_ids) != chunk.payload:
            raise SystemExit('Teams 回写和原文不一致，停止改队伍单位')
        seen = {owner: 0 for owner in unit_retype}
        for props in teams:
            owner = props_dict(props).get('teamOwner')
            repl = unit_retype.get(owner)
            if not repl:
                continue
            for pr in props:
                if isinstance(pr[2], str) and pr[2] in repl:
                    pr[2] = repl[pr[2]]
                    seen[owner] += 1
        missing = [o for o, n in seen.items() if n == 0]
        if missing:
            raise SystemExit('没有改到这些玩家的队伍单位：%s' % ', '.join(missing))
        chunk.payload = _write_teams(teams, key_ids)
        print('  [队伍] %s' % ', '.join(
            '%s %d 处' % (o, n) for o, n in seen.items()))


def _dilate(mask, radius):
    """切比雪夫膨胀。半径 0 原样返回。"""
    out = np.asarray(mask, dtype=bool).copy()
    h, w = out.shape
    src = out
    for _ in range(int(radius)):
        nxt = src.copy()
        nxt[1:, :] |= src[:-1, :]
        nxt[:-1, :] |= src[1:, :]
        nxt[:, 1:] |= src[:, :-1]
        nxt[:, :-1] |= src[:, 1:]
        nxt[1:, 1:] |= src[:-1, :-1]
        nxt[1:, :-1] |= src[:-1, 1:]
        nxt[:-1, 1:] |= src[1:, :-1]
        nxt[:-1, :-1] |= src[1:, 1:]
        src = nxt
    return src


def _stamp_disk(mask, x, y, radius):
    r = int(math.ceil(float(radius)))
    if r < 0:
        return
    h, w = mask.shape
    xi, yi = int(round(x)), int(round(y))
    y0, y1 = max(0, yi - r), min(h, yi + r + 1)
    x0, x1 = max(0, xi - r), min(w, xi + r + 1)
    if y0 >= y1 or x0 >= x1:
        return
    yy = np.arange(y0, y1)[:, None]
    xx = np.arange(x0, x1)[None, :]
    mask[y0:y1, x0:x1] |= (xx - x) ** 2 + (yy - y) ** 2 <= float(radius) ** 2


def _obj_cell(o):
    wx, wy = float(o['position'][0]), float(o['position'][1])
    cx = wx / float(bt.CELL) + bt.BORDER
    cy = (bt.GRID - 1 - bt.BORDER) - wy / float(bt.CELL)
    return cx, cy


def _layer_step(z):
    """陆地格到相邻陆地的最大高度差。水和斜坡上的缓变不算层界。"""
    land = z >= bt.WATER_Z
    acc = np.zeros(z.shape, dtype=np.float64)
    step = np.abs(z[:, 1:] - z[:, :-1])
    both = land[:, 1:] & land[:, :-1]
    step = np.where(both, step, 0.0)
    acc[:, 1:] = np.maximum(acc[:, 1:], step)
    acc[:, :-1] = np.maximum(acc[:, :-1], step)
    step = np.abs(z[1:, :] - z[:-1, :])
    both = land[1:, :] & land[:-1, :]
    step = np.where(both, step, 0.0)
    acc[1:, :] = np.maximum(acc[1:, :], step)
    acc[:-1, :] = np.maximum(acc[:-1, :], step)
    return acc


def _layer_lower(z):
    """层界低的一侧：相邻陆地高出至少一层的格子。"""
    land = z >= bt.WATER_Z
    low = np.zeros(z.shape, dtype=bool)
    dz = z[:, 1:] - z[:, :-1]
    both = land[:, 1:] & land[:, :-1]
    low[:, :-1] |= both & (dz >= 40.0)
    low[:, 1:] |= both & (dz <= -40.0)
    dz = z[1:, :] - z[:-1, :]
    both = land[1:, :] & land[:-1, :]
    low[:-1, :] |= both & (dz >= 40.0)
    low[1:, :] |= both & (dz <= -40.0)
    return low


def paint_boundaries(t, chosen, z):
    """层交界铺岩壁，水陆交界铺沙滩。斜坡车道保持原来的土路和碎石。"""
    land = z >= bt.WATER_Z
    ramp = np.asarray(getattr(t, 'ramp_tex_mask', None), dtype=bool)
    if ramp.shape != z.shape:
        ramp = np.zeros(z.shape, dtype=bool)
    edge = _dilate(_layer_step(z) >= 40.0, 1) & land & ~ramp
    shore = _dilate(z < bt.WATER_Z, 4) & land & ~edge & ~ramp
    rock = TEX_INDEX.get('Rock_Yucatan01', TEX_INDEX.get('Rock_Yucatan02'))
    sand = TEX_INDEX.get('Sand_Hawaii02')
    if rock is not None and edge.any():
        chosen = chosen.copy()
        chosen[edge] = rock
    if sand is not None and shore.any():
        if rock is None or not edge.any():
            chosen = chosen.copy()
        chosen[shore] = sand
    print('  [装饰] 层交界 %d 格，水陆交界 %d 格' % (int(edge.sum()), int(shore.sum())))
    t.layer_edge = edge
    t.shore_band = shore
    return chosen


def decorate_map(t, chosen, z):
    """纹理之后的装饰：层交界、水陆交界，再定树和石头的格子。"""
    t.scenery = []
    t.decor_block = None
    chosen = paint_boundaries(t, chosen, z)
    if not bool(getattr(cfg, 'TERRAIN_ONLY', False)):
        plan_scenery(t, z)
    return chosen


_TREE_TYPES = (
    'CC_Tree01', 'CC_Tree02', 'CC_Tree03',
    'HB_Tree03', 'HB_Tree07', 'HB_Tree10',
    'YU_Bush01', 'YU_Bush03',
)
_ROCK_TYPES = (
    'HA_GardenRock01', 'OK_GardenRock01',
)


def _scenery_clearance(type_name):
    if type_name == 'OreNode':
        return float(ORE_CLEAR_RADIUS) + 3.0
    if type_name == 'OilDerrick':
        return max(float(RESOURCE_MIN_SPACING), 14.0) + 2.0
    if 'ConstructionYard' in type_name or type_name.endswith('Yard'):
        return float(getattr(cfg, 'PAD_CLEARANCE', 26.0))
    if type_name == 'AirportTechStructure':
        return 24.0
    if type_name in ('GarageTechStructure', 'AlliedNavalYard'):
        return 20.0
    if 'Bridge' in type_name:
        return 16.0
    if type_name in ('HospitalTechStructure', 'VeterancyTechStructure',
                     'ObservationPostTechStructure'):
        return 16.0
    return 14.0


def _sample_props(mask, kinds, gap, cap, occ, rng):
    ys, xs = np.where(mask & ~occ)
    if ys.size == 0 or cap <= 0:
        return []
    order = rng.permutation(ys.size)
    out = []
    h, w = occ.shape
    r = int(gap)
    for i in order:
        y, x = int(ys[i]), int(xs[i])
        if occ[y, x]:
            continue
        if y < 1 or x < 1 or y + 1 >= h or x + 1 >= w:
            continue
        name = kinds[len(out) % len(kinds)]
        ang = float(rng.uniform(0.0, math.pi * 2.0))
        out.append((x, y, name, ang))
        _stamp_disk(occ, x, y, r)
        if len(out) >= cap:
            break
    return out


def plan_scenery(t, z):
    """树沿层界、岸边和空地放。小石头只放在层界低的一侧。躲开出生点、资源、建筑和斜坡车道。

    每个物体脚下 3×3 标成不可建造。物体几何本身也会挡住建筑。
    """
    proto_ok = False
    for o in list(getattr(t, 'placed_objs', None) or []):
        if o['type_name'] in ('EI_Structure_01', 'GarageTechStructure'):
            proto_ok = True
            break
    if not proto_ok:
        print('  [装饰] 模板没有可克隆的物体，树和石头不放')
        return
    H, W = z.shape
    land = z >= bt.WATER_Z
    forbid = ~land | (t.d_edge < 4.0)
    ramp = np.asarray(getattr(t, 'ramp_tex_mask', None), dtype=bool)
    if ramp.shape != z.shape:
        ramp = np.zeros(z.shape, dtype=bool)
    forbid |= _dilate(ramp, 1)
    points = []
    pad = float(getattr(cfg, 'PAD_CLEARANCE', 26.0))
    for _nm, (bx, by) in (t.base or {}).items():
        points.append((float(bx), float(by), pad + 2.0))
    groups = (
        (t.ore_cells, float(ORE_CLEAR_RADIUS) + 3.0),
        (t.derrick_cells, max(float(RESOURCE_MIN_SPACING), 14.0) + 2.0),
        (t.post_cells, 16.0),
        (t.garage_cells, 20.0),
        (t.airport_cells, 24.0),
        (t.hospital_cells, 16.0),
        (t.academy_cells, 16.0),
        (t.shipyard_cells, 20.0),
        (t.house_cells, 14.0),
        (t.bridge_cells, 16.0),
    )
    for cells, radius in groups:
        for c in (cells or []):
            points.append((float(c[0]), float(c[1]), radius))
    for o in list(getattr(t, 'placed_objs', None) or []):
        if 'Waypoint' in o['type_name']:
            continue
        cx, cy = _obj_cell(o)
        points.append((cx, cy, _scenery_clearance(o['type_name'])))
    for x, y, radius in points:
        _stamp_disk(forbid, x, y, radius)
    edge = np.asarray(getattr(t, 'layer_edge', None), dtype=bool)
    if edge.shape != z.shape:
        edge = np.zeros(z.shape, dtype=bool)
    shore = _dilate(z < bt.WATER_Z, 8) & land & ~_dilate(z < bt.WATER_Z, 3)
    foot = np.ones((3, 3), dtype=bool)
    # 脚下 3×3 都要是陆地，避免半截落在水里。
    land3 = np.zeros(z.shape, dtype=bool)
    land3[1:-1, 1:-1] = (
        land[:-2, :-2] & land[:-2, 1:-1] & land[:-2, 2:]
        & land[1:-1, :-2] & land[1:-1, 1:-1] & land[1:-1, 2:]
        & land[2:, :-2] & land[2:, 1:-1] & land[2:, 2:]
    )
    open_land = np.asarray(t.passable, dtype=bool) & land3 & ~forbid
    rng = np.random.RandomState(7 + (sum(ord(c) for c in P.map_id) % 9973))
    occ = forbid.copy()
    lower = _layer_lower(z) & land & ~ramp
    upper = (_layer_step(z) >= 40.0) & land & ~lower
    rock_band = _dilate(lower, 2) & land3 & ~upper & ~ramp & ~forbid
    spots = []
    spots += _sample_props(rock_band, _ROCK_TYPES, 9, 140, occ, rng)
    spots += _sample_props(edge & land3 & ~forbid, _TREE_TYPES, 11, 100, occ, rng)
    spots += _sample_props(shore & land3 & ~forbid, _TREE_TYPES, 11, 90, occ, rng)
    inland = open_land & ~_dilate(edge, 8) & ~_dilate(z < bt.WATER_Z, 10)
    spots += _sample_props(inland, _TREE_TYPES, 30, 48, occ, rng)
    block = np.zeros((H, W), dtype=bool)
    kept = []
    for x, y, name, ang in spots:
        if forbid[y, x]:
            continue
        kept.append((x, y, name, ang))
        block[y - 1:y + 2, x - 1:x + 2] |= foot
    # 资源和建筑盘上保持可建造。
    block &= ~forbid
    t.scenery = kept
    t.decor_block = block
    n_tree = sum(1 for _x, _y, name, _a in kept if 'Tree' in name or 'Bush' in name)
    n_rock = len(kept) - n_tree
    print('  [装饰] 树 %d，石头 %d，不可建造 %d 格' % (n_tree, n_rock, int(block.sum())))


def reposition_template_decor(t, objs):
    """模板里已有的装饰挪开资源和出生点。树和石头在这之后另放。"""
    if not DECOR_TYPES or not DECOR_POS:
        return objs
    geo = t.geo
    by_type = {}
    for o in objs:
        by_type.setdefault(o['type_name'], []).append(o)
    occupied = []
    for c in (t.base or {}).values():
        occupied.append((float(c[0]), float(c[1])))
    for group in (t.ore_cells, t.derrick_cells, t.post_cells, t.garage_cells,
                  t.airport_cells, t.hospital_cells, t.academy_cells,
                  t.shipyard_cells, t.house_cells, t.bridge_cells):
        for c in (group or []):
            occupied.append((float(c[0]), float(c[1])))
    ore_set = set((round(float(c[0]), 3), round(float(c[1]), 3)) for c in (t.ore_cells or []))
    lo = bt.BORDER + 2.0
    hi = bt.BORDER + bt.PLAY - 2.0
    base_cells_ll = list(t.base.values()) or [geo.to_cell(lon, lat) for lon, lat, _, _, _ in bt.BASES]
    for tname in DECOR_TYPES:
        for i, o in enumerate(by_type.get(tname, [])):
            lon, lat = DECOR_POS[i % len(DECOR_POS)]
            cx, cy = geo.to_cell(lon, lat)
            cx += (1.2 + i * 0.8) * 2.6
            cy += (1.0 + i * 0.7) * 2.2
            for bx, by in base_cells_ll:
                dx, dy = cx - bx, cy - by
                d = math.hypot(dx, dy)
                if d < PAD_CLEARANCE:
                    f = ((PAD_CLEARANCE + 0.5) / d) if d > 0.01 else 1.0
                    cx, cy = bx + dx * f, by + dy * f
            cx, cy = t.snap(cx, cy, 26)
            for _pass in range(8):
                moved = False
                for px, py in occupied:
                    dx, dy = cx - px, cy - py
                    d = math.hypot(dx, dy)
                    key = (round(float(px), 3), round(float(py), 3))
                    need = ORE_CLEAR_RADIUS + 6.0 if key in ore_set else DECOR_RESOURCE_CLEARANCE
                    if d < need:
                        f = ((need + 0.5) / d) if d > 0.01 else 1.0
                        cx, cy = px + dx * f, py + dy * f
                        moved = True
                if not moved:
                    break
            cx = min(max(cx, lo), hi)
            cy = min(max(cy, lo), hi)
            o['position'] = (cell_to_world(cx, cy)[0], cell_to_world(cx, cy)[1], 0.0)
    return objs


def place_decorations(t, objs):
    """把已经定好格子的树和石头放进物体表。脚下格子在纹理打包时标成不可建造。"""
    spots = list(getattr(t, 'scenery', None) or [])
    if not spots:
        return objs
    proto = None
    for o in objs:
        if o['type_name'] == 'EI_Structure_01':
            proto = o
            break
    if proto is None:
        for o in objs:
            if o['type_name'] == 'GarageTechStructure':
                proto = o
                break
    if proto is None:
        print('  [装饰] 模板没有可克隆的物体，树和石头不放')
        t.decor_block = None
        return objs
    for i, (cx, cy, tname, ang) in enumerate(spots):
        o = dict(proto)
        o['type_name'] = tname
        o['angle'] = float(ang)
        o['props'] = [[k, ty, v] for k, ty, v in proto['props']]
        uid = '%s %d' % (tname, 1000 + i)
        found = False
        for pr in o['props']:
            if pr[0] == 'uniqueID':
                pr[2] = uid
                found = True
            elif pr[0] == 'originalOwner':
                # 矿和油井同属 PlyrNeutral，开局会留下。
                # SkirmishCivilian 不是对局里的玩家，物体进游戏后会被清掉。
                pr[2] = 'PlyrNeutral/teamPlyrNeutral'
            elif pr[0] == 'objectIndestructible':
                pr[2] = 0
            elif pr[0] == 'objectUnsellable':
                pr[2] = 0
            elif pr[0] == 'objectRecruitableAI':
                pr[2] = 1
            elif pr[0] == 'objectTargetable':
                pr[2] = 0
        if not found:
            o['props'].append(['uniqueID', 3, uid])
        # 树按原版可见大小随机缩放。园石保持接近默认大小。
        if 'Rock' in tname:
            scale = 0.92 + (i % 4) * 0.06
        else:
            scale = 1.45 + (i % 8) * 0.12
        o['props'].append(['objectPrototypeScale', 2, scale])
        o['position'] = (cell_to_world(cx, cy)[0], cell_to_world(cx, cy)[1], 0.0)
        objs.append(o)
    return objs


# ------------------------------------------------------------------ main
def main():
    if not TEMPLATE or not os.path.exists(TEMPLATE):
        raise SystemExit('缺少模板骨架（TEMPLATE）：%s' % TEMPLATE)
    print('map recipe : %s  ->  %s' % (P.map_id, MAPNAME))

    t = Terrain()
    tmpl = MapFile.load(TEMPLATE)
    print('template chunks:', len(tmpl.chunks))

    # 模板自身尺寸可能与目标地图不同：先用模板的高度图尺寸解析模板的纹理块（要在替换 payload 之前）
    tmpl_hm = parse_heightmap(tmpl.get('HeightMapData'))
    tmpl_bt = parse_blendtiles(tmpl.get('BlendTileData'), tmpl_hm['width'], tmpl_hm['height'], tmpl.table)
    print('template dims:', tmpl_hm['width'], 'x', tmpl_hm['height'])

    tmpl_objects = parse_objects(tmpl.get('ObjectsList'), tmpl.table)
    # ---- 纯净地形模式：不摆任何对象（出生点 waypoint / 矿 / 油井 / 桥 / 装饰全都不放）。
    #      模板自带的 88 个对象也一并清空，只留地形/纹理/水面/环境。
    sea_yards = []
    if bool(getattr(cfg, 'TERRAIN_ONLY', False)):
        objs = []
        print('  [对象] 纯净地形模式：清空全部 %d 个对象（含模板自带）' % len(tmpl_objects))
    else:
        objs, sea_yards = build_objects(t, tmpl_objects)
        objs = reposition_template_decor(t, objs)
        t.placed_objs = objs

    # --- replace payloads
    def set_payload(name, payload, version=None):
        c = tmpl.get(name)
        assert c is not None, name
        c.payload = payload
        if version is not None:
            c.version = version

    set_payload('HeightMapData', build_heightmap_payload(t))
    # 纹理先铺层和斜坡车道。装饰再盖层交界、水陆交界，并放树和石头。
    _blend_src = build_blendtiles_payload(t, tmpl_bt, decorate=decorate_map)
    if not bool(getattr(cfg, 'TERRAIN_ONLY', False)):
        objs = place_decorations(t, objs)
    set_payload('BlendTileData', _blend_src,
                version=tmpl.get('BlendTileData').version)
    obj_name_idx = None
    for c in tmpl.chunks:
        if c.name == 'ObjectsList':
            obj_name_idx = c.name_idx
    object_asset_idx = next(i for i, n in tmpl.table.items() if n == 'Object')
    if 'objectPrototypeScale' not in tmpl.table.values():
        _sid = (max(tmpl.table) + 1) if tmpl.table else 1
        tmpl.table[_sid] = 'objectPrototypeScale'
        tmpl.order.append(('objectPrototypeScale', _sid))
    key_ids = {name: idx for idx, name in tmpl.table.items()}
    set_payload('ObjectsList', write_objects(objs, object_asset_idx, key_ids),
                version=tmpl.get('ObjectsList').version)
    set_payload('StandingWaterAreas', build_water_payload())
    # WorldInfo
    wi = tmpl.get('WorldInfo')
    props = build_worldinfo(wi, tmpl.table)
    w = W(); write_props(w, props, key_ids)
    set_payload('WorldInfo', bytes(w.b))
    _drop_stale_assetlist(tmpl)
    patch_east_bonus_alliance(tmpl)
    if bool(getattr(cfg, 'ALLOW_NAVY', False)):
        _navy_lock = ('Only Ground', 'Only Ground Fix')
        if bool(getattr(cfg, 'REMOVE_NAVY_BAN', False)):
            remove_named_scripts(tmpl, _navy_lock)
        else:
            deactivate_named_scripts(tmpl, _navy_lock)
    patch_locked_factions(tmpl)
    nkl = getattr(cfg, 'NKL_SCRIPTS', None)
    if nkl:
        from nkl_scripts import append_nkl_scripts
        append_nkl_scripts(tmpl, P.data_file(nkl))
    if bool(getattr(cfg, 'ALLOW_NAVY', False)) and nkl:
        if bool(getattr(cfg, 'REMOVE_NAVY_BAN', False)):
            from sea_ai import NAVY_BAN_SCRIPTS
            remove_named_scripts(tmpl, NAVY_BAN_SCRIPTS)
        else:
            from sea_ai import unlock_navy
            unlock_navy(tmpl)
    if sea_yards:
        sides = parse_sides(tmpl.get('SidesList'), tmpl.table, has_asset_list=True)
        factions = {}
        for p in sides['players']:
            d = props_dict(p['props'])
            factions[d.get('playerName')] = d.get('playerFaction')
        from sea_ai import force_sea_ai
        force_sea_ai(tmpl, sea_yards, factions)
    from public_scripts import apply as apply_public_scripts, duplicate_names
    apply_public_scripts(tmpl, cfg)
    drop = duplicate_names(cfg)
    if drop:
        remove_named_scripts(tmpl, drop)

    # --- write out
    os.makedirs(OUTDIR, exist_ok=True)
    mappath = os.path.join(OUTDIR, MAPNAME + '.map')
    tmpl.save(mappath)
    write_art_tga(t, os.path.join(OUTDIR, MAPNAME + '_art.tga'))
    build_map_str(os.path.join(OUTDIR, 'map.str'))
    print('written', mappath, os.path.getsize(mappath), 'bytes')

    # --- verification: reload and check every chunk parses
    m2 = MapFile.load(mappath)
    print('reloaded chunks:', len(m2.chunks))
    hm = parse_heightmap(m2.get('HeightMapData'))
    print('heightmap:', hm['width'], 'x', hm['height'], 'border', hm['border_width'],
          'bounds', hm['borders'], 'z', min(hm['elevations']) * bt.Z_SCALE, max(hm['elevations']) * bt.Z_SCALE)
    bt2 = parse_blendtiles(m2.get('BlendTileData'), hm['width'], hm['height'], m2.table)
    print('blend tiles consumed %d / %d  textures %d  blends %d' % (
        bt2['consumed'], len(m2.get('BlendTileData').payload), len(bt2['textures']), bt2['blends_count']))
    objs2 = parse_objects(m2.get('ObjectsList'), m2.table)
    print('objects:', len(objs2))
    sl = parse_sides(m2.get('SidesList'), m2.table, has_asset_list=True)
    print('sides players:', sl['num_players'])
    print('water chunk size:', len(m2.get('StandingWaterAreas').payload))
    for o in objs2:
        d = props_dict(o['props'])
        if 'Waypoint' in o['type_name']:
            print('   wp %-28s (%7.1f,%7.1f)' % (d.get('waypointName'), o['position'][0], o['position'][1]))


if __name__ == '__main__':
    main()
