# -*- coding: utf-8 -*-
"""电脑开矿拓展。公共脚本里的可选项，默认不列装。

配方写 ORE_EXPAND = True 才接到公共列表。没写的图和新图都不装。

只处理基地近矿以外的陆地矿，分给最近的出生点。近矿已经在建造场范围内。
水矿不展。可选位先确认不是人类。固定 AI 不看这一条。阵营看基地车或建造场。

苏联：在矿场摆放盘外生成 SovietSurveyor，再展开成前哨。
盟军：已经有一座矿场时，在同一距离生成 AlliedMiner，再展开。
帝国：在矿点上生成 JapanRefineryEgg，再展开。核心展开就是矿场。
矿场本身不由这里放。展开成功之后，电脑自己盖。

落点沿矿口向外，中心离矿点 24 格：摆放盘半径 20，再让出 4 格，身体不压盘。
这一圈不平就换个角度，仍不进盘。整圈都不平就跳过这座矿。
展开用这三台单位共用的 specialpower_unpackreplaceself，动作 192，参数类型沿用科技脚本。
生成用动作 466，动作内部名字写命令字。
"""
import math

from dual_prod import _cell, _clone_waypoint, _flat, _script_or
from nkl_scripts import _ensure_names
from ra3map import parse_heightmap, parse_objects, props_dict, write_objects
from sea_ai import _a, _act, _cond, _count

_COUNT = 'PLAYER_HAS_OBJECT_COMPARISON'
_HUMAN = 'PLAYER_IS_HUMAN_OR_AI_PERSONALITY'
_OWNS = 'NAMED_OWNED_BY_PLAYER'
_SPAWN = 'CREATE_NAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION'
_USE = 'NAMED_USE_COMMANDBUTTON_ABILITY'
_UNPACK = 'specialpower_unpackreplaceself'

_GE = 3
# 矿场摆放盘，和验收的整圆盘一致。再让出展开物占地。
_PAD = 20
_BODY = 4
_STAND = _PAD + _BODY
# 沿矿口向外找平地，最多再退这么多格、转这么多步。
_SEEK = 8
_SWEEP = 16

_PLAYERS = (
    ('Player_1', 'Player_1/teamPlayer_1', 'Player_1_Start', True, 'p1'),
    ('Player_2', 'Player_2/teamPlayer_2', 'Player_2_Start', True, 'p2'),
    ('Player_3', 'Player_3/teamPlayer_3', 'Player_3_Start', True, 'p3'),
    ('Player_4', 'Player_4/teamPlayer_4', 'Player_4_Start', True, 'p4'),
    ('Player_5', 'Player_5/teamPlayer_5', 'Player_5_Start', True, 'p5'),
    ('Player_6', 'Player_6/teamPlayer_6', 'Player_6_Start', True, 'p6'),
    ('PlyrCivilian', 'PlyrCivilian/teamPlyrCivilian', 'Player_7_Start (imported)', False, 'p7'),
    ('PlyrCreeps', 'PlyrCreeps/teamPlyrCreeps', 'Player_8_Start (imported)', False, 'p8'),
)

# 落在矿点上的只有帝国核心。苏联和盟军站在盘外。
_FACTIONS = (
    ('s', ('sovietmcv', 'sovietconstructionyard'), 'sovietsurveyor', False, None),
    ('a', ('alliedmcv', 'alliedconstructionyard'), 'alliedminer', False, 'alliedrefinery'),
    ('j', ('japanmcv', 'japanconstructionyard'), 'japanrefineryegg', True, None),
)


def ore_expand(mmap, cfg):
    """给未开发的陆地矿摆开矿落点，并把三阵营的展开脚本接到公共列表。"""
    terrain = getattr(cfg, 'TERRAIN', None) or {}
    near_r = float(terrain.get('ore_near_max_r', 62.0)) if isinstance(terrain, dict) else 62.0
    z_scale = float(getattr(cfg, 'Z_SCALE', 1.0))
    water_z = float(getattr(cfg, 'WATER_Z', 0.0))
    idmap, _ = _ensure_names(mmap, {
        'Script', 'OrCondition', 'Condition', 'ScriptAction', 'X',
        _COUNT, _HUMAN, _OWNS, _SPAWN, _USE,
    })
    seats, ores, skipped = _targets(mmap, near_r, z_scale, water_z)
    parts = []
    spots = []
    for player, team, _start, skip_human, tag, owned in seats:
        for index, ore in enumerate(owned):
            side = 'ox-%s-%d' % (tag, index)
            node = side + 'n'
            spots.append((ore['stand'], side, ore['face']))
            spots.append((ore['cell'], node, 0.0))
            unit = 'oxu-%s-%d' % (tag, index)
            for fac, marks, obj, on_node, refinery in _FACTIONS:
                wp = node if on_node else side
                face = 0.0 if on_node else ore['face']
                parts.append(_spawn(
                    idmap, player, team, skip_human, fac, marks, obj,
                    refinery, unit, wp, face))
                parts.append(_unpack(
                    idmap, player, skip_human, fac, marks, refinery, unit))
    _place(mmap, spots)
    if parts:
        from team_sides import _append_public
        _append_public(mmap, b''.join(parts))
    print('  [开矿] 未开发陆地矿 %d，盘外不平稳过 %d，脚本 %d 条' % (
        len(ores), skipped, len(parts)))


def _targets(mmap, near_r, z_scale, water_z):
    """近矿两座留给基地。其余陆地矿分给最近的出生点，并在盘外找展开格。"""
    hm = parse_heightmap(mmap.get('HeightMapData'))
    border = hm['border_width']
    objs = parse_objects(mmap.get('ObjectsList'), mmap.table)
    starts = {}
    ore_objs = []
    for o in objs:
        if 'Waypoint' in o['type_name']:
            name = props_dict(o['props']).get('waypointName', '')
            starts[name] = _cell(o['position'][0], o['position'][1], border)
        elif o['type_name'] == 'OreNode':
            ore_objs.append(o)
    bases = []
    for player, _team, start, skip_human, tag in _PLAYERS:
        cell = starts.get(start)
        if cell is None:
            raise SystemExit('没有出生路径点：' + start)
        bases.append((player, tag, skip_human, cell))
    land = []
    for o in ore_objs:
        cell = _cell(o['position'][0], o['position'][1], border)
        elev = hm['elevations'][cell[1] * hm['width'] + cell[0]]
        if elev * z_scale < water_z:
            continue
        land.append((cell, float(o.get('angle') or 0.0)))
    near = set()
    buckets = {i: [] for i in range(len(bases))}
    for oi, (cell, _ang) in enumerate(land):
        bi, bd = 0, 1e18
        for i, (_p, _t, _h, base) in enumerate(bases):
            d = math.hypot(base[0] - cell[0], base[1] - cell[1])
            if d < bd:
                bi, bd = i, d
        buckets[bi].append((bd, oi))
    for items in buckets.values():
        items.sort()
        kept = 0
        for dist, oi in items:
            if dist <= near_r and kept < 2:
                near.add(oi)
                kept += 1
    ore_cells = [c for c, _a in land]
    owned = {tag: [] for _p, tag, _h, _c in bases}
    skipped = 0
    chosen = []
    for oi, (cell, angle) in enumerate(land):
        if oi in near:
            continue
        bi, bd = 0, 1e18
        for i, (_p, _t, _h, base) in enumerate(bases):
            d = math.hypot(base[0] - cell[0], base[1] - cell[1])
            if d < bd:
                bi, bd = i, d
        stand = _stand(hm, cell, angle, ore_cells)
        if stand is None:
            skipped += 1
            continue
        face = _toward(stand, cell)
        rec = {'cell': cell, 'stand': stand, 'face': face}
        owned[bases[bi][1]].append(rec)
        chosen.append(rec)
    seats = []
    for player, team, start, skip_human, tag in _PLAYERS:
        seats.append((player, team, start, skip_human, tag, owned[tag]))
    return seats, chosen, skipped


def _stand(hm, ore, angle, ores):
    """沿矿口向外，站在摆放盘外，并且这一小块和矿点同高。"""
    ox, oy = ore
    base = hm['elevations'][oy * hm['width'] + ox]
    heading = angle
    for extra in range(_SEEK + 1):
        dist = _STAND + extra
        for step in range(_SWEEP):
            turn = step * (math.pi / _SWEEP)
            signs = (1,) if step == 0 else (1, -1)
            for sign in signs:
                ang = heading + sign * turn
                x = int(round(ox + dist * math.cos(ang)))
                y = int(round(oy - dist * math.sin(ang)))
                if not _off_pads((x, y), ores):
                    continue
                if not _flat(hm, x, y, base):
                    continue
                return x, y
    return None


def _off_pads(cell, ores):
    x, y = cell
    for ox, oy in ores:
        if (x - ox) * (x - ox) + (y - oy) * (y - oy) < (_PAD + 1) * (_PAD + 1):
            return False
    return True


def _toward(src, dst):
    """从展开格看向矿点。格 y 向南，游戏角度的正弦取反。"""
    return math.atan2(-(dst[1] - src[1]), dst[0] - src[0])


def _spawn(idmap, player, team, skip_human, fac, marks, obj, refinery, unit, wp, face):
    groups = []
    for mark in marks:
        conds = []
        if skip_human:
            conds.append(_cond(idmap, idmap[_HUMAN], 146, [
                _a(11, s=player), _a(70, s='Human'),
            ], inverted=True))
        conds.append(_count(idmap, idmap[_COUNT], player, mark, _GE, 1))
        if refinery:
            conds.append(_count(idmap, idmap[_COUNT], player, refinery, _GE, 1))
        conds.append(_cond(idmap, idmap[_OWNS], 25, [
            _a(11, s=player), _a(14, s=unit),
        ], inverted=True))
        groups.append(conds)
    return _script_or(idmap, 'ox-%s-%s' % (fac, unit), groups, [
        _act(idmap, idmap[_SPAWN], 466, [
            _a(14, s=unit), _a(15, s=obj), _a(3, s=team),
            _a(7, s=wp), _a(17, f=face),
        ]),
    ])


def _unpack(idmap, player, skip_human, fac, marks, refinery, unit):
    """只给已经生成的那一台展开。别的阵营看见同名单位也不按。"""
    groups = []
    for mark in marks:
        conds = [
            _cond(idmap, idmap[_OWNS], 25, [
                _a(11, s=player), _a(14, s=unit),
            ]),
            _count(idmap, idmap[_COUNT], player, mark, _GE, 1),
        ]
        if skip_human:
            conds.insert(0, _cond(idmap, idmap[_HUMAN], 146, [
                _a(11, s=player), _a(70, s='Human'),
            ], inverted=True))
        if refinery:
            conds.append(_count(idmap, idmap[_COUNT], player, refinery, _GE, 1))
        groups.append(conds)
    return _script_or(idmap, 'ox-%s-%s-go' % (fac, unit), groups, [
        _act(idmap, idmap[_USE], 192, [
            _a(14, s=unit), _a(34, s=_UNPACK),
        ]),
    ])


def _place(mmap, spots):
    """spots 是 (格, 路径点名, 朝向)。格已经算好。"""
    if not spots:
        return 0
    chunk = mmap.get('ObjectsList')
    objs = parse_objects(chunk, mmap.table)
    hm = parse_heightmap(mmap.get('HeightMapData'))
    by_name = {}
    proto = None
    next_id = 1
    for o in objs:
        if 'Waypoint' not in o['type_name']:
            continue
        d = props_dict(o['props'])
        by_name[d.get('waypointName', '')] = o
        proto = o
        wid = d.get('waypointID')
        if isinstance(wid, int) and wid >= next_id:
            next_id = wid + 1
    if proto is None:
        raise SystemExit('模板里没有可复制的路径点')
    made = 0
    for cell, name, face in spots:
        if name in by_name:
            raise SystemExit('开矿路径点已存在：' + name)
        wp = _clone_waypoint(proto, name, next_id, cell, hm)
        wp['angle'] = face
        objs.append(wp)
        by_name[name] = wp
        next_id += 1
        made += 1
    object_idx = next(i for i, n in mmap.table.items() if n == 'Object')
    key_ids = {name: idx for idx, name in mmap.table.items()}
    chunk.payload = write_objects(objs, object_idx, key_ids)
    return made
