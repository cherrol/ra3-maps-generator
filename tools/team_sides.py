# -*- coding: utf-8 -*-
"""按出生位置分队。每张图组装时都加，不看配方开关。

p1–p4（Player_1 到 Player_4）一队，p5–p8（Player_5、Player_6、PlyrCivilian、PlyrCreeps）一队。
队内互相友好，两队互相敌对。每秒重申一次，覆盖房间里选的队伍。

大厅六个位子里正好有四个在局内时，这四个人都算西队。
落在 p5、p6 的队伍会送到空着的 p1–p4 出生点。两个固定 AI 留在东队。
"""
import struct

from nkl_scripts import _ensure_names, _walk
from sea_ai import _a, _act, _cond, _script

# 条件和动作的内部名字用命令字。写 X 时游戏不执行这条，敌对就留在开局的默认关系上。
_RELATE = 'PLAYER_RELATES_PLAYER'
_INC = 'INCREMENT_COUNTER'
_MOVE = 'TEAM_TELEPORT_TO_WAYPOINT'
_SET = 'SET_FLAG'
_COUNTER_IS = 'COUNTER'
_FLAG_IS = 'FLAG'
_AT = 'START_POSITION_IS'

# 条件 1 / 79 的比较号：2 是等于。关系：0 敌对，2 友好。
_EQ = 2

_WEST = ('Player_1', 'Player_2', 'Player_3', 'Player_4')
_EAST_LOBBY = ('Player_5', 'Player_6')
_BONUS = ('PlyrCivilian', 'PlyrCreeps')
_EAST = _EAST_LOBBY + _BONUS
_LOBBY = _WEST + _EAST_LOBBY

_COUNTER = 'seatn'


def team_sides(mmap):
    """把分队和四人落位接到公共脚本列表末尾。"""
    idmap, _ = _ensure_names(mmap, {
        'Script', 'OrCondition', 'Condition', 'ScriptAction', 'X',
        _RELATE, _INC, _MOVE, _SET, _COUNTER_IS, _FLAG_IS, _AT,
    })
    key = idmap['X']
    parts = []
    for index, player in enumerate(_LOBBY, 1):
        parts.append(_count_seat(idmap, key, index, player))
    parts.append(_relations(idmap, key, 'side-slot', _WEST, _EAST, four=False))
    parts.append(_relations(idmap, key, 'side-four', _LOBBY, _BONUS, four=True))
    parts.extend(_move_east(idmap, key, 5))
    parts.append(_mark_absent(idmap, key, 5))
    parts.extend(_move_east(idmap, key, 6))
    blob = b''.join(parts)
    _append_public(mmap, blob)
    print('  [分队] p1–p4 对 p5–p8，四人时西队落位，%d 条' % len(parts))


def _count_seat(idmap, key, index, player):
    """这个位子有人时，座位计数加一。只加一次。"""
    return _script(idmap, key, 'side-c%d' % index, [
        _at(idmap, key, player, index),
    ], [
        _act(idmap, idmap[_INC], 15, [_a(0, i=1), _a(4, s=_COUNTER)]),
    ], deactivate=1, interval=0)


def _relations(idmap, key, name, friends_a, friends_b, four):
    """four 为真时只在正好四人时生效，否则在不是四人时生效。"""
    cond = _cond(idmap, idmap[_COUNTER_IS], 1, [
        _a(4, s=_COUNTER), _a(6, i=_EQ), _a(0, i=4),
    ], inverted=not four)
    actions = []
    actions += _within(idmap, key, friends_a)
    actions += _within(idmap, key, friends_b)
    actions += _between(idmap, key, friends_a, friends_b)
    return _script(idmap, key, name, [cond], actions, deactivate=0, interval=1)


def _move_east(idmap, key, src):
    """把 p5 或 p6 送到编号最小的空西位。p6 等 p5 先占完。"""
    player = 'Player_%d' % src
    team = 'Player_%d/teamPlayer_%d' % (src, src)
    sent = 'sent%d' % src
    out = []
    for dest in range(1, 5):
        conds = [
            _four(idmap, key),
            _at(idmap, key, player, src),
            _flag_off(idmap, key, sent),
            _at(idmap, key, 'Player_%d' % dest, dest, inverted=True),
            _flag_off(idmap, key, 'fill%d' % dest),
        ]
        if src == 6:
            conds.append(_flag_on(idmap, key, 'sent5'))
        out.append(_script(idmap, key, 'side-m%d%d' % (src, dest), conds, [
            _act(idmap, idmap[_MOVE], 379, [
                _a(3, s=team), _a(7, s='Player_%d_Start' % dest),
            ]),
            _set_flag(idmap, key, sent),
            _set_flag(idmap, key, 'fill%d' % dest),
        ], deactivate=1, interval=0))
    return out


def _mark_absent(idmap, key, src):
    """p5 没人时也打下标记，p6 才不会一直等。"""
    return _script(idmap, key, 'side-m%dx' % src, [
        _four(idmap, key),
        _at(idmap, key, 'Player_%d' % src, src, inverted=True),
        _flag_off(idmap, key, 'sent%d' % src),
    ], [
        _set_flag(idmap, key, 'sent%d' % src),
    ], deactivate=1, interval=0)


def _within(idmap, key, group):
    out = []
    for a in group:
        for b in group:
            if a != b:
                out.append(_relate(idmap, key, a, b, True))
    return out


def _between(idmap, key, left, right):
    out = []
    for a in left:
        for b in right:
            out.append(_relate(idmap, key, a, b, False))
            out.append(_relate(idmap, key, b, a, False))
    return out


def _relate(idmap, key, a, b, friend):
    kind = (2, 'Friend') if friend else (0, 'Enemy')
    return _act(idmap, idmap[_RELATE], 78, [
        _a(11, s=a), _a(11, s=b), _a(19, i=kind[0], s=kind[1]),
    ])


def _at(idmap, key, player, index, inverted=False):
    return _cond(idmap, idmap[_AT], 79, [
        _a(11, s=player), _a(0, i=index),
    ], inverted=inverted)


def _four(idmap, key):
    return _cond(idmap, idmap[_COUNTER_IS], 1, [
        _a(4, s=_COUNTER), _a(6, i=_EQ), _a(0, i=4),
    ])


def _flag_on(idmap, key, name):
    return _cond(idmap, idmap[_FLAG_IS], 2, [_a(5, s=name), _a(8, i=1)])


def _flag_off(idmap, key, name):
    return _cond(idmap, idmap[_FLAG_IS], 2, [_a(5, s=name), _a(8, i=1)], inverted=True)


def _set_flag(idmap, key, name):
    return _act(idmap, idmap[_SET], 1, [_a(5, s=name), _a(8, i=1)])


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
