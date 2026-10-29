# RA3 地图工坊（ra3maps）

使用原生 Rust 制作和编辑《红色警戒 3》地图，集成地形、对象、脚本、验收、版本、打包与 WorldBuilder 导出。运行时无需 Python；编译需要 Rust 1.88 或更新版本及 Cargo.lock 锁定依赖。

地图合集为 **九州大地**，默认选择是广西（碧嶂回澜）。当前有五张可玩地图配方（含河南 800×768 矩形地图）；地形预览和坐标声明不等于可玩成品。数量、版本与未完成边界见 [当前状态](docs/SESSION_STATE.md)、[待办](docs/TODO.md)。技术导航见 [文档索引](docs/README.md)，逐版本事实和旧入口保存在 [历史索引](docs/history/README.md)。

## 入口

首次检出先按下方 [验证](#验证) 恢复并核对受控制作输入。之后可编译和查看地图：

```sh
cargo build --release --locked
cargo run --release --locked -- list
cargo run --release --locked -- --help
```

地图选择顺序为 `--map`、`RA3MAP_ID`、`maps/.default`，最后尝试唯一配方；id 使用真实地形名，例如 `--map 陕西`，默认文件填写 `广西`。从其他目录调用时加 `--root <工程根目录>`。也可使用 `build.sh`、`build.cmd` 或 `build.ps1` 包装入口。命名按 [系列命名](docs/九州大地命名.md)，地图数据结构见 [Rust 配方](docs/rust_migration.md)。

## 制作流程

**真实资料 → 地形预览 → 定稿布局/冻结成品 → 构建验收 → 坐标资料与完整发布包。**

纯地形预览从本次提供的高程或参考图出发，保留完整地理取景，宽高统一重采样为 800×800 可玩格（四周另留 20 格边框），整张游戏正方形统一分层；主体地理边界外但游戏框内的真实地形同样纳入。真实海洋/湖泊需准备相应水域掩码，行政中心图需地理范围和中心点资料。基础图固定为 `maps/<id>/source.png`、`preview.png`、`preview--line.png`，同取景、同画幅、同水域；中心标点另生成 `preview--center--line.png`。纯预览不放玩法对象、不递增地图版本、不安装。详见 [预览标准](docs/terrain_preview.md)。

```sh
cargo run --release --locked -- terrain-preview <蓝图原型或图名> <高程NPZ>
cargo run --release --locked -- terrain-preview-centers <蓝图原型或图名>
```

预览图片与离线复现资料一同纳入 Git；实际游戏地图保留完整 `authoring/source.zip`，同版地形、通行和布局图片归档到 `histories/T<版本>/`。文件范围与复现方法见[图片与原生地图](docs/reproducible_builds.md#预览与实际游戏地图)。

可玩制作分两条路径：

| 路径 | 输入与行为 |
|---|---|
| 新图 | `config.json`、`LAYERS` 指定的蓝图（默认 `layers.npz`，含 GRID_HEIGHT×GRID 的 `base_z` 与 `z_scale`，高度未配置时取 GRID）及 `LAYOUT` 指定的布局（默认 `layout.json`）；布局显式定稿，预览缓存不直接作为构建蓝图 |
| 冻结成品 | `COORDINATE_PIPELINE` 引用完整源包、同版布局和基准；现有四图使用 `authoring/source.zip`、`authoring/layout.json`，保留实际原生高度、材质、位图与脚本 |

根 `layers.npz` 按原字节同步当前选定定稿蓝图，版本原件按 `histories/v1/`、`histories/v2/`、`histories/v3/`、`histories/v11/` 保存；它不从 `.map` 回读生成。四张现有图显式使用 `LAYERS: "histories/v11/layers.npz"`，历史配套布局在 `histories/v11/layout.json`，冻结生产来源仍为 `authoring/source.zip`。其他历史制作文件按 `maps/<id>/histories/T<版本>/` 归档，详见 [蓝图与版本资料](docs/blueprint_versions.md)。

每张图在资源落点前准备 `coordinates::Frame`，资源先在本图格坐标中计算，再统一换算为世界坐标。`maps/<id>/coordinate-system.json` 保存稳定声明，版本声明另存于 `assets/.../source/`、`coordinates/` 和 `coordinates.zip`。纯预览设计帧独立保存，已有可玩帧不被预览覆盖。范围、单位和正逆公式见 [地图坐标](docs/map_coordinates.md)。

出生点采用东/西、南/北或西南/东北对置；P1–P4 只能位于东、南或西南侧。每个出生点及其两座开局近矿的完整可用空间须共同位于一片连续平层或水面。制作范围及当前自动验收边界见[公共出生点规则](docs/spawn_placement.md)。

新图与重新布矿采用[公共分级布矿规则](docs/resource_distribution.md)：八个出生位各 6 矿，共 48 矿；家区 100 格内 3 矿，之后按主线前段、争夺热点和次线各安排 1 矿。配方用 `RESOURCE_DISTRIBUTION` 引用本图路线与矿位计划。未迁移的冻结成品可原样重建，改矿位或出生点时需补齐计划。

矿、油井、医院等中立建筑按完整预留占地避开坡面全宽、声明端口和沿轴双向通行区。公共策略位于 `templates/rules/coordinate-pipeline.json#/ramp_clearance`，当前默认两端延伸 25 格、净距 8 格；新图及显式 `RAMP_CLEARANCE` 执行零冲突，默认冻结源逐项留档旧问题并拦截新增/恶化。不会因接入规则自动挪动旧图资源。见 [坡口避让](docs/ramp_clearance.md)。

```sh
cargo run --release --locked -- build --map <id>
cargo run --release --locked -- build --map <id> --bump --note '调整布局'
cargo run --release --locked -- build --map <id> --only map --only verify
cargo run --release --locked -- coordinates --map <id>
```

完整阶段为 `terrain → passability → map → verify → spatial → preview → coordinates → accept → readme → dist → wb`。`--only`/`--skip` 可重复，不补前置产物；新图的 `spatial` 尚无独立空间审计，仍执行 `verify`、`accept` 和公共坡口门禁。玩法读取 `RULESET` 及实际成品脚本，赠送主备、水陆和朝向见 [公共赠送规则](docs/production_placement_fix.md)。

冻结成品的矿、医院等普通坐标修改直接编辑配置指定布局的 `objects[编号−1].cell/angle`，执行 `build --bump`；资源分类清单由对象身份派生。纹理、坡形或封装编辑沿用最新完整 ZIP，选择相应增量入口。核对实际差异、空间结果和同版 `source/layout.json` 后，显式采用并补齐资料：

```sh
cargo run --release --locked -- adopt-coordinate-source --map <id>
cargo run --release --locked -- build --map <id>
```

普通重封装不应用地形或玩法修复，增量发布不自动完成坐标/WB 导出或采用来源；操作边界见 [坐标流水](docs/coordinate_pipeline.md)。采用来源会更新基准，不能替代本轮新增/恶化检查。地图内容变化递增版本；纯工具链重构且地图字节不变时不递增。构建阶段、验证或根蓝图镜像同步在提交前失败时，恢复原版号及根镜像，仅清理本轮新建的版本产物，保留既有地图和旧版。可选安装在本地构建或发布提交后执行；安装失败明确报告“构建已完成，安装失败”或“发布已完成，安装失败”，保留已提交版本、成品和镜像。

本地目录使用真实地形名：`maps/陕西/` 保存输入，`assets/陕西/[CHAI]千壑横原T0.7.8/` 保存地图与制作报告，`dist/陕西/[CHAI]千壑横原T0.7.8/` 保存完整成品目录；ZIP 保留生成名，为 `dist/陕西/[CHAI]千壑横原T0.7.8.zip`。包内 `.map`、XML 和游戏安装目录继续使用生成名。原生清单、XML、图集、脚本声明和已有配套文件成套校验，发布与安装前复查资源/坡口约束。安装用 `--install`，需 `APPDATA`，目标为 `APPDATA/Red Alert 3/Maps/<发布名>/`；WB 默认导出资料，复制到地编另加 `--wb --wb-root <目录>` 或配置 `RA3MAP_WB_ROOT`。生产流程不包含实机检查步骤。

## 项目结构

```text
src/                 二进制与资源编解码、地形/坐标/脚本、构建及增量发布
maps/<真实地形名>/   当前配方、版本、坐标声明、根蓝图镜像与冻结 authoring/ 输入
maps/<id>/histories/ 各蓝图及成品版本的历史制作资料
templates/           地图骨架、公共/专属规则、WorldBuilder 导入资料
tests/               单元与集成回归、受控历史测试素材
tools/test_data.rs    受控素材恢复、检查和最小历史输入更新工具
tools/maintenance.rs  固定缓存清理，默认只输出计划
docs/                当前规范、状态入口及 history/ 历史记录
assets/<id>/<生成发布名+版本>/  生成的地图、制作资料与报告
dist/<id>/<生成发布名+版本>/    完整成品目录；同一 id 下保存生成名 ZIP
work/                本地诊断与研究，不作为首次检出的必需来源
```

技能入口见 [项目技能索引](docs/map_skills_checklist.md)，玩法脚本见 [脚本功能说明](docs/script_functions.md)。材质、坡形、封装和地图格式分别见 [技术文档索引](docs/README.md)；`inspect`、`dump-scripts`、`check-bounds`、`pack/unpack` 等诊断命令见 `ra3maps --help`。

## 验证

首次检出与 CI 使用相同顺序。恢复工具遇到内容不同的已有目标会停止，不覆盖本地制作成果：

```sh
cargo run --locked --example test-data -- restore
cargo run --locked --example test-data -- check
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

回归覆盖原有四张地图构建及横、纵矩形蓝图、冻结成品字节保持、资源封装、坐标/坡口/矿场约束、脚本引用、版本回滚、确定性打包和隔离安装。受控素材范围、离线恢复及历史结果见 [可复现构建](docs/reproducible_builds.md)。CI 配置包含 Linux 与 Windows，实际执行结果以对应日志为准。

本地缓存清理使用 `cargo run --release --locked --example maintenance` 查看计划，核对后加 `-- --apply` 执行；固定范围、平台限制和保留内容见 [维护说明](docs/maintenance.md)。
