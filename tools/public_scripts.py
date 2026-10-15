# -*- coding: utf-8 -*-
"""公共脚本。组装时接到每张图的公共列表上。

每张图都列装：按出生位置分队、左侧只显示电脑现金、电脑第二座产能。
可选脚本写在下面，默认不列装。配方把对应开关写成 True 才装。
新图不要打开这些开关，除非这张图明确要装。
"""

# (配方开关, 模块, 函数)。开关缺省或为假时不列装。
OPTIONAL = (
    ('TEAM_ATK', 'team_atk', 'team_atk'),
    ('ORE_EXPAND', 'ore_expand', 'ore_expand'),
)

# 模板里重写外交的条目。分队脚本每秒重申关系，这些留着会抢同一件事。
_DUP_DIPLOMACY = (
    'DPR Player 1', 'DPR Player 2', 'DPR Player 3',
    'DPR Player 4', 'DPR Player 5', 'DPR Player 6',
    'FAPR Player 1', 'FAPR Player 2', 'FAPR Player 3',
    'FAPR Player 4', 'FAPR Player 5', 'FAPR Player 6',
    'PRC Bonus Enemy', 'PRC Bonus Friend',
    'PRC Player 1 Enemy', 'PRC Player 1 Friend',
    'PRC Player 2 Enemy', 'PRC Player 2 Friend',
    'PRC Player 3 Enemy', 'PRC Player 3 Friend',
    'PRC Player 4 Enemy', 'PRC Player 4 Friend',
    'PRC Player 5 Enemy', 'PRC Player 5 Friend',
    'PRC Player 6 Enemy', 'PRC Player 6 Friend',
)

# NKL blizzard 里和第二座产能、分队做同一件事的条目。
# 基地车、第一座产能、发钱、防守不在这里。
_DUP_NKL = (
    'P5 S 2AIR', 'P6 S 2AIR', 'P5 R2 2AIR', 'P6 R2 2AIR',
    'p5 2air', 'p6 2air', 'P8 2air',
    'P8 YARD', 'R1 P8 Yard', 'R2 P8 Yard',
    'R1 P8 2be 1', 'R2 P8 2be 1', 'R1 P8 2be', 'R2 P8 2be',
    'P5RE4 2be', 'P6RE4 2be', 'p6 out 2be',
    'AI Relationship', 'AI relationship',
    'J MAP AI', 'Allied5', 'Allied6', 'J 5', 'J 6', 'J AI honor',
)

# 固定 AI 的旧科技计时。FORCE_SEA_AI 已经按同一套时间升科技。
_DUP_SEA_AI = (
    'P7 T3', 'P7 T3__1', 'P8 T3 1',
    'P7 T3 1', 'P7 T3 4re', 'P7 T3 1_sup',
    'R1 P8 T2', 'R2 P8 T2', 'P8 T3',
)

# NKL 进攻波。只有列装了对位进攻才删，否则这张图没有别的进攻脚本。
_DUP_TEAM_ATK = (
    'P1 2WAR', 'P2 2WAR', 'P3 2WAR', 'P4 2WAR', 'P2 3WAR',
    'P5 2WAR', 'P5 2WAR__1', 'P6 2WAR', 'P6 2WAR__1',
    'Find p5 war1', 'Find p6 war1', 'P6RE4_2war', 'p6 a2war',
    'P7 2WAR', 'Find p7 war1', 'P8 2 war', 'R1 P8 2WAR', 'R2 P8 2WAR',
    'high house attck', 'high house attck stop',
)


def duplicate_names(cfg):
    """每张图都要删的重复脚本。没接 NKL 的图只删模板外交。"""
    names = list(_DUP_DIPLOMACY)
    if getattr(cfg, 'NKL_SCRIPTS', None):
        names.extend(_DUP_NKL)
        if getattr(cfg, 'FORCE_SEA_AI', False):
            names.extend(_DUP_SEA_AI)
        if getattr(cfg, 'TEAM_ATK', False):
            names.extend(_DUP_TEAM_ATK)
    return tuple(names)


def apply(mmap, cfg):
    from team_sides import team_sides
    team_sides(mmap)
    from ai_money import ai_money
    ai_money(mmap)
    from dual_prod import dual_production
    dual_production(mmap, float(getattr(cfg, 'WATER_Z', 200.0)))
    for flag, module, func in OPTIONAL:
        if not getattr(cfg, flag, False):
            continue
        mod = __import__(module)
        getattr(mod, func)(mmap, cfg)
