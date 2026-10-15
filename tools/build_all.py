# -*- coding: utf-8 -*-
"""一键构建一张地图：地形 -> 连通 -> 组装 -> 校验 -> 预览 -> 验收 -> 说明 -> 打包。

用法：
    python tools/build_all.py                 # 用默认配方
    python tools/build_all.py --map bagui_dadi
    python tools/build_all.py --map bagui_dadi --install     # 同时拷进游戏的 Maps 目录
    python tools/build_all.py --skip terrain  # 跳过某些阶段
    python tools/build_all.py --only accept   # 只验收当前版本产物

--bump 在构建前递增版本号。任一阶段失败（非 0 退出或异常）会把版本号滚回构建前，
并删掉这次新产生的版本目录，避免失败构建占掉一个版本号。
"""
import argparse
import os
import shutil
import sys
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import project as proj

STAGES = [
    ('terrain', 'terrain_gen'),
    ('passability', 'check_passability'),
    ('map', 'map_gen'),
    ('verify', 'verify_map'),
    ('preview', 'make_preview'),
    ('accept', 'accept'),         # 矿规则 / 矿场可建造 / 斜坡 / 轴向（失败则整次构建失败）
    ('readme', 'make_readme'),
    ('dist', 'make_dist'),
    ('wb', 'wb_export'),          # 导出「新地编」可导入文件（heightmap.xyz / object.txt / 插件）
]


def install(P):
    """把发布包里的三个游戏文件拷到 %APPDATA%\\Red Alert 3\\Maps\\<地图名>\\"""
    appdata = os.environ.get('APPDATA')
    if not appdata:
        print('!! 找不到 APPDATA，跳过安装')
        return None
    dst = os.path.join(appdata, 'Red Alert 3', 'Maps', P.map_name)
    os.makedirs(dst, exist_ok=True)
    for name in (P.map_name + '.map', P.map_name + '_art.tga', 'map.str'):
        src = os.path.join(P.assets, name)
        if not os.path.isfile(src):
            print('!! 缺少发布文件：', src)
            continue
        shutil.copy2(src, os.path.join(dst, name))
    print('已安装到：', dst)
    return dst


def _fail_code(exc):
    if isinstance(exc, SystemExit):
        code = exc.code
        if code is None or code == 0:
            return 0
        if isinstance(code, int):
            return code
        print(code)
        return 1
    traceback.print_exc()
    return 1


def _drop_new_version(paths):
    """删掉本次构建新产生、且构建前不存在的版本目录 / zip。"""
    for path, existed in paths:
        if existed or not os.path.exists(path):
            continue
        if os.path.isdir(path):
            shutil.rmtree(path)
        else:
            os.remove(path)
        print('已删除失败构建的产物：', path)


def main():
    ap = argparse.ArgumentParser(description='构建 RA3 自定义地图')
    ap.add_argument('--map', dest='map_id', default=None, help='maps/<id>/ 配方')
    ap.add_argument('--install', action='store_true', help='构建完成后安装到游戏地图目录')
    ap.add_argument('--skip', action='append', default=[], help='跳过阶段（可重复）')
    ap.add_argument('--only', action='append', default=[], help='只跑指定阶段（可重复）')
    ap.add_argument('--bump', action='store_true',
                    help='构建前把版本号 +0.0.1；失败则回滚，不占版本号')
    ap.add_argument('--release', action='store_true', help='配合 --bump：本次为正式版递增（V1.0.0 起）')
    ap.add_argument('--note', default='', help='本次修改说明（写入版本历史）')
    ap.add_argument('--wb', action='store_true',
                    help='导出并安装「新地编」导入文件（heightmap.xyz / object.txt / CHAI_Import 插件）')
    args = ap.parse_args()
    if args.wb:
        os.environ['RA3MAP_WB_INSTALL'] = '1'   # wb_export 据此把文件复制进地编目录
        os.environ.setdefault('RA3MAP_WB_ROOT', r'D:\app\Ra3NewWorldBuilder')
        # 'wb' 已在 STAGES 里：全量构建会自动包含它；若用户显式 --only 则尊重用户选择
        if args.only and 'wb' not in args.only:
            args.only.append('wb')

    map_id = args.map_id or proj.default_map_id()
    prev_state = None
    new_paths = []
    if args.bump or args.release:
        import version as ver
        prev_state = ver.load_state(map_id, proj.MAPS_DIR)
        st = ver.record(ver.bump_state(prev_state, release=args.release),
                        args.note or ('修改' if args.bump else '转为正式版'))
        cfg = proj.load_config(map_id)
        group, new_name = proj.release_names(cfg, ver.format_state(st))
        candidates = [
            os.path.join(proj.ASSETS_DIR, group, new_name),
            os.path.join(proj.ROOT, 'dist', group, new_name),
            os.path.join(proj.ROOT, 'dist', group, new_name + '.zip'),
        ]
        new_paths = [(p, os.path.exists(p)) for p in candidates]
        ver.save_state(map_id, st, proj.MAPS_DIR)
        # 版本号变了 -> 发布目录也变了，地形等产物必须为新版本重新生成
        if 'terrain' not in args.skip and (not args.only or 'terrain' in args.only):
            if args.only and 'terrain' not in args.only:
                args.only.append('terrain')
                print('[版本递增] 自动加入 terrain 阶段（新发布目录需要重新生成地形）')

    P = proj.current(map_id, refresh=True)
    print('=' * 72)
    print('配方 %s  ->  地图 %s   [版本 %s]' % (P.map_id, P.map_name, P.version_string))
    print('发布目录 %s' % P.assets)
    print('=' * 72)

    code = 0
    try:
        for stage, module_name in STAGES:
            if args.skip and stage in args.skip:
                print('[skip] %s' % stage)
                continue
            if args.only and stage not in args.only:
                continue
            print('\n---- %s ----' % stage)
            mod = __import__(module_name)
            rc = mod.main()
            if isinstance(rc, int) and rc != 0:
                raise SystemExit(rc)
    except (SystemExit, Exception) as exc:
        code = _fail_code(exc)
        if code and prev_state is not None:
            import version as ver
            ver.save_state(map_id, prev_state, proj.MAPS_DIR)
            print('构建失败，版本号已回滚到 %s' % ver.format_state(prev_state))
            _drop_new_version(new_paths)
    if code:
        raise SystemExit(code)

    if args.install:
        install(P)


if __name__ == '__main__':
    main()
