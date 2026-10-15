# -*- coding: utf-8 -*-
"""电脑的第二座产能建筑。

每张图组装时都装，没有配方开关。

第二座不进电脑的建造队列，也不扣钱、不看电力。
条件满足后用动作 467，命令字 CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION。
参数依次是物体类型、队伍、路径点、朝向：在路径点上按朝向直接生成该类型。
路径点要先摆在出生点旁边的平地上。脚本名不是路径点。

可选位先确认不是人类。固定 AI 不看这个条件。
兵营：开局不放第二座。该阵营重工至少有一座，并且兵营至少有一座时，放一座兵营。
兵营先盖到两座再出重工时，正好一座的条件会错过，所以兵营用大于等于。
重工、机场、船厂：该建筑正好有一座时，放第二座。还没有第一座时不放。
帝国的核心也算第一座。帝国没有单独的机场，不放机场。
比较号：2 是等于，3 是大于等于。
条件和动作的内部名字用命令字。能生效的 467 写的是命令字，写 X 时游戏不认这条。
"""
import math
import struct

from nkl_scripts import _ensure_names, _walk
from ra3map import parse_heightmap, parse_objects, props_dict, write_objects
from sea_ai import _a, _act, _asset, _cond, _count, _u16

# 和能生效的 NKL 脚本同一套命令字。
_COUNT = 'PLAYER_HAS_OBJECT_COMPARISON'
_HUMAN = 'PLAYER_IS_HUMAN_OR_AI_PERSONALITY'
_SPAWN = 'CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION'

# 0 小于，2 等于，3 大于等于。
_EQ = 2
_GE = 3
# 本仓库地图每格 10 个世界单位。NKL 第二座机场的朝向是 -135 度。
_CELL = 10.0
_FACE = -2.356194496154785
# 落点离出生点至少这么多格，避开基地车展开的那一块。
_MIN_DIST = 14
_MAX_DIST = 26
_PAD = 3
_SEP = 8

# (玩家, 队伍, 出生路径点, 是否要排除人类)
_PLAYERS = (
    ('Player_1', 'Player_1/teamPlayer_1', 'Player_1_Start', True),
    ('Player_2', 'Player_2/teamPlayer_2', 'Player_2_Start', True),
    ('Player_3', 'Player_3/teamPlayer_3', 'Player_3_Start', True),
    ('Player_4', 'Player_4/teamPlayer_4', 'Player_4_Start', True),
    ('Player_5', 'Player_5/teamPlayer_5', 'Player_5_Start', True),
    ('Player_6', 'Player_6/teamPlayer_6', 'Player_6_Start', True),
    ('PlyrCivilian', 'PlyrCivilian/teamPlyrCivilian', 'Player_7_Start (imported)', False),
    ('PlyrCreeps', 'PlyrCreeps/teamPlyrCreeps', 'Player_8_Start (imported)', False),
)

# 机场为空表示这个阵营没有这种建筑。
_FACTIONS = (
    ('a', {
        'bar': 'alliedbarracks',
        'wf': 'alliedwarfactory',
        'air': 'alliedairfield',
        'yard': 'alliednavalyard',
    }),
    ('s', {
        'bar': 'sovietbarracks',
        'wf': 'sovietwarfactory',
        'air': 'sovietairfield',
        'yard': 'sovietnavalyard',
    }),
    ('j', {
        'bar': 'japanbarracks',
        'bar_egg': 'japanbarracksegg',
        'wf': 'japanwarfactory',
        'wf_egg': 'japanwarfactoryegg',
        'air': None,
        'yard': 'japannavalyard',
        'yard_egg': 'japannavalyardegg',
    }),
)

_KINDS = ('bar', 'wf', 'air', 'yard')


def dual_production(mmap):
    """把第二座产能的建造指令接到公共脚本列表末尾，并摆好落点。"""
    idmap, _ = _ensure_names(mmap, {
        'Script', 'OrCondition', 'Condition', 'ScriptAction', 'X',
        _COUNT, _HUMAN, _SPAWN,
    })
    parts = []
    spots = []
    for player, team, start, skip_human in _PLAYERS:
        tag = player.replace('Player_', 'p').replace('Plyr', '').lower()
        for fac, spec in _FACTIONS:
            parts.append(_barracks(idmap, tag, fac, player, team, skip_human, spec))
            for kind in ('wf', 'air', 'yard'):
                obj = spec[kind]
                if not obj:
                    continue
                parts.append(_second(
                    idmap, tag, fac, kind, player, team, skip_human, spec, obj))
        for kind in _KINDS:
            spots.append((start, 'dual-%s-%s' % (tag, kind), kind))
    blob = b''.join(parts)
    _append_public(mmap, blob)
    n_wp = _place_waypoints(mmap, spots)
    print('  [产能] 第二座 %d 条，落点 %d 个（兵营等重工，重工/机场/船厂等第一座）' % (
        len(parts), n_wp))


def _barracks(idmap, tag, fac, player, team, skip_human, spec):
    groups = _combine(idmap, player, skip_human, [
        _alts(spec, 'wf', _GE, 1),
        _alts(spec, 'bar', _GE, 1),
    ])
    name = 'dual-%s-%s-bar' % (tag, fac)
    return _script_or(idmap, name, groups, [
        _build(idmap, spec['bar'], team, 'dual-%s-bar' % tag),
    ])


def _second(idmap, tag, fac, kind, player, team, skip_human, spec, obj):
    groups = _combine(idmap, player, skip_human, [
        _alts(spec, kind, _EQ, 1),
    ])
    name = 'dual-%s-%s-%s' % (tag, fac, kind)
    return _script_or(idmap, name, groups, [
        _build(idmap, obj, team, 'dual-%s-%s' % (tag, kind)),
    ])


def _alts(spec, kind, cmp_, n):
    names = [spec[kind]]
    egg = spec.get(kind + '_egg')
    if egg:
        names.append(egg)
    return [(name, cmp_, n) for name in names]


def _combine(idmap, player, skip_human, categories):
    """每一类里的名字互相或，类与类之间是与。"""
    groups = [[]]
    for choices in categories:
        groups = [prev + [one] for prev in groups for one in choices]
    out = []
    for terms in groups:
        conds = _gate(idmap, player, skip_human)
        for obj, cmp_, n in terms:
            conds.append(_count(idmap, idmap[_COUNT], player, obj, cmp_, n))
        out.append(conds)
    return out


def _gate(idmap, player, skip_human):
    if not skip_human:
        return []
    return [_cond(idmap, idmap[_HUMAN], 146, [
        _a(11, s=player), _a(70, s='Human'),
    ], inverted=True)]


def _build(idmap, obj, team, waypoint):
    return _act(idmap, idmap[_SPAWN], 467, [
        _a(15, s=obj), _a(3, s=team), _a(7, s=waypoint), _a(17, f=_FACE),
    ])


def _script_or(idmap, name, groups, actions):
    """几组条件互相或。一组里面的条件互相与。成功后停。"""
    body = _u16(name) + _u16('') + _u16('') + _u16('')
    body += bytes((1, 1, 1, 1, 1, 0))
    body += struct.pack('<I', 0)
    body += bytes((0, 0)) + struct.pack('<i', 0) + bytes((1,)) + _u16('') + _u16('X')
    for group in groups:
        body += _asset(idmap, 'OrCondition', 1, b''.join(group))
    body += b''.join(actions)
    return _asset(idmap, 'Script', 4, body)


def _append_public(mmap, blob):
    chunk = mmap.get('PlayerScriptsList')
    data = bytearray(chunk.payload)
    ni = struct.unpack_from('<I', data, 0)[0]
    if mmap.table.get(ni) != 'ScriptList':
        raise SystemExit('公共脚本不是第一份列表')
    sz = struct.unpack_from('<I', data, 6)[0]
    if 10 + sz > len(data):
        raise SystemExit('公共脚本列表长度不对')
    data[10 + sz:10 + sz] = blob
    data[6:10] = struct.pack('<I', sz + len(blob))
    chunk.payload = bytes(data)
    _walk(blob, 0, len(blob), mmap.table, lambda *a: None)


def _place_waypoints(mmap, spots):
    """在每个出生点四周的平地上放第二座产能的路径点。"""
    chunk = mmap.get('ObjectsList')
    objs = parse_objects(chunk, mmap.table)
    hm = parse_heightmap(mmap.get('HeightMapData'))
    border = hm['border_width']
    by_name = {}
    proto = None
    next_id = 1
    blocked = []
    for o in objs:
        if 'Waypoint' not in o['type_name']:
            wx, wy = o['position'][0], o['position'][1]
            blocked.append(_cell(wx, wy, border))
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
    for start_name, wp_name, kind in spots:
        if wp_name in by_name:
            continue
        start = by_name.get(start_name)
        if start is None:
            raise SystemExit('没有出生路径点：' + start_name)
        cx, cy = _cell(start['position'][0], start['position'][1], border)
        cell = _find_pad(hm, cx, cy, blocked, kind)
        blocked.append(cell)
        wp = _clone_waypoint(proto, wp_name, next_id, cell, hm)
        objs.append(wp)
        by_name[wp_name] = wp
        next_id += 1
        made += 1
    if made:
        object_idx = next(i for i, n in mmap.table.items() if n == 'Object')
        key_ids = {name: idx for idx, name in mmap.table.items()}
        chunk.payload = write_objects(objs, object_idx, key_ids)
    return made


def _cell(wx, wy, border):
    return int(round(wx / _CELL + border)), int(round(wy / _CELL + border))


def _cell_world(hm, cx, cy):
    border = hm['border_width']
    return (cx - border) * _CELL, (cy - border) * _CELL, 0.0


def _flat(hm, cx, cy, base):
    w, h = hm['width'], hm['height']
    elev = hm['elevations']
    if cx - _PAD < 1 or cy - _PAD < 1 or cx + _PAD >= w - 1 or cy + _PAD >= h - 1:
        return False
    for y in range(cy - _PAD, cy + _PAD + 1):
        row = y * w
        for x in range(cx - _PAD, cx + _PAD + 1):
            if elev[row + x] != base:
                return False
    return True


def _clear_of(cell, blocked, gap):
    x, y = cell
    for ox, oy in blocked:
        if (x - ox) * (x - ox) + (y - oy) * (y - oy) < gap * gap:
            return False
    return True


def _find_pad(hm, cx, cy, blocked, kind):
    base = hm['elevations'][cy * hm['width'] + cx]
    ang0 = _KINDS.index(kind) * (math.pi / 2.0)
    for dist in range(_MIN_DIST, _MAX_DIST + 1):
        for step in range(0, 24):
            ang = ang0 + step * (math.pi / 24.0)
            for sign in ((1,) if step == 0 else (1, -1)):
                x = int(round(cx + dist * math.cos(sign * ang)))
                y = int(round(cy + dist * math.sin(sign * ang)))
                if not _clear_of((x, y), blocked + [(cx, cy)], _SEP):
                    continue
                if _flat(hm, x, y, base):
                    return x, y
    raise SystemExit('出生点 (%d,%d) 旁边没有第二座%s的平地' % (cx, cy, kind))


def _clone_waypoint(proto, name, wid, cell, hm):
    wp = dict(proto)
    wp['position'] = _cell_world(hm, cell[0], cell[1])
    wp['angle'] = _FACE
    wp['props'] = [[k, t, v] for k, t, v in proto['props']]
    for pr in wp['props']:
        if pr[0] in ('waypointName', 'uniqueID'):
            pr[2] = name
        elif pr[0] == 'waypointID':
            pr[2] = wid
    return wp
