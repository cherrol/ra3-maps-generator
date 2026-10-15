# -*- coding: utf-8 -*-
"""[CHAI]岭尽生潮 —— 地图配方

北高南低，南边是海。高度用预览定稿的分层。出生点是预览图上的八个红圈。
"""

MAP_NAME = '[CHAI]岭尽生潮'
TEMPLATE = '8p_template.map'
TERRAIN_ONLY = False
# 模板开局脚本 Only Ground，以及 NKL 里开局禁船厂的条目，都会锁海军。这张图有海，不锁。
ALLOW_NAVY = True
# 禁船厂、禁舰船的脚本从列表里删掉。没开这个开关的图仍只是关掉。
REMOVE_NAVY_BAN = True
# 每个固定 AI 按 AXAI 的时间升科技（180 秒 T2，360 秒科技中心）。
# 落在水上的固定 AI 另外强制造船厂和空军。
FORCE_SEA_AI = True
# NKL blizzard v1.00 的公共脚本，组装时接在本图公共列表后面。
NKL_SCRIPTS = 'nkl_blizzard_scripts.bin'

GRID = 840
BORDER = 20
CELL = 10.0
Z_SCALE = 0.0390625
LAND_Z = 210.0
WATER_Z = 200.0
SLOPE_IMPASSABLE = 24.0
PEAK_IMPASSABLE = 235.0
# 通用层高：水域 0–130，一层 210，二层 280，三层 350，四层 420，五层山峰 500–1000。
HEIGHT_PRESET = 'standard'

# 取景框是主区外接矩形，四周留一点边，南边多留一条海。
# 邻区只补陆地，不画进边界。预览红圈有两处落在框内的邻区高地上。
GEOJSONS = [
    ('main_boundary.json', '', 1.00),
    ('fill_w.json', '', 0.70),
    ('fill_n.json', '', 0.70),
    ('fill_ne.json', '', 0.70),
    ('fill_e.json', '', 0.70),
]
LON0, LON1 = 109.61, 117.23
LAT0, LAT1 = 20.16, 25.55
V_STRETCH = 1.41

# 海拔上限（米）-> 世界高度。海面以下另算。
# 0–200 一层，200–500 二层，500–1500 三层，1500–4000 四层，4000 以上五层（不可通行）。
LAYER_M = (
    (200.0, 210.0),
    (500.0, 280.0),
    (1500.0, 350.0),
    (4000.0, 420.0),
)
# 水面 200。贴岸海底 130，外海落到 0。五层山峰 500–1000，不可通行。
SEA_Z = 130.0
PEAK_Z = 500.0

# 预览阈值。这组数也是工具缺省；别的图可以整份覆盖。
PREVIEW = {
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

# 经纬度框。框里按初始水域留一块宽的凸起，更窄的小水域并进陆地。
# 这张图框住的是南岸河口。
COAST_KEEP = (112.2, 115.0, 21.5, 23.3)

# 出生点身份： (经度, 纬度, 名称, 队伍, 类型)
# 落点以 TERRAIN['base_cells'] 为准。经纬度是这些格的反投影。
# 西侧 P1–P4 自北向南（P4 在南端尖外的水上）。
# 东侧自北向南：P5 东北人类、P7 东部陆地固定 AI、P8 东水上固定 AI、P6 东南水上人类。
# 苏联锁在 P7，帝国锁在 P8，换位置不换阵营。
BASES = [
    (110.2044, 25.1157, '西北', 'W', 'human'),   # P1 (82, 86) 二层
    (110.1815, 23.1431, '西部', 'W', 'human'),   # P2 (80, 378) 一层
    (110.3644, 21.6894, '西南', 'W', 'human'),   # P3 (99, 593) 一层
    (110.5701, 20.4194, '南端', 'W', 'human'),   # P4 (121, 781) 水上
    (116.1556, 25.1535, '东北', 'E', 'human'),   # P5 (707, 80) 二层
    (116.6890, 20.6031, '东南', 'E', 'human'),   # P6 (763, 754) 水上，原 P8
    (116.3537, 23.2782, '东部', 'E', 'ai'),      # P7 (728, 358) 一层，原 P6，苏联
    (116.6585, 21.6732, '东', 'E', 'ai'),        # P8 (760, 595) 水上，原 P7，帝国
]

# 城镇中心。油井和房子落在这些点附近。
CITY_POINTS = [
    (113.28064, 23.12518), (113.59154, 24.80132),
    (114.08595, 22.54700), (113.55399, 22.22498),
    (116.70846, 23.37102), (113.12272, 23.02876),
    (113.09494, 22.59043), (110.36498, 21.27490),
    (110.91923, 21.65975), (112.47253, 23.05155),
    (114.41260, 23.07940), (116.11758, 24.29911),
    (114.69780, 23.74627), (111.97511, 21.85922),
    (113.05123, 23.68502), (113.74626, 23.04624),
    (113.38239, 22.52111), (116.63230, 23.66170),
    (116.35573, 23.54378), (112.04444, 22.92980),
]
BASE_CITY_CLEARANCE = 30.0
CITY_TEX_R = 30.0
CITY_HOUSE_TYPES = ('ST_Building01', 'ST_Building02', 'ST_Apartment_01', 'ST_Building03',
                    'ST_Building06', 'ST_Hotel_01')
CITY_ORE_OFFSET = 14.0
CITY_DERRICK_OFFSET = 11.0

RIVERS = []
BRIDGES = []

CITY_AMP = {}
DEFAULT_AMP = 0.35
OUTSIDE_AMP = 0.70

TERRAIN = {
    # 预览定稿分层，行 0 = 北。浅海 200 已收成 190。不再做噪声和海陆雕刻。
    'use_v11_layers': True,
    'v11_plain': True,
    'v11_smooth': 0,
    # 斜坡边长。隔层落差太大时沿坡会自动加长到还能走的最短，宽度仍用这个数。
    'ramp_size': 25.0,
    'ramp_axis_snap': True,
    # 只连相邻层；坡心至少隔 120 格。凸出朝凸出，左侧边界垂直朝第一层。
    'ramp_layout': 'face',
    'ramp_gap': 120.0,
    # 每段层界最多 5 条斜坡，尽量让不同位置的玩家能过层、能到资源。
    'ramp_max_per_edge': 5,
    # 划线斜坡。角度是线的方向：内部帧 0° 朝东、90° 朝南。上坡取朝高层的一侧。
    'ramp_marks': (
        (339, 90, 56),
        (470, 118, -154),
        (274, 183, 176),
        (449, 284, 145),
        (586, 321, 138),
        (336, 329, -139),
        (147, 407, 49),
        (582, 452, 48),
        (238, 493, -157),
        (133, 500, 142),
        (410, 551, -141),
        (213, 623, -138),
    ),
    # 圈内整段海岸可登陆。圆心和半径是内部帧的格。
    'landing_coasts': (
        (71, 722, 81),
        (453, 491, 55),
    ),
    'lat_bias': 0.62,
    'base_cells': [
        (82, 86),     # P1 西北
        (80, 378),     # P2 西部
        (99, 593),    # P3 西南
        (121, 781),    # P4 南端尖外，水上
        (707, 80),     # P5 东北
        (763, 754),    # P6 东南，水上（原 P8）
        (728, 358),    # P7 东部，一层（原 P6，苏联）
        (760, 595),    # P8 东，水上（原 P7，帝国）
    ],
    'base_clear_want': 8.0,
    'ore_open_rank': True,
    'ore_near_max': 2,
    'ore_near_min_r': 44.0,
    'ore_near_max_r': 62.0,
    'ore_far_min_dist': 160.0,
    'ore_far_max_dist': 320.0,
    'ore_far_fallback': 80.0,
    'ore_far_pull_mid': 0.0,
    'ore_scatter_water': True,
    'ore_scatter_spacing': 64.0,
    'ore_scatter_shore': 32.0,
    'ore_per_player': (5.0, 6.0),
    'neutral_in_water': True,
    'neutral_water_edge': 12.0,
    'center_posts': [(452, 324), (612, 660)],
    'center_garage': [(500, 692)],
    'center_academy': [(516, 356)],
    'center_hospital': [(292, 500)],
    'center_airport': [(580, 644)],
    'sea_derrick_cells': [(404, 724), (324, 660), (660, 484)],
    'ore_in_water': True,
    'ore_water_edge_min': 4.0,
    'ore_open_want': 20.0,
    'ore_edge_clear': 20.0,
    'ore_shore_clear': 20.0,
    'ore_border_clear': 30.0,
    'ore_mouth_r1': 20,
    # 矿3 在北侧三层台地。西北 21 格落到二层，朝南西，那一侧整段开阔。+y 向南。
    'ore_face_at': (
        (416, 52, -1, 1),
    ),
    'ore_flat_soft': 3.0,
    'levels': (210.0, 280.0, 350.0, 420.0),
    'sea_flat': 130.0,
    'base_min_flat_radius': 24.0,
    'base_water_clear': 14.0,
    'base_min_ore': 2,
    'base_ore_range': 66.0,
}

# ---------------------------------------------------------------- 地表纹理
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
# 每层一张，不混贴。一层草地，二层山地草，三层土，四层铺装。
LAYER_TEX_STRICT = True
TEX_NOISE_CELLS = 24.0
BEACH_TEX_R = 5.0
CITY_TEX_R = 16.0
CITY_TEX_ROUND = True
CITY_TEX = ('Pavement_Heidelberg10',)
# 城镇的圆接在一起算一片。单独一片是一口油井加一栋小房子；
# 多座挨着则两栋小房子在两侧，两口油井散在中间。
CITY_CLUSTER = True
CITY_HOUSE_SMALL = ('ST_Building01', 'ST_Building02')
# 离岸距离（格）-> 海床。贴岸一条浅沙，往外礁石，外海深水底。
SEA_TEX_BANDS = (
    (6.0, 'Sand_Hawaii02'),
    (36.0, 'Reef_Yucatan02'),
    (100.0, 'Reef_Yucatan01'),
    (1.0e9, 'RA3_ShallowSeaFloor'),
)
LAYER_TEX = {
    210: ('Grass_NewYork01',),
    280: ('Grass_MtRush04',),
    350: ('Dirt_Iceland05',),
    420: ('Pavement_Amsterdam05',),
}

WATER = dict(name='New Water Area', uv_scroll=0.06, additive=0,
             bump='WaterRippleBump', sky='SkyEnv', fx='FXOceanRA3',
             depth_colors='LUTDepthTint.tga', height=None,
             margin=200.0)

MAP_STR_INTRO = '"欢迎来到[CHAI]岭尽生潮，祝大家玩得开心。"'
MAP_STR_DEBUG = '"西侧 Player_1-4 为一队；东侧 Player_5-6 与两个固定 AI（Player_7、Player_8）为一队"'
MAP_STR_INFO = {
    '0_1': 'P1 西北(西队)', '0_2': 'P2 西部(西队)', '0_3': 'P3 西南(西队)',
    '1_0': 'P4 南端(西队·水上)', '1_1': 'P5 东北(东队)', '1_2': 'P6 东南(东队·水上)',
    '1_3': 'P7 东部(固定AI·苏联)', '2_0': 'P8 东(固定AI·帝国·水上)',
    '2_1': '西队自北向南：P1-P2-P3-P4', '2_2': '东队自北向南：P5、P7、P8、P6',
    '2_3': '南部海域为海战区域',
    '3_0': '地图作者：[CHAI]', '3_1': '西4人 VS 东2人+2个固定AI', '3_2': '版本 {version}',
    '3_3': '祝游戏愉快',
}

PREVIEW_TITLE = '{name}'
PREVIEW_SUBTITLE = '西4人 VS 东2人+2个固定AI'
PREVIEW_LABELS = {
    'Player_1_Start': ('Player1', 'W'), 'Player_2_Start': ('Player2', 'W'),
    'Player_3_Start': ('Player3', 'W'), 'Player_4_Start': ('Player4', 'W'),
    'Player_5_Start': ('Player5', 'E'), 'Player_6_Start': ('Player6', 'E'),
    'Player_7_Start (imported)': ('Player7', 'E'),
    'Player_8_Start (imported)': ('Player8', 'E'),
}
PREVIEW_FOOTER = ('西队（自北向南）：Player1 / Player2 / Player3 / Player4（Player4 在南端尖外的水上）      '
                  '东队（自北向南）：Player5 / Player7（苏联·陆地）/ Player8（帝国·水上）/ Player6（水上）')

BASE_ORE_NEAR = ((-34, -14), (36, -12))
BASE_ORE_FAR = ((-96, -74), (104, -70))
BASE_ORE_OFFSETS = BASE_ORE_NEAR + BASE_ORE_FAR
ORE_NEAR_MAX_DIST = 100.0
BASE_ORE_DIRS = ((0.7071, -0.7071), (-0.7071, -0.7071),
                 (0.7071, 0.7071), (-0.7071, 0.7071))
ORE_NEAR_R = 38.0
ORE_MID_R = 78.0
CENTER_ORE_COUNT = 6
# 中路矿拉开：四块在陆地，两块在海上。x 落在可玩区 15%~85%。
CENTER_ORE_XS = (0.320, 0.500, 0.380, 0.620, 0.480, 0.280)
CENTER_ORE_YS = (0.280, 0.380, 0.550, 0.520, 0.780, 0.860)
ORE_FOOT_MIN = 6.0
ORE_FOOT_MAX = 16.0
ORE_MOUTH_CLEAR = 18.0
BASE_DERRICK_OFFSETS = ((34, 0), (-34, 0), (0, 34), (0, -34),
                        (26, 26), (-26, 26), (26, -26), (-26, -26))
ORE_LEFT_RING = (260.0,)
DECOR_POS = [(111.1340, 24.8022), (112.5056, 24.6942), (114.1820, 24.9103),
             (111.3626, 23.7214), (113.1152, 24.0456), (114.7916, 24.1537),
             (111.5912, 22.6405), (112.7342, 21.8839), (114.1820, 23.7214),
             (115.2488, 24.5861), (110.8292, 22.3163), (113.5724, 24.3699)]
DECOR_TYPES = ('StPetersburgSidewalk03', 'EI_Structure_02', 'EI_Structure_01',
               'EI_Structure_02_Collapse', 'EI_Structure_01_Collapse',
               'GarageTechStructure_Collapse', 'NUObservationPostTechStructure_Collapse',
               'NBOilDerrick_Collapse')
PAD_CLEARANCE = 46.0
RESOURCE_PAD_CLEARANCE = 30.0
ORE_BASE_CLEARANCE = 30.0
ORE_MIN_SPACING = 46.0
RESOURCE_MIN_SPACING = 32.0
DECOR_RESOURCE_CLEARANCE = 10.0
ORE_CLEAR_RADIUS = 20.0
DERRICKS_PER_BASE = 2
