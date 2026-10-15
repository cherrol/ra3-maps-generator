# Red Alert 3 (SAGE) `.map` 二进制格式规范

> 来源：OpenSAGE 开源项目（https://github.com/OpenSAGE/OpenSAGE，master 分支）
> 主要参考目录：`src/OpenSage.Game/Data/Map/`、`src/OpenSage.Game/Logic/Map/`、`src/OpenSage.Game/Scripting/`、`src/OpenSage.FileFormats/BinaryReaderExtensions.cs`
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

## 19. 地形整形配方（山脉 / 河流 / 桥梁）

### 19.1 只保留 2~3 片山脉，其余铺平
思路：先算"山体量"（高出平原多少），做形态学开运算去掉小山，再按连通域面积只留前 N 片。

```python
mountain = relief > relief_min          # 例：relief_min = 60 世界单位
big = opening(mountain, radius)         # 半径 14 格：腐蚀 N 次再膨胀 N 次，去掉零散小山
big = keep_largest_components(big, 3)   # 只留面积最大的 3 片山系
relief = relief * smooth(big, 3, 2)     # 平滑掩码，山体边界不出悬崖
z = LAND_Z + 4 + relief + hills         # 其余地方 = 平原高度
```
- `keep_largest_components`：4 邻接 BFS 给连通块编号（`collections.deque`），按面积排序取前 N。
  格数 ~1e6 时纯 Python BFS 也可接受（只遍历掩码内的格）。
- 效果：山脉从 11.4 万格收到 2.4 万格，陆地坡度>24 的不可通行只占 0.7%。

### 19.2 标志性河流加宽 + 可建造的平坦河岸
河道剖面按"河道 → 岸坡 → 平坦河岸 → 缓坡接回地形"四段做（距离中心线 d，单位格）：

```python
z = where(d < r_bed,                 bed + (d / r_bed) * 8,        z)  # 河床（水面以下）
z = where(r_bed <= d < r_bed+ramp,   lerp(WATER_Z+1, bank_z, w),   z)  # 岸坡
z = where(... <= d < ...+flat,       bank_z,                        z)  # 平坦河岸（可摆建筑）
z = where(... < d < ...+flat+out,    lerp(bank_z, z_地形, w2),      z)  # 缓坡接回地形
z = smooth(z, 2, 2); 再重新钳住河床与平岸
```
- 参考取值：`r_bed=10, ramp=8, flat=10, out=14`，`bank_z = WATER_Z + 22`。
- 河道半宽 10 格 → 水面宽 20 格（200 单位）；平坦河岸 10 格宽、高差 0 → 可摆建筑
  （实测河岸带 80~99% 坡度<1，可直接建造）。
- 加大 `r_bed` 会显著提高水面占比（本例 13.6% → 15.6%），也就更需要桥。

### 19.3 桥梁（让陆军能过河）
- **对象名**：原版 RA3 桥是 `Bridge1a` / `Bridge2a`（官方图 clefted_city、dmap_pool_march 在用）；
  社区图另有 `HB_Bridge_01`、`KV_Bridge_01` 等自定义桥。
- **构造**：一座桥由多个"桥段"对象沿桥轴串成，桥段间距实测 **约 450~500 世界单位**
  （官方图相邻桥段 407~595 单位）。属性与普通对象一致，`originalOwner` 用
  `PlyrCivilian/teamPlyrCivilian`、`objectIndestructible=1`。
- **放法**：桥轴垂直于河流走向；桥段放在**两岸岸顶**（距离中心线 `r_bed + ramp` 格），
  桥心要吸附到附近**最深的河道格**（否则桥心可能落在被压平的岸/平台上，桥就架在旱地上）。
- **前提**：模板里没有桥对象时，可克隆一个已有对象（如 `GarageTechStructure`）改
  `type_name` + `originalOwner`；属性字典是通用的，克隆可行。
- **安全网**：架桥之外仍要保证陆地本身连通（各出生点同一连通域），这样即使某个桥段
  没生效，陆军也能绕行；桥是额外通道而非唯一通道。
- 实测：4 座桥桥心全部落在水面（z 178~185 < 水面 200），两岸桥段全部落在岸上（z≈220），跨度 480 单位。

### 19.4 干谷 vs 成水河道（让次要河流可通行）
不是每条河都要成水。次要河流做成**干谷**（可通行）能避免把地图切得过碎：

- 成水条件只看河床高度：`bed < WATER_Z` 才成水；`bed > WATER_Z` 就是干谷（低地走廊）。
- 雕刻要分两种：
  - 成水的河：`z = min(z, bed + …)` 并**强制** `z = min(z, WATER_Z - 6)`（保证有水面）。
  - 干谷：`z = min(max(z, WATER_Z + 2), 河床高度)`——只下切、**绝不低于水面**，这样永远可通行。
- **两个必须避开的坑**（否则水系会莫名断开）：
  1. 干谷雕刻**不能碰别条河的成水河道**：先算出所有成水河的 `wet_channel` 掩码，干谷只在 `~wet_channel` 上雕刻。
     否则干谷会在汇流口把主河道从 182 抬到 202，主河被截成几段。
  2. 河岸（岸坡/平岸/缓坡）也要 `~channel & ~sea`：既不能盖住任何河道，也不能把海面抬成陆地
     （否则入海口被"岸"封死，河网与海面永远连不上）。
- 自检：**水面连通域数量**。目标 1（河网+海面一整片）。实测踩坑时依次出现过 14 / 4 / 3 / 2 个，
  修完上述两点后为 1 个（100%）。

### 19.5 矿脉必须落在平整地面
矿脉（矿场）除了平台铺平，**选位**就要挑平地，否则平台会切进山体、看着像挂在坡上：

```python
steep = slope > 10                       # 山脚/陡坡
ok_ore = land & (z > WATER_Z + 1) & (slope < 4)          # 自然坡度很小
ok_ore &= distance_transform(steep) >= 14                # 离山脚有距离
ok_ore &= distance_transform(river_channel) >= 22        # 离河有净距（免得平台堵河/被河切）
```
实测：52 个矿脉自然坡度全为 0.00、离山脚 20~40 格，铺平后 r=18 高差全 0。

### 19.6 山地只留指定方位（如东西两侧）
按方位保留山系，中间全部铺平：

```python
w0, w1 = 0.34, 0.66
side = (xx < w0 * GRID) | (xx > w1 * GRID)      # 西侧 / 东侧
big = opening(mountain_mask, radius) & side     # 先开运算去小山，再按方位保留
relief *= smooth(big, 3, 2)
```
实测：西侧高山占 31.9%、东侧 8.5%、中间仅 5.9%（平滑过渡的残留）。

### 19.7 城市驱动的出生点与资源布置
想让"出生点避开城市、资源围绕城市"时，先把**城市中心**解析出来，再用它当约束/锚点：

```python
CITY_SEATS = {'南宁市': (108.37, 22.82), ...}     # 广西各市用真实市区坐标
def city_centers(geo):                            # 邻省城市用多边形质心
    ...                                           # 必须过滤掉落在可玩区之外的中心！
```
- **出生点避开城市**：解算时要求 `到所有城市中心 >= BASE_CITY_CLEARANCE`（例 30 格）。
  注意：如果出生点配置本身已经被移到地图边缘，就会离城市很远（本例 30~140 格）。
- **油田围绕城市**：每个出生点取**离它最近的城市中心**（不是"同名城市"！出生点避开城市后两者会差很远），
  在其周边放 2 个油井（角度错开、吸附到合法格）。
- **非出生点矿脉集中在城市**：每城 1 个，环向偏移（例 14 格）后交给解算器吸附。
  广西 14 市用真实市区坐标，邻省按间隔抽样凑数。
- **地图中心要点**：`POST_POS` 用地图中心经纬度（world 5000,5000 ⇒ cell 520,520 附近）；
  模板自带的观察哨比配置多时，把多余的从对象表里删掉（`objs.remove`）。

### 19.8 矿位离河净距必须大于"河道半宽 + 矿场核心半径"
平台铺平发生在河道雕刻**之后**，而收尾还会再补铺一次河道来保证水系连通。
如果矿位离河道太近，补铺河道会**切进矿场平台核心**，导致平台出现微高差（实测 3.01 单位）：

```
净距 > 河道半宽(r_bed) + 矿场核心半径 + 余量     例：12 + 18 + 6 = 36 格
```
改为 36 格后，52 个矿脉铺平后 r=18 高差 max 1.48（唯一一个 1.48 是圆形取样在核心边缘的取整误差），
平均 0.03。

### 19.9 出生点/资源对象数量的裁剪
模板对象数可以多于需求（如模板 4 个观察哨、需求 2 个雷达）：直接 `objs.remove(o)` 从对象表删掉，
不要留在原地（否则会出现多余的中立建筑）。

### 19.10 出生点纬度按地图比例定位
需求常见形态："南北两队大致在地图 1/5 与 4/5 的纬度附近，且不必排成一条直线"。
做法：先按比例算出目标格坐标，再反算经纬度写进配方（保持**每点不同的纬度/经度**以形成交错）：

```
y = BORDER + frac * PLAY            # 例 frac=0.20 -> y≈220，frac=0.80 -> y≈820
lat = LAT1 - (y - oy) / sy          # 反算
```
- 交错（各点 y 相差 20~50 格）比整齐一排更自然，也能避免出生点互相对轰。
- **必须同时满足"离河道净距"**：出生点平台半径 40 格，而收尾会补铺河道来保证水系连通，
  但为了保住基地平台，补铺时会跳过基地核心。于是**基地离河道中心线太近就会把河堵死**
  （实测北海基地距南流江仅 18 格 → 水面从 1 块裂成 3 块）。
  约束：`离河道中心线 >= 平台核心半径 + 河道半宽 + 余量`（例 36 + 12 + 4 = 52 格）。
- 解算器依次放宽：`避开城市 + 避开河道` → `只避开河道` → `只要求陆地`。

### 19.11 城市中心必须过滤界外点
邻省多边形会超出取景框，其质心可能落在可玩区之外 → 用它做偏移会算出越界格
（实测 `IndexError: index 1241 is out of bounds`）。解析城市中心时先按可玩区过滤。

### 19.12 矿优先级最高：矿区建造范围内清空一切
规则：**所有资源里矿的优先级最高**；矿的"建造范围"内不允许出现任何建筑、资源或山脉
（海洋不影响摆放矿场，允许）。

实现要点（关键是**顺序**和**统一间距**）：
1. **先解算矿**（矿是第一个进 `placed` 列表的资源），后续所有资源都在"已放置点"约束下解算 —— 天然让矿优先。
2. 其他资源解算时把间距从 `RESOURCE_MIN_SPACING` 提升到
   `max(RESOURCE_MIN_SPACING, ORE_CLEAR_RADIUS + 2)`，于是油井/雷达/车库/机场/医院/精兵学院
   都会自动与**任何矿**保持 ≥ 矿区半径的净距。
3. 装饰物同理：对矿用 `ORE_CLEAR_RADIUS + 2`，对其他资源用原来的 `DECOR_RESOURCE_CLEARANCE`。
4. **山脉**：矿位的 `离山脚净距` 必须 > 矿区半径，即
   `ore_mount_clearance = max(配置值, ORE_CLEAR_RADIUS + 4)`；配合"自然坡度 < 4"的选位条件，
   矿区半径内不会出现山体。
5. 矿脉间距 `ORE_MIN_SPACING` 必须 > `2 * ORE_CLEAR_RADIUS`，否则两个矿的矿区会互相侵入
   （本例 38 > 2×18）。

实测（`ORE_CLEAR_RADIUS = 18` 格 = 180 单位）：
- 矿区范围内其他对象 **0 个**；范围内地形高差 **0.00**；
- 其他资源离最近矿脉：油井 32.0、雷达 32.2、精兵学院 32.2、车库 69.1、机场 75.3、医院 84.1、装饰物 18.4 格。

### 19.13 真实海陆判定：用海岸线多边形，别用"每列最南陆地"启发式
需求："真实地形南边的海要做成水域，不能画成山脉"。

**错误做法**（会踩的坑）：只拿中国省界当陆地，用"每列最南的陆地格 + 国界折线以东"来判海。
北部湾（广西与越南之间的海湾）在国界折线以西 → 被当成陆地 → 被"省外补全"填成**山脉**。

**正确做法**：显式给出**真实海岸线闭合多边形**，陆地 = `省界 ∪ 海岸线多边形`，其余即海域：

```python
COAST = [(103.0,18.5), (105.0,19.6), (105.8,20.1), ... 越南海岸 ...
         (108.05,21.55), ... 广西海岸 ... (109.10,21.48), (109.60,21.52),
         ... 雷州半岛 ... (111.05,21.45), (112.00,21.68), (113.5,22.2),
         (113.5,28.8), (103.0,28.8)]          # 四角必须扩到取景框之外！
_img = polygon(COAST);  land_all = land | (_img > 0)
sea = ~land_all
land_or_sea = (~land) & (~sea)                # 越南/老挝方向 → 省外补全（山地）
```
要点：
1. **海岸线要包含邻国海岸**（越南），否则海湾会被判成陆地。
2. **多边形的四角必须扩到地图取景框之外**（本例 lon 103.0~113.5、lat 18.5~28.8）。
   否则可玩区外的边界条带会落在多边形外 → 被整条判成海（实测出现 7793 格 + 215 格的"边框海"）。
3. 取**并集**（`land | polygon`）：海岸线画得略偏也不会把中国省界吃掉。
4. 判定完就丢掉 `land_all` —— 站位/可通行性仍用「省界」，避免把外国领土变成可建造区。

实测：北部湾（锦普外海 z=116、海中央 z=114、湾北 z=198）全部为水域；
越南/老挝方向仍是陆地山脉（z=326~338）；水面占比 17.5% → 18.1%。

### 19.14 高度分层（海面 / 平原 / 一级 / 二级）
约定：**海平面 200 · 平原 210 · 一层地面 280（可通部队）· 二层地面 320（可通部队）· 最高只到 320**。

```python
levels = [210.0, 280.0, 320.0]
landm  = z >= WATER_Z
# 1) 量化前先粗平滑：否则量化结果会碎成小斑块，缓坡占比过高
zs = np.where(landm, z, levels[0])
for _ in range(presmooth):            # 例 3
    zs = smooth(zs, 1, 1)
# 2) 量化到最近的设计层
zq = levels[argmin(|zs - level|)]
# 3) 层间缓坡：整层必须能通部队（坡度 < SLOPE_IMPASSABLE）
base = zq
for _ in range(ramp):                 # 例 2
    base = smooth(base, 1, 1)
z = np.where(landm, clip(base, 210, 320), z)     # 水格保持原样
```
配套必须一起改的地方：
1. **上限**：`z_clip` 与边缘挡墙高度都要压到 320（本例 z_clip hi=110 ⇒ 210+110=320；wall h=110）。
2. **矿位的"山脚"判定要改**：分层后层间缓坡带很宽，用「坡度 > 阈值」判山脚会把大片可建平地误判成山脚，
   候选格从 4.2% 掉到 1.1%。改成**按层边界判定**（相邻格层号不同即为山脚），并给城市加平坦平台
   （`city_pad`，半径 26/过渡 14），候选格回到 3.4%，且城市矿脉有平整落点。
3. **资源要离地图边界留净距**（本例 `d_edge > 6`）：否则落点会被挤到边缘挡墙上，
   而挡墙是最高层 → 矿区 18 格半径内出现 110 单位高差（实测踩到）。
4. **矿位兜底要分级**：严格（平整+净距）→ 干燥陆地 → 任意陆地，绝不退回水里/河上。

实测：高度 113..320；层分布 210 占 22% / 280 占 20% / 320 占 23% / 层间缓坡 35%；
陆地坡度 > 24 仅 0.30%；**矿区 18 格半径内高差 0.00、最大坡度 1.15**（无山脉）。
副产品：量化后地形压缩率大幅提升，地图文件 1.84 MB → 0.75 MB。

### 19.15 出生点平整可建面 ≥ 80×80 格
规则：**每个出生点至少保证 80 格宽或 80 格长**的平整可建面（特殊地形如海岸/断崖除外）。

做法与度量：
1. 出生点平台半径给足：`pad.radius >= 40` 只保证直径 80 的圆，
   但**圆内最大内接正方形只有 0.707×80 ≈ 57 格**，且相邻资源平台会啃掉边缘。
   要两个方向都稳过 80，取 `radius ≈ 47`（核心直径 940 单位）。
2. 度量方式（已固化进 `tools/verify_map.py` 的校验输出）：
   从出生点格出发，沿 x / y 轴向两侧量到高度偏离为止，得到「宽」「长」；
   判定 `宽 >= 80 or 长 >= 80`。
3. **容差取 3 世界单位（≈0.3 格）**，而不是 1 单位：0.3 格的差异远小于一格，
   工程上等同平整、完全可建造；用 1 单位会把缓坡尾段也算成不平，误报。

实测（半径 47 的平台）：
- 容差 1.0：至少一个方向 ≥80 的 **8/8**（最小宽 75、最小长 95）；
- 容差 3.0：**两个方向都 ≥80 的 8/8**（最小宽 81、最小长 97）；
- 容差 4.0：最小宽 84、最小长 103。

### 19.16 高度层 / 坡度 / 通道 / 矿脉布置（一整套硬规则）
现行取值：**海面 200 · 平原 210 · 一层 280 · 二层 350；最高 350**（`levels = (210, 280, 350)`）。

**(1) 斜坡坡度 ≤ 45°**
`slope = |∇z|`（世界单位/格），45° 对应 10。做完高度量化后加一道**坡度限制器**：

```python
smax = slope_max / 1.5          # 关键：∇ 是对角合成 sqrt(dx²+dy²)
for _ in range(slope_relax):    # 例 28
    for axis, sh in ((0,1),(0,-1),(1,1),(1,-1)):
        z = np.minimum(z, np.roll(z, sh, axis=axis) + smax)   # 只削高的一侧
    z = clip(z, levels[0], levels[-1])
```
- 单轴上限须取 `slope_max/1.5`（甚至 `/√2`）：只按 10 约束单轴时，对角梯度可达 14（≈55°）。
- 实测：内陆（离边界>12、离水>4）陡于 45° 的格子从 8.3% 降到 **0.035%**；
  残余都在边界墙带与海岸带（属特殊地形）。
- 边界挡墙高度也要压到"比旁边高一层的量级"（本例墙抬到 280，落差 70），否则 140 的落差削不动。

**(2) 最窄通道 ≥ 20 格**
用"侵蚀检验"代替逐点测宽：把可通行区按到障碍的距离**侵蚀 10 格**，若各出生点仍在同一连通域，
则所有通道都 ≥20 格（窄于 20 的通道会被侵蚀掉而断开）。

```python
eroded = walk & (d_impassable >= 10)
# 检验：各出生点在 eroded 里仍连通
```
实测：侵蚀后 8 个出生点仍同属一个区域 ✓。

**(3) 每个出生点的矿脉：100 格内 2 个 + 需拓展 2 个**
- `BASE_ORE_NEAR = ((-44,-22),(46,-20))`（离基地 ~49/50 格，在 100 内）
- `BASE_ORE_FAR = ((-96,-74),(104,-70))`（~121/125 格，必须挪基地或再起矿场才能采）
- 其余位置（城市矿脉）以**单矿**分布为主。

**(4) 矿贴山脚 + 矿口朝外**
- 判据：矿心到"层边界"的距离落在 `ORE_FOOT_MIN..ORE_FOOT_MAX`（缓坡带之外、紧贴坡脚），
  且有一个斜向沿 `ORE_MOUTH_CLEAR` 格都同层（矿口净空）。
- **必须让资源平台保留陡坎**：平台铺平时加"改动幅度上限"，否则矿场平台会把身后的山脚一起抹平，
  矿就不贴山脚了：
  ```python
  znew = z*(1-w) + h*w
  z = np.where((np.abs(z-h) > PAD_MAX_DELTA) & (~is_base), z, znew)   # 只抹平 <=4 单位的小起伏
  ```
- **`np.roll` 的方向语义**：`out[i] = arr[i-s]`，检查"+d 方向"要传负数偏移，否则矿口会朝反（朝山里）。
- **矿优先级最高要真正生效**：`map_gen` 里若矿脉直接用"地形阶段已解算好的坐标"，
  一定要把坐标补登记进 `placed` 列表，否则后续资源/装饰物的避让逻辑根本不知道矿在哪（实测有 1 个装饰物压在矿区）。
- 装饰物避让要"**先对齐网格再外推**"并反复迭代：先外推再 `snap` 会被拉回矿区。

实测：贴山脚 45/60、矿口净空 58/60、矿区范围内其他对象 0 个、矿脉朝向集合 {45,135,225,315}。

### 19.17 出生点「80×80 真方形强制全平」
要求：每个出生点周边必须是一块 **80×80 格**的平地。

- **圆 vs 方**：平台半径 R 的圆只能内接边长 `R√2` 的正方形。
  要覆盖 80×80，须 **R ≥ 40√2 ≈ 56.6**；用 47 时实测 80×80 方形内高差达 **31.45**（不合格）。
- 取 `pad radius = 62`（62/√2 ≈ 43.8 > 40，留 3.8 格余量），过渡 30。
- 连带：平台核心变大 → **出生点离河道净距**也要跟着放大
  （`> (pad半径-4) + 河道半宽`，本例 58+12 → 取 72），否则平台会压住河把水系切断。
- 度量：以出生点为中心取 ±40 的**方形**（不是轴向十字），高差 < 3 视为全平。
  实测 8/8 全部 **0.00**。

### 19.18 矿场摆放位「强制平整」（锥形，且不造陡坡）
要求：矿场摆放的位置必须强制做成平地。

- 用**矿口锥形扇面**（±75°、半径 20 格）而不是整半平面：
  整半平面会把 90° 方向的崖壁也圈进来，实测边缘会留下 7.19 单位误差。
- **过渡宽度按坡度自动摊开**：`need = |Δz| / (slope_max/1.5)`，
  `w = clip((flat_r + need - d2)/need)`；半径内 `w=1` 强制压平。
  这样落差越大过渡越宽，既保证核心全平、又不会在边缘造出陡于 45° 的人工坡。
- 必须**迭代多遍**（本例 4 遍）：相邻矿脉的过渡带会盖到别人的核心，
  后续遍要重新读中心高度再压实。
- 实测：60 个矿脉的矿口核心（锥内 r16）高差全部 **0.00**；
  身后的山脚陡坎保留（贴山脚 37/60，其余 23 个是"近矿"故意不贴）。

### 19.19 近矿必须落在基地建造范围内（用社区数据反推半径）
游戏数据里没有明文建造半径，**从 462 张社区地图反推**：出生点到最近矿脉的距离
`p50 = 331`、`p90 = 432` 世界单位 —— 这就是玩家实际能直接建造到的范围。

- 每个出生点 4 个矿：**近矿 2 个**（目标 ~37 格 ≈ 370 单位，并把"落在 45 格建造范围内"
  作为硬约束；若目标位置不合法，直接**从基地中心向外找最近合法格**）+
  **远矿 2 个**（~121/125 格，必须挪基地/再起矿场）。
- 近矿**不要求"贴山脚"**：它就坐在基地大平原上（基地平台 62 格内本来就没有陡坎）；
  "贴山脚"只对远矿与城市矿脉生效，否则近矿会被崖壁约束挤到 500+ 单位外。
- 连带：`ORE_BASE_CLEARANCE` 要从 46 降到 30，否则近矿（37 格）根本放不下去。
- 实测：8 个出生点自身近矿全部在 **303~386 单位**（p90 以内）✓。

### 19.20 只保留 2~3 片大山（清掉所有零散小山）
要求：整张图只留 2~3 片大山，其余小山体一律做成平地。

**(1) 致命陷阱：高度场重新内联表达式会让山体掩码变成死代码**
```python
relief = (mg_base + amp * mg_amp) * mount * coast * river
big = opening(relief > relief_min, radius) & side       # 开运算去小山
big = keep_largest_components(big, 3)                   # 只留最大 3 片
keep_soft = smooth(big.astype(float), 3, 2)
relief = relief * keep_soft                             # ← 掩码在这里生效
...
z = LAND_Z + 4.0 \
    + (mg_base + amp * mg_amp) * mount * coast * river \  # ✗ 又内联了一遍未掩码的！
    + hills * (...)
```
第二处**重新内联**了未掩码的表达式，于是 `opening` / `keep_largest_components` / `keep_soft`
全部作废 —— 症状是"满地图零散小山、怎么调山体参数都没反应"：
量化前省内陆地只有 2.1% 高于 245，量化后却有 **57.9%** ≥275（平台层仅占陆地 61%）。
改成用掩码后的变量 `z = LAND_Z + 4.0 + relief + hills * (...)` 后：
**平台层占陆地 90%、层间缓坡 10%、山体核心从 36 万格降到 14 万格、内陆陡坡 0.303% → 0.017%**。
> 教训：地形流水线里同一段数学表达式只能有一处；改参数没反应时，先确认公式没有被别处重新内联。

**(2) "小山"的判据不能用 z > 平原+20**
缓坡会把全图连成一大片（实测"最大的 3 块"占 52% 陆地，等于没清）。
正确做法：以**一层(280)及以上**作山体核心取连通域，只留最大的 N 片；
再把核心**膨胀出缓坡 halo**（`smooth(keep, 10, 3) > 0.02`）算作山体的一部分，
其余隆起（含其缓坡）用平滑权重压回平原层。

**(3) 连带影响**
- 山变少 ⇒ 有崖壁可贴的地方变少，"矿贴山脚"的达标数自然下降（本例 37/60 → 14/60），
  这是**预期结果**，不是回归；近矿本来就不要求贴山脚。
- 地形整体变平 ⇒ 内陆陡坡、通道等指标同步变好。

### 19.21 出生点用标注图直接定坐标 + 南部做成大片水域
用户直接在预览图上圈了 8 个位置（上排 4 / 下排 4）。换算方法：
预览图 = 地图像素 `GRID*S`（S=2）叠在 150px 标题带上，用户图又按 `960/2080` 缩过，
所以 **格 = 用户图坐标 × (2080/960) / S**（≈ ×1.0833）。

- 在配置里加 `base_cells`（格坐标列表，按队伍顺序）**覆盖** `BASES` 的经纬度：
  `'base_cells': [(915,233),(680,233),(433,240),(193,233), (902,856),(623,850),(409,850),(168,845)]`
  即北队 4 人 y≈233、南队 4 人 y≈850，各队自东向西等距排列。
- 圈点若落在海里/省外，仍由 `resolve_base_cells` 就近吸附到**省内合法陆地**
  （避城市+避河道 → 避河道 → 只要陆地）。注意**搜索半径要够大**
  （本例 90 → 240：南部扩海后，西南角那个圈点的最近陆地超过 90 格，会直接报"找不到陆地"）。
- **南部大片水域**：省界以南按列全部算海——
  ```python
  _south = np.where(land, yy, -1).max(axis=0)          # 每列最南端的省界陆地
  sea |= (yy > _south + sea_south_margin)
  ```
  这一步会把原本"省外陆地（越南/老挝山地）"整体变成水：**海域 15.4% → 28.2%**，
  省外陆地 144055 → 5388 格，形成完整的北部湾/南海。
  南队 4 个出生点因此自动落在**海岸线旁**（面向大片水域）。
- 连带：陆地变少 ⇒ 山体核心 22249 → 16465 格、平台层占比升到 **96%**；
  海岸崖壁变多 ⇒ "矿贴山脚"回升（14/60 → 23/60）。

### 19.22 桥梁：中路要有 2~3 座南北向的桥（且预览必须画出来）
- **桥永远垂直于它跨的那条河**：东西向的河（西江/郁江）上的桥自然是**南北向**；
  南北向的河（柳江）上的桥则是东西向。所以"要南北向的桥"= 把桥放在**东西向的河道**上。
  实测角度：西江 0.26→110°、0.44→93.5°、郁江 0.52→77.4°（都是南北向），柳江→14.7°（东西向）。
- 每座桥 = **2 个 `Bridge2a` 桥段**（各占半个桥面），角度相同、沿桥向分开 ~48 格，
  跨度实测 **480 世界单位**（官方桥段间距）。
- **预览图必须画桥**：原来 `make_preview` 只画矿/油井/哨站/车库，
  用户完全看不到桥，误以为"没有桥"。已加：沿桥向画绿色粗线段 + 白点标记，
  并且要在"资源点之后"画，避免被盖住。

### 19.23 矿脉布局（现行方案）：近 2 + 周边 2 + 中路争夺 4~6
每名玩家 4 个矿 + 中路零散争夺 4~6 个，共 **38 个**。

**(1) 每个玩家的 4 个矿按「斜向绕圈」摆**
`python
BASE_ORE_DIRS = ((.707,-.707), (-.707,-.707), (.707,.707), (-.707,.707))  # 东北/西北/东南/西南
ORE_NEAR_R = 38.0     # 近 2 个（基地可直接建造，约 380 单位，落在建造范围内）
ORE_MID_R  = 78.0     # 周边 2 个（约 780 单位，需拓展建造范围）
# 相邻出生点轮流用不同的两个方向，避免两队矿位撞在一起
near_d = [d0, d2] if i % 2 == 0 else [d1, d3]
far_d  = [d1, d3] if i % 2 == 0 else [d0, d2]
`
**矿口一律朝「远离基地」的方向**（snap_diag(atan2(dy,dx))），4 个矿分居 4 个斜向，
天然做到「不并排、矿场建造区不重叠」。

**(2) 中路争夺矿：绕地图中心圆周摆放 4~6 个**，矿口背离中心
`python
CENTER_ORE_COUNT = 6; CENTER_ORE_R = 92.0; CENTER_ORE_PHASE = 22.0
`
实测：6 个、平均位置 (528,506)（中心 520,520），离中心 49~104 格。

**(3) 硬约束：矿场建造区绝不重叠**
- ORE_MIN_SPACING = 46（> 2x 矿场半径 36），实测矿间最小中心距 46 格、重叠 0 对。
- **
esolve 的兜底绝不能无视间距**：原来兜底是「退化成最近的可通行格」，
  结果把矿放到**基地正中心**、还出现两矿只差 **2 格**。现在兜底仍要求
  >= spacing*0.92 且不低于基地净距，实在放不下就**返回 None**，让调用方换更宽的掩码。
- **模板自带的多余矿物件必须删除**：ensure_count 只补不删；模板 40 个而我们只要 38 个时，
  多出的会被「撒到外圈」，绕过全部布局约束。要显式 objs.remove(...)。

**(4) 尽量不并排**
90 格内若已有同朝向（+-12 度）的矿，就把当前矿**转 90 度**（交叉摆放）。
实测相距 <90 格的 39 对里：互相垂直 30 对、同向仅 9 对。

**(5) 归属判定别看对象顺序**
地图保存后矿物件的顺序未必等于放置顺序（按索引 ore[i*4:i*4+4] 归属会错得很离谱，
实测出现「基地正中心有个矿」的假象）。**按「离哪个出生点最近」归属**才可靠。

### 19.24 地表纹理：城市水泥路 + 其余全草地
要求：城市一定范围内用水泥路美化，其余平原与二三层高地一律草地。

- **城市半径**：CITY_TEX_R = 30 格。半径内按噪声在 SteelDeck01(水泥/铺装) +
  BB_Gravel01(碎石) + Dirt_Yucatan06(土路) 之间取纹理。
  实测：城市内 691 块碎石 + 177 块土路 + 176 块水泥。
- **取消按高度铺纹理**：原来 z > 水面+120 -> Dirt、z > 水面+210 -> RA3_Elevation1，
  会把二三层高地铺成泥土/高地纹理；现已删除，**平原与一二层高地全部草地**
  （9 种 Grass_Yucatan* 按噪声选），实测非城市区 **100% 草地**。
- **沙滩必须按「离水距离」判，不能用高度**：
  原来 z < 水面+12 判沙滩，而平地就在 210~212，结果**整片平原被判成沙滩**
  （实测 8894 块沙 vs 1287 块草）。改为 distance_transform(z < 水面) <= 6 才铺沙。
- 陡坡仍有 Rock_Yucatan* 兜底（坡度 >22/34），但本图坡度限制器保证 ≤45°、
  平地为主，实际几乎不触发。

### 19.25 坐标轴朝向：一处翻转，四处必须同步
游戏网格原点在**左下角** -> **第 0 行 = 最南**。地形数组一旦按「第 0 行 = 北」生成，
整张图在游戏里就是上下镜像的（症状：放南边的海显示在北边）。

**改朝向时必须同步检查这四个下游产物**（本项目逐个踩过）：
1. HeightMapData 与所有对象坐标 —— 同源，改纬度->行映射即可一起正过来；
2. _art.tga（游戏大厅/雷达预览）—— write_art_tga 里 
p.flipud(img)；
   注意 TGA 本身是**自下而上**存储（image descriptor=0），最后仍要 rr[::-1]，别相互抵消；
3. preview.png（make_preview）—— 
p.flipud(img)，边界红线图层也要一起翻；
4. uild_view.png（erify_map）—— 
p.flipud(img)，并且**对象标注与出生点标签的 y
   要写成 (GRID-1) - cy**，否则点画对了、标签还在原位。

判定方法：分别取图**顶部与底部**的平均蓝色分量，海（南）那一侧应更蓝。


### 19.28 高度体系（现行）：五档 + 不可通行高山

| 层 | 高度 | 可通行 | 说明 |
|---|---|---|---|
| 海面/海底 | 水面 **200** / 海底 **190** | 不可（水域） | **所有**水面以下一律压成 190（海域拉平），是一整片平坦水域 |
| 平原 | **210** | ✓ | 主要战场 |
| 一层 | **280** | ✓ | 只能从**斜坡口**上 |
| 二层 | **350** | ✓ | 同上；P4(百色) 旁另做一块 350 高台 |
| 三层 | **420** | ✓ | 本轮新增（`levels` 第四档） |
| **高山** | **480 ~ 1000** | ✗ | 由真实 DEM（>850 m）生成，连片、不可通行 |

**(1) 平台默认是"陡坎 + 少量斜坡口"**
不再全局做坡度限制（那样会把所有边界抹成缓坡、到处是中间高度）。
做法：量化到精确层高 → 边界天然是陡坎（坡度超限 → 自动不可通行）→
只在 N 个"口子"（`ramp_openings`，选在外侧地形最平坦处，半径 `ramp_half_width`）做缓坡。

**(2) 海域必须"最终"拉平**
```python
z_sea = np.full(z.shape, sea_flat)          # 生成阶段
...
z = np.where(z < WATER_Z, sea_flat, z)      # 函数末尾再兜一次（河道/岸线会改回中间值）
```
只做前者不行：实测水域会出现 22 种高度。

**(3) 高山层必须放在函数**末尾**，并避开平台**
踩过两次：
- 放在 `if levels:` 段里 → 被后面的"清小山/铺平台"覆盖（max 只到 350）
- 想加平台掩码时改错代码块（`terrain_gen` 里只有**一处** `_peaks = keep_largest_components(...)`，
  它当时在 `levels` 段里，那时 `base_cells` 还没赋值 → `NameError: base_cells` → 高山整层被跳过）
正确顺序：**先量化/铺平台/清小山 → 最后施加高山**，此处 `base_cells` 与 `slots['ore']` 都可用，
用 `pad[0]+10` 与 `res_pad[0]+8` 的圆盘把出生点与矿区抠掉，否则高山会从基地中间穿过
（实测 P1~P4 的 80x80 出现 76~156 落差）。

### 19.29 取景框 = 目标区域外接矩形
- **省域占多高由 `V_STRETCH` 决定，不是经纬度窗口**：
  `sx = PLAY/(lon1-lon0)`，`sy = sx * V_STRETCH` —— 改 `LAT0/LAT1` 只挪位置、不改高度。
  广西要铺满 1000x1000（原只占高度 70%）→ `V_STRETCH = 1.40`。
- 窗口取**边界的最外缘**（±0.5% 余量）：`LON0/LON1 = 104.40/112.11`，`LAT0/LAT1 = 20.84/26.44`。
  实测省界占可玩区 **宽 99% / 高 100%**。
- 代价：贴合省界后南海只剩沿海一小条（水域 ~4%）；要留海就降 `V_STRETCH` 或调 `lat_bias`。

### 19.30 岸线不要用矩形硬拼
西南/西侧陆地若用矩形叠加，会看到明显的直角边界。
做法：用平滑噪声扰动边界（`smooth(n_detail, 9, 2) * 46` 格），使岸线随地形起伏。


### 19.31 矿位新规则：不放山脚/水陆交界，矿口朝开阔地带
- **选址约束**（加进候选掩码 `ok_near` / `ok_near_relax`）：
  - `d_steep >= ore_foot_clear(10)`：不在陡坎/山脚
  - `d_water >= ore_water_clear(9)`：不在水陆交界
  - `d_imp = distance_transform(ok) >= ore_open_min(14)`：周围"可放矿"的开阔半径下限
    （这一条同时排除窄路径/水陆走廊 —— 走廊里 d_imp 很小）
- **矿口朝向**：一律取 `mouth_angle()`（最开阔的斜向），**不再朝基地**。
  `mouth_angle` 逐方向探到"不可通行/明显上坡"为止，取最长的那个方向 —— 自然朝向
  平地、水域或高层陆地。
- 实测：中路 6 个争夺矿的东西分布保持 **20% / 35% / 44% / 56% / 69% / 80%**（没被这条规则挤回中心）；
  矿口方向仍只有 {45,135,225,315}。
- **注意**：兜底路径（放不下时逐级放宽掩码）会绕过上述净距，实测仍有矿位离水 5 格。
  要彻底保证，需在兜底时也带上 `d_water`/`d_steep` 下限（或对兜底结果做一次净距校验）。


### 19.32 建筑/资源落点必须"平整"（避开高度参差处）
要求：所有独立资源（矿脉/油井/哨站/车库/机场/学院/医院）的放置点都要落在高度一致的地面。

掩码（在 `resolve_slots` 里统一构造，传给 `resolve(..., mask=_build_ok)`）：
```python
_build_ok = ok & (slope < 2.5)                 # 坡度
              & (d_steep >= 8.0)               # 离陡坎/山脚
              & (d_water >= 4.0)               # 离水域
              & (distance_transform(_edge) >= 3.0)   # 离**层边界**（高度跳变处）>=3 格
```
`_edge` 用"相邻格层号不同"构造 —— 这是关键：层边界处 3x3 内必然有落差，
光用坡度判不出来。

踩过的坑：
- **同一个落点可能有多条分支**：城市旁油井走的是 `resolve(tx+cos*off, ...)` 那条，
  和 `OUTER_DERRICK` 不是一处 —— 只给后者加掩码，16 个油井全部超标（max 高差 22）。
- **`_build_ok` 的定义位置**：必须放在**第一处使用之前**（放晚了会 `UnboundLocalError`）。

实测（3x3 最大高差）：
```
哨站 0.0 ✓  车库 0.0 ✓  机场 0.1 ✓  医院 0.0 ✓
油井 max 22.0 -> 6.6（仅 1 个超标）   矿脉 38 个里 2 个超标   学院 1 个超标
```
**残留原因**：`resolve` 的兜底分支（找不到就退化成"只保证间距*0.92 + 基地净距"）会绕过掩码。
要彻底干净，需让兜底也带上平整度（或对兜底结果做一次校验 + 换次优位）。

### 19.33 层规则：边界平滑 + 删孤立块 + 连片（让"二层高地"连成一片）
**症状**：分层预览（`layers5_preview_v10.png`）里二层(350)碎成 22~24 块，
二层高地之间散落着一层(280)地面 —— 看起来"高地连不成片"。

**规则（三件套 + 收尾）**，实现在 `tools/terrain_gen.py: layer_build()`，
参数在配方 `TERRAIN['level_rule_*']`：

1. **边界平滑**：层号图做多数投票（窗口 (2·r+1)²，`level_rule_smooth_r`，1~N 轮）。
   海面格固定不参与（否则海岸线会被磨掉）。
2. **删孤立块**：某层里面积 < `level_rule_min_area[层]` 的连通块**整块降一级**
   （350→280、420→350、280→210）。这是"删孤立二层块"的正解 —— 它比"把缝隙填平"更能
   提高连片度：一块 5700 格的孤岛，填边缘没用，整块降级才干净。
3. **连片**：某层闭运算（先膨胀 r 再腐蚀 r，`level_rule_close_r[层]`），
   把缺口整片补上；只吃**相邻中间层**，不动海、不跨级。
4. **第二遍删块 + 收尾平滑**（`level_rule_prune2_area` / `level_rule_final_maj_r`）：
   连片后残留的小块再删一次，最后再多数投票一轮把闭运算产生的**方形边界磨圆**。

**参数怎么来的**：用户手改了一份 `layers5_preview_v11.png` 作为目标，
我把上面这套规则写成参数化流水线，在 900/1040 两个分辨率上做**随机+网格搜索**
（`work/fit_fast.py` / `work/fit1040.py` / `work/fit1040b.py`），
以「逐格一致率 + 逐层 IoU（二层双权重）」为目标函数。

**实测（1040 原生分辨率，目标 v11）**：

| 口径 | 逐格一致率 | 二层 IoU | 二层大块数 |
|---|---|---|---|
| v10 原始（未处理） | 0.788 | 0.483 | 24 |
| 只做边界平滑 | 0.796 | 0.520 | — |
| 平滑+删块+连片（一遍） | 0.828 | 0.618 | — |
| **+第二遍删块+收尾平滑（定稿）** | **0.844** | **0.675** | 2 |

定稿参数（写进 `maps/bagui_dadi/config.py`）：
```python
'level_rule_smooth_r': 5, 'level_rule_smooth_passes': 1,
'level_rule_min_area': {350.0: 1600, 420.0: 2400},
'level_rule_close_r': {350.0: 40, 280.0: 10, 420.0: 10},
'level_rule_prune2_area': {350.0: 5000},
'level_rule_final_maj_r': 4,
```

**试过但不对的路子（别再走）**：
- **单纯"膨胀式连片"**（把二层掩码外扩 r 格以内的一层地面并进来）：一致率只到 0.58，
  而且它是**把边界整体外推**，等于全图放大二层，不是"连片"。
- **闭运算解缝隙**：语义最干净（只填两块之间 ≤2r 的窄缝），但真图上可填的缝隙很少
  （r=12 只填 4500 格），因为缝隙往往比 2r 宽 —— 必须先删块再连片。
- **把图层还原成高度场→平滑→重新量化**：一致率 0.814、二层 IoU 0.556，比逐层操作差。
- **直接用 dem.npz 按分位切层**：一致率只有 0.648、二层 IoU 0.237 —— 说明 v11 与真实
  DEM 海拔无关，它是**在 v10 图层上手改出来的**。
- **迭代闭运算（填一次窄缝再填下一轮）**：会级联雪崩（一次把整片一层谷地吃掉），
  必须用"单轮 + 面积阈值 + 第二遍删块"控制。

**注意**：这条规则只重塑**已有**的层分布，不能让地图凭空长出二层地形。
真地图当前二层只占 0.40%（预览 v10 里是 15.4%），因为两条流水线的高度模型不同
（真地图 = 噪声×城市强度 + 最近层量化；预览 = dem.npz 真实海拔 + 自适应分位）。
规则上线后真地图二层升到 2.08%。要让真地图也有 15% 量级的二层，得先把
DEM 五层体系（§19.28）落进 `build_terrain`。

### 19.34 直接用「手改分层图」出地形 + 纯净地形图（不放任何对象）
**用途**：把用户手改的 `layers5_preview_v11.png` 直接当蓝图出图，先只验证地形，不放任何对象。

**配方开关**（`maps/bagui_dadi/config.py`）：
```python
TERRAIN_ONLY = True                 # 纯净地形：不摆任何对象
'use_v11_layers': True,             # 用 v11 图层当高度场
'v11_plain': True,                  # 该图即最终地形（跳过海陆/河道/平台/层规则改写）
'v11_smooth': 1,                    # 浅平滑，磨掉放大产生的方块边
'v11_flip_v': True,                 # 载入时上下翻转（见下面的轴向坑）
'v11_skip_peak': True,              # 跳过 DEM 高山层（v11 里已有 480 层）
'v11_skip_water': True,             # 跳过"西侧填湾"（v11 的海是蓝图的一部分）
```

**数据准备**：`work/make_v11_layers.py` 把 v11 图（900×900，按调色板分类）导出成
`maps/bagui_dadi/v11_layers.npz`（`base_z` 1040×1040 原始高度 + `layer` 层号）：
海 190 / 平 212（与既有平原层一致）/ 一层 280 / 二层 350 / 三层 420 / 高山 480。

**`build_terrain` 里的处理**：v11 分支在 `z` 算出来之后**立即返回**
`(geo, raw, land, amp=0, d_edge=0, base_cells={}, slots={}, [], [])` ——
不叠加 DEM 起伏、省界海陆、河道、平台、层规则（这些都会把 v11 的分层改掉）。
所以 `base_cells`/`slots` 都是空，靠 `terrain.npz` 的 `.get()` 容错。

**纯净地形**：`map_gen.main` 里若 `cfg.TERRAIN_ONLY` 为真，`objs = []` ——
连模板自带的 88 个对象（OreNode×40 / 装饰×25 / OilDerrick×8 / Waypoint×8 /
观察哨×4 / 两个建造场 / 若干废墟）全部清空，实测输出 `objects: 0`。

**成品校验**：T1.4.3 与 v11 原图逐格一致率 **0.973**，各层 IoU：
海 0.797 / 平 0.973 / 一层 0.967 / 二层 0.971 / 三层 0.969 / 高山 0.954。
（海的 IoU 偏低是因为 v11 图把"海"画到了真实地理的北侧 = 图上颠倒，翻转后才对齐。）

**⚠ 轴向坑（务必先读）**：v10/v11 预览图相对**真实地理**上下颠倒（图上"海在上"，
实际北部湾在南、百色高原在西北）。判定：用真实城市经纬度 `geo.to_cell` 取 `dem.npz` 海拔 ——
**翻转后**百色才是 545 m（桂西北高原）、桂林 40 m；不翻则百色 132 m、桂林 158 m，与地理不符。
**做法：在导出 `v11_layers.npz` 时就 flipud 一次**（`flip_v=1`），此后
**数据坐标 = v11 图坐标**，圈/出生点按图坐标直接落；`build_terrain` 里**绝不能再翻第二次**
（会镜像地形，出生点全错位——这里来回踩过两次）。

### 19.36 出生点：北 4 / 南 4、可落各层与水域、且不压平

**要求**：8 个出生点北南各 4；允许落在 一/二/三/四 层（212/280/350/420）甚至**水域**；
**不做任何 80×80 强制压平**。

**做法**（`TERRAIN['base_clear_want']` / `base_prefer_land` 等）：
1. 可落格 = `一/二/三/四层 ∪ 水域`（**排除高山 480+ 与层间陡坎**），
   并要求"周围 `base_clear_want`(8) 格都在该集合内"→ 天然自带平整场地。
2. 按图坐标给定 8 个圈位置，逐个 `snap_to_mask` 吸附到可落格。
3. **落点收尾**：若落点在水里，就在**圈附近 60 格内**找一个陆上可落格（`base_prefer_land`）；
   找不到就保留水域位置（用户允许）——**绝不为了上陆地把落点推离几百格**。

实测（T1.5.0）：8 个出生点全部满足 80×80 可建面（靠挑场地，不是铲地形）；
分布在 212 / 280 / 350 / 420 四层 + 1 处水域；38 个矿全部坡度 0.00。

**踩过的坑**：
- 早期版本把"落水就退到陆地"写成全图搜索（200 格），结果把落点推离圈选位置 **379~733 格**。
  正确做法是**只在圈附近小范围找陆地**，找不到就接受水域。
- `ALL BASES CONNECTED` 会报 False：水域出生点与主陆不连通。**这是允许水域出生点的必然结果**，
  不是地形错误（陆地出生点之间仍然连通）。


### 19.35 不压平平台：靠"天然净空"挑落点 + 矿位按开阔度摆
**要求**：在 v11 蓝图上出生点/矿**不做任何 80×80 强制压平**，矿要往开阔地方摆。

**做法**（配方 `TERRAIN['flat_base_pads']=False` / `ore_open_rank=True`）：
1. **出生点**：可建处 = 平原层 ∩ 坡度<2.0 ∩ `distance_transform(~平原) >= base_clear_want(12)`。
   即"**周围 12 格都是平原**"才算合格落点（没有平整场地就就近吸附到有场地的地方，而不是去铲地形）。
   实测 8 个出生点坡度全 0.00，7 个 80×80 可建面 ≥100 格（P6 只有 54×38，见待办）。
2. **矿位**：先算**开阔度场** `openness = min(到不可通行, 到陡坎, 到水, 到层边界)`，
   只在 `openness >= ore_open_want(18)` 且自身有 `ore_clear_want(6)` 格平地净空的地方挑；
   排序 = 开阔度最大 → 离目标格最近。
3. **同基地 4 个矿按"斜向扇区 + 半径环"分头找**（`resolve_open(..., sector=...)`），
   天然分处 4 个方向，不会因为 `ORE_MIN_SPACING=46` 互相挡掉。
4. **兜底**：限制半径内找不到，就全图搜一个最开阔的合法点（**绝不放坡上/水里**）。

实测（T1.4.6）：38 个矿坡度 max **0.00**、开阔度中位 21 / 最小 15；15 个油井全在平地。

**踩过的坑**：
- `distance_transform(mask)` 返回"**到最近 True 的距离**"，所以开阔度必须写
  `distance_transform(~ok)`；旧代码写成 `distance_transform(ok)`，在可建地内**恒为 0**，
  导致"开阔度"形同虚设（这条是开源代码里就带的 bug，已修）。
- 不做平台压平后，`resolve()` 会返回 `None`（可建地更零散），所有调用点都要能容忍 `None`，
  否则 `TypeError: 'NoneType' object is not subscriptable`（油井/观察哨/车库/机场都中过）。

### 19.46 轴向大排查：7 个部件的方向必须逐个对齐（T1.8.3）
**教训**：这次"南北反"来回了十几轮，根本原因是**没有一把客观尺子**，全靠推理来回翻。
最终定下的判据只有一条，且必须用它验收：

> **「海在屏幕哪一半」** —— 地图的硬特征，翻转必然改变它。
> ⚠ 不要用形状 IoU（两块近似地形互翻也会给 0.79~0.99 的高分，这次因此误判过两次）。

**结构**：整个管线有 **7 个独立部件**，各自可能被翻：
| # | 部件 | 写在哪 | 状态 |
|---|---|---|---|
| 1 | `HeightMapData` 高度场 | `map_gen.build_heightmap_payload` → `flipud(t.raw)` | ✓ |
| 2 | `BlendTileData`（贴图/不可通行） | 同函数 → `flipud(tiles)` / `flipud(impassable)` | ✓ |
| 3 | `art.tga`（小地图底图 + 省界红线） | `write_art_tga` → **画布翻一次**（`arr_img[::-1]`，详下） | ✓（T1.8.8 实机锚定） |
| 4 | 物件世界 y（城市/矿/出生点/油田） | `cell_to_world` → `(GRID-1-BORDER)-cy` | ✓ |
| 5 | `terrain_preview.png` | `terrain_gen` 尾部 → `flipud(col)` + 标注 `(GRID-1)-cy` | ✓ |
| 6 | `build_view.png` | `verify_map` → 底图同 .map；物件 `wy/10 + BORDER` | ✓ |
| 7 | `preview.png` | `make_preview` → `flipud(z)` + `flipud(gx_boundary)` + 物件/城市翻 y | ✓ |

**`art.tga` 为什么"要翻"（T1.8.8 实机锚定，推翻 T1.8.2 的"不翻"结论）**：
TGA(origin=0) 是**自底向上**存储，引擎/PIL 读取时翻一次。要让小地图显示成**北朝上**
（= 游戏内 3D 地形的实际朝向，见下），PIL 读回必须是 raw（内部帧），
所以**画布写出前翻一次**（`arr_img[::-1]`），存储序 = flipud(raw) = 与高度场**数组**同序。
T1.8.2 的"不翻"错在拿高度场**数组行序**当屏幕方向——数组第 0 行 = 世界 y=0 = 屏幕**下**方，
显示时还要再翻一次。

**游戏内的真实朝向（T1.8.8 实机确认）**：**北在屏幕上方、海（南）在下方**。
本节早先"游戏里北在下、海在上"的说法是错的（当时没上机、纯靠数组行序推断）。
因此所有预览图（preview/build_view/terrain_preview/passability_preview）统一**北朝上**出图。

**`cell_to_world` 与 `to_game_row` 的口径**（实测锚点：出生点）：
- 物件世界 y：`wy = ((GRID-1-BORDER) - cy) * CELL`，即 `cy = (GRID-1-BORDER) - wy/10`
- 但**在 .map 高度场里取行号**要用 `row = wy/10 + BORDER`
  （实测：这样 P8→190 水上基地、P5→350、P1→420 全部吻合设计值）
  两者方向相反并不矛盾 —— 一个是"格→世界"的写入，一个是"世界→高度场行"的读取。

**验收脚本**：`work/final_axis_accept.py`（T1.8.8 起判据改为：高度场数组上半是海（游戏帧）
**且** art/各预览图下半是海（北朝上）——两者方向相反是正常的，一个是存储序、一个是显示序）
```
【基准】.map 高度场（游戏帧）：上半海 21.3%  下半 0.0%  -> 海在上 ✓（=屏幕下方是海）
   art.tga (小地图)      上半 0.0% / 下半 21.0%   同向 ✓
   art_preview.png       上半 0.0% / 下半 21.0%   同向 ✓
   terrain_preview.png   上半 0.0% / 下半 21.3%   同向 ✓
   build_view.png        上半 0.0% / 下半 21.3%   同向 ✓
   passability_preview   上半 0.0% / 下半 21.3%   同向 ✓
   preview.png           上半 0.0% / 下半 20.8%   同向 ✓
结论：高度场游戏帧 ✓ + 显示图全部北朝上 ✓
```
⚠ **该验收有盲区，T1.8.7 证明它不够**：它只检查"地形自身"在各产物里的朝向，
测不出 geo 派生物（红线/城市）与地形的**相互**错位 —— 必须同时跑 §19.47 的交叉锚定验收。

### 19.47 geo 帧根因修复（T1.8.7，权威结论；与 §19.39/§19.40 冲突处以本节为准）
**症状**：游戏里省界红线、城市位置与地形**南北相反**（预览图/小地图上红线的海岸弧悬在海上、
沿海城市落在内陆深处）。T1.4.3（v11 蓝图）起就存在，历经十几轮"修轴向"未愈。

**根因**：管线里存在**两个相反的 y 帧**而不自知——
* 地形帧（v11 蓝图 / `terrain.npz` raw / `dem.npz` / 用户标注图）：**第 0 行 = 最北**（海在数组下半 = 南）；
* geo 帧（`Geo.to_cell` 旧公式 `y = oy + (lat-lat0)*sy`）：**第 0 行 = 最南**。

v11 之前地形由 geo 掩码生成，两帧天然一致；v11 起地形换成蓝图帧，`to_cell` 没跟着改，
于是**所有 geo 派生物**（省界红线 `gx_boundary`、城市中心→房屋/水泥纹理/城市油井、
河流净距掩码、装饰物经纬度落点）全部与地形南北颠倒。而 §19.46 的"海在哪一半"验收只看
地形颜色分布，红线和城市不影响"海"的统计，**结构性发现不了这个错位**（§19.39 当年
"14 城全部落在陆地"的判据也没有区分度——全图 89% 是陆地，翻错了也照落陆地）。

**修复（一行）**：`Geo.to_cell` 改为 `y = self.oy + (self.lat1 - lat) * self.sy`（geo 帧 = 地形帧，
省域竖直跨度不变、仅镜像）。同步修正：`make_preview` 城市标注翻 y、`fetch_dem` 反算公式、
`compute_bridges` 的 vy 取负、非 v11 分支的 `lat_row` 反算。
出生点/矿/斜坡本来就是地形帧显式格坐标，不受影响；城市/河流归位后，
"出生点避城市/避河道"等逻辑顺带从"避镜像位置"变成避真实位置。

**新的判别式验收（交叉锚定，替代单源统计）**：`work/diag_geo_frame.py`
1. 沿海城市锚点：北海/钦州/防城港 按 `to_cell` 换算后**距海 ≤ ~50 格**
   （修复前 516~630 格——在地图另一头的内陆）；
2. `geo.make_mask()` 与 `terrain.npz['land']` 的 IoU：同帧 **0.821** / 翻帧 0.633；
3. `dem.npz` 同帧取海拔：百色 **510m**（桂西北高原）> 桂林 **38m**（桂东北低地）。
T1.8.7 实测：§19.46 旧验收全部通过 + 以上三条全部满足 + 预览红线贴海岸。

**教训**：轴向验收必须用"两个不同来源的数据落在同一锚点上"做**交叉**验证；
单源分布统计（海占比、IoU）在两帧各自自洽时永远全绿。

### 19.45 矿位离「层界 / 水陆交汇」的距离（T1.7.5）
**要求**：矿要**离不同层级陆地、以及水陆交汇处远一点**。

**做法**（`resolve_slots` 里的矿位掩码 + `enforce_ore_base_clearance` 清洗）：
1. 新增两张距离图（`ore_open_maps()`，供各处共用）：
   - `d_edge`  = 每格到最近**层界**的距离（层号不同的相邻格）
   - `d_shore` = 每格到最近**水陆交汇带**的距离
   注意 `distance_transform(m)` 的语义是"每格到**最近的 m 内格子**的距离"，
   所以"离层界多远"要传 `distance_transform(edge)`；**传 `~edge` 会得到恒 0**（踩过）。
2. 严格档 `_big_open` 增加 `d_edge >= ore_edge_clear(16)` 与 `d_shore >= ore_shore_clear(12)`；
   另加一档 `_road_open`（各项要求 ×0.6）作为放宽，插在 `_big_open` 与 `ok_near` 之间。
3. 最终清洗 `enforce_ore_base_clearance()` 的判据也**统一**成
   "基地距离 + 层界距离 + 岸边距离 + 邻矿间距"四件套 ——
   否则"清洗挪位"自己会把矿挪到层界旁边（实测出现过挪完离层界只有 3 格）。

**实测 T1.7.5**：
- 离层界：min **9.0**（1 个，在东侧边界被挤压处）/ 中位 34 格
- 离水陆交汇：min **15.4** / 中位 242 格，**没有任何矿 <10 格**
- 基地旁（<44 格）：0 个（43.6~44.0 的 4 个是整数取整误差）
- 每基地 55 格内仍恰好 2 个矿

**遗留**：东侧边缘有 1~2 个矿因"离基地必须 ≥44 格 + 邻矿间距 46 + 地图边界"三重挤压，
只能停在离层界 9~13 格处。若要彻底满足 16 格，需要挪走那里的邻矿或放宽间距。

### 19.44 矿位最终规则：离基地 >=44 格 + 矿口朝向分类（T1.7.2）
**要求**：① 基地旁的矿要**离基地有点距离**（否则基地会把矿场位置卡住）；
② 基地旁的矿**矿口朝基地**；③ 处在**层界边上 / 水陆交汇（岸边）**的矿，
矿口朝**开阔的那一侧**，而不是朝交汇处。

**做法**：
1. `ore_slots()` 的近矿槽位半径 `ore_near_slot_r=46`；
   `resolve_open` 的扇区半径环改成 `[ore_near_min_r(44), ore_near_max_r(62)]` —— **下界是关键**。
2. `resolve_open(..., min_base_dist=(bx,by,d))` 新增参数：所有基地矿路径
   （正常/水上兜底/全图 `_big_open` 兜底）都要满足"离基地 >= 44 格"。
3. 保底补矿 `ensure_base_min_ore(..., rng=66)` 内部也加了 `_min_r` 检查。
4. **最终清洗 `enforce_ore_base_clearance()`**：把任何仍贴在基地上（< 44 格）的矿，
   沿"原矿相对基地的方向"在 44~90 环上重新落位。实测一次挪走 6 个。
5. **矿口朝向在位置全部定稿后统一重算**（`_final_angles()`）：
   - 离最近基地 <= `ore_near_max_r(62)` -> `mouth_angle(..., toward=基地)` = **朝基地**
   - 其余 -> `mouth_angle(...)` = **朝最开阔的斜向**（沿 4 个斜向做射线打分：
     同层/同水面各 +1，遇到陡坎即停，再给"22 格层号一致"的斜向 +6 加权）

**实测 T1.7.2**：基地旁（<44 格）的矿 **0 个**；基地旁 22 个矿**全部朝基地**；
其余 23 个按最开阔方向定朝口；每基地 55 格内仍恰好 2 个矿。

**踩过的坑**：矿口朝向原来是在矿位循环里**逐个**算的，
但位置之后还会被"保底补矿 / 最终清洗"挪动 -> 角度与新位置不匹配（实测出现 45° 的默认角）。
必须**等位置全部定稿后再统一重算**。

**T1.9.0 补强（矿位四规则收尾，验收 `TAG=… python work\check_ore_rules.py`）**：
- `trim_near_ore_quota`：保底补矿 + 清洗挪位叠加可能让某基地近矿变成 3 个
  （实测东南/极西南各 3 个），裁到**恰好 2 个**，超出的挪到 70~100 格远矿环；
- `enforce_ore_mouth_open`（**取代** `_final_angles`，位置定稿后统一算角度并保证朝向开阔）：
  矿口射线前方 8~20 格内不得撞 水/陡坎/层界/水陆界；撞了先换向、不行再挪位
  （附近 40 格同水陆；水上基地附近可放宽到水面 60 格、离岸 >=4）；
- `mouth_open_map` 作为 `ensure_base_min_ore` / `enforce_ore_base_clearance` 的候选格过滤，
  **从源头**不再放出"矿口撞障"的位置；
- 豁免：水上基地的近矿隔海朝基地是设计本意（R1"朝基地"优先于 R4b"前方开阔"）；
- 中路争夺矿 = `CENTER_ORE_COUNT=6` 个，x 占图宽 20/32/44/56/68/80% 均匀铺开
  （验收按 `ore_slots` 槽位溯源核对，别用"离基地>100 格"当判据——远矿会混进来）。

**T1.9.5 补强（矿场可建造性，验收 `TAG=… python work\check_ore_buildable.py`，0/43 摆不下）**：
查"矿场摆不下"发现三个漏洞，全修：
1. **impassable 豁免洞打错位置**（map_gen）：原来打在 `ore_slots()` 的**设计槽位**上，
   而最终矿位经保底/清洗/挪位可偏出 15~145 格 → 洞全打歪，真正的矿位被禁建位图盖住
   （实测南西某矿 r12 内 24% 不可通行）。改打在 `t.ore_cells` **最终落点**：
   核心盘 r13 + 矿口朝向半圆盘 r18（±99°），且不碰 `d_edge<1` 的图边线；
2. **矿口判定只看一条朝向射线**（terrain_gen）：v11 模式不做矿口半圆盘压平，
   射线畅通但半圆盘两侧带坎的矿（实测 5 个，矿口高差 22~70）照样摆不下。
   `enforce_ore_mouth_open` 新增 `pad_flat`：核心盘 r12 高差 ≤10、矿口半圆盘
   r16（±100°）高差 ≤12、离图边 ≥14（⚠ 东缘矿曾被图边禁建带切掉地基），
   不平即换向/挪位（挪位搜索放宽到 56 格），水面矿豁免地基；
3. 遗留的 `parse_blendtiles` 纹理读取不可信（见 §19.38）：验收位图/纹理一律直接解析
   chunk `.payload`。

### 19.43 游戏内南北反向的根因：高度场行序 ≠ 游戏世界 y（T1.6.7 修）
**现象**：游戏里整幅地形南北颠倒（海跑到北边），而 `terrain_preview` / `preview.png` 看着是对的。

**根因**（`tools/map_gen.py`）：
```python
# 错：
def cell_to_world(cx, cy):
    return ((cx - BORDER) * CELL, (cy - BORDER) * CELL)     # 行号当世界 y 原样写出
```
引擎的地图原点在**左下角**、世界 y 向上，所以**高度场第 0 行会被画在屏幕最下方**。
而内部数据（以及 v11 蓝图、用户标注图）**第 0 行 = 最北**。把行号原样当世界 y 写出去，
游戏里整幅地形就上下翻转了。

**怎么定位的**（`work/ordertest.py`，一次见分晓）：
- `.map` 的 `HeightMapData` 与 `terrain.npz` 的 `raw` **逐字节相同**（海都在行 789~1039）
  -> 排除"写文件时被翻"
- 用出生点当锚点：`row = y/10 + BORDER` 算出来的 z 与设计值**8/8 完全吻合**
  （P8→190 水、P5→350、P1→420…）-> 说明"对象 ↔ 高度场"在**文件内部是自洽的**，
  错的是**两者一起相对屏幕反了**
- 结论：要在**写 .map 时**把两者一起换算到游戏帧

**修法**（两处必须同时改，否则对象与地形错位）：
```python
def cell_to_world(cx, cy):                                  # 对象/waypoint
    return ((cx - BORDER) * CELL, ((GRID - 1 - BORDER) - cy) * CELL)

def build_heightmap_payload(t):                             # 高度场
    w += np.flipud(t.raw).astype('<u2').tobytes()
```
另加 `world_to_cell_y(wy)` 作逆换算（`map_gen` 里"矿区清场"那段需要把世界 y 转回行号）。

**验收**（`work/verify_game_frame.py`）：
- 高度场 = `flipud(raw)` ✓（行序已换算到游戏帧）
- 对象世界 y 逆换算回内部行后，**8/8 个出生点与设计格偏差 0** ✓

**⚠ 连带约定（别再搞混）**：
- `terrain.npz` / v11 蓝图 / 用户标注图：**第 0 行 = 最北**（内部统一口径，所有 `work/*.py` 都按这个）
- `.map` 文件里：高度场第 0 行与对象世界 y 都已经是**游戏帧（第 0 行 = 最南）**
- `preview.png` 画的是内部数据，所以图上**北在下、海在上 —— 这才是游戏里的样子**；
  预览标题里已加"（本图与游戏内一致：北在下、海在上）"的标注

### 19.42 矿位分级（基地旁只留 2 个）+ 中部设施（T1.6.6）
**要求**：① 基地旁边**只允许 2 个矿**，其余矿离基地远一点、可以往**中间或下一层**摆；
② 地图中部**分散**放：2 个哨站 + 1 个车库 + 1 个精兵学院 + 1 个医院 + 1 个机场。

**矿位分级做法**（`resolve_slots` 的矿位段，配置 `ore_near_max` / `ore_far_min_dist` 等）：
- 每基地 4 个矿里，`_k < ore_near_max(2)` 的当**近矿**（扇区半径 = 建造范围 45）；
- 其余当**远矿**：先把目标点朝地图中心拉 `ore_far_pull_mid(0.45)`，再用
  `sector=(基地, ore_far_min_dist(75), 170, ±π/3.2)` 找落点，掩码半径放宽到 180；
- 远矿仍找不到就退到 `ore_far_fallback(55)`。
- 实测 T1.6.6：**每基地 55 格内恰好 2 个**，其余矿在 92~200 格外。

**中部设施做法**：配方里用**格坐标**指定（`center_posts` / `center_garage` /
`center_academy` / `center_hospital` / `center_airport`），`_center_list()` 会在地形定稿后
`_res()` 吸附到附近最平整处。给了 `center_*` 就**不再叠加**旧的经纬度常量位置
（否则哨站会变成 4 个：2 个中部 + 2 个旧的）。

实测 T1.6.6 中部：哨站 (430,560)/(686,457)、车库 (560,360)、学院 (377,700)、
医院 (773,649)、机场 (600,480) —— 全部落在 x/y ∈ [350,780] 的中部区域内。

**⚠ 配置读取的坑**：`terrain.npz` 里的键是**单数**`post_cells / garage_cells /
airport_cells / hospital_cells / academy_cells`，而 `slots` 字典里的键是**复数**
`posts / garage / airports / hospitals / academies` —— 读核验脚本时别搞混（我因此误判过一次"只写出车库"）。

### 19.41 油田（油井）分布：只在地级市 + 每个出生点各 1 个（T1.6.4）
**要求**：油田**只放在地级市**，每个城市分散 1 个；另外**玩家附近每人 1 个**。

**做法**（`terrain_gen.resolve_slots` 里的 derrick 段）：
1. 城市油田：遍历 `city_centers()`，但**只保留 `CITY_SEATS` 里的 14 个广西地级市**
   （`city_centers()` 还会返回邻省湛江/茂名/阳江/云浮和 3 个自治州，这些**不算**，
   另外还会带上 `自治州/自治县` 过滤）；落在水域的城市跳过（本图永州在水里）。
2. 出生点油田：每个 `BASES` 项各放 1 个，优先陆地可建地；
   **水上出生点**（P8 在湖里）改成把油井放在水面。

**实测 T1.6.4：油田 22 个 = 城市 14 + 出生点 8**；14/14 城市有油田（≤60 格），
8/8 出生点有油田（P1~P7 各 30 格、P8 水面 25 格）。

**踩过的两个坑**：
- `resolve()` 是"**从中心按环从 r=0 向外扫**"。把基地中心当搜索中心时，它永远先撞到
  **基地自己那一格**，而那格又被 `base_list` 的净空判据挡掉 → 直接返回 None。
  正确做法：**从离基地 25 格的若干候选方向点起搜**（`bx + cos(a)*25`）。
- 前几级 `resolve` 在陆地掩码上可能找到**离基地 70+ 格**的远处位置（近处被矿的间距占满）。
  水上出生点要**先试水面、再和陆地结果比距离**，取近的那个，否则"玩家附近 1 个"会落空。

### 19.40 四张预览图 + art.tga 的行序必须统一（本轮修的 5 处）
> ⚠ **本节为 T1.5.9 时代的历史记录**，其"统一基准/一律不 flipud"等结论已被后续重构推翻；
> 现行口径见 §19.46 + §19.47（内部帧行 0 = 最北，预览统一 flipud 到游戏帧）。
**统一基准：行 0 = 最北，海在画面下方。** 判据脚本 `work/check_previews_axis.py`
（比较每张图"上半海% / 下半海%"），T1.5.9 实测四张全部 `海南(下半) ✓`：

| 文件 | 生成处 | 原来错在哪 | 修法 |
|---|---|---|---|
| `preview.png` | `make_preview.py` | `np.flipud` 翻地形；且物件坐标 `(GRID-1)-cy` 又翻一次 | 两处都去掉 |
| `terrain_preview.png` | `terrain_gen.py` | `np.flipud` 翻地形 + 出生点 `(GRID-1)-cy` | 两处都去掉 |
| `build_view.png` | `verify_map.py` | 物件与 waypoint 标签 `(GRID-1)-cy` | 去掉 |
| `art_preview.png` | `verify_map.py` | 直接存 art.tga（自底向上存储）→ 看起来上下反 | 存之前 `transpose(FLIP_TOP_BOTTOM)` |
| `*_art.tga` 本体 | `map_gen.py` | 写文件时多做 `arr_img[::-1]` | 去掉（见下） |

**art.tga 的关键**：它与高度场**行序必须一致**。高度场
（`map_gen.build_heightmap_payload`）是**按行原样写出**（行 0 先写）的，
所以 art 也按行原样写，再配合 TGA 自底向上的存储约定即可。
判据（`work/axis_art.py`，比较 art 的陆/海与高度场陆/海的 IoU）：
- 旧代码（带 `[::-1]`）：**0.790**  ← 给游戏看的方向反了 180°
- 修正后：**0.996** ✓

**世界 y 与高度场行的关系（用出生点实测，`work/axis_world_y.py`）**：
`row = y/10 + BORDER`，**直接对应、不翻转**。证据：水上出生的 P8 世界 y=8340
→ row 854 → z=190（水）✓；P4 y=8490 → row 869 → z=212 ✓，8 个全部吻合设计意图。
（旧代码在预览里翻物件坐标，属于同一次轴向误解的残留。）

**⚠ 另一处独立的反向（不要顺手去"修"）**：`Geo.make_mask()` 出来的省界掩码数组是
**行 0 = 最北**没错，但它与**本地图**并不同向（实测 art vs 掩码 IoU 翻 0.637 / 不翻 0.818）。
掩码只用于"哪里是广西省界内"的粗判定，**不要**为了让它和预览对齐而翻转它。

### 19.39 轴向（行 0 = 最北）—— 这个坑踩了三轮，务必按实测结论走
> ⚠ **本节为历史记录**，其中"预览一律不 flipud"等结论已被推翻；
> 现行口径见 §19.46 + §19.47（T1.8.7 最终实锤）。
**结论（2024 复核实机确认）：数据第 0 行 = 最北，行号越大越往南；水域在画面下方。**
v11 蓝图、用户标注图、游戏显示**三者同向**，所以：
- `work/make_v11_layers.py` 导出时**不翻转**（`V11_FLIP` 默认 0，npz 里 `flip_v=0`）
- `TERRAIN['v11_flip_v'] = False`，`build_terrain` 载入后**不再翻转**
- 所有预览/校验渲染（`make_preview.py`、`verify_map.py`、`map_gen.py`、`check_passability.py`）
  **一律不 flipud**
- 标注图坐标 → 数据坐标：`x*1040/1124`、`y*1040/1124`
  （**只有这一处缩放**，不要再乘 `900/…`，多乘一次整体错位）

**三条独立证据（合起来定死方向）**：
1. 用户实机答复"**海在画面下方**"（我上一版把海放到了上方 → 多翻了那一次）。
2. 标注里**东侧 4 个 + 西侧 4 个、各自自北向南**；按"不翻转"换算后，
   东侧 P5→P8 的蓝图层高依次 350 / 280 / 212 / **190(水)** —— 越往南越低、最南是水，完全吻合。
3. 用"**四角红色占比**"判圆/矩形：圆 0.00~0.01、矩形 0.84~0.93，中间无重叠 → 8 圆 + 18 矩形。

**踩过的三个坑**：
- `terrain_gen.Geo.to_cell` 的旧注释写着"游戏网格原点在左下角：第 0 行 = 最南"——
  **这条注释是错的**。照它把地形 flipud 一次，"我分析的图"与"用户标注的图"就永远上下颠倒。已改正。
- `ImageDraw.polygon` 用 y 向下坐标，所以 `Geo.make_mask()` 出来的数组本身就是"行 0 = 最北"，
  与城市经纬度换算自洽 —— 不要为了"对齐"再去翻掩码。
- 预览脚本里的 `np.flipud` 会翻**地形**但不翻后画的标记，于是标记看着对、底图是反的
  —— 这是最难发现的一种错位。

### 19.38 层界默认不可通行 + 只在标注矩形处开斜坡（T1.5.5~T1.5.7）
**要求**：不同层之间的边界**不允许通行**，只允许在用户标注的矩形（斜坡）处上下；
圆圈是出生点，矩形是斜坡。层级连接多的地方多开几个斜坡，至少一个。

**做法**（`TERRAIN['use_v11_layers']` + `maps/bagui_dadi/ramps.json`）：
1. `quantize_strict_layers(z)`：把整幅高度场**压成严格分层**——每格归到最近的合法层高
   （海/水一律 190），层与层之间是 70 单位的硬坎（坡度 70 单位/格，远超可通行上限）。
2. `carve_marked_ramps(z, ramps)`：**只在标注矩形处**挖坡（**T1.9.1 重做为车道式**）：
   - 坡道剖面用纯 **smoothstep**（lo→hi，端点导数 0），不采样地形；
   - 掩码 = **车道内权重恒 1**（|u|≤半长 且 |v|≤半宽），只在车道外做 4 格羽毛过渡；
     剖面在车道两端之外截断在台面高度 -> 端缝为 0；
   - ⚠ 旧版"采样剖面 + 各向异性高斯权重"的坑：高斯在坡道端点权重只有 ~0.6，
     混入 40% 旧硬坎 -> 端缝 30~90、沿坡单格落差最高 56（`work/check_ramps.py` 可复测）。
     **坡道内必须权重 1**，让剖面精确生效；
   - 沿坡最大单格落差 <= `ramp_max_step`（默认 2.5，T1.9.3）：按 `half_u >= 0.75*Δ层/step`
     反推最小坡长，短标注框的坡道会**顺坡线向两侧台面延长引道**（不会在别处开新口子，
     层界锁定仍在 99%+）；
   - 坡面宽高严格按标注框（别放大，否则会把大片地形拉平；第一版把 su 写成
     `max(hw,hh)*1.05`，结果把 40 格宽的台面拉平成了斜坡）。
3. **绝不做全图限坡**：限坡会把坡面之外的层界也削成缓坡 = 到处都是通路
   （实测违规通路从 9k 格涨到 65k 格）。
4. **视觉（T1.9.1）**：`build_blendtiles_payload` 把每条斜坡车道刷成
   `Dirt_Yucatan06` 土路 + 外圈 8 格 `Gravel_Yucatan01` 过渡（8×8 块内占比 ≥25% 才换），
   玩家一眼能看出"这里能上下层"。
   ⚠ 验收纹理时直接读 chunk `.payload` 解析 tiles —— `parse_blendtiles` 返回的
   tiles 起点不同，读出来的纹理名不可信（踩过：误以为没刷上，实际 .map 是对的）。

**核对口径**：`work/check_layer_lock.py` —— 统计"层间过渡带上的可走格"有多少落在标注框内。
`work/check_ramps.py` —— 逐条斜坡的剖面平滑度（沿坡/端缝/横向单格落差）+ 车道纹理（T1.9.1 起）。
**T1.5.6 结果：99.1% 在框内**（212→280 是 18846/18848），可走主域 875,929 格，
7 个陆地出生点全在主域，2 个水上出生点走登陆堤道。

**踩过的坑**：
- 只比较"相邻两格层号不同"来判定层界会**完全失效**：`smooth()` 把硬坎软化成过渡带后，
  相邻格层号往往相同，实测层界从 11302 格掉到 **11 格**。必须用"窗口内层号极差"判定，
  且要把 -1（无层号的格）先填成最近的有效层号，否则极值被 -1 污染、`span` 恒为负。
- 出生点所在层若没有斜坡接到主陆，就会变成孤岛：`work/add_ramp_for_isolated.py`
  会自动为这类出生点补一条斜坡（T1.5.6 给 P8 的 420 高台补了一条 350→420）。
- 斜坡记录的 `x/y` 是**数据格坐标**，由 `work/export_ramps.py` 从标注图换算
  （标注图 1124px → v11 900px → 数据 1040px，**两处都要缩放**，漏一处就整体错位）。

