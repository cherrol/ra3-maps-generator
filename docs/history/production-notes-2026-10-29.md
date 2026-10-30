> 历史快照：2026-10-29 文档整理前的 `README.md`。保留当时事实、命令和待办供追溯；执行入口与当前边界见 [项目首页](../../README.md)、[当前状态](../SESSION_STATE.md)。其中旧 Python 工具、玩法默认和清理指令均按历史语境阅读。

# RA3 地图工坊（ra3maps）

使用原生 Rust 从分层蓝图、声明式布局和脚本模板制作新地图，或沿用完整原生成品继续编辑《红色警戒 3》地图。构建、校验、预览、版本管理、ZIP 打包和 WorldBuilder 导出均在一个可执行文件中完成，运行时无需 Python、NumPy 或 Pillow。

地图合集为 **九州大地**。四张图的配方目录、CLI id、规则模板、发布目录与游戏显示名均已统一为：千壑横原、碧嶂回澜、千流叩海、翠屿浮光。默认地图为碧嶂回澜，以下命令使用新名。定名见 [系列命名](../spec/九州大地命名.md)，迁移与清理见 [命名迁移](../spec/naming_migration.md)。每图最后一个旧发布版本归档于 `archive/九州大地/改名前/`。

## 一句话制作地形预览

提供本次真实地形参考资料，说「以 xxx 为蓝图制作游戏地形图」即可。公共流程自动查 docs 命名、以主体居中确定紧包范围的矩形，将省界内外的全部游戏范围统一分层，最终只保存 `maps/<地图名>/source.png`、`preview.png`、`preview--line.png`；三图使用同一取景。

提供高程数据时，公共执行入口为 `cargo run --release --locked -- terrain-preview <蓝图原型或图名> <提供的高程NPZ>`，不需要预先建立完整地图配方。仅有参考图时按图制作地形设计，不推断成测量高程。完整步骤与输入说明见 [公共游戏地形预览](../spec/terrain_preview.md)。

现有预览可运行 `cargo run --release --locked -- terrain-preview-centers <蓝图原型或图名>`，读取各图 `data/terrain-preview/centers.json` 新增带行政中心名称、标点和边界的 `preview--center--line.png`，保留三张基础图和原有分层。

## 构建与使用

四张图均已启用 [通用坐标制作流水](../spec/coordinate_pipeline.md)：以各自的定稿成品为基准，自动派生资源清单，检查占地、坡口、近矿及陆路/水路，输出逐格材质坐标。公共策略在 `templates/rules/coordinate-pipeline.json`，每图可做局部覆盖；新增或恶化问题阻止发布。普通流水包含独立的 `coordinates` 阶段，新蓝图构建同样自动导出坐标。

现有四图的制作输入为各图 `authoring/source.zip`、`authoring/layout.json` 和空间检查基准；旧 `layout.json`、`v11_layers.npz` 保留作历史与新蓝图路径参考。未启用 `COORDINATE_PIPELINE` 的新图才从配方指定的分层蓝图和布局重新生成地形、材质与玩法。当前地图版本以 `maps/<id>/version.json` 为准，冻结来源版本以 `config.json` 的 `COORDINATE_PIPELINE.source_name` 为准。

每张地图先建立独立的 `coordinates::Frame`，资源位置在本图格坐标中计算，再统一换算为游戏世界坐标。`maps/<id>/coordinate-system.json` 保存不含版本的当前坐标声明，同版声明另写入 `assets/.../source/`、`coordinates/` 和 `coordinates.zip`。宽高、边框、格长和高度比例读取本图实际网格与配方，不能按 PNG 像素或其他地图的固定参数换算。纯地形预览另存 `data/terrain-preview/coordinate-system.json`，使用明确的设计参数；已有可玩图的坐标声明保持独立。具体产物与公式见 [地图坐标资料](../spec/map_coordinates.md)。

矿、油井、医院等中立建筑统一避开坡面全宽、声明端口及沿坡轴双向通行延伸区，并检查完整矿场或建筑预留占地。公共策略在 `templates/rules/coordinate-pipeline.json#/ramp_clearance`，当前默认两端各延伸 25 格、净距 8 格，可由本图 `RAMP_CLEARANCE` 覆盖。新图及显式配置 `RAMP_CLEARANCE` 的图执行零冲突检查；其余冻结成品将旧问题逐项留档并拦截新增或恶化，不自动移动既有资源。地图生成、回读、打包、安装及采用制作源均核对该规则，细则见 [通用坡口避让](../spec/ramp_clearance.md)。

坐标查询使用 `cargo run --release --locked -- coordinates --map 千壑横原`，导出离线交互地图、完整地形格点、层界和对象 CSV/JSON；按原生对象顺序编号，附旧预览编号对照，不修改游戏地图。见 [坐标系统与统计](../spec/map_coordinates.md)。

需要 Rust 1.88 或更新版本。首次编译需要获取 Cargo.lock 锁定的依赖。首次检出先按 [可复现构建与测试输入](../spec/reproducible_builds.md) 恢复并检查受控素材，再执行检查与构建；完整命令顺序见 [验证](#验证)。

```sh
cargo build --release --locked
cargo run --release -- list
cargo run --release -- build --map 碧嶂回澜
```

Windows 可以使用原有入口：

```powershell
.\build.cmd --map 碧嶂回澜
.\build.ps1 -Map 碧嶂回澜 -Only map,verify
.\build.ps1 -Map 碧嶂回澜 -Bump -Note '调整矿位' -Install
```

Linux/macOS 可使用 `./build.sh --map 碧嶂回澜`。编译完成后也可直接运行 `target/release/ra3maps`（Windows 为 `ra3maps.exe`）。从其他目录运行时传入 `--root <工程根目录>`。

地图选择顺序为 `--map`、`RA3MAP_ID`、`maps/.default`，最后尝试唯一配方。不指定地图即可构建默认配方。

## 项目结构

```text
src/
  binary.rs       有边界检查的基础二进制编解码
  refpack.rs      RefPack LZ77 压缩、解压与 EAR 容器
  map.rs          CkMp、字符串表、属性、对象、高度图和纹理块
  archive.rs      ZIP/NPY 输入读取和确定性 ZIP 输出
  project.rs      配方、路径、版本和原子文件写入
  terrain_preview.rs 制作前公共地形预览：本次蓝图、整图分层、三图同取景
  terrain.rs      分层蓝图、严格层高、海深、斜坡、登陆岸与出生平台
  surface.rs      可复现纹理噪声、层界/水岸装饰和通行/建造位图
  scripts.rs      脚本树解析、字符串编号链接与引用检查
  script_spawns.rs 通用脚本建筑航点的地形适配与占地验收
  resources.rs    实际对象与脚本资源清单、纹理图集和地图 XML
  coordinate_pipeline.rs 完整成品来源、坐标编辑与空间增量验收
  render.rs       原生 PNG/TGA 和文字标注
  build.rs        构建、验收、发布、安装与 WorldBuilder 导出
  release.rs      成品 ZIP 的增量编辑、封装校验与新版本发布
maps/<id>/
  config.json     名称、尺寸、层高、纹理、文本和输入路径
  layout.json     新蓝图路径的定稿布局；现有四图保留历史参考
  v11_layers.npz  新蓝图路径的分层输入；现有四图保留历史参考
  authoring/      现有四图的完整原生来源、可编辑布局及空间检查基准
  version.json    版本状态与历史
  data/           地理边界等原始素材
templates/
  8p_template.map 地图骨架
  rules/          已链接规则的声明式脚本树与阵营/队伍模板
  worldbuilder/   WorldBuilder 导入插件源码
tests/            编解码、失败回滚与四张地图的回归测试
lib/opensage/     参考源码
docs/             地图格式和迁移说明
```

公共地形主纹理已按用户定稿统一为五层：`Grass_NewYork01` → `Grass_MtRush04` → `Grass_CapeCod05` → `Rock_Yucatan03` → `Rock_Iceland02`。前三层草地保留已有变体，四五层采用棕岩、浅灰岩；新蓝图构建和显式 `polish-textures` 共用规则，缺失材质自动补齐并同步加载清单。冻结成品按原制作源继续，不自动重刷；本次接入没有发布或安装地图。见 [通用纹理规则](../spec/terrain_texture_rules.md)。

配方与布局的修改方法见 [Rust 迁移与数据配方](../spec/rust_migration.md)。历史 DEM、地理边界与预览素材保留在各地图目录；现有图继续制作以冻结原生成品为准，新蓝图路径以配方指定的分层蓝图为准。

通用玩法模板为 `templates/rules/common-T0.5.1.json`，来自千壑横原前身 T0.5.1 的 224 条脚本。碧嶂回澜、千流叩海、翠屿浮光通过 `RULESET: "common-T0.5.1"` 共用它。新蓝图构建会在各图出生点附近的平坦陆地补齐 31 个超武、电厂航点，并检查占地、水域、障碍、矿位与建筑间距；落点报告保存在产物的 `script_waypoints.json`。冻结成品构建保留来源已有的脚本与航点，并回读检查。模板保留第二座产能及固定 AI，不包含电脑现金显示、主动进攻、按位置分队和四人落座脚本。

千壑横原 T0.6.5 继续保持 T0.6.3 已取消分钟进攻的状态：没有 `minute-attack-*` 脚本或对应诊断计数，专属模板不配置 `periodic_attack`。`update-scripts --map 千壑横原 --bump <源ZIP>` 支持在移除配置后清理成品中的旧模块。详见 [脚本变更记录](../spec/periodic_attack.md)。

千壑横原 T0.6.4 已将 568、567 号斜坡分别移到原 376、244 的位置，并按授权挪开医院、避让树石。指定坡心、旧口恢复和端口通行通过核对，使用 `relocate-ramps <源ZIP> <方案JSON> --map <id> --bump` 增量编辑。见 [斜坡移动记录](../spec/ramp_relocation.md)。

千壑横原 T0.6.5 在原 377、220、309 位置新增三坡（569/570/571），按授权将 200 号矿移至同层 (752,470)。保留原 24 坡，矿位建造盘、矿口和资源通路通过检查。见 [斜坡新增记录](../spec/ramp_additions.md)。

山地坡口参考 AXAI、NKL、SMAI 系列的实际地形和 WorldBuilder 渲染。`polish-ramps` 按成品高度重做稳定坡面、收拢坡肩，并将局部材质接回上下台地；同时保护赠送建筑主、备用完整占地。碧嶂回澜 T2.3.6 更新 24 处坡口材质、重做其中 11 处坡形，空间增量检查无新增或恶化问题。见 [参考与前后对比](../spec/mountain_ramps.md)。

碧嶂回澜 T2.3.7 按实机反馈取消坡面孤立的短土斑，让坡面连续接回两端草地。`polish-ramp-surfaces --map <id> --bump <上一版ZIP>` 仅整理坡口材质，高度和全部通行/建造位图逐字节保持；后续山地坡口也不再主动刷短土痕。

通用 `RAMP_PROFILE` 缺省为 `mountain`：新蓝图自动使用当前坡形与草地衔接，烘焙和冻结成品保留实际原生高度；已有图从最新 ZIP 局部升级后重新采用为制作源。旧版蓝图复现显式设 `RAMP_PROFILE: "legacy"`。通用构建与纹理整理共用规则，流程与边界见 [通用斜坡默认](../spec/mountain_ramps.md#通用流水默认)。

其他三图已应用当前通用斜坡并打包安装：千壑横原 T0.6.7、千流叩海 T0.5.6、翠屿浮光 T0.3.6。制作源及坐标同步更新，64 处坡口刷新材质、33 处重做坡形，空间检查无新增/恶化问题。见 [发布与验收记录](../spec/mountain_ramps.md#其他三图通用化发布)。

千壑横原 T0.6.8 在四处红框位置增加不可翻越岩石高山，参考 AXAI 麦格马戈登原生山体。山体全域陆地禁行，其他导航位图保持；528 条资源路线、27 条坡及原有对象和脚本保留。已验收、安装并更新制作源，游戏内待复测。见 [高山编辑记录](../spec/impassable_mountains.md)。

千壑横原 T0.6.9 保留 48 矿，调整 32 个外扩矿：家区最多 3 矿，第 4 矿沿四条主线、第 5 矿进入争夺热点、第 6 矿沿次要路线，固定 AI 与玩家采用同一规则。验收已检查矿位分级和家区上限，四处高山及其他地形、对象和脚本保持；已安装并更新制作源。见 [资源分布规则](../spec/resource_distribution.md)。

千壑横原 T0.7.2 修整 568 号坡口迁移后遗留的方形高台与凹槽，圆顺崖线和材质边缘；原坡心、方向与 25 格完整通行宽度保持。实际来源空间检查无新增或恶化，已核对 WorldBuilder 渲染。见 [修整记录与对比](../spec/mountain_ramps.md#千壑横原-t072清除旧坡口残留方台)。

千壑横原 T0.7.1 按用户标出的“44gg 正面硬钢”山体重做四处高山：连贯圆润的岩脊、较宽山脚及灰棕岩壁，植被随山面升起。48 矿、分级布矿及主次路线保持，陆军禁行、禁飞位图保持；已核对 WorldBuilder 俯视图、验收并安装；后续实机反馈发现岩壁黑块，见下述排查。见 [山体重做与对比](../spec/impassable_mountains.md#t071按-44gg-正面硬钢的红框山体重做)。

千壑横原 T0.7.3 整理新增岩石在运行时图集中的顺序，逐字节保留 T0.7.2 原生地图。已通过封装和空间验收；游戏内黑块是否消失仍待近景对照，不能用俯视渲染代替验证。见 [排查记录](../spec/impassable_mountains.md#t073排查游戏内岩壁黑块)。

千壑横原 T0.7.4 修复地形封装流水：新增岩石同步原生和 XML 加载清单，刷新地图/图集版本 ID，混合记录从 8,500 条整理为 1,844 条。高度、矿位、对象身份和全部导航保持；发布与安装会拦截缺项封装。见 [地编保存对照与流水修复](../spec/terrain_package_fix.md)。

公共地形封装已统一为“暂存处理 → 清单同步 → 校验 → 保存”，新图、增量编辑和坐标新版本共用。仅执行 map 或采用制作源也会拒绝残缺材质声明；只读检查避免重复解包、克隆和重编码。102 项回归通过，冻结成品重建逐字节保持。见 [公共入口与失败保护](../spec/terrain_package_fix.md#公共入口收敛与失败保护)。

通用脚本由 **224 条基础脚本 + 公共赠送建筑规则** 组成。以下三项统一声明在 `common-T0.5.1.json#/production_placement`，所有可玩地图均应用，包括使用独立 `RULESET` 的千壑横原。

| 通用规则 | 统一行为 | 执行方式 |
|---|---|---|
| 赠送避让 `gift-clear-*` | 检查主、备用落点；主点被占尝试备用点，全占保留资格重试，成功后防止重复赠送 | 构建时生成主备航点、检测区域和运行时占用脚本；条数随赠送项目变化 |
| 建筑水陆落点 | 重工完整占地位于合格平坦陆地，船厂完整占地位于水域；主备点均满足约束 | 构建时选址，成品回读验收 |
| 赠送建筑朝向 | 赠送兵营、重工、机场、船厂、电厂和超武朝向对方队伍出生区域中心，按各自主备落点吸附到最近的 45、135、-45、-135 度 | 构建时设置创建动作的角度，成品回读验收 |

三项保留原赠送条件、阵营及归属。公共规则同时检查建筑间距和障碍；找不到合格位置则构建失败。对应报告为 `gift-clearance.json`、`production-spawns.json`、`production-facing.json`。水陆和朝向属于通用脚本配套的构建规则，脚本总数按成品实际统计，不固定增加三条。

已有成品通过 `cargo run --release -- repair-production --map <id> --bump --note '应用通用赠送建筑规则' <上一版ZIP>` 统一应用水陆、避让和朝向规则，安装加 `--install`。普通 `repack` 仅重新封装。公共规则的适用范围、配置及历史发布记录见 [公共赠送建筑规则](../spec/production_placement_fix.md)。

## 构建阶段

完整顺序为 `terrain → passability → map → verify → spatial → preview → coordinates → accept → readme → dist → wb`，以 `src/build.rs` 的 `STAGES` 为准。失败时返回非零退出码。现有四图在这些阶段读取冻结成品并执行空间增量检查；新蓝图路径生成地形和通行位图，其中 `spatial` 阶段尚不执行独立空间审计，仍由 `verify`、`accept` 执行对应验收。

```sh
# 只重建地图并回读校验
cargo run --release -- build --map 碧嶂回澜 --only map --only verify

# 单独执行某个阶段
cargo run --release -- terrain --map 碧嶂回澜
cargo run --release -- accept --map 碧嶂回澜
cargo run --release -- dist --map 碧嶂回澜 --no-zip

# 修改后递增版本并安装到游戏
cargo run --release -- build --map 碧嶂回澜 --bump --note '调整矿位' --install

# 导出并安装地编导入文件
cargo run --release -- build --map 碧嶂回澜 --wb --wb-root 'D:\app\Ra3NewWorldBuilder'
```

`--only`、`--skip` 可以重复，只筛选已有阶段，不自动补齐前置依赖；拼错阶段名会报错。地图安装需要设置 `APPDATA`，安装目标为 `APPDATA/Red Alert 3/Maps/<发布名>/`。`wb` 默认导出资料，复制到地编目录还需 `--wb`，目标通过 `--wb-root` 或 `RA3MAP_WB_ROOT` 指定。

`preview` 阶段自动额外生成 `preview--layout.png` 全要素编号图与 `layout-index.json` 索引，保存在当前版本的 `assets` 目录。编号覆盖定稿布局中的每一个对象（包括矿脉、油井、科技建筑、房屋、树木、灌木、石头、装饰、出生点和航点）及每一条斜坡。图片按类别着色，引线连接编号与实际落点；斜坡另画配方范围框和坡轴。画布北朝上，可放大查看密集位置。图外对象会扩展显示范围，在灰底区域保留真实落点。

编号先按实际制作布局的 `objects` 顺序从 1 排起，再排 `ramps`；现有四图使用 `authoring/layout.json`，新蓝图路径使用配方指定的布局。索引记录编号、类别、格坐标、对象类型、唯一名称及 JSON Pointer 路径；`position_pointers` 列出移动时应同步修改的坐标字段。只移动坐标不会改变编号，增删或重排条目后需以新图为准。布局变更后可运行 `cargo run --release -- build --map <id> --only terrain --only preview` 刷新编号图，无需打包安装。

本机发布约定：每次打包完成后，将成品地图目录复制一份到 `C:\Users\cherrol\AppData\Roaming\Red Alert 3\Maps\`。当前 WSL 对应路径为 `/mnt/c/Users/cherrol/AppData/Roaming/Red Alert 3/Maps/`；使用常规构建安装时设置 `APPDATA=/mnt/c/Users/cherrol/AppData/Roaming` 并传入 `--install`。独立 repack 或地编重存包也需要在打包后复制成品目录；ZIP 本身保留在 `dist/`。

已有成品只需要修复封装、重新发布时，使用 `repack`，保留所有地图区块、地形、对象与脚本，要求递增版本：

```sh
APPDATA=/mnt/c/Users/cherrol/AppData/Roaming cargo run --release -- repack \
  --map 千壑横原 --bump --note '修复地图封装与资源配套' --install \
  'dist/[CHAI]千壑横原/[CHAI]千壑横原T0.5.6.zip'
```

该入口可用于四张地图各自的源 ZIP。地编定稿图必须使用完整的地编保存包，保留 `AssetList`、`map.xml` 与脚本资产声明；资源资产 ID 保持不变，只更新 XML 文件路径中的版本。发布和安装会校验字符串表严格按 ID 倒序、纹理引用及资源配套，成品中存在的 XML 和地编备注也会随包复制。新版本路径已存在时拒绝覆盖。

缺少资源配套的历史成品可运行 `cargo run --release -- repair-resources --map <id> --bump --note '修复资源封装' <上一版ZIP>`。该命令保留高度、斜坡、通行/建造位图、对象和脚本，重建资源清单及 XML，并沿用上一版实际布局快照。构建和发布会拒绝缺少资源清单、XML、图集、脚本列表声明或被引用配套文件的包。原版资源定义来自 RA3 1.12 的 WBStatic/WBData 目录；新增非全局对象需在 `OBJECT_RESOURCE_SOURCES` 中声明 XML 来源。

已有成品的斜坡可用 `cargo run --release -- polish-ramps --map <id> --bump --note '优化斜坡' <上一版ZIP>` 局部修改后发布。该命令只改高度和地表区块：坡轴按附近两层边界调整，坡肩与坡口平缓衔接，纹理逐格铺设并写入两种纹理的混合数据。实际坡面宽度至少 20 格；允许第一层直连第三层，但坡面须避开第二层等第三个台地。每个候选检查全宽并行车道、两端台面、建筑和矿盘占地、原有陆地/水域出生点到矿位的连通关系；不合格的坡保留原地形，仅整理纹理。对象、脚本、图集、资源清单及其他区块逐字节保留。安装可加 `--install`。

新蓝图构建默认采用 `classic-natural` 纹理规则，沿用已确认的碧嶂回澜前身 T2.2.9 分布规则和当前五层主纹理；按实际地形重新选材的预览方案未启用。新蓝图 `build` 在 `map` 阶段固定高度、对象与脚本后，应用纹理分配和混合规则，再同步资源封装、校验与打包。冻结成品 `build` 保留来源材质，应用当前规则需显式运行 `polish-textures`。配方可写 `TEXTURE_PROFILE: "legacy"` 复现迁移前纹理；缺省或 `"classic-natural"` 使用当前规则。完整规则见 [通用纹理规则](../spec/terrain_texture_rules.md)。

已有成品使用 `cargo run --release -- polish-textures --map <id> --bump --note '应用通用 T2.2.9 纹理规则' <上一版ZIP>`。它调用与普通构建相同的选材逻辑，沿用成品及其布局快照，仅重绘 `BlendTileData` 的纹理和混合数组。使用原图已登记的纹理与资源编号，逐字节核对高度、全部通行/建造位图、对象和脚本。`textures--comparison.png` 展示实际材质分布的示意颜色，不是游戏截图；`texture-report.json` 记录规则名、参考版本、前后纹理数量与保留检查。已有局部斜坡优化的成品应走此入口，以保留其几何与布局。

逐坡结果、前后对比和编号图写入新版本 `assets` 目录。保留该目录的 `source/layout.json`，作为后续增量修改的实际布局。增量发布入口不会自动完成坐标导出、WorldBuilder 导出或采用制作源；普通 `repack` 也不会生成布局快照，缺少时须先补齐并核对同版实际布局。

现有四图增量编辑后，先核对相对上一版的空间检查结果、新成品与 `source/layout.json`，再运行 `cargo run --release --locked -- adopt-coordinate-source --map <id>`。该命令会更新冻结包和空间问题基准，不能替代对本轮新增/恶化问题的检查。采用后运行不带 `--bump` 的完整 `build --map <id>`，补齐当前版预览、空间报告、坐标、发布包和 WorldBuilder 资料；后续制作沿用这个新来源。尚未启用冻结成品模式的新图继续从配方指定的蓝图与布局生成，定稿后可采用完整成品启用该模式。操作步骤见 [成品编辑后的衔接](../spec/coordinate_pipeline.md#成品编辑后的衔接)。

新蓝图验收包含成品回读、对象边界、脚本格式和第二座产能路径点引用、近矿配额与朝向、矿场建造圆盘、出生点平整跨度、斜坡剖面及高度场轴向。冻结成品验收还会比对原生投影、检查赠送建筑与资源路线，并拒绝相对基准新增或恶化的空间问题。版本递增后的构建失败会恢复原始 `version.json`，只清理本次新产生的版本路径。

## 版本与产物

测试版为 `T0.0.1` 起，正式版为 `V1.0.0` 起，十进制进位：`T0.0.9 → T0.1.0`。工具链重构不递增地图内容版本。

```sh
cargo run --release -- version --map 碧嶂回澜 --show
cargo run --release -- version --map 碧嶂回澜 --release --note '首个正式版'
cargo run --release -- version --map 碧嶂回澜 --set T2.2.1 --note '指定版本'
```

- `assets/<地图名>/<发布名+版本>/`：地图、TGA、文本、预览、说明、地形缓存及地编导出。
- `dist/<地图名>/<发布名+版本>/`：包含 `.map`、`_art.tga`、`map.str`、`map.xml`、`overrides.xml`；地编备注等已有配套文件也会保留。
- `dist/<地图名>/<发布名+版本>.zip`：同样完整的资源配套文件，带发布目录，可直接解压到游戏 Maps 目录。

发布前会检查地图、预览、文本和 XML 配套文件齐全，并核对资源清单、脚本资产声明与纹理图集，再替换目标发布目录。ZIP 的文件顺序和时间戳固定，同一输入可重复得到相同压缩包。

## 格式与轴向

内部高度、布局和预览均为第 0 行在北侧。写入 `.map` 时将高度和逐格纹理/位图翻到游戏坐标，物体 y 同步换算；TGA 用底部原点保存，读回仍为北朝上。

RefPack 兼容社区地图的 `0x80/0x81` 大文件头。输出使用标准 `0x10 0xFB`。所有读取检查长度、区块范围和引用，未知顶层区块保留。写出字符串表时统一使用严格的 `N…1` 顺序；导入允许旧顺序，以便修复历史成品。程序生成图从实际对象、脚本创建的单位、道路、环境和音乐重建 `AssetList`，同时生成匹配的 `map.xml`、纹理图集和脚本资产声明；不再删除资源清单后依赖按需加载。地编完整导出包重新发布时保留其有效清单与配套 XML。

## 诊断命令

```sh
ra3maps inspect path/to/map.map
ra3maps dump-scripts path/to/map.map
ra3maps check-bounds path/to/map.map
ra3maps unpack packed.map plain.map
ra3maps pack plain.map packed.map
ra3maps analyze-terrain --map 碧嶂回澜
ra3maps refpack compress input.bin output.refpack
ra3maps --help
```

PNG 标注会查找系统中文字体，也可通过 `RA3MAP_FONT` 指定 TTF/TTC 文件。Windows 默认查找微软雅黑和黑体，Linux 查找 Noto CJK、文泉驿，再退回系统字体。

## 验证

首次检出与 CI 使用相同顺序：先恢复冻结制作源和最小历史测试输入，再检查输入与工具链。`restore` 遇到内容不同的已有目标会停止，不覆盖本地制作成果；输入清单、更新与恢复细则见 [可复现构建与测试输入](../spec/reproducible_builds.md)。

```sh
cargo run --locked --example test-data -- restore
cargo run --locked --example test-data -- check
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

回归测试在临时目录、空 `PATH` 下完整构建四张地图，包含发布与地编导出，并与对应版本的 `.map` 区块及压缩文件校验值比较。采用通用模板的三张图还检查脚本一致性、建筑航点引用与占地、原有地形及对象保留；测试覆盖非法输入、版本进位、失败回滚、确定性打包及隔离安装。当前生产流程以这些自动化检查作为验收依据，不包含实机检查步骤。
