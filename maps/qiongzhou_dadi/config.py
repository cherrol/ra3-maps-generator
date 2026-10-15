# -*- coding: utf-8 -*-
"""[CHAI]琼州大地 —— 地图配方

命名对照已有的图：两个字的古称加「大地」。
八桂是广西，南粤是广东，三秦是陕西，琼州是海南。

以海南岛行政边界为蓝图。层高按手圈：大圈是二层，中间小圈是三层，
圈外陆地是一层。出生点按环岛的八个圆圈，斜坡按红线。
西侧四人，东侧两人加两个固定 AI（盟军、帝国）。脚本用共用的 NKL blizzard 套。
三沙的远海岛礁不放进这张图。
"""

MAP_NAME = '[CHAI]琼州大地'
TEMPLATE = '8p_template.map'
TERRAIN_ONLY = False
ALLOW_NAVY = True
REMOVE_NAVY_BAN = True
FORCE_SEA_AI = True
NKL_SCRIPTS = 'nkl_blizzard_scripts.bin'

GRID = 840
BORDER = 20
CELL = 10.0
Z_SCALE = 0.0390625
LAND_Z = 210.0
WATER_Z = 200.0
SLOPE_IMPASSABLE = 24.0
PEAK_IMPASSABLE = 235.0
HEIGHT_PRESET = 'standard'

GEOJSONS = [
    ('hainan_boundary.json', '海南', 1.00),
]
LON0, LON1 = 108.59, 111.07
LAT0, LAT1 = 18.14, 20.16
V_STRETCH = 1.059

LAYER_M = (
    (200.0, 210.0),
    (500.0, 280.0),
    (1500.0, 350.0),
    (4000.0, 420.0),
)
SEA_Z = 130.0
PEAK_Z = 500.0

PREVIEW = {
    'boundary_label': '省界',
}

LAYER_RINGS = 'layer_rings.npz'

# 八个圆圈都在海上。西侧四人是人类，东侧两人是人类，
# 东南和南边两个固定 AI：盟军、帝国。人类在前，固定 AI 在后。
BASES = [
    (109.4115, 19.9638, '北', 'W', 'human'),
    (109.1232, 19.7384, '西北', 'W', 'human'),
    (108.8597, 19.5715, '西', 'W', 'human'),
    (109.8300, 18.3011, '南海', 'W', 'human'),
    (110.2888, 20.1453, '东北', 'E', 'human'),
    (110.7011, 19.1705, '东', 'E', 'human'),
    (110.5492, 18.8104, '东南', 'E', 'ai'),
    (110.1741, 18.4357, '南', 'E', 'ai'),
]
BASE_LABELS = {
    '北': '北（人类）',
    '西北': '西北（人类）',
    '西': '西（人类）',
    '南海': '南海（人类）',
    '东北': '东北（人类）',
    '东': '东（人类）',
    '东南': '东南（盟军 AI）',
    '南': '南（帝国 AI）',
}
PREVIEW_LABELS = {
    'Player_1_Start': ('北', 'W'),
    'Player_2_Start': ('西北', 'W'),
    'Player_3_Start': ('西', 'W'),
    'Player_4_Start': ('南海', 'W'),
    'Player_5_Start': ('东北', 'E'),
    'Player_6_Start': ('东', 'E'),
    'Player_7_Start (imported)': ('东南·盟军', 'E'),
    'Player_8_Start (imported)': ('南·帝国', 'E'),
}

# 模板里 PlyrCreeps 是盟军、PlyrCivilian 是苏联。
# 固定位要一个盟军、一个帝国：盟军建造场留在 7 号，苏联建造场改成帝国放到 8 号。
YARD_PLACEMENT = [
    ('AlliedConstructionYard', 'Player_7_Start (imported)', 2, -3, 20),
    ('JapanConstructionYard', 'Player_8_Start (imported)', -2, 3, 20),
]
YARD_RETYPE = {'SovietConstructionYard': 'JapanConstructionYard'}
FACTION_LOCK = {'PlyrCivilian': 'PlayerTemplate:Japan'}
TEAM_UNIT_RETYPE = {
    'PlyrCivilian': {'GameObject:SovietEngineer': 'GameObject:JapanEngineer'},
}

RIVERS = []
CITY_AMP = {}
DEFAULT_AMP = 0.35
OUTSIDE_AMP = 0.70

# 地级市和县的中心。名字与省界要素一致，油井和房子围着它们。
# 自治县由放置逻辑跳过。
CITY_SEATS = {
    '海口市': (110.4230, 19.8521),
    '三亚市': (109.4217, 18.3623),
    '儋州市': (109.3901, 19.5793),
    '五指山市': (109.5119, 18.8240),
    '琼海市': (110.4220, 19.2104),
    '文昌市': (110.7885, 19.7474),
    '万宁市': (110.3245, 18.8304),
    '东方市': (108.8188, 18.9962),
    '定安县': (110.3129, 19.4834),
    '屯昌县': (110.0557, 19.3422),
    '澄迈县': (109.9904, 19.7002),
    '临高县': (109.7155, 19.8097),
}
CITY_HOUSE_TYPES = ('ST_Building01', 'ST_Building02', 'ST_Building03')
CITY_HOUSE_SMALL = ('ST_Building01', 'ST_Building02')
CITY_TEX_R = 30.0
CITY_TEX_ROUND = True
CITY_TEX = ('Pavement_Heidelberg10',)
CITY_CLUSTER = True

# 中路矿铺在岛上，横坐标落在全图 15%–85%。
CENTER_ORE_COUNT = 6
CENTER_ORE_XS = (0.38, 0.50, 0.45, 0.58, 0.42, 0.62)
CENTER_ORE_YS = (0.40, 0.46, 0.55, 0.52, 0.62, 0.58)
ORE_MIN_SPACING = 46.0

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
    ('Pavement_Heidelberg10', 'Pavement_Heidelberg10', 'Pavement_Heidelberg10_nrm'),
]
GRASS_TEXTURES = ('Grass_CapeCod01', 'Grass_CapeCod03', 'Grass_CapeCod05', 'Grass_CapeCod07',
                  'Grass_CapeCod09', 'Grass_CapeCod12', 'Grass_Golf01', 'Grass_Golf03',
                  'Grass_Golf05', 'Grass_Geneva01', 'Grass_Geneva03')
TEX_NOISE_CELLS = 24.0
BEACH_TEX_R = 5.0
SEA_TEX_BANDS = (
    (6.0, 'Sand_Hawaii02'),
    (36.0, 'Reef_Yucatan02'),
    (100.0, 'Reef_Yucatan01'),
    (1.0e9, 'RA3_ShallowSeaFloor'),
)

WATER = dict(name='New Water Area', uv_scroll=0.06, additive=0,
             bump='WaterRippleBump', sky='SkyEnv', fx='FXOceanRA3',
             depth_colors='LUTDepthTint.tga', height=None,
             margin=200.0)

MAP_STR_INTRO = '"欢迎来到[CHAI]琼州大地，祝大家玩得开心。"'
MAP_STR_BRIEFING = ('"西侧北、西北、西、南海四个人类出生点。\\n'
                    '东侧东北、东两个人类，东南是盟军 AI，南是帝国 AI。\\n'
                    '八个出生点都在海上。斜坡按标注的红线。海陆空都不限制。"')
MAP_STR_STARTPOS = ('"房间里的 1–6 号是人类位，按北、西北、西、南海、东北、东。\\n'
                    '7 号东南是固定盟军，8 号南是固定帝国，不在房间里选。"')

PREVIEW_SUBTITLE = '海南岛 · 西4人 VS 东2人+2个固定AI（盟军、帝国）'
PREVIEW_LEGEND = [
    '按手圈：大圈二层，中间小圈三层，圈外陆地一层',
    '八个圆圈是海上出生点，红线是斜坡',
    '西侧四人人类；东侧两人人类，东南盟军 AI，南帝国 AI',
]
PREVIEW_LEGEND_ITEMS = (
    ((255, 214, 0), '矿脉'),
    ((255, 92, 0), '油井'),
    ((90, 160, 255), '西队'),
    ((255, 120, 90), '东队（含盟军、帝国）'),
    ((32, 90, 160), '海面'),
    ((80, 200, 255), '斜坡'),
    ((120, 96, 74), '不可通行'),
)

TERRAIN = {
    'lat_bias': 0.50,
    'art_keep_sea': True,
    'use_v11_layers': True,
    'v11_plain': True,
    'v11_smooth': 0,
    'v11_skip_water': True,
    'v11_skip_peak': True,
    'z_clip': (0.0, 420.0),
    'ramp_layout': 'face',
    'ramp_size': 25,
    'ramp_gap': 120,
    'ramp_max_per_edge': 5,
    # 水上，贴着各自的水陆斜坡，落在坡下的平水面。建造面不压到斜坡，也不贴岸线。
    'base_cells': [
        (285, 142),    # 北，人类，朝 (282,176)
        (192, 219),    # 西北，人类，朝 (219,240)
        (107, 276),    # 西，人类，朝 (130,300)
        (420, 710),    # 南海，人类，朝 (412,677)
        (568, 80),     # 东北，人类，朝 (583,111)
        (701, 413),    # 东，人类，朝 (671,396)
        (652, 536),    # 东南，盟军 AI，朝 (620,524)
        (531, 664),    # 南，帝国 AI，朝 (504,643)
    ],
    # 和 base_cells 一一对应，基地朝向这条水陆斜坡。
    'base_face': (
        (282, 176),
        (219, 240),
        (130, 300),
        (412, 677),
        (583, 111),
        (671, 396),
        (620, 524),
        (504, 643),
    ),
    # 红线。角度 0 朝东、90 朝南，沿笔画方向。
    'ramp_marks': (
        (587, 120, 63),
        (397, 146, 92),
        (275, 190, 96),
        (226, 241, -142),
        (754, 278, -141),
        (135, 306, 47),
        (237, 373, 64),
        (677, 397, -147),
        (414, 402, 145),
        (42, 401, -175),
        (116, 508, -148),
        (626, 527, -159),
        (526, 546, -152),
        (198, 631, 125),
        (511, 644, -142),
        (412, 672, 76),
        (339, 682, 52),
        (266, 713, 83),
        # 三层通往北面和东面。西南那条由交界自动开口。
        (282, 418, 90),
        (432, 530, 180),
    ),
    'base_clear_want': 8.0,
    'base_min_flat_radius': 24.0,
    'base_water_clear': 14.0,
    'base_min_ore': 2,
    'base_ore_range': 66.0,
    'ore_open_rank': True,
    'ore_near_max': 2,
    'ore_near_min_r': 44.0,
    'ore_near_max_r': 62.0,
    'ore_near_slot_r': 50.0,
    'ore_far_min_dist': 160.0,
    'ore_far_max_dist': 320.0,
    'ore_scatter_water': True,
    'ore_scatter_spacing': 64.0,
    'ore_scatter_shore': 32.0,
    'ore_per_player': (5.0, 6.0),
    'neutral_in_water': True,
    'neutral_water_edge': 12.0,
    'ore_in_water': True,
    # 水上基地的近矿只落在水下，离开岸线，也不进别的基地的近矿环。
    'ore_near_water': True,
    'ore_water_edge_min': 4.0,
    'ore_open_want': 20.0,
    'ore_edge_clear': 20.0,
    'ore_shore_clear': 20.0,
    'ore_border_clear': 30.0,
    'ore_mouth_r1': 20,
    'ore_flat_soft': 3.0,
    'levels': (210.0, 280.0, 350.0, 420.0),
    'sea_flat': 130.0,
    'sea': (6.0, 0.0, 220.0, 0.0),
}
