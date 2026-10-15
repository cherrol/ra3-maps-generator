# -*- coding: utf-8 -*-
"""[CHAI]叠岭连原 —— 地图配方

以主区外轮廓和定稿分层为蓝图。北边是高原，中间是平原，
南边是山脉和盆地，再往南又是山。这张图不靠海。

北侧四个圆圈是人类出生点。南侧从西向东是人类、固定 AI、固定 AI、人类。
红线是斜坡。固定 AI 与南侧人类结盟。
"""

MAP_NAME = '[CHAI]叠岭连原'
TEMPLATE = '8p_template.map'
TERRAIN_ONLY = False
INLAND = True
# 共用脚本：NKL blizzard。禁船厂、禁舰船的条目从列表里删掉。这张图不靠海，不另放船厂。
ALLOW_NAVY = True
REMOVE_NAVY_BAN = True
FORCE_SEA_AI = True
NKL_SCRIPTS = 'nkl_blizzard_scripts.bin'
# 可选公共脚本，这张图列装。其他图和新图默认不装。
# 开局满 1 分钟，不看出兵数量，强制让进攻队打对位出生点，并优先打对位玩家。
# 之后每 2 分钟再看一次：人数 >= 20 就再下令；低于 20 把队伍交还 AI 并取消优先目标。
TEAM_ATK = True
LAYER_TEX_PRESET = 'mountain'
LAYER_TEX_STRICT = True
CELL_SIZE = 4
TEX_SET = [
    ('Grass_CapeCod01', 'TGrass_CapeCod01', 'TGrass_CapeCod01_nrm'),
    ('Grass_CapeCod03', 'TGrass_CapeCod03', 'TGrass_CapeCod03_nrm'),
    ('Grass_CapeCod05', 'TGrass_CapeCod05', 'TGrass_CapeCod05_nrm'),
    ('Grass_CapeCod07', 'TGrass_CapeCod07', 'TGrass_CapeCod07_nrm'),
    ('Grass_CapeCod09', 'TGrass_CapeCod09', 'TGrass_CapeCod09_nrm'),
    ('Grass_CapeCod12', 'TGrass_CapeCod12', 'TGrass_CapeCod12_nrm'),
    ('Grass_Golf01', 'TGrass_Golf01', 'TGrass_Golf01_nrm'),
    ('Grass_Golf03', 'TGrass_Golf03', 'TGrass_Golf03_nrm'),
    ('Grass_Golf05', 'TGrass_Golf05', 'TGrass_Golf05_nrm'),
    ('Grass_Geneva01', 'TGrass_Geneva01', 'TGrass_Geneva01_nrm'),
    ('Grass_Geneva03', 'TGrass_Geneva03', 'TGrass_Geneva03_nrm'),
    ('Rock_Yucatan01', 'TRock_Yucatan01', 'TRock_Yucatan01_nrm'),
    ('RA3_Elevation1', 'RA3_Elevation1', 'RA3_Elevation1_NRM'),
    ('Transition_Yucatan18', 'TTransition_Yucatan18', 'TTransition_Yucatan18_nrm'),
    ('RA3_Elevation0', 'RA3_Elevation0', 'RA3_Elevation0_NRM'),
    ('Transition_Yucatan14', 'TTransition_Yucatan14', 'TTransition_Yucatan14_nrm'),
    ('Gravel_Yucatan01', 'TGravel_Yucatan01', 'TGravel_Yucatan01_nrm'),
    ('Dirt_Yucatan06', 'TDirt_Yucatan06', 'TDirt_Yucatan06_nrm'),
    ('Transition_Yucatan13', 'TTransition_Yucatan13', 'TTransition_Yucatan13_nrm'),
    ('Transition_Yucatan16', 'TTransition_Yucatan16', 'TTransition_Yucatan16_nrm'),
    ('Transition_Yucatan20', 'TTransition_Yucatan20', 'TTransition_Yucatan20_nrm'),
    ('Transition_Yucatan15', 'TTransition_Yucatan15', 'TTransition_Yucatan15_nrm'),
    ('Transition_Yucatan04', 'TTransition_Yucatan04', 'TTransition_Yucatan04_nrm'),
    ('Dirt_Yucatan03', 'TDirt_Yucatan03', 'TDirt_Yucatan03_nrm'),
    ('Transition_Yucatan03', 'TTransition_Yucatan03', 'TTransition_Yucatan03_nrm'),
    ('Dirt_Yucatan05', 'TDirt_Yucatan05', 'TDirt_Yucatan05_nrm'),
    ('Transition_Yucatan08', 'TTransition_Yucatan08', 'TTransition_Yucatan08_nrm'),
    ('Dirt_Yucatan04', 'TDirt_Yucatan04', 'TDirt_Yucatan04_nrm'),
    ('Rock_Yucatan02', 'TRock_Yucatan02', 'TRock_Yucatan02_nrm'),
    ('RA3_ShallowSeaFloor', 'RA3_ShallowSeaFloor', 'RA3_ShallowSeaFloor_NRM'),
    ('Dirt_Yucatan02', 'TDirt_Yucatan02', 'TDirt_Yucatan02_nrm'),
    ('Dirt_Yucatan01', 'TDirt_Yucatan01', 'TDirt_Yucatan01_nrm'),
    ('Reef_Yucatan02', 'TReef_Yucatan02', 'TReef_Yucatan02_nrm'),
    ('Reef_Havana02', 'Reef_Havana02', 'Reef_Havana02_NRM'),
    ('Reef_Yucatan01', 'TReef_Yucatan01', 'TReef_Yucatan01_nrm'),
    ('Transition_Yucatan02', 'TTransition_Yucatan02', 'TTransition_Yucatan02_nrm'),
    ('Rock_Yucatan04', 'TRock_Yucatan04', 'TRock_Yucatan04_nrm'),
    ('Rock_Yucatan03', 'TRock_Yucatan03', 'TRock_Yucatan03_nrm'),
    ('Transition_Yucatan06', 'TTransition_Yucatan06', 'TTransition_Yucatan06_nrm'),
    ('Transition_Yucatan19', 'TTransition_Yucatan19', 'TTransition_Yucatan19_nrm'),
    ('Transition_Yucatan21', 'TTransition_Yucatan21', 'TTransition_Yucatan21_nrm'),
    ('SteelDeck01', 'TMisc_SteelDeck01', 'TMisc_SteelDeck01_nrm'),
    ('Sand_Hawaii02', 'TSand_Hawaii02', 'TSand_Hawaii02_NRM'),
    ('BB_Gravel01', 'BB_Gravel01', 'BB_Gravel01_NRM'),
    ('Grass_NewYork01', 'TGrass_NewYork01', 'TGrass_NewYork01_NRM'),
    ('Grass_MtRush04', 'Grass_MtRush04', 'Grass_MtRush04_NRM'),
    ('Dirt_Iceland05', 'TDirt_Iceland05', 'TDirt_Iceland05_NRM'),
    ('Pavement_Amsterdam05', 'TPave_Amsterdam05', 'TPave_Amsterdam05_NRM'),
]
GRASS_TEXTURES = ('Grass_CapeCod01', 'Grass_CapeCod03', 'Grass_CapeCod05', 'Grass_CapeCod07',
                  'Grass_CapeCod09', 'Grass_CapeCod12', 'Grass_Golf01', 'Grass_Golf03',
                  'Grass_Golf05', 'Grass_Geneva01', 'Grass_Geneva03')
TEX_NOISE_CELLS = 24.0

GRID = 840                # 高度图边长。可玩区 800 格（GRID - 2*BORDER）
BORDER = 20
CELL = 10.0
Z_SCALE = 0.0390625
LAND_Z = 210.0
WATER_Z = 200.0
SLOPE_IMPASSABLE = 24.0
PEAK_IMPASSABLE = 235.0

# 取景框是主区外接矩形，四周留一点边。
GEOJSONS = [
    ('main_boundary.json', '', 1.00),
]
LON0, LON1 = 105.43, 111.31
LAT0, LAT1 = 31.65, 39.65
V_STRETCH = 0.735

# 海拔上限（米）-> 世界高度。这张图整体比沿海图高，200 米分界会把平原并进高原。
# 800 以下是盆地和平原；800–1200 是丘陵和高原南缘；1200–1600 是高原；
# 1600–2400 是山脉；2400 以上是高峰（不可通行）。
LAYER_M = (
    (800.0, 212.0),
    (1200.0, 280.0),
    (1600.0, 350.0),
    (2400.0, 420.0),
)
SEA_Z = 190.0
PEAK_Z = 480.0

PREVIEW = {
    'boundary_label': '边界',
}

# 出生点身份：(经度, 纬度, 名称, 队伍, 类型)
# 格子以 TERRAIN['base_cells'] 为准，经纬度是这些格的反投影。
# 北侧自西向东 P1–P4 人类。南侧自西向东 P5 人类、P7 固定 AI、P8 固定 AI、P6 人类。
# 固定 AI 与南侧人类结盟，模板里这侧是 Player_5 / Player_6。
BASES = [
    (106.1191, 39.1500, '西北', 'W', 'human'),  # (95, 60) 二层
    (107.6258, 39.1500, '北', 'W', 'human'),    # (259, 60) 三层
    (109.2520, 38.6125, '北东', 'W', 'human'),  # (436, 103) 三层
    (109.9319, 39.1500, '东北', 'W', 'human'),  # (510, 60) 三层
    (106.4131, 32.6250, '西南', 'E', 'human'),  # P5
    (110.1983, 32.6375, '东南', 'E', 'human'),  # P6，原东南固定 AI 的位置
    (107.5248, 32.3875, '南', 'E', 'ai'),       # P7，原南侧人类的位置
    (108.6732, 32.4625, '南东', 'E', 'ai'),     # P8，原南东固定 AI 的位置
]

MAP_STR_INTRO = '"欢迎来到[CHAI]叠岭连原，祝大家玩得开心。"'
MAP_STR_BRIEFING = ('"北侧西北、北、北东、东北四个人类出生点。\\n'
                    '南侧西南和东南是人类，南和南东是固定 AI。\\n'
                    '固定 AI 与南侧人类结盟。"')
MAP_STR_STARTPOS = ('"房间里的 1–6 号是人类位，按西北、北、北东、东北、西南、东南。\\n'
                    '7 号南、8 号南东是固定 AI，不在房间里选。"')

PREVIEW_TITLE = '{name}'
PREVIEW_SUBTITLE = '北4人 VS 南2人+2个固定AI'
PREVIEW_LEGEND = [
    '按定稿红线分层',
    '数字 1–8 是出生点，红线是斜坡',
    '北侧四人人类；南侧两人人类，南东、东南为固定 AI',
]
PREVIEW_LEGEND_ITEMS = (
    ((255, 214, 0), '矿脉'),
    ((90, 160, 255), '北队'),
    ((255, 120, 90), '南队（含固定 AI）'),
    ((80, 200, 255), '斜坡'),
    ((120, 96, 74), '不可通行'),
)
PREVIEW_LABELS = {
    'Player_1_Start': ('p1', 'W'),
    'Player_2_Start': ('p2', 'W'),
    'Player_3_Start': ('p3', 'W'),
    'Player_4_Start': ('p4', 'W'),
    'Player_5_Start': ('p5', 'E'),
    'Player_6_Start': ('p6', 'E'),
    'Player_7_Start (imported)': ('p7', 'E'),
    'Player_8_Start (imported)': ('p8', 'E'),
}
RIVERS = []
CITY_AMP = {}
DEFAULT_AMP = 0.35
OUTSIDE_AMP = 0.70

TERRAIN = {
    'lat_bias': 0.50,
    'use_v11_layers': True,
    'v11_plain': True,
    'v11_smooth': 0,
    # 高度场是 T0.2.0 的 800 格定稿，斜坡已经在里面。没开的图仍会重切层、重挖坡。
    'v11_baked': True,
    'ramp_layout': 'face',
    # 划线如果跨过两级以上，一条坡直接连最低层和最高层。没开的图仍只连相邻层。
    'ramp_mark_span': True,
    'ramp_size': 25.0,
    'ramp_gap': 80.0,
    'ramp_max_per_edge': 5,
    'base_cells': [
        (114, 60),     # 西北 P1，二层
        (319, 60),     # 北 P2，三层
        (470, 60),     # 北东 P3，三层
        (633, 60),     # 东北 P4，三层
        (85, 779),     # 西南 P5，一层
        (742, 761),    # 东南 P6，一层。原 p8 的位置
        (305, 746),    # 南 P7，一层，固定 AI。原 p6 的位置
        (485, 779),    # 南东 P8，二层，固定 AI。原 p7 的位置
    ],
    # 红线。角度 0 朝东、90 朝南，沿笔画方向。上坡取朝高层的一侧。
    'ramp_marks': (
        (574, 768, 146),
        (217, 110, 31),
        (675, 130, -124),
        (72, 172, 120),
        (601, 265, -154),
        (90, 285, 82),
        (768, 293, -8),
        (238, 333, -180),
        (694, 444, -110),
        (238, 501, 144),
        (658, 535, 82),
        (58, 605, -82),
        (215, 604, -132),
        (688, 626, -180),
        (776, 633, -174),
        (213, 649, -127),
        (118, 665, -84),
        (442, 718, 62),
        (70, 731, -72),
        (208, 743, -116),
        (552, 344, 180),
        (570, 472, -90),
        # 南侧谷地北缘。第四项为 1 的允许一条坡跨过中间层。上坡朝高层。
        (440, 517, -135, 1),
        (330, 578, 180, 1),
    ),
    'base_clear_want': 8.0,
    'base_min_flat_radius': 24.0,
    'base_min_ore': 2,
    'base_ore_range': 66.0,
    'ore_open_rank': True,
    # 近矿 2、中矿 2 标给这个出生点。远矿 2 不标归属。
    # 没开的图仍是每名玩家 4 个不归属的外围矿。
    'ore_bands': True,
    'ore_near_max': 2,
    'ore_near_min_r': 44.0,
    # 只放宽点名的近矿：可以贴着基地，矿口按 ore_face_at 朝外。没写的矿仍按 44 格、朝基地。
    'ore_beside': True,
    'ore_near_max_r': 62.0,
    'ore_far_min_dist': 160.0,
    'ore_far_max_dist': 320.0,
    'ore_far_fallback': 80.0,
    'ore_per_player': (5.0, 6.0),
    'ore_open_want': 20.0,
    'ore_edge_clear': 20.0,
    'ore_shore_clear': 20.0,
    'ore_border_clear': 30.0,
    'ore_mouth_r1': 20,
    'ore_flat_soft': 3.0,
    'levels': (212.0, 280.0, 350.0, 420.0),
    'sea_flat': 130.0,
}

# 单独圈出的层。只改这几处的海拔，让它们落进邻层，预览原有的并入步骤会留住结果。
MERGE_LAYER = 'merge_layer.npz'


def _install_merge():
    import os
    import sys
    import numpy as np
    import relief_preview as rp
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'data', MERGE_LAYER)
    # 直接跑 tools/relief_preview.py 时，切层函数在 __main__ 里，不在模块副本上。
    mods = [rp]
    main = sys.modules.get('__main__')
    if main is not None and getattr(main, 'quantize', None) is not None and main is not rp:
        mods.append(main)

    def make(orig):
        def quantize(dem, land, cfg, _orig=orig):
            if os.path.isfile(path) and getattr(cfg, 'MAP_NAME', '') == MAP_NAME:
                pack = np.load(path)
                ox, oy, sx, sy = rp.frame(cfg)
                x = np.rint(ox + (pack['lon'] - cfg.LON0) * sx).astype(np.int32)
                y = np.rint(oy + (cfg.LAT1 - pack['lat']) * sy).astype(np.int32)
                h, w = dem.shape
                ok = (x >= 0) & (x < w) & (y >= 0) & (y < h)
                dem = np.array(dem, copy=True)
                dem[y[ok], x[ok]] = pack['elev'][ok]
            return _orig(dem, land, cfg)
        quantize._merge_heights = True
        return quantize

    for mod in mods:
        if getattr(mod.quantize, '_merge_heights', False):
            continue
        mod.quantize = make(mod.quantize)


_install_merge()
