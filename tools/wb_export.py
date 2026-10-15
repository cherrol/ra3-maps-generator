# -*- coding: utf-8 -*-
"""把当前构建好的地图导出成「新地编(World Builder)」可直接导入的文件，并可选安装到地编目录。

产出（写到 assets/<地图名>/<版本>/wb/）：
  heightmap.xyz   给地编内置插件 data/scripts/ImportXYZ —— 每行 "格x 格y 世界高度Z"（全图格坐标）
  object.txt      给地编内置插件 data/scripts/loadObjFromFiles —— 每行 "类型名,可玩区x,可玩区y"（角度固定 45）
  chai_objects.txt  给本工具自带插件 CHAI_Import —— 每行 "类型名,世界x,世界y,角度(度)"（保留精确朝向）
  plugins/CHAI_Import/{Main.cs, plugin_meta.json}  精确对象导入插件
  README_导入说明.md

坐标口径（已用官方样例图核对）：
  * 高度图 elevations 的下标是**全图格坐标**（0..W-1），值就是世界高度（平原 210、水面 130）
  * 对象世界坐标 = (格 - 边框) * 10；内置插件会对文件里的值 ×10，所以它要**可玩区相对格坐标**
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ra3map import MapFile, parse_heightmap, parse_objects, props_dict  # noqa: E402
import project as proj  # noqa: E402
import terrain_gen as bt  # noqa: E402

PLUGIN_CS = r'''using System;
using System.Globalization;
using System.IO;
using MapCoreLib.Core.Asset;
using MapCoreLib.Util;

namespace MapCoreLib.Core.Scripts.ScriptFile
{
    // CHAI 专用：从 chai_objects.txt 导入对象并保留精确朝向
    // 文件格式（逗号分隔，与地编工作目录同名文件）：类型名,世界x,世界y,角度(度)
    public class Main : ScriptInterface
    {
        public void Apply(MapDataContext context)
        {
            var objectsList = context.getAsset<ObjectsList>(Ra3MapConst.ASSET_ObjectsList);
            int n = 0;
            foreach (var raw in File.ReadLines("chai_objects.txt"))
            {
                var line = raw.Trim();
                if (line.Length == 0 || line[0] == '#') continue;
                var t = line.Split(',');
                if (t.Length < 4) continue;
                double x = double.Parse(t[1], CultureInfo.InvariantCulture);
                double y = double.Parse(t[2], CultureInfo.InvariantCulture);
                double a = double.Parse(t[3], CultureInfo.InvariantCulture);
                objectsList.AddObject(context, t[0], new Vec3D(x, y), a);
                n++;
            }
        }
    }
}
'''

PLUGIN_META = '''{
    "PluginName": "CHAI_Import",
    "PluginVersion": "v1.0.0",
    "RequireFileDictionary": {
        "data/plugins/CHAI_Import/Main.cs": "Main.cs",
        "data/plugins/CHAI_Import/plugin_meta.json": "plugin_meta.json"
    }
}
'''

# 地编伴侣「微程序」：用官方库给我们的地图做混合/通行性优化
NANO_CS = r'''using System;
using System.IO;
using Dreamness.Ra3.Map.Facade.Core;

// CHAI 地图优化：自动地形混合 + 官方通行性 + 备份保存
// 用法：地编伴侣 -> 微程序 -> CHAI_OptimizeMap（可选传参 MapFilePath）
string? mapFilePath = null;

if (ArgumentDictionary != null
    && ArgumentDictionary.TryGetValue("MapFilePath", out var arg)
    && !string.IsNullOrWhiteSpace(arg))
{
    mapFilePath = arg;
}
else
{
    var dialog = Ra3MapUtils.ScriptViews.MapFileSelectorDialog.Create("选择要优化的地图 (.map)");
    dialog.ShowDialog();
    if (dialog.DialogResult == System.Windows.Forms.DialogResult.OK)
    {
        mapFilePath = dialog.Tag as string;
    }
}

if (string.IsNullOrWhiteSpace(mapFilePath))
{
    throw new Exception("未选择地图文件，程序终止。");
}

var ra3map = Ra3MapFacade.Open(mapFilePath);
var backup = ra3map.Backup();

// 1) 自动检测并写入地形混合（把硬边纹理交界变成过渡）
ra3map.AutoDetectBlendsEntireMap();

// 2) 用官方逻辑重算通行性
ra3map.UpdatePassabilityMap();

ra3map.Save();

Ra3MapUtils.ScriptViews.MsgDialog.Create(
    "CHAI 地图优化完成",
    $"地图: {Path.GetFileName(mapFilePath)}\n" +
    $"已执行: 自动地形混合 + 通行性重算\n" +
    $"备份: {backup}").ShowDialog();
'''

NANO_INFO = '''{{
    "ID": "{uid}",
    "Name": "CHAI 地图优化(混合+通行性)",
    "Description": "对 .map 执行官方库的自动地形混合与通行性重算，并自动备份。",
    "Author": "CHAI / dsh"
}}
'''


def main():
    import argparse
    ap = argparse.ArgumentParser(allow_abbrev=False)   # 否则 build_all 的 --wb 会被当成 --wb-root
    ap.add_argument('--map', dest='map_id', default=None)
    ap.add_argument('--install-wb', action='store_true',
                    help='同时复制到地编目录（data/scripts/ImportXYZ、loadObjFromFiles、data/plugins/CHAI_Import）')
    ap.add_argument('--wb-root', default=os.environ.get('RA3MAP_WB_ROOT', r'D:\app\Ra3NewWorldBuilder'))
    a, _unknown = ap.parse_known_args()      # 允许被 build_all 直接 main() 调用（会带别的参数）
    if os.environ.get('RA3MAP_WB_INSTALL') == '1':
        a.install_wb = True
    if a.map_id:
        os.environ['RA3MAP_ID'] = a.map_id

    P = proj.current(os.environ.get('RA3MAP_ID'))
    outdir = os.path.join(P.assets, 'wb')
    os.makedirs(outdir, exist_ok=True)

    m = MapFile.load(P.asset(P.map_name + '.map'))
    hm = parse_heightmap(m.get('HeightMapData'))
    W, H, B = hm['width'], hm['height'], P.BORDER
    z = [0.0] * (W * H)
    for i, v in enumerate(hm['elevations']):
        z[i] = v * P.Z_SCALE

    # ---- heightmap.xyz（全图格坐标，x 外 y 内，值=世界高度）
    p_xyz = os.path.join(outdir, 'heightmap.xyz')
    with open(p_xyz, 'w', encoding='ascii', newline='\n') as f:
        w = f.write
        for y in range(H):
            base = y * W
            for x in range(W):
                w('%d %d %.4f\n' % (x, y, z[base + x]))

    # ---- 对象
    objs = parse_objects(m.get('ObjectsList'), m.table)
    lines_simple, lines_rich, wp_lines = [], [], []
    for o in objs:
        tn = o['type_name']
        wx, wy = o['position'][0], o['position'][1]
        cx = wx / P.CELL + B             # 全图格坐标
        cy = wy / P.CELL + B
        px, py = wx / P.CELL, wy / P.CELL  # 可玩区相对格坐标（内置插件会 ×10）
        ang = float(o.get('angle') or 0.0)
        if 'Waypoint' in tn:
            nm = props_dict(o['props']).get('waypointName', '')
            wp_lines.append('# %-26s 世界(%.1f, %.1f)  格(%d, %d)' % (nm, wx, wy, round(cx), round(cy)))
            continue
        lines_simple.append('%s,%d,%d' % (tn, round(px), round(py)))
        lines_rich.append('%s,%.1f,%.1f,%.4f' % (tn, wx, wy, ang))

    p_obj = os.path.join(outdir, 'object.txt')
    with open(p_obj, 'w', encoding='ascii', newline='\n') as f:
        f.write('\n'.join(lines_simple) + '\n')
    p_rich = os.path.join(outdir, 'chai_objects.txt')
    with open(p_rich, 'w', encoding='ascii', newline='\n') as f:
        f.write('# typename,worldX,worldY,angleDeg\n')
        f.write('\n'.join(lines_rich) + '\n')

    # ---- 自带插件
    pdir = os.path.join(outdir, 'plugins', 'CHAI_Import')
    os.makedirs(pdir, exist_ok=True)
    open(os.path.join(pdir, 'Main.cs'), 'w', encoding='utf-8', newline='\n').write(PLUGIN_CS)
    open(os.path.join(pdir, 'plugin_meta.json'), 'w', encoding='utf-8', newline='\n').write(PLUGIN_META)

    # ---- 地编伴侣微程序（用官方库优化地图）
    import uuid
    ndir = os.path.join(outdir, 'nanoprogram', 'CHAI_OptimizeMap')
    os.makedirs(ndir, exist_ok=True)
    open(os.path.join(ndir, 'Main.cs'), 'w', encoding='utf-8', newline='\n').write(NANO_CS)
    open(os.path.join(ndir, 'info.json'), 'w', encoding='utf-8', newline='\n').write(
        NANO_INFO.format(uid=str(uuid.uuid5(uuid.NAMESPACE_DNS, 'chai.ra3.optimize_map'))))

    # ---- 说明
    readme = '''# 新地编导入说明（{ver}）

本目录是把本项目生成的地图导入「新地编」所需的全部文件。

## 方式一：地编内置插件（最快，2 步）
1. 打开地编 `WorldBuilder_Mod_1.12.exe`，**新建一张地图**（可玩区 {play}×{play}、边框 {border}，
   这样格数与高度图完全对齐），或打开一张已有地图。
2. 依次点两个内置插件：
   - `ImportXYZ`（导入高度图）—— 读 `heightmap.xyz`（已给出全图 {W}×{H} 格、世界高度）
   - `loadObjFromFiles`（批量放物体）—— 读 `object.txt`
   注意：这两个插件从**地编工作目录**读文件，请先把 `heightmap.xyz` / `object.txt`
   复制到 `{wbroot}\\data\\scripts\\ImportXYZ\\` 与 `...\\loadObjFromFiles\\`
   （或直接用 `--install-wb` 让本工具自动复制）。
   内置插件把对象角度固定为 45°，**矿脉的 45/135/225/315 精确朝向会丢失**。

## 方式二：本工具自带插件 CHAI_Import（推荐，保留精确朝向）
把 `plugins/CHAI_Import/` 整个复制到地编的 `data/plugins/CHAI_Import/`，
把 `chai_objects.txt` 复制到地编**工作目录**，然后在地编里运行该插件：对象带着
正确的角度导入（每行：类型名,世界x,世界y,角度）。

## 坐标口径（已用官方样例图核对，别搞反）
- 高度图 `elevations[x, y]`：**全图格坐标**（0..{Wminus}），值 = 世界高度（平原 210 / 一层 280 / 二层 350 / 海面 ≈130）。
- 对象世界坐标 = `(格 - 边框{lr}) * 10`；内置对象插件会把文件里的值再 ×10，
  所以 `object.txt` 用**可玩区相对格坐标**（0..{play}）；`chai_objects.txt` 直接用**世界坐标**。

## 本图要素
- 对象 {nobj} 个（矿脉/油井/雷达/车库/机场/精兵学院/医院/桥/装饰）
- 出生点 {nwp} 个（路径点需在地编里单独摆；下方清单给出对应格坐标）
- 高度层：海面 200 / 平原 210 / 一层 280 / 二层 350

### 出生点格坐标
{wp}
'''.format(ver=P.map_name, play=bt.PLAY, border=B, W=W, H=H, Wminus=W - 1, lr=B,
           wbroot=a.wb_root, nobj=len(lines_rich), nwp=len(wp_lines),           wp='\n'.join(wp_lines))
    open(os.path.join(outdir, 'README_导入说明.md'), 'w', encoding='utf-8', newline='\n').write(readme)

    print('世界地编导出 → %s' % outdir)
    print('  heightmap.xyz  %.1f MB（%d×%d 格）' % (os.path.getsize(p_xyz) / 1048576.0, W, H))
    print('  object.txt     %d 个对象（内置插件格式）' % len(lines_simple))
    print('  chai_objects.txt %d 个对象（带角度）' % len(lines_rich))
    print('  路径点 %d 个（已列在 README）' % len(wp_lines))

    if a.install_wb:
        import shutil
        targets = [
            (p_xyz, os.path.join(a.wb_root, 'data', 'scripts', 'ImportXYZ', 'heightmap.xyz')),
            (p_obj, os.path.join(a.wb_root, 'data', 'scripts', 'loadObjFromFiles', 'object.txt')),
            (p_rich, os.path.join(a.wb_root, 'data', 'plugins', 'CHAI_Import', 'chai_objects.txt')),
            (os.path.join(pdir, 'Main.cs'), os.path.join(a.wb_root, 'data', 'plugins', 'CHAI_Import', 'Main.cs')),
            (os.path.join(pdir, 'plugin_meta.json'),
             os.path.join(a.wb_root, 'data', 'plugins', 'CHAI_Import', 'plugin_meta.json')),
        ]
        comp = os.path.join(os.path.dirname(a.wb_root), '地编伴侣', 'current', 'data', 'nano_programs')
        if not os.path.isdir(comp):
            comp = os.path.join(a.wb_root, '地编伴侣', 'current', 'data', 'nano_programs')
        for name in ('Main.cs', 'info.json'):
            targets.append((os.path.join(ndir, name), os.path.join(comp, 'CHAI_OptimizeMap', name)))
        for src, dst in targets:
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            shutil.copy2(src, dst)
            print('  已安装 -> %s' % dst)


if __name__ == '__main__':
    main()
