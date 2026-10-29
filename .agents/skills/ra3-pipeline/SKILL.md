---
name: ra3-pipeline
description: 原生 Rust 地图构建、成品增量编辑、验收、冻结制作源、版本、发布与安装。
---

# 地图流水线

地图 id 使用真实地形名，例如 `--map 陕西`；`RA3MAP_ID` 与 `maps/.default` 同样填写真实 id（当前默认 `广西`）。输入在 `maps/<id>/`，制作资料在 `assets/<id>/<生成发布名+版本>/`，成品目录在 `dist/<id>/<生成发布名+版本>/`；ZIP 位于 `dist/<id>/<生成发布名+版本>.zip`。例如 `assets/陕西/[CHAI]千壑横原T0.7.8/`、`dist/陕西/[CHAI]千壑横原T0.7.8/`、`dist/陕西/[CHAI]千壑横原T0.7.8.zip`。目录身份不改变 `MAP_NAME` / `PUBLISH_NAME`、规则模板名、包内文件名或游戏安装目录。

使用 `cargo run --release --locked -- <命令>`。地图 id 按 `--map`、`RA3MAP_ID`、`maps/.default` 选择，必要时加 `--root`。首次检出先按 [可复现构建](../../../docs/reproducible_builds.md) 恢复并核对受控输入；运行时无需 Python。

## 两条制作路径

| 路径 | 真实输入 |
|---|---|
| 新图 | `maps/<id>/config.json`、`LAYERS` 指定的蓝图（默认 `layers.npz`，含 GRID×GRID 的 `base_z` 与 `z_scale`）及 `LAYOUT`（默认 `layout.json`）；预览缓存不是可直接构建的蓝图 |
| 冻结成品 | `COORDINATE_PIPELINE` 引用的完整源 ZIP、同版布局及基准；现有四图为 `authoring/source.zip`、`authoring/layout.json`、`authoring/baseline.json`，旧蓝图只作参考 |

先准备本图 `coordinates::Frame`。稳定声明为 `maps/<id>/coordinate-system.json`；版本声明写入 `assets/.../source/`、`coordinates/` 及 `coordinates.zip`。宽高、边框来自实际网格，格长与高度比例取本图配方；资源在格坐标内计算后使用同一 Frame 正逆转换，不按 PNG 像素或其他地图常数换算。预览设计帧另存 `data/terrain-preview/`，不覆盖已有可玩图 Frame。

## 构建与门禁

```sh
cargo run --release --locked -- build --map <id>
cargo run --release --locked -- build --map <id> --bump --note '调整布局'
cargo run --release --locked -- build --map <id> --only map --only verify
cargo run --release --locked -- accept --map <id>
```

完整阶段为 `terrain → passability → map → verify → spatial → preview → coordinates → accept → readme → dist → wb`，以代码 `STAGES` 为准。`--only`/`--skip` 可重复，不补前置产物；冻结路径执行空间增量审计，新图的 `spatial` 尚无独立审计，仍有 `verify`/`accept` 和公共坡口门禁。

公共 `templates/rules/coordinate-pipeline.json#/ramp_clearance` 检查坡面全宽、端口和沿轴双向通行区及完整矿场/中立建筑预留，当前默认两端延伸 25 格、净距 8 格。新图及显式 `RAMP_CLEARANCE` 执行零冲突；默认冻结成品按 CRC 绑定源包留档旧问题并拦截新增/恶化。打包、安装、采用源和增量发布同样检查，不能通过重设基准掩盖本轮问题。详见 [坡口规则](../../../docs/ramp_clearance.md)。

公共 `spawn_placement` 自动检查出生点对置侧别、每座基地两矿、完整预留及同层/水域连通，同样覆盖构建、回读、发布、安装和采用源。新图严格零问题；默认冻结源逐项继承已有问题，新增/恶化拦截；显式 `SPAWN_PLACEMENT` 要求严格验收。报告区分 `raw_passed` 与 `passed`，策略/来源变化使旧通过缓存失效。规则及报告入口见[公共出生点规则](../../../docs/spawn_placement.md)；现有四张成品本轮暂不调整。

公共[分级布矿](../../../docs/resource_distribution.md)采用 `templates/rules/coordinate-pipeline.json#/resource_distribution`：每位 6 个阶段，共 48 矿，家区 100 格内 3 矿，随后进入主线前段、热点与次线。新图必须提供 `RESOURCE_DISTRIBUTION` 本图计划；回读、坐标制作、成品发布和采用来源检查实际矿位及全部阶段。公共参数和计划内容绑定空间缓存、发布证明，不能复用旧规则的通过结果。

已有未配置计划的冻结源仅在矿位和出生点保持原样时沿用；移动这些位置需先补齐公共分级计划。普通矿口朝向修正不触发矿位重布。公共规则升级本身不修改成品字节、不递增地图版本；纯地形及精确绑定的历史复现保持原边界。

资源封装应同时保持原生清单、XML、纹理图集和脚本资产声明一致；不能删除清单或只拷 `.map` 绕过门禁。未知原生区块、有效资源身份和已有配套文件按成品入口保留。

## 增量编辑与采用制作源

冻结成品只修改矿、医院等对象坐标或允许的朝向时，直接修改配置指定布局的 `objects[编号−1].cell/angle`，用 `build --bump` 计算和验收本轮差异；分类清单由对象身份派生，不用 `repack` 搬动物件。

其他原生成品编辑沿用最新完整 ZIP。按任务选择 `repack`、`polish-textures`、`polish-ramps`、`polish-ramp-surfaces`、`relocate-ramps`、`repair-production` 或对应修复入口；普通 `repack` 只重新封装，不应用地形/玩法修复。具体范围见 [坐标流水](../../../docs/coordinate_pipeline.md)、[纹理规则](../../../docs/terrain_texture_rules.md)、[山地斜坡](../../../docs/mountain_ramps.md)。

核对相对旧源的实际差异、空间结果和新版本 `source/layout.json` 后，再显式采用并补齐同版资料：

```sh
cargo run --release --locked -- adopt-coordinate-source --map <id>
cargo run --release --locked -- build --map <id>
```

增量发布不自动完成坐标/WB 导出或采用新源；普通重封装缺少布局快照时先补齐实际同版布局。采用会更新基准，不能替代对本轮变化的检查。

地图内容变化递增版本；地图字节不变的工具链重构不递增。构建在版本和产物写入前预读所选蓝图；阶段、验证与根镜像同步均在提交前的回滚事务中。提交前失败恢复原版号及根镜像，仅清理本轮新建版本产物，不删除旧版本或其他地图。成功后提交本地版本、同版成品与根镜像，再执行可选安装；安装失败明确报告“构建已完成，安装失败”，不回滚本地成果。增量发布同样先同步根镜像后安装，安装失败报告“发布已完成，安装失败”，保留已提交发布结果。

`assets/` 保存制作资料，`dist/` 保存完整成品目录与确定性 ZIP。安装在已有授权范围内用 `--install`，目标为 `APPDATA/Red Alert 3/Maps/<发布名>/`。WB 默认导出资料，复制到地编另加 `--wb --wb-root <目录>` 或配置 `RA3MAP_WB_ROOT`。

验证命令及 CI 输入见 [README](../../../README.md#验证)。生产流程不包含实机检查，不把启动游戏或 GUI 测试加入构建/发布步骤。

## 蓝图与版本资料

蓝图输入由 `LAYERS` 选择，默认 `layers.npz`；根 `layers.npz` 始终与当前选定定稿蓝图原件逐字节一致，不从 `.map` 回读生成。四张现有图显式使用 `histories/v11/layers.npz`，同版历史布局在 `histories/v11/layout.json`；冻结生产来源仍为 `authoring/source.zip`。预览 `terrain.npz` 不直接替代构建蓝图。当前版本构建在本地提交前同步根镜像，增量发布在可选安装前同步；采用制作源也同步所选蓝图。历史版本操作不覆盖当前副本，来源字节相同时不重写。

蓝图版本按本图 `histories/v1/`、`histories/v2/`、`histories/v3/`、`histories/v11/` 保存，成品版本资料也保存在 `histories/T<版本>/`，各版编号对照为 `histories/<T版本>/coordinate-id-map.json`。编辑方案、蓝图/布局/配置/基准快照和高程材质配套按版本一起归档，活动引用同步新路径；原始游戏输入 ZIP 保留原文件名，制作源快照使用 `histories/T<版本>/source.zip`，所有 ZIP 字节保持。广东无法确认版本的旧 `layers.npz` 保留于 `histories/legacy/layers.npz`，另建所选版本根镜像。详见 [蓝图与版本资料](../../../docs/blueprint_versions.md)，不改变旧 `v11_*` 开关的算法语义。
