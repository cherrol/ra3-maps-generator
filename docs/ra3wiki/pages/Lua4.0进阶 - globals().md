---
title: "Lua4.0进阶 - globals()"
catalog_path: "Lua教程"
slug: grg6b5mq5z7ogst5
source: https://www.yuque.com/muzeqaq/ra3mapwiki/grg6b5mq5z7ogst5
fetched: 2026-10-15
content_updated_at: "2024-12-19T11:45:50.000Z"
type: Doc
format: lake
---

# Lua4.0进阶 - globals()

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/grg6b5mq5z7ogst5
> 目录位置：Lua教程

:::success
作者: dreamness

:::

globals()可以获取全局变量所在的table, 通过修改它, 可以修改全局变量的值

```lua
-- 以下两个语句等价
var_a = 1
globals().var_a = 1

-- 以下两个语句等价
function func_b(param1, param2) end
globals().func_b = function(param1, param2) end

-- 也可以用来取消函数的定义
globals().func_b = nil


```



该函数可以用来动态地定义全局函数

```lua
function register_func(func)
  globals().amazing_func = func
end
```
