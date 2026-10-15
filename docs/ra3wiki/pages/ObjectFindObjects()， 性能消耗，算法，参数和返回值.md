---
title: "ObjectFindObjects()， 性能消耗，算法，参数和返回值"
catalog_path: "Lua API "
slug: vp3pixg2rbomqpz9
source: https://www.yuque.com/muzeqaq/ra3mapwiki/vp3pixg2rbomqpz9
fetched: 2026-10-15
content_updated_at: "2025-02-28T22:56:14.000Z"
type: Doc
format: lake
---

# ObjectFindObjects()， 性能消耗，算法，参数和返回值

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/vp3pixg2rbomqpz9
> 目录位置：Lua API 

:::info
作者：Inertia

日期：2024年12月11日

更新日期：2024年12月26日

:::

在游戏开发中，运行成本的优化直接关系到系统的性能表现与玩家体验。在以《红色警戒3：日冕模组》的诺曼底V4地图为例的实验中，通过对不同Lua函数的性能测试，我们可以清晰地看到函数运行效率的差异，以及如何针对不同场景选择合适的实现方案，从而平衡复杂功能与运行效率。

本研究分析了从全图对象搜索到区域内敌军判定，从复杂过滤器的使用到函数调用成本的具体数据，耗时从低至50纳秒到高达200万纳秒不等。这些数据为开发者提供了明确的优化方向，例如在大型地图中减少不必要的搜索或使用预先构建的过滤器以降低资源消耗。

通过深入了解这些运行成本，开发者可以更精准地调整脚本逻辑与实现方案，为游戏提供更流畅的运行体验，同时最大限度地利用硬件资源。

## 运行成本
以诺曼底V4地图为例，有4700个建筑物，700个单位（共9500个各类物体），使用14900K CPU进行测试。

### a. 全图建筑搜索
使用`ObjectFindObjects`的Include STRUCTURE 进行搜索，每次耗时约2000us。

而使用`CopyPlayerRegisteredObjectSet`，则只需要1000us。

**结论**：获得全图建筑清单时不推荐使用`ObjectFindObjects`

### b. 全图单位搜索
在全图搜索单位时，差距则不明显，但大部分时候`CopyPlayerRegisteredObjectSet`耗时比`ObjectFindObjects` 少约50%-70%。

即不论是使用“<All Players>” 或是连续叫8个不同player（“Player_1”到“Player_6”，以及“PlyrCivilian”，“PlyrCreeps”），`CopyPlayerRegisteredObjectSet`的总耗时均在100-200us上下，而`ObjectFindObjects` 使用Include和exclude搜索单位耗时则需要约500us。

差距不大的原因可能是单位数量相比建筑数量更少，`ObjectFindObjects`对每个单位的检查因此累计消耗更少。

```lua
CreateObjectFilter({
    Rule = "ANY",
    Include = "SELECTABLE",
    Exclude = "STRUCTURE",
    IncludeThing = {}
})
```



而当IncludeThing是一个非常长的清单的时候，运行成本会大幅增加。例如20个载具名，10个舰船名，单次搜索只返回130个单位，但是耗时高达1500us，而使用Include和exclude搜索单位返回500个单位，耗时仅500us。

**结论**：获得全图单位清单时，首先推荐使用`CopyPlayerRegisteredObjectSet`，其次是使用`Include`的`ObjectFindObjects` 。

要获得多种单位时，最好先获得大类清单然后额外写LUA代码过滤，而不是超长的`IncludeThing`清单。

### c. 区域敌军搜索
为600个单位进行单次2000半径的周围敌军搜索,

`ObjectCountNearbyEnemies`平均60us每单位每次

`ObjectFindObjects` 平均90us每单位每次

虽然`ObjectFindObjects` 用时更长，但那可能是因为`ObjectFindObjects` 做了额外的工作，即返回敌军清单，而`ObjectCountNearbyEnemies`则只是计数。

```lua
CreateObjectFilter({
    Rule="ANY",
    Relationship = "ENEMIES",
    Include = "SELECTABLE",
    IncludeThing = {}
})
```

当半径为1000时，

`ObjectCountNearbyEnemies`平均20us每单位每次

`ObjectFindObjects` 平均22us每单位每次(Include和IncludeThing均为空的空白搜索则为约5 us)

成本降低的可能原因是每次需要搜索的范围变少，需要返回的敌军数量也更少。

这里也可以发现一个优化思路，即时间和空间的取舍。例如如果需求只是需要判断周围是否有敌军接近以从闲置状态进入警戒状态，那么每秒大范围搜索（2000半径，成本为60us 每秒）比每帧连续小范围搜索（1000半径，成本为20*15=300us每秒）的成本更低。作为参考，游戏中最快的单位阿波罗，速度为250，飞过2000距离也需要8秒。

### d. 代码成本研究
首先把全图5200个可选择物体标记为REPAIR_ALLIES_WHEN_IDLE， 然后使用2个过滤器进行实验。

```lua
local filter_returnNone = CreateObjectFilter({
    Rule = "ANY",
    StatusBitFlagsExclude = "REPAIR_ALLIES_WHEN_IDLE",
    Include = "SELECTABLE",
    IncludeThing = {}
})

local filter_returnAll = CreateObjectFilter({
    Rule = "ANY",
    Include = "SELECTABLE",
    IncludeThing = {}
})
```

filter_returnNone和filter_returnAll将会检查相同的5200个物体，但是区别是filter_returnNone因为StatusBitFlagsExclude 将会不返回任何物体，而filter_returnAll将返回所有物体。

实验结果：

`filter_returnNone`仅耗时200us，

`filter_returnAll`耗时高达2000-3000us。

**结论**：ObjectFindObjects()在生成返回的单位列表这一步上成本很高。单纯遍历单位的消耗极低。

### e. 对比普通函数的消耗
`exPlayerGetSelectedObjectIds(player)`

150-200 us (选择约300单位时)

5 us(未选择单位时)

`exObjectMoveTo(unitID, x, y, z)`

30-50 us(路径无变化时，5-10 us)



`ObjectDescription(current)`

800-1200 ns

`exObjectGetInstanceId(objectId)`

1900-2000 ns 

`CreateObjectFilter()`

1200-3500 ns  （相比ObjectTestTargetObjectWithFilter 80-100ns每次的消耗，CreateObjectFilter 上千ns的消耗预示着预先创建而非临时创建过滤器的必要性）

`GetObjectById(unitID)`

500-600 ns (平均值)

`ObjectGetId(current)`

50-70 ns

`exObjectGetCurrentSecondaryDamage(unitID)`

500-700 ns (平均值)

`ObjectGetCurrentHealth(current)` 

80-120 ns(增加容差整数化，180-210 ns)

`ObjectGetPosition(current)`

80-120 ns(增加容差整数化，350-400 ns)

`ObjectGetInitialHealth(current)`

70-80 ns

`exObjectGetCurrentHealth(unitID)`

500-600 ns

`exObjectGetMaxHealth(unitID)`

400-600 ns



`EvaluateCondition("NAMED_NOT_DESTROYED", current)`

150-300 ns

`ObjectIsAlive(current)`

60-70 ns 

`ObjectTestTargetObjectWithFilter(nil, current, filter)`

80-100 ns  （原生引擎函数）



`EvaluateCondition("UNIT_SPECIAL_POWER_READY", current, special_power_name)`

1000-1100 ns

`EvaluateCondition("UNIT_USING_STANCE", current, "AGGRESSIVE")`

1200-1600 ns

`EvaluateCondition("UNIT_USING_STANCE", current, "GUARD")`

600-800 ns

`ExecuteAction("UNIT_SET_STANCE", current, stance)`

2400-3000 ns

`ExecuteAction("UNIT_CHANGE_OBJECT_STATUS", current, status, bool)`

1000-1100 ns

` ObjectSetObjectStatus(current, status)`

50-60 ns  （原生引擎函数）

注意`ExecuteAction`消耗为相同目的的专用LUA函数的数十倍，原因为`ExecuteAction`调用对应函数需要经过一个巨大的字符串数组(约1000项一一比对）最终才能找到对应的代码执行。

### f. 总结
初步的结论是，`ObjectFindObjects`的性能消耗取决于过滤器中的判断条件和返回单位的数量。

过滤器中的判断条件会带来不同的成本。

低成本的条件是Include， Exclude， StatusBitFlagsExclude。

高成本的条件是IncludeThing，且IncludeThing清单越长成本越高。

## 过滤器规则
有一个小细节要注意，可能是笔误的关系，第2步时，如果目标单位在IncludeThing列表内，仍然会检查其ObjectStatus是否在StatusBitFlagsExclude内，如果在的话，会返回`不符合`。



过滤器算法  
作者：公主大佬， 根据游戏引擎汇编代码得来，绝对准确

  
1. 检查关系（Relationship）。  
假如过滤器定义了 Relationship，而且目标单位不满足 Relationship，  
  目标单位就【不符合】该过滤器。  
假如满足 Relationship，或者 Relationship 未定义，或者当前情况不适用 Relationship，则进入第二步。  
  
2. 检查目标单位是否在过滤器的 IncludeThing 列表内。  
假如目标单位在 IncludeThing 列表内，  
  目标单位就【符合】该过滤器。  
假如目标单位不在 IncludeThing 列表内，则进入第三步。  
  
3. 检查目标单位是否在过滤器的 ExcludeThing 列表内。  
假如目标单位在 ExcludeThing 列表内，  
  目标单位就【不符合】该过滤器。  
假如目标单位不在 ExcludeThing 列表内，则进入第四步。  
  
4. 检查目标单位的 KindOf 是否符合过滤器的 Exclude。  
假如目标单位的 KindOf 符合 Exclude，  
  目标单位就【不符合】该过滤器。  
假如目标单位的 KindOf 不符合 Exclude，则进入第五步。  
  
5. 根据过滤器的 Rule，检查目标单位的 KindOf 是否符合过滤器的 Include。  
假如 Rule 为 ALL，无论目标单位的 KindOf 是否符合 Include，则进入第六步。  
假如 Rule 为 ANY，只要目标单位的 KindOf 有任意一项符合 Include，则进入第六步。  
假如 Rule 为 ANY，而且目标单位的 KindOf 没有任何一项符合 Include，  
        目标单位就【不符合】该过滤器。假如 Include 为空，单位的 KindOf 就无法符合 Include。  
假如 Rule 写了但是不是 ALL或ANY，就会认为【不符合】该过滤器。  
假如 Rule 为 NONE，只要单位的 KindOf 包含了过滤器 Include 里面的每一个元素，则进入第六步。  
假如 Rule 为 NONE，而且目标单位的 KindOf 不包含过滤器 Include 里的每一个元素 或 Include 为空，  
        目标单位就【不符合】该过滤器。  
  
6. 检查目标单位的状态（ObjectStatus）是否符合过滤器的 StatusBitFlags。  
假如过滤器定义了 StatusBitFlags，而且目标单位的状态没有任何一项符合 StatusBitFlags，  
  目标单位就【不符合】该过滤器。  
假如目标单位的状态有任何一项符合 StatusBitFlags，或者 StatusBitFlags 未定义，则进入第七步。  
  
7. 检查目标单位的状态（ObjectStatus）是否符合过滤器的 StatusBitFlagsExclude。  
假如过滤器定义了 StatusBitFlagsExclude，而且目标单位的状态有任何一项符合 StatusBitFlagsExclude，  
  目标单位就【不符合】该过滤器。  
否则，目标单位就【符合】该过滤器。





## 函数参数, 返回值和对应的类型定义
注意当`ObjectFindObjects` 参数错误时， 第二个返回值会是包含报错信息的字符串，而不是正常情况下的整数。

例如过滤器指定了关系但敌我识别物体`iff_ref_object`已经无效时，第二个返回值是"Error: GameObjects not found" 

```lua
--- @class StandardUnitType
--- @field GUID UnitID The GUID of the unit, which is unique and invariant for the unit
-- ---Note this field name is just for illustration, the actual field name is hidden away.
-- This represents a standard unit type used across the game, details are hidden away.
-- This is actually a table that hold the unit's GUID which is unique and invariant for the unit.
-- The table retrived from game engine(ex: through ) however is not invariant and can change over time.
-- Assume this class handles all unit interactions and is referenced across multiple functions.


--- @alias UnitID integer
--- The actual value stored in the StandardUnitType table
--- This is the GUID of the unit, which is unique and invariant for the unit.
--- This is the actual value stored in the StandardUnitType table.
--- Value can be retrived by ObjectGetId(unit) function.


--- @class FilterParams
--- @field Relationship "ENEMIES"|"NEUTRAL"|"ALLIES"|"SAME_PLAYER"|nil Optional. The relationship between the objects
--- @field IncludeThing (string|number)[] Optional. Array of things to include, which could be names or hashed values.
--- @field ExcludeThing (string|number)[]|nil Optional. Array of things to exclude, must be used with IncludeThing.
--- @field Exclude KindOfList|nil Optional. Defines the kinds of objects to exclude. Directly reduce search space.
--- @field Rule "ANY"|"ALL"|nil Optional. The filter rule, either "ANY" or "ALL".
--- @field Include KindOfList|nil Optional. Defines the kinds of objects to include, such as "SMALL_MISSILE", "AIRCRAFT", "SHIP", etc.
--- @field StatusBitFlags string|nil Optional. Defines the status flags to include, such as "IGNORING_POWER_DOWN", "SHRUNK".
--- @field StatusBitFlagsExclude string|nil Optional. Defines the status flags to exclude, such as "IGNORING_POWER_DOWN", "SHRUNK".

--- @class ObjectFilter
-- This represents a filter object, which is used to filter specific units or objects based on some criteria.
-- The details of how the filter works are hidden away.
-- This filter is returned by the CreateObjectFilter function using the FilterParams table.

--- @class AreaParams
--- @field X number The X coordinate.
--- @field Y number The Y coordinate.
--- @field Z number The Z coordinate.
--- @field Radius number The radius of the area.
--- @field DistType "CENTER_2D" The type of distance calculation (e.g., "CENTER_2D").


--这个函数可以找到这一帧刚刚被创建的对象，以及现存的对象
--- Finds objects in a specified area around a reference object, using the given filter.
--- @param iff_ref_object StandardUnitType|nil The reference object for Identifying Friend or Foe (IFF), if nil, all objects are considered.
--- @param area_parameter_table AreaParams|nil The table containing area parameters (e.g., radius, coordinates).
--- @param filter ObjectFilter|nil The filter to apply to the search.
--- @return StandardUnitType[]|nil matched_objects An array of StandardUnitType objects found in the search.
--- @return integer|"Error: GameObjects not found"|string count The number of objects found.
function ObjectFindObjects(iff_ref_object, area_parameter_table, filter)
    return {}, 0
end
```

##
