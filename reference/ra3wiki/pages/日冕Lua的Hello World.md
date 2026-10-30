---
title: "日冕Lua的Hello World"
catalog_path: "Lua教程"
slug: lx066ch0tq9h13ve
source: https://www.yuque.com/muzeqaq/ra3mapwiki/lx066ch0tq9h13ve
fetched: 2026-10-15
content_updated_at: "2024-12-19T10:21:09.000Z"
type: Doc
format: lake
---

# 日冕Lua的Hello World

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/lx066ch0tq9h13ve
> 目录位置：Lua教程

:::info
作者：Inertia

日期：2024年12月19日

:::

> 地图编辑器中有一个脚本项：**第 0 号脚本 Scripting/Debug/Display message and pause**。通过这个脚本，用户可以输入任意文本。日冕 Mod 修改了这个脚本的功能——如果该脚本的文本以 `#!ra3luabridge` 开头，则该文本将被作为 Lua 脚本执行。因此，地图作者们可以利用这个脚本来运行 Lua 代码。
>
> 使用日冕V3.22x版本客户端启动地图编辑器时，在该脚本的参数编辑窗口中还会出现一个“Load Lua File”按钮，点击它可以加载一个 Lua 文件。
>



打开地图编辑器，新建空白地图，使用路径点工具（图中黑色箭头指出）增加2个路径点并点击三角形下拉菜单，选择名字为`Player_1_Start`和`Player_2_Start`（这会是玩家1和玩家2的出生点）, 

![](https://cdn.nlark.com/yuque/0/2024/png/50038568/1734603236313-0adec3e0-ef2f-48da-addb-c0ab62f9efe3.png)

![](https://cdn.nlark.com/yuque/0/2024/png/50038568/1734602735296-7a82d8e9-e884-4f0b-8e37-fa0c1bc5e5b0.png)

路径点放下来未改名字前的样子（Waypoint 后面带数字编号）：

![](https://cdn.nlark.com/yuque/0/2024/png/50038568/1734603338398-9702ba69-8888-4753-ad4f-3762a8e571ab.png)



然后，在界面左上角进入`编辑`-`脚本`，

![](https://cdn.nlark.com/yuque/0/2024/png/50038568/1734602762279-ecd05bb6-6390-47e7-b387-9521669fb4ba.png)

点击`新脚本`

![](https://cdn.nlark.com/yuque/0/2024/png/50038568/1734602882149-31e27a81-36d1-4332-91a7-328510d3a45b.png)

新建一条默认脚本。

![](https://cdn.nlark.com/yuque/0/2024/png/50038568/1734602850854-ed02770e-7f72-4d1f-b57e-7de67abcda8c.png)

点击`Action If True`，然后双击蓝色高亮行

![](https://cdn.nlark.com/yuque/0/2024/png/50038568/1734602946979-e21caaf7-fdbc-43e1-9545-8ee1ecc7187e.png)

在打开的页面单击 `Display message and pause 显示信息并暂停.[0]`

![](https://cdn.nlark.com/yuque/0/2024/png/50038568/1734603024545-82a8deeb-51ae-453b-8dc8-923fcae92c6e.png)



然后单击界面右边的 蓝色字体 `<font style="color:#1DC0C9;">String '???'</font>`，在打开的窗口直接输入或通过Lua文件输入

> #!ra3luabridge
>
> exMessageAppendToMessageArea("Hello World!")
>

![](https://cdn.nlark.com/yuque/0/2024/png/50038568/1734602521418-da877cfc-8825-409e-8164-71731bbcb598.png)

保存后，进入地图。你会在左下角发现一行文字：Hello World!

![](https://cdn.nlark.com/yuque/0/2024/png/50038568/1734601994944-155ad888-09f8-49f5-b993-e5a0db533135.png)

恭喜，你完成了你的第一条日冕Lua脚本!
