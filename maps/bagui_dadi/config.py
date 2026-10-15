# -*- coding: utf-8 -*-
"""[CHAI]八桂大地 —— 地图配方（recipe）

以广西壮族自治区行政边界与真实地势为蓝图：西北高（云贵高原边缘）、东南低（盆地平原），
西江—郁江—柳江—桂江水系自西北向东南汇流，南部为北部湾海面。

一份配方 = 一张地图。复制本文件到 maps/<new_id>/config.py 改内容即可生成新地图；
所有字段都被 tools/ 下的通用工具读取（缺失的可选字段会退回工具内默认值）。
"""

# ---------------------------------------------------------------- 基本信息
MAP_NAME = '[CHAI]八桂大地'        # 地图名（= 游戏里 Maps 下的文件夹名 / .map 文件名）
TEMPLATE = '8p_template.map'       # templates/ 下的骨架（含 6 位 + 2 固定 AI 的脚本系统）
TERRAIN_ONLY = False               # True = 纯净地形图：不摆任何对象；False = 正常放出生点/矿/油井等
# 共用脚本：NKL blizzard。禁船厂、禁舰船的条目从列表里删掉。
ALLOW_NAVY = True
REMOVE_NAVY_BAN = True
FORCE_SEA_AI = True
NKL_SCRIPTS = 'nkl_blizzard_scripts.bin'
# 通用层高：水域 0–130，一层 210，二层 280，三层 350，四层 420，五层山峰 500–1000。
HEIGHT_PRESET = 'standard'

# ---------------------------------------------------------------- 尺寸与高程
GRID = 840                # 高度图边长（格）。可玩区 800 格（GRID - 2*BORDER），规范是每边 600–800
BORDER = 20                # 可玩区外的边框格数
CELL = 10.0                # 每格对应的世界单位（800 格 = 8000×8000 世界单位）
Z_SCALE = 0.0390625        # 高度原始值 -> 世界 z（uint16 * scale）
LAND_Z = 210.0             # 陆地基准高度
WATER_Z = 200.0            # 水面高度（低于此高度的地形会被水覆盖）
SLOPE_IMPASSABLE = 24.0    # 坡度阈值（z/格），超过判定为陡崖不可通行
PEAK_IMPASSABLE = 235.0    # 高于水面这么多的山峰判定为岩壁不可通行

# ---------------------------------------------------------------- 地理蓝图
# 地形蓝图：广西（14 个地级市）+ 邻省（用邻省地形补全省外区域）
#   (边界文件, 省/地区名, 省级山地强度)  省内地级市可用 CITY_AMP 单独覆盖
GEOJSONS = [
    ('guangxi_boundary.json', '广西', 1.00),
    ('yunnan_boundary.json', '云南', 0.90),
    ('guizhou_boundary.json', '贵州', 0.92),
    ('hunan_boundary.json', '湖南', 0.62),
    ('guangdong_boundary.json', '广东', 0.45),
]
# 取景框 = 已确认预览的四角。铺满 1000 格可玩区，出生圈才不会被挤出北缘。
LON0, LON1 = 104.258650, 112.246025
LAT0, LAT1 = 20.766741, 26.523876
V_STRETCH = 1.387387

# 预览用的海拔上限（米）-> 世界高度。海面以下另算。
# 0–200 沿海与盆地，200–500 丘陵，500–1500 桂西北与桂东北山地，
# 1500–4000 猫儿山等高峰，4000 以上五层。游戏图仍用 v11 蓝图，不改这里的格子和取景框。
LAYER_M = (
    (200.0, 210.0),
    (500.0, 280.0),
    (1500.0, 350.0),
    (4000.0, 420.0),
)
# 水面 200。贴岸海底 130，外海落到 0。五层山峰从 500 起。
SEA_Z = 130.0
PEAK_Z = 500.0
PREVIEW = {
    'boundary_label': '省界',
}
# 手圈的几处改成二层。只涂圈内陆地，圈外和水域不动。
LAYER2_PATCH = 'layer2_patch.npz'

# 出生点身份： (经度, 纬度, 名称, 队伍, 类型)
#   类型 human = 房间可选位，ai = 固定位（waypoint 名带 "(imported)"，不进房间列表）
#   队伍 W = 西队，E = 东队。固定 AI 与东队的结盟在组装时改模板脚本。
# 落点以 TERRAIN['base_cells'] 为准，与本列表按顺序一一对应。
# 经纬度是这些格的反投影：缺了 base_cells 时，Geo.to_cell 会回到同一格。
# 现行：图上数字 1–8 即 P1–P8。北侧四人是一队，南侧两人与两个固定 AI 是另一队。
BASES = [
    (105.93600, 26.19572, '西北', 'W', 'human'),  # P1 (188, 66) 三层
    (107.67725, 26.17845, '北', 'W', 'human'),    # P2 (362, 68) 三层
    (109.40252, 26.22450, '中北', 'W', 'human'),  # P3 (535, 62) 三层
    (111.04792, 25.44729, '东北', 'W', 'human'),  # P4 (700, 170) 二层
    (105.55260, 21.35973, '西南', 'E', 'human'),  # P5 (150, 738) 一层
    (111.54314, 21.22731, '东南', 'E', 'human'),  # P6 (750, 756) 水上
    (107.07021, 21.88938, '南', 'E', 'ai'),       # P7 (302, 664) 二层，固定 AI
    (109.67409, 21.89514, '南部', 'E', 'ai'),     # P8 (562, 663) 一层，固定 AI
]

# 广西 14 个地级市市区（城市中心）：出生点要避开、油田与非出生点矿脉围绕城市布置
CITY_SEATS = {
    '南宁市': (108.37, 22.82), '柳州市': (109.42, 24.33), '桂林市': (110.29, 25.27),
    '梧州市': (111.28, 23.48), '北海市': (109.12, 21.48), '防城港市': (108.35, 21.69),
    '钦州市': (108.65, 21.98), '贵港市': (109.60, 23.10), '玉林市': (110.18, 22.63),
    '百色市': (106.62, 23.90), '贺州市': (111.55, 24.42), '河池市': (108.06, 24.70),
    '来宾市': (109.23, 23.73), '崇左市': (107.36, 22.40),
}
BASE_CITY_CLEARANCE = 30.0        # 出生点离最近城市中心的最小距离（格）——出生点避开城市
CITY_TEX_R = 30.0                 # 城市半径（格）：范围内铺水泥路（SteelDeck+碎石），其余一律草地
# 城市房屋：每座城市放 1~2 个房子（类型名经 WB 物件库核实），预览里在房子旁标城市名
CITY_HOUSE_TYPES = ('ST_Building01', 'ST_Building02', 'ST_Apartment_01', 'ST_Building03',
                    'ST_Building06', 'ST_Hotel_01')
CITY_ORE_OFFSET = 14.0            # 城市矿脉离市中心的距离（格）
CITY_DERRICK_OFFSET = 11.0        # 城市油井离市中心的距离（格）——油田在城市中心或周边

# 真实海岸线（闭合多边形，自西南的越南海岸 -> 广西海岸 -> 雷州半岛 -> 广东海岸 -> 地图上沿）。
# 用"省界 ∪ 该多边形"判定陆地，其余即真实海域：西南的北部湾不会再被画成山脉。
COAST = [
    (103.00, 18.50), (105.00, 19.60), (105.80, 20.10), (106.50, 20.30), (106.80, 20.72),
    (107.35, 21.02), (107.95, 21.50), (108.05, 21.55), (108.35, 21.62), (108.62, 21.87),
    (109.10, 21.48), (109.60, 21.52), (110.05, 21.00), (110.20, 20.30), (110.60, 20.40),
    (110.50, 21.10), (111.05, 21.45), (112.00, 21.68), (112.20, 21.75),
    (113.50, 22.20), (113.50, 28.80), (103.00, 28.80),
]

# 河流： (名称, [(经度,纬度)...], 河床高度)。河床低于 WATER_Z 即形成水面，天然渡口由地形决定
# 各支流的端点与干流共用同一坐标 -> 水系连成一张网（船可以从干流开进支流）
RIVERS = [
    ('红水河-黔江-浔江-西江', [(107.10, 24.62), (107.75, 24.30), (108.35, 23.95), (108.90, 23.75),
                              (109.23, 23.73), (109.70, 23.45), (110.10, 23.40), (110.65, 23.45),
                              (111.28, 23.48), (111.95, 23.40)], 178.0),
    ('郁江-邕江',            [(106.60, 23.92), (107.20, 23.55), (107.70, 23.10), (108.32, 22.82),
                              (108.90, 22.95), (109.35, 23.10), (109.70, 23.30),
                              (109.70, 23.45)], 182.0),          # 东端接到西江干流
    ('柳江',                 [(109.30, 25.35), (109.42, 24.85), (109.40, 24.33), (109.30, 23.95),
                              (109.23, 23.73)], 185.0),          # 南端接西江（来宾）
    ('桂江-漓江',            [(110.35, 25.60), (110.29, 25.28), (110.45, 24.85), (110.60, 24.55),
                              (110.95, 24.10), (111.28, 23.48)], 184.0),   # 南端接西江（梧州）
    ('左江',                 [(106.90, 22.75), (107.36, 22.40), (107.85, 22.55), (108.32, 22.82)], 206.0),  # 干谷（可通行）
    ('贺江',                 [(111.55, 24.42), (111.60, 24.05), (111.45, 23.70), (111.28, 23.48)], 206.0),  # 干谷（可通行）
    ('南流江',               [(109.70, 23.30), (110.18, 22.63), (109.95, 22.25), (109.60, 21.90),
                              (109.42, 21.60), (109.30, 21.25), (109.25, 20.90),
                              (109.20, 20.55), (109.18, 20.20)], 180.0),   # 穿过海岸沙堤入北部湾
    ('北仑河',               [(107.36, 22.40), (107.60, 21.90), (108.05, 21.60), (108.30, 21.35),
                              (108.45, 21.05), (108.50, 20.70), (108.52, 20.30)], 206.0),  # 干谷（可通行）
]

# 桥梁： (河流名, 沿线比例 0..1)。桥段沿垂直河流方向放在两岸，让陆军能过河。
#   桥段对象名经本机游戏数据核实：Bridge1a / Bridge2a 是原版 RA3 桥（官方图在用）。
BRIDGE_TYPE = 'Bridge2a'
# 中路（地图中部河网交汇区）必须有 2~3 座**南北向**的桥，方便南北两队互通；
# 两侧再各留 1 座，总共 5 座（每座 2 个桥段）。
BRIDGES = [
    ('红水河-黔江-浔江-西江', 0.26),      # 中路（西段）
    ('红水河-黔江-浔江-西江', 0.44),      # 中路（正中）
    ('郁江-邕江', 0.52),                  # 中路（南侧支流）
    ('柳江', 0.52),                       # 北侧
    ('红水河-黔江-浔江-西江', 0.74),      # 东侧
]

# 各地级市的山地强度（0=平原, 1=高山）
CITY_AMP = {
    '百色市': 1.00, '河池市': 0.95, '桂林市': 0.78, '贺州市': 0.82, '柳州市': 0.55,
    '来宾市': 0.34, '崇左市': 0.62, '南宁市': 0.30, '贵港市': 0.28, '玉林市': 0.40,
    '梧州市': 0.60, '北海市': 0.12, '钦州市': 0.24, '防城港市': 0.52,
}
DEFAULT_AMP = 0.35          # 落在省内但未匹配到城市时
OUTSIDE_AMP = 0.70          # 所有省界之外（如越南边境山区）的陆地强度

# 地形塑形参数（可选；缺省用工具内默认值）。key 与 tools/terrain_gen.py 的 TERRAIN_DEFAULTS 对应
TERRAIN = {
    # ---- 蓝图来源：True = 用用户手改的 v11 分层图（maps/bagui_dadi/v11_layers.npz）
    #      五层：海 190 / 平 212 / 一层 280 / 二层 350 / 三层 420 / 高山 480（不可通行）
    'use_v11_layers': True,
    'v11_plain': True,                      # 预览定稿分层就是最终地形
    'v11_smooth': 0,
    # 斜坡边长。划线附近的坡挪到线上，方向跟线一致。
    'ramp_size': 25.0,
    'ramp_layout': 'face',
    'ramp_gap': 120.0,
    'ramp_max_per_edge': 5,
    'ramp_marks': (
        (117, 154, 15),
        (774, 146, -34),
        (588, 170, -91),
        (487, 246, -127),
        (738, 278, 5),
        (354, 375, -110),
        (510, 391, -131),
        (590, 423, -83),
        (695, 425, -92),
        (326, 486, 6),
        (678, 491, -119),
        (378, 525, 3),
        (176, 577, -87),
        (686, 603, -97),
        (103, 633, -107),
        (424, 638, 0),
        (759, 707, -127),
        (247, 714, -51),
        (525, 718, -69),
        (102, 750, -31),
        (244, 786, -91),
    ),
    'lat_bias': 0.50,
    # 出生点：预览上的数字 1–8。北侧四人，南侧 P5、P6 是人类（P6 在海上），P7、P8 是固定 AI。
    'base_cells': [
        (188, 66),     # P1 西北，三层
        (362, 68),     # P2 北，三层
        (535, 62),     # P3 中北，三层
        (700, 170),    # P4 东北，二层
        (150, 738),    # P5 西南，一层
        (750, 756),    # P6 东南，水上
        (302, 664),    # P7 南，二层，固定 AI
        (562, 663),    # P8 南部，一层，固定 AI
    ],
    'base_clear_want': 8.0,                 # 落点周围同层净空（格）：够摆基地即可，不改地形
    # 定稿高度里没有这些旧河线。清掉河距，水上圈才不会被一条不存在的河推走。
    'base_river_clearance': 0.0,
    # ---- 矿位：往开阔地方摆
    'ore_open_rank': True,                  # 候选里挑"开阔度最大"的位置
    # ---- 矿位分级（用户要求）：基地旁只留 2 个，其余 2 个推到更远处/下一层
    'ore_near_max': 2,                      # 每个基地 55 格内最多几个矿
    # 基地旁矿位的可用区间：别贴基地（否则建造场会被矿场卡住），也别太远
    'ore_near_min_r': 44.0,                 # 近矿离基地**至少** 44 格（约一个建造场半径）
    'ore_near_max_r': 62.0,                 # 近矿最远 62 格
    'ore_far_min_dist': 75.0,               # 其余矿至少离基地这么远
    'ore_far_fallback': 58.0,               # 实在放不下时的放宽距离
    'ore_far_pull_mid': 0.45,               # 远矿朝地图中部拉的强度（0=不拉，1=直接摆到中部）
    # ---- 中部设施（用户要求）：2 哨站 + 1 车库 + 1 学院 + 1 医院 + 1 机场
    #      用格坐标指定（x, y），会在地形定稿后自动吸附到附近最平整的地方
    'center_posts': [(348, 452), (564, 380)],           # 观察哨 x2
    'center_garage': [(452, 292)],                      # 车库
    'center_academy': [(308, 564)],                     # 精兵学院
    'center_hospital': [(612, 532)],                    # 医院
    'center_airport': [(484, 388)],                     # 机场
    'ore_in_water': True,                   # 水上出生点的近矿允许直接放在水面（保证"至少 2 矿"）
    'ore_water_edge_min': 4.0,              # 水上矿离岸的最小距离（格）
    'ore_open_want': 20.0,                  # 期望开阔半径（格）：到不可通行/陡坎/水的最小距离，与矿场圆盘一致
    'ore_edge_clear': 20.0,                 # 矿心离层界（格）。层坎在建造圆盘内就摆不下矿场
    'ore_shore_clear': 20.0,                # 矿心离水陆交汇（格）
    'ore_border_clear': 30.0,               # 矿心离可玩边界（格）。再近，矿场会压进图边禁建带
    'ore_mouth_r1': 20,                     # 矿口射线长度（格），与建造圆盘半径一致
    # 外矿打平后会背向最近基地。这两处更长的净空在另一侧，按格子指定斜向（+y 向南）。
    'ore_face_at': (
        (332, 528, -1, 1),                  # 矿32 朝南西
        (223, 587, 1, 1, 327, 172),         # 矿35 放回原格，朝南东
    ),
    'ore_flat_soft': 3.0,                   # 开阔地内部允许的坡度上限
    'noise_mount': (26, 5, 12345),     # (基础波长, 倍频数, 随机种子)
    'noise_detail': (7, 4, 4242),
    'coast_cells': 52.0,               # 距海岸多远算“沿海低地”
    'river_cells': 36.0,               # 距河流多远算“河谷”
    'mount_gain': (14.0, 230.0),       # 山地增益 (基础, 按城市强度)；配合平滑形成渐变坡
    'hills_gain': (1.5, 4.0),          # 平原起伏（除连片高山外几乎全部铺平）
    'outside': (26.0, 18.0, 300.0, 90.0),   # 省界外陆地（邻国山区）的高度基准
    'sea': (18.0, 78.0, 90.0, 6.0),         # 海面 (浅滩, 最深处, 跨度, 细节)
    'river': (3, 22.0, 2.6, 120.0, 5.0),    # (线宽, 河谷跨度, 坡度系数, 河谷深度, 河道半宽=5格/100单位宽)
    'river_bank': (4.0, 0.0, 6.0),          # 河岸：(岸坡宽, 平坦可建岸宽, 接回地形缓坡宽) —— 河道变小，不再留建筑平台
    'bank_above': 10.0,                     # 平坦河岸高出水面的高度 = 平原层 210
    'bridge_causeway': 0,                   # 0 = 桥下保留水面（水系统一）；>0 = 桥下做一条过河堤道
    'bridge_drop': 2.0,                     # 堤道比岸顶低多少（世界单位，仅 causeway>0 时用）
    'bridge_span_cells': 26.0,              # 桥段到桥心距离上限（格）≈ 官方 490~520 世界单位
    'bridge_min_off': 24.0,                 # 桥段至少离桥心这么远
    'bridge_seg_step': 24.5,                # 桥段沿桥轴的铺开步长（格）≈ 官方 490/2
    'bridge_max_water': 46.0,               # 水面宽超过此值（跨海/大湾）不架桥
    'bridge_margin_cells': 7.0,             # 桥段向两岸各多铺这么多格，保证"盖过"河面
    'smooth': (3, 2),                       # 整体平滑：大面积为缓坡/平地，山地是渐变坡
    # ---- 高度分层（世界单位）：海面 200 / 平原 210 / 一层地面 280 / 二层地面 320（最高 320）
    #      陆地高度一律量化到这三层；层间用缓坡过渡，保证各层都能通过部队
    'levels': (210.0, 280.0, 350.0, 420.0),   # 平原/一层/二层/三层(新增)        # 海面200 / 平原210 / 一层280 / 二层350（最高350）
    'level_ramp': 5,                        # 层间缓坡迭代：70 单位落差摊到 ~11 格 => 坡度 ~6.4 (<10 即 <45°)
    'level_presmooth': 3,                   # 量化前的粗平滑：让同一层的区域更连片
    # ---- 二层连片（规则见 docs/ra3_map_format_spec.md §19.10）：
    #      把二层(350)高地之间散落的一层(280)地面整片并成二层，让高地连成一片。
    #      多尺度保护闭运算：≤2*radius 格的缝隙被整片抹平；只作用于离平原/海 >= radius 格的内部缝隙，
    #      且该缝隙必须把 >=2 个面积 >= min_block 的一层地块连起来；单块面积不限（整块抹平口径）。
    # ---- 二层连片 / 层规则（规则见 docs/ra3_map_format_spec.md §19.10）
    #      口径：边界平滑(多数投票) + 删孤立块(按面积整块降一级) + 连片(闭运算) + 收尾平滑
    #      参数由「逆向拟合用户手改的 layers5_preview_v11.png」得到：
    #      与 v11 逐格一致率 0.846、二层 IoU 0.668（v10 基线仅 0.788 / 0.483）
    'level_rule': True,
    'level_rule_smooth_r': 5,               # 边界平滑窗口半径（(2r+1)² 多数投票）
    'level_rule_smooth_passes': 1,          # 平滑轮数
    'level_rule_min_area': {350.0: 1600, 420.0: 2400, 280.0: 0, 210.0: 0},  # 第一遍删孤立块
    'level_rule_close_r': {350.0: 40, 280.0: 10, 420.0: 10},                # 连片（闭运算半径）
    'level_rule_prune2_area': {350.0: 5000, 420.0: 0, 280.0: 0},            # 第二遍删孤立块
    'level_rule_final_maj_r': 4,            # 收尾平滑（磨圆连片产生的方形边界）
    'level_rule_sea_close': 0,              # 海域填充（0=关）
    'level_rule_sea_min': 0,
    'level_rule_verbose': True,
    # 兼容旧字段（连片半径；level_rule=True 时不再使用）
    'level_join_radius': 0,
    'level_join_min_block': 400,
    'level_join_low': (210.0,),             # 保护带参照的"低地"层
    # 检测口径：expand = 二层掩码外扩 radius 格以内的一层地面（预览图口径，边界会外推）；
    #           closing = 只填闭运算露出的窄缝（语义最干净，不外扩）
    'level_join_mode': 'expand',
    'level_join_rounds': 8,                 # 迭代轮数：每轮把已填缝隙并入掩码后再来一轮
    'level_join_verbose': True,
    'slope_max': 10.0,                      # 坡度上限（世界单位/格）：10 = 45 度
    'slope_relax': 28,                      # 坡度限制器迭代次数（越大越彻底，陡坎被削成缓坡）
    'city_pad': (26.0, 20.0),               # 城市平台 (半径, 过渡)：城市建在平地上，也让城市矿脉有平整落点
    # 只保留连片/大面积高山，零散小山全部铺平
    'mountain_relief': 60.0,                # 高出平原这么多（世界单位）才算山
    'mountain_radius': 14,                  # 山体小于该半径（格）就铺平
    'mount_bands': (0.34, 0.66),            # 只保留东西两侧的山脉：x < 0.34*GRID（西）或 x > 0.66*GRID（东），中间铺平
    # 矿脉必须放在平整的地方：坡度上限 + 离山脚（陡坡）的最小距离
    'ore_flat_slope': 2.0,
    'ore_steep_slope': 1.0,                 # 分层地形下：任何不平整处（层间缓坡/岸坡）都算山脚
    'ore_mount_clearance': 24.0,            # 矿位离山脚的最小距离（须 > 矿区建造范围 18）
    'ore_river_clearance': 36.0,            # 矿位离河道中心线净距（须 > 河道半宽12 + 矿场核心18，否则补铺河道会切进平台）
    'pad': (62.0, 46.0, 350.0),             # 出生点平台：核心 62 格；过渡 46 格（边缘不再是明显的圆）
    'res_pad': (18.0, 14.0),                # 矿位平台 (半径, 过渡)：核心 18 格(360单位)全平，够放矿场+矿车通道
    'ore_mouth_flat': 20.0,                 # 矿场摆放位**强制平整**半径（矿口方向半圆盘，须 >= 矿场核心 18）
    'p4_plateau': (150.0, 60.0, 420.0),   # 西北成片山体 -> 420 层大平台      # P4(百色) 旁的高山做成更高一层的可通行平台（半径/过渡/层高）
    'ore_mouth_flat_soft': 4.0,             # 强制平整边缘过渡格数（避免硬台阶）
    'sea_cut_lon': 108.30,                  # （备用）该经线以东的省外海域才算海
    # 中越边界折线（海岸+陆界，按纬度插值成一条斜线）：其东南侧算海，西北侧按陆地补全
    'sea_border': [(108.05, 21.55), (107.90, 21.40), (107.25, 21.00), (106.90, 20.75),
                   (106.75, 22.05), (106.85, 22.40), (106.40, 23.10), (105.70, 23.50)],
    # ---- 新规则：海域拉平；平台精确层高 + 少量斜坡口；高山不可通行
    'sea_flat': 130.0,                      # 贴岸海底。外海由通用层高落到 0
    'v11_skip_water': True,                 # v11 模式：跳过"西侧填湾"（保留 v11 图里的海）
    'v11_skip_peak': True,                  # v11 模式：跳过 DEM 高山层（v11 里已有 480 高山层）
    'peak_from_m': 850.0,                   # DEM 海拔超过这个高度 -> 高山层
    'peak_level': (480.0, 1000.0),          # 高山层高度范围（不可通行）
    'ramp_openings': 2,                     # 每个平台留几个可通行斜坡口
    'ramp_half_width': 5.0,                 # 坡面半宽（格）→ 全宽 11 格，够部队与矿车通行
    'ramp_len': 14.0,                       # 坡长（格）：70 单位落差摊到 14 格 ≈ 5 单位/格（缓坡）
    # ---- 层间斜坡（spec §19.37）：不同层之间必须能走上去
    'level_ramps': True,                    # 是否挖层间斜坡
    'ramp_per_px': 6000.0,                  # 每多少格交界面积开 1 个斜坡（交界越大开越多）
    'ramp_max_per_patch': 6,                # 单块交界最多开几个
    'ramp_min_patch': 40,                   # 小于这个格数的交界片忽略
    'ramp_clear': 2.0,                      # 保留参数
    'ramp_bias_mid': 1.0,                   # 位置偏好：离地图中部（两组交汇点）越近越好
    'ramp_bias_player': 0.5,                # 位置偏好：离最近出生点越近越好
    'ramp_bias_cap': 260.0,                 # 超额斜坡只在中部/玩家附近开

    'z_clip': (60.0, 800.0),                # 相对 LAND_Z 的上下限：上限放宽到 210+800（容纳高山层 400~1000）
    'wall': (2.0, 70.0),                    # 最外圈挡墙 (格数, 高度)：抬到 280（一层），落差只 70
    'sw_plain': (0.30, 0.30),               # (x 上限比例, y 上限比例)：该矩形内为平原陆地
    'west_band': 0.46,                     # 西侧 46% 宽度内不要任何水域
    'sw_band': (0.46, 0.40),                # 西南角 (宽比, 高比) 也全部陆地
    'sea_south_margin': 3,                  # 省界最南端以南再 N 格即算海（做大南部水域）
    'lat_bias': 0.50,                       # 广西竖向正居中（0.5 = 上下对称）
}

# ---------------------------------------------------------------- 地表纹理
# 按地形层铺：一层到四层各一张。不沿用旧图按层混草地的刷法。
LAYER_TEX_PRESET = 'mountain'
LAYER_TEX_STRICT = True
CELL_SIZE = 4              # 纹理块尺寸（(CELL_SIZE*2)^2 格/张，与实机地图一致）
# (BlendTileData 名称, WorldInfo 漫反射资源名, 法线资源名) —— 均取自本机可正常运行的社区地图
TEX_SET = [
    # 草地：改用游戏里**鲜绿**的草地族 —— CapeCod(温带浓绿) + Golf(球场嫩绿) + Geneva(欧洲绿)，
    # 原来的 Grass_Yucatan* 偏橄榄/热带，不够绿。
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

# ---------------------------------------------------------------- 水面
WATER = dict(name='New Water Area', uv_scroll=0.06, additive=0,
             bump='WaterRippleBump', sky='SkyEnv', fx='FXOceanRA3',
             depth_colors='LUTDepthTint.tga', height=None,   # None => WATER_Z
             margin=200.0)                                  # 水面多边形外扩

# ---------------------------------------------------------------- 地图文本（map.str，UTF-8）
# {version} / {name} 会在构建时替换成当前版本号与带版本的发布名
MAP_STR_INTRO = '"欢迎来到[CHAI]八桂大地，祝大家玩得开心。"'
MAP_STR_DEBUG = '"北侧 Player_1-4 为一队；南侧 Player_5-6 与两个固定 AI（Player_7、Player_8）为一队"'
MAP_STR_INFO = {
    '0_1': 'P1 西北(北队)', '0_2': 'P2 北(北队)', '0_3': 'P3 中北(北队)',
    '1_0': 'P4 东北(北队)', '1_1': 'P5 西南(南队)', '1_2': 'P6 东南(南队·水上)',
    '1_3': 'P7 南(固定AI)', '2_0': 'P8 南部(固定AI)',
    '2_1': '北队自西向东：P1-P2-P3-P4', '2_2': '南队：P5、P6 + 固定位 P7、P8',
    '2_3': '南部北部湾为海战区域',
    '3_0': '地图作者：[CHAI]', '3_1': '蓝图：广西壮族自治区', '3_2': '版本 {version}',
    '3_3': '祝游戏愉快',
}

# ---------------------------------------------------------------- 预览图
PREVIEW_TITLE = '{name}'
PREVIEW_SUBTITLE = '红色警戒3 · 8人4v4 · 广西壮族自治区地形蓝图 · 1000×1000 大型 · {version}'
PREVIEW_LABELS = {          # waypoint 名 -> (标注, 队伍 W/E)：出生点只标 Player1~8
    'Player_1_Start': ('Player1', 'W'), 'Player_2_Start': ('Player2', 'W'),
    'Player_3_Start': ('Player3', 'W'), 'Player_4_Start': ('Player4', 'W'),
    'Player_5_Start': ('Player5', 'E'), 'Player_6_Start': ('Player6', 'E'),
    'Player_7_Start (imported)': ('Player7', 'E'),
    'Player_8_Start (imported)': ('Player8', 'E'),
}
PREVIEW_FOOTER = ('北队（自西向东）：Player1 / Player2 / Player3 / Player4      '
                  '南队：Player5 / Player6 + 固定位 Player7、Player8（后两位为固定 AI）')

# ---------------------------------------------------------------- 对象布局
# 出生点 waypoint 名由 BASES 自动推导：human 依次为 Player_1_Start…，固定位接续编号并加
# "(imported)" 后缀（游戏不会把它们列进房间可选位，由地图脚本固定生成）。
# 矿、油井、中部设施的落点在 TERRAIN。这里只留建造场、矿位条数和装饰。
# 固定 AI 的建造场： (对象类型, 依附的 waypoint, dx格, dy格, 吸附半径)
YARD_PLACEMENT = [
    ('SovietConstructionYard', 'Player_7_Start (imported)', 2, -3, 20),
    ('AlliedConstructionYard', 'Player_8_Start (imported)', -2, 3, 20),
]
# 每个出生点 4 个矿：
#   近矿 2 个 —— 必须落在**基地建造范围内**。实测 462 张社区图，出生点到最近矿脉
#                中位数 331、p90 432 世界单位（≈33~43 格），故取 ~37 格（370 单位）。
#   远矿 2 个 —— 更远，必须挪基地/再起矿场才能采（~121/125 格）。
BASE_ORE_NEAR = ((-34, -14), (36, -12))                   # 离基地 ~37/38 格 = 368/379 单位（建造范围内）
BASE_ORE_FAR = ((-96, -74), (104, -70))                   # 离基地 ~121/125 格（需拓展）
BASE_ORE_OFFSETS = BASE_ORE_NEAR + BASE_ORE_FAR            # 兼容旧接口（顺序：近2 + 远2）
ORE_NEAR_MAX_DIST = 100.0                                 # 「出生地 100 范围内」的判定半径（格）

# ---- 矿脉新布局：每名玩家 4 个 + 中路争夺 4~6 个，全部按斜向「绕圈」摆放 ----
#   每名玩家：近 2 个（基地可直接建造，落在建造范围内）+ 周边 2 个（需拓展建造范围）
#   4 个矿分在 4 个斜向上、矿口一律朝远离基地的方向
#   -> 矿场建造区互不重叠，矿也不并排（交叉/绕圈摆放）
BASE_ORE_DIRS = ((0.7071, -0.7071), (-0.7071, -0.7071),
                 (0.7071, 0.7071), (-0.7071, 0.7071))     # 东北/西北/东南/西南（屏幕坐标 y 向下）
ORE_NEAR_R = 38.0                                         # 近矿半径（格）≈380 单位，在建造范围内
ORE_MID_R = 78.0                                          # 周边矿半径（格）≈780 单位，需拓展
CENTER_ORE_COUNT = 6                                      # 中路零散争夺矿数量（4~6）
# 中路矿沿**东西方向**均匀铺开（占可玩区宽度的比例），y 交错摆动避免排成一条直线
CENTER_ORE_XS = (0.20, 0.32, 0.44, 0.56, 0.68, 0.80)
CENTER_ORE_YS = (0.62, 0.38, 0.56, 0.44, 0.60, 0.40)     # 占可玩区高度的比例
# 观察哨：放在东西方向的 1/3 与 2/3 处（格坐标，超出会被就近吸附到合法地面）
POST_CELLS = [(286, 420), (554, 420)]
# 矿贴山脚：矿心到"层边界/陡坎"的距离范围（格）与矿口方向的净空要求
ORE_FOOT_MIN = 6.0                                        # 矿心到层边界的距离：缓坡带之外、又紧贴坡脚
ORE_FOOT_MAX = 16.0
ORE_MOUTH_CLEAR = 18.0                                    # 矿口方向需要的平整净空（格）
# 每个出生点 2 个油井（格偏移，离基地 30~38 格，位于基地建造范围内但不贴基地）
BASE_DERRICK_OFFSETS = ((34, 0), (-34, 0), (0, 34), (0, -34),
                        (26, 26), (-26, 26), (26, -26), (-26, -26))
ORE_LEFT_RING = (260.0,)          # 多余矿脉沿外环铺开（半径，格）
DECOR_POS = [(106.90, 24.30), (107.60, 24.90), (108.60, 25.20), (110.90, 25.20),
             (111.20, 24.85), (111.75, 23.90), (110.95, 22.85), (110.05, 22.05),
             (109.05, 21.80), (108.15, 22.05), (107.00, 22.90), (106.60, 22.30),
             (108.55, 24.90), (111.10, 23.15)]
DECOR_TYPES = ('StPetersburgSidewalk03', 'EI_Structure_02', 'EI_Structure_01',
               'EI_Structure_02_Collapse', 'EI_Structure_01_Collapse',
               'GarageTechStructure_Collapse', 'NUObservationPostTechStructure_Collapse',
               'NBOilDerrick_Collapse')
PAD_CLEARANCE = 46.0      # 装饰物与出生点平台的最小距离（格）
RESOURCE_PAD_CLEARANCE = 30.0     # 油井/观察哨/车库与出生点的最小距离（格）
# 摆放互斥：基地与矿/资源互不重叠
ORE_BASE_CLEARANCE = 30.0        # 矿脉与出生点中心的最小距离（近矿要落在建造范围内，故不能太远）
ORE_MIN_SPACING = 46.0           # 矿脉之间的最小间距（格，> 2*平台核心半径 36，保证核心互不重叠）
RESOURCE_MIN_SPACING = 32.0      # 油井/观察哨/车库彼此的最小间距（格）
DECOR_RESOURCE_CLEARANCE = 10.0  # 装饰物与资源点的最小距离（格）
# 矿优先级最高：矿的“建造范围”内不允许出现任何建筑、资源（含油井/车库/机场/医院/精兵学院/雷达）
# 与山脉；海洋不影响摆放矿场，所以允许。其他资源解算时会强制与该范围保持净距。
ORE_CLEAR_RADIUS = 20.0          # 矿场建造圆盘半径（格）= 200 单位。绕矿点整盘都要平，不只是矿口前方
# 每个玩家 2 个油井：总共需要的油井数量（模板自带 8 个，不足的由工具克隆补齐）
DERRICKS_PER_BASE = 2
