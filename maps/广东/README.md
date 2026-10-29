# 九州大地·千流叩海

## 800×800 正方形预览（2026-10-29）

当前可玩范围 **800×800 格**，四周各 20 格制作边框，完整设计网格 **840×840 格**。保留原地理取景和北向，宽高分别重采样；图例与留白不计入格数。已沿用现有陆地层号、海拔切点及设计岸线，四张预览同步重绘，行政中心按原 WGS84 经纬度重新定位。PNG 画布为 1104×1002 像素。

[游戏分层](preview.png) · [边界版](preview--line.png) · [行政中心版](preview--center--line.png) · [尺寸与图片核对](data/terrain-preview/square-verification.json)。调整前分层、报告、参数和旧统计保存在 `data/terrain-preview/raw/before-square-800/`；下文早期整理的处理格数按当时网格记录。

## 2026-10-29 水陆岸线补充整理

收回伸入水域的细长陆地、窄颈半岛和尖细尾部，保留宽阔主体及圆整小岛；基础图与中心标点图同步更新。小岛、小水域按整圈边界连续、双向跨层通行设计，不能只留少数坡口。当前为纯地形预览，全周过渡需在可玩制作时落实到高度与导航。

[岸线规则](data/terrain-preview/shoreline-refinement.json) · [整理记录](data/terrain-preview/shoreline-refinement-report.json) · [全周过渡设计](data/terrain-preview/shoreline-transition-plan.json)

## 2026-10-29 游戏预览水域整理

按[公共水域整理规则](../../docs/terrain_preview.md#游戏水域整理)，相邻分叉与碎水优先就近合并、圆整为宽阔闭合水面，无法合理合并的细小水域并入附近陆地层；开阔海域保留并清理窄水道。当前基础图与中心标点图均使用设计水域。真实高程、边界、取景和原始来源掩码保持一致，岸线调整不作为真实水体面积资料。

[当前预览](preview--line.png) · [中心标点版](preview--center--line.png) · [本图整理规则](data/terrain-preview/water-shaping.json) · [水域统计](data/terrain-preview/water-report.json) · [验收记录](data/terrain-preview/water-verification.json)

整理前完整输入保存在 `data/terrain-preview/raw/reference-before-water-shaping.npz`；原始筛选政策和统计另存 `raw/before-water-shaping-*`。下列早期水域说明保留为来源记录，当前水面以本节链接为准。

## 地形预览（全国统一分层）

2026-10-28 按全国基础海拔切点 **200、500、1500、4000 米**重新生成三张同取景预览。游戏范围为 **800×800 格**，四周制作边框各 20 格，北朝上；省界只作参考，矩形内的周边真实地形一并切层。

当前保留 **3 个陆地层级**（第 1、2、3 层），对应游戏高度 **210、280、350**；水域单独计算。没有的高度层省略，碎层并入相邻层；不足两层时按当地真实高程补分并记录切点；高原提高高山门槛，宽阔高原面保留为可玩台地。三条规则及算法见[公共预览规范](../../docs/terrain_preview.md)。本图本轮无需局部调整海拔切点。

[真实地形](source.png) · [游戏分层](preview.png) · [边界版](preview--line.png)

本轮参数、原始输入与当前结果均在 `data/terrain-preview/`：

- [参数](data/terrain-preview/parameters.json)：公共入口优先读取。
- [输入](data/terrain-preview/input.npz)：真实高程、水域、主体边界与 WGS84 经纬度取景。
- [当前结果](data/terrain-preview/terrain.npz)：真实高程、分层、游戏高度及水域/省界掩码。
- [分层报告](data/terrain-preview/report.json)：最终切点、实际层数与各层面积。
- [核对记录](data/terrain-preview/verification.json)：三图取景、水域、完整边界和层高一致性。

这是地形设计预览。现有可玩地图的配方、布局、版本、定稿地形数组和制作源均保持原样；旧蓝图及历史报告按版本目录保留供回查，不代表本轮预览结果。本轮已去除河流，仅保留海洋及完整轮廓估算面积不少于 80 平方公里的湖泊、水库；第五层禁行设计仍需后续制作地图落实。

T0.5.6 已应用通用山地斜坡：16 处更新材质、6 处重做坡形；坡面延续台地草地，取消孤立短土斑。受保护基地、矿位和科技建筑保持，赠送建筑朝向与主备占地通过检查，712 条资源路线无新增/恶化空间问题。已打包、安装并采用为后续制作源，旧版保留；WorldBuilder 实际渲染已导出，游戏内待复测。见 [斜坡记录](../../docs/mountain_ramps.md#其他三图通用化发布)。

T0.5.5 已将全部赠送建筑（含电厂、超武）朝向限制为 45、135、-45、-135 度，按实际主备落点选择最接近敌方出生区中心的角度。成品仅修改生成动作的 ANGLE 字段；回读、水陆、避让及坐标流水验收通过，新增/恶化空间检查项为 0。已生成 ZIP、更新后续制作源并导出 WorldBuilder 文件；本轮未安装，实机待测。见 [朝向记录](../../docs/production_placement_fix.md)。

T0.5.4 已按公共坐标流水递增重建，补齐原生脚本航点并导出对象、层界、水陆边界、材质与资源路线。继承检查项逐项留档，新增/恶化项为 0；原生地图、地形、脚本及赠送建筑规则保持。新成品已安装，文件一致性核对通过，并已设为后续制作基准。见 [通用坐标流水](../../docs/coordinate_pipeline.md)。

T0.5.3 已更新并安装公共生产建筑朝向规则：赠送兵营、重工、机场和船厂从实际主备落点朝向对方队伍出生区域中心。仅修改脚本角度，保留水陆、避让和其他玩法，实机待复测。见 [公共规则](../../docs/production_placement_fix.md)。

T0.5.2 已加入公共赠送建筑避让：主点被占则使用备用点，全占则保留资格重试，成功后不重复。地形与归属保留，详细范围见 [修复记录](../../docs/production_placement_fix.md)。

T0.5.1 按公共 `production_placement` 规则修复 P4、P6、P8 的赠送重工及跨岸船厂；重工完整占地仅允许平坦陆地，船厂完整占地仅允许水域。地形、纹理和脚本保持原样。水陆专项通过，完整发布与既有坡道验收问题见 [修复记录](../../docs/production_placement_fix.md)。

千道溪流穿岭汇聚，浪涌开阔河口，潮声直抵天边。

地势来自公开高程，按配方里的层高切开，再收成连片的大层。北为山地，中南部是河口和沿海丘陵，西南是半岛，南边是海。北朝上，海在下方。

八个出生点按地形预览上的红圈放置。P1–P4 自北向南位于西侧，P4 在南端尖外的水上；东侧 P5、P7 在陆地，P6、P8 在海上。P7 是固定苏联 AI，P8 是固定帝国 AI。每个出生点旁边有两处近矿和油井。外围的矿铺在陆地和海上，大约每人 5 到 6 处。机场、车库和一部分油井也可以落在水上。层与层、水与陆靠自动斜坡连通，其余交界不可通行。地表按层各贴一张。外海是深水底，贴岸一条浅滩。

T0.4.4 使用 `common-T0.5.1` 通用模板，共 224 条脚本。保留第二座产能、固定 AI 和新增超武、电厂规则；移除电脑现金显示、电脑主动进攻、位置分队与四人落座。固定 AI 外交按模板初始化。新增建筑落在可建造陆地；远海 P6、P8 对应的建筑位于东侧陆地，归属不变。航点坐标及距基地距离见构建产物 `script_waypoints.json`。

水域筛选：海洋取真实海洋轮廓，河流不参与水域掩码。湖泊、水库按完整参考轮廓估算面积筛选，省界及游戏外框不作为水体面积边界。三张图共用筛选结果，真实高程未改动，原始水域输入保留在 `data/terrain-preview/raw/reference-before-water-filter.npz`。详见[水域取舍](data/terrain-preview/water-policy.json)和[保留水体格数](data/terrain-preview/water-report.json)。

## 行政中心标点预览

[查看中心标点与边界图](preview--center--line.png)：地级行政中心，图中标注 21 处，含名称与必要的引线。点位及来源见 `data/terrain-preview/centers.json`，核对记录见 `centers-report.json`。

## 蓝图与版本目录

本图显式选择 `LAYERS: "histories/v11/layers.npz"`，历史重建布局在 `histories/v11/layout.json`；根 `layers.npz` 是所选定稿蓝图的原字节镜像，不从当前 `.map` 回读，也不随成品局部编辑自动改变。日常冻结生产继续读取 `authoring/source.zip`、`authoring/layout.json` 和配置指定基准。

历史成品版本的编号对照、编辑方案和配套材料统一放在本图 `histories/T<版本>/`，当版编号对照名为 `coordinate-id-map.json`。当前预览仍在 `data/terrain-preview/`，不替代构建蓝图；完整约定见 [蓝图与版本资料](../../docs/blueprint_versions.md)。

旧 `layers_v1/v2/v3.npz` 与对应历史预览分别保留在 `histories/v1/`、`histories/v2/`、`histories/v3/`。原先无法确认版本的无版本 `layers.npz` 保存在 `histories/legacy/layers.npz`，没有被 v11 或根镜像覆盖。
