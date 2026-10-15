# -*- coding: utf-8 -*-
"""电脑进攻对位。公共脚本里的可选项，默认不列装。

配方写 TEAM_ATK = True 才接到公共列表。没写的图和新图都不装。

开局满 60 秒，不看出场人数，强制把该阵营的 team_atk 交给脚本，进攻对位出生点，
并让这个电脑优先打对位玩家。之后每 120 秒再看一次步兵、载具、飞机。
矿车和基地车从人数里扣掉。人数大于等于 20 就再下这两条指令。
人数低于 20，而且之前下过进攻令，才把队伍交还遭遇战 AI，并清掉优先目标。

人数用种类计数。编号沿用早期 SAGE 的种类顺序：8 步兵、9 载具、10 飞机、
12 推土机（基地车）、13 矿车。死亡单位和还在队列里的单位不计入。舰船不计入。

造多少不写进脚本。队伍只列出兵种、最低 0、最高留出空位，交给 AI 自己决定。
坐标进攻只带动已经进了这支队伍的单位。优先攻击作用在该电脑的全部部队上。

对位按出生位子：p1 对 p5，p2 对 p6，p3 对 p7，p4 对 p8。
四人房间会把 p5、p6 送到空的西侧出生点，这里的对位路点仍是原来的南侧出生点。
八个位子各有自己的队伍，名字是 team_atk_a_p1 这种，脚本里写成 Player_1/team_atk_a_p1。

计时是倒数计数器 atk-wait。脚本按列表顺序跑：先点人数，再下令，然后才拨回 120 并记下已经过了第一次，最后每秒减 1。
第一次到 0 时还没有这个记号，所以强制进攻。之后再到 0 才按人数决定进攻或收回。
"""
from nkl_scripts import _ensure_names
from ra3map import PROP_ASCII, PROP_BOOL, PROP_INT, PROP_UNKNOWN, parse_teams, props_dict
from sea_ai import _a, _act, _cond, _count, _script, _true
from team_sides import _append_public, _flag_off, _flag_on, _set_flag

_GE = 3
_EQ = 2
_LT = 0
_FIRST_SECONDS = 60
_REPEAT_SECONDS = 120
_NEED = 20
# 队伍容量，不是建造配额。最低为 0，脚本不要求造满。
_TEAM_ROOM = 99

# (玩家, 要排除人类, 对位玩家, 对位出生点, 自己的出生点)
_SEATS = (
    ('Player_1', True, 'Player_5', 'Player_5_Start', 'Player_1_Start'),
    ('Player_2', True, 'Player_6', 'Player_6_Start', 'Player_2_Start'),
    ('Player_3', True, 'PlyrCivilian', 'Player_7_Start (imported)', 'Player_3_Start'),
    ('Player_4', True, 'PlyrCreeps', 'Player_8_Start (imported)', 'Player_4_Start'),
    ('Player_5', True, 'Player_1', 'Player_1_Start', 'Player_5_Start'),
    ('Player_6', True, 'Player_2', 'Player_2_Start', 'Player_6_Start'),
    ('PlyrCivilian', False, 'Player_3', 'Player_3_Start', 'Player_7_Start (imported)'),
    ('PlyrCreeps', False, 'Player_4', 'Player_4_Start', 'Player_8_Start (imported)'),
)

_TAGS = {
    'Player_1': 'p1',
    'Player_2': 'p2',
    'Player_3': 'p3',
    'Player_4': 'p4',
    'Player_5': 'p5',
    'Player_6': 'p6',
    'PlyrCivilian': 'p7',
    'PlyrCreeps': 'p8',
}

# 阵营标记看基地车或建造厂，卖掉其中一座后标记还在。
_FACTIONS = (
    ('a', 'team_atk_a', ('AlliedMCV', 'AlliedConstructionYard'), (
        'AlliedAntiInfantryInfantry',
        'AlliedAntiVehicleInfantry',
        'AlliedAntiVehicleVehicleTech1',
        'AlliedAntiInfantryVehicle',
        'AlliedAntiStructureVehicle',
        'AlliedAntiVehicleVehicleTech3',
        'AlliedAntiGroundAircraft',
    )),
    ('s', 'team_atk_s', ('SovietMCV', 'SovietConstructionYard'), (
        'SovietAntiInfantryInfantry',
        'SovietAntiVehicleInfantry',
        'SovietHeavyAntiVehicleInfantry',
        'SovietAntiVehicleVehicleTech1',
        'SovietAntiVehicleVehicleTech3',
        'SovietAntiStructureVehicle',
        'SovietAntiGroundAircraft',
    )),
    ('j', 'team_atk_j', ('JapanMCV', 'JapanConstructionYard'), (
        'JapanAntiInfantryInfantry',
        'JapanAntiVehicleInfantry',
        'JapanInfiltrationInfantry',
        'JapanAntiInfantryVehicle',
        'JapanAntiVehicleVehicleTech1',
        'JapanAntiStructureVehicle',
        'JapanAntiVehicleVehicleTech3',
    )),
)

# 种类、编号、暂存计数器。后两项从总数里减掉。
_COUNT_KINDS = (
    (8, 'INFANTRY', 'atk-inf', False),
    (9, 'VEHICLE', 'atk-veh', False),
    (10, 'AIRCRAFT', 'atk-air', False),
    (12, 'DOZER', 'atk-doz', True),
    (13, 'HARVESTER', 'atk-har', True),
)


def team_atk(mmap, cfg=None):
    """给八个位子各加三支进攻队，并把进攻脚本接到公共列表末尾。"""
    _add_teams(mmap)
    idmap, _ = _ensure_names(mmap, {
        'Script', 'OrCondition', 'Condition', 'ScriptAction', 'X',
        'SET_FLAG', 'FLAG',
    })
    key = idmap['X']
    parts = [_boot(idmap, key)]
    attacks = []
    for player, skip_human, enemy, enemy_wp, _home in _SEATS:
        tag = _TAGS[player]
        for fac, _team, objs, _units in _FACTIONS:
            for obj in objs:
                parts.append(_pick(idmap, key, tag, fac, player, skip_human, obj))
        parts.append(_tally(idmap, key, tag, player, skip_human))
        for fac, _team, _objs, _units in _FACTIONS:
            full = '%s/%s' % (player, _team_name(fac, tag))
            attacks.append(_force(
                idmap, key, tag, fac, player, skip_human, full, enemy, enemy_wp))
            attacks.append(_attack(
                idmap, key, tag, fac, player, skip_human, full, enemy, enemy_wp))
            attacks.append(_recall(
                idmap, key, tag, fac, player, skip_human, full))
    parts.extend(attacks)
    parts.append(_rearm(idmap, key))
    parts.append(_tick(idmap, key))
    _append_public(mmap, b''.join(parts))
    print('  [进攻] %d 支队伍，%d 条脚本，满 %d 秒强制进攻，之后每 %d 秒人数 >= %d 再下令' % (
        len(_SEATS) * len(_FACTIONS), len(parts),
        _FIRST_SECONDS, _REPEAT_SECONDS, _NEED))


def _add_teams(mmap):
    from map_gen import _write_teams
    keys = ['teamName', 'teamOwner', 'teamIsSingleton', 'teamHome',
            'teamProductionPriority', 'teamIsAIRecruitable', 'teamAutoReinforce',
            'teamMaxInstances', 'exportWithScript', 'teamBuildFromOldBuildSystem']
    for i in range(1, 8):
        keys.append('teamUnitType%d' % i)
        keys.append('teamUnitMinCount%d' % i)
        keys.append('teamUnitMaxCount%d' % i)
    _ensure_names(mmap, keys)
    key_ids = {name: idx for idx, name in mmap.table.items()}
    chunk = mmap.get('Teams')
    teams = parse_teams(chunk, mmap.table)
    if _write_teams(teams, key_ids) != chunk.payload:
        raise SystemExit('Teams 回写和原文不一致，停止加进攻队')
    have = {props_dict(props).get('teamName') for props in teams}
    added = 0
    for player, _skip, _enemy, _wp, home in _SEATS:
        tag = _TAGS[player]
        for fac, _team, _objs, units in _FACTIONS:
            team = _team_name(fac, tag)
            if team in have:
                raise SystemExit('进攻队已存在：%s' % team)
            teams.append(_team_props(player, team, home, units))
            have.add(team)
            added += 1
    chunk.payload = _write_teams(teams, key_ids)
    if added != len(_SEATS) * len(_FACTIONS):
        raise SystemExit('进攻队数量不对：%d' % added)


def _team_props(player, team, home, units):
    if len(units) > 7:
        raise SystemExit('%s 超过 7 种单位' % team)
    props = [
        ['teamName', PROP_ASCII, team],
        ['teamOwner', PROP_ASCII, player],
        ['teamIsSingleton', PROP_BOOL, 1],
        ['teamHome', PROP_ASCII, home],
        ['teamProductionPriority', PROP_INT, 1],
        ['teamIsAIRecruitable', PROP_BOOL, 1],
        ['teamAutoReinforce', PROP_BOOL, 0],
        ['teamMaxInstances', PROP_INT, 1],
        ['exportWithScript', PROP_BOOL, 1],
        ['teamBuildFromOldBuildSystem', PROP_BOOL, 0],
    ]
    for i, unit in enumerate(units, 1):
        props.append(['teamUnitType%d' % i, PROP_UNKNOWN, 'GameObject:' + unit])
        props.append(['teamUnitMinCount%d' % i, PROP_INT, 0])
        props.append(['teamUnitMaxCount%d' % i, PROP_INT, _TEAM_ROOM])
    return props


def _boot(idmap, key):
    return _script(idmap, key, 'atk-boot', [_true(idmap, key)], [
        _set_counter(idmap, key, 'atk-one', 1),
        _set_counter(idmap, key, 'atk-wait', _FIRST_SECONDS),
        _set_flag(idmap, key, 'atk-live'),
    ], deactivate=1, interval=0)


def _tick(idmap, key):
    return _script(idmap, key, 'atk-tick', [
        _flag_on(idmap, key, 'atk-live'),
        _counter(idmap, key, 'atk-wait', _GE, 1),
    ], [
        _math(idmap, key, 'atk-wait', 1, 'atk-one'),
    ], deactivate=0, interval=1)


def _rearm(idmap, key):
    return _script(idmap, key, 'atk-rearm', [
        _flag_on(idmap, key, 'atk-live'),
        _counter(idmap, key, 'atk-wait', _EQ, 0),
    ], [
        _set_flag(idmap, key, 'atk-later'),
        _set_counter(idmap, key, 'atk-wait', _REPEAT_SECONDS),
    ], deactivate=0, interval=1)


def _pick(idmap, key, tag, fac, player, skip_human, obj):
    conds = _gate(idmap, key, player, skip_human)
    conds.append(_count(idmap, key, player, obj, _GE, 1))
    kind = 'mcv' if obj.endswith('MCV') else 'yard'
    return _script(idmap, key, 'atk-%s-%s-%s' % (tag, fac, kind), conds, [
        _set_flag(idmap, key, _fac_flag(tag, fac)),
    ], deactivate=1, interval=0)


def _tally(idmap, key, tag, player, skip_human):
    total = _num(tag)
    actions = [_set_counter(idmap, key, total, 0)]
    for kind, name, scratch, subtract in _COUNT_KINDS:
        actions.append(_act(idmap, key, 493, [
            _a(11, s=player),
            _a(27, i=kind, s=name),
            _a(4, s=scratch),
            _a(8, i=0),
            _a(8, i=0),
        ]))
        actions.append(_math(idmap, key, total, 1 if subtract else 0, scratch))
    return _script(idmap, key, 'atk-%s-n' % tag, _gate(idmap, key, player, skip_human) or [
        _true(idmap, key),
    ], actions, deactivate=0, interval=1)


def _force(idmap, key, tag, fac, player, skip_human, team, enemy, waypoint):
    conds = _pulse(idmap, key, tag, fac, player, skip_human)
    conds.append(_flag_off(idmap, key, 'atk-later'))
    return _script(idmap, key, 'atk-%s-%s-now' % (tag, fac), conds,
                   _orders(idmap, key, tag, team, player, enemy, waypoint),
                   deactivate=0, interval=1)


def _attack(idmap, key, tag, fac, player, skip_human, team, enemy, waypoint):
    conds = _pulse(idmap, key, tag, fac, player, skip_human)
    conds.append(_flag_on(idmap, key, 'atk-later'))
    conds.append(_counter(idmap, key, _num(tag), _GE, _NEED))
    return _script(idmap, key, 'atk-%s-%s-go' % (tag, fac), conds,
                   _orders(idmap, key, tag, team, player, enemy, waypoint),
                   deactivate=0, interval=1)


def _recall(idmap, key, tag, fac, player, skip_human, team):
    conds = _pulse(idmap, key, tag, fac, player, skip_human)
    conds.append(_flag_on(idmap, key, 'atk-later'))
    conds.append(_flag_on(idmap, key, _go_flag(tag)))
    conds.append(_counter(idmap, key, _num(tag), _LT, _NEED))
    return _script(idmap, key, 'atk-%s-%s-stop' % (tag, fac), conds, [
        _act(idmap, key, 450, [_a(3, s=team), _a(69, i=1)]),
        _act(idmap, key, 518, [_a(11, s=player)]),
        _act(idmap, key, 520, [_a(11, s=player)]),
        _clear_flag(idmap, key, _go_flag(tag)),
    ], deactivate=0, interval=1)


def _pulse(idmap, key, tag, fac, player, skip_human):
    conds = _gate(idmap, key, player, skip_human)
    conds.append(_flag_on(idmap, key, 'atk-live'))
    conds.append(_flag_on(idmap, key, _fac_flag(tag, fac)))
    conds.append(_counter(idmap, key, 'atk-wait', _EQ, 0))
    return conds


def _orders(idmap, key, tag, team, player, enemy, waypoint):
    return [
        # 1 是 NKL 里把队伍交给遭遇战 AI 的值，0 取回来由脚本下令。
        _act(idmap, key, 450, [_a(3, s=team), _a(69, i=0)]),
        _act(idmap, key, 413, [_a(3, s=team), _a(7, s=waypoint)]),
        _act(idmap, key, 459, [_a(11, s=player), _a(11, s=enemy)]),
        _set_flag(idmap, key, _go_flag(tag)),
    ]


def _gate(idmap, key, player, skip_human):
    if not skip_human:
        return []
    return [_cond(idmap, key, 146, [
        _a(11, s=player), _a(70, s='Human'),
    ], inverted=True)]


def _counter(idmap, key, name, cmp_, n):
    return _cond(idmap, key, 1, [
        _a(4, s=name), _a(6, i=cmp_), _a(0, i=n),
    ])


def _set_counter(idmap, key, name, n):
    return _act(idmap, key, 2, [_a(4, s=name), _a(0, i=n)])


def _math(idmap, key, dest, op, src):
    # 0 加，1 减。被改的计数器在前，演算符用 57，另一个计数器在后。
    return _act(idmap, key, 291, [
        _a(4, s=dest), _a(57, i=op), _a(4, s=src),
    ])


def _clear_flag(idmap, key, name):
    return _act(idmap, key, 1, [_a(5, s=name), _a(8, i=0)])


def _team_name(fac, tag):
    # 队伍名在整张图里必须唯一，所以座位写在后面。脚本里写成 Player_1/team_atk_a_p1。
    return 'team_atk_%s_%s' % (fac, tag)


def _fac_flag(tag, fac):
    return 'atk-%s-%s' % (tag, fac)


def _go_flag(tag):
    return 'atk-go-%s' % tag


def _num(tag):
    return 'atk-n-%s' % tag
