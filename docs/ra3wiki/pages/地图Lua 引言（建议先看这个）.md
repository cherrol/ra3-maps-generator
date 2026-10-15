---
title: "地图Lua 引言（建议先看这个）"
catalog_path: "Lua教程"
slug: qllxru0p99ilqosx
source: https://www.yuque.com/muzeqaq/ra3mapwiki/qllxru0p99ilqosx
fetched: 2026-10-15
content_updated_at: "2026-04-07T17:03:07.000Z"
type: Doc
format: lake
---

# 地图Lua 引言（建议先看这个）

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/qllxru0p99ilqosx
> 目录位置：Lua教程

:::info
<font style="color:rgb(38, 38, 38);">作者：</font>**<font style="color:rgb(38, 38, 38);">物wwu</font>**

<font style="color:rgb(38, 38, 38);">更新者</font>**<font style="color:rgb(38, 38, 38);">：物wwu</font>**

<font style="color:rgb(38, 38, 38);">更新时间</font>**<font style="color:rgb(38, 38, 38);">：2025年12月5日</font>**

:::

## 什么是 Lua 代码，为什么要用它？
原生地图编辑器的脚本功能比较有限，并且不够灵活，因此日冕制作组开发出一套新的接口，允许在地图脚本里嵌入lua代码（一种灵活的编程语言），并且提供了更多新的函数

需要注意的是，这里使用的lua版本是4.0。而非现在很多其他插件和游戏中使用的5.0及以上

## Lua 代码应该写在哪里？
日冕制作组修改了地编脚本里的 **第 0 号脚本 Scripting/Debug/Display message and pause** 的功能，——如果该脚本的文本以 `#!ra3luabridge` 开头，则剩余文本将被作为 Lua代码执行。因此，地图作者们可以利用这个脚本来运行 Lua 代码。

在使用最新版日冕客户端启动地图编辑器后，在该脚本的参数编辑窗口中还会出现一个“Load Lua File”按钮，点击它可以加载一个 Lua 文件。

## Lua4.0原生语法教程
参见 [https://www.lua.org/manual/4.0/manual.html](https://www.lua.org/manual/4.0/manual.html)  ， 不过请注意，这个教程里，有关文件读写和os部分可以不用看，因为地图lua里屏蔽了这些函数

## 地图lua有哪些特殊之处
1. 同一张地图里，所有嵌入的lua代码共享同一个环境，也就是可以互相访问其他地方的全局变量和函数
2. lua4.0原生的一些函数已经被屏蔽掉了，比如文件读写，os系列函数
3. 地图lua可以通过ExecuteAction、EvaluateCondition调用原生脚本动作和条件，原生脚本和条件参见 [地编条件](https://www.yuque.com/muzeqaq/ra3mapwiki/eaht4xli7pd51gzk) [地编动作](https://www.yuque.com/muzeqaq/ra3mapwiki/pzle8sromsst4oh9) [地编动作(特殊)](https://www.yuque.com/muzeqaq/ra3mapwiki/ao2x6koxgn4lewpg)
4. 地图的变量和函数在地图结束时会被清空

## 新的lua函数哪些
**参见 **[**LuaAPI-LuaBridge**](https://www.yuque.com/muzeqaq/ra3mapwiki/rg4gm1ibq054y6o4)** 和 **[**LuaAPI-EnhancerCorona**](https://www.yuque.com/muzeqaq/ra3mapwiki/eksx1whrfix7tlno)  这些函数会随着mod更新不断增加

## 哪些mod支持地图lua
目前来说，日冕，ArmorRush，冲突世界 均支持地图lua

## 如何调试代码
首先确保你已经安装了新地图编辑器（如果安装了日冕客户端，那么日冕客户端已经自带了新地编），并且你的游戏版本是老steam版本，在新地编根目录下的game文件夹里，运行WuRa3GameDebug这个调试器

然后运行游戏，即可在游戏里看到这个界面

![](https://cdn.nlark.com/yuque/0/2025/png/855117/1764815867574-8536ce3c-e810-457c-b3a9-43791baae5ce.png)

在上方的菜单栏里，点击基础功能里面的lua编辑，在这里可以输入lua代码进行代码片段的测试



如果要调试地图脚本里的lua，只能通过打印日志来进行调试

首先确保地图脚本里定义了以下函数

```lua
    -- **********************************Important**********************************
    -- This function override the LUA interal error handling function and redirect the error message to the debug console
    function _ALERT(s)
    -- 错误消息在公屏显示30秒
    exAddTextToPublicBoard("Lua Alert: " .. (s or "debug string"), 30)
    end
    -- *****************************************************************************
```

当脚本发生错误时, 游戏中就会看到相关的报错信息(在公屏上, 很显眼)

## 使用 Lua 操作红警3的单位
Lua 脚本最主要的功能之一就是操作游戏中的单位。红警3的单位在 Lua 脚本中有两种表现形式：**Lua Table** 和 **单位命名**。

+ **a. 单位命名：**
    - 在地图编辑器的**单位属性（Object Properties）**窗口内可以给单位命名。
    - 单位的名字（字符串）可以在 Lua 中直接传递给 `ExecuteAction` 和 `EvaluateCondition` 函数。
    - 可以使用 `GetObjectByScriptName` 函数来根据名字获取单位的 Lua Table。
+ **b. Lua Table：**
    - Lua Table 可以在很多 Lua 函数、所有 `EvaluateCondition` 以及绝大部分 `ExecuteAction` 函数中使用。
    - 可以通过一系列函数来获取单位的 Lua Table，例如 `GetObjectByScriptName`。这些函数都是由日冕 Mod 新增的 Lua 函数，详细信息请参考文档[**LuaAPI-LuaBridge**](https://www.yuque.com/muzeqaq/ra3mapwiki/rg4gm1ibq054y6o4)** 和 **[**LuaAPI-EnhancerCorona**](https://www.yuque.com/muzeqaq/ra3mapwiki/eksx1whrfix7tlno)。

## 地图Lua代码库
可以使用由dreamness提供的lua代码库，这个库包装了很多相关api函数，大幅简化代码，详细参见 [Lua库 (推荐使用)](https://www.yuque.com/muzeqaq/ra3mapwiki/afe5t67gxdl6krgd)

## 有用的工具
建议安装地编伴侣，有效管理地图脚本 [地编伴侣](https://www.yuque.com/muzeqaq/ra3mapwiki/wn3aerk3amyg9bw0)

## 地图lua例子
新地图编辑器下的 日冕地图lua例子 文件夹里有一些简单的例子可以参考

## 建议的学习路线
如果你还没有地图编辑基础，建议先学习基本的地图制作，主要包括：

1. 如何新建地图并且可以在游戏里游玩
2. 如何摆放物体
3. 如何编辑脚本



然后再进行地图lua学习：（假设你已经学习了基本的lua语法，如果还没有请先去学习 [通用Lua 4.0 简短教程](https://www.yuque.com/muzeqaq/ra3mapwiki/sddcnchpvag2slf1) ）

1. 看最基本的例子 [日冕Lua的Hello World](https://www.yuque.com/muzeqaq/ra3mapwiki/lx066ch0tq9h13ve)
2. 学习新地图编辑器下的 日冕地图lua例子 文件夹里的地图
3. 对照api文档，在调试器的lua编辑里尝试各个函数的功能



## 相关资源
可以从这里下载相关资料 [百度网盘](https://pan.baidu.com/s/1u7U-mCRNMxMrqiM6bK9bUw?pwd=wuwu)
