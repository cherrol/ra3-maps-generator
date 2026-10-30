---
title: "Lua傻瓜教程"
catalog_path: "Lua教程"
slug: rtuqhos8kqmxizf0
source: https://www.yuque.com/muzeqaq/ra3mapwiki/rtuqhos8kqmxizf0
fetched: 2026-10-15
content_updated_at: "2024-11-26T15:29:13.000Z"
type: Doc
format: lake
---

# Lua傻瓜教程

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/rtuqhos8kqmxizf0
> 目录位置：Lua教程

:::info
作者：请叫我CY

格式化：Inertia

日期：2024年11月22日

:::

# LUA使用教程（基础篇）
此版本只讲使用，不讲原理。如果想更深入地学习LUA，请参考进阶教程。

:::info
本教程假设读者对代码没有任何基础，但熟练操作地编。

:::

---

## 1. 如何把LUA放进地编
日冕地编有一个0号脚本，只需将写好的LUA复制粘贴进去即可。

---

## 2. 如何引用单位
引用单位是将地图里的命名单位在LUA中调用，方法如下：

### 示例：
若地图上有一个单位命名为`"biaozhi"`，想在LUA中调用它，就需要给它起一个别名`BZ`，代码如下：

```lua
local BZ = GetObjectByScriptName("biaozhi")
```

### 说明：
+ `local`：定义变量`BZ`的作用域。
+ `GetObjectByScriptName("biaozhi")`：获取命名单位`"biaozhi"`。

只需修改`BZ`和`biaozhi`即可，其他内容保持不变。

> GetObjectByScriptName的作用是引用括号中单位的名称，记得用`""`标起来。这句代码作用是给`biaozhi`起个名为`BZ`的别名，方便以后使用。如果想调用其他名称，只需要修改代码中的`biaozhi`和`BZ`即可。  
>

---

## 3. 如何查找单位
LUA使用过滤器查找单位，与地编的方式不同。

### 过滤器示例：
```lua
local Filter = CreateObjectFilter({
   Rule="ANY",
   Relationship="SAME_PLAYER",
   IncludeThing={"AlliedSupportAircraft",},
})
```

### 查找范围示例：
```lua
local SearchArea = {
   X = 4000, Y = 4000, Z = 200,
   Radius = 20000,
   DistType = "EXTENTS_3D"
}
```

### 查找单位代码示例：
```lua
local units, count = ObjectFindObjects(BZ, SearchArea, Filter)
```

### 说明：
+ `Filter`：过滤器的名称。
+ `SearchArea`：查找范围。
+ `ObjectFindObjects(BZ, SearchArea, Filter)`：查找`SearchArea`范围内，以`BZ`为标志，符合`Filter`条件的所有单位。
+ `units`：符合`ObjectFindObjects(BZ, SearchArea, Filter)`条件的所有单位集合
+ `count`：`units`里面包含的单位数量

> 过滤器`Filter`解析：  
`Rule="ANY"`意思是必须满足`Filter`内所有条件才会被纳入过滤器(其他规则可以在进阶版本中学习)。  
`Relationship="SAME_PLAYER",`意思是和对照单位(`BZ`)的关系是所属玩家相同(其他关系可以在进阶版本中学习)。  
`IncludeThing={"AlliedSupportAircraft",},`的意思是，查找的单位是`AlliedSupportAircraft`。  
使用该过滤器时，只需要修改示例代码中`Filter`和`IncludeThing={"AlliedSupportAircraft",},`中的`AlliedSupportAircraft`即可。
>

> 查找范围`SearchArea`解析：  
x、y、z的值为查找范围中心的坐标。  
`Radius = 20000`代表查找半径为20000。  
`DistType = "EXTENTS_3D"`代表3D范围内查找。  
该查找范围基本可以覆盖全图，在未进一步了解LUA之前直接使用示例代码即可。
>

> 查找单位代码`ObjectFindObjects(BZ, SearchArea, Filter)`解析：  
`BZ`是命名单位的别名(必须用别名，不能直接使用单位名称)，`SearchArea`是查找范围，`Filter`是过滤器。  
`local units, count = ObjectFindObjects(BZ, SearchArea, Filter)`整体意思就是→查找范围(`SearchArea`)内，所有与`BZ`所属玩家相同，符合过滤器`Filter`的所有单位，并储存到`units`中  
此部分可以复制示例代码使用，切记`BZ`、`SearchArea`、`Filter`要与前面的一致。
>

---

## 4. 如何给单位命名
### 示例代码：
```lua
for i = 1, count do
  local objectId = ObjectGetId(units[i])
  local str = "Unit" .. objectId
  ExecuteAction("SET_UNIT_REFERENCE", str, units[i])
end
```

### 说明：
+ `for i = 1, count do`：遍历找到的所有单位。
+ `ObjectGetId(units[i])`：获取单位唯一ID。
+ `ExecuteAction("SET_UNIT_REFERENCE", str, units[i])`：为单位命名。

> `for i = 1, count do`的作用是：从1开始，直到`count`结束，也就是为了确保能覆盖`units`里面的每一个单位。  
`units[i]`的意思就是，这是`units`里面储存的第i个单位。  
`local objectId = ObjectGetId(units[i])`作用是获取到单位的数字ID，每个单位的数字ID都是唯一的，可以避免命名重复。  
`local str = "Unit" .. objectId`作用就是给单位起名字，例如这个单位的数字ID是114514，那么给起的名字就是Unit114514。  
`ExecuteAction("SET_UNIT_REFERENCE", str, units[i])`的作用是将起的名字赋予单位。  
这样就完成了给找到的所有单位命名，此部分可以直接复制示例代码使用，只需保证`units`的前后一致即可。
>

---

## 5. 如何在LUA中结合地编脚本使用
LUA可通过`EvaluateCondition`调用条件脚本，通过`ExecuteAction`调用动作脚本。

### 示例：
查找单位是否有`NO_REFUND`状态，如果没有则添加：

```lua
if not EvaluateCondition("UNIT_HAS_OBJECT_STATUS", unit, "NO_REFUND") then
  ExecuteAction("UNIT_CHANGE_OBJECT_STATUS", unit, "NO_REFUND", "TRUE")
end
```

### 地编脚本参考：
在[地编动作列表](https://www.yuque.com/muzeqaq/ra3mapwiki/orutriyre2fllrih)/[条件列表](https://www.yuque.com/muzeqaq/ra3mapwiki/fnshx90tfo5m4k8k)中查找对应的编号和英文命令。

> 动作脚本和条件脚本对应的英文去哪里找？首先需要知道要用哪一条脚本，比如判断物体状态的脚本是[59]号条件脚本，改变物体状态的脚本是[411]号动作脚本。 你需要进入[地编百科](https://www.yuque.com/muzeqaq/ra3mapwiki)：地编脚本列表-->地编动作列表/地编条件列表中，找到对应编号的脚本，然后查看他们的【命令字】后面的英文，然后使用`EvaluateCondition`调用条件脚本，通过`ExecuteAction`来调用动作脚本，填参数的位置与地编中的位置一致。 到此，我们就彻底完成了一条可以使用的LUA↓  
>

---

## 综合示例脚本：
此脚本查找`biaozhi`单位周围20000范围内所有属于相同玩家的`AlliedSupportAircraft`单位，为它们命名并添加`NO_REFUND`状态。

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
    ExecuteAction("UNIT_CHANGE_OBJECT_STATUS", units[i], "NO_REFUND", "TRUE")
  end
end
```

---

### 注意：
有些动作脚本需要通过命名单位才能使用。详细信息请参考Lua教程中的《[Lua里的地编脚本动作（需要命名）](https://www.yuque.com/muzeqaq/ra3mapwiki/ao2x6koxgn4lewpg)》章节。



### 小礼物：
作为看到结尾的奖励，给大家准备了一份小礼物，下面的代码是一份LUA模板，可以快速查找与名为P1的单位所属玩家相同的的所有单位并命名

你只需要将下面的代码全部复制粘贴到一个txt文本中，然后将后缀txt改成bat，这样双击之后，在对话框中输入你想查找并命名的单位ID，它就会自动输出对应的LUA，并给你需要找的所有单位命名↓



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
