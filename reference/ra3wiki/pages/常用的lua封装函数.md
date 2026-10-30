---
title: "常用的lua封装函数"
catalog_path: "Lua教程"
slug: akpr7lls9d1gf8cn
source: https://www.yuque.com/muzeqaq/ra3mapwiki/akpr7lls9d1gf8cn
fetched: 2026-10-15
content_updated_at: "2026-07-20T05:44:13.000Z"
type: Doc
format: lake
---

# 常用的lua封装函数

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/akpr7lls9d1gf8cn
> 目录位置：Lua教程

:::info
**作者：请叫我CY       如有疑问可以加群：613550502   找作者联系询问**

**更新日期：2026年6月11日**

:::

> 1.--给C2单位所在的玩家以及盟友的单位加BUFF
>
> 2.--命名为BOSS的单位向四周的敌方单位发射子弹
>
> 3.--调用函数,让"YAOGUANG"单位攻击"YADIANNA"单位附近的敌人并删除"YADIANNA"单位
>
> 4.--封装函数,设置特定玩家的特定类型单位的最大血量
>
> 5.--玩家联盟内塞单位
>
> 6.--封装函数,设置P1所在玩家的特定类型单位的弹药数量
>
> 7.--定义一个函数来升级特定类型的单位
>
> 8.--定义一个函数来查找盟友所有某类单位，这些单位会向周围的敌人开火
>
> 9.--封装函数-把名为p2的单位的玩家以及同盟处于QUYU1内所有单位的最大生命值调为1002
>
> 10.--封装函数-p2所在玩家的单位,如果有处于QUYU1区域内的,就传送到点AAA
>
> 11.--封装函数,设置与单位相同玩家的特定类型单位的最大血量
>
> 12.--攻击时附带武器
>
> 13.--封装函数,在P1所在玩家的某建筑前生产指定队伍的某种单位
>
> 14.--封装函数-在某点范围内随机生产单位
>
> 15.--封装函数,在P1所在玩家的某种建筑前方生产与该建筑相同队伍的单位
>
> 16.--单位攻击时给异常状态
>
> 17.--单位攻击带收买
>
> 18.--调用函数示例,让某单位对P1单位所属玩家以及盟友的,处于某状态的单位发射武器
>
> 19.--封装函数--把瞄准的单位替换为另一种单位
>
> 20.--封装函数-查找某玩家某种属性的某种单位(性能更优秀)
>
> 21.--封装函数--对P1所在玩家的步兵使用百夫长协议
>
> 22.--设置矿脉含量
>
> 23.--带内置CD的攻击附带
>
> 24.--封装函数-把一种单位替换成另一种单位（数量不要太多）
>
> 25.--围墙替换碉堡(不可出售与回收)
>
> 26.--玩家围墙变随机防御塔
>
> 27.--开局生产基地车并塞进世纪里面
>
> 28.--封装函数-分配队伍
>
> 29.--开局给玩家双精兵一车库并分配标志单位
>
> 30.--移除地图上所有玩家信标(包括不在单位上的)
>
> 31.--记录初始位置，并将初始位置相对位置持续保留的函数（拼模用）
>
> 32.--查找玩家的所有的某种类型的建筑，如果某单位数量不超过最大值，则在建筑前方80码处生成一个与建筑同队伍的单位
>
> 33.--切换阵营然后给基地车的封装函数
>
> 34.--设置单位位置和朝向
>
> 35.--利用过滤器实现P1所在玩家的超时空可以传送建筑
>
> 36.--利用创造函数实现玩家的超时空可以传送建筑
>
> 37.--设定基础脚本(玩家间关系，AI协议和升级以及禁止生产)
>
> 38.--死亡函数-子弹爆炸模板
>
> 39.--顶部技能按钮模板之战争银行
>
> 40.--通过射线法判断某个点在不在某个范围内
>







# 1.--给C2单位所在的玩家以及盟友的单位加BUFF
--具体的buff列表可以参考[https://www.yuque.com/muzeqaq/ra3mapwiki/chqogqlxe9pzb0k1](https://www.yuque.com/muzeqaq/ra3mapwiki/chqogqlxe9pzb0k1)

--[https://www.yuque.com/muzeqaq/ra3mapwiki/wpz5fwgb181qgy4r#6edd](https://www.yuque.com/muzeqaq/ra3mapwiki/wpz5fwgb181qgy4r#6edd)

--这两个网址中包含当前日冕可用的绝大多数buff

```lua
#!ra3luabridge
BuffUnitsForPlayer("C2", Filter_AIDeZhuRong, "AttributeMod_CelestialCoreBOSSAIUnitCheat", 90000)

#!ra3luabridge
function BuffUnitsForPlayer(scriptName, Filter, buffType, buffDuration)
    local PlayerUnit = GetObjectByScriptName(scriptName)
    local x, y, z = ObjectGetPosition(PlayerUnit)
    local units, count = ObjectFindObjects(PlayerUnit, {
        X = x, Y = y, Z = z,
        Radius = 20000,
        DistType = "EXTENTS_3D"
    }, Filter)
    if count < 1 then return end
    for i = 1, count do
        local unit = units[i]
        ObjectSetObjectStatus(unit, "NO_REFUND")
        ObjectLoadAttributeModifier(unit, buffType, buffDuration)
    end
end
Filter_AIDeZhuRong = CreateObjectFilter({
  Rule = "ANY",
  IncludeThing = {"SovietBomberAircraft"},
  Relationship = "SAME_PLAYER",
  StatusBitFlagsExclude = "NO_REFUND",
})
```



--随机发射几种弹药之一

--武器ID可以去[https://www.yuque.com/muzeqaq/ra3mapwiki/yipsr7vgtxahdk7n#6edd](https://www.yuque.com/muzeqaq/ra3mapwiki/yipsr7vgtxahdk7n#6edd) 中查找

```lua
#!ra3luabridge
FireSJWeaponAtEnemies("BOSS",weaponNameLD,1200)

#!ra3luabridge
function FireSJWeaponAtEnemies(unitName, weaponNames, range)
  local unit = GetObjectByScriptName(unitName)
  if not unit then return end
  local x, y, z = ObjectGetPosition(unit)
  local targets, targetCount = ObjectFindObjects(unit, {
    X = x, Y = y, Z = z,
    Radius = range,
    DistType = "EXTENTS_3D"
  }, Filter_attackArea)
  for j = 1, targetCount, 1 do
    local RandomUnit = ceil(GetRandomNumber()*WeaponCount)
    local weaponName = weaponNames[RandomUnit]
    local x3, y3, z3 = ObjectGetPosition(targets[j])
    ObjectCreateAndFireTempWeaponToTarget(unit, weaponName, { X = x3, Y = y3, Z = z3 }, targets[j])
  end
end
Filter_attackArea = CreateObjectFilter({
  Rule = "ALL",
  Relationship = "ENEMIES"
})
weaponNameLD = {
  "Celestial_Ship_Missile",
  "SovietAntiInfantryInfantryMolotovCocktail",
  "AlliedSupportAircraftIceMissile",
  "CelestialAntiStructureVehicleWeapon",
}
WeaponCount = getn(weaponNameLD)
```



# 2.--命名为BOSS的单位向四周的敌方单位发射子弹  
--武器ID可以去[https://www.yuque.com/muzeqaq/ra3mapwiki/yipsr7vgtxahdk7n#6edd](https://www.yuque.com/muzeqaq/ra3mapwiki/yipsr7vgtxahdk7n#6edd) 中查找

```lua
#!ra3luabridge
--单位名称、武器类型、范围
FireWeaponAtEnemies("BOSS", "AlliedSupportAircraftCryobeam", 1200)

#!ra3luabridge
function FireWeaponAtEnemies(unitName, weaponName, range)
  local unit = GetObjectByScriptName(unitName)
  if not unit then return end
  local Filter = CreateObjectFilter({
    Rule = "ALL",
    Relationship = "ENEMIES"
  })
  local x, y, z = ObjectGetPosition(unit)
  local attackArea = {
    X = x, Y = y, Z = z,
    Radius = range,
    DistType = "EXTENTS_3D"
  }
  local targets, targetCount = ObjectFindObjects(unit, attackArea, Filter)
  for j = 1, targetCount, 1 do
    ObjectLoadAttributeModifier(unit, "AttributeModifier_INVULNERABLE", 150)
    local x3, y3, z3 = ObjectGetPosition(targets[j])
    ObjectCreateAndFireTempWeaponToTarget(unit, weaponName, { X = x3, Y = y3, Z = z3 }, targets[j])
    ObjectLoadAttributeModifier(unit, "AttributeModifier_INVULNERABLE", 5)
  end
end
```







# 3.-- 调用函数,让"YAOGUANG"单位攻击"YADIANNA"单位附近的敌人并删除"YADIANNA"单位
--武器ID可以去[https://www.yuque.com/muzeqaq/ra3mapwiki/yipsr7vgtxahdk7n#6edd](https://www.yuque.com/muzeqaq/ra3mapwiki/yipsr7vgtxahdk7n#6edd) 中查找

```lua
#!ra3luabridge
AttackNearbyEnemies("YAOGUANG", "YADIANNA", "JapanAntiStructureShipWeapon", 1)

-- 封装AttackNearbyEnemies函数
#!ra3luabridge
function AttackNearbyEnemies(attackerName, targetName, weaponName, range)
  local attacker = GetObjectByScriptName(attackerName)
  local target = GetObjectByScriptName(targetName)
  if not attacker or not target then return end
  local Filter = CreateObjectFilter({
    Rule = "ALL",
    Relationship = "ENEMIES"
  })
  local x, y, z = ObjectGetPosition(target)  
  local attackArea = {
    X = x, Y = y, Z = z,
    Radius = range,
    DistType = "EXTENTS_3D"
  }
  local targets, targetCount = ObjectFindObjects(target, attackArea, Filter)
  ObjectLoadAttributeModifier(attacker, "AttributeModifier_INVULNERABLE", 150)
  for j = 1, targetCount, 1 do
    local x2, y2, z2 = ObjectGetPosition(targets[j])
    ObjectCreateAndFireTempWeaponToTarget(attacker, weaponName, { X = x2, Y = y2, Z = z2 }, targets[j])
  end
  ObjectLoadAttributeModifier(attacker, "AttributeModifier_INVULNERABLE",nil)
end
```





# 4.-- 调用--封装函数-- --封装函数,设置特定玩家的特定类型单位的最大血量
```lua
#!ra3luabridge
SetUnitMaxHealthForPlayer("PlyrCivilian", "AlliedAC130GunshipAircraft", "18888")

--封装函数本体
#!ra3luabridge
function SetUnitMaxHealthForPlayer(playerID, unitType, maxHealth)
    local Filter = CreateObjectFilter({
        IncludeThing = { unitType },
    })
    local units, count = ObjectFindObjects(nil, nil, Filter)
    for i = 1, count do
        local unit = units[i]
        local playerName = ObjectPlayerScriptName(unit)
        if playerName == playerID then
            if not EvaluateCondition("UNIT_HAS_OBJECT_STATUS", unit, "PLAYER_POWER_2") then
                ExecuteAction("UNIT_CHANGE_OBJECT_STATUS", unit, "PLAYER_POWER_2", "TRUE")
                ExecuteAction("DEBUG_STRING", "Lua Alert: " .. (playerName or "debug string"))
                -- 生成唯一的单位引用名称
                local objectId = ObjectGetId(unit)
                local str = "unit_" .. objectId
                ExecuteAction("SET_UNIT_REFERENCE", str, unit)
                ExecuteAction("NAMED_SET_MAX_HEALTH", str, maxHealth, "TRUE")
            end
        end
    end
end
```



# 5.--玩家联盟内塞单位
--找到属于Player_1的SovietAntiVehicleVehicleTech4单位,生成4个属于Player_1/team1的单位AlliedAntiVehicleInfantry塞到SovietAntiVehicleVehicleTech4里面,然后Player_1/team1加入Player_1/teamPlayer_1

-- --封装函数,设置特定玩家的特定类型单位的状态,并在单位位置生成新的单位

-- 玩家必须两个队伍都有,否则不会生效

```lua
-- 调用--封装函数
#!ra3luabridge
SetUnitStatusAndSpawnUnits("P1", Filter_AIDeLianMeng, "SovietAntiVehicleInfantry", "PlyrCreeps/A_LM1", "PlyrCreeps/A_LM2", 3)

#!ra3luabridge
function SetUnitStatusAndSpawnUnits(unitName, Filter, unitType1, playerTeam1, playerTeam2, numSpawns)
  local playerUnit = GetObjectByScriptName(unitName)
    local units, count = ObjectFindObjects(playerUnit, {
        X = 4000, Y = 4000, Z = 200,
        Radius = 20000,
        DistType = "EXTENTS_3D"
      }, Filter)
      if count < 1 then return end
    for i = 1, count,1 do
      local objectId = ObjectGetId(units[i])
      local str="unit_"..objectId
      ObjectSetObjectStatus(units[i], "NO_REFUND")
      ExecuteAction("SET_UNIT_REFERENCE", str, units[i])
      for j = 1, numSpawns do
        ExecuteAction("UNIT_SPAWN_NAMED_OBJECT_ON_TEAM_AT_NAMED_OBJECT_LOCATION", "", unitType1, playerTeam2, str)
      end
      ExecuteAction("UNIT_SPAWN_NAMED_OBJECT_ON_TEAM_AT_NAMED_OBJECT_LOCATION", "", "AlliedCryoLegionnaire", playerTeam2, str)
      ExecuteAction("UNIT_SPAWN_NAMED_OBJECT_ON_TEAM_AT_NAMED_OBJECT_LOCATION", "", "AlliedCryoLegionnaire", playerTeam2, str)
      ExecuteAction("TEAM_SET_EXPERIENCE_POINTS", playerTeam2, "666666")
      ExecuteAction("TEAM_SET_STANCE", playerTeam2, "AGGRESSIVE")
      ExecuteAction("TEAM_GARRISON_SPECIFIC_BUILDING_INSTANTLY", playerTeam2, str)
      ExecuteAction("TEAM_MERGE_INTO_TEAM", playerTeam2, playerTeam1)
    end
end
Filter_AIDeLianMeng = CreateObjectFilter({
  Rule="ANY",
  IncludeThing = {"SovietAntiVehicleVehicleTech4"},
  Relationship="SAME_PLAYER",
  StatusBitFlagsExclude = "NO_REFUND", 
})
```



# 6.--封装函数,设置P1所在玩家的特定类型单位的弹药数量
```lua
-- 调用--封装函数
#!ra3luabridge
SetAmmoForUnits("P1", "JapanAntiVehicleVehicleTech1", 50)

#!ra3luabridge
function SetAmmoForUnits(unitName, unitType, ammoCount)
  local filter = CreateObjectFilter({
    Rule="ANY",
    IncludeThing={unitType},
    Relationship="SAME_PLAYER",
  })
  local playerUnit = GetObjectByScriptName(unitName)
  if not playerUnit then
    ExecuteAction("DEBUG_STRING", "Lua Alert: Player unit not found")
    return
  end
  local x, y, z = ObjectGetPosition(playerUnit)
  local searchArea = {
    X = x, Y = y, Z = z,
    Radius = 20000,
    DistType = "EXTENTS_3D"
  }
  local units, count = ObjectFindObjects(playerUnit, searchArea, filter)
  if count > 0 then
    ExecuteAction("DEBUG_STRING", "count: " .. (count or "debug string"))
    for i = 1, count do
      local unit = units[i]
      ObjectSetWeaponSetUpdateWeaponCurrentAmmoCount(unit, 1, 1, 1, ammoCount)
      ObjectSetWeaponSetUpdateWeaponCurrentAmmoCount(unit, 1, 1, 2, ammoCount)
      ObjectSetWeaponSetUpdateWeaponCurrentAmmoCount(unit, 1, 1, 3, ammoCount)
      ObjectSetWeaponSetUpdateWeaponCurrentAmmoCount(unit, 1, 1, 4, ammoCount)
    end
  else
    ExecuteAction("DEBUG_STRING", "Lua Alert: No " .. unitType .. " units found within range")
  end
end
```



# 7.-- 定义一个函数来升级特定类型的单位
```lua
#!ra3luabridge
UpgradeUnits("p2", "SovietAntiVehicleVehicleTech1", 9000)

#!ra3luabridge
function UpgradeUnits(scriptName, unitType, newHealth)
  local TCF = CreateObjectFilter({
    Rule="ANY",
    IncludeThing={unitType},
    Relationship="ALLIES",
  })
  local obj = GetObjectByScriptName(scriptName)
  local x, y, z = ObjectGetPosition(obj)
  local area = {
    X = x, Y = y, Z = z,
    Radius = 10000, -- 默认半径值
    DistType = "EXTENTS_3D"
  }
  local units, count = ObjectFindObjects(obj, area, TCF)
  for i = 1, count do
    local unit = units[i]
    if not EvaluateCondition("UNIT_HAS_OBJECT_STATUS", unit, "PLAYER_POWER_2") then
      ExecuteAction("UNIT_CHANGE_OBJECT_STATUS", unit, "PLAYER_POWER_2", "TRUE")
      -- 生成唯一的单位引用名称
      local objectId = ObjectGetId(unit)
      local unitRef = "unit_" .. objectId
      ExecuteAction("SET_UNIT_REFERENCE", unitRef, unit)
      ExecuteAction("NAMED_SET_MAX_HEALTH", unitRef, newHealth, "TRUE")
      ExecuteAction("UNIT_GIVE_EXPERIENCE_POINTS", unitRef, 666666)
    end
  end
end
```



# 8.-- 定义一个函数来查找盟友所有某类单位，这些单位会向周围的敌人开火
--武器ID可以去[https://www.yuque.com/muzeqaq/ra3mapwiki/yipsr7vgtxahdk7n#6edd](https://www.yuque.com/muzeqaq/ra3mapwiki/yipsr7vgtxahdk7n#6edd) 中查找

```lua
#!ra3luabridge
FindAlliesAndAttackEnemies("p2", Filter_JapanBaseDefenseAdvancedEgg, "Celestial_Ship_Missile", "AlliedInterceptorAircraftTech3ProtonMissile", 900, 5)

--封装函数如下↓
#!ra3luabridge
--全局变量(Filter_Attack不要动)
Filter_JapanBaseDefenseAdvancedEgg = CreateObjectFilter({
  Rule = "ANY",
  IncludeThing = {"JapanBaseDefenseAdvancedEgg"},
  Relationship = "SAME_PLAYER",
})
Filter_Attack = CreateObjectFilter({
    Rule="ANY",
    Include="INFANTRY STRUCTURE CAN_ATTACK",
    Relationship="ENEMIES"
  })

--封装函数本体
--scriptName：参考单位的名称
--Filter：开火单位的过滤器
--weaponName_D：对地武器的ID
--weaponName_K：对空武器的ID
--attackRadius：攻击范围
--loopStep：密集程度，越小发射越密集，最小为1
function FindAlliesAndAttackEnemies(scriptName, Filter, weaponName_D, weaponName_K, attackRadius, loopStep)
  local Tesla = GetObjectByScriptName(scriptName)
  local TC, Pcount = ObjectFindObjects(Tesla, {
    X = 4000, Y = 4000, Z = 200,
    Radius = 20000,
    DistType = "EXTENTS_3D"
  }, Filter)
  if Pcount < 1 then return end
  for i = 1, Pcount, 1 do
    local unit = TC[i]
    local x, y, z = ObjectGetPosition(unit)
    local YGA, targetCount = ObjectFindObjects(unit, {
      X = x, Y = y, Z = z,
      Radius = attackRadius,
      DistType = "EXTENTS_3D"
    }, Filter_Attack)
    if targetCount > 0 then
      for j = 1, targetCount, loopStep do
        local x2, y2, z2 = ObjectGetPosition(YGA[j])
        if not EvaluateCondition("UNIT_HAS_OBJECT_STATUS", YGA[j], "AIRBORNE_TARGET") then
          ObjectCreateAndFireTempWeaponToTarget(unit, weaponName_D, {
            X = x2, Y = y2, Z = z2
          }, YGA[j])
        else
          ObjectCreateAndFireTempWeaponToTarget(unit, weaponName_K, {
            X = x2, Y = y2, Z = z2
          }, YGA[j])
        end
      end
    end
  end
end
```



# 9.--封装函数-把名为p2的单位的玩家以及同盟处于QUYU1内所有单位的最大生命值调为1002
```lua
#!ra3luabridge
SetUnitMaxHealthInArea("p2", "QUYU1", "1002")

#!ra3luabridge
function SetUnitMaxHealthInArea(scriptName, Area, maxHealth)
  local Filter = CreateObjectFilter({
    Rule="ALL",
    Relationship="ALLIES",
  })
  local obj = GetObjectByScriptName(scriptName)
  local x, y, z = ObjectGetPosition(obj)
  local FanWei = {
    X = x, Y = y, Z = z,
    Radius = 20000,
    DistType = "EXTENTS_3D"
  }
  local units, count = ObjectFindObjects(obj, FanWei, Filter)
  for i = 1, count do
      local unit = units[i]
      local playerName = ObjectPlayerScriptName(unit)
          if not EvaluateCondition("UNIT_HAS_OBJECT_STATUS", unit, "PLAYER_POWER_2") then
            if EvaluateCondition("NAMED_INSIDE_AREA", unit, Area) then
              ExecuteAction("UNIT_CHANGE_OBJECT_STATUS", unit, "PLAYER_POWER_2", "TRUE")
              ExecuteAction("DEBUG_STRING", "Lua Alert: " .. (playerName or "debug string"))
              -- 生成唯一的单位引用名称
              local objectId = ObjectGetId(unit)
              local str = "unit_" .. objectId
              ExecuteAction("SET_UNIT_REFERENCE", str, unit)
              ExecuteAction("NAMED_SET_MAX_HEALTH", str, maxHealth, "TRUE")
          end
      end
  end
end
```



# 10.--封装函数-p2所在玩家的单位,如果有处于QUYU1区域内的,就传送到点AAA
```lua
#!ra3luabridge
SendUnitInAreaToPoint("p2", "QUYU1", "AAA")


#!ra3luabridge
function SendUnitInAreaToPoint(scriptName, Area, Point)
  local Filter1 = CreateObjectFilter({
    Rule="ALL",
    Relationship="SAME_PLAYER",
  })
  local obj = GetObjectByScriptName(scriptName)
  local x, y, z = ObjectGetPosition(obj)
  local ChuanSong = {
    X = x, Y = y, Z = z,
    Radius = 20000,
    DistType = "EXTENTS_3D"
  }
  local units, count = ObjectFindObjects(obj, ChuanSong, Filter1)
  for i = 1, count do
    local unit = units[i]
    local playerName = ObjectPlayerScriptName(unit)
      if EvaluateCondition("NAMED_INSIDE_AREA", unit, Area) then
         local objectId = ObjectGetId(unit)
         local str = "unit_" .. objectId
         ExecuteAction("SET_UNIT_REFERENCE", str, unit)
         ExecuteAction("UNIT_TELEPORT_TO_WAYPOINT", str, Point)
      end
  end
end
```



# 11.-- 调用--封装函数-- --封装函数,设置与单位相同玩家的特定类型单位的最大血量
```lua
#!ra3luabridge
SetUnitMaxHealthForPlayer("C1", Filter_AlliedSupportAircraft, "4000")

-- --封装函数本体
#!ra3luabridge
function SetUnitMaxHealthForPlayer(scriptName, filter, maxHealth)
  local obj = GetObjectByScriptName(scriptName)
  local x, y, z = ObjectGetPosition(obj)
  local units, count = ObjectFindObjects(obj, {
    X = x, Y = y, Z = z,
    Radius = 20000,
    DistType = "EXTENTS_3D"
  }, filter)
  
  for i = 1, count do
    local unit = units[i]
    ObjectSetObjectStatus(unit, "NO_REFUND")
    local objectId = ObjectGetId(unit)
    local str = "unit_" .. objectId
    ExecuteAction("SET_UNIT_REFERENCE", str, unit)
    ExecuteAction("NAMED_SET_MAX_HEALTH", str, maxHealth, "TRUE")
  end
end

Filter_AlliedSupportAircraft = CreateObjectFilter({
  Rule = "ANY",
  IncludeThing = {"AlliedSupportAircraft"},
  Relationship = "SAME_PLAYER",
  StatusBitFlagsExclude = "NO_REFUND"
})
```





--p2所在玩家的单位，如果有至少5个单位处于区域QUYU1内，则将这些单位传送到点AA中

```lua
#!ra3luabridge
SendUnitInAreaToNumber("p2", "QUYU1", 5,"QUYU1","AAA")


#!ra3luabridge
function SendUnitInAreaToNumber(scriptName, Area1, limit, Area2, Point)
  local Filter2 = CreateObjectFilter({
    Rule="ALL",
    Relationship="SAME_PLAYER",
  })
  local WJDW = GetObjectByScriptName(scriptName)
  local x, y, z = ObjectGetPosition(WJDW)
  local ChuanSong = {
    X = x, Y = y, Z = z,
    Radius = 20000,
    DistType = "EXTENTS_3D"
  }
  local number = 0
  local units, count = ObjectFindObjects(WJDW, ChuanSong, Filter2)
  for i = 1, count do
    local unit = units[i]
    local playerName = ObjectPlayerScriptName(unit)
    if EvaluateCondition("NAMED_INSIDE_AREA", unit, Area1) then
       number = number + 1
    end
  end
  if number <= limit then
    SendUnitInAreaToPoint(scriptName, Area2, Point)
  end
  ExecuteAction("DEBUG_STRING", "Lua Alert: " .. (number or "debug string"))
  return number
  
end

function SendUnitInAreaToPoint(scriptName, Area, Point)
  local Filter1 = CreateObjectFilter({
    Rule="ALL",
    Relationship="SAME_PLAYER",
  })
  local obj = GetObjectByScriptName(scriptName)
  local x, y, z = ObjectGetPosition(obj)
  local ChuanSong = {
    X = x, Y = y, Z = z,
    Radius = 20000,
    DistType = "EXTENTS_3D"
  }
  local units, count = ObjectFindObjects(obj, ChuanSong, Filter1)
  for i = 1, count do
    local unit = units[i]
    local playerName = ObjectPlayerScriptName(unit)
      if EvaluateCondition("NAMED_INSIDE_AREA", unit, Area) then
         local objectId = ObjectGetId(unit)
         local str = "unit_" .. objectId
         ExecuteAction("SET_UNIT_REFERENCE", str, unit)
         ExecuteAction("UNIT_TELEPORT_TO_WAYPOINT", str, Point)
      end
  end
end
```



# 12.--攻击时附带武器
--武器ID可以去[https://www.yuque.com/muzeqaq/ra3mapwiki/yipsr7vgtxahdk7n#6edd](https://www.yuque.com/muzeqaq/ra3mapwiki/yipsr7vgtxahdk7n#6edd) 中查找

```lua
-- 使用示例：
#!ra3luabridge
FireAtEnemiesInRange("P1", Filter_FiringAircraft, "SovietInterceptorAircraftFlakMissile")

#!ra3luabridge
function FireAtEnemiesInRange(scriptName, Filter, weaponTemplateName)
  local unit = GetObjectByScriptName(scriptName)
  local enemies, ecount = ObjectFindObjects(unit, {
    X = 4000, Y = 4000, Z = 200,
    Radius = 20000, 
    DistType = "EXTENTS_3D"
  }, Filter)
  if ecount < 1 then return end
  for i = 1, ecount, 1 do
    local enemyUnit = enemies[i]
    local targetUnit = ObjectGetTarget(enemyUnit)
    if targetUnit then
      local x1, y1, z1 = ObjectGetPosition(targetUnit)
      ObjectCreateAndFireTempWeaponToTarget(enemyUnit, weaponTemplateName, {
        X = x1, Y = y1, Z = z1
      }, targetUnit)
    end
  end
end
Filter_FiringAircraft = CreateObjectFilter({
  Rule = "ANY",
  IncludeThing = {"CelestialInterceptorAircraft"},
  Relationship = "ALLIES",
  StatusBitFlags = "IS_FIRING_WEAPON",
})
```



# 13.-- 调用--封装函数-- --封装函数,在P1所在玩家的某建筑前生产指定队伍的某种单位
```lua
#!ra3luabridge
ConstructionPrepUnit("C2","AlliedWarFactory","AlliedAntiVehicleVehicleTech1","PlyrCreeps/SHZ")

-- --封装函数本体
#!ra3luabridge
function ConstructionPrepUnit(scriptName, unitType, tankType, playerTeam)
  local Filter1 = CreateObjectFilter({
        IncludeThing = { unitType },
        Relationship="SAME_PLAYER",
    })
  local obj1 = GetObjectByScriptName(scriptName)
  local x, y, z = ObjectGetPosition(obj1)
  local ChuanSong1 = {
    X = x, Y = y, Z = z,
    Radius = 20000,
    DistType = "EXTENTS_3D"
  }
  local units, count = ObjectFindObjects(obj1, ChuanSong1, Filter1)
  if count > 0 then
    for i = 1, count do
      local unit = units[i]
      local x1, y1, z1 = ObjectGetPosition(unit)
      local matrix = ObjectGetTransform(unit)
      local direction2D = { X=matrix[1], Y=matrix[5] }
      local angle = atan2(direction2D.Y, direction2D.X)
      ExecuteAction("DEBUG_STRING", "Lua Alert: " .. (angle or "debug string"))
      ExecuteAction("CREATE_OBJECT",tankType,playerTeam,{X=x1+80*cos(angle),Y=y1+80*sin(angle),Z=z1},angle)
    end
  end
  ExecuteAction("TEAM_SET_EXPERIENCE_POINTS", playerTeam, "666666")
end
```



# 14.--封装函数-在某点范围内随机生产单位
```lua
#!ra3luabridge
local units = {
  "JapanAntiStructureVehicle",
  "AlliedAntiStructureVehicle",
  "CelestialAntiInfantryInfantryAdvanced",
  "SovietAntiStructureVehicle",
  "SovietAntiVehicleVehicleTech4"
}--添加单位的名称
CreateRandomUnit(3397.27, 3157.61, 210,    --XYZ轴坐标
"Player_1/teamPlayer_1",                   --设置玩家队伍
100,                                       --圆心半径
units,                                     --单位列表(这里不要改,从上面改)
"180"                                      --生产单位朝向角度
)

#!ra3luabridge
function CreateRandomUnit(Xaxis, Yaxis, Zaxis, playerTeam, maxDistance, units, Degree)
    ExecuteAction("DEBUG_STRING", "Lua Alert: X axis - " .. (Xaxis or "undefined"))
    ExecuteAction("DEBUG_STRING", "Lua Alert: Y axis - " .. (Yaxis or "undefined"))
    ExecuteAction("DEBUG_STRING", "Lua Alert: Z axis - " .. (Zaxis or "undefined"))
    local Distance = ceil(GetRandomNumber() * maxDistance / 50) * 50
    ExecuteAction("DEBUG_STRING", "Lua Alert: Distance - " .. (Distance or "undefined"))
    local Angle = 181 - ceil(GetRandomNumber() * 361)
    ExecuteAction("DEBUG_STRING", "Lua Alert: Angle - " .. (Angle or "undefined"))
    local unitCount = getn(units)
    local RandomUnit = ceil(GetRandomNumber()*unitCount)
    local unitName = units[RandomUnit]
    local x = Xaxis + Distance * cos(Angle)
    local y = Yaxis + Distance * sin(Angle)
    ExecuteAction("CREATE_OBJECT", unitName, playerTeam, {X = x, Y = y, Z = Zaxis}, Degree)
end
```



# 15.--封装函数,在P1所在玩家的某种建筑前方生产与该建筑相同队伍的单位
```lua
#!ra3luabridge
MakingSameTeamUnits("C1",Filter_sovietairfield,"JapanEmperorMecha")

#!ra3luabridge
function MakingSameTeamUnits(scriptName, Filter, DivisionType)
  local Facility = GetObjectByScriptName(scriptName)
  local x, y, z = ObjectGetPosition(Facility)
  local ChuanSong1 = {
    X = x, Y = y, Z = z,
    Radius = 10000,
    DistType = "EXTENTS_3D"
   }
  local Divisions, count = ObjectFindObjects(Facility, ChuanSong1, Filter)
  for i = 1, count do
    local Division = Divisions[i]
    local x1, y1, z1 = ObjectGetPosition(Division)
    local matrix = ObjectGetTransform(Division)
    local direction2D = { X=matrix[1], Y=matrix[5] }
    local angle = atan2(direction2D.Y, direction2D.X)
    local Team = ObjectTeamName(Division)
    local player = ObjectPlayerScriptName(Division)
    local playerTeam = player .. "/" .. Team
    ExecuteAction("CREATE_OBJECT",DivisionType,playerTeam,{X=x1+80*cos(angle),Y=y1+80*sin(angle),Z=z1},angle)
  end
end

Filter_sovietairfield = CreateObjectFilter({
  IncludeThing = { "sovietairfield" },
  Relationship="SAME_PLAYER",
})
```



# 16.--单位攻击时给异常状态
```lua
#!ra3luabridge
ApplyStatusToEnemyUnits("C2", "SovietCommandoTech1", "UNMANNED")

#!ra3luabridge
function ApplyStatusToEnemyUnits(scriptName, unitType, statusName)
  local playerUnit = GetObjectByScriptName(scriptName)
  if not playerUnit then
    ExecuteAction("DEBUG_STRING", "Lua Alert: Player unit not found")
    return
  end

  local filter = CreateObjectFilter({
    Rule = "ANY",
    IncludeThing = {unitType},
    Relationship = "SAME_PLAYER",
  })
  
  local x, y, z = ObjectGetPosition(playerUnit)
  local searchArea = {
    X = x, Y = y, Z = z,
    Radius = 20000, 
    DistType = "EXTENTS_3D"
  }
  
  local foundEnemies, enemyCount = ObjectFindObjects(playerUnit, searchArea, filter)
  for i = 1, enemyCount do
    local currentEnemy = foundEnemies[i]
    local enemyTarget = ObjectGetTarget(currentEnemy)
    if enemyTarget then
      if EvaluateCondition("UNIT_HAS_OBJECT_STATUS", currentEnemy, "IS_FIRING_WEAPON") then
        ExecuteAction("NAMED_SET_DISABLED", enemyTarget, statusName, true)
      end
    end
  end
end
```





# 17.--单位攻击带收买
-- 调用函数示例

```lua
#!ra3luabridge
ChangeAttackedUnitTeam("C2", "AlliedAntiVehicleVehicleTech1")

#!ra3luabridge
function ChangeAttackedUnitTeam(scriptName, unitType)
  local sourceUnit = GetObjectByScriptName(scriptName)
  if not sourceUnit then
    ExecuteAction("DEBUG_STRING", "Lua Alert: Source unit not found")
    return
  end
  local searchFilter = CreateObjectFilter({
    Rule = "ANY",
    IncludeThing = {unitType},
    Relationship = "SAME_PLAYER",
  })
  local x, y, z = ObjectGetPosition(sourceUnit)
  local searchRegion = {
    X = x, Y = y, Z = z,
    Radius = 20000, 
    DistType = "EXTENTS_3D"
  }
  local enemyUnits, enemyCount = ObjectFindObjects(sourceUnit, searchRegion, searchFilter)
  for i = 1, enemyCount do
    local enemy = enemyUnits[i]
    local target = ObjectGetTarget(enemy)
    if target then
      if EvaluateCondition("UNIT_HAS_OBJECT_STATUS", enemy, "IS_FIRING_WEAPON") then
        local enemyTeam = ObjectTemName(enemy)
        local enemyPlayer = ObjectPlayerScriptName(enemy)
        local newTeamString = enemyPlayer .. "/" .. enemyTeam
        ExecuteAction("UNIT_SET_TEAM", target, newTeamString)
      end
    end
  end
end
```



# 18.--调用函数示例,让某单位对P1单位所属玩家以及盟友的,处于某状态的单位发射武器
```lua
#!ra3luabridge
FindUnitsWithStatusAndFireWeapon("P1", "YAOGUANG", "WeaponName", "UNDER_FROZEN")

#!ra3luabridge
function FindUnitsWithStatusAndFireWeapon(scriptName, targetUnitName, weaponName, statusToCheck)
  local playerUnit = GetObjectByScriptName(scriptName)
  local yaoguangUnit = GetObjectByScriptName(targetUnitName)

  local filter = CreateObjectFilter({
    Rule = "ALL",
    Relationship = "ALLIES",
  })
  
  local x, y, z = ObjectGetPosition(playerUnit)
  local searchArea = {
    X = x, Y = y, Z = z,
    Radius = 20000, 
    DistType = "EXTENTS_3D"
  }
  
  local alliedUnits, alliedCount = ObjectFindObjects(playerUnit, searchArea, filter)
  for i = 1, alliedCount do
    local currentUnit = alliedUnits[i]
    if EvaluateCondition("UNIT_HAS_OBJECT_STATUS", currentUnit, statusToCheck) then 
      local posX, posY, posZ = ObjectGetPosition(currentUnit)
      ObjectLoadAttributeModifier(yaoguangUnit, "AttributeModifier_INVULNERABLE", 150)
      ObjectCreateAndFireTempWeaponToTarget(yaoguangUnit, weaponName, {
        X = posX, Y = posY, Z = posZ
      }, currentUnit)
      ObjectLoadAttributeModifier(yaoguangUnit, "AttributeModifier_INVULNERABLE", 5)
    end
  end
end
```







# 19.--封装函数--把瞄准的单位替换为另一种单位
```lua
#!ra3luabridge
ReplaceTargetUnit("C2","CAN_ATTACK","UnitCrate","PlyrCreeps/teamPlyrCreeps")

#!ra3luabridge
function ReplaceTargetUnit(scriptName,unitType,unitTypeToSpawn,alliedTeam)
  local sourceUnit = GetObjectByScriptName(scriptName)
  if not sourceUnit then
    ExecuteAction("DEBUG_STRING", "Lua Alert: Source unit not found")
    return
  end
  local searchFilter = CreateObjectFilter({
    Rule = "ANY",
    IncludeThing = {unitType},
    Relationship = "SAME_PLAYER",
  })
  local x, y, z = ObjectGetPosition(sourceUnit)
  local searchRegion = {
    X = x, Y = y, Z = z,
    Radius = 20000, 
    DistType = "EXTENTS_3D"
  }
  local enemyUnits, enemyCount = ObjectFindObjects(sourceUnit, searchRegion, searchFilter)
  ExecuteAction("DEBUG_STRING", "单位数量t: " .. (enemyCount or "debug string"))
  for i = 1, enemyCount do
    local enemy = enemyUnits[i]
    local target = ObjectGetTarget(enemy)
    if target then
      if EvaluateCondition("UNIT_HAS_OBJECT_STATUS", enemy, "IS_FIRING_WEAPON") then
        local objectId = ObjectGetId(target)
        local str = "target_" .. objectId
        ExecuteAction("SET_UNIT_REFERENCE", str, target)
        ExecuteAction("UNIT_SPAWN_NAMED_OBJECT_ON_TEAM_AT_NAMED_OBJECT_LOCATION", "",unitTypeToSpawn,alliedTeam,str)
        ExecuteAction("NAMED_DELETE", str)
      end
    end
  end
end
```



# 20.--封装函数-查找某玩家某种属性的某种单位(性能更优秀)
```lua
#!ra3luabridge
CountSpecificUnitsForPlayer("Player_1", "UNITS", "AlliedAntiVehicleVehicleTech1")

#!ra3luabridge
function CountSpecificUnitsForPlayer(playerName, unitKind, unitType)
  local units, unitCount = CopyPlayerRegisteredObjectSet(playerName, unitKind)
  local filter = CreateObjectFilter({
    Rule = "ANY",
    IncludeThing = {unitType},
  })
  local matchingUnitCount = 0
  for i = 1, unitCount do
    local currentUnit = units[i]
    if ObjectTestTargetObjectWithFilter(nil, currentUnit, filter) then
      matchingUnitCount = matchingUnitCount + 1
    end
  end
  ExecuteAction("DEBUG_STRING", "Lua Alert: Found " .. matchingUnitCount .. " " .. unitType .. " for " .. playerName)
end
```



# 21.--封装函数--对P1所在玩家的步兵使用百夫长协议
```lua
#!ra3luabridge
UpgradeUnitCenturion("C1", Filter_INFANTRY, "Command_Celestial_CenturionUpgrade")
GiveStatus("C1", Filter_INFANTRY)

#!ra3luabridge
--魔导书的过滤器
Filter_playerspellbook = CreateObjectFilter({
    Rule = "ANY",
    IncludeThing = { "playerspellbook" },
    Relationship = "SAME_PLAYER",
})
--步兵的过滤器
Filter_INFANTRY = CreateObjectFilter({
    Rule = "ANY",
    Include = "INFANTRY",
    Relationship = "SAME_PLAYER",
    StatusBitFlagsExclude = "NO_REFUND",
})

--查找魔导书，并使用魔导书对单位使用某种技能
function UpgradeUnitCenturion(unitName, Filter, specialability,ZT)
    local playerUnit = GetObjectByScriptName(unitName)
    local UnitPlayer = ObjectPlayerScriptName(playerUnit)
    local units, unitCount = ObjectFindObjects(playerUnit, nil, Filter_playerspellbook)
    local objectId = ObjectGetId(units[1])
    local MDSname = "unit" .. objectId
    ExecuteAction("SET_UNIT_REFERENCE", MDSname, units[1])
    NameAndUseSpecialAbilityOnInfantry(unitName, Filter, MDSname, specialability,ZT)
end

--查找单位,并用魔导书对对应单位释放技能
function NameAndUseSpecialAbilityOnInfantry(unitName, Filter, nameofunit, specialability, ZT)
    local playerUnit = GetObjectByScriptName(unitName)
    local units_INFANTRY, count = ObjectFindObjects(playerUnit, {
        X = 4000,
        Y = 4000,
        Z = 200,
        Radius = 10000,
        DistType = "EXTENTS_3D"
    }, Filter)
    for i = 1, count do
        local objectId = ObjectGetId(units_INFANTRY[i])
        local str = "target_" .. objectId
        ExecuteAction("SET_UNIT_REFERENCE", str, units_INFANTRY[i])
        ExecuteAction("NAMED_USE_COMMANDBUTTON_ABILITY_ON_NAMED", nameofunit, specialability, str)
    end
    if ZT == 1 then
        for j = 1, count do
            GiveStatus(units_INFANTRY[j])
        end
    end
end
--给标记状态并给BOSSai的BUFF
function GiveStatus(unit)
    local x, y, z = ObjectGetPosition(unit)
    ObjectCreateAndFireTempWeaponToTarget(unit, "PlayerPowerPointDefenseDronesWeapon_Individual", { X = x, Y = y, Z = z }, unit)
    ObjectSetObjectStatus(unit, "NO_REFUND")
    ObjectLoadAttributeModifier(unit, "AttributeMod_GenericBOSSAIUnitCheat", 90000)
end



Filter_playerspellbook = CreateObjectFilter({
  Rule = "ANY",
  IncludeThing = {"playerspellbook"},
  Relationship = "SAME_PLAYER",
})

Filter_INFANTRY = CreateObjectFilter({
  Rule = "ANY",
  Include = "INFANTRY",
  Relationship = "SAME_PLAYER",
  StatusBitFlagsExclude = "NO_REFUND",
})
```



# 22.--设置矿脉含量
```lua
#!ra3luabridge
ControlOreAmount(Filter_OreNode, 50000)

#!ra3luabridge
function ControlOreAmount(Filter, number)
  local OreAmount = 50000-number
  local unitTables, count = ObjectFindObjects(nil, nil, Filter)
  ExecuteAction("DEBUG_STRING", "找到了" .. count.. "个符合过滤器条件的OreNode单位")
  for i = 1, count do
    ObjectSetGatheredOreAmount(unitTables[i], OreAmount)
  end
end
Filter_OreNode = CreateObjectFilter({
  Rule = "ANY",
  IncludeThing = {"OreNode"}
})
```



# 23.--带内置CD的攻击附带
```lua
#!ra3luabridge
FireWithWeapon("P1", Filter_AlliedAntiAirVehicleTech1, "CelestialAntiStructureVehicleWeapon", "SovietInterceptorAircraftFlakMissile",45)

#!ra3luabridge

-- 全局变量，用于存储每个单位上次触发的时间(这个不要动)
lastTriggerTimes = {}
ClearTime = 0

-- 全局变量，用于查找对应处于开火状态的单位(把AlliedBaseDefense改成需要的单位)
Filter_AlliedBaseDefense = CreateObjectFilter({
  Rule = "ANY",
  IncludeThing = {"AlliedBaseDefense"},
  Relationship = "SAME_PLAYER",
})

-- 定义函数，用于检查是否已经过了指定的冷却时间
function CanTriggerAttack(unit, currentTime, cooldownTime)
  local unitId = ObjectGetId(unit)
  if lastTriggerTimes[unitId] == nil then
    lastTriggerTimes[unitId] = currentTime - cooldownTime
  end
  if currentTime - lastTriggerTimes[unitId] >= cooldownTime then
    lastTriggerTimes[unitId] = currentTime
    return true
  else
    return false
  end
end

-- 定义函数，用于单位开火时加装临时武器开火
function UnitAttackEvent(unit, weaponName_D, weaponName_K, currentTime, cooldownTime)
  if EvaluateCondition("UNIT_HAS_OBJECT_STATUS", unit, "IS_FIRING_WEAPON") then
    if CanTriggerAttack(unit, currentTime, cooldownTime) then
      local target = ObjectGetTarget(unit)
      local x, y, z = ObjectGetPosition(target)
      if not EvaluateCondition("UNIT_HAS_OBJECT_STATUS", target, "AIRBORNE_TARGET") then
        ObjectCreateAndFireTempWeaponToTarget(unit, weaponName_D, {
          X = x, Y = y, Z = z
        }, target)
      else
        ObjectCreateAndFireTempWeaponToTarget(unit, weaponName_K, {
          X = x, Y = y, Z = z
        }, target)
      end   
    end
  end
end


-- 定义函数，用于设置所有单位的攻击事件
function FireWithWeapon(scriptName, Filter, weaponName_D, weaponName_K, cooldownTime)
  local currentTime = GetFrame()
  if currentTime - ClearTime >= 4500 then
    lastTriggerTimes = {}
    ClearTime = currentTime
  end
  local playerUnit = GetObjectByScriptName(scriptName)
  local units, count = ObjectFindObjects(playerUnit, {
    X = 4000, Y = 4000, Z = 200,
    Radius = 20000,
    DistType = "EXTENTS_3D"
  }, Filter)
  if count < 1 then return end
  for i = 1, count do
    local unit = units[i]
    UnitAttackEvent(unit, weaponName_D, weaponName_K, currentTime, cooldownTime)
  end
end
```



# 24.--封装函数-把一种单位替换成另一种单位（数量不要太多）
```lua
#!ra3luabridge
CreateAdvancedDefensesAndRemoveEggs("p2", Filter_japanegg_collapse, "JapanBaseDefenseAdvanced", "Player_1/teamPlayer_1")

#!ra3luabridge
function CreateAdvancedDefensesAndRemoveEggs(scriptName, Filter, newUnitType, teamName)
  local playerUnit = GetObjectByScriptName(scriptName)
  local unitTables, count = ObjectFindObjects(playerUnit, {
    X = 4000, Y = 4000, Z = 200,
    Radius = 20000,
    DistType = "EXTENTS_3D"
  }, Filter)
  if count < 1 then return end
  for i = 1, count do
    local x1, y1, z1 = ObjectGetPosition(unitTables[i])
    ExecuteAction("CREATE_OBJECT", newUnitType, teamName, {X = x1, Y = y1, Z = z1}, -135)
    ExecuteAction("NAMED_DELETE", unitTables[i])
  end
  ExecuteAction("TEAM_SET_STANCE", teamName, "AGGRESSIVE")
end
Filter_japanegg_collapse = CreateObjectFilter({
  Rule = "ANY",
  IncludeThing = {"japanegg_collapse"},
  Relationship = "SAME_PLAYER",
})
```



# 25.--围墙替换碉堡(不可出售与回收)
```lua
#!ra3luabridge
WallChangeBunker(PlayerBZ, Filter_WallPiece, "SovietBunker")

#!ra3luabridge
Filter_WallPiece = CreateObjectFilter({
    Rule = "ANY",
    IncludeThing = { "AlliedWallPiece", "SovietWallPiece", "CelestialWallPiece", "JapanWallPiece" },
})

--围墙变碉堡函数(围墙不小于3时才会启动)
function WallChangeBunker(UnitName, Filter, bunkerType)
    local playerUnit = GetObjectByScriptName(UnitName)
    if playerUnit then
        local units, count_1 = ObjectFindObjects(playerUnit, {
            X = 4000,
            Y = 4000,
            Z = 200,
            Radius = 6000,
            DistType = "EXTENTS_3D"
        }, Filter)
        if count_1 < 3 then return end
        local player = ObjectPlayerScriptName(playerUnit)
        local playerTeam = player .. "/team" .. player
        for i = 1, count_1 do
            local objectId = ObjectGetId(units[i])
            local Name = "unit_" .. objectId
            local x, y, z = ObjectGetPosition(units[i])
            ExecuteAction("UNIT_SPAWN_NAMED_LOCATION_ORIENTATION", Name, bunkerType, playerTeam, { X = x, Y = y, Z = z }, 0)
            ExecuteAction("UNIT_CHANGE_OBJECT_STATUS", Name, "CANNOT_BE_SOLD", "TRUE")
            ExecuteAction("NAMED_SET_MAX_HEALTH", Name, 2000, "TRUE")
            ExecuteAction("NAMED_DELETE", units[i])
        end
    end
end

#!ra3luabridge
-- 全局变量
PlayerBZ = {"weiqaing1","weiqaing2","weiqaing3","weiqaing4","weiqaing5","weiqaing6"}
DefenTowers = {"SovietBaseDefenseGround",
"SovietBaseDefenseAir",
"SovietBaseDefenseAdvanced",
"CelestialBaseDefenseAir",
"CelestialBaseDefenseAdvanced",
"JapanBaseDefense",
"JapanBaseDefenseAdvanced",
"AlliedBaseDefense",
"AlliedBaseDefenseAdvanced",
}
Filter_WallPiece = CreateObjectFilter({
  Rule = "ANY",
  IncludeThing = {"AlliedWallPiece", "SovietWallPiece", "CelestialWallPiece", "JapanWallPiece"},
  Relationship = "SAME_PLAYER",
})
```



# 26.--玩家围墙变随机防御塔
```lua
#!ra3luabridge
ConvertWallsToRandomDefens(PlayerBZ, Filter_WallPiece, DefenTowers)

-- 调用围墙变随机防御塔函数
function ConvertWallsToRandomDefens(plyrUnitNames, filterCriteria, unitNameList)
  local num = getn(plyrUnitNames)
  ExecuteAction("DEBUG_STRING", "列表中一共有 " .. num .. " 个单位名称")
  for j = 1, num do
    local unit = plyrUnitNames[j]
    ExecuteAction("DEBUG_STRING", "当前正在执行 " .. unit .. " 单位所属玩家的单位")
    RandomizeWallsDefens(unit, filterCriteria, unitNameList)
  end
end

--围墙变随机防御塔函数(围墙不小于3时才会启动)
function RandomizeWallsDefens(unitName, searchFilter, unitList)
  local playerUnit = GetObjectByScriptName(unitName)
  local listSize = getn(unitList)
  if playerUnit then
    local foundUnits, count = ObjectFindObjects(playerUnit, {
      X = 4000, Y = 4000, Z = 200,
      Radius = 20000,
      DistType = "EXTENTS_3D"
    }, searchFilter)
    ExecuteAction("DEBUG_STRING", "找到了 " .. count .. " 个属于玩家的围墙")
    if count > 2 then       
      local teamName = ObjectTeamName(playerUnit)
      local playerName = ObjectPlayerScriptName(playerUnit)
      local playerTeam = playerName .. "/" .. teamName
      for i = 1, count do
        local random = ceil(GetRandomNumber()*listSize)
        local chosenUnitName = unitList[random]
        local x, y, z = ObjectGetPosition(foundUnits[i])
        ExecuteAction("CREATE_OBJECT", chosenUnitName, playerTeam, {X = x, Y = y, Z = z}, 0)
        ExecuteAction("NAMED_DELETE", foundUnits[i])
        ExecuteAction("PLAYER_GIVE_MONEY", playerName, -600)
      end
    end
  end
end
```



# 27.--开局生产基地车并塞进世纪里面
```lua
#!ra3luabridge
AssignBaseWagonByProtocol(ProtocolList, BaseVehicleProtocolList)

#!ra3luabridge
ProtocolList={
  [1]="PlayerTech_Allied",
  [2]="PlayerTech_Soviet",
  [3]="PlayerTech_Celestial",
  [4]="PlayerTech_Japan"
}

BaseVehicleProtocolList={
  [1]="AlliedMCV",
  [2]="SovietMCV",
  [3]="CelestialMCV",
  [4]="JapanMCV"
}

function AssignBaseWagonByProtocol(List_1, List_2)
  for i = 1,6 do
    local PlayerName = "Player_"..i
    ExecuteAction("CREATE_NAMED_ON_TEAM_AT_WAYPOINT", PlayerName..i.."SJ", "AlliedBomberAircraft", PlayerName .. "/team"..PlayerName, PlayerName .. "_Start")
    local ShiJiUnit = GetObjectByScriptName(PlayerName..i.."SJ")
    ObjectLoadAttributeModifier(ShiJiUnit, "AttributeMod_GenericSpeedSet400", 1000)
    ObjectLoadAttributeModifier(ShiJiUnit, "AttributeMod_ChronoRiftEffect_Contained", 1000)
         
    for j = 1,4 do
      if EvaluateCondition("PLAYER_HAS_PLAYER_TECH", PlayerName, List_1[j]) then
        ExecuteAction("CREATE_NAMED_ON_TEAM_AT_WAYPOINT", PlayerName..i..j.."JDC", List_2[j], PlayerName .. "/team"..PlayerName, PlayerName .. "_Start")
        ExecuteAction("NAMED_GARRISON_SPECIFIC_BUILDING_INSTANTLY", PlayerName..i..j.."JDC", PlayerName..i.."SJ")
      end
    end
  end
end
```



# 28.--封装函数-分配队伍
```lua
#!ra3luabridge
TeamAssignment("P2",Filter_BombCraft,"PlyrCreeps/AF", TeamID_2, 9, 1)

#!ra3luabridge
TeamID_1 = 1
TeamID_2 = 1
function TeamAssignment(unit,Filter,team, TeamNum, maxID, QH)
  local PlayerUnit = GetObjectByScriptName(unit)
  local x, y, z = ObjectGetPosition(PlayerUnit)
  local unitTables, count = ObjectFindObjects(PlayerUnit, {
    X = x, Y = y, Z = z,
    Radius = 20000,
    DistType = "EXTENTS_3D"
  }, Filter)
  if count < 1 then return end
  for i = 1, count do
    if TeamNum > maxID then TeamNum = 1 end
    local PlyaerTeam = team..TeamNum
    ObjectSetObjectStatus(unitTables[i], "NO_REFUND")
    ExecuteAction("UNIT_SET_TEAM", unitTables[i],PlyaerTeam)
    if QH == 1 then
        ObjectLoadAttributeModifier(unitTables[i], "AttributeMod_GenericBOSSAIUnitCheat", 90000)
        ObjectLoadAttributeModifier(unitTables[i], "AttributeMod_ChronoRiftEffect", 90000)
    end
    TeamNum = TeamNum + 1
  end
end
Filter_AirCraft = CreateObjectFilter({
  Rule = "ANY",
  IncludeThing = {"CelestialInterceptorAircraft","SovietInterceptorAircraft","AlliedInterceptorAircraft"},
  Relationship = "ALLIES",
  StatusBitFlagsExclude = "NO_REFUND",
})
Filter_BombCraft = CreateObjectFilter({
  Rule = "ANY",
  IncludeThing = {"SovietTransportAircraft","AlliedBomberAircraft","CelestialBomberAircraft","AlliedAC130GunshipAircraft","SovietBomberAircraft"},
  Relationship = "ALLIES",
  StatusBitFlagsExclude = "NO_REFUND",
})
```



# 29.--开局给玩家双精兵一车库并分配标志单位
```lua
#!ra3luabridge
local angle = 3.1415926535

-- 创建并设置单位
ExecuteAction("CREATE_NAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION", "jingbing1", "VeterancyTechStructure", "PlyrCivilian/teamPlyrCivilian", "BonusUnitSpawn", angle)
ExecuteAction("CREATE_NAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION", "jingbing2", "VeterancyTechStructure", "PlyrCivilian/teamPlyrCivilian", "BonusUnitSpawn", angle)
ExecuteAction("CREATE_NAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION", "cheku", "GarageTechStructure", "PlyrCivilian/teamPlyrCivilian", "BonusUnitSpawn", angle)

ExecuteAction("NAMED_SET_MAX_HEALTH", "jingbing1", 50000, "true")
ExecuteAction("NAMED_SET_MAX_HEALTH", "jingbing2", 50000, "true")
ExecuteAction("NAMED_SET_MAX_HEALTH", "cheku", 50000, "true")

-- 为每个玩家创建单位并设置属性
for i = 1, 6 do
    local playerName = "Player_" .. i
    ExecuteAction("CREATE_NAMED_ON_TEAM_AT_WAYPOINT", playerName, "SovietBunker", playerName .. "/team" .. playerName, "BonusUnitSpawn")
    local unit = GetObjectByScriptName(playerName)
    ExecuteAction("NAMED_SET_MAX_HEALTH", playerName, 50000, "true")
    ExecuteAction("UNIT_SET_MODELCONDITION_GENERIC", unit, "HIDDEN", 0)
    ObjectSetObjectStatus(unit, "UNATTACKABLE")
    ObjectSetObjectStatus(unit, "UNSELECTABLE")
    ObjectSetObjectStatus(unit, "UNDER_IRON_CURTAIN")
    ObjectSetObjectStatus(unit, "CANNOT_BE_SOLD")

    local description1 = ObjectDescription(unit)
    if strfind(description1, "(nEFDul6WIAOAAAAAAAAY)",nil,nil) then
        ExecuteAction("PLAYER_SET_MONEY", playerName, "130000")
    else
        ExecuteAction("PLAYER_SET_MONEY", playerName, "80000")
    end
end

-- 将单位所有权转移给玩家
for j = 1, 6 do
    local Playerunit = GetObjectByScriptName("Player_" .. j)
    if Playerunit then
        ExecuteAction("NAMED_TRANSFER_OWNERSHIP_PLAYER", "jingbing1", "Player_" .. j)
        ExecuteAction("NAMED_TRANSFER_OWNERSHIP_PLAYER", "jingbing2", "Player_" .. j)
        ExecuteAction("NAMED_TRANSFER_OWNERSHIP_PLAYER", "cheku", "Player_" .. j)
        return
    end
end
```



# 30.--移除地图上所有玩家信标(包括不在单位上的)
```lua
#!ra3luabridge
DeleteBeacon(Filter_Beacon)

#!ra3luabridge
Filter_Beacon = CreateObjectFilter({
    Rule = "ANY",
    IncludeThing = { "MultiplayerBeacon", "MultiplayerBeaconAttached" },
})

function DeleteBeacon(Filter)
    local Beacons, numbers = ObjectFindObjects(nil, nil, Filter_Beacon)
    if numbers < 1 then return end
    for i = 1, numbers do
        ExecuteAction("NAMED_DELETE", Beacons[i])
    end
end
```



# 31.--记录初始位置，并将初始位置相对位置持续保留的函数（拼模用）
```lua
#!ra3luabridge
WeiZhiList = {
    [1] = {
        [1] = {},
        [2] = {},
        [3] = {},
        [4] = {},
        [5] = {},
        [6] = {},
        [7] = {},
    },
    [2] = {
        [1] = {},
        [2] = {},
        [3] = {},
        [4] = {},
        [5] = {},
        [6] = {},
        [7] = {},
    },
    [3] = {
        [1] = {},
        [2] = {},
        [3] = {},
        [4] = {},
        [5] = {},
        [6] = {},
        [7] = {},
    },
    [4] = {
        [1] = {},
        [2] = {},
        [3] = {},
        [4] = {},
        [5] = {},
        [6] = {},
        [7] = {},
    },
    [5] = {
        [1] = {},
        [2] = {},
        [3] = {},
        [4] = {},
        [5] = {},
        [6] = {},
        [7] = {},
    },
    [6] = {
        [1] = {},
        [2] = {},
        [3] = {},
        [4] = {},
        [5] = {},
        [6] = {},
        [7] = {},
    },
    [7] = {
        [1] = {},
        [2] = {},
        [3] = {},
        [4] = {},
        [5] = {},
        [6] = {},
        [7] = {},
    },
}

AngleList = {}

OB1 = GetObjectByScriptName("P10")
OB2 = GetObjectByScriptName("P20")
OB3 = GetObjectByScriptName("P30")
OB4 = GetObjectByScriptName("P40")
OB5 = GetObjectByScriptName("P50")
OB6 = GetObjectByScriptName("P60")


local x0, y0, z0 = ObjectGetPosition(OB1)
local matrixOB1 = ObjectGetTransform(OB1)
local direction2D = { X = matrixOB1[1], Y = matrixOB1[5] }
AngleList[1] = atan2(direction2D.Y, direction2D.X)
ObjectSetObjectStatus(OB1, "IN_NANOHIVE")
ObjectSetObjectStatus(OB1, "NO_COLLISIONS")
ObjectSetObjectStatus(OB1, "UNCONTROLLABLY_SCARED")

for a = 1, 6 do
    for i = 1, 7 do
        local OBunit = GetObjectByScriptName("P" .. a .. i)
        if OBunit then
            ObjectSetObjectStatus(OBunit, "IMMOBILE")
            ObjectSetObjectStatus(OBunit, "UNATTACKABLE")
            ObjectSetObjectStatus(OBunit, "NO_COLLISIONS")
            ObjectSetObjectStatus(OBunit, "UNSELECTABLE")
            ExecuteAction("UNIT_SET_MODELCONDITION_GENERIC", OBunit, "HIDDEN", 0)

            local xi, yi, zi = ObjectGetPosition(OBunit)
            WeiZhiList[a][1][i] = xi - x0
            WeiZhiList[a][2][i] = yi - y0
            WeiZhiList[a][3][i] = zi - z0

            local matrixOBunit = ObjectGetTransform(OBunit)
            WeiZhiList[a][4][i] = matrixOBunit[1]
            WeiZhiList[a][5][i] = matrixOBunit[2]
            WeiZhiList[a][6][i] = matrixOBunit[5]
            WeiZhiList[a][7][i] = matrixOBunit[6]
        end
    end
end

function GaoDaPinZhuang(OBunit, PName, ID)
    local x0, y0, z0 = ObjectGetPosition(OBunit)
    local matrix = ObjectGetTransform(OBunit)
    local direction2D = { X = matrix[1], Y = matrix[5] }
    local UnitAngle = atan2(direction2D.Y, direction2D.X)
    local angle = UnitAngle - AngleList[ID]
    ExecuteAction("DEBUG_STRING", "x0: " .. x0 .. " y0: " .. y0 .. " z0: " .. z0)
    ExecuteAction("DEBUG_STRING", "angle: " .. angle)

    for i = 1, 7 do
        local OBpart = GetObjectByScriptName(PName .. i)
        if ObjectIsAlive(OBpart) then
            local xi = x0 + WeiZhiList[ID][1][i] * cos(angle) - WeiZhiList[ID][2][i] * sin(angle)
            local yi = y0 + WeiZhiList[ID][1][i] * sin(angle) + WeiZhiList[ID][2][i] * cos(angle)
            local zi = z0 + WeiZhiList[ID][3][i]
            ObjectSetPosition(OBpart, xi, yi, zi)

            local matrixOBpart = ObjectGetTransform(OBpart)
            matrixOBpart[1] = WeiZhiList[ID][4][i] * cos(angle) + WeiZhiList[ID][5][i] * sin(angle)
            matrixOBpart[2] = WeiZhiList[ID][5][i] * cos(angle) - WeiZhiList[ID][4][i] * sin(angle)
            matrixOBpart[5] = WeiZhiList[ID][6][i] * cos(angle) + WeiZhiList[ID][7][i] * sin(angle)
            matrixOBpart[6] = WeiZhiList[ID][7][i] * cos(angle) - WeiZhiList[ID][6][i] * sin(angle)
            ObjectSetTransform(OBpart, matrixOBpart)
        end
    end
end
```



# 32.--查找玩家的所有的某种类型的建筑，如果某单位数量不超过最大值，则在建筑前方80码处生成一个与建筑同队伍的单位
```lua
#!ra3luabridge
Table_P1 = GetObjectByScriptName("P1")
Table_P2 = GetObjectByScriptName("P2")
SearchRegion = {
    X = 4000, Y = 4000, Z = 200,
    Radius = 6000,
    DistType = "EXTENTS_3D"
}
function FindAndActOnUnits(UnitTable, Filter1, Filter2, maxCount, classification)
    local unitTables, count = ObjectFindObjects(UnitTable, SearchRegion, Filter2)
    if count < maxCount then
        MakingSameTeamUnits(UnitTable, Filter1, classification)
    end
end

function MakingSameTeamUnits(UnitTable, Filter, DivisionType)
    local player = ObjectPlayerScriptName(UnitTable)
    local Divisions, count = ObjectFindObjects(UnitTable, SearchRegion, Filter)
    if count < 1 then return end
    for i = 1, count do
        local x, y, z = ObjectGetPosition(Divisions[i])
        local matrix = ObjectGetTransform(Divisions[i])
        local angle = atan2(matrix[5], matrix[1])
        local Team = ObjectTeamName(Divisions[i])
        local playerTeam = player .. "/" .. Team
        ExecuteAction("CREATE_OBJECT", DivisionType, playerTeam,
            { X = x + 80 * cos(angle), Y = y + 80 * sin(angle), Z = z }, rad(angle))
    end
end
```



# 33.--切换阵营然后给基地车的封装函数
```lua
ProtocolList = {
    ["Soviet"] = "PlayerTechStore_Soviet",
    ["Allied"] = "PlayerTechStore_Allied",
    ["Celestial"] = "PlayerTechStore_Neutral",
    ["Japan"] = "PlayerTechStore_Japan"
}

YardList = {
    ["Soviet"] = "SovietConstructionYard",
    ["Allied"] = "AlliedConstructionYard",
    ["Celestial"] = "CelestialConstructionYard",
    ["Japan"] = "JapanConstructionYard"
}

function TransformkProtocolAndYard(playerName, dialogId, money, Faction, CD)
    local playerMoney = exPlayerGetCurrentMoney(playerName)
    if playerMoney < money then
        exAddTextToPublicBoardForPlayer(playerName, "您现在没有足够的金钱", 2)
        return
    end
    local previous = SetWorldBuilderThisPlayer(1);
    ExecuteAction("PLAYER_GIVE_MONEY", playerName, -money)
    exPlayerTechChangeTechFaction(playerName, ProtocolList[Faction])
    ExecuteAction("CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT", "AlliedSuperWeaponEffect", playerName .. "/team" .. playerName,
        playerName .. "_Start")
    ExecuteAction("CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT", YardList[Faction], playerName .. "/team" .. playerName,
        playerName .. "_Start")
    exCenterTopBtnSetTimerOverlayVisibilityForPlayer(playerName, dialogId, 1)
    exCenterTopBtnSetTimerOverlayValueForPlayer(playerName, dialogId, 0)
    exCenterTopBtnSetEnableForPlayer(playerName, dialogId, 0)
    SkillCDList[playerName][dialogId] = CD
    SkillRemainingCDList[playerName][dialogId] = SkillCDList[playerName][dialogId]
    SetWorldBuilderThisPlayer(previous)
end
```



# 34.--设置单位位置和朝向
```lua
#!ra3luabridge
function SetUnitPositionAndFacing(unit, x, y, z, angle)
    local matrix = ObjectGetTransform(unit)
    local COS = cos(angle)
    local SIN = sin(angle)
    matrix[1] = COS
    matrix[2] = -SIN
    matrix[4] = x
    matrix[5] = SIN
    matrix[6] = COS
    matrix[8] = y
    matrix[12] = z
    ObjectSetTransform(unit, matrix)
end
```



# 35.--利用过滤器实现P1所在玩家的超时空可以传送建筑
```lua
#!ra3luabridge
SelectUnitsAndTeleportTargets(Table_P1)

#!ra3luabridge
--把P1定为全局变量
Table_P1 = GetObjectByScriptName("P1")

--建筑的过滤器
Filter_STRUCTURE = CreateObjectFilter({
    Rule = "ANY",
    Include = "STRUCTURE FS_BASE_DEFENSE",
})

--超时空特效的过滤器
Filter_Allied1 = CreateObjectFilter({
    Rule = "ANY",
    IncludeThing = { "AlliedSuperWeaponPersistentEffect" },
    Relationship = "SAME_PLAYER",
})

Filter_Allied2 = CreateObjectFilter({
    Rule = "ANY",
    IncludeThing = { "AlliedSuperWeaponEffect" },
    Relationship = "SAME_PLAYER",
})

--选择并传送单位
function SelectUnitsAndTeleportTargets(Unittable)
    local Weapon1, count1 = ObjectFindObjects(Unittable, nil, Filter_Allied1)
    ExecuteAction("DEBUG_STRING", "找到了" .. count1 .. "个传送阵1" or "未找到传送阵1")
    local x1, y1, z1 = ObjectGetPosition(Weapon1[count1])
    local Weapon2, count2 = ObjectFindObjects(Unittable, nil, Filter_Allied2)
    ExecuteAction("DEBUG_STRING", "找到了" .. count2 .. "个传送阵2" or "未找到传送阵2")
    local STRUCTUREs, num = ObjectFindObjects(Weapon1[count1], {
        X = x1,
        Y = y1,
        Z = z1,
        Radius = 150,
        DistType = "EXTENTS_3D"
    }, Filter_STRUCTURE)
    ExecuteAction("DEBUG_STRING", "找到了" .. num .. "个传送阵1附近的单位" or "未找到单位")
    local x2, y2, z2 = ObjectGetPosition(Weapon2[count1])
    local x0 = x2 - x1
    local y0 = y2 - y1
    local z0 = z2 - z1
    for i = 1, num do
        local x, y, z = ObjectGetPosition(STRUCTUREs[i])
        ObjectSetPosition(STRUCTUREs[i], x + x0, y + y0, z + z0)
    end
end
```



# 36.--利用创造函数实现玩家的超时空可以传送建筑
```lua
#!ra3luabridge
PlayerTable = {}

for i = 1, 6 do
    local playerName = "Player_" .. i
    local objectId = exCreateObject({
        ObjectType = FastHash("SovietBunker"),
        TeamName = playerName.. "/team" ..playerName,
        Position = { X = -200, Y = -200, Z = 0 },
        Health = 50000,
    })
    if objectId ~= 0 then
        local UnitTable = GetObjectById(objectId)
        PlayerTable[playerName] = UnitTable
        ExecuteAction("UNIT_SET_MODELCONDITION_GENERIC", UnitTable, "HIDDEN", 0)
        ObjectSetObjectStatus(objectId, "UNATTACKABLE")
        ObjectSetObjectStatus(objectId, "UNSELECTABLE")
        ObjectSetObjectStatus(objectId, "UNDER_IRON_CURTAIN")
        ObjectSetObjectStatus(objectId, "CANNOT_BE_SOLD")
    end
end
--建筑的过滤器
Filter_STRUCTURE = CreateObjectFilter({
    Rule = "ANY",
    Include = "STRUCTURE FS_BASE_DEFENSE",
})

--超时空特效的过滤器
Filter_Allied1 = CreateObjectFilter({
    Rule = "ANY",
    IncludeThing = { "AlliedSuperWeaponPersistentEffect" },
    Relationship = "SAME_PLAYER",
})

-----------------------------------------------------------------------------------------------------------------------------------------------
--选择并传送单位（带玩家）
function SelectAndTeleport(ID, Player)
    local Weapon1, count1 = ObjectFindObjects(PlayerTable[Player], nil, Filter_Allied1)
    ExecuteAction("DEBUG_STRING", "找到了" .. count1 .. "个传送阵1" or "未找到传送阵1")
    local x1, y1, z1 = ObjectGetPosition(Weapon1[count1])
    local x2, y2, z2 = ObjectGetPosition(ID)
    local x0 = x2 - x1
    local y0 = y2 - y1
    local z0 = z2 - z1
    local STRUCTUREs, num = ObjectFindObjects(Weapon1[count1], {
        X = x1,
        Y = y1,
        Z = z1,
        Radius = 150,
        DistType = "EXTENTS_3D"
    }, Filter_STRUCTURE)
    ExecuteAction("DEBUG_STRING", "找到了" .. num .. "个传送阵1附近的单位" or "未找到单位")
    for i = 1, num do
        local x, y, z = ObjectGetPosition(STRUCTUREs[i])
        ObjectSetPosition(STRUCTUREs[i], x + x0, y + y0, z + z0)
    end
end
-----------------------------------------------------------------------------------------------------------------------------------------------
exObjectRegisterCreateEvent("AlliedSuperWeaponEffect")
-----------------------------------------------------------------------------------------------------------------------------------------------
--超时空传送传送建筑
CSKCSJZ = function(createdObjId, ownerPlayerName)
    SelectAndTeleport(createdObjId, ownerPlayerName)
end
-----------------------------------------------------------------------------------------------------------------------------------------------
--设置不同单位的创造事件↓
WJCreateEventList = {
    [FastHash("AlliedSuperWeaponEffect")] = CSKCSJZ
}

AICreateEventList = {}

CreateEventList = {
    ["Player_1"] = WJCreateEventList,
    ["Player_2"] = WJCreateEventList,
    ["Player_3"] = WJCreateEventList,
    ["Player_4"] = WJCreateEventList,
    ["Player_5"] = WJCreateEventList,
    ["Player_6"] = WJCreateEventList,
    ["PlyrCivilian"] = AICreateEventList,
    ["PlyrCreeps"] = AICreateEventList,
}
-----------------------------------------------------------------------------------------------------------------------------------------------
--创造事件函数↓
function onUnitCreateEvent(createdObjId, createdObjInstanceId, ownerPlayerName)
    if CreateEventList[ownerPlayerName][createdObjInstanceId] then
        CreateEventList[ownerPlayerName][createdObjInstanceId](createdObjId, ownerPlayerName)
    end
end
```



# 37.--设定基础脚本(玩家间关系，AI协议和升级以及禁止生产)
```lua
#!ra3luabridge
--设定玩家之间的关系
ExecuteAction("PLAYER_RELATES_PLAYER", "PlyrCivilian", "PlyrCreeps","Friend")
ExecuteAction("PLAYER_RELATES_PLAYER", "PlyrCivilian", "PlyrCreeps","Friend")
for PlayerNumber = 1,6 do
    local PlayerName = "Player_"..PlayerNumber
    ExecuteAction("PLAYER_RELATES_PLAYER", PlayerName, "PlyrCreeps","Enemy")
    ExecuteAction("PLAYER_RELATES_PLAYER", PlayerName, "PlyrCivilian","Enemy")
    ExecuteAction("PLAYER_RELATES_PLAYER", "PlyrCreeps", PlayerName,"Enemy")
    ExecuteAction("PLAYER_RELATES_PLAYER", "PlyrCivilian", PlayerName,"Enemy")
end

--给对应的协议
ExecuteAction("PLAYER_GRANT_SPECIAL_POWER", "SpecialPowerJapanRoboticAssembly","<All Players>")
ExecuteAction("PLAYER_GRANT_SPECIAL_POWER", "SpecialPowerJapanNavalPower","PlyrCreeps")
ExecuteAction("PLAYER_GRANT_SPECIAL_POWER", "SpecialPowerJapanNavalPower","PlyrCivilian")
ExecuteAction("PLAYER_GRANT_SPECIAL_POWER", "SpecialPowerJapanAdvancedMissilePacks","PlyrCreeps")
ExecuteAction("PLAYER_GRANT_SPECIAL_POWER", "SpecialPowerJapanAdvancedMissilePacks","PlyrCivilian")
ExecuteAction("PLAYER_GRANT_SPECIAL_POWER", "SpecialPowerJapanEnhancedKamikaze","PlyrCreeps")
ExecuteAction("PLAYER_GRANT_SPECIAL_POWER", "SpecialPowerJapanEnhancedKamikaze","PlyrCivilian")
ExecuteAction("PLAYER_GRANT_SPECIAL_POWER", "SpecialPower_Celestial_ElectromagneticGun","PlyrCreeps")
ExecuteAction("PLAYER_GRANT_SPECIAL_POWER", "SpecialPower_Celestial_ElectromagneticGun","PlyrCivilian")
ExecuteAction("PLAYER_GRANT_SPECIAL_POWER", "SpecialPowerAirPower","PlyrCreeps")
ExecuteAction("PLAYER_GRANT_SPECIAL_POWER", "SpecialPowerAirPower","PlyrCivilian")
ExecuteAction("PLAYER_GRANT_SPECIAL_POWER", "SpecialPowerHighTechnology","PlyrCreeps")
ExecuteAction("PLAYER_GRANT_SPECIAL_POWER", "SpecialPowerHighTechnology","PlyrCivilian")

--给高科升级(必须得有对应高科)
function UpgradeTechBuilding(PlayerName)
    ExecuteAction("GIVE_PLAYER_UPGRADE", PlayerName, "upgrade_sovietcompositearmor")
    ExecuteAction("GIVE_PLAYER_UPGRADE", PlayerName, "upgrade_sovietimprovedteslacoil")
    ExecuteAction("GIVE_PLAYER_UPGRADE", PlayerName, "upgrade_sovietmissileintercept")
    ExecuteAction("GIVE_PLAYER_UPGRADE", PlayerName, "upgrade_sovietmultimodeguidance")
    ExecuteAction("GIVE_PLAYER_UPGRADE", PlayerName, "upgrade_japanintelligentammunition")
    ExecuteAction("GIVE_PLAYER_UPGRADE", PlayerName, "upgrade_japanmecharush")
    ExecuteAction("GIVE_PLAYER_UPGRADE", PlayerName, "upgrade_japannanosustainingforcefield")
    ExecuteAction("GIVE_PLAYER_UPGRADE", PlayerName, "upgrade_japannanotransmissionstructure")
    ExecuteAction("GIVE_PLAYER_UPGRADE", PlayerName, "upgrade_japansubmarineattack")
end

UpgradeTechBuilding("PlyrCivilian")
UpgradeTechBuilding("PlyrCreeps")


--禁止生产列表中的单位
DISALLOWBUILDINGList = { "AlliedInfiltrationInfantry", "AlliedSuperWeapon", "AlliedFighterAircraft", "SovietSuperWeapon",
    "CelestialBattery", "CelestialSuperWeapon", "CelestialInfiltrationInfantry", "CelestialHeavyAntiAirVehicleTech3",
    "JapanSuperWeaponEgg", "JapanAntiVehicleInfantryTech3", "JapanLightTransportVehicle", "JapanInfiltrationInfantry", }

for i = 1, getn(DISALLOWBUILDINGList) do
    ExecuteAction("ALLOW_DISALLOW_ONE_BUILDING", "<All Players>", DISALLOWBUILDINGList[i], "false")
end
```



# 38.--死亡函数-子弹爆炸模板
```lua
#!ra3luabridge
--设置不同单位的死亡函数↓

AIDieEventList = {}

WJDieEventList = {}

DieEventList = {
    ["Player_1"]     = WJDieEventList,
    ["Player_2"]     = WJDieEventList,
    ["Player_3"]     = WJDieEventList,
    ["Player_4"]     = WJDieEventList,
    ["Player_5"]     = WJDieEventList,
    ["Player_6"]     = WJDieEventList,
    ["PlyrCivilian"] = AIDieEventList,
    ["PlyrCreeps"]   = AIDieEventList,
}
--死亡事件函数↓
function onUnitDieEvent(dyingObjId, attackerId, dyingObjInstanceId, attackerInstanceId, ownerPlayerName)
    if DieEventList[ownerPlayerName][dyingObjInstanceId] then
        DieEventList[ownerPlayerName][dyingObjInstanceId](dyingObjId, ownerPlayerName)
    end
end

--- @param projectile string 死亡单位
--- @param weapon string 单位死亡时触发的武器ID
--- @param DieEventList table 输入AIDieEventList或者WJDieEventList来确定是玩家的单位触发还是AI的单位触发
function ConfigurePostDetonationStrike(projectile, weapon, DieEventList)
  exObjectRegisterDieEvent(projectile)
  DieEventList[FastHash(projectile)] = function(dyingObjId, ownerPlayerName)
      ObjectCreateAndFireTempWeapon(dyingObjId, %weapon)
  end
end

--未来坦克子弹爆炸时带EMP
ConfigurePostDetonationStrike("AlliedFutureTankNeutronWeaponProjectile_Vet",
  "AlliedAntiStructureShipEMPCruiseMissleWarhead", AIDieEventList)
```



# 39.--顶部技能按钮模板之战争银行
```lua
#!ra3luabridge
-- 设置玩家的顶部按钮
exCenterTopBtnShow(1, "AUA_Bribe",
    "「战争银行」\n——————————————————\n开启战争银行,可以存入或取出金钱\n——————————————————\n")

-- 启用顶部按钮
exCenterTopBtnSetEnable(1, 1)

-- 设置银行的初始存款数量
AvailableBalance = 100000

--顶部按钮点击事件
TopBtnClick = {
    [1] = function(playerName, btnIndex)
        exShowCustomBtnChoiceDialogForPlayer(
            playerName,
            1,
            "战争银行当前存款：" .. AvailableBalance,
            "取出5000",
            "存入5000",
            "取出50000",
            "存入50000",
            "回到战场!",
            "",
            ""
        )
    end,
}

-- 处理顶部按钮点击事件的函数
function onCenterTopBtnClickEvent(playerName, btnIndex, shiftPressed)
    if TopBtnClick[shiftPressed][btnIndex] then
        TopBtnClick[shiftPressed][btnIndex](playerName, btnIndex)
    end
end

--列表按钮点击事件
DialogBoxList = {
    --战争银行存钱和取钱
    [1] = {
        [1] = function(playerName)
            PlayerQuQian(playerName, 5000)
        end,
        [2] = function(playerName)
            PlayerCunQian(playerName, 5000)
        end,
        [3] = function(playerName)
            PlayerQuQian(playerName, 50000)
        end,
        [4] = function(playerName)
            PlayerCunQian(playerName, 50000)
        end,
        [5] = function(playerName)
            exAddTextToPublicBoardForPlayer(playerName, "战争银行感谢您的使用，希望能对您有所帮助", 2)
        end,
    }
}

-- 处理列表按钮点击事件的函数
function onUserBtnChoiceDialogEvent(playerName, btnIndex, dialogId)
    if DialogBoxList[dialogId][btnIndex] then
        DialogBoxList[dialogId][btnIndex](playerName, dialogId)
    end
end

--战争银行_存钱
function PlayerCunQian(PlayerName, moneynumber)
    local playerMoney = exPlayerGetCurrentMoney(PlayerName)
    if playerMoney < moneynumber then
        exAddTextToPublicBoardForPlayer(PlayerName, "您当前没有足够的金钱存入银行", 2)
        return
    end
    local previous = SetWorldBuilderThisPlayer(1)
    AvailableBalance = AvailableBalance + moneynumber
    ExecuteAction("PLAYER_GIVE_MONEY", PlayerName, -moneynumber)
    exShowCustomBtnChoiceDialogForPlayer(
        PlayerName,
        1,
        "战争银行当前存款：" .. AvailableBalance,
        "取出5000",
        "存入5000",
        "取出50000",
        "存入50000",
        "回到战场!",
        "",
        ""
    )
    SetWorldBuilderThisPlayer(previous)
end

--战争银行_取钱
function PlayerQuQian(PlayerName, moneynumber)
    if AvailableBalance < moneynumber then
        exAddTextToPublicBoardForPlayer(PlayerName, "银行没有足够的余额供您取出", 2)
        return
    end
    local previous = SetWorldBuilderThisPlayer(1)
    AvailableBalance = AvailableBalance - moneynumber
    ExecuteAction("PLAYER_GIVE_MONEY", PlayerName, moneynumber)
    exShowCustomBtnChoiceDialogForPlayer(
        PlayerName,
        1,
        "战争银行当前存款：" .. AvailableBalance,
        "取出5000",
        "存入5000",
        "取出50000",
        "存入50000",
        "回到战场!",
        "",
        ""
    )
    SetWorldBuilderThisPlayer(previous)
end
```



# 40.--通过射线法判断某个点在不在某个范围内
```lua
#!ra3luabridge
-- 定义多边形区域的五个顶点（必须按顺时针或逆时针顺序排列）
Polygon = {
  { x = 100,  y = 1500 },
  { x = 1000, y = 2000 },
  { x = 2000, y = 340 },
  { x = 2500, y = 200 },
  { x = 3000, y = 100 }
}

-- 判断点 (x, y) 是否在多边形内
local function PointInPolygon(X, Y, Poly)
  local inside = false
  local n = getn(Poly)
  local j = n
  for i = 1, n do
    local xi, yi = Poly[i].x, Poly[i].y
    local xj, yj = Poly[j].x, Poly[j].y
    if ((yi > Y) ~= (yj > Y)) and
      (X < (xj - xi) * (Y - yi) / (yj - yi) + xi) then
      inside = not inside
    end
    j = i
  end
  return inside
end
```
