# -*- coding: utf-8 -*-
"""山地分层的通用地表。

配方写 LAYER_TEX_PRESET = 'mountain'，且没有自己的 LAYER_TEX 时，
按该图 LAYER_M 从低到高各贴一张。已经写了 LAYER_TEX 的图不改。
"""

# (BlendTile 名, 漫反射, 法线)。顺序即一层到四层。
MOUNTAIN_LAYERS = (
    ('Grass_NewYork01', 'TGrass_NewYork01', 'TGrass_NewYork01_NRM'),
    ('Grass_MtRush04', 'Grass_MtRush04', 'Grass_MtRush04_NRM'),
    ('Dirt_Iceland05', 'TDirt_Iceland05', 'TDirt_Iceland05_NRM'),
    ('Pavement_Amsterdam05', 'TPave_Amsterdam05', 'TPave_Amsterdam05_NRM'),
)
def land_levels(cfg):
    """该图陆地高度，从低到高。优先 LAYER_M，其次配方里的 levels。"""
    layer_m = getattr(cfg, 'LAYER_M', None) or ()
    if layer_m:
        return [float(z) for _ceil, z in layer_m]
    levels = (getattr(cfg, 'TERRAIN', {}) or {}).get('levels') or ()
    return [float(z) for z in levels]


def apply_mountain_preset(cfg):
    """套上山地四层纹理。返回是否改了这份配方的内存。"""
    if getattr(cfg, 'LAYER_TEX_PRESET', None) != 'mountain':
        return False
    changed = False
    if not getattr(cfg, 'LAYER_TEX', None):
        levels = land_levels(cfg)
        if not levels:
            raise RuntimeError('山地纹理需要配方里的 LAYER_M')
        cfg.LAYER_TEX = {
            levels[i]: (MOUNTAIN_LAYERS[i][0],)
            for i in range(min(len(MOUNTAIN_LAYERS), len(levels)))
        }
        if not hasattr(cfg, 'LAYER_TEX_STRICT'):
            cfg.LAYER_TEX_STRICT = True
        have = {e[0] for e in cfg.TEX_SET}
        extra = [e for e in MOUNTAIN_LAYERS if e[0] not in have]
        if extra:
            cfg.TEX_SET = list(cfg.TEX_SET) + extra
        changed = True
    return changed
