# -*- coding: utf-8 -*-
"""发布打包：只把游戏真正加载的文件放进 dist/<地图名>/，并可生成同名 zip。

游戏只需要三个文件：
    <地图名>.map        地图本体
    <地图名>_art.tga    雷达/小地图美术
    map.str             地图文本
预览图、说明、source/ 自检图等一律不进 dist。

用法：
    python tools/make_dist.py [--map bagui_dadi] [--no-zip]
"""
import argparse
import os
import shutil
import sys
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import project as proj

GAME_FILES = ('{name}.map', '{name}_art.tga', 'map.str')


def game_files(P):
    return [f.format(name=P.map_name) for f in GAME_FILES]


def make_dist(P, zip_=True, verbose=True):
    """把 assets/<地图名>/<版本>/ 里的游戏文件复制到 dist/<地图名>/<版本>/（可选打 zip）。"""
    dist_group = P.dist_group
    dst = os.path.join(dist_group, P.map_name)
    if os.path.isdir(dst):
        shutil.rmtree(dst)
    os.makedirs(dst, exist_ok=True)

    copied = []
    for name in game_files(P):
        src = os.path.join(P.assets, name)
        if not os.path.isfile(src):
            raise SystemExit('缺少游戏文件：%s（先跑 build' % src + '）')
        shutil.copy2(src, os.path.join(dst, name))
        copied.append(name)

    zip_path = None
    if zip_:
        zip_path = os.path.join(dist_group, P.map_name + '.zip')
        os.makedirs(dist_group, exist_ok=True)
        with zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as z:
            for name in copied:
                z.write(os.path.join(dst, name), os.path.join(P.map_name, name))
    if verbose:
        for name in copied:
            print('  dist/%s/%s/%s  (%d bytes)' % (
                P.base_name, P.map_name, name, os.path.getsize(os.path.join(dst, name))))
        if zip_path:
            print('  dist/%s/%s.zip  (%d bytes)' % (
                P.base_name, P.map_name, os.path.getsize(zip_path)))
    return dst, zip_path


def main():
    ap = argparse.ArgumentParser(description='打包可直接进游戏的地图文件到 dist/')
    ap.add_argument('--map', dest='map_id', default=None, help='maps/<id>/ 配方')
    ap.add_argument('--no-zip', action='store_true', help='不生成 zip')
    # 作为 build_all 的一个阶段被调用时，命令行上还有别的参数，忽略即可
    args, _unknown = ap.parse_known_args()

    P = proj.current(args.map_id, refresh=True)
    print('打包 %s -> dist/%s/%s/' % (P.map_id, P.base_name, P.map_name))
    make_dist(P, zip_=not args.no_zip)


if __name__ == '__main__':
    main()
