---
title: "LUA超级傻瓜教程"
catalog_path: "Lua教程"
slug: hnirufqwl0z7uzaw
source: https://www.yuque.com/muzeqaq/ra3mapwiki/hnirufqwl0z7uzaw
fetched: 2026-10-15
content_updated_at: "2024-12-06T13:44:56.000Z"
type: Doc
format: lake
---

# LUA超级傻瓜教程

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/hnirufqwl0z7uzaw
> 目录位置：Lua教程

:::info
作者：请叫我CY

日期：2024年11月22日

:::

:::info
--此版本只讲使用，不讲原理，如果想更深入地学习LUA，请去看进阶教程

:::

### 1.如何把LUA放进地编
   日冕地编有一个0号脚本，你把写好的LUA复制粘贴进去就行

### 2.如何引用单位
   引用单位，也就是把地图里面的命名单位，让它能在LUA中被用出来。方法就是给个代号，我举个例子：如果地图上有个单位被命名成"biaozhi"，要想引用这个单位，首先我们得给它想个别名，比如BZ，然后下一步就是引用它↓



```lua
   local BZ = GetObjectByScriptName("biaozhi")
```



   local 的作用，就是定义这个别名，GetObjectByScriptName的作用就是引用它后面括号内的名称，也就是单位的名称，记得用""标起来。这句代码也就是给biaozhi起个名为BZ的别名，方便以后我们用

   如果实在不懂的话，你就复制上面我写的那个代码，然后只改一下BZ和biaozhi就行了，其他的不用动。记住，BZ是你起的别名，biaozhi是你要引用的那个单位的命名。

### 3.如何查找单位
   查找单位，一般来说LUA就是用过滤器查找单位了，但是它查找单位和地编不一样，它是通过过滤器查找单位的，下面是个简单的过滤器↓

```lua
local Filter = CreateObjectFilter({
    Rule="ANY",
    Relationship="SAME_PLAYER",
    IncludeThing={"AlliedSupportAircraft",},
  })
```



   local 的作用上面说了，就是把Filer定义为等号后面这一大堆东西的别名，别名自己起就行，没什么需要注意的，别和上面重复就行。

   这个Rule="ANY",不用管，你就直接这么写就行，基础篇用不到这么高深的东西；这个Relationship="SAME_PLAYER",就是关系，关系是相同玩家，也就是同一个玩家，也不用改，就这么写就行；

   这个IncludeThing={"AlliedSupportAircraft",}就是找单位了，这里的意思是找AlliedSupportAircraft（冰冻直升机）这个单位，要是想找其他单位，就把这个AlliedSupportAircraft换成你想要的单位名称即可，如果想找多个单位，那你就用,隔开然后在中括号内加就行，比如我想找AlliedSupportAircraft和CelestialHJ-10这俩单位，我就把IncludeThing={"AlliedSupportAircraft",}改成IncludeThing={"AlliedSupportAircraft","CelestialHJ-10",}就行了。

   如果实在不懂的话，你就复制我上面写的代码，然后只把AlliedSupportAircraft改成你想找的单位ID就行了。



```lua
   local SearchArea = {
      X = 4000, Y = 4000, Z = 200,
      Radius = 20000,
      DistType = "EXTENTS_3D"
   }
```



   上面这个就是查找范围，xyz的值就是以这个坐标为球心，Radius = 20000就是半径20000，DistType = "EXTENTS_3D"就是查找3D范围内的，也就是这个半径20000的球内都是查找范围。

   这个能懂则懂，懂不了直接复制代码粘贴就行，啥都不用改，20000范围妥妥的覆盖全图了。



```lua
   local units, count = ObjectFindObjects(BZ, SearchArea, Filter)
```



   这个就比较重要了，等号后面这一堆，BZ是命名单位的别名，SearchArea是上面的查找范围，Filter就是查找条件。等号后面这一堆的整体意思就是→查找范围(SearchArea)内，所有与BZ所在玩家相同的，符合过滤器Filter的所有单位。

   这个units呢，就是找到的所有单位的集合，count就是找到的所有单位的数量

   如果实在不懂，你还是直接复制我上面的代码就行，记得BZ改成和步骤2你起的别名一样啊，Filer也要改成和你自己起的别名一样



   综合一下，查找单位的整体LUA如下↓

```lua
   local BZ = GetObjectByScriptName("biaozhi")
   local Filter = CreateObjectFilter({
      Rule="ANY",
      Relationship="SAME_PLAYER",
      IncludeThing={"AlliedSupportAircraft",},
    })
    local SearchArea = {
      X = 4000, Y = 4000, Z = 200,
      Radius = 20000,
      DistType = "EXTENTS_3D"
   }
   local units, count = ObjectFindObjects(BZ, SearchArea, Filter)
```

   到了这一步，我们就把找到的单位全都储存在units里面了

### 4.如何给单位命名
   我们继续上面的来，已经把找到的所有单位都储存在units里面了，然后这里面一共有count个单位，于是，我们可以开始写了



```lua
   for i = 1, count do
     local objectId = ObjectGetId(units[i])
     local str = "Unit" .. objectId
     ExecuteAction("SET_UNIT_REFERENCE", str, units[i])
   end
```



   for i = 1, count do这一句的作用就是，从1开始，直到count结束，也就是为了确保能覆盖units里面的每一个单位

   units[i]的意思就是，这是units里面储存的第i个单位，local objectId = ObjectGetId(units[i])这一句是为了获取到单位的数字ID，每个单位的数字ID都是唯一的，可以避免重复

   local str = "Unit" .. objectId就是起个名字了，比如这个单位的数字ID是114514，那么给起的名字就是Unit114514

   这样我们就完成了给找到的所有单位命名

   如果你实在不会，你就复制粘贴我写的就行了

### 5.如何在LUA中结合地编脚本使用
   在LUA中可以通过EvaluateCondition调用条件脚本，通过ExecuteAction来调用动作脚本，举个例子:我们查找某物体是否有NO_REFUND状态，如果没有的话就给添加NO_REFUND状态



```lua
   if not EvaluateCondition("UNIT_HAS_OBJECT_STATUS", unit, "NO_REFUND") then
     ExecuteAction("ExecuteAction", unit, "NO_REFUND")
   end
```



   首先，要确定动作脚本和条件脚本对应的英文去哪里找

   [https://www.yuque.com/muzeqaq/ra3mapwiki](https://www.yuque.com/muzeqaq/ra3mapwiki)

   你要先知道你要用哪一条脚本，比如判断物体状态的脚本是[59]号条件脚本，给物体状态的脚本是[411]号动作脚本

   你需要进入网址中：地编脚本列表-->地编动作列表/地编条件列表中，找到对应编号的脚本，然后查看他们的【命令字】后面的英文，然后使用EvaluateCondition调用条件脚本，通过ExecuteAction来调用动作脚本，填参数的位置与地编中的位置一致。

   到此，我们就彻底完成了一条可以使用的LUA↓

   

```lua
   #!ra3luabridge
   local BZ = GetObjectByScriptName("biaozhi")
   local Filter = CreateObjectFilter({
      Rule="ANY",
      Relationship="SAME_PLAYER",
      IncludeThing={"AlliedSupportAircraft",},
    })
    local SearchArea = {
      X = 4000, Y = 4000, Z = 200,
      Radius = 20000,
      DistType = "EXTENTS_3D"
   }
   local units, count = ObjectFindObjects(BZ, SearchArea, Filter)
   for i = 1, count do
     local objectId = ObjectGetId(units[i])
     local str = "Unit" .. objectId
     ExecuteAction("SET_UNIT_REFERENCE", str, units[i])
     if not EvaluateCondition("UNIT_HAS_OBJECT_STATUS", units[i], "NO_REFUND") then
       ExecuteAction("ExecuteAction", units[i], "NO_REFUND")
     end
   end
```



   这个脚本的作用就是查找biaozhi单位周围20000范围内所有和它相同玩家的AlliedSupportAircraft单位，给命名，然后如果没有NO_REFUND状态的话就给一个NO_REFUND状态

   有一些动作脚本是不能直接套在单位身上使用的，需要命名后用命名来调用，这就是命名的意义所在

   有哪些动作脚本中需要命名才能使用，可以去地编百科中，Lua教程-->《Lua里的地编脚本动作（需要命名）》中查看



   作为看到结尾的奖励，我给大家准备了一份小礼物，下面的代码是一份LUA模板，可以快速查找与P1相同玩家的所有单位并命名

   你只需要将下面的代码全部复制粘贴到一个txt文本中，然后将后缀改成bat，这样双击之后，在对话框中输入你想查找并命名的单位ID，它就会自动输出对应的LUA，并给你需要找的所有单位命名↓



```bash


@echo off
:loop
echo 输入单位的ID生成快速过滤器:
set /p includeThing="> "

if "%includeThing%"=="" goto end
set playerunit=P1
set rule=ANY
set relationship=SAME_PLAYER
set statusBitFlagsExclude=NO_REFUND

echo 生成的快速过滤器结构如下↓
echo.
echo #!ra3luabridge
echo local PlayerUnit = GetObjectByScriptName("%playerunit%")
echo local x, y, z = ObjectGetPosition(PlayerUnit)
echo.
echo local Filter_%includeThing% = CreateObjectFilter({
echo   Rule = "%rule%",
echo   IncludeThing = {"%includeThing%"},
echo   Relationship = "%relationship%",
echo   StatusBitFlagsExclude = "%statusBitFlagsExclude%",
echo })
echo.
echo local unitTables, count = ObjectFindObjects(PlayerUnit, {
echo     X = x, Y = y, Z = z,
echo     Radius = 20000,
echo     DistType = "EXTENTS_3D"
echo     }, Filter_%includeThing%)
echo ExecuteAction("DEBUG_STRING", "找到了" .. count.. "个符合过滤器条件的%includeThing%单位")
echo for i = 1, count do
echo   ObjectSetObjectStatus(unitTables[i], "%statusBitFlagsExclude%")
echo   local objectId = ObjectGetId(unitTables[i])
echo   local str = "Unit" .. objectId
echo   ExecuteAction("SET_UNIT_REFERENCE", str, unitTables[i])
echo end
echo.


goto loop

:end
echo Exiting...
```
