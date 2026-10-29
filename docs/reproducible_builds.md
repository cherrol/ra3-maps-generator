# 可复现构建与测试输入

干净检出包含可玩地图的配方、冻结制作源及最小历史测试素材，不依赖开发者已有的 `assets/`、`dist/`、`work/` 或游戏安装目录。历史产物按测试实际读取的文件保存，避免把整份本地制作目录纳入 Git。

## 首次检出与 CI

安装项目要求的 Rust 工具链及 `rustfmt`、`clippy` 后，在独立干净检出副本的根目录执行：

```sh
cargo run --locked --example test-data -- restore
cargo run --locked --example test-data -- check
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

`restore` 离线读取 `tests/fixtures/historical-inputs.json`、`historical-inputs.zip` 及各图 `authoring/source.zip`。它先验证清单、文件长度、CRC32 和全部目标路径，再恢复测试需要的 `assets/`、`dist/` 文件；已有内容相同则复用，存在不同内容则停止，不覆盖本地制作成果。这里的离线仅指素材恢复，首次编译仍需 Cargo 依赖可用。

`check` 检查所需目标是否齐全且与输入一致。缺失或损坏会明确失败；修复输入或在单独的干净工作副本中重新恢复后再测试。现有制作目录与冻结包不同时，应先核对差异，不用删除整个 `assets/` 作为恢复步骤。例如当前制作缓存的 `source/layout.json` 与采用来源时规范化后的包内布局可能字节不同，即使地图文件相同，恢复也会拒绝覆盖；这时使用干净副本。CI 同样先恢复、检查，再执行格式、Clippy 和完整测试。

CRC32 用于发现误损和版本不一致，不是安全散列，也不证明素材来源或可信性。

## 受控输入与生成物

`.gitignore` 对 `maps/` 使用文件白名单，规则可用于后续新地图：

| 位置 | Git 中保留的输入 |
|---|---|
| `maps/<id>/` | `config.json`、`version.json`、所选蓝图原字节镜像 `layers.npz`、本图 `coordinate-system.json`；默认选择为 `maps/.default` |
| `maps/<id>/` 预览 | `README.md`、`source.png`、`preview.png`、`preview--line.png` 及已有的 `preview--center--line.png` |
| `data/terrain-preview/` | 当前高程输入 `input.npz` / `reference.npz`、分层 `terrain.npz`、参数、设计坐标、行政中心、来源说明、水域方案和核对报告；白名单同时保留参数与报告引用的 `raw/` JSON/NPZ 快照 |
| `histories/v<版本>/` | 已有蓝图 `layers.npz`、同版 `layout.json`、`ramps.json` 及历史预览 PNG；当前四图选用 v11 |
| `histories/legacy/` | 版本无法确认的旧 `layers.npz` 原件 |
| `authoring/` | 完整 `source.zip`、可编辑 `layout.json`、空间 `baseline.json`、可选 `policy.json`、当前编号对照及 `resource-distribution.json` |
| `histories/T<版本>/` | 各版 `coordinate-id-map.json`、历史方案与配置/布局/基准快照、所需局部高程 `.u16`、材质 `.u8`、蓝图 NPZ、冻结源与原名历史输入 ZIP；同版可玩图的预览、地形、通行、布局图片及 `preview-snapshot.json` |
| `templates/` | 地图骨架、脚本与规则模板、资源标识数据和 WorldBuilder 导出模板 |
| `tests/fixtures/` | 回归基线、历史素材清单及其最小 ZIP、绑定原样几何的坡口净空参考报告 |

`assets/`、`dist/`、`work/`、`target/` 和 `archive/` 继续忽略。当前预览图片、离线复现输入及被引用的设计快照通过精确白名单进入 Git；下载的 Terrarium 栅格瓦片、其他原始地理资料、日志和包含未受控缓存的旧 `manifest.json` 继续忽略。原始瓦片无需再次下载，预览直接读取已保存的高程与掩码 NPZ。`.gitattributes` 将地图、PNG、ZIP/NPZ 和局部地形数据标为二进制，避免 Windows 检出时转换字节。若新增输入不属于白名单，先评估并明确补充规则，不使用 `git add -f maps/` 绕过边界。

## 预览与实际游戏地图

纯地形设计预览在 `maps/<id>/`，实际可玩图图片在 `maps/<id>/histories/T<版本>/`。后者的 `preview.png` 为玩法总览，`preview--layout.png` 标注对象编号，`preview--terrain.png` 和 `preview--passability.png` 分别显示实际原生地形与通行；同目录的 `layout-index.json` 解释编号，`preview-snapshot.json` 绑定配置、可编辑布局、完整原生源包、字体及图片的 SHA256。

实际游戏文件由已经受控的 `maps/<id>/authoring/source.zip` 完整保存，包括同名 `.map`、`_art.tga`、`map.str` 及随包 XML 等文件。可编辑布局和验收基准另存 `authoring/`；采用新源时按下文更新整套同版输入。游戏安装包仍由这些文件构建到 `dist/`，不用再次跟踪整份生成目录。

从工程根目录可直接离线重做设计预览，或从冻结原生源生成可玩图图片：

```sh
cargo run --release --locked -- terrain-preview 广西 maps/广西/data/terrain-preview/reference.npz
cargo run --release --locked -- build --map 广西 --only preview
```

第一条读取本图参数及其 `water_design_land_reference` 依赖；存在 `centers.json` 时同时重绘中心标点图。第二条在 `assets/<id>/<发布名+版本>/` 输出 `preview.png`、`preview--layout.png`、`layout-index.json` 及 `source/terrain_preview.png`、`source/passability_preview.png`。保存可玩图图片时，将这两张 `source/` 图片分别归档为 `preview--terrain.png`、`preview--passability.png`，并同步同版图片和来源指纹。

小水域归层后的 10 张设计预览改由本图参数引用 `water-layer-reference.npz` 与对应报告，仍使用上述普通预览命令离线复现。修改前快照和逐水域验收记录也在同图 `data/terrain-preview/` 纳管，详见[全系列归层重绘记录](preview_water_layers.md)。

NPZ 分层数据可以独立核对；要得到逐字节相同的带文字 PNG，还需使用相同字体，可通过 `RA3MAP_FONT` 指定。河南当前只有三张基础预览，没有行政中心图；可玩成品仍使用其冻结原生网格。

2026-10-29 在只复制 Git 文件和本次新增受控资料的隔离目录中，34 张设计预览的 135 张现有 PNG 及全部已有 NPZ 数组逐字节复现；五张可玩图均能从冻结源生成上述图片，完整源包成员也可离线原样恢复。本轮使用 `wqy-zenhei.ttc`，SHA256 为 `79c18ebe7b811951e8311bad7103ebeae8c337ed9988ea69e8a78a66cfe029b9`。

同时补齐河南配置已引用但缺失的 `histories/v1/layers.npz`，字节来自已受控的根 `layers.npz` 镜像；河南、海南、陕西的 `map → verify → preview → accept → dist` 重建通过，原生游戏文件与冻结源一致。广东、广西的图片与源包可恢复，但完整重建仍受既有 XML 资源版本校验拦截：广东 `GameMap` 资源 ID 含 `T0.4.8`，当前版本为 `T0.5.6`；广西含 `T2.2.6`，当前版本为 `T2.3.7`。本次资料纳管保留原包字节，未重新采用源或改写这些资源 ID。

四张现有地图的常规制作读取 `COORDINATE_PIPELINE` 指定的 `authoring/source.zip` 和 `authoring/layout.json`。`histories/v11/layout.json`、`histories/v11/layers.npz` 仍供迁移回归测试复现旧蓝图；四图显式配置 `LAYERS: "histories/v11/layers.npz"`，缺省输入名为 `layers.npz`。根 `layers.npz` 与所选定稿蓝图原件逐字节一致，不从成品地图回读；其他版本资料按 [版本目录约定](blueprint_versions.md) 保留。地图 id 使用真实地形名，默认选择文件填写 `广西`。恢复当前源包时，包内游戏文件放入 `assets/<真实地形名>/<生成发布名+版本>/`，包内 `layout.json` 对应该目录的 `source/layout.json`；恢复不会重做地形、搜索布局或递增地图版本。

## 最小历史素材

当前历史 ZIP 只包含以下 15 个文件，长度与 CRC32 以 `historical-inputs.json` 为准。每项 `path` 保留 ZIP 内原来的历史成员名，`target` 指定当前恢复位置；迁移目录不会改写历史 ZIP。下表列出恢复目标，assets 的共同前缀为 `assets/陕西/`，同名地图文件仍包含完整 `[CHAI]千壑横原<版本>` 前缀。

| 成品目录（位于 `assets/陕西/`） | 文件 |
|---|---|
| `[CHAI]千壑横原T0.6.7` | 同名 `.map`、`source/layout.json` |
| `[CHAI]千壑横原T0.6.9` | 同名 `.map`、同名 `_art.tga`、`map.xml`、`map.str`、`overrides.xml`、`NewWorldBuilder.notes.json`、`source/layout.json` |
| `[CHAI]千壑横原T0.7.1` | 同名 `.map`、`source/layout.json` |
| `[CHAI]千壑横原T0.7.6` | 同名 `.map` |
| `[CHAI]千壑横原T0.7.7` | 同名 `.map`、`source/layout.json` |
| `dist/陕西/` | `[CHAI]千壑横原T0.6.9.zip` |

历史文件分别覆盖山体添加与重做、坡肩修复、矿口建造空间和斜坡占用回归。当前版本由受控冻结源恢复，避免在历史 ZIP 中重复保存。无需导入其他发布版本或整套坐标、纹理及预览导出。

## 更新制作源与历史用例

修改现有图时，先按 [坐标制作流水](coordinate_pipeline.md) 完成版本递增、成品差异核对、空间增量检查和验收。采用新源会重设问题基准，因此必须在采用前确认本轮没有新增或恶化问题。核对同版 `source/layout.json` 后，再执行：

```sh
cargo run --release --locked -- adopt-coordinate-source --map <id>
cargo run --release --locked -- build --map <id>
```

提交该次修改涉及的 `config.json`、`version.json`、冻结源包、可编辑布局、基准、策略、编辑输入和必要编号对照；保持配置中的源名称、源 CRC 和 ZIP 内容一致。新图启用冻结制作源后，将其 id 加入 `historical-inputs.json` 的 `maps` 列表，使恢复工具也准备该图的当前成品。地图内容变化要递增版本；地图字节未变的纯工具链修改不递增。提交前在干净工作副本中执行首次检出的恢复及完整检查，确认新源确实能独立复现。

新增受控输入只精确放行实际使用的文件，不因增加 `histories/v1/`、`histories/v2/`、`histories/v3/` 或 `T` 版本目录就把全部历史资料纳入 Git。

新增测试若必须读取历史产物，应明确把最少历史成员路径 `path` 和恢复目标 `target` 加入 `tests/fixtures/historical-inputs.json`，保留现有回归案例，然后在具备这些文件的工作副本执行：

```sh
cargo run --locked --example test-data -- capture
cargo run --locked --example test-data -- check
```

`capture` 只按清单已有的条目从 `target` 读取文件，以 `path` 作为 ZIP 成员名重建确定性的历史 ZIP，并更新各文件长度及 CRC32；未提供 `target` 时沿用 `path`。它不会扫描或收录整个 `assets/`、`dist/`。目录迁移只改恢复目标及当前地图 id，不运行 `capture`，不改历史 ZIP 成员、长度或 CRC。审查清单与 ZIP 差异，提交两者，并再次验证干净恢复和测试。增加当前制作输入则更新地图文件白名单，无需自动扩充历史 ZIP。

## 外部资源边界

自动化测试使用已保存地图、局部编辑数据和资源标识，不要求安装原版游戏或读取 `Terrain.big`、`TerrainFix.big`。清单或源包中材料缺失时应报错，不能通过跳过相应回归把结果记为通过；需要补充有权使用的对应输入并保持版本一致。

预览文字可通过 `RA3MAP_FONT` 指定本地字体；没有可用字体时渲染器会省略文字，地图数据测试仍可运行。验收覆盖地图数据、封装、静态空间规则及脚本约束；生产流程不包含实机检查步骤。

## 本次验证

2026-10-29 将地图版本资料统一归档到 `histories/<版本>/`，并接入选定蓝图原字节同步。在独立干净副本中，仅复制 Git 可见的 592 个文件，初始没有 `assets/`、`dist/` 或 `work/`；离线恢复并核对 40 个测试输入后，格式、全目标严格 Clippy 和完整回归的 186 项测试全部通过，四张地图的构建结果保持基线字节。

四图根 `layers.npz` 均与配置选中的 `histories/v11/layers.npz` 原字节一致；历史输入、现有成品、冻结制作源与版本号按迁移前 SHA256 核对。构建验证提交前失败保留旧镜像，提交后的安装失败保留同版成果；新增回归覆盖这两种边界。发布 CLI 已重新编译。明细及日志见本地 `work/blueprint-versions/verification.json`。Windows 检查已配置在 CI 矩阵中，本轮在 Linux 运行，没有运行游戏。
