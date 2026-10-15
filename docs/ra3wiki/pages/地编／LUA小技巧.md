---
title: "地编/LUA小技巧"
catalog_path: ""
slug: ew6u7s0qm2w68sho
source: https://www.yuque.com/muzeqaq/ra3mapwiki/ew6u7s0qm2w68sho
fetched: 2026-10-15
content_updated_at: "2025-01-26T08:08:21.000Z"
type: Doc
format: lake
---

# 地编/LUA小技巧

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/ew6u7s0qm2w68sho
> 目录位置：(未分组)

1. 生产建筑物可以加上其他阵营的科技升级（Upgrade），即可实现对应建筑物使用Command Ability建造其他阵营的单位。

背后的原理是生产本质上是有前提条件的技能，只要产能建筑满足条件，就能使用这个技能

队伍建造可以直接无视阵营要求，例如把盟军机场放入队伍，然后就可以命令队伍建造米格战斗机达成盟军机场建造米格的效果。



2. 没有对应玩家的队伍的字符串名字是 /team，使用/team spawn出来的单位是中立单位。



3. 地编出现一堆绿色方框卡住了怎么办，顶上工具栏切换一下分辨率即可（即改变窗口大小）



4. 经常记得把地图保存为zip，这样地编崩溃了也不会丢失所有进度。

比如当弹出下面错误时，点击确定，地图就会被清空为0KB且无法恢复

![](https://cdn.nlark.com/yuque/0/2025/png/50038568/1737878896074-cc4ac2a6-9b55-432f-965d-61eb350304c1.png)



5. 日冕更新的12个地图AI，如01A_Campaign  ，  02A_Campaign, 互相之间的区别只有头像和使用的协议，但注意即使是同阵营AI，使用协议也会有不同倾向。



6. 在地图可玩边界外放置边界将导致小地图错位，放置的离边界越远，错位越严重。
