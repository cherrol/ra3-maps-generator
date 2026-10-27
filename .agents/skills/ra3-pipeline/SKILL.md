---
name: ra3-pipeline
description: 地图构建、验收、版本、发布与安装；使用原生 Rust ra3maps。
---

# 流水线

原生 Rust 工具链，无 Python 运行依赖。配方为 `maps/<id>/config.json`，定稿布局为 `layout.json`。地图 id 来自参数、RA3MAP_ID 或 maps/.default，不写死某一张图。

通用纹理默认采用 `classic-natural`（已确认的 T2.2.9 方案），普通 `build` 在 `map` 阶段自动执行。按地形重新选材的预览方案未采用。已有成品只改纹理时，用 `cargo run --release -- polish-textures --map <id> --bump --note "应用通用纹理规则" <源ZIP>`，保留成品高度、斜坡、位图、对象与脚本。历史纹理复现可在配方写 `TEXTURE_PROFILE: "legacy"`。细则见 `docs/terrain_texture_rules.md`。

```sh
cargo run --release -- build --map <id>
cargo run --release -- build --map <id> --bump --note '调整布局' --install
cargo run --release -- accept --map <id>
cargo run --release -- build --map <id> --only map --only verify
```

顺序：terrain → passability → map → verify → spatial → preview → coordinates → accept → readme → dist → wb。`coordinates` 是通用阶段，自动导出离线坐标地图、地形/对象/层界 CSV/JSON 和旧编号对照；会从原生成品补齐旧缓存遗漏的已注册脚本航点。验收以 `src/build.rs` 为准，不过则构建失败。修改地图内容时递增版本；纯工具链重构且地图字节不变时不递增。用户明确要求新版本时按要求递增。

四张现有地图均通过 `COORDINATE_PIPELINE` 使用各自 `authoring/source.zip` 完整原生成品继续制作，避免旧蓝图覆盖定稿高度和斜坡。公共空间策略为 `templates/rules/coordinate-pipeline.json`，每图 `policy` 可局部覆盖；`spatial` 检查占地、近矿、全宽坡道及陆路/水路，跨水陆关系标记为需要运输，不推断距离。来源既有问题逐项留档，新增/恶化问题阻止发布。`adopt-coordinate-source --map <id>` 可启用流水并将当前完整成品及同版布局冻结为新制作起点；新版本核对完成后再次采用，使后续构建直接使用最新版本。详情见 `docs/coordinate_pipeline.md`。

`--bump` 在构建前写新版本，失败则恢复原始 version.json，只清理本次新出现的版本路径。不要删除游戏 Maps 目录里其他地图或已安装旧版本。

未启用冻结成品模式的新图以分层蓝图为高度来源，DEM 和底图仅保留为参考。Rust 从蓝图重新量化高度、挖布局中已定稿的坡、生成纹理和位图；不会重新自动搜索布局。冻结成品模式直接读取原生高度和位图，资源分类清单由对象身份派生。见 `docs/rust_migration.md` 与 `docs/coordinate_pipeline.md`。

WorldBuilder 导出为默认阶段。安装导入文件需要 `--wb --wb-root <目录>` 或 RA3MAP_WB_ROOT。地图安装需要 APPDATA。
