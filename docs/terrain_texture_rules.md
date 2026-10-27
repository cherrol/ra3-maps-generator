# 通用地形纹理规则

2026-10-27，用户确认保留夹川汇潮 T2.2.9 的纹理，并要求将其接入通用流水。规则名为 `classic-natural`。`work/terrain-material-preview/` 的按山坡、台地和水岸重新选材实验未采用，不导入正式流水。

## 构建与成品更新

- `build`：在 `map` 阶段完成高度、位图、对象、脚本及资源配套后，自动执行已确认的纹理后处理。后续 `verify`、`accept`、`dist` 使用后处理后的成品。
- `polish-textures <源ZIP> --map <id> --bump`：对现有成品执行相同函数，保留已经完成的斜坡优化和 WorldBuilder 定稿内容。
- 配方 `TEXTURE_PROFILE` 缺省或为 `classic-natural` 时自动启用；`legacy` 用于复现历史纹理，其他值拒绝执行。显式 `polish-textures` 始终应用当前通用规则。
- 普通构建仍由各图自己的蓝图和布局生成地形，不能代替成品局部更新。已有成品只改纹理时应使用后一个入口。

## T2.2.9 选材

高度层取当前地图的 `LAYER_M`，缺省使用 `TERRAIN.levels`，不硬编码地图坐标。下表为纹理存在时的首选；缺失时仅在原图图集中选择现有的兼容材质。

| 地表 | 材质与分配 |
|---|---|
| 最低陆地层 | `Grass_NewYork01`，少量 `Grass_CapeCod05` |
| 第二层 | `Grass_MtRush04`，少量 `Grass_Geneva01` |
| 第三层 | `Grass_CapeCod05`，少量 `Dirt_Iceland05` |
| 第四层及更高 | `Grass_CapeCod12`，少量 `Dirt_Iceland05` |
| 层界 | `Rock_Yucatan01`，邻近碎石或土壤衔接 |
| 岸线 | `Sand_Hawaii02`，约 3–5 格宽，随后局部土壤衔接 |
| 近岸到远海 | `Reef_Yucatan02` → `Reef_Yucatan01` → `RA3_ShallowSeaFloor` |
| 城镇 | 紧凑的 `BB_Gravel01` / `Dirt_Yucatan06` 空地，沿用 T2.2.9 的范围规则 |
| 斜坡 | `Dirt_Yucatan06` 车道、较窄的 `Gravel_Yucatan01` 边缘；不把相邻陡坎刷成道路 |

纹理变化使用固定种子 2771、9323，尺度 52、17 格；低两层变体阈值为 0.58，高层裸土阈值为 0.72。这些均与 T2.2.9 一致。层界按相邻陆地至少 35 的高差识别。材质边界逐格写入双纹理混合，沿用原图子贴片相位。

`Grass_CapeCod01/03/07`、`Grass_Golf01/03/05` 不用于大面积草地填充，避免重复裸土图案与斜条纹；原图图集中的登记保留，不改变资源编号。原贴图与真实渲染核对见 [纹理平铺修复](texture_tiling_fix.md)。

## 验证与产物

通用实现位于 `src/texture_polish.rs`，常规构建和成品更新共享它。测试对照普通构建与显式更新结果，并逐字节核对非纹理区块及全部玩法位图。历史迁移测试保留 `legacy`，继续核对原始各区块基线。

`texture-report.json` 记录 `profile`、`reference_version`、纹理直方图和保留检查。`textures--comparison.png` 只使用示意颜色；视觉验收应查看 WorldBuilder 真实渲染及游戏实测。

## 本轮发布

| 地图 | 源版本 | 新版本 |
|---|---|---|
| 叠岭连原 | T0.5.4 | T0.5.5 |
| 岭尽生潮 | T0.4.8 | T0.4.9 |
| 环海相望 | T0.2.8 | T0.2.9 |

三张新图均在各自最新成品上应用相同规则，并已打包、安装。ZIP、assets、dist 与游戏目录全部成品文件逐字节一致；旧 ZIP 保持原哈希。三张图均完成真实 WorldBuilder D3D9 渲染，输出在各自 assets 目录的 `overview--worldbuilder.png`，核对记录为 `render-validation.json`；游戏内复测待完成。36 项 Rust 测试通过。

夹川汇潮 T2.2.9 的版本、发布包及已安装文件未更改。实验预览方案未启用。

上述为纹理通用化发布时的记录。之后四张图已按九州大地命名重新封装并安装，旧版按用户要求归档、清理；当前版本见 [命名迁移](naming_migration.md)。此次改名的 `.map` 二进制与纹理仍逐字节保持源版本。
