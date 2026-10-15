# -*- coding: utf-8 -*-
"""通用分层高度。

配方写 HEIGHT_PRESET = 'standard' 时套用。没写的图仍用自己的 levels、sea_flat，
一层仍是原来的高度，不会被这组改掉。
"""

LEVELS = (210.0, 280.0, 350.0, 420.0)
# 外海到贴岸。贴岸保持高值，斜坡和水上基地用这个高度。
WATER_RANGE = (0.0, 130.0)
# 五层山峰，不可通行，不接斜坡。
PEAK_RANGE = (500.0, 1000.0)
# 高于四层、已经够不上 420 平台的高度，归进五层。
PEAK_FROM = 450.0
SHORE_SHELF = 80.0
DEEP_SPAN = 220.0


def enabled(T=None):
    return (T or {}).get('height_preset') == 'standard'


def apply(cfg):
    """把通用层高写进这张图的 TERRAIN。没开预设则什么都不改。"""
    if getattr(cfg, 'HEIGHT_PRESET', None) != 'standard':
        return False
    T = cfg.TERRAIN
    T['height_preset'] = 'standard'
    T['levels'] = LEVELS
    T['water_range'] = WATER_RANGE
    T['peak_range'] = PEAK_RANGE
    T['sea_flat'] = float(WATER_RANGE[1])
    return True


def play_levels(T=None):
    """可建陆地层。开了预设用 210 起，否则保持原来的 212 起。"""
    if enabled(T):
        return LEVELS
    return (212.0, 280.0, 350.0, 420.0)
