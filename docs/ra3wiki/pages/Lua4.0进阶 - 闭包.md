---
title: "Lua4.0进阶 - 闭包"
catalog_path: "Lua教程"
slug: ngl1yx5bxfgc81q5
source: https://www.yuque.com/muzeqaq/ra3mapwiki/ngl1yx5bxfgc81q5
fetched: 2026-10-15
content_updated_at: "2025-02-04T05:24:02.000Z"
type: Doc
format: lake
---

# Lua4.0进阶 - 闭包

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/ngl1yx5bxfgc81q5
> 目录位置：Lua教程

引用于文心一言

> **<font style="color:rgb(51, 51, 51);">闭包</font>**<font style="color:rgb(51, 51, 51);">‌（Closure）是一个编程概念，指的是一个函数及其相关的词法环境（即定义该函数时的环境变量）的组合。闭包允许一个函数访问并操作函数外部的变量。</font>
>
> ### <font style="color:rgb(51, 51, 51);">闭包的定义和作用</font>
> <font style="color:rgb(51, 51, 51);">闭包是一个能够访问其外部作用域中变量的函数。在JavaScript中，闭包通常通过函数嵌套实现，内部函数可以访问外部函数的局部变量，即使外部函数已经执行完毕，这些局部变量仍然会被保留在内存中，供内部函数使用。闭包的作用主要包括：</font>
>
> + <font style="color:rgb(51, 51, 51);">‌</font>**<font style="color:rgb(51, 51, 51);">数据封装</font>**<font style="color:rgb(51, 51, 51);">‌：通过闭包，可以将函数和其作用域封装起来，形成一个独立的单元。</font>
> + <font style="color:rgb(51, 51, 51);">‌</font>**<font style="color:rgb(51, 51, 51);">隐私保护</font>**<font style="color:rgb(51, 51, 51);">‌：通过闭包，可以控制外部对内部变量的访问，实现数据的封装和隐私保护。</font>
> + <font style="color:rgb(51, 51, 51);">‌</font>**<font style="color:rgb(51, 51, 51);">状态管理</font>**<font style="color:rgb(51, 51, 51);">‌：闭包可以用于创建具有独立状态的函数，这在模块化编程中非常有用。</font>
>



简单来说, 通常情况下, 函数中是无法访问函数外的变量的如:

```lua
local a = 1

function test_func()
  print(a) -- 会出错, 因为这里无法访问外部变量a
end
```



使用闭包, 可以访问函数外的变量, 但是需要注意, 如果这个变量是普通类型, 如number和string, 使用闭包无法修改值; 但是如果这个变量是引用类型, 如table, 是可以修改这个值

```lua
local a = 1
local b = 'b'
local c = { n = 'n1', m = 'm2'}

function test_func()
  print(tostring(%a))
  print(%b)
  print(%c['n'])

  %a = 2 -- 不会改变
  %b = 'b2' -- 不会改变
  %c['n'] = 'n3' -- 会改变
  
end
```



但是, EmmyLua插件并不兼容lua4的闭包语法, 会报错误, 忽略即可

![](https://cdn.nlark.com/yuque/0/2025/png/34558013/1738646613843-2651bc7a-4456-45d2-b1cc-6438a0a710b4.png)
