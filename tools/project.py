# -*- coding: utf-8 -*-
"""Project / recipe loader for the RA3 map workshop.

Layout
------
  maps/<map_id>/config.py     the recipe: name, size, geography, layout, textures, text
  maps/<map_id>/data/         recipe inputs (boundary geojson, ...)
  assets/<map name>/          published package (map + art + map.str + preview + README)
  assets/<map name>/source/   generated artefacts (terrain.npz, self-check images)
  templates/                  .map skeletons reused by recipes
  tools/                      map-agnostic toolchain
  lib/, docs/, work/          reference sources, docs, scratch

Selecting a map: ``--map <map_id>`` CLI flag, ``RA3MAP_ID`` env var, the content of
``maps/.default``, or the single map folder when only one exists.
"""
import os
import sys
import importlib.util

import version as _version

TOOLS = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(TOOLS)
MAPS_DIR = os.path.join(ROOT, 'maps')
ASSETS_DIR = os.path.join(ROOT, 'assets')
TEMPLATES_DIR = os.path.join(ROOT, 'templates')
WORK_DIR = os.path.join(ROOT, 'work')


def list_maps():
    if not os.path.isdir(MAPS_DIR):
        return []
    return sorted(d for d in os.listdir(MAPS_DIR)
                  if os.path.isfile(os.path.join(MAPS_DIR, d, 'config.py')))


def default_map_id():
    for candidate in (os.environ.get('RA3MAP_ID'), _file_default()):
        if candidate and candidate in list_maps():
            return candidate
    maps = list_maps()
    if len(maps) == 1:
        return maps[0]
    raise SystemExit('请用 --map <map_id> 指定地图；可用：' + ', '.join(maps))


def _file_default():
    p = os.path.join(MAPS_DIR, '.default')
    if os.path.isfile(p):
        return open(p, encoding='utf-8').read().strip()
    return None


def release_names(cfg, version_string):
    """返回 (总目录名, 版本目录名)。

    总目录始终是 MAP_NAME。某张图若写了 PUBLISH_NAME，版本目录用那个名字加版本号，
    仍放在 MAP_NAME 下面。没写的图与原来相同。
    """
    group = getattr(cfg, 'MAP_NAME', '')
    leaf = getattr(cfg, 'PUBLISH_NAME', None) or group
    return group, leaf + version_string


def load_config(map_id):
    path = os.path.join(MAPS_DIR, map_id, 'config.py')
    if not os.path.isfile(path):
        raise SystemExit('找不到配方：' + path)
    spec = importlib.util.spec_from_file_location('ra3map_cfg_' + map_id, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


class Project:
    """Resolved paths + recipe for one map."""

    def __init__(self, map_id=None, cfg=None):
        self.map_id = map_id or default_map_id()
        self.cfg = cfg or load_config(self.map_id)
        self.dir = os.path.join(MAPS_DIR, self.map_id)
        self.data = os.path.join(self.dir, 'data')
        # 版本号：测试版 T0.0.1 起，正式版 V1.0.0 起（见 tools/version.py）
        self.version = _version.load_state(self.map_id, MAPS_DIR)
        self.version_string = _version.format_state(self.version)
        self.base_name, self.map_name = release_names(self.cfg, self.version_string)
        # 每张地图一个总目录，各版本再各自一层：
        #   assets/<地图名>/<发布名+版本>/          （工程产物）
        #   dist/<地图名>/<发布名+版本>[/.zip]      （发布包）
        # 发布名默认就是地图名。写了 PUBLISH_NAME 的图用那个名字，仍放在地图名下面。
        self.assets_group = os.path.join(ASSETS_DIR, self.base_name)
        self.dist_group = os.path.join(ROOT, 'dist', self.base_name)
        self.assets = os.path.join(self.assets_group, self.map_name)
        self.source = os.path.join(self.assets, 'source')
        tpl = getattr(self.cfg, 'TEMPLATE', None)
        if tpl:
            self.template = tpl if os.path.isabs(tpl) else os.path.join(TEMPLATES_DIR, tpl)
        else:
            self.template = None

    # recipe values are available directly on the project object
    def __getattr__(self, item):
        if item.startswith('_') or item in ('cfg', 'dir', 'data', 'assets', 'assets_group',
                                            'dist_group', 'source', 'base_name', 'map_name',
                                            'map_id', 'template', 'version', 'version_string'):
            raise AttributeError(item)
        cfg = self.__dict__.get('cfg')
        if cfg is not None and hasattr(cfg, item):
            return getattr(cfg, item)
        raise AttributeError(item)

    def render(self, text):
        """把配方文本里的 {version} / {name} 占位符替换成当前版本。"""
        if not isinstance(text, str):
            return text
        return text.replace('{version}', self.version_string).replace('{name}', self.map_name)

    def ensure_dirs(self):
        os.makedirs(self.source, exist_ok=True)
        return self

    def data_file(self, name):
        p = os.path.join(self.data, name)
        if os.path.isfile(p):
            return p
        # legacy fallback: a recipe data file that still lives in the asset bundle
        p2 = os.path.join(self.source, name)
        if os.path.isfile(p2):
            return p2
        # 共用模板。某张图 data/ 里的同名文件优先。
        p3 = os.path.join(TEMPLATES_DIR, name)
        if os.path.isfile(p3):
            return p3
        raise SystemExit('缺少配方数据文件：' + p)

    def asset(self, name):
        return os.path.join(self.assets, name)

    def source_file(self, name):
        return os.path.join(self.source, name)

    def __repr__(self):
        return '<Project %s (%s)>' % (self.map_id, self.map_name)


_CURRENT = None


def current(map_id=None, refresh=False):
    """Process-wide project (so several tools share one selection)."""
    global _CURRENT
    if _CURRENT is None or refresh or (map_id and map_id != _CURRENT.map_id):
        _CURRENT = Project(map_id).ensure_dirs()
    return _CURRENT


def add_map_argument(parser):
    parser.add_argument('--map', dest='map_id', default=None,
                        help='配方 id（maps/<id>/）；默认取 RA3MAP_ID / maps/.default')
    return parser
