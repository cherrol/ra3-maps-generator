---
name: ra3-authoring
description: >-
  根据真实地形资料制作三张公共地形预览，或将定稿推进为可玩地图；
  也用于新图规划。名称查项目文档，现有成品沿用冻结原生来源。
---

# 从资料到地图

目录与展示名称分开：按系列映射把真实地形名作为 id，例如 `maps/陕西/`；输入“陕西”或“千壑横原”均使用该目录，预览标题保留“九州大地·千壑横原”。可玩命令用 `--map 陕西`，`maps/.default` 填真实 id（当前 `广西`）；不创建生成名目录。

先区分本轮交付：纯地形预览、新图可玩制作、已有成品修改。预览或坐标声明存在，不表示该图已有可玩配方。当前入口为原生 Rust，配方与布局均为 JSON。

## 地形预览

用户说「以 xxx 为蓝图制作游戏地形图」时，识别本次提供的参考图/高程资料，按 [系列命名](../../../docs/九州大地命名.md) 定名。由 Codex 准备内部参数；仅有图片时按图设计，不将目测分层称为测量高程。

```sh
cargo run --release --locked -- terrain-preview <蓝图原型或图名> <高程NPZ>
cargo run --release --locked -- terrain-preview-centers <蓝图原型或图名>
```

第一条用于高程输入；第二条在已有预览及中心资料上生成可选中心图。中心图需要有效的 `bounds_wgs84` 地理范围及 `data/terrain-preview/centers.json` 中的 WGS84 中心点；命令不获取中心数据，由 Codex 按可靠资料准备，缺少地理配准时不能猜坐标。按 [公共预览标准](../../../docs/terrain_preview.md) 确定矩形取景、全范围分层、水域与北向，主体边界外但游戏框内的真实地形也纳入。高程输入应按可靠海洋/湖泊轮廓准备 `water_mask`；缺失时工具回退到 `elevation_m <= 0`，不能据此宣称真实水域已经核对。

预览水域避免细长条、分叉和碎水：优先就近合并为宽阔、闭合的水面，无法合理合并的并入附近陆地层。伸入水域的细长陆地同样整理；小岛、小水域按整圈边界均可双向通向相邻层级设计。具体整理、原始资料保留与同步重绘见 [游戏水域整理](../../../docs/terrain_preview.md#游戏水域整理)。

基础交付固定为 `maps/<id>/source.png`、`preview.png`、`preview--line.png`，三图同取景、同画幅、同水域。中心名称、标点及边界另在 `preview--center--line.png` 展示。原始输入、参数、缓存与设计坐标帧放在 `data/terrain-preview/`。

纯预览不创建玩法布局、递增版本或安装地图，也不改已有可玩图的冻结制作源。预览设计帧与可玩 Frame 分开保存，不能按 PNG 像素反推游戏格数。

## 推进可玩制作

- 新图建立 `maps/<id>/config.json`，布局读取 `LAYOUT`（默认 `layout.json`）；当前生成器读取 `LAYERS`（默认 `layers.npz`，现有四图为 `histories/v11/layers.npz`）中的 `base_z`（GRID×GRID）与 `z_scale`，配方需启用声明式蓝图。公共预览的 `terrain.npz` 不直接替代该输入。布局显式定义出生点、斜坡、资源、对象与属性，不从 DEM 或预览自动搜索布局。
- 启用 `COORDINATE_PIPELINE` 的成品使用完整 `authoring/source.zip`、配置指定的可编辑布局及空间基准；现有四图使用 `authoring/layout.json`。不从旧蓝图覆盖定稿高度、斜坡或脚本。
- 资源落点前建立本图 `coordinates::Frame`，保存宽高、边框、格长与高度比例，资源在格坐标中计算后统一换算至世界坐标。矩形预览推进可玩图时先核对生成路径支持的网格；坐标帧支持矩形不代表新图生成器已支持矩形地图。
- 依次确定地形/水域、坡道及双向通行预留，将出生点与两座开局近矿共同选址，再放油井、外围资源与中立建筑、装饰。按[公共出生点规则](../../../docs/spawn_placement.md)选择东/西、南/北或东南/西北对置，P1–P4 位于东、南或东南侧；以中间线分侧后，两队所占陆地面积和水域面积各自相差不超过 20%，口径见公共规则；每个出生点及两矿完整可用空间须同处一片连续平层或水面。矿、油井、医院等按完整建筑或矿场占地避让坡面全宽及轴向端口区；公共策略和本图覆盖决定距离。
- 新图修改分类清单时同步对应对象；冻结模式按原生对象身份派生资源清单。已确认出生点、地形和现有对象仅在本轮要求内修改。

新图及矿位/出生点重布必须按[公共矿位分布规则](../../../docs/resource_distribution.md)制作：八个出生位各 6 矿，共 48 矿；100 格家区内 3 矿，第 4–6 矿分布在主线前段、争夺热点与次线。以本图通行地形规划四条主次路线，通过 `RESOURCE_DISTRIBUTION` 提交完整计划，不照搬陕西的具体矿位或配对。已有冻结图缺少计划时仅沿用原样构建，重新布置前补齐计划。

玩法读取本图 `RULESET` 与原生成品脚本。公共赠送规则保留原触发条件、归属及主备避让，不额外注入位置分队、四人落座、现金显示或主动进攻。

出生点用 `ra3-spawns`，资源用 `ra3-resources`，高度/纹理/坡形用 `ra3-terrain`，构建与冻结源衔接用 `ra3-pipeline`。具体数据结构见 [Rust 配方](../../../docs/rust_migration.md)，坐标与来源约束见 [坐标流水](../../../docs/coordinate_pipeline.md)。生产流程不包含实机检查步骤。

## 蓝图与版本资料

蓝图输入由 `LAYERS` 选择，默认 `layers.npz`；根 `layers.npz` 始终与当前选定定稿蓝图原件逐字节一致，不从 `.map` 回读生成。四张现有图显式使用 `histories/v11/layers.npz`，同版历史布局在 `histories/v11/layout.json`；冻结生产来源仍为 `authoring/source.zip`。预览 `terrain.npz` 不直接替代构建蓝图。

蓝图版本按本图 `histories/v1/`、`histories/v2/`、`histories/v3/`、`histories/v11/` 保存，成品版本资料也保存在 `histories/T<版本>/`，各版编号对照为 `histories/<T版本>/coordinate-id-map.json`。编辑方案、蓝图/布局/配置/基准快照和高程材质配套按版本一起归档，活动引用同步新路径；原始游戏输入 ZIP 保留原文件名，制作源快照使用 `histories/T<版本>/source.zip`，所有 ZIP 字节保持。广东无法确认版本的旧 `layers.npz` 保留于 `histories/legacy/layers.npz`，另建所选版本根镜像。详见 [蓝图与版本资料](../../../docs/blueprint_versions.md)，不改变旧 `v11_*` 开关的算法语义。
