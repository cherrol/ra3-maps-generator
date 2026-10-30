---
title: "Lua4.0进阶 - Table中的函数"
catalog_path: "Lua教程"
slug: hv11w9hk1g5sxq9o
source: https://www.yuque.com/muzeqaq/ra3mapwiki/hv11w9hk1g5sxq9o
fetched: 2026-10-15
content_updated_at: "2025-01-07T09:21:00.000Z"
type: Doc
format: lake
---

# Lua4.0进阶 - Table中的函数

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/hv11w9hk1g5sxq9o
> 目录位置：Lua教程

:::success
作者: dreamness

:::

函数除了可以定义为全局函数外, table中也可以存储函数

```lua
TestTable = { v1=1, v2=2}

TestTable.func1 = function(a, b)
  TestTable.v1 = a
  TestTable.v2 = b
end

-- 调用方式
TestTable.func1(3, 4)

```



除此之外, 还有"冒号"来定义和调用函数

```lua
TestTable = { v1=1, v2=2}

function TestTable:func2(a, b)
  self.v1 = a
  self.v2 = b
end

-- 调用方式
TestTable:func2(5, 6)

```



注意func2的函数体中, 使用了"self", 它代表了该函数所在的table, 也就是例子中的TestTable
