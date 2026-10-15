---
title: "地编脚本中的BUG"
catalog_path: ""
slug: crd4xmcur34fpkgc
source: https://www.yuque.com/muzeqaq/ra3mapwiki/crd4xmcur34fpkgc
fetched: 2026-10-15
content_updated_at: "2025-01-16T22:24:00.000Z"
type: Doc
format: lake
---

# 地编脚本中的BUG

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/crd4xmcur34fpkgc
> 目录位置：(未分组)

:::info
作者：Inertia

日期：2024年11月11日

:::

# 地编脚本条件
### 23号：`NAMED_DISCOVERED`
    - **功能本意**：用于检测特定物体是否被特定玩家看见（即是否处于战争迷雾中）。
    - **实际问题**：该函数仅对建筑物和静止物体的可见性检测准确。
        * 对于正在移动的物体，该函数只在物体停下的时候会返回正确的是否可见状态。
        * 但如果该物体没有停下，即使有大量侦察单位已发现它，函数均返回“不可见”。

### 45号：`ENEMY_SIGHTED`
    - **功能本意**：用于检测某个单位是否看见了敌军。
    - **实际问题**：经过验证，对于部分单位，该函数检测的实际上是敌军是否在一个小范围（疑似射程范围）内，而非视野范围。
        * **出问题的单位例子**：天眼哨机、阿波罗、战熊、警犬。
        * **基本正常的单位例子**：将军、无畏
        * **具体表现**：对于视野范围较大但射程较短的单位（如天眼哨机、阿波罗），当敌军进入视野范围时，函数仍返回“没看见”；只有当敌军进入非常短的距离内，函数才会返回“看见”。

>             + 距离有多短？
>             + 使用冰冻直升机和VX直升机测试，首次接近敌方时，距离在略大于索敌半径，但远小于视野半径时，函数会返回看见。直升机立即远离，但留在视野半径内，该“看见”结果会保留几秒钟，随后变为“没看见”
>

### 56号：`PLAYER_HAS_COMPARISON_UNIT_KIND_IN_TRIGGER_AREA`
    - **功能本意**：用于检测某个区域内属于某个玩家的单位数量大小。
    - **实际问题**：该函数不区分具体的玩家参数。
        * 例如，当参数设置为`Player_1`时，`Player_2`的单位也会被计算在内。



# 地编脚本动作
### 405号：`LINK_MISSION_OBJECTIVE_TO_OBJECT`
该函数在LUA代码中无效，原因疑似为MissionObjective参数不识别。

类似问题也出现在492号`SET_MISSION_OBJECTIVE_STATUS`中。

#
