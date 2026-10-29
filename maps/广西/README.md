# 九州大地·碧嶂回澜

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

当前保留 **4 个陆地层级**（第 1、2、3、4 层），对应游戏高度 **210、280、350、420**；水域单独计算。没有的高度层省略，碎层并入相邻层；不足两层时按当地真实高程补分并记录切点；高原提高高山门槛，宽阔高原面保留为可玩台地。三条规则及算法见[公共预览规范](../../docs/terrain_preview.md)。本图本轮无需局部调整海拔切点。

[真实地形](source.png) · [游戏分层](preview.png) · [边界版](preview--line.png)

本轮参数、原始输入与当前结果均在 `data/terrain-preview/`：

- [参数](data/terrain-preview/parameters.json)：公共入口优先读取。
- [输入](data/terrain-preview/input.npz)：真实高程、水域、主体边界与 WGS84 经纬度取景。
- [当前结果](data/terrain-preview/terrain.npz)：真实高程、分层、游戏高度及水域/省界掩码。
- [分层报告](data/terrain-preview/report.json)：最终切点、实际层数与各层面积。
- [核对记录](data/terrain-preview/verification.json)：三图取景、水域、完整边界和层高一致性。

这是地形设计预览。现有可玩地图的配方、布局、版本、定稿地形数组和制作源均保持原样；旧蓝图及历史报告按版本目录保留供回查，不代表本轮预览结果。本轮已去除河流，仅保留海洋及完整轮廓估算面积不少于 80 平方公里的湖泊、水库；第五层禁行设计仍需后续制作地图落实。

T2.3.7 已去掉坡面孤立短土斑，恢复与两端草地的连续衔接。2983 个材质格更新；高度、坡形及全部玩法位图保持。已验收并安装，旧版保留。

T2.3.6 已安装到本机新版本目录，旧版保留。已按 AXAI、NKL、SMAI 山地图改进坡口：24 处更新局部材质，11 处重做稳定坡面和收拢坡肩，其余保留高度。赠送建筑主、备用完整占地及矿盘保持；648 条资源路线检查无新增/恶化问题，来源问题由 67 项降为 45 项。见 [山地坡口参考与对比](../../docs/mountain_ramps.md)。

T2.3.5 已将全部赠送建筑（含电厂、超武）朝向限制为 45、135、-45、-135 度，按实际主备落点选择最接近敌方出生区中心的角度。成品仅修改生成动作的 ANGLE 字段；回读、水陆、避让及坐标流水验收通过，新增/恶化空间检查项为 0。已生成 ZIP、更新后续制作源并导出 WorldBuilder 文件；本轮未安装，实机待测。见 [朝向记录](../../docs/production_placement_fix.md)。

T2.3.4 已按公共坐标流水递增重建，补齐原生脚本航点并导出对象、层界、水陆边界、材质与资源路线。继承检查项逐项留档，新增/恶化项为 0；原生地图、地形、脚本及赠送建筑规则保持。新成品已安装，文件一致性核对通过，并已设为后续制作基准。见 [通用坐标流水](../../docs/coordinate_pipeline.md)。

T2.3.3 已更新并安装公共生产建筑朝向规则：赠送兵营、重工、机场和船厂从实际主备落点朝向对方队伍出生区域中心。仅修改脚本角度，保留水陆、避让和其他玩法，实机待复测。见 [公共规则](../../docs/production_placement_fix.md)。

T2.3.2 已加入公共赠送建筑避让：主点被占则使用备用点，全占则保留资格重试，成功后不重复。地形与归属保留，详细范围见 [修复记录](../../docs/production_placement_fix.md)。

碧色峰丛环绕曲水，清流转过石壁，回澜映出天光。

## 当前脚本

T2.3.1 修复赠送重工落水：P6 重工航点移至附近平坦陆地。重工与船厂均检查完整占地，分别只允许陆地与水域；原有地形、纹理、脚本和资源身份保持原样。

T2.2.2 使用 `common-T0.5.1` 通用模板，共 224 条脚本。保留第二座产能、固定 AI 和新增超武、电厂规则；移除电脑现金显示、电脑主动进攻、位置分队与四人落座。固定 AI 外交按模板初始化。31 个新增建筑航点根据本图地形选址并检查占地与间距。

以下为早期设计记录；现行出生点以 `COORDINATE_PIPELINE` 引用的 `authoring/layout.json` 和完整原生成品为准，历史重建布局保存在 `histories/v11/layout.json`，玩法以 `templates/rules/common-T0.5.1.json` 为准。旧 Python 补丁和组装时改写外交的方式已停用。

## 历史蓝图与地形设计

* **轮廓**：直接栅格化主区外轮廓，西端的凸起、北部凸起、南部海岸线和近岸小岛都保留。
* **地势**：西北高、东南低。西北和东北是山地，中部和南部是盆地平原。
* **水系**：按走向开凿河谷并注入水面，形成天然分界与渡口。干流自西北向东南横贯，北支、南支和几条干谷接到干流或入海。
* **海面**：南岸海域水位 200（世界单位），海床最深约 112，沿岸为浅滩与珊瑚、沙底纹理；沿海出生位可造海军。
* **地图边界**：边界之外为高耸的不可通行山体（北部、西部、东部）与海面（南部），形成天然封闭战场。
* **可通行性**：按坡度与海拔自动生成不可通行标记（陡崖、山峰、边界山体），8 个出生点陆路互通（洪水填充验证）。

## 队伍与战术要点

* **西队 4 人**（P1–P4，自北向南）沿西侧高地排开。
* **东队 2 人 + 2 个固定 AI**（P5–P6 房间可选，P7–P8 脚本固定生成；P8 为水上基地），自北向南排在东侧。组装时把模板里 Player_5 / Player_6 的额外 AI 关系从敌对改成友好，西队 Player_1–4 仍与这两个 AI 敌对。
* **中路** 为无归属争夺区，放有争夺矿和哨站、车库、学院、医院、机场。
* 南部海域为海战区域；河流上的浅滩渡口是装甲推进的关键节点。

## 技术实现

本图不使用 WorldBuilder 手工绘制，而是按 SAGE/RA3 `.map` 二进制格式直接生成：

1. `EAR\0` + RefPack 压缩容器（解压后为 `CkMp` 明文：字符串表 + 顺序 chunk）。
2. 骨架取自 8 人模板（`templates/8p_template.map`），**保留其脚本系统**（房间 6 个可选位 + 2 个固定 AI 位的 "Bonus Players System"）、阵营、队伍、光照等 chunk。
3. 替换生成：`HeightMapData`（真实高程）、`BlendTileData`（地表纹理与不可通行位）、`ObjectsList`（出生点/矿脉/油井等对象，属性逐字节保持）、`StandingWaterAreas`（海/河水面）、`WorldInfo`（纹理表与地图名）。
4. `_art.tga` 为 512×512 24 位未压缩 TGA（自下而上），用作雷达/小地图美术。
5. 生成后重新解析校验：所有 chunk 精确解析到末尾、对象序列化与模板逐字节一致、出生点全部连通。

## 待办 / 已知事项

* 两个固定 AI 的**阵营关系**：`map_gen.patch_east_bonus_alliance` 对调 DPR/FAPR Player 5 与 Player 6 的真假分支，使额外 AI 与东队人类友好，并保持与西队敌对。
* 建议先在单机模式加载一次确认地形、出生点、水面与矿石位置，再进战网。

水域筛选：海洋取真实海洋轮廓，河流不参与水域掩码。湖泊、水库按完整参考轮廓估算面积筛选，省界及游戏外框不作为水体面积边界。三张图共用筛选结果，真实高程未改动，原始水域输入保留在 `data/terrain-preview/raw/reference-before-water-filter.npz`。详见[水域取舍](data/terrain-preview/water-policy.json)和[保留水体格数](data/terrain-preview/water-report.json)。

## 行政中心标点预览

[查看中心标点与边界图](preview--center--line.png)：地级行政中心，图中标注 14 处，含名称与必要的引线。点位及来源见 `data/terrain-preview/centers.json`，核对记录见 `centers-report.json`。

## 蓝图与版本目录

本图显式选择 `LAYERS: "histories/v11/layers.npz"`，历史重建布局在 `histories/v11/layout.json`；根 `layers.npz` 是所选定稿蓝图的原字节镜像，不从当前 `.map` 回读，也不随成品局部编辑自动改变。日常冻结生产继续读取 `authoring/source.zip`、`authoring/layout.json` 和配置指定基准。

历史成品版本的编号对照、编辑方案和配套材料统一放在本图 `histories/T<版本>/`，当版编号对照名为 `coordinate-id-map.json`。当前预览仍在 `data/terrain-preview/`，不替代构建蓝图；完整约定见 [蓝图与版本资料](../../docs/blueprint_versions.md)。
