---
title: "LuaAPI-LuaBridge"
catalog_path: "Lua API "
slug: rg4gm1ibq054y6o4
source: https://www.yuque.com/muzeqaq/ra3mapwiki/rg4gm1ibq054y6o4
fetched: 2026-10-15
content_updated_at: "2026-04-19T11:08:02.000Z"
type: Doc
format: lake
---

# LuaAPI-LuaBridge

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/rg4gm1ibq054y6o4
> 目录位置：Lua API 

:::info
作者：岚依公主

日期：2024年12月14日

可读性与格式化：Inertia

更新日期1：2024年12月20日

更新日期2：2025年8月30日（第39和40条）

更新日期3：2025年10月24日

:::

日冕V3.22x新增的lua功能由两个dll提供，分别是[EnhancerCorona](https://www.yuque.com/muzeqaq/ra3mapwiki/eksx1whrfix7tlno)和LuaBridge（本文内容）。

V3.22x版的LuaBridge相比[（日冕V3.215)日冕Lua.lua](https://www.yuque.com/muzeqaq/ra3mapwiki/qu7m8g9b8zft1i2b) 中有的第0-38条，新增了第39-42条。

为0-38条中少量函数提供了更贴切功能的新名字（旧名字仍然可用且将永远可用）。

第27条提到的Weapon的值，见[Weapon清单(适用于地图使用)](https://www.yuque.com/muzeqaq/ra3mapwiki/yipsr7vgtxahdk7n)

第32和33条提到的RegisteredObjectSetId的值，见[RegisteredObjectSetId 清单](https://www.yuque.com/muzeqaq/ra3mapwiki/nx856npyys4vtp53)

第38条提到的AttributeModifier的值，见[单项AttributeModifier清单](https://www.yuque.com/muzeqaq/ra3mapwiki/chqogqlxe9pzb0k1) 和 [组合式AttributeModifier清单](https://www.yuque.com/muzeqaq/ra3mapwiki/wpz5fwgb181qgy4r#6edd)

> 日冕V3.220更新日期为2024年12月14日
>

## 目录
```lua
-- 此文件不会被调用，仅作为示例用途
if 1 then return end

[[
 I. 日冕客户端提供的额外 Lua 功能。
    当前 LuaBridge 版本：2024/11/16
    假如你是在 2024/11/16 之前下载的 LuaBridge，那么你需要重新下载一份最新版本。
    目录：
     0. 引擎修复
     4. 获取单位所属的玩家的“地编脚本名称”，也就是 Player_1 这种
     5. 哈希相关
     6. 创建过滤器
     7. 测试单位是否符合过滤器
     8. 找到某个区域内所有符合 ObjectFilter 的单位（无论单位是否有 Lua 模块都能被找到）
        也可以找到地图上所有符合过滤器的单位，或者获取地图上的所有单位
     9. 获取，或者设置单位坐标
    10. 计算两个单位之间的距离
    11. 获取，或者设置，包含单位的坐标、旋转和缩放的矩阵
    12. 获取单位的数字 ID（比手动解析 ObjectDescription 更快也更方便哟！）
    13. 获取单位血量。与只能得到百分比的地编脚本不同，这些函数可以得到精确的血量
    14. 操作矿脉，例如获取以及设置开采的矿石数量
    15. 读取以及设置将来的下一个建造面板的序号
    16. 设置 SCRAPE 渲染脚本变量，来和 Shader 这些与渲染有关的东西交互
    17. 调用 Apt 的 ActionScript 函数
    18. 检测两个单位的 table 是否是同一个单位
    19. 根据数字 ID 获取一个单位的 table
    20. 根据地编脚本命名获取一个单位的 table
    21. AttachUpdate 相关函数，获取被贴的单位以及贴人的单位
    22. Contain 相关函数，获取包含本身的单位，以及该单位包含的乘客列表
    23. 获取建造序列信息
    24. 获取以及修改威胁度信息（左下角威胁指示器那个）
    25. 获取单位在前一帧的坐标（便于计算移动速度）
    26. 引爆抛射体
    27. 向任意目标发射任意武器
    28. 获取单位目标（另见 40. 额外的获取单位目标以及设置目标相关的函数）
    29. 获取单位的建造者（生产者）
    30. 获取单位造价
    31. 获取玩家所有单位的造价
    32. 检测单位是否属于某种类型
    33. 获取玩家所有某一类型的单位
    34. 获取以及设置单位武器的弹药量
    35. 获取单位武器的当前 WeaponLaunchBone 序号
    36. 检测单位是否被瘫痪
    37. 检测单位是否存活
    38. AttributeModifier 相关函数，获取，添加，删除各种 buff / debuff
    40. 额外的获取单位目标以及设置目标相关的函数
    41. 性能相关
    42. 虚拟伤害（用于避免火力溢出）
	  43. 自定义存档支持
    44. 本地化 / 翻译文本
    45. 获取下一个单位 ID
	  46. 地编队伍调试信息（针对日冕 AI 在自定义地图的闪退问题）
    47. 切换单位 LocomotorSet
    48. 获取单位的 InstanceId（增援代码）
    49. 贝塞尔抛射体修改
  以下实例中，self 和 other 都是指代表单位的 table。
]]
```

## 日冕V3.215
### 第0-11条
```lua
[[0]]
-- 引擎修复
-- 2023/07/02 版本的 LuaBridge 修复了一些引擎的 bug
-- 例如，现在可以直接把单位的 table 传入 EvaluateConditon 而无需命名。
-- 此外，Comparison 类型的参数可以直接传字符串，而不再需要传数字或是借助 CompareTable。
-- 例如：
if EvaluateCondition("UNIT_HEALTH", self, ">", 50) then
  -- ...
end
-- 该版本还修复了 PositionCoordinate 类型的参数，现在可以直接以 table 的形式传入。
-- 例如：
ExecuteAction("UNIT_SPAWN_NAMED_LOCATION_ORIENTATION",
  "NAME",
  "JapanAntiVehicleInfantryTech3",
  "Player_1/teamPlayer_1",
  { X=500, Y=200, Z=300 }, -- 位置
  30 -- 朝向
)

[[4]]
-- 获取单位所属的玩家的“地编脚本名称”，也就是 Player_1 这种
ObjectPlayerScriptName(self) == "Player_1"
ObjectPlayerScriptName(other) == "Player_1"
-- 假如事先调用过 SetWorldBuilderThisPlayer，那么也可以传入字符串：
SetWorldBuilderThisPlayer(self) -- 必须设置 this player 之后才能使用字符串
ObjectPlayerScriptName("Player_1") == "Player_1"
ObjectPlayerScriptName("<This Player>") == ObjectPlayerScriptName(self)
ObjectPlayerScriptName("<1st Human Player>") == "Player_1"
-- 注意：虽然从名称上来看是玩家所有的敌人（复数），
-- 但是实际上只会返回第一个敌人的名称
ObjectPlayerScriptName("<1st Human Player's Enemies>") == "Player_2"

[[5]]
-- 哈希相关
HexToUInt32("0x683C7936") == 2739059666 -- 从 16 进制字符串转换到数字
FastHash("CelestialAttackerAircraftFireProjectile") == 2739059666 -- 从字符串计算哈希数字

[[6]]
-- 创建过滤器
-- 用法和 XML 里的 ObjectFilter 一致
filter = CreateObjectFilter({
  Rule="ANY",
  Relationship="ENEMIES NEUTRAL ALLIES",
  Include="SMALL_MISSILE PROJECTILE",
  StatusBitFlagsExclude="IGNORING_POWER_DOWN",
  IncludeThing={
    "SovietAntiNavyShipTech2TorpedoProjectile",
    FastHash("JapanInterceptorAircraft")
  },
  ExcludeThing={
    "CelestialAttackerAircraftFireProjectile"
    HexToUInt32("0x683C7936"),
  }
})
-- 虽然可以随时创建任意数量的过滤器，
-- 但是还是建议提前创建好过滤器，然后重复使用已经创建好的过滤器
-- 而不是每次都重新创建相同的过滤器

[[7]]
-- 测试单位是否符合过滤器
if (ObjectTestTargetObjectWithFilter(self, target, filter)) then
  -- ...
else
  -- ...
end

[[8]]
-- 找到某个区域内所有符合 ObjectFilter 的单位
-- 无论单位是否有 Lua 模块，都能被找到，但需要符合以下条件：
-- 1. 单位 KindOf 不能包含 INERT
-- 2. 单位 KindOf 不能包含 OPTIMIZED_PROP
-- 3. 可能还有其他尚未发现的限制，但总之，一般意义上的“单位”都能被找到
-- 因此，诸如悬崖、鸟叫声音效之类的单位，可能是无法在区域搜索内被找到的
local x, y, z = ObjectGetPosition(self)
-- 需要使用最新（指 2023/05/13 之后的）LuaBridge 日冕客户端，
-- 才能同时返回 table 和 count
local matchedObjects, count = ObjectFindObjects(self, {
  X=x, Y=y, Z=z, Radius=200, DistType="CENTER_2D"
}, filter)
-- 这里使用“数字形式”的 for 循环，
-- 而不是“迭代器形式”的 for 循环（for key, value in table do）
-- 来确保循环的顺序和 ObjectFindObjects 返回的顺序一致
-- 否则有可能出现不同步
for i = 1, count, 1 do
  local current = matchedObjects[i]
  -- ...
end
-- 找到某个区域内的所有符合上述条件的单位（无论单位是否有 Lua 模块都能被找到）
local x, y, z = ObjectGetPosition(self)
local matchedObjects = ObjectFindObjects(nil, {
  X=x, Y=y, Z=z, Radius=200, DistType="CENTER_2D"
}, nil)
for i,obj in matchedObjects do
  -- ...
end
-- 找到地图上所有符合过滤器的单位
-- 但是，全图搜索较慢。假如条件允许，
-- 建议使用 [[33]] 里的 CopyGlobalRegisteredObjectSet 或者 CopyPlayerRegisteredObjectSet，
-- [[33]] 里的函数会更快一些
-- 假如过滤器包含 Relationship, 判断关系时以 self 为参照
-- 假如过滤器不考虑 Relationship, self 也可以是 nil
local matchedObjects, count = ObjectFindObjects(self, nil, filter)
for i = 1, count, 1 do
  local current = matchedObjects[i]
  -- ...
end

[[9]]
-- 获取单位坐标
local x, y, z = ObjectGetPosition(self)
-- 假如不需要高度（z 坐标），也可以
local x, y = ObjectGetPosition(other)
-- 设置单位坐标
ObjectSetPosition(self, x, y, z)
-- 假如不需要修改高度（z 坐标），也可以
ObjectSetPosition(other, x, y)

[[10]]
-- 获取单位之间的距离
local distance = ObjectsDistance2D(self, other)
local distance = ObjectsDistance3D(self, other)

[[11]]
-- 获取一个包含单位的坐标、旋转、缩放信息的矩阵
-- 这个矩阵有 12 个元素（table[1]、table[2] …… table[12]）
-- 可以视为一个 4x3 的矩阵：
-- ┏               ┓
-- ┃ m11 m12 m13 X ┃
-- ┃ m21 m22 m23 Y ┃
-- ┃ m31 m32 m33 Z ┃
-- ┗               ┛
-- 在 lua 的 table 里实际上则是
-- local table = { m11, m12, m13, X, m21, m22, m23, Y, m31, m32, m33, Z }
-- 其中 XYZ 是单位的坐标，剩下的则是一个 3x3 矩阵，可以用来代表旋转、缩放、拉伸、扭曲、镜像翻转等，或者是这些操作的组合。
local matrix = ObjectGetTransform(self)
local x, y, z = matrix[4], matrix[8], matrix[12]
-- 获取单位的方向向量：把单位的默认方向向量 { X=1, Y=0, Z=0, W=0 } 乘以旋转矩阵
local direction = {
  X = (matrix[1] * 1 + matrix[2] * 0 + matrix[3] * 0 + matrix[4] * 0),
  Y = (matrix[5] * 1 + matrix[6] * 0 + matrix[7] * 0 + matrix[8] * 0),
  Z = (matrix[9] * 1 + matrix[10] * 0 + matrix[11] * 0 + matrix[12] * 0)
}
-- 简化之后，其实就是
local direction = { X=matrix[1], Y=matrix[5], Z=matrix[9] }
-- 假如无视高度，就能得到 2D 的方向向量
local direction2D = { X=matrix[1], Y=matrix[5] }
-- 假如需要计算 2D 角度，可以使用 atan2 函数
local angle = atan2(direction2D.Y, direction2D.X)
-- 3D 角度（俯仰、偏航、滚转）的计算方法则更加复杂，
-- 可以自行在网上搜索 3D Rotation Matrix to Euler Angles，
-- 也就是 3D 旋转矩阵转换到欧拉角的方法。

```



### 第12-23条
```lua

[[12]]
-- 获取单位的数字 ID，这个 ID 是唯一的，不会重复
-- 游戏自带的 ObjectDescription 返回的字符串也会包含这个 ID
-- 但是使用这个函数比使用 ObjectDescription 更快，也更简单
local objectId = ObjectGetId(self)

[[13]]
-- 获取单位的血量
local health = ObjectGetCurrentHealth(self) -- 当前血量
local initialHealth = ObjectGetInitialHealth(self) -- 初始血量
-- 假如单位不存在或者单位没有 ActiveBody 模块，会返回 nil

[[14]]
-- 操作矿脉，例如获取以及设置开采的矿石数量
-- self 需要是一个有 OreNodeBody 模块的单位（例如矿脉）
local maxOreAmount = ObjectGetInitialOreAmount(self) -- 矿脉一开始有多少矿石
local gatheredOreAmount = ObjectGetGatheredOreAmount(self) -- 矿脉已经被开采了多少矿石
ObjectSetGatheredOreAmount(self, maxOreAmount) -- 让矿脉干枯
ObjectSetGatheredOreAmount(self, 0) -- 让矿脉恢复到初始状态
-- 上面让矿脉干枯的操作，假如重复执行，让矿脉满了又干枯，
-- 并不会重新触发“矿脉已枯竭”的 EVA 播报。这是因为 EVA 播报默认只会播放一次。
-- 假如想要重新播报，需要在设置矿石数量之前，重置矿脉的 EVA 播报状态。
ObjectResetOreNodeBehaviorEvaState(self)
ObjectSetGatheredOreAmount(self, maxOreAmount)

[[15]]
-- PDZ 曾经说过，神州建造系统由于建造栏显示的序号有问题，
-- 所以只能使用了一些奇奇怪怪的实现方式。
-- 现在，可以用 lua 来直接修改建造栏的下一个序号了。
-- 已经存在的序号不会被修改，只能修改下一个序号。
-- 调用下面这个函数能够获取 self 所属玩家的建造面板的下一组序号。
local indices = ObjectGetBuildPagesNextIndices(self)
-- 这个 indices 包含所有建造类型（参见 xsd 里的 ProductionQueueType）
indices[1] = 11 -- MAIN_STRUCTURE 
indices[2] = 13 -- OTHER_STRUCTURE
indices[3] = 17 -- INFANTRY
indices[4] = 19 -- VEHICLE
-- 然后，把修改后的 indices 传回去就可以了。
ObjectSetBuildPagesNextIndices(self, indices)

[[16]]
-- 参见 Additional/Shaders/compiled/ingame.scrapeo（可以直接用文本编辑器打开）
-- SCRAPE 渲染脚本里有下面这些命令，可以用来设置一些传给 shader 的参数。
-- setbool SCOPE NAME true
-- setfloat SCOPE NAME 3.1415926
-- 我们现在可以通过 Lua 来实时修改渲染脚本，
-- 让 SCRAPE 脚本里的 setbool SCOPE NAME true 变成 setbool SCOPE NAME false
-- 或者让 setfloat SCOPE NAME 3.1415926 变成 setfloat SCOPE NAME 0 之类的
ScrapeSetBool("SCOPE", "NAME", false)
ScrapeSetFloat("SCOPE", "NAME", 0)
-- 例如，关闭天空盒：
ScrapeSetBool("Lanyi", "UseSkybox", false)

[[17]]
-- 调用 Apt 的 ActionScript 函数。
-- 必须启用配置文件里 RA3AptPlugin 的 customData 里的 newMode 选项，
-- 才能使用这个函数。以后日冕客户端应该会把这个选项默认启用。现在还没有。
-- 需要对 Apt （或者说 flash）有所了解。
-- 第一个参数是 MovieClip 的名字，第二个参数是函数名，后面的参数是函数的参数
-- 所有参数都会被转换成字符串再被传递给 ActionScript 函数。
-- 例如：假设下面这个文件存在： DATA/aptui/hakurei_reimu.manifest
-- 首先让我们创建一个新的空白 MovieClip
InvokeAptFunction("_level0", "createEmptyMovieClip", "reimu", 100)
-- 然后把 hakurei_reimu.swf 加载进去。
InvokeAptFunction("_level0.reimu", "loadMovie", "hakurei_reimu.swf")
-- 接下来我们就可以调用该 Apt 自定义的函数了
InvokeAptFunction("_level0.reimu", "setHp", "100")

-- 可能你没有现成的 Apt 文件来测试上面那个例子，
-- 但你也可以试试下面这个，很好玩的哦！！
InvokeAptFunction("_level0.hud", "removeMovieClip")

[[18]]
-- 检测两个单位的 table 是否是同一个单位
-- 在启用了 LuaBridge 的新选项之后，每次 Lua 事件被调用时，self 都是一个全新的 table
-- 因此无法再使用 self == other 来判断两个单位是否是同一个单位了
-- 但是，可以使用下面这个函数来判断
if ObjectsIsSame(self, other) then
  -- ...
end

[[19]]
-- 根据数字 ID 获取一个单位的 table
-- 假如这个单位不存在，会返回 nil
local object = GetObjectById(objectId)

[[20]]
-- 根据地编脚本命名获取一个单位的 table
-- 可以理解为 SET_UNIT_REFERENCE 的逆操作
-- 假如这个单位不存在，会返回 nil
local object = GetObjectByScriptName("object_name")

[[21]]
-- 返回贴在当前单位身上贴着的单位的列表。
-- 例如，假如 self 是矿车，那么可能返回包含蜻蜓以及信标、钱套子之类的列表
local table, count = ObjectGetAttachers(self)
for i = 1, count, 1 do
  ObjectDescription(table[i])
end
-- 返回贴在当前单位正在贴的单位。
-- 例如，假如 self 是蜻蜓，那么可能返回包含矿车的列表。
-- 一般来说这个列表只会有 0 个或者 1 个单位，除非 self 有多个 AttachUpdate 模块
local table, count = ObjectGetAttachees(self)
for i = 1, count, 1 do
  ObjectDescription(table[i])
end

[[22]]
-- 返回包含当前单位的单位。
-- 例如，假如 self 是子炮台，那么可能返回大船。
-- 假如 self 是标枪兵，那么可能返回屎车。
local container = ObjectGetContainerObject(self)
if container ~= nil then
  ObjectDescription(container)
end
-- 返回当前单位包含的乘客列表。
-- 例如，假如 self 是屎车，那么可能返回包含标枪兵的列表
local table, count = ObjectGetContainedPassengers(self)
for i = 1, count, 1 do
  ObjectDescription(table[i])
end

[[23]]
-- 获取建造序列信息
local productions, count = ObjectGetProductionQueues(self)
-- 一个单位可能有多个建造序列，遍历所有的建造序列
for i = 1, count, 1 do
  local production = productions[i]
  -- Id 是数字序号
  if production.Id == 1 then
    -- ...
  end
  -- ModuleId 是 XML 里模块 ID 的哈希值
  if production.ModuleId == FastHash("ModuleTag_ProductionUpdate") then
    -- ...
  end
  for j = 1, production.QueueLength, 1 do
    local item = production.Queue[j]
    -- item.Type 可能是 "Object" 或者 "Upgrade"
    if item.Type == "Object" then
      -- ...
    elseif item.Type == "Upgrade" then
      -- ...
    end
    -- item.InstanceId 正在建造的单位或者升级的哈希值
    if item.InstanceId == FastHash("AlliedConstructionYard") then
      -- ...
    end
    if item.ProductionPercentage > 50 then
      -- ...
    end
    if (item.ProductionFrames / 15) > 10 then
      -- ...
    end
    if item.BuildCost > 2500 then
      -- ...
    end
  end
end

```

### 第24-29条
```lua

[[24]]
-- 获取以及修改单位所属的玩家的威胁度信息（左下角威胁指示器那个）
local threat = ObjectGetRedAlertCurrentThreatLevel(self)
if threat > 12345 then
  ObjectSetRedAlertCurrentThreatLevel(self, 54321)
end
-- 假如调用过 SetWorldBuilderThisPlayer，那么也可以传入字符串：
SetWorldBuilderThisPlayer(self) -- 必须设置 this player 之后才能使用字符串
local threat = ObjectGetRedAlertCurrentThreatLevel("<This Player>")
if threat > 12345 then
  ObjectSetRedAlertCurrentThreatLevel("<1st Human Player>", 54321)
  -- 注意：虽然从名称上来看是玩家所有的敌人（复数），
  -- 但是实际上只会修改第一个敌人的威胁度
  ObjectSetRedAlertCurrentThreatLevel("<1st Human Player's Enemies>", 54321)
end

[[25]]
-- 获取单位在前一帧的坐标（便于计算速度）
local px, py, pz = ObjectGetPreviousPosition(self)
local x, y, z = ObjectGetPosition(self)
local dx, dy, dz = x - px, y - py, z - pz
local distance = sqrt(dx * dx + dy * dy + dz * dz)
-- 【请注意】单位也有可能根本没有移动，在这种情况下，distance 会是 0
-- 请在做除法之前进行判断，避免除以 0
local speed = distance / 15 -- 15 帧每秒

[[26]]
-- 引爆抛射体。
-- 假如 self 是一个抛射体，那么这个函数会让这个抛射体爆炸，并且造成伤害
-- target 是目标单位
-- 【注意】抛射体必须是由武器发射，或自行触发的（例如蜻蜓本身就是抛射体，它能在特定条件下触发自身）
-- 假如是凭空生成的抛射体，那么这个函数无法将其引爆
ObjectDetonateProjectile(self, target)
-- 假如没有特别指定的目标单位，那么必须传入 nil
ObjectDetonateProjectile(self, nil)

[[27]]
-- 向任意位置发射任意武器
-- X、Y、Z 是目标坐标，三个参数都是必须的，否则武器不会发射
-- target 是目标单位，可选，用于追踪武器
-- *****注意，如果选择追踪武器：***********************************************************
-- *****1. 目标单位必须是敌方单位，否则会出现可追踪武器不追踪的bug***************************
-- *****2. 目标单位必须满足可追踪武器的条件才会发射，比如苏霍伊的导弹只有天狗在空中才会发射*****
ObjectCreateAndFireTempWeaponToTarget(self, "WeaponTemplateName", {
  X=x, Y=y, Z=z
}, target)
-- 假如没有特别指定的目标单位，那么必须传入 nil
ObjectCreateAndFireTempWeaponToTarget(self, "WeaponTemplateName", {
  X=x, Y=y, Z=z
}, nil)

[[28]]
-- 获取单位目标，优先获取当前右键目标，在没有右键目标的情况下获取当前自动索敌目标
-- 假如单位没有目标，会返回 nil
local target = ObjectGetTarget(self)
-- ObjectGetTarget 在前面的版本里被称作 ObjectFindTarget
-- 但实际上 ObjectGetTarget 是更加准确的名字
-- ObjectFindTarget 为了保持兼容性，依然可以使用，效果相同，但仍然建议改用 ObjectGetTarget

[[29]]
-- 获取单位的建造者（生产者）
-- 例如可以通过矿车找到矿场，或者通过小飞机找到航母
-- 假如单位没有建造者，或者建造者已经不存在于地图上，会返回 nil
local producer = ObjectGetProducerObject(self)

```

### 第30-38条
```lua

[[30]]
-- 获取单位造价
-- 第二个参数是可选的，传入一个生产建筑的 table，可以获取该单位在该建筑里的造价
-- 第三个参数也是可选的，传入一个自定义的价格，可以根据玩家以及生产建筑的各种 buff 来计算造价
local cost = ObjectGetValue(self)
local cost = ObjectGetValue(self, producer)
-- 一般来说，下面这个就会直接返回 1000（而不是单位原来的造价），
-- 除非有特殊的 buff
local cost = ObjectGetValue(self, producer, 1000)

[[31]]
-- 获取玩家所有单位的造价
local totalCost = GetPlayerAllUnitsValue(self)
-- 假如调用了 SetWorldBuilderThisPlayer，那么也可以传入字符串：
SetWorldBuilderThisPlayer(self) -- 必须设置 this player 之后才能使用字符串
local totalCost = GetPlayerAllUnitsValue("<This Player>")
-- 与前面几个函数不同，这个函数能够正确处理 <1st Human Player's Enemies>
-- 这种复数的玩家名称，会把所有敌人的造价加起来
local enemiesTotalCost = GetPlayerAllUnitsValue("<1st Human Player's Enemies>")
-- 获取玩家所有单位和建筑的造价
local totalCost = GetPlayerAllUnitsStructuresValue(self)
SetWorldBuilderThisPlayer(self) -- 必须设置 this player 之后才能使用字符串
local totalCost = GetPlayerAllUnitsStructuresValue("<This Player>")

[[32]]
-- 检测单位是否属于某种类型
-- 根据玩家以及 RegisteredObjectSetId 来判断
-- 关于 RegisteredObjectSetId 的值，可以参考 Mod SDK 里的相关 XSD 文件
-- 例如，假如想要检测 self 是否是全局（属于任意玩家）的可驻军建筑，可以这样：
if ObjectIsInGlobalRegisteredObjectSet(self, "GLOBAL_HARVESTERS") then
  -- ...
end
-- 也可以检测单位是否属于玩家的某个 RegisteredObjectSet
if ObjectIsInPlayerRegisteredObjectSet(self, "HARVESTERS") then
  -- 上面这句其实就是检查 self 是否属于 self 所属玩家的矿车……
  -- 听起来有点像废话
  -- 但是也有更加复杂的应用场景：
  ObjectIsInPlayerRegisteredObjectSet(self, "HARVESTERS", anotherUnit)
  -- 这样就可以检查 self 是否是属于【anotherUnit 所属玩家】的矿车了
  -- 又或者：
  SetWorldBuilderThisPlayer(self) -- 必须设置 this player 之后才能使用字符串
  ObjectIsInPlayerRegisteredObjectSet(self, "HARVESTERS", "<1st Human Player's Enemies>")
end

[[33]]
-- 获取玩家所有某一类型的单位
-- 根据玩家以及 RegisteredObjectSetId 来判断
-- 关于 RegisteredObjectSetId 的值，可以参考 Mod SDK 里的相关 XSD 文件
-- 例如，假如想要获取玩家所有的矿车，可以这样：
local harvesters, count = CopyGlobalRegisteredObjectSet("GLOBAL_HARVESTERS")
-- 也可以获取 self 所属的玩家，所拥有的，所有的矿车：
local harvesters, count = CopyPlayerRegisteredObjectSet(self, "HARVESTERS")
-- 也可以传入字符串：
SetWorldBuilderThisPlayer(self) -- 必须设置 this player 之后才能使用字符串
local harvesters, count = CopyPlayerRegisteredObjectSet("<1st Human Player>", "HARVESTERS")
local units, unitCount = CopyPlayerRegisteredObjectSet("Player_1", "UNITS")
local unitStructures, unitStructureCount = CopyPlayerRegisteredObjectSet("Player_2", "UNITS_AND_STRUCTURES")

[[34]]
-- 获取以及设置单位武器的弹药量
-- 单位必须有 WeaponSetUpdate 组件
-- 通过 WeaponSlot 的 ID、Weapon 的 Ordering 和 WeaponSlot 中武器的序号来获取武器
-- 注意，WeaponSlot 的 ID 并不是 WeaponSlot 的序号！
-- 武器 Ordering 对应如下：
-- PRIMARY_WEAPON → 1
-- SECONDARY_WEAPON → 2
-- TERTIARY_WEAPON → 3
-- QUATERNARY_WEAPON → 4
-- QUINARY_WEAPON → 5
-- 而「武器的序号」指的是当前 WeaponSlot 内，
-- 该武器在「所有 Ordering 相同的 Weapon」里的序号
-- 举例：日冕 3.192 维护者轰炸机的 WeaponSetUpdate 代码如下：
-- <WeaponSlotHardpoint
--   ID="1">
--   <Weapon Ordering="PRIMARY_WEAPON" Template="AlliedAntiGroundAircraftMissilePodWeapon" />
--   <Weapon Ordering="PRIMARY_WEAPON" Template="AlliedAntiGroundAircraftMissilePodWeapon_AirPower" />
--   <Weapon Ordering="PRIMARY_WEAPON" Template="AlliedAntiGroundAircraftMissilePodWeaponAuto" />
--   <Weapon Ordering="PRIMARY_WEAPON" Template="AlliedAntiGroundAircraftMissilePodWeaponAuto_AirPower" />
--   <Weapon Ordering="PRIMARY_WEAPON" Template="AlliedAntiGroundAircraftMissilePodWeaponAI" />
--   <Weapon Ordering="PRIMARY_WEAPON" Template="AlliedAntiGroundAircraftMissilePodWeaponAI_AirPower" />
-- </WeaponSlotHardpoint>
-- WeaponSlot 的 ID 是 1，
-- WeaponSlot 中所有武器的 Ordering 都是 PRIMARY_WEAPON，
-- 这些 PRIMARY_WEAPON 武器的序号是从 1 到 6

-- 假如这是一架属于人类玩家的、拥有先进航空学协议的维护者轰炸机（112，_AirPower）
-- 想要获取它的武器弹药量，可以这样写：
local ammoCount = ObjectGetWeaponSetUpdateWeaponCurrentAmmoCount(self, 1, 1, 2)

-- 假如这是一架属于人类玩家的、拥有先进航空学协议和自动分弹升级的维护者轰炸机，（114，Auto_AirPower）
-- 想要设置它的武器弹药量为 1000，可以这样写：
ObjectSetWeaponSetUpdateWeaponCurrentAmmoCount(self, 1, 1, 4, 1000)

-- 假如这是一架属于 AI 的无协议维护者轰炸机，（115，AI）
-- 想要设置它的武器弹药量，可以这样写：
ObjectSetWeaponSetUpdateWeaponCurrentAmmoCount(self, 1, 1, 5, 1000)

-- 获取武器的最大弹药量
local maxAmmoCount = ObjectGetWeaponSetUpdateWeaponMaxAmmoCount(self, 1, 1, 1)

[[35]]
-- 获取单位武器的当前 WeaponLaunchBone 序号
-- 参数与上面的 ObjectGetWeaponSetUpdateWeaponCurrentAmmoCount 一致
-- 返回的序号从 1 开始，代表武器下一发子弹的发射位置
-- 例子：
-- <WeaponLaunchBone
--   WeaponSlotID="1"
--   WeaponSlotType="PRIMARY_WEAPON"
--   BoneName="FX_Weapon_" />
local boneIndex = ObjectGetWeaponSetUpdateWeaponCurrentWeaponLaunchBoneIndex(self, 1, 1, 1)
-- 假如 boneIndex 为 1，那么下一发子弹将会从 FX_Weapon_01 发射

[[36]]
-- 检测单位是否被瘫痪
-- 停电（UNDERPOWERED）、断电（EMP）、狙白（UNMANNED）、蜘蛛 F（HELD）均是瘫痪状态
-- 检测单位是否处于任意一种瘫痪状态：
if ObjectIsDisabled(self) then
  -- ...
end
-- 检测单位是否处于某些特定的瘫痪状态：
if ObjectIsDisabled(self, "EMP UNDERPOWERED UNMANNED") then
  -- 单位处于停电或者断电或者狙白状态
end
-- 也可以提前把瘫痪状态的字符串转换成数字，然后传入数字。这样可以在某些情况下提高性能。
DisabledStatus = ParseDisabledBitFlags("EMP UNDERPOWERED UNMANNED")
for i = 1, 100, 1 do
  if ObjectIsDisabled(units[i], DisabledStatus) then
    -- 单位处于停电或者断电或者狙白状态
  end
end

[[37]]
-- 检测单位是否存活
-- 假如单位存活，会返回 1
-- 假如单位不存在或者已经死亡，会返回 nil
if ObjectIsAlive(self) then
  -- ...
end
-- 这个函数和 EvaluateCondition("NAMED_NOT_DESTROYED", self) 是等价的
-- 但是它比 EvaluateCondition 更快，也更简单

[[38]]
-- AttributeModifier 相关函数，获取，添加，删除各种 buff / debuff
-- 加载一个 AttributeModifier，单位将会「享受」到被守护者技能激光指着的 debuff
ObjectLoadAttributeModifier(self, "AttributeMod_TargetPainted")
-- 强制潜艇上浮
ObjectLoadAttributeModifier(self, "AttributeMod_Submarine_Float")
-- 各种 buff / debuff 的持续时间默认为 XML 里定义的时间，
-- 但是也可以手动指定。
-- 强制潜艇上浮 30 帧
ObjectLoadAttributeModifier(self, "AttributeMod_Submarine_Float", 30)
-- 移除一个 AttributeModifier
ObjectUnloadAttributeModifier(self, "AttributeMod_TargetPainted")
-- 查询某个属性，具体属性类型可以参考 Mod SDK 里的相关 XSD 文件
-- 无加成会返回 1。假如有 20% 的加成，会返回 1.2。假如出现错误会返回 nil
local speedModifier = ObjectGetAttributeModifier(self, "SPEED")
-- 查询加成时，可以忽略某些特定类型的 buff / debuff，
-- 类似于 XML 里的 IncompatibleAttributeModifier
-- 例如查询伤害加成，但是不考虑毕方的加成：
local rateOfFireModifier = ObjectGetAttributeModifier(self, "DAMAGE_MULT", {
  -- 过滤以下这些 buff / debuff
  "AttributeModifier_CelestialSupportAircraftBuffEffect", -- 普通毕方加成
  "AttributeModifier_CelestialAAVehicleAttackMoveEffect", -- 毕方对于磁弩的加成
})
-- 反复查询 AttributeModifier 可能比较消耗性能。
-- 因此可以提前获取 AttributeModifier 对象，然后反复使用。
local targetPainted = FindAttributeModifier("AttributeMod_TargetPainted")
local celestialSupportBuffEffect = FindAttributeModifier("AttributeModifier_CelestialSupportAircraftBuffEffect")
local celestialAAVehicleAttackMoveEffect = FindAttributeModifier("AttributeModifier_CelestialAAVehicleAttackMoveEffect")
ObjectLoadAttributeModifier(self, targetPainted)
local rateOfFireModifier = ObjectGetAttributeModifier(self, "DAMAGE_MULT", {
  celestialSupportBuffEffect,
  celestialAAVehicleAttackMoveEffect
})

```



## 日冕V3.22x
### 第39-42条
```lua
[[39]]
-- 加强版的 <LuaEventNugget />
-- ********************************************************************************
-- ****************注意，该项在现版本日冕地图中暂不可用******************************
-- ********************************************************************************
-- 原来的 LuaEventNugget 许多时候并不是很好用，
	-- 它会忽略 SpecialObjectFilter，无视高度对范围内的所有单位执行 Lua 事件，
-- 而且它还必须要求目标存在且拥有 LuaEventList 才能执行事件，
	-- 也就是说假如对着空地板强 A，则 Lua 事件根本不会被执行。
-- 这些问题现在都可以被解决了。
-- 首先需要在配置文件里规定一个特殊的前缀：
-- [文件 Additional/Data/Scripts/RA3LuaBridge.json]
-- {
--   "enhancedLuaNuggetEventPrefix": "LuaNugget_"
-- }
-- 在此之后，假如 LuaEventNugget.EventName 以该前缀开头，
	-- 则会使用新的增强版 LuaEventNugget：
-- 1. 朝目标开火之前，会正确使用 SpecialObjectFilter 等限制条件
-- 2. 无论 AOE 范围内有没有目标、有多少目标、
	--    无论目标是否拥有 LuaEventList，
--    都会执行一次且仅一次 Lua 函数
-- 3. 它将会调用名字与 LuaEventNugget.EventName 相符的函数
-- 示例：
-- <LuaEventNugget
--   Radius="56.7"
--   EventName="LuaNugget_Warhead">
--   <SpecialObjectFilter Relationship="ENEMIES NEUTRAL" Rule="ALL">
--     <ExcludeThing>AlliedAntiVehicleInfantry</ExcludeThing>
--   </SpecialObjectFilter>
-- </LuaEventNugget>
-- 对应的 Lua 函数：
function LuaNugget_Warhead(data)
  -- 函数参数 data 里包含了所有相关的信息。
	  -- 由于性能因素，data 只能在函数内部使用，不能被保存到全局变量或者其他地方。
  -- 否则会造成游戏崩溃。
  -- 因此建议先把 data 里的信息复制到普通的 Lua 变量里（就和下面一样），
  -- 然后再使用这些普通的 Lua 变量。
  -- data 提供的信息列表：
	  -- 1. 攻击者的 id，假如攻击者已经不存在，会得到 nil
  local attackerId = data.AttackerId
  if attackerId ~= nil then
    local attacker = GetObjectById(attackerId)
  end
	  -- 2. 目标单位的 id，假如不存在目标单位，会得到 nil
  local targetId = data.TargetId
	  -- 3. 当前抛射体的 id，只适用于 Warhead 内部的 LuaEventNugget，其他情况下是 nil
  local projectileId = data.ProjectileId
  -- 4. 目标坐标
  local x, y, z = data.X, data.Y, data.Z
  -- 5. 上级武器的哈希值，数字。
	  --    对于 Warhead 内部的 LuaEventNugget，这个值是发射抛射体的武器的哈希值。
  --    其他情况下大概是 nil 吧。
  local sourceWeaponHash = data.SourceWeaponHash
  -- 6. 当前武器的哈希值，数字。
  local currentWeaponHash = data.CurrentWeaponHash
  -- 7. 当前武器的弹药量，可以参考 [[34]]
  local currentWeaponAmmo = data.CurrentWeaponAmmo
  -- 8. 当前武器的开火点序号，也是数字，可以参考 [[35]]
  local currentWeaponLaunchBoneIndex = data.CurrentWeaponLaunchBoneIndex
  -- 9. 当前 LuaEventNugget 受到 attr 影响的实际 Radius
  local actualRadius = data.ActualRadius
	  -- 10. 当前 LuaEventNugget 的 SpecialObjectFilter，假如没有的话会是 nil
  local specialObjectFilter = data.SpecialObjectFilter
  -- 11. 当前 LuaEventNugget 的范围判定类型
  local distType = data.PartitionFilterTestType
  -- 示例：
  local attacker = GetObjectById(data.AttackerId)
  local x, y, z = data.X, data.Y, data.Z
  local radius = data.ActualRadius
  local specialObjectFilter = data.SpecialObjectFilter
  local distType = data.PartitionFilterTestType
  local aoeTargets, count = ObjectFindObjects(
    attacker,
    {
      X = x, Y = y, Z = z,
      Radius = actualRadius, DistType = distType
    },
    specialObjectFilter
  )
  for i = 1, count, 1 do
    local target = aoeTargets[i]
    -- ...
  end
end

[[40]]
-- 检测当前单位的目标是否是右键目标
-- 更严谨的说法是，检测单位当前的目标是否是单位 AI 索敌模块的右键目标
-- 假如单位没有目标，或者单位 AI 没有索敌模块，或者单位索敌模块没有右键目标，会返回 nil
-- 假如单位目标与右键目标不同，也会返回 nil
-- 假如单位目标就是单位 AI 索敌模块的右键目标，会返回 1
if ObjectIsTargetAssignedTarget(self) then
  -- ...
end
if not ObjectIsTargetAssignedTarget(self) then
  -- ...
end

-- 设置单位索敌时的自定义数据
-- 但是它不会影响当前的目标，也不会影响右键目标
-- 因此，设置了这个过滤器之后，可能需要手动重置目标，
-- 或者在 XML 里设置较短的 ReAcquireDelay，迫使单位重新索敌
ObjectSetCustomTargetChooserData(self, {
  CustomFilter = CreateObjectFilter(...), -- 可选：自定义索敌过滤器，不符合该过滤器的单位不会被索敌到
  PreferredTargetId = ObjectGetId(xxx), -- 可选：优先索敌目标的 id
  CompareFilterList = { -- 可选：自定义索敌排序器，符合这些过滤器的单位会按照列表中的顺序被优先索敌
    CreateObjectFilter(...),
    CreateObjectFilter(...),
    CreateObjectFilter(...),
  },
})
-- 也可以清除
ObjectSetCustomTargetChooserData(self, nil)

	-- 2025/08/30 更新：ObjectSetCustomTargetChooserData 支持更多参数
-- 以下列出新的参数以及它们的使用方法
ObjectSetCustomTargetChooserData(self, {
  -- 仅在 CompareFilterList 没有选出更好的目标的情况下，优先选择某一个目标
  -- 该选项与 PreferredTargetId 类似，但是 PreferredTargetIdUnlessBetter 的优先级更低。
  -- 假如有其他单位由于 CompareFilterList 的原因被优先选择，那么 PreferredTargetIdUnlessBetter 指定的目标将会被忽略。
  PreferredTargetIdUnlessBetter = ObjectGetId(unit),
  -- 反向距离比较。也就是，优先选择距离较远的目标，而不再选择距离较近的目标。
  -- 该选项只能对使用了 TargetingDistanceCompare 的单位生效（目前所有单位应该都启用了该选项）
  ReverseRangeCompare = true,
  -- 优先选择射程内的目标。
  -- “射程”包括最小射程和最大射程，
  -- 因此启用该选项的单位将不再会优先选择处于最小射程内的目标。
  -- 这对于炮车等拥有最小射程的单位来说可能会有很大的用处。
  PreferTargetInsideRange = true,
})

-- 设置右键目标
-- 这个函数可能造成不可预料的副作用，请谨慎使用
ObjectSetAssignedTarget(self, target)
-- 既可以设置，也可以清除目标
ObjectSetAssignedTarget(self, nil)

-- 设置单位的自动索敌延迟。
-- 调整单位的索敌模块，使其在指定的秒数后进行下一次自动索敌。
-- 如果单位当前拥有右键目标（即手动攻击目标），该函数不会生效。
-- 仅当单位处于自动索敌状态时，该设置才会生效。
ObjectSetTargetChooserNextAutoAcquireDelay(self, 0) -- 下一帧立即索敌
ObjectSetTargetChooserNextAutoAcquireDelay(self, 100) -- 100 秒之后再索敌
-- 【引擎机制说明】
-- 红警 3 的自动索敌延迟由单位索敌模块的内部计时器控制，该计时器记录了
-- “自上一次自动索敌以来经过的时间”。索敌逻辑遵循以下规则：
-- 1. 当单位没有自动索敌目标时：
	--    - 若经过时间 ≤ IdleScanDelay，则不会索敌；
	--    - 若经过时间 > IdleScanDelay，则触发自动索敌。
-- 2. 当单位拥有自动索敌目标时：
	--    - 若经过时间 ≤ ReAcquireDelay，则不会重新索敌；
	--    - 若经过时间 > ReAcquireDelay，则重新索敌。
-- 【函数的实际效果】
-- 调用本函数时，会根据单位当前的索敌状态调整内部计时值：
--   • 若单位有目标：内部“经过时间” = ReAcquireDelay - delay
--   • 若单位无目标：内部“经过时间” = IdleScanDelay - delay
-- 【特殊情况说明】
	-- 若在调用本函数时单位有目标（使用 ReAcquireDelay），
	-- 但之后失去了目标（切换为 IdleScanDelay），
-- 实际索敌时间会偏差为：
--   IdleScanDelay - (ReAcquireDelay - delay)
	-- 因此，在这种情况下，若 (ReAcquireDelay - delay) > IdleScanDelay，单位会在下一帧立即重新索敌。

[[41]]
-- 性能相关 / 垃圾回收
-- 纳秒级别计时：
HighResolutionClock()
-- （其他代码）
local duration = HighResolutionClock()
	-- 每一次调用 HighResolutionClock，都会返回从上一次调用到这次调用的时间间隔，单位是纳秒
	-- 计时的时候请注意，HighResolutionClock() 的调用以及 Lua 本身都会消耗时间
-- 因此虽然 HighResolutionClock 理论上可以精确到纳秒，但是请不要拘泥于纳秒级别的测量结果
-- 假如有某段代码快了数百纳秒或者慢了数百纳秒，实际上可能都是误差。
-- 请注意不要使用 HighResolutionClock 去干除了性能测试之外的任何事情，因为联机必然会不同步。

-- 回收辣鸡。一般来说垃圾是自动回收的，而且 lua 本身就有 collectgarbage() 函数，因此完全不需要下面这个函数。
-- 但假如有特殊的少数情况，可以调用 GarbageCollectAll 来同时回收逻辑 Lua、地图 Lua 和动画 Lua 三个 Lua 环境的垃圾。
-- 但是，再说一遍，一般来说完全不需要调用这个函数。哪怕真需要回收垃圾，一般也应该使用 collectgarbage()。
GarbageCollectAll()

[[42]]
-- 获取 / 设置目标的虚拟伤害（对于支持虚拟伤害的攻击者，用于避免、或故意造成火力溢出）
-- 目标必须拥有 ActiveBody 或者其他支持伤害的 Body 组件
-- 因此 PropBody 和 InactiveBody 将会导致函数调用失败并返回 nil + 错误信息
-- 请注意以下函数并没有为性能进行优化，因此请避免过于频繁地调用这些函数
local list, count = ObjectGetVirtualDamages(self)
if list ~= nil then
  local attackerId = list[1].AttackerId
  local projectileId = list[1].ProjectileId
  local damage = list[1].Damage
  local startFrame = list[1].StartFrame
  local durationInFrames = list[1].DurationInFrames
end
-- 一次性替换所有的虚拟伤害
ObjectSetVirtualDamages(self, {
  {
    AttackerId = attackerId,
    ProjectileId = projectileId,
    Damage = 100,
    StartFrame = startFrame,
    DurationInFrames = durationInFrames,
  },
  {
    AttackerId = attackerId,
    ProjectileId = projectileId,
    Damage = 200,
    StartFrame = startFrame,
    DurationInFrames = durationInFrames,
  }
})
-- 添加一个虚拟伤害
ObjectAddVirtualDamage(self, {
  AttackerId = attackerId,
  ProjectileId = projectileId,
  Damage = 300,
  StartFrame = startFrame,
  DurationInFrames = durationInFrames,
})
```

ObjectSetAssignedTarget(self, target) 效果实验：

用玩家的多功能步兵车VS AI的防空墩子进行每秒设置测试

只要设置了目标，单位将无视其他敌方单位（不攻击）。

当目标在射程外，

警戒和侵略模式下，单位会接近并攻击目标，

固守和停火模式下，单位不反应。

当目标对于多功能的武器不合法时，单位不反应。



当单位在移动时，设置目标无效。当单位在攻击前进时，设置目标将打断单位当前移动命令。



用玩家的多功能步兵车VS AI的犀牛坦克进行每10秒设置测试

当双方距离很远时, 

警戒模式下，双方会靠近一段距离即撤回。

侵略模式下，双方会一直靠近至射程内，持续追击。

## 日冕V3.23x
### 第43-45条
_注意，第43-45条作为日冕V3.23版新加入的lua函数，可能存在测试时未发现的问题。如发现问题，欢迎在页面下方留言。_

```lua
[[43]]
-- 自定义存档支持
-- 启用方式：
-- 在 Mod 的 RA3LuaBridge.json 中设置：
--     "enableCustomSaveGameSupport": true
--
-- 启用后，游戏在保存存档时会尝试调用名为 OnSaveGame 的函数。
-- 该函数需要由脚本自行定义。
-- 游戏逻辑 Lua 环境和地图 Lua 环境都支持定义 OnSaveGame。
--
-- 【存档函数】
function OnSaveGame(data)
  -- 参数说明：
	  --   data: 一个用于保存数据的数组（table）。
  --
  -- 注意事项：
  -- 1. 必须使用 tinsert(data, value) 依次向数组中插入数据（顺序数组）。
  --    或者，也可直接使用数字索引手动设置（从 1 开始）。
	  --    如果手动设置索引，建议同时设置 data.n 表示数组长度（tinsert 会自动设置 data.n）。
  --        data[1] = x
  --        data[2] = y
  --        data.n = 2
  -- 2. 请勿直接用赋值方式替换整个 data 表，例如：
  --        data = {...} 
  --    因为这只会改变传入 OnSaveGame 局部变量，不会影响真正的 data。
  --
  -- 支持的类型：
  --   • nil
	  --   • boolean（布尔值）
	  --   • number（数字）32位整数可以被精确保存；超过二进制32位的整数以及浮点数可能会有精度损失
	  --   • string（字符串）
  --   • 仅包含上述类型元素的数组（可嵌套）

  -- 示例：
  tinsert(data, true)           -- 布尔值
  tinsert(data, 12345)          -- 数字
  tinsert(data, "Hello, world!") -- 字符串

  local subTable = {}
  tinsert(data, subTable)       -- 子数组
  tinsert(subTable, false)
  tinsert(subTable, 3.1415926)

  local anotherSubTable = {}
  tinsert(subTable, anotherSubTable) -- 嵌套数组
  tinsert(anotherSubTable, nil)
  tinsert(anotherSubTable, "DWYF")

  -- 限制与注意事项：
  -- • 不支持直接存储函数、自定义类型（例如 ObjectFilter 过滤器）。
  --   尝试保存这些类型将导致存档失败。
	  -- • table 中的非数字索引（如 data["abc"] 或 data.X）会被忽略。
  -- • 单位 table 隐含的 ObjectId 也不会被保存。
  -- 由于无法直接保存函数、自定义类型等，地图作者应在设计数据结构时考虑这一点。
  --
  -- 对于无法直接保存的数据，可考虑转换为字符串、数字等基本类型后再保存。
  -- 例如，将代码以字符串形式保存；从存档数据中恢复字符串后再用 dostring() 执行。
	  -- 但请注意：dostring 的性能不如普通的函数，大量使用可能造成卡顿。
  --
  -- 此外，也可以考虑直接重新执行一些开局执行过的 Lua 代码来重新初始化数据，
  -- 而不是将这些数据存储在存档中。
  -- 例如，不依靠存档数据，而是直接重新执行 CreateObjectFilter(...) 来在加载存档之后重新创建过滤器。
  --
  -- 建议根据具体情况合理取舍，避免存储不必要或无法恢复的数据。

  return true  -- 返回 true/1 表示保存成功
               -- 返回 false/nil 表示保存失败，存档将被阻止
end

-- 【读取存档数据】
-- 游戏在加载存档时，会将之前保存的数据赋值给全局变量：
--     SavedGameDataTable
-- 该表结构与保存时完全一致，可直接访问以恢复数据。
	-- 游戏逻辑 Lua 和地图 Lua 各自拥有独立的 SavedGameDataTable，
-- 它们互不影响。

for i = 1, getn(SavedGameDataTable) do
  local v = SavedGameDataTable[i]
  -- 恢复数据逻辑 ...
end

-- 建议在使用完毕后清空表以释放内存：
SavedGameDataTable = nil


[[44]]
-- 本地化 / 翻译文本
-- 【功能说明】
-- 函数 TextTranslate 可根据当前语言环境从 map.str 文件或游戏语言包中
-- 获取指定名称（如 "NAME:AlliedMCV"）对应的本地化文本。
-- 如果找不到对应翻译，将返回原始字符串本身。
-- 【重要注意事项】
-- 不要将本地化文本参与游戏逻辑判断！
-- 因为不同玩家的翻译内容可能不同，使用这些字符串参与具体游戏逻辑，会导致联机不同步。
	-- 仅在传递给用户界面（UI）时使用是安全的。
-- 【使用场景】
-- 该功能仅用于用户界面层面的文本显示：
	--   1. 公屏文本（例如 exAddTextToPublicBoard）
	--   2. 左下角消息区（例如 exMessageAppendToMessageArea）
	--   3. 正上方提示框（例如 exInfoBoxTopCenterShowForPlayer）
--   4. 自定义界面（按钮、对话框、计分板等）
--      例如 exCenterTopBtnShowForPlayer、exShowLongTextDialogForPlayer、exScoreBoardSetTextForPlayer 等
--
-- 【示例代码 - 基本用法】
local text = TextTranslate("NAME:AlliedMCV") -- text 可能为 "基地车" 或 "MCV"
local text2 = TextTranslate("UPGRADENAME:Upgrade_CelestialAuxiliaryAimingSystem")
if text2 == "UPGRADENAME:Upgrade_CelestialAuxiliaryAimingSystem" then
  text2 = "英文版日冕的翻译较为落后，许多升级名称暂时还没有翻译"
end
--
-- 【示例代码 - 格式化文本】
-- 假设 map.str 中包含以下条目：
-- SCRIPT:MISSION_1
--   "Enemy %s has been attacked by %s and has %d HP left!"
-- END
-- SCRIPT:MISSION_1?chinese
	--   "敌方%s被%s攻击，还剩%d点生命值！"
-- END
DisplayNames = {
  ["Player_1"] = "$p1Name",
  ["Player_2"] = "$p2Name",
  ["Player_3"] = "$p3Name",
  ["Player_4"] = "$p4Name",
  ["Player_5"] = "$p5Name",
  ["Player_6"] = "$p6Name",
}
local attackerPlayer = ObjectPlayerScriptName(attacker)
local enemyHP = ObjectGetCurrentHealth(enemy)
exAddTextToPublicBoard(
  format(
    TextTranslate("SCRIPT:MISSION_1"),
    TextTranslate("NAME:SOVIETBOMBERAIRCRAFT"),
    DisplayNames[attackerPlayer] or "???",
    enemyHP
  ),
  10
)
-- 公屏文本有可能会显示：
--   英文环境："Enemy Kirov Airship has been attacked by 爱丽丝玛格特洛依德 and has 1500 HP left!"
	--   中文环境："敌方基洛夫空艇被爱丽丝玛格特洛依德攻击，还剩1500点生命值！"
-- 具体内容取决于玩家使用的语言
-- 【格式化说明】
-- • format 函数的格式化占位符：
--   %s - 字符串
--   %d - 整数
--   %f - 浮点数
--   详细语法参考：
--   https://www.lua.org/manual/4.0/manual.html#format
-- • 玩家名称占位符：
--   $p1Name, $p2Name, ... 等由 EnhancerCorona 提供，
--   会在公屏文字中自动替换为对应玩家名称。
--   更多说明参见：
--   https://www.yuque.com/muzeqaq/ra3mapwiki/eksx1whrfix7tlno#r3X8J


[[45]]
-- 获取下一个单位 ID
-- 【功能说明】
-- GetNextObjectId() 返回当前游戏中“即将被创建”的单位 ID。
-- 单位 ID 通常是全局递增的，因此可以据此推测下一个新单位的 ID。
	-- （虽然游戏引擎理论上允许非自增的 ID，但目前暂未发现实际例外）
--
-- 【示例代码 - 基本用法】
local nextId = GetNextObjectId()
-- 此时该 ID 对应的单位尚未创建，因此 GetObjectById(nextId) 会返回 nil
if GetObjectById(nextId) == nil then
  -- ...
end
--
-- 【示例代码 - 获取最后创建的单位】
-- 由于 ID 通常是自增的，可以通过 “下一个 ID - 1” 获取最后创建的单位
local lastCreatedObjectId = GetNextObjectId() - 1
local lastCreatedObject = GetObjectById(lastCreatedObjectId)
if lastCreatedObject ~= nil and ObjectIsAlive(lastCreatedObject) then
  -- 可在此对最近创建的单位进行处理
end
-- 倒查最后十个创建的单位
for i = 0, 9, 1 do
  local objId = lastCreatedObjectId - i
  local obj = GetObjectById(objId)
  if obj ~= nil and ObjectIsAlive(obj) then
    -- 处理最近创建的单位
  end
end
```

## 日冕V3.24x
### 第46条
```lua
[[46]]
-- 地编队伍调试信息（针对日冕 AI 在自定义地图的闪退问题）
-- 【功能说明】
-- GetPlayersSquadDebugData 函数返回当前地图中所有玩家（Player）的 Squad（队伍实例）占用情况。
-- 数据来源于游戏引擎内部结构：
--   • 每个 Player 拥有固定数量的 Squad（共 85 个，实际可用数量为 70 左右）
--   • 创建 Team 时，引擎会从该数组里找一个空闲 Squad 分配给 Team
--   • 如果所有 Squad 被占满，继续创建 Team 将导致游戏崩溃
-- 用途：
--   ✔ 查看每个玩家当前占用了多少 Squad
--   ✔ 分析 AI 或地图脚本创建 Team 的行为，协助日冕制作组修复自定义地图闪退问题
--   ✔ 调试 Squad 与 Team 的关系
--   ✔ 输出调试信息到日志文件或 UI（消息区、公屏文本、自定义文本等）
-- 【重要注意事项】
--   - 不要依赖此函数返回的数据控制游戏逻辑
--     例如：不要直接在代码里根据 Squad 数量决定是否创建 Team
--   - 数据仅用于调试和观察，不能保证实时或跨版本一致
--
-- 【返回值数据结构】
-- 返回值是一个数组，每个元素表示一个玩家：
--   debugData[1], debugData[2], ... 表示玩家 1、玩家 2 ...
-- 每个玩家有可能是 table 或者 nil，假如是 table，则包含字段：
--   PlayerId              → 玩家 ID（整数）
--   PlayerName            → 玩家名称（字符串）
--   SkirmishAIInstanceId  → AI ID 哈希值（整数）
--   SkirmishAITypeName    → AI 类型名称（字符串）
--   SkirmishAIUIName      → AI UI 名称（字符串）
--   Squads                → 玩家 Squad 数组，长度固定
-- 每个 Squad 包含字段：
--   TeamId        → 该 Squad 分配的 Team ID（整数），若 Squad 空闲则为 nil
--   TeamName      → Team 名称（字符串），若 Squad 空闲则为 nil
--   ObjectIdList  → Squad 内单位 ID 数组
--     ObjectIdList[1], [2], ... → 单位 ObjectId
--     ObjectIdList.n            → 单位数量（也可使用 getn(ObjectIdList) 获取）
--
-- 【示例代码】
-- 打印每个玩家的 Squad 使用情况
local debugData = GetPlayersSquadDebugData()
-- 该示例中使用 exCreateCustomText 输出调试信息到屏幕上
-- exCreateCustomText 的自动换行长度取决于第一行的畅度
-- 更多信息请参考 EnhancerCorona 文档
local text = strrep('-', 96) .. "\n"
for i = 1, getn(debugData), 1 do
  local player = debugData[i]
  if player == nil then
    -- 玩家不存在
    text = text .. format("Player %d: [不存在]\n", i)
  else
    -- 打印玩家基本信息
    local aiText = "并非 AI"
    if player.SkirmishAIInstanceId ~= 0 then
      aiText = format("AI %X %s %s",
        player.SkirmishAIInstanceId,
        player.SkirmishAITypeName,
        TextTranslate(player.SkirmishAIUIName)
      )
    end
    text = text .. format("Player %d: %s %s\n",
      player.PlayerId,
      player.PlayerName,
      aiText
    )
  end
  -- 遍历玩家的每个 Squad
  for slotIndex = 1, getn(player.Squads), 1 do
    local squad = player.Squads[slotIndex]
    -- Squad 空闲
    if squad.TeamId == nil then
      -- text = text .. format("  Squad %d: [空闲]\n", slotIndex)
    else
      -- Squad 已分配给 Team
      text = text .. format(
        "  Squad %d: Team %d (%s) — Units: %d\n",
        slotIndex,
        squad.TeamId,
        squad.TeamName,
        getn(squad.ObjectIdList)
      )
    end
    -- 遍历 Squad 内所有单位
    for i = 1, getn(squad.ObjectIdList), 1 do
      local objectId = squad.ObjectIdList[i]
      -- 可根据 objectId 进一步获取对象信息
      -- local obj = GetObjectById(objectId)
    end
  end
end
-- 输出调试信息
exCreateCustomText({
    Index = 1,
    Content = text,
    X = 50,
    Y = 50,
    Color = HexToUInt32("FFFFFF"),
    Size = 15,
    AlignX = "left",
    AlignY = "top",
  })
```

## 日冕V3.34
### 第47条 切换单位 LocomotorSet
```lua
-- 切换单位 LocomotorSet
-- 【功能说明】
-- 单位可以有多种 LocomotorSet（移动方式），具体取决于单位的 XML 源码。
-- 例如天狗就有 NORMAL（地面/水面）和 SUPERSONIC（飞行）两种 LocomotorSet。
-- 使用 ObjectChooseLocomotorSet 可以在这些 LocomotorSet 之间切换。
-- 假如切换成功，返回 1；
-- 假如切换失败（例如指定的 LocomotorSet 不存在），返回 nil + 错误信息。
-- 【注意事项】
-- NORMAL 与 NORMAL_UPGRADED 被游戏引擎特殊处理，
-- 游戏有时候会把这两种类型视为同一种，因此不一定总是能从 NORMAL_UPGRADED 切换到 NORMAL。
-- 可用的 LocomotorSet 类型可参考游戏的 XSD 定义。
-- 建议查看单位的 XML 源码以确认实际可用的 LocomotorSet。
ObjectChooseLocomotorSet(self, "NORMAL")
ObjectChooseLocomotorSet(self, "SUPERSONIC")
```

### 第48条 获取单位的 InstanceId
```lua
-- 获取单位的 InstanceId（增援代码）
-- 与 EnhancerCorona 的 exObjectGetInstanceId 类似，但不会有负数的问题
if ObjectGetInstanceId(self) == 2739059666 then
  -- ...
end
if ObjectGetInstanceId(self) == FastHash("CelestialAttackerAircraftFireProjectile") then
  -- ...
end
-- ObjectGetInstanceIdHexString 等价于 format("%X", ObjectGetInstanceId(self))，但性能更好一些
if ObjectGetInstanceIdHexString(self) == "8D0A384A" then
  -- ...
end

```

### 第49条 贝塞尔
```lua
-- 贝塞尔抛射体修改
-- 提供两种方式修改贝塞尔抛射体行为：
-- - 全局修改（推荐）：对指定 GameObject 类型的所有抛射体生效
-- - 实时修改：对单个抛射体实例进行操作

----------------------------------------------------
-- 1. 全局贝塞尔抛射体修改
----------------------------------------------------
-- 注册贝塞尔抛射体修改规则。
-- 该函数只需要在游戏开始时调用一次，对整局游戏生效。
-- 因此不需要重复注册相同修改。
-- 参数说明：
-- GameObject        : string, 抛射体所属单位 ID
-- 左右漂移修改：
-- SidewaysDriftMin  : number, 抛射体在贝塞尔曲线 P1 控制点的左右漂移最小值，负数表示向左
-- SidewaysDriftMax  : number, 抛射体在贝塞尔曲线 P1 控制点的左右漂移最大值
-- SidewaysDrift2Min : number, 抛射体在贝塞尔曲线 P2 控制点的左右漂移最小值
-- SidewaysDrift2Max : number, 抛射体在贝塞尔曲线 P2 控制点的左右漂移最大值
-- 注意：抛射体必须在 XML 内定义有非零 SidewaysDrift，否则左右漂移修改将不会生效。
-- 目前暂时仅支持修改左右漂移参数，未来可以增加更多可修改的参数以及功能。
RegisterBezierModification({
    GameObject = "CelestialAttackerAircraftFireProjectile",
    SidewaysDriftMin = -10,
    SidewaysDriftMax = -5,
    SidewaysDrift2Min = 20,
    SidewaysDrift2Max = 100,
})
-- 上例表示 CelestialAttackerAircraftFireProjectile 抛射体飞行时左右漂移范围在 [-10, -5] 之间。

----------------------------------------------------
-- 2. 实时修改单个抛射体
----------------------------------------------------
-- 若条件允许，建议请求让 RegisterBezierModification 增加更多的参数与功能，
-- 而不是用 Lua 实时修改单个抛射体，因为实时修改性能消耗更大，还可能引起抛射体行为异常。
-- RegisterBezierModification 由 C++ 实现，性能更高，并且可以同时对所有同类型抛射体生效
-- 注意事项：
-- 第一个参数必须是拥有 BezierProjectileBehavior 的单位，否则函数调用会失败。
-- 如果单位有多个 BezierProjectileBehavior，则只对第一个抛射体模块进行操作。
-- 实时修改在以下情况可能不会立即更新抛射体路径：
-- 1. 抛射体在 XML 代码里被设定为不追踪目标
-- 2. 抛射体的追踪目标已死亡或存在
-- 因此有时候可能会需要用 ObjectForceBezierProjectileMakeNewPath 重建贝塞尔路径。

-- 获取贝塞尔路径起点
local beginX, beginY, beginZ = ObjectGetBezierProjectileBeginPoint(projectile)
-- 获取贝塞尔路径终点
local endX, endY, endZ = ObjectGetBezierProjectileEndPoint(projectile)
-- 获取目标单位 ID（可能无效，使用前请检测 ObjectIsAlive(targetId)）
local targetId = ObjectGetBezierProjectileTargetId(projectile)

-- 设置贝塞尔路径起点
ObjectSetBezierProjectileBeginPoint(projectile, newBeginX, newBeginY, newBeginZ)
-- 设置贝塞尔路径终点
ObjectSetBezierProjectileEndPoint(projectile, newEndX, newEndY, newEndZ)
-- 设置目标单位 ID
ObjectSetBezierProjectileTargetId(projectile, newTargetId)

-- 获取贝塞尔进度
-- 在初始状态下，
-- distanceTraveled 代表抛射体在曲线上已经飞行的距离；
-- curveSpeed 代表抛射体在曲线上的移动速度，也就是 distanceTraveled 的变化率；
-- pathPointIndex 代表当前处于的采样点。
-- pathPointCount 代表采样点总数，通常在 1 到 42 之间，具体取决于路径长度和引擎设置。
local distanceTraveled, curveSpeed, pathPointIndex, pathPointCount = ObjectGetBezierProjectileProgress(projectile)
-- 假如只需要前几个参数，可以只获取它们，后面的参数会被忽略（正常 lua 语法）
local distanceTraveled, curveSpeed = ObjectGetBezierProjectileProgress(projectile)

-- 设置新的贝塞尔进度，可以设置三个参数：
-- distanceTraveled 代表抛射体在曲线上已经飞行的距离；
-- curveSpeed 代表抛射体在曲线上的移动速度，也就是 distanceTraveled 的变化率；
-- pathPointIndex 代表当前处于的采样点。（请注意 pathPointCount 是只读的，无法直接修改）
-- 这三个参数都是可选的，假如为 nil 则表示不修改该参数。
ObjectSetBezierProjectileProgress(projectile, nil, nil, nil)
-- 在初始状态下，可以仅通过设置 distanceTraveled 来调整抛射体在路径上的位置，
-- 而不需要调整 curveSpeed 和 pathPointIndex。
-- 在初始状态下，游戏引擎可以根据 distanceTraveled 的值，自动调整 pathPointIndex
ObjectSetBezierProjectileProgress(projectile, newDistanceTraveled)
-- 但在最终状态下（curveSpeed 等于 special speed），
-- 游戏引擎会直接无视 distanceTraveled 和 curveSpeed，每帧把 pathPointIndex 增加 1 来推进抛射体飞行，
-- 因此在这种状态下，必须直接调整 pathPointIndex 来改变抛射体在曲线上的位置。
ObjectSetBezierProjectileProgress(projectile, nil, nil, newPathPointIndex)

-- 强制抛射体重新计算飞行路径
ObjectForceBezierProjectileMakeNewPath(projectile)
-- 注意：
-- 1. 该函数性能开销较大，请避免频繁调用。
-- 2. 引擎通常不会期望路径在 Lua 执行期间被重建，
--    因此重建路径后不保证抛射体行为完全稳定。

-- ObjectForceBezierProjectileMakeNewPath 高级参数
-- 支持两个可选参数：
-- fullRebuild
--   false：
--     仅根据新的起点/终点对曲线进行局部调整。
--   true：
--     完全重新构建贝塞尔轨迹，包括：
--     - 所有控制点重新计算
--     - 随机参数重新生成
--     - 内部路径采样点数组长度可能变化
--     有可能导致贝塞尔模块不稳定。
-- straightLine
--   true  ：使用直线路径
--   false ：使用正常贝塞尔曲线
-- （推测 straightLine 参数可能与黑洞装甲 / 护盾 / 导弹偏转的机制有关，尚未验证）
--
-- 贝塞尔模块正常状态下自身在更新路径时使用参数：
--   fullRebuild = false
--   straightLine = false
-- WeaponTemplate 中可用于偏转导弹的 ScatterProjectileNugget 使用参数：
--   fullRebuild = true
--   straightLine = false
-- ScatterProjectileNugget 在重建曲线之前会：
-- 1. 把抛射体设为 UNATTACKABLE 状态，
--    以确保抛射体未来不会再重新被 ScatterProjectileNugget 重建路径
-- 2. 把起点设为抛射体的当前位置
-- 3. 重设一个新的终点
-- 4. 还会把贝塞尔曲线的采样点序号进行重置为第一个点（请参见 ObjectSetBezierProjectileProgress 函数）
--
-- 如果使用不同参数，
-- 行为可能与引擎默认逻辑产生差异，
-- 从而产生冲突、导致抛射体行为异常。
local fullRebuild = false
local straightLine = false
ObjectForceBezierProjectileMakeNewPath(
    projectile,
    fullRebuild,
    straightLine
)

----------------------------------------------------
-- 3. 调试接口（仅用于调试）
----------------------------------------------------
-- 获取贝塞尔路径调试数据
-- 注意：
-- 该函数性能较差，仅建议用于调试。
-- 示例：
local debugData = ObjectGetBezierProjectileDebugData(projectile)
local projectileId = ObjectGetId(projectile)
local projectileTypeId = ObjectGetInstanceId(projectile)
local p0x, p0y, p0z = ObjectGetBezierProjectileBeginPoint(projectile)
local p3x, p3y, p3z = ObjectGetBezierProjectileEndPoint(projectile)
local debugText = format(
  "Bezier#%d instance id %X, special speed %.2f\n"
  .. "curve speed %.2f, curve distance traveled %.2f\n"
  .. "BezierP0 (%.2f, %.2f, %.2f), BezierP1 (%.2f, %.2f, %.2f)\n"
  .. "BezierP2 (%.2f, %.2f, %.2f), BezierP3 (%.2f, %.2f, %.2f)\n"
  .. "current path point index %d / %d\n",
  projectileId,
  projectileTypeId,
  debugData.SpecialSpeed,
  debugData.CurveSpeed,
  debugData.CurveDistanceTraveled,
  p0x, p0y, p0z,
  -- 经过实测，发现贝塞尔模块的控制点 P1 和 P2 总是实时计算、
  -- 大部分情况下并不会缓存在贝塞尔模块内
  -- 因此此处得到的 P1 和 P2 可能会是 0
  debugData.P1X, debugData.P1Y, debugData.P1Z,
  debugData.P2X, debugData.P2Y, debugData.P2Z,
  p3x, p3y, p3z,
  debugData.PathPointIndex,
  debugData.PathPointCount
)
for i = 1, debugData.PathPointCount, 1 do
  local pathPoint = debugData.PathPoints[i]
  debugText = debugText .. format(
    "  PathPoint %d: (%.2f, %.2f, %.2f)\n",
    i,
    pathPoint.X,
    pathPoint.Y,
    pathPoint.Z
  )
end
exCreateCustomText({
  Index = 1,
  Content = debugText,
  X = 50,
  Y = 50,
  Color = HexToUInt32("FFFFFF"),
  Size = 15,
  AlignX = "left",
  AlignY = "top",
})
```
