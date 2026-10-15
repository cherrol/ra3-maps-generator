# -*- coding: utf-8 -*-
"""生成发布包里的地图说明 assets/<地图名>/README.md。

内容 = 自动部分（版本、基本信息、自检数据、出生点表、版本历史、安装说明）
      + 配方里的散文 maps/<id>/README.md（若存在）。

用法：  python tools/make_readme.py [--map <map_id>]
"""
import argparse
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

from ra3map import MapFile, parse_heightmap, parse_blendtiles, parse_objects, props_dict
import project as proj

TEAM_LABEL = {'W': '西队', 'E': '东队'}


def collect_stats(P):
    """从成品地图里读回自检数据。"""
    mappath = P.asset(P.map_name + '.map')
    if not os.path.isfile(mappath):
        return None
    m = MapFile.load(mappath)
    hm = parse_heightmap(m.get('HeightMapData'))
    W, H = hm['width'], hm['height']
    z = [v * P.Z_SCALE for v in hm['elevations']]
    bt = parse_blendtiles(m.get('BlendTileData'), W, H, m.table)
    imp = bt['impassability']
    impassable = sum(sum(r) for r in imp)
    water = sum(1 for v in z if v < P.WATER_Z)
    objs = parse_objects(m.get('ObjectsList'), m.table)
    from collections import Counter
    counts = Counter(o['type_name'] for o in objs)
    ways = []
    for o in objs:
        if 'Waypoint' in o['type_name']:
            d = props_dict(o['props'])
            ways.append((d.get('waypointName', ''), o['position'][:2]))
    return dict(dims=(W, H), border=hm['border_width'], bounds=hm['borders'],
                zmin=min(z), zmax=max(z), water_pct=100.0 * water / (W * H),
                impassable_pct=100.0 * impassable / (W * H),
                textures=len(bt['textures']), objects=len(objs), counts=counts, ways=ways,
                size=os.path.getsize(mappath))


def build_readme(P):
    st = collect_stats(P)
    L = []
    L.append('# %s' % P.map_name)
    L.append('')
    L.append('红警3（Red Alert 3）自定义地图 · **版本 %s**（%s）' % (
        P.version_string, '正式版' if P.version['stage'] == 'V' else '测试版'))
    L.append('')
    L.append('![地图预览](preview.png)')
    L.append('')

    L.append('## 基本信息')
    L.append('')
    L.append('| 项目 | 内容 |')
    L.append('|---|---|')
    L.append('| 地图名（游戏内） | `%s` |' % P.map_name)
    L.append('| 版本 | `%s`（%s） |' % (P.version_string,
                                        '正式版' if P.version['stage'] == 'V' else '测试版'))
    if st:
        pw = st['dims'][0] - 2 * st['border']
        L.append('| 尺寸 | 可玩区 %d×%d 格（%d×%d 世界单位），高度图 %d×%d，边框 %d 格 |' % (
            pw, pw, pw * int(P.CELL), pw * int(P.CELL), st['dims'][0], st['dims'][1], st['border']))
    L.append('| 配方 | `maps/%s/config.py` |' % P.map_id)
    L.append('| 骨架 | `%s` |' % os.path.basename(P.template or '—'))
    L.append('| 文件 | `%s.map`、`%s_art.tga`、`map.str` |' % (P.map_name, P.map_name))
    L.append('')

    bases = list(getattr(P.cfg, 'BASES', []) or [])
    if bases:
        L.append('## 出生点')
        L.append('')
        L.append('| 位号 | 名称 | 队伍 | 类型 |')
        L.append('|---|---|---|---|')
        # 与 waypoint 名按顺序对应：Player_1..Player_N
        for i, b in enumerate(bases, 1):
            lon, lat, name, team, kind = b
            L.append('| Player_%d | %s | %s | %s |' % (
                i, name, TEAM_LABEL.get(team, team),
                '固定位（房间不显示，脚本生成）' if kind != 'human' else '房间可选位'))
        L.append('')

    if st:
        L.append('## 自检结果')
        L.append('')
        L.append('| 检查项 | 结果 |')
        L.append('|---|---|')
        L.append('| 文件大小 | %s 字节 |' % format(st['size'], ','))
        L.append('| 高程范围 | z = %.1f ～ %.1f（水面 %.1f） |' % (
            st['zmin'], st['zmax'], P.WATER_Z))
        L.append('| 水域占比 | %.1f%% |' % st['water_pct'])
        L.append('| 不可通行占比 | %.1f%%（陡崖、山峰、边界山体） |' % st['impassable_pct'])
        L.append('| 地表纹理 | %d 种 |' % st['textures'])
        L.append('| 对象数 | %d 个（%s） |' % (
            st['objects'], '、'.join('%s×%d' % (k, v) for k, v in st['counts'].most_common(5))))
        L.append('')

    prose = os.path.join(P.dir, 'README.md')
    if os.path.isfile(prose):
        L.append(open(prose, encoding='utf-8').read().rstrip())
        L.append('')

    hist = P.version.get('history', [])
    if hist:
        L.append('## 版本历史')
        L.append('')
        L.append('| 版本 | 日期 | 说明 |')
        L.append('|---|---|---|')
        for h in hist:
            L.append('| `%s` | %s | %s |' % (h.get('version', ''), h.get('date', ''), h.get('note', '')))
        L.append('')

    L.append('## 安装')
    L.append('')
    L.append('把 `dist/%s/` 整个复制到 `%%APPDATA%%\\Red Alert 3\\Maps\\`（或解压同名 zip），' % P.map_name)
    L.append('或在工程根目录执行 `build.cmd --map %s --install`。' % P.map_id)
    L.append('')
    L.append('> 本说明由 `tools/make_readme.py` 依据配方与成品地图自动生成，'
             '散文部分来自 `maps/%s/README.md`。' % P.map_id)
    L.append('')
    return '\n'.join(L)


def main():
    ap = argparse.ArgumentParser(description='生成发布包内的地图说明')
    ap.add_argument('--map', dest='map_id', default=None)
    args, _unknown = ap.parse_known_args()
    P = proj.current(args.map_id, refresh=True)
    text = build_readme(P)
    out = P.asset('README.md')
    with open(out, 'w', encoding='utf-8') as f:
        f.write(text)
    print('  %s (%d 字节)' % (out, len(text.encode('utf-8'))))


if __name__ == '__main__':
    main()
