---
title: "常用的日冕LuaBridge函数API注释.lua"
catalog_path: "Lua API "
slug: nintw5inok0aybfm
source: https://www.yuque.com/muzeqaq/ra3mapwiki/nintw5inok0aybfm
fetched: 2026-10-15
content_updated_at: "2026-04-27T13:27:16.000Z"
type: Doc
format: lake
---

# 常用的日冕LuaBridge函数API注释.lua

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/nintw5inok0aybfm
> 目录位置：Lua API 

:::info
作者：Inertia

日期：2024年11月14日

更新者: dreamness

更新日期：2026年04月27日

:::

本文列出了一些常用日冕Lua函数的LuaCATS格式注释，可复制到VS Code中帮助插件进行**自动参数类型检查**。推荐使用VS Code阅读。

这些函数和参数由于经常使用，更为可靠。未在这里列出的函数或参数，则存在由于被使用次数少，有不为人知的bug的可能。

完整的日冕自带的LuaBridge函数， 见[LuaAPI-LuaBridge](https://www.yuque.com/muzeqaq/ra3mapwiki/rg4gm1ibq054y6o4)

![VS Code中查看时的过滤器参数定义](https://cdn.nlark.com/yuque/0/2024/png/50038568/1735457397122-22b5da57-1f52-4ca3-b15c-b1e0df44b6a8.png)

```lua

--- @meta
--- This file is for type definition only, it is not executable code.
--- It is used to define the types of the functions and variables used in the code.


--- @class StandardUnitType
--- @field GUID UnitID The GUID of the unit, which is unique and invariant for the unit
-- ---Note this field name is just for illustration, the actual field name is hidden away.
-- This represents a standard unit type used across the game, details are hidden away.
-- This is actually a table that hold the unit's GUID which is unique and invariant for the unit.
-- The table retrived from game engine(ex: through ) however is not invariant and can change over time.
-- Assume this class handles all unit interactions and is referenced across multiple functions.


--- @class UnitID
--- The actual value stored in the StandardUnitType table
--- This is the GUID of the unit, which is unique and invariant for the unit.
--- This is the actual value stored in the StandardUnitType table.
--- Value can be retrived by ObjectGetId(unit) function.

--- 未完整列出
--- @alias KindOfList
--- | KindOf
--- | "SHIP SUBMARINE"
--- | "STRUCTURE AIRCRAFT"
--- | "STRUCTURE AIRCRAFT INFANTRY"
--- | "STRUCTURE AIRCRAFT INFANTRY SHIP SUBMARINE"
--- | "STRUCTURE INFANTRY SHIP"
--- | "STRUCTURE INFANTRY SHIP SUBMARINE"
--- | "HARVESTER AIRCRAFT INFANTRY"

--- 未完整列出
--- @alias ObjectStatusList
--- | ObjectStatus
--- | "AIRBORNE_TARGET SUBMERGED"
--- | "DAMAGED REALLYDAMAGED HAS_SECONDARY_DAMAGE"

--- @class FilterParams
--- @field Relationship "ENEMIES"|"NEUTRAL"|"ALLIES"|"SAME_PLAYER"|nil Optional. The relationship between the objects
--- @field IncludeThing (string|number)[] Optional. Array of things to include, which could be names or hashed values.
--- @field ExcludeThing (string|number)[]|nil Optional. Array of things to exclude, must be used with IncludeThing.
--- @field Exclude KindOfList|nil Optional. Defines objects KindOf to exclude. Directly reduce search space.
--- @field Rule "ANY"|"ALL"|nil Optional. The filter rule, either "ANY" or "ALL".
--- @field Include KindOfList|nil Optional. Defines objects KindOf to include, such as "SMALL_MISSILE", "AIRCRAFT", "SHIP", etc.
--- @field StatusBitFlags ObjectStatusList|nil Optional. Defines the status flags to include, such as "IGNORING_POWER_DOWN", "SHRUNK".
--- @field StatusBitFlagsExclude ObjectStatusList|nil Optional. Defines the status flags to exclude, such as "IGNORING_POWER_DOWN", "SHRUNK".

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

--- @class Coordinate
--- @field X integer The X coordinate.
--- @field Y integer The Y coordinate.
--- @field Z integer The Z coordinate.

--- JapanMissileMechaAdvanced 
---     TRANSFORMER VEHICLE
--- JapanInterceptorAircraft
---     SUBMARINE VEHICLE
--- 炮兵和重轰炸舰, 变形单位, 基地防御塔

--- FIGHTER_AIRCRAFT (包括3阵营6种飞机，不包括心神和樱花)
--- 未完整列出
--- @alias KindOf
--- | "SELECTABLE"
--- | "STRUCTURE"
--- | "AIRCRAFT"
--- | "FIGHTER_AIRCRAFT"
--- | "INFANTRY"
--- | "SHIP"
--- | "SUBMARINE"
--- | "VEHICLE"
--- | "SIEGE_WEAPON" 
--- | "TRANSFORMER" 
--- | "FS_BASE_DEFENSE"
--- | "EGG"

--- 未完整列出
--- @alias ObjectStatus
--- | "IMMOBILE"
--- | "IGNORE_AI_COMMAND"
--- | "REPAIR_ALLIES_WHEN_IDLE"
--- | "CLEARED_FOR_LANDING"
--- | "WATER_LOCOMOTOR_ACTIVE"
--- | "IS_FIRING_WEAPON"
--- | "IS_RELOADING_WEAPON"
--- | "UNSELECTABLE" 
--- | "WEAPON_UPGRADED_03"
--- | "NO_AUTO_ACQUIRE"
--- | "WEAPON_UPGRADED_01"
--- | "WEAPON_UPGRADED_02"
--- | "AIRBORNE_TARGET"
--- | "HIJACKED"
--- | "STEALTHED"
--- | "NO_COLLISIONS"
--- | "OVERCHARGING_WEAPON"
--- | "SUBMERGED"
--- | "SHROUD_REVEAL_TO_ALL"
--- | "SPECIALABILITY_ACTIVE"
--- | "DAMAGED"
--- | "REALLYDAMAGED"
--- | "HAS_SECONDARY_DAMAGE"
--- | "OVER_WATER"
--- | "POINT_DEFENSE_DRONE_ATTACHED"

--- 未完整列出
--- @alias SpecialPower
--- | "SpecialPower_AlliedFutureTankLaserWeapon"
--- | "SpecialPower_ShrinkRay"
--- | "SpecialPower_AANSTier3IceMissile"
--- | "SpecialPower_AlliedGunshipAircraftHyperSpaceMove"
--- | "SpecialPower_JAIV_Transform"

--- 未完整列出
--- @alias Weapon
--- | "SovietBomberAircraftDeathWeapon"
--- | "AlliedAntiStructureAttackDrones"
--- | "AlliedAntiNavyShipTech1DepthCharge"
--- | "AlliedSupportAircraftIceMissile"

--- 未完整列出
--- @alias ModelStatus 
--- | "ENGAGED" 
--- | "PREATTACK_E" 
--- | "SELECTED" 
--- | "RELOADING_A"
--- | "MOVING"
--- | "ATTACK_MOVING"
--- | "OVER_WATER"

--- @alias Comparison
--- | "<" 
--- | "LT" 
--- | "<=" 
--- | "LE" 
--- | ">"  
--- | "GT"
--- | ">="
--- | "GE" 
--- | "=="
--- | "EQ" 
--- | "~="
--- | "!="
--- | "NE" 

--- 未完整列出
--- @alias Player
--- |"<1st Human Player's Allies incl Self>"
--- |"<All Players>"
--- | "PlyrCivilian"
--- | "PlyrCreeps"
--- | "Player_1"
--- | "Player_2"
--- | "Player_3"
--- | "Player_4"
--- | "Player_5"
--- | "Player_6"

--- @meta Corona-LUA-API
--这个文件用来压制外部函数调用产生的未定义警告
--日冕lua包含lua标准库的函数 https://www.lua.org/manual/4.0/manual.html#6
--以及日冕自带的函数， 见日冕Lua.lua
---@diagnostic disable: unused-local
--返回对象

-- EA 提供的引擎函数：


-- IncludeThing和Include有一个有就行了，filter才有意义
-- Include是直接返回对应大类的所有单位
-- IncludeThing是把数组里的单位先找到最小共享的大类，然后搜索范围直接降到大类，然后一个一个检查对应大类单位并返回
-- IncludeThing 只包含1个单位的话只比Include慢1点
-- 但是IncludeThing 包含越多的话越慢，因为要检查每个单位，检查成本和包含的单位数成正比
-- 如果IncludeThing包含的单位跨过了多个大类，那就更慢了
-- 举例：
-- AIRCRAFT 大类， 有AlliedInterceptorAircraft, CelestialInterceptorAircraft, SovietInterceptorAircraft
-- IncludeThing = {"AlliedInterceptorAircraft"} 的话只比 Include = "AIRCRAFT" 慢1点
-- 但是 IncludeThing = {"AlliedInterceptorAircraft", "CelestialInterceptorAircraft", "SovietInterceptorAircraft"} 就会慢2点

--- Creates an object filter based on the provided rules.
--- @param filter_table FilterParams The table containing filter parameters.
--- @return ObjectFilter A filter object created based on the rules provided.
function CreateObjectFilter(filter_table)
    -- Implementation is hidden. This function is provided by the engine.
    return {}
end


-- 过滤器算法
-- 作者：公主大佬， 根据游戏引擎汇编代码得来，绝对准确
-- 1. 检查关系（Relationship）。
-- 假如过滤器定义了 Relationship，而且目标单位不满足 Relationship，
--   目标单位就【不符合】该过滤器。
-- 假如满足 Relationship，或者 Relationship 未定义，或者当前情况不适用 Relationship，则进入第二步。

-- 2. 检查目标单位是否在过滤器的 IncludeThing 列表内。
-- 假如目标单位在 IncludeThing 列表内，
--   目标单位就【符合】该过滤器。
-- 假如目标单位不在 IncludeThing 列表内，则进入第三步。

-- 3. 检查目标单位是否在过滤器的 ExcludeThing 列表内。
-- 假如目标单位在 ExcludeThing 列表内，
--   目标单位就【不符合】该过滤器。
-- 假如目标单位不在 ExcludeThing 列表内，则进入第四步。

-- 4. 检查目标单位的 KindOf 是否符合过滤器的 Exclude。
-- 假如目标单位的 KindOf 符合 Exclude，
--   目标单位就【不符合】该过滤器。
-- 假如目标单位的 KindOf 不符合 Exclude，则进入第五步。

-- 5. 根据过滤器的 Rule，检查目标单位的 KindOf 是否符合过滤器的 Include。
-- 假如 Rule 为 ALL，无论目标单位的 KindOf 是否符合 Include，则进入第六步。
-- 假如 Rule 为 ANY，只要目标单位的 KindOf 有任意一项符合 Include，则进入第六步。
-- 假如 Rule 为 ANY，而且目标单位的 KindOf 没有任何一项符合 Include，
--   目标单位就【不符合】该过滤器。假如 Include 为空，单位的 KindOf 就无法符合 Include。
-- 假如 Rule 写了但是不是 ALL或ANY，就会认为【不符合】该过滤器。
-- 假如 Rule 为 NONE，只要目标单位的所有 KindOf 都和过滤器的 Include 完全相同，则进入第六步。
-- 假如 Rule 为 NONE，而且目标单位的 KindOf 和过滤器的 Include 不完全相同，
--   目标单位就【不符合】该过滤器。

-- 6. 检查目标单位的状态（ObjectStatus）是否符合过滤器的 StatusBitFlags。
-- 假如过滤器定义了 StatusBitFlags，而且目标单位的状态没有任何一项符合 StatusBitFlags，
--   目标单位就【不符合】该过滤器。
-- 假如目标单位的状态有任何一项符合 StatusBitFlags，或者 StatusBitFlags 未定义，则进入第七步。

-- 7. 检查目标单位的状态（ObjectStatus）是否符合过滤器的 StatusBitFlagsExclude。
-- 假如过滤器定义了 StatusBitFlagsExclude，而且目标单位的状态有任何一项符合 StatusBitFlagsExclude，
--   目标单位就【不符合】该过滤器。
-- 否则，目标单位就【符合】该过滤器。



--这个函数可以找到这一帧刚刚被创建的对象，以及现存的对象
--- Finds objects in a specified area around a reference object, using the given filter.
--- @param iff_ref_object StandardUnitType|nil The reference object for Identifying Friend or Foe (IFF), if nil, all objects are considered.
--- @param area_parameter_table AreaParams|nil The table containing area parameters (e.g., radius, coordinates).
--- @param filter ObjectFilter|nil The filter to apply to the search.
--- @return StandardUnitType[]|nil matched_objects An array of StandardUnitType objects found in the search.
--- @return integer|"Error: GameObjects not found" count The number of objects found.
function ObjectFindObjects(iff_ref_object, area_parameter_table, filter)
    return {}, 0
end




--- 武器会在object位置原地自爆
function ObjectCreateAndFireTempWeapon(object, weapon)
end

--- This function return the number of frame passed since the game started.
--- 15 logical frames per second
---@return integer
function GetFrame()
    return 0
end

--- This function return the number of enemy objects in the specified radius around the object.
--- @param object StandardUnitType
--- @param radius number
--- @return integer
function ObjectCountNearbyEnemies(object, radius)
    return 0
end

--- 设置物体状态, 只能设置为真，设置为假需要ExecuteAction("UNIT_CHANGE_OBJECT_STATUS", current, status, bool)
--- @param object StandardUnitType 单位table
--- @param object_status ObjectStatus 物体状态
function ObjectSetObjectStatus(object, object_status)

end


-- 日冕LUA函数：
--- comment
--- @param name_in_world_builder string
--- @return StandardUnitType
function GetObjectByScriptName(name_in_world_builder)
    return {}
end

--- comment
--- @param object StandardUnitType
--- @return boolean
function ObjectIsAlive(object)
    return true
end

--- This function return the position of the object.
--- Warning, this function return float which may cause desync
--- @param object StandardUnitType
--- @return number, number, number
function ObjectGetPosition(object)
    return 0.0, 0.0, 0.0
end

--- Returns the previous position of the object.
--- If object is dead, this function will return nil.
--- @param object StandardUnitType
--- @return number|nil, number|nil, number|nil
function ObjectGetPreviousPosition(object)
    return 0,0,0
end

--- Return object by its ID
---@param id UnitID
---@return StandardUnitType
function GetObjectById(id)
    return {}
end

--- @param object StandardUnitType
--- @return Player|nil
function ObjectPlayerScriptName(object)
     --Ex: "Player_2", "Player_3" etc
end

--- 获取单位所属的队伍名, 注意不是玩家名而是地编里的队伍名
--- @param unit_table StandardUnitType 单位table
--- @return string
function ObjectTeamName(unit_table)
    return "" --Ex: "teamPlayer_1", "teamPlayer_2", etc
end

--- comment
--- @param object StandardUnitType
--- @return integer
function ObjectGetCurrentHealth(object)
    return 0
end

--- comment
--- @param object StandardUnitType
--- @return integer
function ObjectGetInitialHealth(object)
    return 0
end

--- comment
--- @param object1 StandardUnitType
--- @param object2 StandardUnitType
--- @return boolean
function ObjectsIsSame(object1, object2)
    return true
end




--- 测试单位是否满足某个模型状态
--- @param unit_table StandardUnitType 单位table
--- @param model_status ModelStatus
--- @return boolean
function  ObjectTestModelCondition(unit_table, model_status)
    return true
end
    
    


-- Find target of a unit. if it's not aiming at anyone, return nil
--- @param object StandardUnitType
--- @return StandardUnitType|nil
function ObjectFindTarget(object)
    return {}
end

-- Find target of a unit. if it's not aiming at anyone, return nil. Same as ObjectFindTarget
--- @param object StandardUnitType
--- @return StandardUnitType|nil
function ObjectGetTarget(object)
    return {}
end

--- @param object StandardUnitType
function ObjectTemplateName(object)
    return "" --Ex: {-1808793811}C4B2E7AF
    -- C4B2E7AF is the template name for HJ10(Qing Feng), 
    -- In filter it can be refered as   HexToUInt32("0xC4B2E7AF")
end

--- @param object StandardUnitType
function ObjectDescription(object)
    return "" -- Ex: Object 130 [{-1808793811}c4b2e7af, owned by player 3 (Ine)]
end

function HexToUInt32(hex)
    return 0 -- Ex: HexToUInt32("0xC4B2E7AF")
end

--- @param iff_object StandardUnitType|nil
--- @param target StandardUnitType
--- @param filter ObjectFilter
function ObjectTestTargetObjectWithFilter(iff_object, target, filter)
    return true
end

--- This function return list of passengers in the object.
--- @param object StandardUnitType
--- @return StandardUnitType[], integer
function ObjectGetContainedPassengers(object)
    return {}, 0
end


--- This function is an interface to execute an action via World Builder Script
--- @param action string
--- @param ... unknown
function ExecuteAction(action, ...)
end


--- This function is an interface to evaluate a condition via World Builder Script
--- @param condition string
--- @param ... unknown
--- @return boolean
function EvaluateCondition(condition, ...)
    local firstArg, secondArg = ...
    return true
end


--- 注意这虽然返回完整清单，但仍然比只计数的 ExecuteAction("SET_COUNTER_TO_NUMBER_OBJECTS_PLAYER_OWNS_WITH_KINDOF") 要快
--- MOBILE_GROUND_UNITS 包括变形单位，船和步兵，不包括飞机
--- VEHICLES 包括变形单位，船，船上的子炮塔，坦克型步兵，飞机，不包括普通步兵
---@param player_ref Player|StandardUnitType
---@param player_object_set "UNITS"|"STRUCTURES"|"UNITS_AND_STRUCTURES"|"BASE_DEFENSES"|"AIRCRAFT"|"SHIPS"
---@return StandardUnitType[], integer
function CopyPlayerRegisteredObjectSet(player_ref, player_object_set)
    return {},0
end

--- 疑似没用的global_object_set参数：
--- "TIBERIUM_FIELDS"|"SIGMA_FIELDS"|"GLOBAL_TIBERIUM_REFINERIES"|"GLOBAL_ORE_REFINERIES"|"GLOBAL_SIGMA_REFINERIES"
--- 有用的global_object_set参数：
--- @param global_object_set "GARRISONS"|"NEUTRAL_TECH"|"ORE_MINES"|"GLOBAL_HARVESTERS"
--- @return StandardUnitType[], integer
function CopyGlobalRegisteredObjectSet(global_object_set)
    return {},0
end

function HighResolutionClock()
    return 0
end



--这个函数可以创建武器并发射到位置，伤害目标。如果武器有AOE, 会伤害到目标周围的单位
--这个函数有一个可选的目标参数，如果不写这个参数，武器会直接发射到位置
--目标参数必须是敌方单位(相对object)，如果目标是友方单位，武器会直接走直线，不制导
--目标必须满足武器发射条件，比如苏霍伊导弹打天狗，只有天狗在空中才会发射
---@param object StandardUnitType
---@param weapon Weapon
---@param position Coordinate
---@param target StandardUnitType|nil
function ObjectCreateAndFireTempWeaponToTarget(object, weapon, position, target)
end

--这个函数有一个可选的持续时间参数，在modifier后面。代码改动记得退出再进入地图才能生效，直接重新加载地图会保留上次的修改
--注意如果duration写nil就不会用默认的duration了，要使用默认的duration就不要写这个参数（只输入两个参数）
function ObjectLoadAttributeModifier(object, modifier, duration)
end

--- @return AttributeModifier
function FindAttributeModifier(modifier_name)
end



--- 获取一个0-1范围内的伪随机数，这个数字对于局内的每一位玩家都是相同的，因此不会造成不同步
--- @return number
function GetRandomNumber() -- 
    return 0
end

-- 这个函数可以获得一个对象的数字ID，这个ID是唯一的，不会重复
---@param object StandardUnitType
---@return UnitID|nil
function ObjectGetId(object)
    local id = 0
    return id
end
-----已死亡的单位疑似没有GUID，所以这个函数可能会返回nil
-----if not ObjectIsAlive(current) then
----------local id = ObjectGetId(current)

--- Warning, this function return float which may cause desync
--- @param obj_self StandardUnitType
--- @param obj_other StandardUnitType
--- @return number
function ObjectsDistance3D(obj_self, obj_other)
    return 0
end



---这个函数是LUA4.0的标准库函数，用于向下取整
---计算可能会有误差, 但是不会导致不同步
---@param x number
---@return integer
function floor(x)
    return 0
end

---这个函数是LUA4.0的标准库函数，用于取模。计算可能会有误差, 但是不会导致不同步
---3.1415926 mod 3 = 0.1415926000000001
---60 mod 7 = 4, 
---60 mod 3 = 0
---@param x number IEEE754 double precision floating point number
---@param y number IEEE754 double precision floating point number
---@return number IEEE754 double precision floating point number
function mod(x, y)
    return 0.0
end

---这个函数是LUA4.0的标准库函数，用于开方
---Warning, this function return float which may cause desync
---@param x number
---@return number
function sqrt(x)
    return 0.0
end

function getn(table)
    return 0
end

function setglobal(name, value)
end



```
