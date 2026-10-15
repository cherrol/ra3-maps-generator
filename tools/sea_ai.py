# -*- coding: utf-8 -*-
"""固定 AI 的科技，以及水上固定 AI 的海军和空军。

FORCE_SEA_AI 打开时：每个固定 AI 都按 AXAI 的时间升科技；
只有落在水里的固定 AI 才强制造船厂和空军。陆地 AI 不强制出海军。
船厂禁令由 ALLOW_NAVY 关掉，见 unlock_navy。

时间照 AXAI：开局 180 秒买 T2，360 秒造科技中心，科技中心落地后再买 T3。
帝国科技中心由基地车建造；苏联用建造动作放 soviettechstructure。
脚本写进该玩家自己的列表，不改模板文件。
"""
import struct

from nkl_scripts import _ensure_names, _walk

# 这些脚本在开局用「禁止建造」锁船厂或舰船。ALLOW_NAVY 时关掉。
NAVY_BAN_SCRIPTS = (
    'p5 b t3y',
    'p6 b t3y',
    'ban niuwa',
    'ban yard 1',
    'p5 a yard ban',
    'p6 a yard ban',
    'p7 b t3y',
    'ban yard',
    'ban niu gound',
)

# 比较：0 小于，3 大于等于。和 AXAI 脚本里的用法一致。
_LT = 0
_GE = 3
# AXAI 船厂1-T2 / 重工1-T2 的 /Game Start 门槛，高科的 Game Start 门槛。
T2_SECONDS = 180.0
TECH_SECONDS = 360.0


def unlock_navy(mmap):
    """关掉 NKL 里开局禁止船厂和舰船的脚本。名字对不上就停。"""
    from map_gen import deactivate_named_scripts
    deactivate_named_scripts(mmap, NAVY_BAN_SCRIPTS)


def force_sea_ai(mmap, yards, factions):
    """每个固定 AI 升科技。水上的另外强制造船厂和空军。"""
    idmap, _ = _ensure_names(mmap, {
        'Script', 'OrCondition', 'Condition', 'ScriptAction', 'X',
    })
    for yard in yards:
        faction = factions.get(yard['player'], '')
        spec = _spec(faction)
        if spec is None:
            raise SystemExit('固定 AI %s 的阵营 %s 没有科技脚本' % (
                yard['player'], faction or '（空）'))
        blob = _pack(yard, spec, idmap, water=bool(yard.get('water')))
        _append_player_list(mmap, yard['slot'], blob)
        if yard.get('water'):
            what = '船厂+空军，并升科技'
        else:
            what = '升科技'
        has_t2 = any(spec.get(k) for k in ('buy_yard_t2', 'buy_air_t2', 'buy_bar_t2'))
        t2 = ('，T2 %d 秒' % int(T2_SECONDS)) if has_t2 else ''
        print('  [科技] %s %s：%s%s，科技中心 %d 秒' % (
            yard['city'], yard['player'], what, t2, int(TECH_SECONDS)))


def _spec(faction):
    if faction == 'PlayerTemplate:Japan':
        return _japan()
    if faction == 'PlayerTemplate:Soviet':
        return _soviet()
    if faction in ('PlayerTemplate:Allied', 'PlayerTemplate:Allies'):
        return _allied()
    return None


def _japan():
    return {
        'yard': 'japannavalyard',
        'air': 'japanwarfactory',
        'tech': 'japantechstructure',
        'tech_cmd': 'Command_ConstructJapanTechStructureEgg',
        'yard_t2': 'upgrade_japannavalyardtech2',
        'yard_t3': 'upgrade_japannavalyardtech3',
        'air_t2': 'upgrade_japanwarfactorytech2',
        'air_t3': 'upgrade_japanwarfactorytech3',
        'buy_yard_t2': 'Command_PurchaseJapanNavalYardTech2',
        'buy_yard_t3': 'Command_PurchaseJapanNavalYardTech3',
        'buy_air_t2': 'Command_PurchaseJapanWarFactoryTech2',
        'buy_air_t3': 'Command_PurchaseJapanWarFactoryTech3',
        'bar': 'japanbarracks',
        'bar_t2': 'upgrade_japanbarrackstech2',
        'bar_t3': 'upgrade_japanbarrackstech3',
        'buy_bar_t2': 'Command_PurchaseJapanBarracksTech2',
        'buy_bar_t3': 'Command_PurchaseJapanBarracksTech3',
        't1_navy': ('Command_ConstructJapanNavyScoutShip',),
        't2_navy': ('Command_ConstructJapanAntiVehicleShip',
                    'Command_ConstructJapanAntiAirShip'),
        't3_navy': ('Command_ConstructJapanAntiStructureShip',
                    'Command_ConstructJapanAntiAirShip'),
        'air_units': ('Command_ConstructJapanAntiInfantryVehicle',),
    }


def _soviet():
    # 苏联空军是机场。没有和帝国一样的分建筑升级命令，科技中心用建造动作。
    return {
        'yard': 'sovietnavalyard',
        'air': 'sovietairfield',
        'tech': 'soviettechstructure',
        'tech_cmd': None,
        'yard_t2': '',
        'yard_t3': '',
        'air_t2': '',
        'air_t3': '',
        'buy_yard_t2': None,
        'buy_yard_t3': None,
        'buy_air_t2': None,
        'buy_air_t3': None,
        't1_navy': ('Command_ConstructSovietAntiNavyShipTech1',),
        't2_navy': ('Command_ConstructSovietAntiNavyShipTech2',
                    'Command_ConstructSovietAntiAirShip'),
        't3_navy': (),
        'air_units': ('Command_ConstructSovietFighterAircraft',),
    }


def _allied():
    return {
        'yard': 'alliednavalyard',
        'air': 'alliedairfield',
        'tech': 'alliedtechstructure',
        'tech_cmd': None,
        'yard_t2': '',
        'yard_t3': '',
        'air_t2': '',
        'air_t3': '',
        'buy_yard_t2': None,
        'buy_yard_t3': None,
        'buy_air_t2': None,
        'buy_air_t3': None,
        't1_navy': ('Command_ConstructAlliedAntiNavyShipTech1',),
        't2_navy': ('Command_ConstructAlliedAntiAirShip',),
        't3_navy': (),
        'air_units': ('Command_ConstructAlliedFighterAircraft',),
    }


def _pack(yard, spec, idmap, water):
    slot = yard['slot']
    player = yard['player']
    team = yard['team']
    mcv = yard['mcv']
    clock = 'sea-clock-%d' % slot
    yard_name = 'sea-yard-%d' % slot
    air_name = 'sea-air-%d' % slot
    key = idmap['X']
    parts = []

    def add(name, conds, actions, deactivate=0, interval=10):
        parts.append(_script(idmap, key, name, conds, actions, deactivate, interval))

    add('sea%d-clock' % slot, [_true(idmap, key)], [
        _act(idmap, key, 21, [_a(4, s=clock), _a(1, f=0.01)]),
    ], deactivate=1, interval=0)

    if water:
        add('sea%d-yard' % slot, [
            _count(idmap, key, player, spec['yard'], _LT, 1),
        ], [
            _act(idmap, key, 371, [
                _a(61, s=spec['yard']), _a(11, s=player), _a(3, s=team), _a(54, s=yard_name),
            ]),
        ], interval=1)

        add('sea%d-air' % slot, [
            _count(idmap, key, player, spec['air'], _LT, 1),
        ], [
            _act(idmap, key, 371, [
                _a(61, s=spec['air']), _a(11, s=player), _a(3, s=team), _a(54, s=air_name),
            ]),
        ], interval=1)

    def produce(tag, obj_name, upgrades_have, upgrades_lack, commands):
        if not commands:
            return
        conds = [_has_obj(idmap, key, player, obj_name)]
        for up in upgrades_have:
            conds.append(_upgrade(idmap, key, obj_name, up, inverted=False))
        for up in upgrades_lack:
            conds.append(_upgrade(idmap, key, obj_name, up, inverted=True))
        add('sea%d-%s' % (slot, tag), conds, [
            _act(idmap, key, 192, [_a(14, s=obj_name), _a(34, s=cmd)])
            for cmd in commands
        ])

    if water:
        produce('navy1', yard_name, (), _ups(spec, 'yard', (2, 3)), spec['t1_navy'])
        produce('navy2', yard_name, _ups(spec, 'yard', (2,)), _ups(spec, 'yard', (3,)), spec['t2_navy'])
        produce('navy3', yard_name, _ups(spec, 'yard', (3,)), (), spec['t3_navy'])
        produce('jets', air_name, (), (), spec['air_units'])

    def buy(tag, obj_name, upgrade, command, extra):
        if not command or not upgrade:
            return
        conds = [_has_obj(idmap, key, player, obj_name),
                 _upgrade(idmap, key, obj_name, upgrade, inverted=True)]
        conds.extend(extra)
        add('sea%d-%s' % (slot, tag), conds, [
            _act(idmap, key, 192, [_a(14, s=obj_name), _a(34, s=command)]),
        ])

    clock_t2 = [_timer(idmap, key, clock, T2_SECONDS)]
    has_tech = [_count(idmap, key, player, spec['tech'], _GE, 1)]
    if water:
        buy('yard-t2', yard_name, spec['yard_t2'], spec['buy_yard_t2'], clock_t2)
        buy('yard-t3', yard_name, spec['yard_t3'], spec['buy_yard_t3'], has_tech)
        buy('air-t2', air_name, spec['air_t2'], spec['buy_air_t2'], clock_t2)
        buy('air-t3', air_name, spec['air_t3'], spec['buy_air_t3'], has_tech)
    elif spec.get('buy_air_t2'):
        # 陆地也升重工，但不强制出空军。
        add('sea%d-air' % slot, [
            _count(idmap, key, player, spec['air'], _LT, 1),
        ], [
            _act(idmap, key, 371, [
                _a(61, s=spec['air']), _a(11, s=player), _a(3, s=team), _a(54, s=air_name),
            ]),
        ], interval=1)
        buy('air-t2', air_name, spec['air_t2'], spec['buy_air_t2'], clock_t2)
        buy('air-t3', air_name, spec['air_t3'], spec['buy_air_t3'], has_tech)
    if spec.get('bar'):
        bar_name = 'sea-bar-%d' % slot
        add('sea%d-bar' % slot, [
            _count(idmap, key, player, spec['bar'], _LT, 1),
        ], [
            _act(idmap, key, 371, [
                _a(61, s=spec['bar']), _a(11, s=player), _a(3, s=team), _a(54, s=bar_name),
            ]),
        ], interval=1)
        buy('bar-t2', bar_name, spec['bar_t2'], spec['buy_bar_t2'], clock_t2)
        buy('bar-t3', bar_name, spec['bar_t3'], spec['buy_bar_t3'], has_tech)

    tech_conds = [
        _count(idmap, key, player, spec['tech'], _LT, 1),
        _timer(idmap, key, clock, TECH_SECONDS),
    ]
    if spec['tech_cmd']:
        add('sea%d-tech' % slot, tech_conds, [
            _act(idmap, key, 192, [_a(14, s=mcv), _a(34, s=spec['tech_cmd'])]),
        ], deactivate=1, interval=0)
    else:
        add('sea%d-tech' % slot, tech_conds, [
            _act(idmap, key, 371, [
                _a(61, s=spec['tech']), _a(11, s=player), _a(3, s=team),
                _a(54, s='sea-tech-%d' % slot),
            ]),
        ], deactivate=1, interval=0)

    return b''.join(parts)


def _ups(spec, kind, levels):
    out = []
    for lv in levels:
        name = spec['%s_t%d' % (kind, lv)]
        if name:
            out.append(name)
    return out


def _u16(s):
    raw = s.encode('latin-1')
    return struct.pack('<H', len(raw)) + raw


def _a(typ, i=0, f=0.0, s=''):
    return struct.pack('<Iif', typ, int(i), float(f)) + _u16(s)


def _asset(idmap, name, ver, body):
    return struct.pack('<IHI', idmap[name], ver, len(body)) + body


def _key(idx):
    return bytes((3, idx & 0xFF, (idx >> 8) & 0xFF, (idx >> 16) & 0xFF))


def _cond(idmap, key, ctype, args, inverted=False):
    body = struct.pack('<I', ctype) + _key(key) + struct.pack('<I', len(args))
    for a in args:
        body += a
    body += struct.pack('<II', 1, 1 if inverted else 0)
    return _asset(idmap, 'Condition', 6, body)


def _act(idmap, key, atype, args):
    body = struct.pack('<I', atype) + _key(key) + struct.pack('<I', len(args))
    for a in args:
        body += a
    body += struct.pack('<I', 1)
    return _asset(idmap, 'ScriptAction', 3, body)


def _true(idmap, key):
    return _cond(idmap, key, 3, ())


def _count(idmap, key, player, obj, cmp_, n):
    return _cond(idmap, key, 49, [
        _a(11, s=player), _a(6, i=cmp_), _a(0, i=n), _a(61, s=obj),
    ])


def _has_obj(idmap, key, player, name):
    return _cond(idmap, key, 25, [_a(11, s=player), _a(14, s=name)])


def _upgrade(idmap, key, obj, upgrade, inverted):
    return _cond(idmap, key, 138, [_a(14, s=obj), _a(33, s=upgrade)], inverted)


def _timer(idmap, key, flag, seconds):
    return _cond(idmap, key, 82, [_a(4, s=flag), _a(6, i=_GE), _a(1, f=seconds)])


def _script(idmap, key, name, conds, actions, deactivate, interval):
    body = _u16(name) + _u16('') + _u16('') + _u16('')
    body += bytes((1, 1 if deactivate else 0, 1, 1, 1, 0))
    body += struct.pack('<I', int(interval))
    body += bytes((0, 0)) + struct.pack('<i', 0) + bytes((1,)) + _u16('') + _u16('X')
    body += _asset(idmap, 'OrCondition', 1, b''.join(conds))
    body += b''.join(actions)
    return _asset(idmap, 'Script', 4, body)


def _append_player_list(mmap, slot, blob):
    """接到「Player N Set Up」所在的那份玩家脚本列表末尾。"""
    chunk = mmap.get('PlayerScriptsList')
    data = bytearray(chunk.payload)
    table = mmap.table
    needle = ('Player %d Set Up' % slot).encode('latin-1')
    pat = struct.pack('<H', len(needle)) + needle
    p = 0
    found = None
    while p + 10 <= len(data):
        ni = struct.unpack_from('<I', data, p)[0]
        sz = struct.unpack_from('<I', data, p + 6)[0]
        body, pend = p + 10, p + 10 + sz
        if table.get(ni) == 'ScriptList' and data.find(pat, body, pend) >= 0:
            found = (p, sz, pend)
            break
        p = pend
    if found is None:
        raise SystemExit('没有找到 Player %d 的脚本列表' % slot)
    pos, sz, pend = found
    data[pend:pend] = blob
    data[pos + 6:pos + 10] = struct.pack('<I', sz + len(blob))
    chunk.payload = bytes(data)
    _walk(blob, 0, len(blob), table, lambda *a: None)
