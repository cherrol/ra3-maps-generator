# 蓝图选择与版本资料

本地地图 id 使用真实地形名，例如 `maps/陕西/`；所有版本资料统一放入本图 `histories/<版本>/`，根目录保留当前配置、蓝图镜像和活动制作源。游戏文件和发布包继续使用生成名。蓝图版本 `v1`、`v2`、`v3`、`v11` 与成品版本 `T0.7.8` 分别保留各自资料，不互相换算编号。

## 当前选定蓝图

`LAYERS` 指定新蓝图构建读取的 NPZ，路径相对本图目录，默认 `layers.npz`。输入包含 `base_z`（`GRID × GRID`）和 `z_scale`；`LAYOUT` 指定配套布局，默认 `layout.json`。

根 `layers.npz` 始终是当前选定定稿蓝图的原字节镜像。选用某个版本时，保留版本目录原件，并将其完整复制为根镜像，不重新编码 NPZ、不改变数组类型或比例、不从 `.map` 回读生成，也不把预览缓存直接改名作为蓝图。

构建在写入版本或产物前预读所选蓝图原字节。构建阶段、验证和根镜像同步都在提交前的回滚事务中；成功后提交本地版本、成品和根镜像，提交前失败恢复此前版号及根镜像，只清理本轮新建的版本产物。

可选安装在本地构建提交后执行；安装失败明确报告“构建已完成，安装失败”，不回滚已提交的同版成品和镜像。增量发布也先同步根镜像，再执行可选安装；安装失败明确报告“发布已完成，安装失败”，保留本地发布结果。发布或采用制作源前会预读所选输入，提交后的镜像同步失败仍明确报告已提交状态。历史版本操作不覆盖当前副本，来源字节相同时不重写文件。

四张现有地图显式配置 `LAYERS: "histories/v11/layers.npz"`，其历史重建布局为 `histories/v11/layout.json`。它们的日常生产仍读取 `COORDINATE_PIPELINE` 指定的完整 `authoring/source.zip`、可编辑布局与基准；整理蓝图目录或同步根镜像不改变冻结原生成品。

```text
maps/<真实地形名>/
  config.json
  version.json
  layers.npz                       当前选定定稿蓝图的原字节镜像
  histories/v1/layers.npz           已存在的早期蓝图
  histories/v2/layers.npz
  histories/v3/layers.npz
  histories/v11/layers.npz          四图当前选定的历史蓝图
  histories/v11/layout.json         同版历史重建布局
  histories/legacy/layers.npz       版本无法确认的旧输入（已有时保留）
  histories/T0.7.8/                 对应成品版本的制作快照、编辑输入和编号对照
  authoring/                       当前冻结制作源、可编辑布局与基准
  data/terrain-preview/            独立的当前预览资料与缓存
```

各图只建立实际存在的版本目录，不补造缺失版本。广东原来的无版本 `layers.npz` 另存 `histories/legacy/layers.npz`；它只有 1040×1040 的 `z` 数组，与 840×840 的 v11 蓝图格式不同。归档旧文件后再建立所选版本的根镜像，不能覆盖旧输入或把它推定为 v11。广东 `histories/v1/`、`histories/v2/`、`histories/v3/` 也只有 1040×1040 的 `z` 数组；版本目录不保证输入格式相同，不能仅修改 `LAYERS` 就把这些旧设计资料当作当前 `base_z` 蓝图。

## 版本资料归档

文件名中明确的版本前后缀归入本图 `histories/` 下对应版本目录，目录内保留功能名。例如，`v11_layers.npz` 改为 `histories/v11/layers.npz`，脚本变更报告归入对应 `T` 版本目录。历史 `edits/`、`repack/` 的版本输入同样按本图 `histories/` 下的版本目录组织。

| 当前目录 | 同版资料 |
|---|---|
| 广东 `histories/v1/`、`histories/v2/`、`histories/v3/` | 各自的 `layers.npz`、`preview.png`、`preview--line.png` |
| 四图 `histories/v11/` | `layers.npz`、历史 `layout.json`；广西另保留 `ramps.json` |
| 四图 `histories/` 下各 `T` 版本目录 | 当版 `coordinate-id-map.json`，仅保留实际存在的版本 |
| 陕西 `histories/T0.3.9/`、`histories/T0.4.0/`、`histories/T0.5.1/` | 对应脚本清理、变更或合并报告；原始游戏输入 ZIP 仍用完整原文件名 |
| 陕西 `histories/T0.5.1/` | `rules.json`、`rebuild-comparison.txt`、原名 rebuild ZIP，另有 `repack/` 本地历史辅助源码归档 |
| 陕西 `histories/T0.6.3/` | `layers.npz`、`relocate-567-568.json` |
| 陕西 `histories/T0.6.4/` | `layers.npz`、`add-377-220-309.json` |
| 陕西 `histories/T0.6.7/` | `add-four-mountains.json`、`axai-mountain-relief.u16` |
| 陕西 `histories/T0.6.9/` | `config.json`、`layout.json`、`baseline.json`、`source.zip`、`resource-distribution.json`、重做方案及 `44gg-*` 高程/材质文件 |
| 陕西 `histories/T0.7.1/`、`histories/T0.7.4/` | 分别保留 568、567 坡肩方案和对应 `.u16` |
| 陕西 `histories/T0.7.8/` | `ramp-clearance-cells.json` 及当版编号对照 |

`histories/T0.5.1/repack/` 中的旧 Cargo 辅助程序仅作本地历史源码归档，原始入口说明保留，不作为当前生产命令或测试依赖。

每版编号对照为 `maps/<id>/histories/<T版本>/coordinate-id-map.json`，例如 `maps/陕西/histories/T0.7.8/coordinate-id-map.json`；`authoring/coordinate-id-map.json` 保留当前采用来源的对照。坐标导出同时读取这些历史版本记录。

同版配套的方案、蓝图快照、布局、配置、基准、逐格高程或材质文件应一并保留，并同步活动引用。版本目录可以表示方案所依赖的来源版，不自动改成执行后产生的新版本。原始游戏发布输入 ZIP 保留完整生成名和版本文件名，制作源快照使用 `histories/T<版本>/source.zip`；所有 ZIP 的完整字节保持。历史报告中的原始来源、版本与研究源码字段保留。目录整理不递增地图版本、不重新打包或安装地图。

根镜像与版本原件需要逐字节一致；相同大小、相同数组尺寸或相近画面都不能替代该检查。更换所选蓝图时同步 `LAYERS`、配套 `LAYOUT` 和根镜像，避免一份蓝图配另一份历史布局。`use_v11_layers`、`v11_plain`、`v11_baked` 等已有算法开关不因目录名变化而改义。

## 构建与回归

四图的冻结源、旧蓝图和历史成品回归承担不同用途，不能因为日常生产使用 `authoring/source.zip` 就删除 v11 或 `T` 版本输入。文件移动后同步配方路径、编辑方案引用、测试读取位置、Git 白名单和本说明中的链接，按 [可复现构建](reproducible_builds.md) 验证干净检出。

预览 `terrain.npz` 保存真实高程、设计分层和掩码；它与构建蓝图、根镜像分别管理。生成地图后的 `assets/.../source/terrain.npz` 也不能回写成根 `layers.npz`。
