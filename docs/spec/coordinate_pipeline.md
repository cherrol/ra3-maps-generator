# 通用坐标制作流水

当前保留河南当前版，通过 `COORDINATE_PIPELINE` 读取完整原生来源并继续制作。公共空间策略为 `rules/coordinate-pipeline.json`，允许的本图 `policy` 仅填写差异字段。全部可玩图使用当前两矿家区和严格零问题验收，来源完整并不豁免任何门禁。

技术阶段由 `src/pipeline/build/mod.rs::STAGES` 定义，当前为 `terrain → passability → map → verify → spatial → preview → coordinates → accept → readme → dist → wb`。这是产物依赖顺序；布局设计按[专业交接](map_skills_checklist.md#共用执行顺序)先确定通路和斜坡、再布矿。`coordinates` 是独立公共阶段；未启用冻结成品模式的新蓝图构建也会自动输出坐标资料。现有成品构建模式从完整来源保留原生高度、纹理、通行/建造位图、对象和脚本，避免从旧布局重新制作时覆盖定稿内容；添加斜坡通过下述受控编辑更新原生候选，不靠重新套用旧蓝图生成。

两条路径共用阶段名称。已有完整来源的 `terrain` 从原生包读取地形，`passability` 与 `spatial` 执行当前严格审计；未启用此模式的新图转换预览设计及本轮显式斜坡布局，按布局生成材质、对象与脚本。两条路径均须保持层边界、水陆边界，新增斜坡按下述受控编辑及独立验收衔接，不借默认处理重新设计边界。新图的 `spatial` 阶段尚不执行独立空间审计，验收由该路径的 `verify`、`accept` 完成。`--only`、`--skip` 只筛选阶段，不自动补齐前置产物。

斜坡、层界、水陆边界及交汇处按[地形交接规范](terrain_transitions.md)贯穿上述阶段：设计落实为实际输入，高度、导航、材质分别留证，验收按[交接章节](map_acceptance.md#地形交接如何验)补足公共自动覆盖之外的判断。可玩制作允许添加斜坡，但不允许修改层边界、水陆边界；新增坡道的局部高度与通行差异单独记录。工具默认地形处理、纹理整理或重包均不能证明边界保护、全周岸坡和交汇处合格，不把设计顺序误写成程序已自动执行的联合修复。

```sh
# 完整制作：含验收、坐标地图、发布包和 WorldBuilder 导出
cargo run --release --locked -- build --map <id>

# 只计算占地、坡道、资源通路和纹理资料
cargo run --release --locked -- spatial --map <id>

# 对坐标修改生成新版本；任何当前检查问题都会拒绝发布并回滚版本
cargo run --release --locked -- build --map <id> --bump --note '说明坐标调整'
```

## 制作输入

| 文件 | 用途 |
|---|---|
| `maps/<id>/authoring/source.zip` | 冻结的完整原生成品和同版布局，不依赖 assets 目录中的历史缓存 |
| `authoring/layout.json` | 可编辑对象坐标与朝向；对象编号、类型、归属、属性保持身份一致 |
| `rules/coordinate-pipeline.json` | 通用占地、平整度、通行及 `ramp_clearance` 坡口资源避让策略，单位为格或世界高度 |
| `authoring/policy.json`（可选） | 对公共策略做局部覆盖 |
| `authoring/baseline.json` | 来源对照记录，包含对象身份、坐标、坡道参数和测量值；不授权继承问题 |

以上 `authoring/` 路径均位于对应的 `maps/<id>/` 下。根 `layers.npz` 是当前 `LAYERS` 指定蓝图的原字节镜像，不从成品 `.map` 回读。当前地图版本读取 `version.json`，已采用来源读取 `COORDINATE_PIPELINE.source_name/source_crc32`；已删除地图及历史输入不再作为构建来源。

新图的蓝图输入为 `LAYERS`（默认 `layers.npz`），布局输入为 `LAYOUT`（默认 `layout.json`）；完整原生来源使用 `COORDINATE_PIPELINE.layout`，当前河南为 `authoring/layout.json`。两条路径均先建立本图坐标系，再处理资源与对象，并执行相同现行规则。

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
6. **验收汇总**：自动检查执行当前严格门禁，再按[公共交付验收](map_acceptance.md)完成适用独立判断；发布入口核对结构化独立记录的候选身份、证据及状态。来源问题和对象未移动均不豁免，程序不自动给设计判断填通过。
7. **输出与发布**：有修改时要求递增版本，同步对象 XML 和小地图；无修改时复制精确原始字节。自动更新坐标地图与检查资料，发布前再次验收。

检查缓存由原生内容、归一化布局、策略、基准和算法版本共同决定；同时校验导出文件内容。输入未变且资料完整时复用结果，修改输入或损坏导出后会重新计算。纹理 JSON 使用紧凑格式，完整保留逐格数据。

空间及发布报告保存 `spawn_placement`，覆盖范围见[出生点规则](spawn_placement.md)；策略、水位、布局或地图字节变化须重新匹配当前证明。已拒绝的历史参考字段与配方值统一见[Rust 配方](rust_migration.md#当前严格验收与格式保护)。

## 公共分级布矿

通过 `RESOURCE_DISTRIBUTION` 提供本图实际路线、座位配对与全部原生矿编号；分级和配额含义见[资源规则](resource_distribution.md)，公共默认参数保存在 `rules/coordinate-pipeline.json#/resource_distribution`。所有非 `TERRAIN_ONLY` 地图必须提供当前计划；`validate_transition` 使用同一严格检查，计划及公共参数参与空间缓存与发布证明。

斜坡与矿位冲突按[斜坡优先级与冲突处理](ramp_clearance.md#斜坡优先级与冲突处理)安排移矿及联动复验，保护全部既定及本轮新增合理坡道和完整引道。可玩制作可在保持边界的前提下添加斜坡；仍无布局解时记录冲突，确需改层边界或水陆边界则返回预览阶段。自动零冲突不能代替[斜坡优先级验收](map_acceptance.md#斜坡优先级如何验)。

## 通用坡口资源避让

坡口门禁读取 `rules/coordinate-pipeline.json#/ramp_clearance` 与允许的本图 `RAMP_CLEARANCE` 覆盖，几何口径和报告判定统一见[坡口避让](ramp_clearance.md)。此处不重复维护默认距离与通过字段。

该门禁接入 `map`、`verify`、`dist`、安装、采用来源及增量编辑发布入口，单独选择阶段也执行。重新封装同样需要同版布局及严格验收；资源几何不变不构成豁免。设计优先级仍按独立验收核对，自动零冲突不能证明从未为保矿削坡或移坡。

## 成品编辑后的衔接

候选生成、独立审核与提交发布分开：增量编辑完成自动检查后，须有[匹配本候选的独立验收记录](map_acceptance.md#结构化独立验收门禁)才能打包提交；缺失时将候选与待核验清单保存到本图 `data/delivery-candidates/`，保留原版本、成品和冻结源。审核后重试同一操作，不能仅重命名或复用旧版记录。

赠送修复与已确认的局部坡口材质接缝可按[同一显式计划](production_placement_fix.md#已有成品的受阻修复与重选)组合，保持同一原始来源、一个最终候选和一次版本提交。组合后的原生内容须重新计算身份并复验；前一步几何结论只有在相关输入逐项相同的证据成立时才能复用，材质变化仍须独立复核。

可玩制作按[预览边界保护](preview_terrain_lock.md)保持层边界、水陆边界，允许添加斜坡。新坡使用显式方案限定坡体、端口与引道，核对设计分层、水陆掩码及原生差异；现有工具支持新增或移坡不代表任意模式均符合本轮规则，不以重包或工具成功替代独立边界验收。既有合理坡道及新增范围外高度和玩法位图继续保护。

坡面材质用 `polish-ramp-surfaces`，全图材质用 `polish-textures`；赠送局部受阻用[显式计划修复](production_placement_fix.md#已有成品的受阻修复与重选)，整体生产选址用 `repair-production`，封装修复选择相应 `repair-*`。这些材质、布局和封装入口仍不夹带高度变化。上述入口不自动执行完整构建、坐标导出、WorldBuilder 导出或采用制作源；发布保存同版 `source/layout.json`，不得以旧布局覆盖候选。

先核对新成品与上一版的实际差异、当前严格空间结果及新版本 `assets/.../source/layout.json`；布局缺失表示交付资料不完整，不能用旧蓝图重新生成来代替。确认本轮变化后，显式采用当前完整成品，再无版本递增地补齐制作资料：

```sh
# 通过当前严格检查后更新完整原生包、可编辑布局及来源对照
cargo run --release --locked -- adopt-coordinate-source --map <id>

# 补齐当前版预览、空间报告、坐标、发布包和 WorldBuilder 导出
cargo run --release --locked -- build --map <id>
```

采用来源只保存已经通过当前严格验收的完整候选，不能用更新基准掩盖问题。普通构建不会放宽规则；未采用最新成品时，来源预检查会阻止构建。通过坐标修改生成新版本后，继续制作前同样须核对并采用该版本。安装按发布约定添加 `--install`；WB 默认导出资料，复制到地编还需 `--wb`，目标通过 `--wb-root <目录>` 或 `RA3MAP_WB_ROOT` 指定。生产流程不包含实机检查步骤。

## 同版输出

每次构建的资料位于本版 `assets/.../spatial/`：`README.md`、`report.json`、`issues.csv`、`routes.csv/json`、`texture-coordinates.csv/json`，并生成 `spatial.zip`。交互坐标地图位于同版 `coordinates/index.html`，完整坐标包为 `coordinates.zip`；对象编号以当前完整原生对象及 `authoring/coordinate-id-map.json` 为准。

发布还须通过[官方最终场景与 XML 严格检查](official_resource_pipeline.md)和[公共交付验收](map_acceptance.md)。诊断结果、完整来源、原样复制或版本对照均不能替代当前合格证据。
