# -*- coding: utf-8 -*-
"""版本号逻辑（每张地图一份状态，存在 maps/<id>/version.json）。

规则
----
* 测试版： ``T<major>.<minor>.<patch>``，从 ``T0.0.1`` 起，每次修改 ``+0.0.1``
* 正式版： ``V<major>.<minor>.<patch>``，从 ``V1.0.0`` 起，每次 ``+0.0.1``
* 递增按三位十进制进位：0.0.9 -> 0.1.0 -> … -> 0.9.9 -> 1.0.0
* 发布名 = 配方里的 ``MAP_NAME`` + 版本号，例如 ``[CHAI]夹川汇潮T0.0.1``

CLI
---
    python tools/version.py --show
    python tools/version.py --bump --note "调整矿脉位置"      # 测试版 +0.0.1
    python tools/version.py --release                        # 转正式版 V1.0.0
    python tools/version.py --bump --release --note "修复卡位"  # 正式版 +0.0.1
    python tools/version.py --set T0.0.5 --note "回滚"
"""
import argparse
import datetime
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

STATE_FILE = 'version.json'
DEFAULT = dict(stage='T', major=0, minor=0, patch=1, history=[],
               note='初始测试版（版本号机制建立前的构建不计入）')


# ------------------------------------------------------------------ 纯逻辑
def format_state(st):
    return '%s%d.%d.%d' % (st['stage'], st['major'], st['minor'], st['patch'])


def parse_version(text):
    text = text.strip()
    if not text:
        raise ValueError('空版本号')
    stage = text[0].upper()
    if stage not in ('T', 'V'):
        raise ValueError('版本号必须以 T（测试）或 V（正式）开头：%s' % text)
    nums = text[1:].split('.')
    if len(nums) != 3 or not all(n.isdigit() for n in nums):
        raise ValueError('版本号必须形如 T0.0.1 / V1.0.0：%s' % text)
    return dict(stage=stage, major=int(nums[0]), minor=int(nums[1]), patch=int(nums[2]))


def _add_patch(st, step=1):
    total = (st['major'] * 100) + (st['minor'] * 10) + st['patch'] + step
    st = dict(st)
    st['major'] = total // 100
    st['minor'] = (total // 10) % 10
    st['patch'] = total % 10
    return st


def bump_state(st, release=False, step=1):
    """测试版 -> 测试版 +0.0.1；release=True 时先转正式版 V1.0.0，再按需递增。"""
    st = dict(st)
    if release and st['stage'] != 'V':
        st = dict(st, stage='V', major=1, minor=0, patch=0)
        if step:
            st = _add_patch(st, step)
        return st
    return _add_patch(st, step)


def promote_release(st):
    st = dict(st, stage='V', major=1, minor=0, patch=0)
    return st


# ------------------------------------------------------------------ 状态读写
def state_path(map_dir):
    return os.path.join(map_dir, STATE_FILE)


def load_state(map_id, maps_dir):
    p = state_path(os.path.join(maps_dir, map_id))
    if os.path.isfile(p):
        with open(p, encoding='utf-8') as f:
            st = json.load(f)
        st.setdefault('history', [])
        return st
    st = dict(DEFAULT)
    st['history'] = [dict(version=format_state(st),
                          date=datetime.date.today().isoformat(),
                          note=st.pop('note'))]
    return st


def save_state(map_id, st, maps_dir):
    d = os.path.join(maps_dir, map_id)
    os.makedirs(d, exist_ok=True)
    with open(state_path(d), 'w', encoding='utf-8') as f:
        json.dump(st, f, ensure_ascii=False, indent=2)
        f.write('\n')


def record(st, note=''):
    st = dict(st)
    st.setdefault('history', [])
    st['history'] = list(st['history']) + [dict(version=format_state(st),
                                                date=datetime.date.today().isoformat(),
                                                note=note)]
    return st


# ------------------------------------------------------------------ CLI
def main():
    import project as proj

    ap = argparse.ArgumentParser(description='地图版本号（测试 T / 正式 V）')
    ap.add_argument('--map', dest='map_id', default=None)
    ap.add_argument('--show', action='store_true', help='显示当前版本与历史')
    ap.add_argument('--bump', action='store_true', help='版本 +0.0.1')
    ap.add_argument('--release', action='store_true', help='转正式版（配合 --bump 则再 +0.0.1）')
    ap.add_argument('--set', dest='set_to', default=None, help='直接设为指定版本，如 T0.0.5')
    ap.add_argument('--note', default='', help='本次修改说明（记入历史）')
    args, _unknown = ap.parse_known_args()

    map_id = args.map_id or proj.default_map_id()
    st = load_state(map_id, proj.MAPS_DIR)

    if args.set_to:
        st = dict(parse_version(args.set_to), history=st.get('history', []))
        st = record(st, args.note or '手动指定版本')
        changed = True
    elif args.release and not args.bump and st['stage'] != 'V':
        st = record(promote_release(st), args.note or '转为正式版')
        changed = True
    elif args.bump or args.release:
        st = record(bump_state(st, release=args.release), args.note or ('修改' if args.bump else ''))
        changed = True
    else:
        changed = False

    if changed:
        save_state(map_id, st, proj.MAPS_DIR)

    cfg_name = None
    try:
        cfg_name = proj.load_config(map_id).MAP_NAME
    except Exception:
        pass
    print('配方      : %s' % map_id)
    print('当前版本  : %s' % format_state(st))
    if cfg_name:
        print('发布名    : %s%s' % (cfg_name, format_state(st)))
    if args.show or not changed:
        print('历史:')
        for h in st.get('history', [])[-12:]:
            print('   %-10s %s  %s' % (h.get('version', ''), h.get('date', ''), h.get('note', '')))


if __name__ == '__main__':
    main()
