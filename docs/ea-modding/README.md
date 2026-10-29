# EA 官方 RA3 数据（本地镜像）

上游是 EA 公开的 [CnC_Modding_Support](https://github.com/electronicarts/CnC_Modding_Support) 仓库里的 `Red Alert 3/` 目录：官方重建 RA3 基础资源用的 Xml、Schema、地图脚本库和着色器源码。这里是一份本地镜像，用于写地图时核对模板、字段名和官方地图的声明方式，不必联网。

| 项 | 值 |
| --- | --- |
| 上游地址 | `https://github.com/electronicarts/CnC_Modding_Support`，子树 `Red Alert 3/` |
| 上游 revision | `3aad8e412961ab543192ae70f8503dc8e178e095`（2025-05-08） |
| 抓取日期 | 2026-10-30 |
| 体积 | 7721 个文件 / 225.9 MB；`6527` 个 `.xml`、`821` 个 `.xsd`、`206` 个 `.map`、`89` 个 `.fx` |
| 本地位置 | `data/ea-modding/red-alert-3/` |
| 许可 | GPL-3.0 加附加条款；原文另存在本地 `data/ea-modding/upstream/LICENSE.md`（含上游 README） |

**不写入版本库。** `.gitignore` 排除了 `/data/`：镜像从上游一条命令就能重建，没必要进 git。目录形状见 [tree.txt](tree.txt)，逐文件字节数与 sha256 见 `data/ea-modding/manifest.jsonl`，来源与 revision 见 `data/ea-modding/provenance.json`（这两个文件同样只在本地）。

## 重建

```sh
tools/fetch_ea_modding.sh                                                  # 默认写 data/ea-modding
CNC_MODDING_SUPPORT_PROXY=http://127.0.0.1:10808 tools/fetch_ea_modding.sh # git 需要代理时
```

脚本用 `--filter=blob:none --depth 1` 部分克隆加稀疏检出，只下 `Xml`、`Schemas`、`Libraries`、`Shaders` 和 `RA3Music.h`，跳过 `Xml/Maps/**/*.tga`。目标目录默认为 `data/ea-modding`，可以传参换位置；`CNC_MODDING_SUPPORT_URL` / `_REV` 可指到 fork 或指定分支。抓完后脚本重写清单并打印新的 revision，上游有更新时看这里的差异。

## 数据清单

| 顶层 | 文件 | 体积 | 内容 |
| --- | --- | --- | --- |
| `Xml/` | 6741 | 223.1 MB | 官方游戏数据，含 194 张地图的源文件 |
| `Schemas/` | 821 | 1.4 MB | 资产类型的 XSD 定义 |
| `Shaders/` | 129 | 833 KB | 着色器源码 |
| `Libraries/` | 29 | 521 KB | 9 个地图脚本库 |
| `RA3Music.h` | 1 | 62 KB | 音乐脚本用到的枚举头文件 |

### `Xml/` 的子目录

| 目录 | 文件 | 体积 | 内容 |
| --- | --- | --- | --- |
| `Maps/` | 737 | 201.4 MB | 194 张官方地图的 `.map`、`map.xml`、`overrides.xml`，见下节 |
| `Civilian/` | 3456 | 8.3 MB | 平民建筑与道具，按地图分组（`Amsterdam_AM`、`Brighton_Beach_BB` 等），另有公共 `Buildings/` |
| `FXParticleSystems/` | 1387 | 2.3 MB | 粒子系统：`AircraftDamageFire`、`ABConyardWakeSplash`、尾迹、烟雾等 |
| `GlobalData/` | 72 | 1.4 MB | 全局模板，见下 |
| `Soviet/` 等阵营 | 158 / 150 / 134 | 3.9 MB | `Soviet/`、`Allied/`、`Japan/` 的单位、建筑、道具与战役专用单位 |
| `dlcontent/` | 19 | 1.9 MB | 附加下载内容 |
| `Sounds/` | 20 | 1.9 MB | 音效与音乐脚本 |
| `SkirmishAI/` | 65 | 0.7 MB | 遭遇战 AI 人格、策略与参数 |
| `UI/` `AptUI/` `UIInGame/` `Shell/` | 56 / 175 / 19 / 16 | 0.4 MB | 界面布局与控件 |
| `Neutral/` | 78 | 312 KB | 中立单位与建筑（`Structures/` 20 个，含 `OreNode*`、`OilDerrick`） |
| `Includes/` | 80 | 144 KB | 可复用的升级、特效、科技片段 |
| `System/` | 31 | 133 KB | 引擎设置：相机、建造系统、LOD、赠送管理、动态碰撞等 |
| `BaseObjects/` | 48 | 96 KB | 基础对象模板，`BaseOreNode.xml` 等 |
| `Video/` `PathMusic/` | 2 / 6 | 196 KB | 过场视频与路径音乐 |

`Xml/` 下还有 17 个顶层文件：`MapMetaData.xml`（148 KB，196 条地图元数据）、`Static.xml`、`StaticGameObjects.xml`、`global.xml`、`WorldBuilder.xml`、`MapSpecific.xml`、`IntelDB.xml`、`SubsystemSettings.xml`、`Bootup.xml`、`Download.xml`、`Cinematics.xml`、`Audio.xml`、`Locale.xml`、`W3XView.xml` 和各平台的 `*_platformspecific.xml`。`Terrain/` 目录只有一个 `Terrain.xml`（67 KB，406 条 `TerrainAsset`）。

### `GlobalData/`（72 个）

- 战斗数值：`Weapon.xml`、`Armor.xml`、`DamageFX.xml`、`ProjectilePaths.xml`、`AttributeModifier.xml`、`Stances.xml`、`ScorchMarks.xml`
- 移动与地形：`Locomotor.xml`、`Roads.xml`、`WaterTemplates.xml`、`WaterModels.xml`、`Environment.xml`、`Weather.xml`
- 经济与规则：`ObjectCreationLists.xml`、`upgrade.xml`、`ExperienceLevels.xml`、`MultiplayerSettings.xml`、`LogicCommand.xml`、`LogicCommandSet.xml`、`SpecialPowerTemplates.xml`
- 表现与界面：`FXList*.xml`（7 个）、`Font*`（4 个）、`Mouse*.xml`、`UnitTypeIcons.xml`、`RedAlerts.xml`、`UIConfigs.xml`、`LookupTableTextures.xml`
- 其它：`TheVersion.xml`、`RegisteredObjectSets.xml`、`GlobalDefines.xml`、`InvisibilityTemplates.xml`、`TutorialData.xml`、`UnitIntros.xml`

### 阵营对象

| 目录 | 文件 | 单位 | 建筑 | 道具 | 战役专用单位 |
| --- | --- | --- | --- | --- | --- |
| `Allied/` | 150 | 27 | 21 | 84 | 16 |
| `Soviet/` | 158 | 27 | 23 | 99 | 7 |
| `Japan/` | 134 | 22 | 17 | 74 | — |
| `Neutral/` | 78 | 2 | 20 | 38 | — |

每个阵营另有 `<阵营>.xml`（总表）和 `<阵营>MapSpecific.xml`（战役覆盖）。

### `Schemas/`（821）

| 部分 | 文件 | 内容 |
| --- | --- | --- |
| `AssetType*.xsd` | 105 | 资产类型定义：`GameObject`（54 KB）、`WeaponTemplate`（42 KB）、`GameMap`、`MapMetaData`、`TerrainAsset`、`TerrainTextureAtlas`、`WaterSetting`、`PlayerTemplate`、`LocomotorTemplate` 等 |
| `CnC3Types.xsd`、`GameData.xsd` | 2 | 公共类型底板 |
| `Modules/` | 381 | 单位/建筑行为模块：`AIUpdate`、`WeaponSetUpdate`、`OreNodeBehavior`、`SpawnPointProductionExitUpdate`、`ActiveBody` 等 |
| `UI/` | 164 | 界面控件 schema |
| `Includes/` | 62 | 公共片段：`AIKindOf`、`AIBaseType`、`AIUpgradeHeuristic` 等 |
| `System/` | 24 | 引擎设置类型 |
| `InGameUI/` 20、`SkirmishAI/` 16、`UnitAbilityButtons/` 15、`Shell/` 14、`FXParticleSystem/` 6、`PlayerPowerButtons/` 4、`Multiplayer/` 3、`GameObject/` 2、`HotKey/` 2、`Base/` 1 | 83 | 分领域的子 schema |

### `Libraries/` 与 `Shaders/`

9 个脚本库：`Lib_Skirmish_Setup`（遭遇战开局）、`Lib_Victory_Relevance`、`Lib_End_Mission`、`Lib_Power_Restrictions`、`Lib_Camp_Restrictions`、`Lib_Camp_Restrictions_AllPlayers_Everything`、`Lib_Object_Lists`、`Lib_SP_Customization`、`Lib_S03_StingrayKills_vs_JapanUnits`。每个库一份 `<库名>.map` 和 `map.xml`，部分另有 `overrides.xml` 和 `.tga` 预览。

`Shaders/` 是 89 个 `.fx`、19 个 `.fxh`、17 个 `.scrapeh`、4 个 `.scrape`：`Terrain.fx`（57 KB）、`Objects.fxh`（65 KB）、`Ocean.fx`、`RiverWater.fx`、`PostFX_*`、`GPUParticle*` 等，纯渲染侧，做地图一般用不到。

## 做一张地图会用到的数据

按制作阶段列，标出这份数据在本镜像里有没有；没有的就在项目自己的 `maps/`、`templates/` 里。

| 阶段 | 需要的数据 | 在哪 |
| --- | --- | --- |
| 地形 | 真实高程/参考图 → 可玩格高度、通行、水域掩码 | 项目 `maps/<id>/source.png`、`layers.npz`。镜像里没有，地形来自外部资料 |
| 地表材质 | `TerrainAsset` 名单（材质名、UV、混合规则），全图纹理不超过 49 种 | 镜像 `Xml/Terrain/Terrain.xml`（406 条）、`Schemas/AssetTypeTerrainAsset.xsd`、`AssetTypeTerrainTextureAtlas.xsd`；项目生成 `_art.tga` 与 `TerrainTextureAtlas` |
| 地图骨架 | 一张空的原生地图底面和脚本容器 | 项目 `templates/8p_template.map`、`nkl_blizzard_scripts.bin`；官方 `Xml/Maps/official/MAP_MP_*/*.map` 可对照 |
| 地图元数据 | 显示名、玩家数、宽高、出生点、`FileName`、`IsMultiplayer` | 镜像 `Xml/MapMetaData.xml`、`Schemas/AssetTypeMapMetaData.xsd`；项目写进生成的一行 `MapMetaData` |
| 对象摆放 | 对象 ID、`DATA:` 源路径、模块字段（占地、朝向、模型） | 项目 `templates/resources/stock-object-sources.json`（36 条对照）、`stock-global-objects.json`（507 个 ID）；源 XML 就在镜像 `Xml/Neutral/`、`Xml/Civilian/` 等目录，路径一一对应 |
| 资源与中立建筑 | `OreNode*`、`OilDerrick`、医院、机场、观察站 | 镜像 `Xml/Neutral/Structures/`、`Xml/BaseObjects/BaseOreNode.xml`、`Schemas/Modules/OreNodeBehavior.xsd` |
| 道路与水 | 道路网、`WaterAreaType`、水面高度与水面模板 | 镜像 `Xml/GlobalData/Roads.xml`、`WaterTemplates.xml`、`WaterModels.xml`；项目写进 `map.xml` 的 `<Road>`、`<WaterAreaType>` |
| 脚本 | 动作/条件 ID 与参数、航点、队伍与出生 | 项目 `docs/script_functions.md`、`docs/ra3wiki/`；实例看镜像 `Xml/Maps/*/*/map.xml`，字段看 `Schemas/AssetTypeGameScriptList.xsd`，遭遇战整套脚本可参考 `Libraries/Lib_Skirmish_Setup` |
| 遭遇战 AI | AI 人格、军队构成、开局套路 | 镜像 `Xml/SkirmishAI/`（65 个：`ArmyDefAllied` 等、`Personalities/`、`OpeningMoves/`） |
| 本地化 | 地图名、简介、提示文本 | 项目自己生成 `map.str`（`Intro:` / `End:` 段）；镜像没有 `.str`，上游不带本地化文件 |
| 打包 | `.map` + `map.xml` + `overrides.xml` + `map.str` + `_art.tga` + 预览图 | 项目 `src/pipeline/build/`；安装到 `APPDATA/Red Alert 3/Maps/<发布名>/` |

用不上的部分：`Shaders/`（渲染）、`Xml/UI` 系与 `AptUI/`（界面）、`Sounds/`、`Video/`、`PathMusic/`、`dlcontent/`、27 张战役图的专用脚本，做遭遇战地图时都不涉及。

当前生产接入方式见[官方资源索引与最终场景验收](../official_resource_pipeline.md)。镜像用于离线提取，普通构建读取已纳管的小型索引。

## 做地图时先看哪几个

| 想查什么 | 文件 |
| --- | --- |
| 官方地图清单：显示名、玩家数、尺寸、出生点、CRC | `Xml/MapMetaData.xml`（148 KB） |
| 单张官方地图的资产声明，含 `Includes` 写法 | `Xml/Maps/<组>/<地图名>/map.xml` |
| 地图级覆盖声明 | `Xml/Maps/<组>/<地图名>/overrides.xml` |
| 地形与材质模板：406 条 `TerrainAsset` | `Xml/Terrain/Terrain.xml`（67 KB） |
| 全局数据：武器 `Weapon.xml`、护甲 `Armor.xml`、移动 `Locomotor.xml`、道路 `Roads.xml`、水 `WaterTemplates.xml`、水模型 `WaterModels.xml` 等 | `Xml/GlobalData/`（72 个文件） |
| 矿点与中立建筑模板：`OreNode` 系列（`2a`/`2b`/`4a`–`4d` 均继承 `BaseOreNode`）、油井 `OilDerrick`、医院、机场等 | `Xml/Neutral/Structures/`（20 个文件）、`Xml/BaseObjects/BaseOreNode.xml` |
| 阵营单位与建筑 | `Xml/Allied/`、`Xml/Soviet/`、`Xml/Japan/` |
| 中立与平民对象 | `Xml/Neutral/`（78）、`Xml/Civilian/`（3456） |
| 粒子与特效 | `Xml/FXParticleSystems/`（1387） |
| 遭遇战 AI | `Xml/SkirmishAI/`（65） |
| 资产字段定义 | `Schemas/AssetTypeGameObject.xsd`（54 KB）、`AssetTypeWeaponTemplate.xsd`（42 KB）、`AssetTypeGameMap.xsd`、`AssetTypeMapMetaData.xsd`、`AssetTypeTerrainAsset.xsd`、`AssetTypeWaterSetting.xsd` 等 |
| 模块字段（单位行为、采矿、出生） | `Schemas/Modules/OreNodeBehavior.xsd`、`SpawnPointProductionExitUpdate.xsd`、`AIUpdate.xsd`、`WeaponSetUpdate.xsd` 等 |
| WorldBuilder 配置 | `Xml/WorldBuilder.xml` |

## 官方地图源（`Xml/Maps/`）

| 组 | 地图目录 | 构成 |
| --- | --- | --- |
| `official/` | 93 | `CAMP_*` 27 张战役、`MAP_MP_*` 53 张遭遇战、`TUTORIAL_*` 12 张教程、`BalanceTestMap` 1 张 |
| `internal/` | 101 | `TEST_*` 61 张测试图、`ART_*` 21 张美术/过场、`UNIT_INTRO_*` 8 张单位展示、`CAMP_*` 3 张、其余 8 张（`AllUnitsAndStructures`、`Cinetest`、`Roosevelt_test` 等） |
| `obsolete/` | — | 只有一个 `empty.txt` 占位 |

每个目录含 `<地图名>.map`（RefPack 压缩的地图二进制，头 4 字节 `EAR\0`）、`map.xml`、`overrides.xml`；三组合计 196 个 `.map`、535 个 `.xml`，201.4 MB。`Libraries/` 里另有 9 个 `.map`，是脚本库自带的。

`map.xml` 是资产声明：`<Includes>` 列出这张图要挂的参考资产（`DATA:static.xml`、`DATA:global.xml`、`ART:*` 特效等），后面是地图自己的 `GameMap`、出生点、脚本等定义。本项目生成器写出的 `map.xml` / `overrides.xml`（`src/pipeline/build/mod.rs`）与它们是同一套格式，拿官方图对照字段名和 include 顺序最省事。

`.map` 的二进制布局不在本镜像范围内，见 [`.map` 格式规范](../ra3_map_format_spec.md)。

## 检索示例

```sh
rg -n "DisplayName=" data/ea-modding/red-alert-3/Xml/MapMetaData.xml | head
rg -c "<TerrainAsset " data/ea-modding/red-alert-3/Xml/Terrain/Terrain.xml
rg -n 'source="ART:' data/ea-modding/red-alert-3/Xml/Maps/official/CAMP_A01_BrightonBeach_Smith/map.xml | head
rg -l "OreNode" data/ea-modding/red-alert-3/Xml --glob '*.xml' | head
```

XML 是 UTF-8，个别文件带 BOM（如 `Xml/MapMetaData.xml`），按字节读时要先跳过。`MapMetaData.xml` 共 196 个 `<MapMetaData>` 节点，`FileName` 指向游戏内的 `data\maps\...` 路径，和本地目录名不完全对应（本地按 `<组>/<名>/` 排）。

## 没抓的部分

- `Xml/Maps/**/*.tga`：上游 325 个、131.7 MB 的贴图预览，与数据无关。
- 上游其它游戏的目录（`Uprising`、`Tiberian Twilight`、`Kanes Wrath`、`Tiberium Wars`、`Zero Hour`、`Generals`、`Renegade`）不在本次范围内。RA3 起义时刻如果要用，同样改 `tools/fetch_ea_modding.sh` 里的 `sparse_patterns` 再跑。

## 和其它文档的关系

- [`.map` 格式规范](../ra3_map_format_spec.md)：二进制容器、WorldInfo、Waypoints 等，来源是 OpenSAGE，与本镜像互补。
- [WorldBuilder 集成](../ra3_worldbuilder_integration.md)：地编侧的使用方式。
- [离线 RA3Wiki](../ra3wiki/README.md)：编辑器脚本动作、条件和单位数据的说明，来自 wiki，不是 EA 源码；两者冲突时以本镜像的 `.xml` / `.xsd` 为准。
- [脚本功能分类](../script_functions.md)：本项目对脚本函数的归类，对着 `Xml/Maps/*/*/map.xml` 和 `Libraries/` 里的实际用法看更快。
