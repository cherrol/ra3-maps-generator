# RA3 地图工坊（ra3maps）

使用原生 Rust 制作和编辑《红色警戒 3》地图，集成地形、对象、脚本、验收、版本、打包与 WorldBuilder 导出。运行时与作者工具均无需 Python（EA 镜像抓取仍为 shell 脚本）；编译需要 Rust 1.88 或更新版本及 Cargo.lock 锁定依赖。

地图合集为 **九州大地**。当前默认地图由 `maps/.default` 决定；各图配方、版本与冻结来源以本图配置为准，运行 `ra3maps project-status` 查看实时清单。预览、可构建输入和已验收交付分别记录，不从配方存在推定制作完成。制作要求见[公共约束总表](docs/spec/map_rules.md)，状态口径见[项目状态](docs/SESSION_STATE.md)。

## 入口

首次检出先按下方 [验证](#验证) 恢复并核对受控制作输入。之后可编译和查看地图：

```sh
cargo build --release --locked
cargo run --release --locked -- list
cargo run --release --locked -- --help
```

地图选择顺序为 `--map`、`RA3MAP_ID`、`maps/.default`，最后尝试唯一配方；id 使用真实地形名，例如 `--map 河南`，默认文件填写 `河南`。从其他目录调用时加 `--root <工程根目录>`。也可使用 `build.sh`、`build.cmd` 或 `build.ps1` 包装入口。命名按 [系列命名](docs/spec/九州大地命名.md)，地图数据结构见 [Rust 配方](docs/spec/rust_migration.md)。

## 制作流程

**真实资料 → 游戏预览与边界定稿 → 保持边界、安排斜坡与布局 → 构建验收 → 坐标资料与完整发布包。**

**制作地图过程中不允许修改层边界、水陆边界，允许添加斜坡。** 以选定预览及同版设计数据或完整冻结来源为基准，新增斜坡可包含必要的局部坡面高度与通行变化，并须独立核对两类边界保持不变；需要改边界时返回预览制作阶段。完整口径见[边界保护与斜坡添加](docs/spec/preview_terrain_lock.md)。

纯地形预览从本次提供的高程或参考图出发，保留完整取景，按[预览标准](docs/spec/terrain_preview.md)生成同画幅的原图、分层图和线稿；行政中心图另需地理范围和中心点资料。纯预览不放玩法对象、不递增地图版本、不安装。

```sh
cargo run --release --locked -- terrain-preview <蓝图原型或图名> <高程NPZ>
cargo run --release --locked -- terrain-preview-centers <蓝图原型或图名>
```

预览图片与离线复现资料一同纳入 Git；实际游戏地图保留完整 `authoring/source.zip`，同版地形、通行和布局图片归档到 `histories/T<版本>/`。文件范围与复现方法见[图片与原生地图](docs/spec/reproducible_builds.md)。

可玩制作分两条路径：

| 路径 | 输入与行为 |
|---|---|
| 新图 | `config.json`、`LAYERS` 指定的蓝图（默认 `layers.npz`，含 GRID_HEIGHT×GRID 的 `base_z` 与 `z_scale`，高度未配置时取 GRID）及 `LAYOUT` 指定的布局（默认 `layout.json`）；布局显式定稿，预览缓存不直接作为构建蓝图 |
| 冻结成品 | `COORDINATE_PIPELINE` 引用完整源包、同版布局和基准；具体源包与布局由本图配置指定，保持层边界、水陆边界及本轮无关的原生内容，允许按本轮计划添加斜坡 |

根 `layers.npz` 按原字节同步当前选定蓝图；冻结成品的生产来源仍为完整 `authoring/source.zip`。选定蓝图与版本资料的目录约定见[蓝图与版本资料](docs/spec/blueprint_versions.md)。

每张图在资源落点前准备本图 `coordinates::Frame`，格坐标与世界坐标通过同一 Frame 换算。纯预览设计帧独立保存。声明位置、单位和正逆公式见[地图坐标](docs/spec/map_coordinates.md)。

可玩布局按[技能分工与交接](docs/spec/map_skills_checklist.md)执行：先在既定边界内选择主攻路线，识别已有跨层斜坡并按需要添加斜坡，再布矿；**斜坡优先于矿**，冲突时调整矿位，保护坡体、坡口与完整上下引道。出生组、分级配额、中立建筑与完整占地分别读取[出生点规则](docs/spec/spawn_placement.md)、[资源规则](docs/spec/resource_distribution.md)和[坡口避让](docs/spec/ramp_clearance.md)，参数不在首页另存一份。

斜坡、层界、水陆交界及交汇处按[地形交接规范](docs/spec/terrain_transitions.md)贯穿预览、制作、纹理和交付：预览阶段定稿边界与设计，可玩阶段保持边界，允许添加斜坡并衔接材质；新增坡面的高度、导航及边界保护逐段留证，按[交接验收](docs/spec/map_acceptance.md#地形交接如何验)归档，不能以纹理整理或构建成功推定合格。

```sh
cargo run --release --locked -- build --map <id>
cargo run --release --locked -- build --map <id> --bump --note '调整布局'
cargo run --release --locked -- build --map <id> --only map --only verify
cargo run --release --locked -- coordinates --map <id>
```

开始构建前可执行快速诊断和有预算的布局搜索。搜索只输出候选报告，不采用布局、不移动基地或坡道；粗预检不等于完整验收。用法、缓存、断点、输入锁定和计时范围见[布局规划与诊断](docs/spec/layout_workflow.md)。

```sh
cargo run --profile authoring --locked -- check-layout --map <id>
cargo run --profile authoring --locked -- plan-layout --map <id> --budget 100000 --time-limit 60 --seed 0
cargo run --profile authoring --locked -- plan-layout --map <id> --resume
```

构建阶段以 `src/pipeline/build/mod.rs::STAGES` 为准，顺序与两条输入路径的检查差异见[坐标流水](docs/spec/coordinate_pipeline.md)。`--only`/`--skip` 可重复，不补前置产物。玩法读取 `RULESET` 及实际成品脚本，赠送主备、水陆和朝向见[公共赠送规则](docs/spec/production_placement_fix.md)。

发布、安装和采用来源还须通过[结构化独立验收](docs/spec/map_acceptance.md#结构化独立验收门禁)。首次制作可先 `build --map <id> --skip dist --skip wb` 生成候选，再用 `prepare-delivery --map <id>` 保存待核验清单；完成专业审核并在本图配置 `DELIVERY_ACCEPTANCE` 后执行 `dist --map <id>`。缺证据的增量编辑会保留待审候选并停止提交版本，审核后重试原操作。

冻结成品的矿、医院等普通坐标修改直接编辑配置指定布局的 `objects[编号−1].cell/angle`，执行 `build --bump`；资源分类清单由对象身份派生。纹理或封装编辑沿用最新完整 ZIP，选择不改地形的增量入口；添加斜坡须另列局部修改范围，并核对层边界、水陆边界及受影响通路，工具能力不代表已满足边界保护。核对实际差异、空间结果和同版 `source/layout.json` 后，显式采用并补齐资料：

```sh
cargo run --release --locked -- adopt-coordinate-source --map <id>
cargo run --release --locked -- build --map <id>
```

普通重封装不应用地形或玩法修复；增量入口的资料输出、来源采用与后续构建见[成品编辑后的衔接](docs/spec/coordinate_pipeline.md#成品编辑后的衔接)。采用新基准不能放行当前问题。地图内容变化递增版本，纯工具链重构且地图字节不变时不递增。构建回滚与安装失败的处理见[流水事务](docs/spec/coordinate_pipeline.md#制作输入)。

本地目录使用真实地形名：`maps/<id>/` 保存输入，`assets/<id>/<发布名>/` 保存地图与制作报告，`dist/<id>/<发布名>/` 保存完整成品目录，同一 id 下保存生成名 ZIP。包内 `.map`、XML 和游戏安装目录继续使用生成名。安装用 `--install`，需 `APPDATA`，目标为 `APPDATA/Red Alert 3/Maps/<发布名>/`；WB 默认导出资料，复制到地编另加 `--wb --wb-root <目录>` 或配置 `RA3MAP_WB_ROOT`。生产流程不包含实机检查步骤。

规范、skills、公共参数、本图覆盖和自动门禁的对应关系统一见[公共约束总表](docs/spec/map_rules.md)。流水按[公共交付验收](docs/spec/map_acceptance.md)汇总同版自动报告与独立证据；缺证据记待核验，不能以 CLI 成功替代完整验收。纹理与建筑朝向分别读取[纹理规范](docs/spec/terrain_texture_rules.md)和[建筑四向规范](docs/spec/production_placement_fix.md#所有建筑的四向约束)。

## 项目结构

```text
src/                 二进制与资源编解码、地形/坐标/脚本、构建及增量发布
maps/<真实地形名>/   当前配方、版本、坐标声明、根蓝图镜像与冻结 authoring/ 输入
maps/<id>/histories/ 当前选定蓝图与当前成品的配套资料
rules/               公共规则：coordinate-pipeline、common-<RULESET> 与脚本命令签名
templates/           地图骨架、资源清单与 WorldBuilder 导入资料
tests/               单元与集成回归、当前河南源与独立格式片段
xtask/               工程工具：冻结源恢复、缓存维护、预览批处理、EA 索引提取与官方语料校验（以 example 形式运行）
tools/               本地脚本：EA 官方镜像抓取（shell）
reference/           外部参考资料，不参与构建：opensage 解析器参照、离线 ra3wiki
docs/                文档：spec/ 规范、evidence/ 定版证据、history/ 历史
data/ea-modding/     EA 官方 RA3 数据镜像（本地，不入库；tools/fetch_ea_modding.sh 重建）
assets/<id>/<生成发布名+版本>/  生成的地图、制作资料与报告
dist/<id>/<生成发布名+版本>/    完整成品目录；同一 id 下保存生成名 ZIP
work/                本地诊断、work/runs/<id>/尝试计时，不作为首次检出的必需来源
```

目录按用途分三类：`src/`+`rules/`+`templates/` 是构建输入，`assets/`+`dist/`+`work/` 是生成物（不入库），`reference/` 是只供查阅的外部资料。游戏规则与可执行参数只在 `rules/` 维护，不放进 `templates/`。

技能入口见 [项目技能索引](docs/spec/map_skills_checklist.md)，玩法脚本见 [脚本功能说明](docs/spec/script_functions.md)。材质、坡形、封装和地图格式分别见 [技术文档索引](docs/README.md)；`inspect`、`dump-scripts`、`check-bounds`、`pack/unpack` 等诊断命令见 `ra3maps --help`。

## 验证

首次检出与 CI 使用相同顺序。恢复工具遇到内容不同的已有目标会停止，不覆盖本地制作成果：

```sh
cargo run --locked --example test-data -- restore
cargo run --locked --example test-data -- check
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

回归覆盖当前河南构建及横、纵矩形蓝图、冻结成品字节保持、资源封装、坐标/坡口/矿场约束、脚本引用、版本回滚、确定性打包和隔离安装。受控素材范围、离线恢复及历史结果见 [可复现构建](docs/spec/reproducible_builds.md)。CI 配置包含 Linux 与 Windows，实际执行结果以对应日志为准。

本地缓存清理使用 `cargo run --release --locked --example maintenance` 查看计划，核对后加 `-- --apply` 执行；固定范围、平台限制和保留内容见 [维护说明](docs/spec/maintenance.md)。
