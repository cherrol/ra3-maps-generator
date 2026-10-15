---
title: "Lua地编脚本中的参数定义"
catalog_path: "Lua教程"
slug: rge16id4wsoloub8
source: https://www.yuque.com/muzeqaq/ra3mapwiki/rge16id4wsoloub8
fetched: 2026-10-15
content_updated_at: "2024-11-11T12:34:27.000Z"
type: Doc
format: lake
---

# Lua地编脚本中的参数定义

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/rge16id4wsoloub8
> 目录位置：Lua教程

:::info
作者：Inertia

日期：2024年11月11日

:::

### 参数是什么
在 Lua 地编脚本中，参数用于动态指定条件或行为。例如，以下来自《[Lua里的地编条件脚本](https://www.yuque.com/muzeqaq/ra3mapwiki/eaht4xli7pd51gzk)》代码片段中 `${}` 内的内容即为参数：  

> {
>
>     .index = "56",
>
>     .id = "PLAYER_HAS_COMPARISON_UNIT_KIND_IN_TRIGGER_AREA",
>
>     .name = R"#(Player/Area/Player has (comparison) object types with kindof in an area.[56])#",
>
>     .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",
>
>     .argHint = R"#( ${PlayerName} has ${Comparison} ${Integer} objects with ${UnitOrStructureKind} in the ${TriggerAreaName}.)#",
>
> },
>



上面的 `.argHint` 字段定义了一些标准参数，例如 `${PlayerName}`、`${Comparison}`、`${Integer}` 等。这些参数可以在脚本执行时被动态赋值。

下面是Comparison的参数定义：

```lua
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
```

> 注: 上面这段定义代码可以复制到类型定义文件中，方便VS Code 中的Lua插件进行类型检测。
>

下面是一个使用Comparison 参数的例子，通过地编条件脚本函数PLAYER_HAS_COMPARISON_UNIT_KIND_IN_TRIGGER_AREA，

定义 `Comparison` 类型变量 `com_operator` 并在 `EvaluateCondition` 函数中使用：

```lua
--- @type Comparison
local com_operator = "GT"
local bool = EvaluateCondition("PLAYER_HAS_COMPARISON_UNIT_KIND_IN_TRIGGER_AREA", "Player_1", 
  com_operator, "0", "SELECTABLE", "AREA")
```

> 注：--- @type Comparison 为注释，可略去，功能为方便VS Code 中的Lua插件进行类型检测
>
> PLAYER_HAS_COMPARISON_UNIT_KIND_IN_TRIGGER_AREA 这个函数本身存在bug, 见[地编脚本中的BUG](https://www.yuque.com/muzeqaq/ra3mapwiki/crd4xmcur34fpkgc)
>





推荐使用VS Code进行Lua代码开发并在VS Code上安装[EmmyLua](https://marketplace.visualstudio.com/items?itemName=tangzx.emmylua)或[sumneko](https://marketplace.visualstudio.com/items?itemName=sumneko.lua)的lua插件，并按照[**LuaCATS (Lua Comment And Type System)**](https://luals.github.io/wiki/annotations/)的格式注释参数。

### 快速参数注释入门教程
如果你正在编写 Lua 代码并希望提升可读性和代码维护性，推荐使用 [**LuaCATS (Lua Comment And Type System)**](https://luals.github.io/wiki/annotations/)注解风格。它的注释标准非常直观，可以帮助你快速为代码添加类型提示和注释，让编辑器提供智能提示、类型检查、自动补全等功能，非常适合团队协作和大型项目。

1. **安装 Lua Language Server**  
    - 在 VS Code 等支持 Lua 的编辑器中，安装 **Lua Language Server** 插件（例如通过 VS Code 扩展市场搜索 "Lua" 找到插件）。
2. **注解示例**

```lua
--- @class Person
--- @field name string 姓名
--- @field age number 年龄
local person = { name = "Alice", age = 30 }

--- 加法函数
--- @param a number 第一个数字
--- @param b number 第二个数字
--- @return number 两数之和
local function add(a, b)
    return a + b
end
```

3. **常用注解标签**
    - `---@param`：定义函数参数类型。
    - `---@return`：定义返回值类型。
    - `---@class` 和 `---@field`：定义类和表结构。
    - `---@alias`：定义类型别名。
4. **更多参考资料**
    - 查看 Lua Language Server 的注解文档以了解更多标签：[Lua Language Server 注解文档](https://luals.github.io/wiki/annotations/)

通过这种注解方式，你可以大幅提升代码的清晰度和开发体验。
