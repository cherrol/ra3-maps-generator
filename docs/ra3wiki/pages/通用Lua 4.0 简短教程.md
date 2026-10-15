---
title: "通用Lua 4.0 简短教程"
catalog_path: "Lua教程"
slug: sddcnchpvag2slf1
source: https://www.yuque.com/muzeqaq/ra3mapwiki/sddcnchpvag2slf1
fetched: 2026-10-15
content_updated_at: "2024-12-16T21:27:04.000Z"
type: Doc
format: lake
---

# 通用Lua 4.0 简短教程

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/sddcnchpvag2slf1
> 目录位置：Lua教程

# 
:::info
**Author**: Inertia  
**Last Updated**: 2024-Oct-11

:::

## 简介
Lua 是一种轻量级、快速、灵活的脚本语言，非常适合嵌入式开发、游戏开发和数据处理等场景。  
学习 Lua 的好处在于它的语法简洁易学，执行速度快，并且可以轻松嵌入到 C/C++ 项目中，极大地提高开发效率。  
现代许多应用程序和游戏引擎（如 Roblox、World of Warcraft、和 Adobe Lightroom）都采用 Lua 作为脚本引擎，方便开发人员扩展功能和实现高度可定制的行为。  
Lua 在物联网（IoT）设备、游戏 AI、配置文件处理、数据解析、服务器脚本等众多领域也得到了广泛应用。  
如果你希望快速上手编程、开发高效的脚本系统或在现有软件中加入自定义功能，Lua 将是一个非常理想的选择。

以下是 Lua 4.0 的基础教程，帮助你快速掌握这门语言的基本使用方法。

> 参考资料：[https://www.lua.org/manual/4.0/manual.html](https://www.lua.org/manual/4.0/manual.html)
>

## 教程
### 1. 变量与数据类型
Lua 是动态类型语言，意味着变量不需要声明数据类型。你可以直接赋值，Lua 会根据值的类型自动判断。

```lua
a = 10          -- 整数 --注，lua中的整数也是由浮点数存储的，10实际上是10.0
b = 3.14        -- 浮点数
c = "hello"     -- 字符串
d = true        -- 布尔类型
```

### 2. 注释
Lua 中单行注释使用 `--`。  
注意多行注释在 Lua 4.0 中不存在，如果需要多行注释，可以使用多个单行注释。Lua5.0参考：[https://www.lua.org/manual/5.0/manual.html#2.1](https://www.lua.org/manual/5.0/manual.html#2.1)

```lua
-- 这是单行注释

--[[
  这是多行注释，无法通过 Lua 4.0 的解释器，在 Lua 5.0 中才被支持。 
]]


--- 这是
--- 正确的
--- 4.0"多行注释"
--- 在VS CODE中可以通过CTRL+/ 快捷键生成

```

### 3. 条件语句
Lua 的条件语句与其他语言类似，使用 `if`、`else` 和 `elseif`。

```lua
a = 5

if a > 10 then
    print("a 大于 10")
elseif a == 5 then
    print("a 等于 5")
else
    print("a 小于 10 且不等于 5")
end
```

### 4. 循环
Lua 提供了 `while` 和 `for` 两种常用循环。

```lua
-- while 循环
i = 1
while i <= 5 do
    print(i)
    i = i + 1
end

-- for 循环
for i = 1, 5 do
    print(i)
end
```

### 5. 函数
函数是 Lua 的核心部分，可以定义和调用函数。

```lua
-- 定义一个简单的函数
function greet(name)
    return "Hello, " .. name
end

-- 调用函数
print(greet("Lua"))
```

### 6. 表 (Table)
表是 Lua 的唯一数据结构，既可以用作数组，也可以用作字典。

```lua
-- 数组
arr = {1, 2, 3, 4}
print(arr[1])  -- 输出 1

-- 字典
dict = {name = "Lua", age = 30}
print(dict["name"])  -- 输出 Lua
```

### 7. 全局变量和局部变量
默认情况下，Lua 中的变量都是全局的。如果希望定义局部变量，使用 `local` 关键字。  
尽可能减少全局变量的使用，以避免命名冲突和不必要的内存消耗。

```lua
x = 10          -- 全局变量
local y = 20    -- 局部变量
```

### 8. 表（Table）中的函数变量
Lua 4.0 没有复杂的模块系统，但你可以通过表来组织函数。  
表不仅可以存储数据，还可以存储函数。函数在 Lua 中是变量，可以像其他数据一样存储、传递或赋值。  
Lua 中的函数是一等公民，也就是说，函数可以像其他变量一样在表中定义或独立定义并赋值给表的字段。  
通过这种方式，你可以将函数存储在表里，类似于模块化编程，也可以将函数作为变量来灵活使用。  
以下是一些简单的例子：

```lua
-- 直接定义一个带有函数的表
mymodule = {
    myfunc = function()
        print("Hello from myfunc!")
    end
}

-- 在表中定义另一个函数
mymodule.sayHello = function()
    print("Hello from sayHello!")
end

-- 在外部定义函数
function sayGoodbye()
    print("Goodbye from global!")
end

-- 将外部函数添加到表中
mymodule.sayGoodbye = sayGoodbye

-- 调用表中的函数
mymodule.myfunc()       -- 输出：Hello from myfunc!
mymodule.sayHello()     -- 输出：Hello from sayHello!
mymodule.sayGoodbye()   -- 输出：Goodbye from global!
```

### 9. 注意 `nil` 在 Lua 中的隐性问题
Lua 中，未定义的变量默认值为 `nil`，且 `nil` 在条件判断中被视为 `false`。这意味着拼写错误的变量名不会引发错误，但可能导致意想不到的逻辑结果。

#### 示例：
```lua
local playerData = {
  playerHealthCurrent = 100,
  playerIsIdle = true,
}

-- 拼写错误，将 playerHealthCurrent 写成 playerHealthcurrent
if playerData.playerHealthcurrent then
    print("Player is alive with health: " .. playerData.playerHealthCurrent)
else
    print("Player is dead or health value is invalid.")  -- 执行到这里
end
```

在上述代码中，`playerHealthcurrent`是拼写错误的变量名，它的值默认为 `nil`，导致条件判断错误地认为玩家已经“死亡”。为了避免类似问题，建议：

1. 使用清晰且统一的变量命名规则。
2. 借助工具（如 `EmmyLUA`和`Lua Language Server`）检测未定义变量。
3. 在手动检查变量时正确使用文本查找功能，严格匹配大小写。
4. 抛出运行时警告和错误

### 10. 错误处理与 `_ALERT`
文档参考：[https://www.lua.org/manual/4.0/manual.html#4.7](https://www.lua.org/manual/4.0/manual.html#4.7)

在 Lua 4.0 中，错误处理机制依赖于 `error()` 和 `call()`，而全局变量 `_ALERT` 是用来处理未捕获的错误的默认方法。

+ `_ALERT`：当程序中发生未捕获的错误时，Lua 会自动调用 `_ALERT`，通常会将错误信息输出到控制台。你可以将 `_ALERT` 重新定义为自己的错误处理函数，方便捕捉和记录错误信息。
+ `error()`：当你需要手动生成错误时，可以调用 `error()`。它会中断程序的执行，并触发 `_ALERT`，输出相应的错误信息。
+ `call()`：与现代的 `pcall()` 类似，`call()` 用来安全地调用某个函数。如果函数内发生错误，程序不会崩溃，而是会返回错误信息。这个方式在 Lua 4.0 中提供了基本的错误捕捉机制。

#### 自定义 `_ALERT`
你可以重定义 `_ALERT` 来控制错误的处理方式。默认情况下，它只会将错误信息打印出来，但你可以将其修改为更复杂的行为，比如记录到文件或显示在特定的地方。

```lua
-- 自定义 _ALERT
_ALERT = function(message)
    print("捕获到错误: " .. message)
    -- 在这里你可以做更多的处理，比如将错误写入日志
end

-- 触发一个错误，_ALERT 将处理它
error("这是一个测试错误")
```

#### 解释：
1. `_ALERT`：默认处理未捕捉的错误，通常用于输出错误信息。可以根据需要自定义它。
2. `error()`：用于手动生成错误，方便在程序中捕获异常情况。
3. `call()`：用于安全地执行某个函数，避免程序因为错误而崩溃。

通过自定义 `_ALERT`，你可以控制 Lua 程序中的错误处理行为，适应不同的调试或日志记录需求。

## 结语
以上就是 Lua 4.0 的简短教程。通过学习这些基础知识，你已经可以开始编写简单的 Lua 程序。  
深入学习可以参考 Lua 4.0 的官方文档：[https://www.lua.org/manual/4.0/](https://www.lua.org/manual/4.0/)  


## 上手试试
最后，下面是本教程的Lua版本，整个文件是一个完整的没有错误的 Lua 4.0 脚本（同时也符合Lua 5.0标准)，你可以直接复制粘贴到下面的在线解释器中运行，看看效果或开始自己的探索：  
[https://www.tutorialspoint.com/execute_lua_online.php](https://www.tutorialspoint.com/execute_lua_online.php)

```lua
-- #### Lua 4.0 简短教程
-- Author: Inertia
-- Last Updated: 2024-Oct-11
-- 推荐使用VS Code阅读此教程

-- #### 简介
--- Lua 是一种轻量级、快速、灵活的脚本语言，非常适合嵌入式开发、游戏开发和数据处理等场景。
--- 学习 Lua 的好处在于它的语法简洁易学，执行速度快，并且可以轻松嵌入到 C/C++ 项目中，极大地提高开发效率。
--- 现代许多应用程序和游戏引擎（如 Roblox、World of Warcraft、和 Adobe Lightroom）都采用 Lua 作为脚本引擎，
--- 方便开发人员扩展功能和实现高度可定制的行为。
--- Lua 在物联网（IoT）设备、游戏 AI、配置文件处理、数据解析、服务器脚本等众多领域也得到了广泛应用。
--- 如果你希望快速上手编程、开发高效的脚本系统或在现有软件中加入自定义功能，Lua 将是一个非常理想的选择。

-- 以下是 Lua 4.0 的基础教程，帮助你快速掌握这门语言的基本使用方法。



-- 教程
-- #### 1. **变量与数据类型**
-- Lua 是动态类型语言，意味着变量不需要声明数据类型。你可以直接赋值，Lua 会根据值的类型自动判断。

-- ```lua
a = 10          -- 整数
b = 3.14        -- 浮点数
c = "hello"     -- 字符串
d = true        -- 布尔类型
-- ```

-- #### 2. **注释**
-- Lua 中单行注释使用 `--`，
-- 注意多行注释在LUA4.0中不存在，如果需要多行注释，可以使用多个单行注释。https://www.lua.org/manual/5.0/manual.html#2.1

-- ```lua

-- 这是单行注释

--[[
  这是多行注释， 无法通过LUA4.0的解释器，在LUA5.0中才被支持。 
]]

-- ```

-- #### 3. **条件语句**
-- Lua 的条件语句与其他语言类似，使用 `if`、`else` 和 `elseif`。

-- ```lua
a = 5

if a > 10 then
    print("a 大于 10")
elseif a == 5 then
    print("a 等于 5")
else
    print("a 小于 10 且不等于 5")
end
-- ```

-- #### 4. **循环**
-- Lua 提供了 `while` 和 `for` 两种常用循环。

-- ```lua
-- while 循环
i = 1
while i <= 5 do
    print(i)
    i = i + 1
end

-- for 循环
for i = 1, 5 do
    print(i)
end
-- ```

-- #### 5. **函数**
-- 函数是 Lua 的核心部分，可以定义和调用函数。

-- ```lua
-- 定义一个简单的函数
function greet(name)
    return "Hello, " .. name
end

-- 调用函数
print(greet("Lua"))
-- ```

-- #### 6. **表 (Table)**
-- 表是 Lua 的唯一数据结构，既可以用作数组，也可以用作字典。

-- ```lua
-- 数组
arr = {1, 2, 3, 4}
print(arr[1])  -- 输出 1

-- 字典
dict = {name = "Lua", age = 30}
print(dict["name"])  -- 输出 Lua
-- ```

-- #### 7. **全局变量和局部变量**
-- 默认情况下，Lua 中的变量都是全局的。如果希望定义局部变量，使用 `local` 关键字。
-- 尽可能减少全局变量的使用，以避免命名冲突和不必要的内存消耗。

-- ```lua
x = 10          -- 全局变量
local y = 20    -- 局部变量
-- ```

-- #### 8. **表（Table）中的函数变量**
-- Lua 4.0 没有复杂的模块系统，但你可以通过表来组织函数。
-- 表不仅可以存储数据，还可以存储函数。函数在 Lua 中是变量，可以像其他数据一样存储、传递或赋值。
-- Lua中的函数是一等公民，也就是说，函数可以像其他变量一样在表中定义或独立定义并赋值给表的字段。
-- 通过这种方式，你可以将函数存储在表里，类似于模块化编程，也可以将函数作为变量来灵活使用。
-- 以下是一些简单的例子：

-- 直接定义一个带有函数的表
mymodule = {
    myfunc = function()
        print("Hello from myfunc!")
    end
}

-- 在表中定义另一个函数
mymodule.sayHello = function()
    print("Hello from sayHello!")
end

-- 在外部定义函数
function sayGoodbye()
    print("Goodbye from global!")
end

-- 将外部函数添加到表中
mymodule.sayGoodbye = sayGoodbye

-- 调用表中的函数
mymodule.myfunc()       -- 输出：Hello from myfunc!
mymodule.sayHello()     -- 输出：Hello from sayHello!
mymodule.sayGoodbye()   -- 输出：Goodbye from global!

-- #### 9. 注意 `nil` 在 Lua 中的隐性问题
-- Lua 中，未定义的变量默认值为 `nil`，且 `nil` 在条件判断中被视为 `false`。这意味着拼写错误的变量名不会引发错误，但可能导致意想不到的逻辑结果。

-- #### 示例：
-- ```lua
local playerData = {
  playerHealthCurrent = 100,
  playerIsIdle = true,
}

-- 拼写错误，将 playerHealthCurrent 写成 playerHealthcurrent
if playerData.playerHealthcurrent then
    print("Player is alive with health: " .. playerData.playerHealthCurrent)
else
    print("Player is dead or health value is invalid.")  -- 执行到这里
end
-- ```

-- 在上述代码中，`playerHealthcurrent`是拼写错误的变量名，它的值默认为 `nil`，导致条件判断错误地认为玩家已经“死亡”。为了避免类似问题，建议：

-- 1. 使用清晰且统一的变量命名规则。
-- 2. 借助工具（如 `EmmyLUA`和`Lua Language Server`）检测未定义变量。
-- 3. 在手动检查变量时正确使用文本查找功能，严格匹配大小写。

-- #### 10. **错误处理与 `_ALERT`**

-- 文档参考： https://www.lua.org/manual/4.0/manual.html#4.7

-- 在 Lua 4.0 中，错误处理机制依赖于 `error()` 和 `call()`，而全局变量 **`_ALERT`** 是用来处理未捕获的错误的默认方法。

-- **`_ALERT`**：当程序中发生未捕获的错误时，Lua 会自动调用 `_ALERT`，通常会将错误信息输出到控制台。
-- 你可以将 `_ALERT` 重新定义为自己的错误处理函数，方便捕捉和记录错误信息。

-- **`error()`**：当你需要手动生成错误时，可以调用 `error()`。它会中断程序的执行，并触发 `_ALERT`，输出相应的错误信息。

-- **`call()`**：与现代的 `pcall()` 类似，`call()` 用来安全地调用某个函数。
-- 如果函数内发生错误，程序不会崩溃，而是会返回错误信息。这个方式在 Lua 4.0 中提供了基本的错误捕捉机制。

-- #### 自定义 `_ALERT`

-- 你可以重定义 `_ALERT` 来控制错误的处理方式。默认情况下，它只会将错误信息打印出来，
-- 但你可以将其修改为更复杂的行为，比如记录到文件或显示在特定的地方。

-- ```lua
-- 自定义 _ALERT
_ALERT = function(message)
    print("捕获到错误: " .. message)
    -- 在这里你可以做更多的处理，比如将错误写入日志
end

-- 触发一个错误，_ALERT 将处理它
error("这是一个测试错误")
-- ```

-- #### 解释：
-- 1. **`_ALERT`**：默认处理未捕捉的错误，通常用于输出错误信息。可以根据需要自定义它。
-- 2. **`error()`**：用于手动生成错误，方便在程序中捕获异常情况。
-- 3. **`call()`**：用于安全地执行某个函数，避免程序因为错误而崩溃。

-- 通过自定义 `_ALERT`，你可以控制 Lua 程序中的错误处理行为，适应不同的调试或日志记录需求。



-- #### 结语
-- 以上就是 Lua 4.0 的简短教程。通过学习这些基础知识，你已经可以开始编写简单的 Lua 程序。
-- 深入学习可以参考LUA4.0的官方文档：https://www.lua.org/manual/4.0/
-- 最后，这整个文件是一个完整的没有错误的LUA4.0脚本，你可以直接复制粘贴到下面的在线解释器中运行，看看效果或开始自己的探索。
-- https://www.tutorialspoint.com/execute_lua_online.php
```

点击红圈处运行

![](https://cdn.nlark.com/yuque/0/2024/png/50038568/1730699437208-91141652-ae64-4a76-adb8-34207242f575.png)

下面是预期的输出结果  
![](https://cdn.nlark.com/yuque/0/2024/png/50038568/1733957207789-bb43daef-d368-4555-a5a2-73b52cfc5b5b.png)
