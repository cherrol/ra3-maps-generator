# 通用坐标制作流水

四张现有地图均通过 `COORDINATE_PIPELINE` 启用原生坐标流水，以各自的最新完整成品继续制作。公共空间策略为 `templates/rules/coordinate-pipeline.json`，每图 `policy` 文件只需填写差异字段，其余继承公共值。千壑横原的局部策略为空对象。

通用流水为 `terrain → passability → map → verify → spatial → preview → coordinates → accept → readme → dist → wb`。`coordinates` 是独立公共阶段；未启用冻结成品模式的新蓝图构建也会自动输出坐标资料。现有成品模式保留原生高度、纹理、通行/建造位图、对象和脚本，避免从旧布局重新制作时覆盖定稿内容。

两条路径共用阶段名称。现有四图的 `terrain` 从冻结原生包读取地形，`passability` 与 `spatial` 执行空间增量审计；未启用此模式的新图从配方指定的分层蓝图和布局生成地形、材质、对象与脚本，`spatial` 阶段尚不执行独立空间审计，验收由该路径的 `verify`、`accept` 完成。`--only`、`--skip` 只筛选阶段，不自动补齐前置产物。

```sh
# 完整制作：含验收、坐标地图、发布包和 WorldBuilder 导出
cargo run --release --locked -- build --map <id>

# 只计算占地、坡道、资源通路和纹理资料
cargo run --release --locked -- spatial --map <id>

# 对坐标修改生成新版本；遇到新增/恶化问题会拒绝发布并回滚版本
cargo run --release --locked -- build --map <id> --bump --note '说明坐标调整'
```

## 制作输入

| 文件 | 用途 |
|---|---|
| `maps/<id>/authoring/source.zip` | 冻结的完整原生成品和同版布局，不依赖 assets 目录中的历史缓存 |
| `authoring/layout.json` | 可编辑对象坐标与朝向；对象编号、类型、归属、属性保持身份一致 |
| `templates/rules/coordinate-pipeline.json` | 通用占地、平整度、通行及 `ramp_clearance` 坡口资源避让策略，单位为格或世界高度 |
| `authoring/policy.json`（可选） | 对公共策略做局部覆盖 |
| `authoring/baseline.json` | 来源成品既有检查项，包含对象身份、坐标、坡道参数和测量值 |

以上 `authoring/` 路径均位于对应的 `maps/<id>/` 下。现有四图的 `histories/v11/layout.json`、`histories/v11/layers.npz` 保留为历史重建输入，不参与冻结成品地形重建；根 `layers.npz` 只是当前选定蓝图的原字节镜像，不从成品 `.map` 回读。当前地图版本读取 `version.json`，已采用的来源版本读取 `config.json` 中的 `COORDINATE_PIPELINE.source_name`；下文历史状态表不替代这两个文件。

新图的蓝图输入为 `LAYERS`（默认 `layers.npz`），布局输入为 `LAYOUT`（默认 `layout.json`）；四图历史蓝图使用 `histories/v11/layers.npz`、`histories/v11/layout.json`，冻结成品使用 `COORDINATE_PIPELINE.layout`（现有四图为 `authoring/layout.json`）；两条路径均先建立本图坐标系，再处理资源与对象。

构建先预读选定蓝图，再写版本和产物；阶段、验证及根镜像同步均属于提交前事务，失败回滚版号与镜像并清理本轮新建产物。成功提交本地版本、成品和镜像后才执行可选安装。安装失败报告“构建已完成，安装失败”，保留本地成果；增量发布同样先同步镜像后安装，安装失败报告“发布已完成，安装失败”。

## 每图坐标与资源计算

坐标统一由 `coordinates::Frame` 正逆换算，字段为 `width`、`height`、`border`、`cell_size`、`z_scale`。全图格原点在西北采样点，X 向东、Y 向南；游戏世界坐标 X 向东、Y 向北。新图从本图 `GRID/BORDER/CELL/Z_SCALE` 建立，读取原生成品时以实际高度场宽高、边框为准，并使用本图 `CELL/Z_SCALE`。完整公式见 [地图坐标资料](map_coordinates.md)。

资源先在本图格坐标内计算位置，检查地形、层级、坡口与完整占地后，用同一 Frame 写入原生对象；读取原生对象时使用其逆变换。不能按预览 PNG 像素、其他地图的尺寸或硬编码边框/格长换算。斜坡、出生点、赠送主备航点和资源共用这份坐标关系。

| 坐标声明 | 作用 |
|---|---|
| `maps/<id>/coordinate-system.json` | 当前地图的稳定声明，包含地图身份、坐标帧、范围及正逆公式，不含发布版本；导出旧版本不会覆盖当前声明 |
| `assets/<真实地形名>/<生成发布名+版本>/source/coordinate-system.json` | 构建准备阶段在资源/对象处理前写入，含本版身份 |
| 同版 `coordinates/coordinate-system.json` | 坐标导出时写入，与同版 source 声明一致，纳入 `coordinates.zip` |
| `maps/<id>/data/terrain-preview/coordinate-system.json` | `scope: terrain-preview-design` 的预览设计帧，宽高/边框读取本次缓存与报告；格长 10、高度比例 0.0390625 为明确的设计默认值 |

纯预览没有 `config.json` 时同时保存地图根坐标声明；已有可玩配方时，只写预览资料中的设计帧，不覆盖实际地图声明。`terrain_preview::initialize_coordinate_system` 可从现有报告和缓存补齐上述元数据，不重画 PNG，不改高度、资源布局或游戏地图。推进可玩制作时按实际配方建立可玩 Frame，不直接套用设计默认值。

坐标编辑以 `objects[编号−1].cell` 为准。出生点、矿、医院等分类清单按来源对象身份自动派生；修改对象后无需再手动同步矿位或医院清单。矿的朝向变化也同步转换至内部朝向字段。对象数量、类型、属性等结构变化需要采用完整的新来源。

## 流水步骤

1. **输入检查**：校验完整成品、来源校验码及对象身份；当前游戏文件与冻结来源不一致时停止，避免覆盖更新的成品。
2. **地形读取**：先准备本图 Frame；直接读取原生高度、材质、混合和通行/建造位图，保存同版制作缓存。
3. **空间检查**：分桶查询邻近对象，检查矿盘及按矿口朝向确定的完整矿场预留、建筑预留范围、树石间距，以及坡面全宽、声明端口和沿轴双向通行延伸区。
4. **坡道检查**：按声明方向、跨度、宽度和落地段做全宽采样，记录纵向高差、端口误差和原生阻挡格。
5. **资源检查**：检查八个出生点的近矿配额；分别在原生陆地和水域上加入对象障碍、通行余量和单步高差限制，计算到矿、油井和科技建筑的四邻接最短路。跨水陆标记为 `requires_transport`，`reachable` 和距离为 null，避免将运输需求记成陆路阻断。
6. **增量验收**：旧问题按身份、位置及测量值逐项比较；新位置产生的冲突、重叠加深、近矿配额变化或可达性丢失均拒绝通过。
7. **输出与发布**：有修改时要求递增版本，同步对象 XML 和小地图；无修改时复制精确原始字节。自动更新坐标地图与检查资料，发布前再次验收。

检查缓存由原生内容、归一化布局、策略、基准和算法版本共同决定；同时校验导出文件内容。输入未变且资料完整时复用结果，修改输入或损坏导出后会重新计算。纹理 JSON 使用紧凑格式，完整保留逐格数据。

出生点额外执行[公共方位与两矿同面验收](spawn_placement.md)：从原生航点逐座判断对置侧别，检查基地、两矿矿盘及矿口规划区的完整占地、同层/水面连通与位图条件。默认冻结源逐项继承已有问题并拦截新增/恶化，显式 `SPAWN_PLACEMENT` 要求严格零问题；空间及发布报告保存 `spawn_placement`，策略、水位、来源或历史参考变化不能复用旧通过缓存。现有四张成品本轮暂不调整。

## 公共分级布矿

新图及重布矿位/出生点任务采用[公共矿位分布规则](resource_distribution.md)：每位 6 个阶段、总计 48 矿，家区 100 格内 3 矿，第 4–6 矿分别进入主线前段、热点与次线。通过 `RESOURCE_DISTRIBUTION` 提供本图实际路线、座位配对与全部原生矿编号；公共默认参数保存在 `templates/rules/coordinate-pipeline.json#/resource_distribution`。

已有未配置计划的冻结源只在矿位和出生点保持原样时沿用，不代表已完成分级验收。位置变化会要求完整计划；公共参数和计划内容同时绑定空间检查缓存及发布证明。本轮公共规则调整不自动搬矿或递增地图版本。

## 通用坡口资源避让

`templates/rules/coordinate-pipeline.json#/ramp_clearance` 缺省作用于所有地图，当前默认沿坡轴两端各延伸 25 格、占地净距 8 格；本图 `RAMP_CLEARANCE` 对参数做局部覆盖并明确执行严格零冲突检查。参数来自生效策略，不按某张图的坡宽或编号固定。矿、油井、医院等中立建筑均检查完整预留占地，不能只看对象中心到坡心的距离。

新图严格要求零冲突；没有显式 `RAMP_CLEARANCE` 的冻结成品与 CRC 绑定的 `authoring/source.zip` 和其中同版布局比较。旧问题只读留档，移动对象、改变矿口朝向或坡形后产生的新冲突及距离恶化均拒绝通过，不自动移动既有资源。几何审计报告保留 `raw_passed`、全部 `findings`、继承数 `inherited`、`regressions` 和 `status`，继承通过不代表零冲突。

该门禁接入 `map`、`verify`、`dist`、安装、采用来源及增量编辑发布入口，单独选择阶段也执行。纯封装路径可验证资源和原生高度几何不变，并保存与本版 native CRC、生效策略绑定的证明；不能用缺失布局绕过检查。历史回归专用 `RAMP_CLEARANCE_REFERENCE` 固定几何、策略和原始报告，校验不会自动创建或重置它，相关几何变化即拒绝。模型、报告及历史记录见 [通用坡口避让](ramp_clearance.md)。

## 成品编辑后的衔接

斜坡的几何变化继续使用 `relocate-ramps`，材质美化继续使用 `polish-textures` 等原生增量编辑器。禁止只改坡心参数却沿用旧高度场。`repack`、`polish-*`、`repair-*` 等成品发布入口不自动执行完整构建、坐标导出、WorldBuilder 导出或采用制作源；普通 `repack` 不生成同版 `source/layout.json`。

先核对新成品与上一版的实际差异、空间增量结果及新版本 `assets/.../source/layout.json`；布局缺失时先补齐并核对，不能用旧蓝图重新生成来代替。确认本轮变化后，显式采用当前完整成品，再无版本递增地补齐制作资料：

```sh
# 更新冻结原生包、可编辑布局和空间问题基准
cargo run --release --locked -- adopt-coordinate-source --map <id>

# 补齐当前版预览、空间报告、坐标、发布包和 WorldBuilder 导出
cargo run --release --locked -- build --map <id>
```

采用来源会把当前问题写为新基准，不能用它代替本轮相对旧基准的新增/恶化检查。普通构建不会自动放宽基准；未采用最新成品时，来源预检查会阻止构建。通过坐标修改生成新版本后，继续制作前同样须核对并采用该版本。安装按发布约定添加 `--install`；WB 默认导出资料，复制到地编还需 `--wb`，目标通过 `--wb-root <目录>` 或 `RA3MAP_WB_ROOT` 指定。生产流程不包含实机检查步骤。

## 接入时的历史版本与结果

以下是四图接入通用坐标流水时的历史快照，并非当前版本清单。当时三张新版本完成完整构建、空间增量检查、坐标导出、打包和 WorldBuilder 导出；千壑横原保留 T0.6.5。下表与末段检查结论仅描述那次接入记录，各图当时新增/恶化检查项均为 0。

| 地图 | 版本 | 原生对象 | 斜坡 | 资源点 | 静态可达关系 | 需运输关系 | 同类地形静态未达 | 来源检查项 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| 千壑横原 | T0.6.5 | 544 | 27 | 136 | 528 | 0 | 0 | 203 |
| 千流叩海 | T0.5.4 | 568 | 16 | 77 | 372 | 315 | 25 | 89 |
| 碧嶂回澜 | T2.3.4 | 582 | 24 | 69 | 529 | 117 | 2 | 67 |
| 翠屿浮光 | T0.3.4 | 525 | 21 | 68 | 320 | 320 | 0 | 52 |

来源检查项不是不同对象数，也不表示均已修复。完整保留继承项的身份、坐标和测量值；本次没有通过移动资源、改变坡道或降低阈值来掩盖问题。占地采用可配置的保守圆形模型，路线是静态估算，不等同于实机寻路或 AI 行为；运输需求不推断行程。

三张图的旧布局分别漏记 31 个已生成的公共脚本航点。采用来源时按原生对象顺序补齐；`histories/T<版本>/coordinate-id-map.json`（如 `histories/T0.7.8/coordinate-id-map.json`）保存旧编号到原生编号的转换，统一归档到本图 `histories/` 下的版本目录。坐标包中的 `coordinate-id-map.json` 包含当前编号和历史版本对照，千壑横原的原编号保持一致。

每次构建的资料位于对应版本的 `assets/.../spatial/`：`README.md`、`report.json`、`issues.csv`、`routes.csv/json`、`texture-coordinates.csv/json`，另有 `spatial.zip`。交互坐标地图位于同版 `coordinates/index.html`，完整坐标包为 `coordinates.zip`。

该次接入的复核与发布证据在 `work/common-coordinates/`。历史记录为：74 项测试、格式、Clippy 与差异检查通过；四图高度、通行/建造标记、完整边界格边、对象和四个材质数组逐项匹配原生数据。三个新包各有五个游戏文件，在 assets、dist、ZIP 和新安装目录中逐字节一致；地图二进制及小地图与上一版相同，旧安装目录保留。当时的新版本均再次采用为制作基准。该次回归覆盖遗漏航点导入、公共策略局部覆盖和跨水陆路线标记；千壑横原先行接入的原始记录保留在 `work/coordinate-pipeline/`。后续版本的结果以各自报告和发布记录为准。
