# Red Alert 3 (SAGE) `.map` 二进制格式规范

> 本文保留早期研究中的文件名、路径、原始格式数据和源码字段，仅供格式研究，不构成旧地图、旧布局或旧规则的执行入口。当前仅保留河南可玩成品，蓝图以实际配方的 `LAYERS` 为准，版本与根镜像见[蓝图与版本资料](blueprint_versions.md)。所有可玩生产统一执行当前两矿和严格零问题规则；原生格式解析能力不提供历史豁免。

> 来源：OpenSAGE 开源项目（https://github.com/OpenSAGE/OpenSAGE，master 分支）
> 主要参考目录：`src/OpenSage.Game/Data/Map/`、`src/OpenSage.Game/Logic/Map/`、`src/OpenSage.Game/Scripting/`、`src/OpenSage.FileFormats/BinaryReaderExtensions.cs`
> 本地对照数据：196 张官方 `.map` 源文件及其 `map.xml` 在 [EA 官方 RA3 数据镜像](../ea-modding/README.md) 的 `data/ea-modding/red-alert-3/Xml/Maps/`，可以用来核对字节；模板与字段定义看同镜像的 `Schemas/`。
> 适用范围：`SageGame.Ra3` 与 `SageGame.Ra3Uprising`。文中所有未标注"未能确认"的内容均直接来自 OpenSAGE 解析代码。
>
> **字节序：除非特别说明，全部为小端（little-endian）。无任何对齐/填充。**

---

## 1. 容器层（压缩外壳）

文件最前 4 字节是压缩标志 FourCC：

| 字节 | 含义 | 后续布局 |
|---|---|---|
| `45 41 52 00` `"EAR\0"` | **RefPack 压缩（RA3 地图的实际形态）** | `uint32 decompressedSize`，之后是 RefPack 压缩流 |
| `43 6B 4D 70` `"CkMp"` | 未压缩 | 回退 4 字节，整个文件即"解压后格式" |
| `5A 4C 35 00` `"ZL5\0"` | zlib 压缩（仅发现于一张 Generals 地图） | `uint32 decompressedSize` + zlib 流（头 `78 9C`） |

**要点：**
- RA3 的 `.map` 在磁盘上几乎总是 `"EAR\0"` 开头。你能在原始字节里看到 `WaypointsList` 等 ASCII 字符串，是因为 RefPack 压缩流包含"字面量直拷贝段"（literal runs），短字符串经常原样出现——这不代表文件未压缩。
- 解压后的字节流与未压缩格式**完全相同**，且以 `"CkMp"` magic 开头。
- RefPack 解压算法实现参考：`src/OpenSage.FileFormats/RefPack/RefPackStream.cs`（EA RefPack，又名 DBPF 压缩）。Python 侧可用现成的 refpack 实现。
- OpenSAGE 写地图时总是输出未压缩（`CkMp`）格式。

## 2. 解压后文件布局总览

```
offset 0   char[4]   "CkMp" magic
offset 4   字符串表（AssetNameCollection，见 §3）
...        chunk 序列（见 §4），直到 EOF
```

没有文件级版本号、没有 chunk 目录/偏移表——chunk 严格顺序排列、顺序解析，靠每个 chunk 自带的 `dataSize` 定位下一个。

## 3. 字符串表（AssetNameCollection）

> ⚠️ **对已知线索的修正**：条目顺序是 **字符串在前、uint32 id 在后**（不是 id 在前）；长度前缀是 .NET 的 7-bit varint（<128 字节时就是 1 个 uint8），编码 ASCII。条目按 id **降序**存储。

```
uint32   numEntries
重复 numEntries 次（第 i 个条目的 id 必须等于 numEntries, numEntries-1, ..., 1）：
    str      assetName    // 7-bit-varint 长度前缀 + ASCII 字节（.NET BinaryReader.ReadString）
    uint32   index        // 1-based，文件中降序出现
```

- id 从 1 开始（没有 0）。
- 表中包含所有 chunk 名（`HeightMapData`、`ObjectsList`…）、嵌套资产名（`Object`、`ScriptList`…）**以及全部属性键名**（`playerName`、`waypointID`…，见 §8）。
- 源文件：`Data/Map/AssetNameCollection.cs`。

## 4. Chunk 头（Asset）

每个 chunk（OpenSAGE 称 asset）：

| 偏移 | 类型 | 含义 |
|---|---|---|
| +0 | uint32 | nameIndex：字符串表 1-based 索引 |
| +4 | uint16 | **version**（本 chunk 格式版本，按 version 条件解析字段） |
| +6 | uint32 | dataSize：payload 字节数 |
| +10 | byte[dataSize] | payload |

- 顶层 chunk 序列一直读到 EOF；容器型 chunk（`ObjectsList`、`SidesList`(v<5)、`PlayerScriptsList`、`ScriptList`、`Script`、`MPPositionList`）内部同样是"uint32 nameIndex + chunk 头"的子 chunk 序列，读到本 chunk payload 末尾为止。
- 解析器可用 dataSize 跳过不认识的 chunk。OpenSAGE 对未知 chunk 名会抛异常，但实际解析器应容错跳过。
- 源文件：`Data/Map/Asset.cs`。

### 4.1 RA3 顶层 chunk 典型顺序

来自 `MapFile.WriteTo`（解析时实际接受任意顺序；括注 = RA3 中一般**不**出现，属于更老游戏）：

1. `AssetList` → 2. `GlobalVersion` → 3. `HeightMapData` → 4. `BlendTileData` → 5. `WorldInfo` → 6. (`MPPositionList`) → 7. `SidesList` → 8. (`LibraryMapLists`) → 9. `Teams` → 10. `PlayerScriptsList` → 11. `BuildLists` → 12. `ObjectsList` → 13. (`PolygonTriggers`，仅 Generals/ZH/BFME1) → 14. `TriggerAreas` → 15. `GlobalWaterSettings` → 16. `FogSettings` → 17. `MissionHotSpots` → 18. `MissionObjectives` → 19. `StandingWaterAreas` → 20. `RiverAreas` → 21. `StandingWaveAreas` → 22. `GlobalLighting` → 23. `PostEffectsChunk` → 24. `EnvironmentData` → 25. `NamedCameras` → 26. `CameraAnimationList` → 27. (`CastleTemplates`，BFME) → 28. `WaypointsList` → 29. (`SkyboxSettings`，BFME)

---

## 5. 基础类型编码约定

来源：`src/OpenSage.FileFormats/BinaryReaderExtensions.cs`、`Data/Map/MapColorArgb.cs`。

| 名称 | 编码 |
|---|---|
| `uint16-prefixed ASCII` | uint16 长度 N + N 字节 ASCII（Windows-1252/ANSI 语义，`.map` 内均为 ASCII） |
| `uint16-prefixed Unicode` | uint16 字符数 N + 2N 字节 UTF-16LE |
| `bool`（checked） | 1 字节，合法值只有 0x00/0x01 |
| `bool32`（`ReadBooleanUInt32Checked`） | 1 字节 0/1 + 3 字节必须全 0 |
| `uint24` | 3 字节小端 |
| `Vector2/3/4` | 2/3/4 个 float32 |
| `Quaternion` | 4 个 float32 (x,y,z,w) |
| `ColorRgb` | 3 字节 R,G,B |
| `ColorRgbF` | 3 个 float32 R,G,B |
| `MapColorArgb` | uint32，位布局 `A<<24|R<<16|G<<8|B`（即字节序 B,G,R,A） |
| 7-bit varint 字符串 | .NET `BinaryReader.ReadString`：7-bit varint 长度 + ASCII 字节（仅用于字符串表） |
| 位压缩 bool 数组 `ReadSingleBitBooleanArray2D(w,h)` | 按行（y 外层、x 内层）扫描；每行 x%8==0 时读 1 字节，取 `bit (x%8)`（LSB 优先）。**行与行之间按字节重新对齐**，行尾不足 8 的位丢弃 |

二维数组一律 **y（行）外层、x（列）内层** 顺序存储。

---

## 6. HeightMapData

源：`Data/Map/HeightMapData.cs`。RA3 推定 version = 6（v≥5 起高度为 16 位——BFME/CnC3/RA3 均如此；v≥6 起 border 含 Corner1，标注 `[AddedIn(SageGame.Cnc3)]`）。**具体版本号以文件内 chunk 头为准，未能实测确认。**

| 字段 | 类型 | 条件 |
|---|---|---|
| Width | uint32 | |
| Height | uint32 | |
| BorderWidth | uint32 | |
| BorderCount | uint32 | |
| Borders[BorderCount] | 见下 | |
| Area | uint32 | 必须 == Width*Height |
| Elevations | 逐格高度，y 外层 x 内层 | v≥5: 每格 uint16；v<5: 每格 uint8 |

Border 条目：

| 字段 | 类型 | 条件 |
|---|---|---|
| Corner1X | uint32 | 仅 v≥6 |
| Corner1Y | uint32 | 仅 v≥6 |
| X (Corner2X) | uint32 | |
| Y (Corner2Y) | uint32 | |

**高度换算：** `worldZ = raw * scale`；v≥5: scale = 0.0390625 (=10/256，最大 65535→2559.98)；v<5: scale = 0.625。MinZ/MaxZ 由数据计算，不存储。

**尺寸与边框关系：** Width/Height 是含边框的完整高度图尺寸；可玩区域 = 去掉四边各 BorderWidth 格，即 `(Width-2*BorderWidth) × (Height-2*BorderWidth)`（SAGE 惯例；OpenSAGE 用 `Borders[0]` 的 Corner1/Corner2 作为 MapCache 的 ExtentMin/Max）。每格对应 10×10 世界单位（SAGE 标准网格，未能从 OpenSAGE 该文件直接确认，属引擎常量）。

## 7. BlendTileData

源：`Data/Map/BlendTileData.cs`。要求 version ≥ 6。**RA3 推定 version = 25**（v≥25 才有 `DynamicShrubberyDensity`，标注 `[AddedIn(SageGame.Ra3)]`；未能实测确认）。宽高取自**前一个** `HeightMapData` chunk（W×H）。

| 顺序 | 字段 | 类型 | 条件 |
|---|---|---|---|
| 1 | NumTiles | uint32 | == W*H |
| 2 | Tiles | uint16[W×H] | 每格 uint16，指向全局 tile 编号（见 §7.1） |
| 3 | Blends | uint16/uint32 [W×H] | 位宽 = (14≤v<24) ? 32 : **16** |
| 4 | ThreeWayBlends | 同上 | |
| 5 | CliffTextures | 同上 | |
| 6 | Impassability | bit 数组 | v>6。v==7 时行宽按 `((W+1)/8)*8` 对齐裁剪（仅 Generals）；其余版本行宽=W。0=可通行，1=不可通行 |
| 7 | ImpassabilityToPlayers | bit 数组 | v≥10 |
| 8 | PassageWidths | bit 数组 | v≥11 |
| 9 | Taintability | bit 数组 | 14≤v<25（RA3 v25 **无**此字段） |
| 10 | ExtraPassability | bit 数组 | v≥15 |
| 11 | Flammability | byte[W×H] 枚举 | 16≤v<25（0=FireResistant,1=Grass,2=HighlyFlammable,3=Undefined） |
| 12 | Visibility | bit 数组 | v≥17 |
| 13 | Buildability | bit 数组 | v≥24（代码内注 "TODO: 顺序待确认"，与下两行一起） |
| 14 | ImpassabilityToAirUnits | bit 数组 | v≥24 |
| 15 | TiberiumGrowability | bit 数组 | v≥24 |
| 16 | DynamicShrubberyDensity | byte[W×H] | **v≥25（RA3）** |
| 17 | TextureCellCount | uint32 | |
| 18 | BlendsCount | uint32 | **存储值 = 实际 BlendDescription 数 + 1**（若 >0 则减 1；可能存 0） |
| 19 | CliffMappingsCount | uint32 | 同样 +1 规则 |
| 20 | TextureCount | uint32 | |
| 21 | Textures[TextureCount] | BlendTileTexture | 见 §7.2 |
| 22 | MagicValue1 | uint32 | 含义未知，原样保留 |
| 23 | MagicValue2 | uint32 | 必须为 0 |
| 24 | BlendDescriptions[BlendsCount] | 见 §7.3 | |
| 25 | CliffTextureMappings[CliffMappingsCount] | 见 §7.4 | |

### 7.1 Tiles 单元格 → 纹理换算
`Tiles` 的值是**全局 cell 编号**。每个纹理占 `(CellSize*2)²` 个 cell（见 §7.2 注意），按 Textures 数组顺序累加 CellCount 求所属纹理与偏移。32×32 block 在纹理内的排布顺序见 `BlendTileTextureIndex.cs` 注释（特殊的 2×2 分组顺序）。

### 7.2 BlendTileTexture
| 字段 | 类型 | 备注 |
|---|---|---|
| CellStart | uint32 | |
| CellCount | uint32 | 必须 == CellSize² |
| CellSize | uint32 | 纹理边长（64×64 cell 为单位） |
| MagicValue | uint32 | 必须为 0 |
| Name | uint16-prefixed ASCII | 纹理名 |

（注意：代码中"占格数"用 `(CellSize*2)²` 计算，与校验用的 `CellSize²` 不同，写解析器时以代码为准——见 `BlendTileData.CalculateTextureIndices`。）

### 7.3 BlendDescription
| 字段 | 类型 | 备注 |
|---|---|---|
| SecondaryTextureTile | uint32 | |
| BlendDirection | byte[4] | 每字节 0/1 作为位标志：bit0=向右、bit1=向上、bit2=向右上、bit3=向左上（RA3 地图中出现过 221=0xDD 的异常字节，OpenSAGE 原样保留，**不要校验**） |
| Flags | byte | 1=Flipped, 2=AlsoHasBottomLeftOrTopRightBlend（同样可能出现 0xDD） |
| TwoSided | byte(bool) | |
| MagicValue1 | uint32 | 0xFFFFFFFF 或 24（OpenSAGE 不强制校验） |
| MagicValue2 | uint32 | 必须 == 0x7ADA0000 |

### 7.4 CliffTextureMapping
| 字段 | 类型 |
|---|---|
| TextureTile | uint32 |
| BottomLeft / BottomRight / TopRight / TopLeft | 各 Vector2（8 float32 共 32 字节） |
| Unknown2 | uint16 |

## 8. 属性字典（AssetPropertyCollection）—— 通用 KV 结构

`WorldInfo`、`SidesList` 的每个 Player、`Teams` 的每个 Team、`Object` 的属性都使用同一结构。源：`AssetPropertyCollection.cs` / `AssetProperty.cs` / `AssetPropertyKey.cs` / `AssetPropertyType.cs`。

```
uint16  numProperties
重复：
    uint8   propertyType    // 0=Boolean 1=Integer 2=RealNumber 3=AsciiString 4=UnicodeString 5=Unknown(CnC3+)
    uint24  nameIndex       // 字符串表索引（属性名也进字符串表！）
    value                  // 按 type：
                           //   0: bool(1字节)  1: int32  2: float32
                           //   3/5: uint16-prefixed ASCII   4: uint16-prefixed Unicode(UTF-16LE)
```

## 9. ObjectsList 与 Object（含 waypoint 表示）

源：`ObjectsList.cs` / `MapObject.cs` / `Scripting/Waypoints.cs`。

`ObjectsList` payload = 连续的子 chunk，每个子 chunk 名为 `"Object"`（带标准 chunk 头：nameIndex+version+size）。`Object` payload：

| 字段 | 类型 |
|---|---|
| Position | Vector3（float32 ×3） |
| Angle | float32（弧度） |
| RoadType | uint32 flags（None=0, Start=2, End=4, Angled=8, BridgeStart=16, BridgeEnd=32, TightCurve=64, EndCap=128, +256 BFME2, +512 CnC4；PrimaryType = Start|End|BridgeStart|BridgeEnd） |
| TypeName | uint16-prefixed ASCII |
| Properties | AssetPropertyCollection（§8） |

**Waypoint 就是普通 Object**：`TypeName == "*Waypoints/Waypoint"`，属性键：

| 键 | 类型 | 含义 |
|---|---|---|
| `waypointID` | Integer | 路径连接用 ID（0x7FFFFFFF 为无效值） |
| `waypointName` | AsciiString | 如 `Player_1_Start`…`Player_8_Start`、`InitialCameraPosition` |
| `waypointPathLabel1/2/3` | AsciiString | 路径标签（若存在 1 则三个都在，可为空串） |

**出生点规则**（来自 `UserMapCache.cs`）：`Player_N_Start`（N=1..8）必须**连续编号**才被识别（有 1、2、4 号则按 2 人图算）；`InitialCameraPosition` 为初始镜头。地图玩家人数由此推导，而非 SidesList。

## 10. WaypointsList

源：`WaypointsList.cs` / `WaypointPath.cs`。存 waypoint 之间的**路径连线**（不含点本身，点在 ObjectsList 里）：

```
uint32 numPaths
重复: int32 startWaypointID, int32 endWaypointID
```

## 11. SidesList（玩家列表）

源：`Logic/Map/SidesList.cs` + `Data/Map/Player.cs`。**RA3 推定 version = 6**（v≥5 起 teams/scripts 移到独立顶层 chunk；v≥6 多 1 个 bool。未能实测确认）。

| 字段 | 类型 | 条件 |
|---|---|---|
| Unknown | bool(1B) | v≥6 |
| NumPlayers | int32 | |
| Players[NumPlayers] | 见下 | |
| （v<5 还有 Teams 数组 + 内嵌 PlayerScriptsList 子 chunk——RA3 不走此分支） | | v<5 |

Player 条目：

| 字段 | 类型 |
|---|---|
| Properties | AssetPropertyCollection（§8），键见下表 |
| NumBuildListItems | uint32 |
| BuildListItems | BuildListInfo 数组（见 §12.1；CnC3+/RA3 即"地图含 AssetList 且 SidesList v≥6"时每条多 1 个 bool） |

Player 属性键（来自 `Player.cs`，类型即存储类型）：

| 键 | 类型 | 含义 |
|---|---|---|
| `playerName` | Ascii | 内部名；空串=Neutral |
| `playerIsHuman` | Bool | |
| `playerIsSkirmish` | Bool | |
| `playerFaction` | Ascii | 如 `FactionAllies` |
| `playerDisplayName` | **Unicode** | 显示名 |
| `playerEnemies` / `playerAllies` | Ascii | 空格分隔的玩家名列表 |
| `playerStartMoney` | Int | 可选 |
| `playerColor` / `playerNightColor` | Int | 可选 |
| `multiplayerStartIndex` | Int | 对应 `Player_*_Start` waypoint |
| `skirmishDifficulty` | Int | 可选 |
| `multiplayerIsLocal` | Bool | 可选 |
| `playerIsPreorder` | Bool | 可选 |

（并非固定 8 人定义——NumPlayers 可变；8 人上限来自 waypoint 命名约定。）

## 12. Teams / Team

源：`Teams.cs` / `Team.cs`（RA3：`[AddedIn(SageGame.Bfme)]` 起为独立顶层 chunk）。

```
int32 numTeams
重复: Team = AssetPropertyCollection（§8）
```

Team 属性键（全部来自 `Team.cs`，均可选；前缀均为 `team`）：`teamName`(Ascii), `teamOwner`(Ascii), `teamIsSingleton`(Bool), `teamHome`(Ascii, waypoint 名), `teamUnitType1..7`(Ascii), `teamUnitMinCount1..7`(Int), `teamUnitMaxCount1..7`(Int), `teamOnCreateScript`, `teamOnIdleScript`, `teamInitialIdleFrames`(Int), `teamOnUnitDestroyedScript`, `teamOnDestroyedScript`, `teamDestroyedThreshold`(Real), `teamEnemySightedScript`, `teamAllClearScript`, `teamAutoReinforce`(Bool), `teamIsAIRecruitable`(Bool), `teamIsBaseDefense`(Bool), `teamIsPerimeterDefense`(Bool), `teamAggressiveness`(Int, -2..2), `teamTransportsReturn`(Bool), `teamAvoidThreats`(Bool), `teamAttackCommonTarget`(Bool), `teamMaxInstances`(Int), `teamDescription`(Ascii), `teamProductionCondition`(Ascii), `teamProductionPriority`(Int), `teamProductionPrioritySuccessIncrease`(Int), `teamProductionPriorityFailureDecrease`(Int), `teamTransport`(Ascii), `teamReinforcementOrigin`(Ascii), `teamStartsFull`(Bool), `teamTransportsExit`(Bool), `teamVeterancy`(Int), `teamExecutesActionsOnCreate`(Bool), `teamGenericScriptHook`(Ascii)。

### 12.1 BuildListInfo（SidesList 玩家与 BuildLists 共用）

源：`Logic/Map/BuildListInfo.cs`。

| 字段 | 类型 | 条件 |
|---|---|---|
| BuildingName | uint16-prefixed ASCII | |
| TemplateName | uint16-prefixed ASCII | |
| Location | Vector3 | |
| Angle | float32 | |
| IsInitiallyBuilt | bool(1B) | |
| Unknown1 | bool(1B) | **仅当（地图含 AssetList）——即 CnC3/RA3** |
| NumRebuilds | uint32 | 0xFFFFFFFF=无限 |
| Script | uint16-prefixed ASCII | |
| Health | int32 | |
| Whiner | bool(1B) | |
| Unsellable | bool(1B) | |
| Repairable | bool(1B) | |

`BuildLists` 顶层 chunk（`BuildLists.cs`/`BuildList.cs`）：子结构为若干 BuildList：若地图含 AssetList（RA3 属于此列）则开头是 `uint16-prefixed ASCII FactionName`，否则是 AssetPropertyKey；然后 `uint32 count` + BuildListInfo 数组。

## 13. PlayerScriptsList / ScriptList / Script（脚本顶层结构）

源：`Data/Map/PlayerScriptsList.cs`、`Scripting/ScriptList.cs`、`Scripting/Script.cs`。

- `PlayerScriptsList` payload = 连续子 chunk，每个名为 `"ScriptList"`。
- `ScriptList`（version 必须为 1）payload = 连续子 chunk，名为 `"Script"` 或 `"ScriptGroup"`（组内递归）。
- `Script` payload 头字段（**RA3 = version 5；Ra3Uprising = version 6**，由 `[AddedIn]` 标注与代码条件推得）：

| 字段 | 类型 | 条件 |
|---|---|---|
| Name | uint16-prefixed ASCII | |
| Comment / ConditionsComment / ActionsComment | uint16-prefixed ASCII ×3 | |
| IsActive, DeactivateUponSuccess, ActiveInEasy/Medium/Hard, IsSubroutine | bool(1B) ×6 | |
| EvaluationInterval | uint32 | v≥2 |
| UsesEvaluationIntervalType | bool(1B) | **v==5（CnC3/RA3）** |
| EvaluationIntervalType | uint32 枚举（0=Operation,1=MoveForces,2=Battle,3=Upkeep,4=Complete,5=Any,6=FrameOrSeconds） | v==5 |
| ActionsFireSequentially, LoopActions | bool(1B) ×2 | v≥3 |
| LoopCount | int32 | v≥3 |
| SequentialTargetType | byte（0=Team,1=Unit） | v≥3 |
| SequentialTargetName | uint16-prefixed ASCII | v≥3 |
| Unknown | uint16-prefixed ASCII（"ALL"/"Planning"/"X"） | v≥4 |
| Unknown2 | int32 | **v≥6（Ra3Uprising）** |
| Unknown3 | uint16（必须 0） | v≥6 |

之后是子 chunk 序列：`"OrCondition"`（ScriptOrCondition）、`"ScriptAction"`（真分支）、`"ScriptActionFalse"`（假分支）。每个 ScriptOrCondition 内含 `"ScriptCondition"` 子 chunk；条件/动作内部为 version + 参数列表（本规范不展开，参数为 uint8 类型 + 按类型编码的值，详见 `Scripting/ScriptAction.cs`、`ScriptCondition.cs`、`ScriptContent.cs`——**本次未逐一获取，标注"未能确认细节"**）。

脚本与玩家的归属：Generals 时代按索引挂到 SidesList 玩家；RA3 顶层 `PlayerScriptsList` 中各 `ScriptList` 与玩家的对应关系 OpenSAGE 未明确处理（代码中直接取 `PlayerScriptsList.ScriptLists`），**归属规则未能确认**。

## 14. 其余 chunk 速查

### 14.1 `AssetList`（CnC3+，RA3 必有）
`uint32 count` + 每条 `{ uint32 TypeId, uint32 InstanceId }`。

### 14.2 `GlobalVersion`（RA3 新增）
payload **为空**（0 字节），仅有 chunk 头里的 version 号有意义。

### 14.3 `WorldInfo`
payload = 单个 AssetPropertyCollection（§8）。OpenSAGE 未枚举键名（常见键如 `mapWeather`、`mapDescription` 等——**具体键集合未能确认**）。

### 14.4 `GlobalLighting`
源：`GlobalLighting.cs` / `GlobalLightingConfiguration.cs` / `GlobalLight.cs`。

| 字段 | 类型 | 条件 |
|---|---|---|
| Time | uint32（1=Morning,2=Afternoon,3=Evening,4=Night） | |
| LightingConfigurations[4] | 按 Morning→Night 顺序 4 组 | 每组见下 |
| ShadowColor | MapColorArgb (uint32) | |
| Unknown | byte[44]（v7-8）或 byte[4]（v9-10） | 7≤v<11 |
| Unknown2 | Vector3 | v≥12（CnC4） |
| Unknown3 | MapColorArgb | v≥12 |
| NoCloudFactor | ColorRgbF | v≥8 |

每组 GlobalLightingConfiguration（v≥10 起只剩 Terrain 三个光源——RA3 光源结构依版本而定，**RA3 具体版本未能确认**）：
`TerrainSun, [ObjectSun, (InfantrySun)], TerrainAccent1, [ObjectAccent1, (InfantryAccent1)], TerrainAccent2, [ObjectAccent2, (InfantryAccent2)]`——方括号部分仅 v<10，圆括号部分仅 7≤v<10。每个 GlobalLight = 3×Vector3（Ambient, Color, Direction），36 字节。

### 14.5 `PostEffectsChunk`（BFME2+）
`uint32 count`（v≥2；v1 为 byte）+ PostEffect 数组。PostEffect（v≥2，CnC3/RA3 形态）：`Name`(u16-ASCII) + `uint32 numParams` + 参数数组；参数 = `Name`(u16-ASCII) + `Type`(u16-ASCII) + 值（Type ∈ "Float"→float32, "Float4"→Vector4, "Int"→int32, "Texture"→u16-ASCII）。v1（BFME2）：`Name` + float BlendFactor + u16-ASCII LookupImage。

### 14.6 `EnvironmentData`
| 字段 | 类型 | 条件 |
|---|---|---|
| WaterMaxAlphaDepth, DeepWaterAlpha | float32 ×2 | v≥3 |
| IsMacroTextureStretched | bool(1B) | v<5 |
| MacroTexture, CloudTexture | u16-ASCII ×2 | |
| UnknownTexture | u16-ASCII | v≥4（CnC3+） |
| UnknownTexture2 | u16-ASCII | **v≥6 且未到 payload 末尾（Ra3Uprising；CnC4 也用 v6 但无此字段，故按 endPosition 判断）** |

代码注释明确："RA3 Uprising 和 C&C4 用 v6" → **RA3 本体 ≤ v5（推定 v5），Uprising = v6**。

### 14.7 `NamedCameras`
`uint32 count` + 每条：`LookAtPoint`(Vector3), `Name`(u16-ASCII), `Pitch,Roll,Yaw,Zoom,FieldOfView,Unknown`(float32 ×6)。

### 14.8 `CameraAnimationList`
`uint32 count` + CameraAnimation：`AnimationType`(4 字节 FourCC，**大端读取**为 "free"/"look"，即文件字节序为 'e','e','r','f' 或 'k','o','o','l'), `Name`(u16-ASCII), `NumFrames`(u32), `StartOffset`(u32)，然后：
- free：`uint32 n` + 帧数组：`FrameIndex`(u32), `InterpolationType`(FourCC 大端 "catm"/"line"), `Position`(Vector3), `Rotation`(Quaternion), `FieldOfView`(f32)
- look：两段数组（camera 帧：`Position,Roll,FieldOfView`；lookAt 帧：`LookAt` Vector3），各带 uint32 计数。

### 14.9 `GlobalWaterSettings`（CnC3+）
`bool32 ReflectionOn`（1 字节 0/1 + 3 字节 0）, `float32 ReflectionPlaneZ`。共 8 字节。

### 14.10 `FogSettings`（CnC3+）
固定 **24 字节，含义未知**（OpenSAGE 标注 TODO，原样保留）。

### 14.11 `TriggerAreas`（BFME2+，RA3 用它替代 PolygonTriggers）
`uint32 count` + TriggerArea：`Name`(u16-ASCII), `LayerName`(u16-ASCII), `ID`(u32), `uint32 numPoints` + Vector2 数组, `uint32 0`。

### 14.12 `StandingWaterAreas`
`uint32 count` + StandingWaterArea：`UniqueID`(u32), `Name`,`LayerName`(u16-ASCII), `UvScrollSpeed`(f32), `UseAdditiveBlending`(bool 1B), `BumpMapTexture`,`SkyTexture`(u16-ASCII), `uint32 numPoints` + Vector2 数组, `WaterHeight`(u32), `FxShader`,`DepthColors`(u16-ASCII)。

### 14.13 `RiverAreas`
`uint32 count` + RiverArea：`UniqueID`(u32), `Name`,`LayerName`(u16-ASCII), `UvScrollSpeed`(f32), `UseAdditiveBlending`(bool), `RiverTexture`,`NoiseTexture`,`AlphaEdgeTexture`,`SparkleTexture`(u16-ASCII ×4), `Color`(RGB 3B), `byte 0`, `Alpha`(f32), `WaterHeight`(u32), `RiverType`(u16-ASCII, **v≥3/CnC3+**), `MinimumWaterLod`(u16-ASCII), `uint32 numLines` + Line2D 数组（每条 = 2×Vector2，16 字节）。

### 14.14 `StandingWaveAreas`
`uint32 count` + StandingWaveArea：`UniqueID`(u32), `Name`,`LayerName`(u16-ASCII), `UvScrollSpeed`(f32), `UseAdditiveBlending`(bool), `uint32 numPoints` + Vector2 数组, `uint32 0`；v<3 另有 9×u32 + Texture + (v==2: bool32 EnablePcaWave)；**v≥4（RA3 新增）：`WaveParticleFXName`(u16-ASCII)**。

### 14.15 `MissionHotSpots`（RA3 新增）
`uint32 count` + 每条 3× u16-ASCII（ID, Title, Description）。

### 14.16 `MissionObjectives`（RA3 新增）
`uint32 count` + 每条：`ID`,`Text`,`Description`(u16-ASCII ×3), `IsBonusObjective`(bool 1B), `ObjectiveType`(u32：0=Attack,1/2=Unknown,3=Build,4=Capture,5=Move,6=Protect)。

### 14.17 `MPPositionList`（BFME 系；RA3 一般不出现）
子 chunk `"MPPositionInfo"` 序列；每条：`IsHuman`,`IsComputer`(bool), v>0: `LoadAIScript`(bool), `Team`(u32), v>0: `uint32 n` + u16-ASCII SideRestrictions。

### 14.18 `SkyboxSettings`（BFME；RA3 一般不出现）
`Position`(Vector3), `Scale`(f32), `Rotation`(f32), `TextureScheme`(u16-ASCII)。

### 14.19 `PolygonTriggers`
仅 Generals/ZH/BFME1（`MapFile.cs` 注释明确：BFME2 及以后改用 TriggerAreas/StandingWaterAreas/RiverAreas/StandingWaveAreas）。RA3 不出现，本规范不展开。

## 15. 地图显示名存在哪里？

**不在 `.map` 文件内。** OpenSAGE 的 `UserMapCache.cs` 中 `DisplayName`/`Description` 为未实现的 TODO，说明 .map 二进制里没有可读的地图显示名字段。RA3 自定义地图的显示名/描述来自地图目录下的外部本地化文件（`.str`，如 `<地图名>.str`）——此点属 RA3 通行做法，**OpenSAGE 代码中未能直接确认 .str 的键名细节**。`.map` 内最接近的是 `WorldInfo` 属性字典与 `MissionHotSpots`/`MissionObjectives` 的 Text 字段（均为本地化标签引用而非最终文本）。

## 16. RA3 版本差异速查表（CnC3 → RA3 → Ra3Uprising）

| 位置 | CnC3 | RA3 | Ra3Uprising | 依据 |
|---|---|---|---|---|
| AssetList chunk | ✔ 新增 | ✔ | ✔ | `[AddedIn(Cnc3)]` |
| GlobalVersion chunk | — | ✔ 新增 | ✔ | `[AddedIn(Ra3)]` |
| MissionHotSpots / MissionObjectives | — | ✔ 新增 | ✔ | `[AddedIn(Ra3)]` |
| BlendTileData v≥25 DynamicShrubberyDensity | — | ✔ | ✔ | `[AddedIn(Ra3)]`，推定 RA3=v25 |
| BlendTileData Taintability/Flammability | v<25 有 | v≥25 无 | 无 | 代码条件 |
| StandingWaveArea WaveParticleFXName (v≥4) | — | ✔ | ✔ | `[AddedIn(Ra3)]` |
| EnvironmentData | v4/5 | v5（推定） | **v6**（多一个纹理字符串） | 代码注释明示 |
| Script chunk | v5 | v5 | **v6**（多 int32+uint16） | `[AddedIn(Ra3Uprising)]` |
| AssetPropertyType 5 (Unknown) | ✔ 新增 | ✔ | ✔ | `[AddedIn(Cnc3)]` |
| HeightMapData 16 位高度 + v6 border | ✔ | ✔ | ✔ | 代码注释/标注 |

> ⚠️ 所有"推定"的 RA3 chunk 版本号未能用真实 RA3 .map 实测确认；解析器**必须**按 chunk 头中的 version 字段做条件解析，不要硬编码版本号。

## 17. 编写解析器的注意点（易踩坑）

1. 先处理 RefPack（`EAR\0`）解压，再按未压缩格式解析；解压后首 4 字节必须是 `CkMp`。
2. 字符串表条目是 **string→index** 顺序、**index 降序**；长度是 7-bit varint。
3. 属性键的 nameIndex 是 **uint24**（3 字节），不是 uint32。
4. `BlendsCount`/`CliffMappingsCount` 存储值 = 实际数 +1（>0 时）。
5. bit 数组按行对齐：每行从**新字节**开始，行尾补位丢弃；bit 顺序 LSB-first。
6. BlendTileData 中三类 blend 数组在 RA3（v≥24）是 **uint16** 而非 uint32。
7. CameraAnimation 的 FourCC 是**大端**读取（`bigEndian: true`）。
8. BlendDescription 里 RA3 存在 0xDD 填充字节，OpenSAGE 原样透传——解析时不要断言校验。
9. 一切数组都是 y 外层、x 内层。
10. 容器 chunk 靠 payload 末尾（start+dataSize）判断子 chunk 序列结束，环境数据等用"是否还有剩余字节"区分 RA3 与 Uprising。

## 18. 引擎硬限制（实测/社区地图归纳，超出会崩 `Drawable/Object position too large`）

> 实测依据：本机 465 张社区地图扫描，没有一张可玩区超过 1000 格。T0.1.1 把可玩区做到
> 1080 格（10800 世界单位）后游戏直接崩溃。

- **可玩区上限：1000 × 1000 格 = 10000 × 10000 世界单位**（每个高度图格 10 世界单位）。
- 高度图格数本身可以更大（实测有 1160×1160 高度图的地图可跑），崩溃只与**可玩区/对象坐标**有关。
- 因此：**所有对象（出生点/矿/装饰/水面多边形顶点…）的世界坐标必须落在可玩区内**，
  水面多边形顶点也不例外——超出即崩。
- 建议默认尺寸：GRID 1040 / BORDER 20 / 可玩区 1000（这是可跑的最大值）。
  如果一定要再大，目前没有可行证据支持。
- **矿脉（OreNode）朝向只能取 45/135/225/315 四个斜向**（RA3 矿脉模型只支持斜向，
  不支持 0/90/180/270 正方向）；摆放朝向时用 `round((a − π/4) / (π/2)) · (π/2) + π/4` 对齐。

## 19. 历史研究与现行制作边界

原地形整形配方及早期布局／坐标研究已移出格式规范，保存在[历史研究](../history/terrain-recipes-before-preview-only-2026-10-30.md)，不提供现行执行步骤。可玩制作按[预览边界保护](preview_terrain_lock.md)保持层边界、水陆边界并允许添加斜坡；需改变边界时返回[游戏预览制作](terrain_preview.md)，不以旧配方绕过当前规则。坐标及资源分别按[当前坐标规范](map_coordinates.md)和[分级布矿](resource_distribution.md)执行。
