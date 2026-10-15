# -*- coding: utf-8 -*-
"""左下角只显示电脑的现金。每张图组装时都加，不看配方开关。

大厅六个位子是人，或这个位子没人，就不显示。是电脑才显示，说明写成 AI1–AI6。
两个固定 AI 不看「是不是人类」——模板里这两位被标成人类。按他们的出生位显示成 AI7 或 AI8。
说明必须是 map.str 里的键（COUNTER:AI1）。显示动作不认字面文字。
现金每秒抄一次。抄的是身上的钱，不含地上还没采的矿。显示只挂一次。
"""
import struct

from nkl_scripts import _ensure_names, _walk
from sea_ai import _a, _act, _cond, _script

_LOBBY = (
    'Player_1', 'Player_2', 'Player_3', 'Player_4', 'Player_5', 'Player_6',
)
_BONUS = ('PlyrCivilian', 'PlyrCreeps')


def label(index):
    """显示动作使用的文本键，对应 map.str 里的 COUNTER:AI1 到 AI8。"""
    return 'COUNTER:AI%d' % index


def map_str_entries():
    """左侧金额的说明。游戏按这个键取字，取不到就不画这一行。"""
    lines = []
    for index in range(1, 9):
        lines += [label(index), '"AI%d"' % index, 'End', '']
    return lines


def ai_money(mmap):
    """把电脑现金的显示接到公共脚本列表末尾。"""
    idmap, _ = _ensure_names(mmap, {
        'Script', 'OrCondition', 'Condition', 'ScriptAction', 'X',
    })
    key = idmap['X']
    parts = []
    for index, player in enumerate(_LOBBY, 1):
        parts.append(_hide_human(idmap, key, index, player))
        parts.append(_hide_empty(idmap, key, index, player))
        conds = [
            _human(idmap, key, player, inverted=True),
            _at(idmap, key, player, index),
        ]
        parts.append(_once(idmap, key, 'cash-s%d' % index, conds,
                           _publish(idmap, key, player, index)))
        parts.append(_every(idmap, key, 'cash-t%d' % index, conds,
                            [_copy(idmap, key, player, index)]))
    for player in _BONUS:
        tag = 'c' if player == 'PlyrCivilian' else 'k'
        for index in (7, 8):
            conds = [_at(idmap, key, player, index)]
            parts.append(_once(idmap, key, 'cash-%s%d' % (tag, index), conds,
                               _publish(idmap, key, player, index)))
            parts.append(_every(idmap, key, 'cash-%st%d' % (tag, index), conds,
                                [_copy(idmap, key, player, index)]))
    blob = b''.join(parts)
    _append_public(mmap, blob)
    print('  [现金] 只显示电脑，%d 条' % len(parts))


def _once(idmap, key, name, conds, actions):
    return _script(idmap, key, name, conds, actions, deactivate=1, interval=0)


def _every(idmap, key, name, conds, actions):
    return _script(idmap, key, name, conds, actions, deactivate=0, interval=1)


def _hide_human(idmap, key, index, player):
    return _script(idmap, key, 'cash-h%d' % index, [
        _human(idmap, key, player),
    ], [_hide(idmap, key, index)], deactivate=0, interval=1)


def _hide_empty(idmap, key, index, player):
    return _script(idmap, key, 'cash-x%d' % index, [
        _at(idmap, key, player, index, inverted=True),
    ], [_hide(idmap, key, index)], deactivate=0, interval=1)


def _publish(idmap, key, player, index):
    return [
        _copy(idmap, key, player, index),
        _act(idmap, key, 188, [_a(4, s='cash%d' % index), _a(25, s=label(index))]),
    ]


def _copy(idmap, key, player, index):
    return _act(idmap, key, 300, [_a(11, s=player), _a(4, s='cash%d' % index)])


def _hide(idmap, key, index):
    return _act(idmap, key, 189, [_a(4, s='cash%d' % index)])


def _human(idmap, key, player, inverted=False):
    return _cond(idmap, key, 146, [
        _a(11, s=player), _a(70, s='Human'),
    ], inverted=inverted)


def _at(idmap, key, player, index, inverted=False):
    return _cond(idmap, key, 79, [
        _a(11, s=player), _a(0, i=index),
    ], inverted=inverted)


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
