> 此文保留迁移前的设计记录；其中 Python 工具路径已停用。当前命令与数据格式见 [Rust 迁移说明](rust_migration.md)。

# World Builder 解析结论与对接方案

> 勘察对象：`D:\app\Ra3NewWorldBuilder`（新地编 / 物版地编，基于 EA 官方 RA3 地编改造）
> 结论：**既能读取制作规则，也能程序化操作**。三条对接路径，按省力程度排序见 §3。

---

## 1. 目录与关键文件

| 位置 | 内容 | 价值 |
|---|---|---|
| `说明.txt` | 版本说明（v14~v21） | 功能清单、地图尺寸范围 10~2000、自动保存路径 |
| `bin/WorldBuilder_Mod_1.12.exe` | 地编本体 | GUI |
| `bin/MapMixer.exe` + `MapCoreLib.dll` | 地图加密/核心库 | `_mix` 加密地图 |
| `data/scripts/*/Source/*.cs` | **地编 C# 插件源码** | 插件 API（改高度、批量放物体…） |
| `data/RA3_MapLib_AI_Guide.md` | **官方 AI 使用指南** | 地图库 C# API 全貌 |
| `data/objectsTemplate/*.object.bin` | 美化模板 | 可复用的物件组 |
| `data/objectScreenShot/*.jpg` | 3000+ 物件截图 | 查权威对象名 |
| `data/locale/`, `data/config/` | 汉化与配置 | 中文对象名 |
| `maps/CAMP_A01_*` | 官方战役样例图 | 参考 |
| `地编伴侣/current/Ra3MapUtils.exe` | **地编伴侣（带 HTTP API）** | 程序化操作入口 |
| `地编伴侣/current/Dreamness.RA3.Map.*.xml` | Facade / Lua / Parser / Transform 库文档 | 地图读写库 |
| `地编伴侣/current/data/nano_programs/*/Main.cs` | **微程序（脚本化改图）** | 可自定义、可传参 |
| `日冕地图lua例子/` | Lua 脚本地图例子 + `map.xml` | 游戏的 XML 地图格式 |

---

## 2. 能读到的"制作规则"

### 2.1 高度约定（与我们的项目一致）
来自 `data/scripts/CircleTerrain/Source/Main.cs`：

```csharp
heightMapData.elevations[j, i] = 210;   // 平原
heightMapData.elevations[j, i] = 130;   // 水面
```
- **平原 = 210**、**水面 = 130** —— 与我们项目 `LAND_Z = 210` / 海面 200 的体系一致，
  印证了"平原垂直高度 0"的正确解读（相对平原层的落差为 0）。
- 数组索引是 `[x, y]`（第一维 x），与我脚本里 `z[y, x]` 的写法相反，注意别混。

### 2.2 地图尺寸
- 地编允许 **10 ~ 2000**（`说明.txt`：地图创建大小范围修改，目前范围是 10~2000）。
- 但**游戏可跑上限仍是可玩区 1000**（我们扫过的 465 张社区图 + 实测 1080 崩溃结论不变）。
  地编能建 2000 不等于游戏能跑。

### 2.3 坐标体系（官方 AI 指南 §1.2）
- **网格坐标 Grid**：整数、原点左下角，用于地形（高度/通行性/纹理）。
- **世界坐标 World**：浮点，用于单位/路径点。
- 两者不可混用（我们对象摆放用 `(cell - BORDER) * 10` 的换算与之吻合）。

### 2.4 AI 玩家命名与散矿开关
- AI 玩家命名惯例：`AIPlayer01` … `AIPlayer06`（`COR_ADD_AI_PLAYERS`）。
- **散矿是一个任务目标开关**：MissionObjective id = `ForceLooseOre1`（强制开）/ `ForceLooseOre0`（强制关），
  针对日冕 mod（`COR_FORCE_LOOSE_ORE`）。这解释了我们"矿区贴山脚/散矿"需求背后的机制。
- 纹理图鉴（在线）：<https://zybdatasupport.online/terrain>

### 2.5 编辑器插件 API（`data/scripts`）
```csharp
using MapCoreLib.Core.Asset;
public class X : ScriptInterface {
    public void Apply(MapDataContext context) {
        var hm = context.getAsset<HeightMapData>(Ra3MapConst.ASSET_HeightMapData);
        hm.elevations[x, y] = 210;
        var objs = context.getAsset<ObjectsList>(Ra3MapConst.ASSET_ObjectsList);
        objs.AddObject(context, "OreNode", new Vec3D(x * 10, y * 10), 45);  // 格 -> 世界 ×10，角度 45
    }
}
```
- 已有范例：`ImportXYZ`（导入高度图）、`loadObjFromFiles`（批量放物体）、`CircleTerrain`、
  `RandomAddTrees`、`TerrainLevelUp`、`TerrainNoiseOver`、`OnlyInfantry`（**程序化写脚本**）。
- `OnlyInfantry` 展示了脚本 API：`new Script{ Name, scriptOrConditions = { new OrCondition{ conditions = { ScriptCondition.of(context, "CONDITION_TRUE") } } } }`。

---

## 3. 操作它的三条路径（按省力排序）

### 路径 A：数据交换文件（最省事，零编程）
地编内置两个"一键导入"插件，接受**纯文本**：

| 插件 | 文件 | 格式 |
|---|---|---|
| `ImportXYZ` | `heightmap.xyz` | 每行 `x y z`（格坐标，z 为高度值，平原 210） |
| `loadObjFromFiles` | `object.txt` | 每行 `类型名,格x,格y`（内部 ×10 转世界坐标，角度 45） |

**可做的事**：把本项目生成的地形导出为 `heightmap.xyz`、把物体导出为 `object.txt`，
放进对应插件目录，在已打开的地图上点一下即可灌入 → **我们的地图立刻能在地编里可视化编辑**。

### 路径 B：微程序 / 插件（可自动化，无需 GUI 交互）
- **微程序**：`地编伴侣/data/nano_programs/<名字>/{Main.cs, info.json}`，
  由伴侣执行，支持 `ArgumentDictionary["MapFilePath"]` 传参 → **可无界面批处理**。
- **地编插件**：`地编/data/plugins/<名字>/{Main.cs, plugin_meta.json}`，在地编里调用。
- 脚本能力（见 `Dreamness.RA3.Map.Facade`）：`Open/Save/SaveAs`、`GetTerrainHeight/SetTerrainHeight`、
  `GetTileTexture/SetTileTexture`、`RegisterSupportedTexture`、`AutoDetectBlendsEntireMap`、
  `GetPassability/SetPassability/UpdatePassabilityMap`、`AddUnitObject`、`AddWaypoint/AddPlayerStartWaypoint`、
  `AddPlayer/GetPlayers/ExportPlayersToJsonStr/ImportPlayersFromJsonStr`、
  `AddTeam/GetTeams/ExportTeamsToJsonStr/ImportTeamsFromJsonStr`、`AddMissionObjective`、`Backup`。

### 路径 C：HTTP API（伴侣已启用，端口 17289）
伴侣配置（`%APPDATA%\Ra3MapUtils\Ra3MapUtils.db` → `program_settings`）：
```
ApiService_IsApiServiceEnabled = True
ApiService_ApiServicePort      = 17289
```
- 端点（Swagger）：`CSharpScriptAPIService.Run / RunFile`、`LuaImportAPIService.ExportScheme / ImportScheme`、
  `NanoProgramAPIService.ExecuteNanoProgram(name, args)`、`ImageEncodingAPIService`、`StatusAPIService`。
- **前提**：必须先启动 `地编伴侣`（GUI 程序）；当前未运行，故端口不可达。
- 另有 MCP 服务 `Ra3MapUtils.MCP.CSharpScriptService`（自动加载程序集 + XMLDoc，供 AI 发现 API）。

---

## 4. 对本项目的直接价值

1. **地图可继续在地编里编辑**：路径 A 即可把我们的地形/物体灌进地编（强烈建议先做，用于人工微调）。
2. **替代我手写的两处**：
   - 纹理混合：`AutoDetectBlendsEntireMap()` 是官方的自动混合，比我按公式算更稳；
   - 通行性：`UpdatePassabilityMap()` 由库统一计算，省掉我的斜率/边界规则。
3. **可能解决"固定 AI 与南队同盟"**：`GetTeams/AddTeam` + `AiType`/`InitialAggressiveness` 字段，
   以及 `ImportTeamsFromJsonStr` 可直接改写队伍关系（此前我用脚本块逆向没做成）。
4. **散矿开关**：需要时可加 MissionObjective `ForceLooseOre1`。
5. 我们已实现的高度分层（210/280/350）与地编"平原 210"约定天然一致，导入不会错层。

---

## 5. 注意事项

- 地编允许 2000 尺寸，但**游戏上限仍是可玩区 1000**，别被编辑器误导。
- 数组索引 `[x, y]` vs `[y, x]`：插件里是 `elevations[x, y]`。
- 伴侣 API 依赖 GUI 程序运行；批处理请走微程序（带 `MapFilePath` 参数）。
- `data/scripts/*/readme.txt`、部分文档是 **GBK** 编码，按 UTF-8 读会乱码。
- 另发现第三方地编副本：`D:\BaiduNetdiskDownload\RA3工具\Ra3MapEditor(第三方地编)\Ra3MapEditor\`
  （`Ra3MapEditor.exe` + `Ra3CoronaMapLuaLib`）——可能是同源的另一版本/更早版本，需要时再核对。

---

## 6. 已落地的整合（本仓库）

### 6.1 `tools/wb_export.py`（新增，已并入流水线）

```powershell
& $py tools\build_all.py --map bagui_dadi --install --wb   # 构建 + 安装游戏 + 导出并装进地编
& $py tools\build_all.py --only wb                          # 只做导出
```
产出到 `assets\<地图名>\<版本>\wb\`：
| 文件 | 用途 |
|---|---|
| `heightmap.xyz` | 给地编内置插件 `ImportXYZ`（每行「格x 格y 世界高度」，**全图格坐标**，1040×1040） |
| `object.txt` | 给地编内置插件 `loadObjFromFiles`（每行「类型名,可玩区x,可玩区y」，角度固定 45°） |
| `chai_objects.txt` | 给自带插件（每行「类型名,世界x,世界y,角度°」） |
| `plugins/CHAI_Import/` | **自带地编插件**：精确导入对象并保留矿脉 45/135/225/315 朝向 |
| `nanoprogram/CHAI_OptimizeMap/` | **地编伴侣微程序**：调官方库做自动地形混合 + 通行性重算 |
| `README_导入说明.md` | 导入步骤、坐标口径、出生点格坐标清单 |

`--wb` 会把上述文件复制进 `D:\app\Ra3NewWorldBuilder`：
`data/scripts/ImportXYZ/`、`data/scripts/loadObjFromFiles/`、`data/plugins/CHAI_Import/`、
`地编伴侣/current/data/nano_programs/CHAI_OptimizeMap/`。

### 6.2 坐标口径（已用官方样例图核对，写进 README 免得搞反）
- **高度图**：`elevations[x, y]` 用**全图格坐标**，值 = **世界高度**（平原 210 / 一层 280 / 二层 350 / 海面 ≈130）。
- **对象**：世界坐标 = `(格 − 边框) × 10`；内置对象插件会对文件值再 ×10，
  所以 `object.txt` 必须用**可玩区相对格坐标**（0..1000），否则整体偏 200 单位。

### 6.3 为什么用「微程序」而不是自己写混合
实测本图 `blends` 数组**全为 0**（0 个地形混合）→ 所有纹理交界是硬边。
官方库有 `AutoDetectBlendsEntireMap()`，其混合描述含 `secondary_tile` + 4 维 `direction`，
语义未完全公开；硬猜方向可能画出错误斑块。因此交给官方库执行（`CHAI_OptimizeMap`），
它同时会 `UpdatePassabilityMap()` 用官方逻辑重算通行性，并 `Backup()` 备份。

**运行方式**：启动 `地编伴侣` → 微程序 → `CHAI 地图优化(混合+通行性)` → 选本项目的 .map。
（伴侣的 HTTP API 在 17289 端口，启动后我也可以直接调它批量跑。）

