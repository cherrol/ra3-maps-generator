---
title: "单位创建帮助类-UnitCreateHelper"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/helper"
slug: igaotdou4iz2d55u
source: https://www.yuque.com/muzeqaq/ra3mapwiki/igaotdou4iz2d55u
fetched: 2026-10-15
content_updated_at: "2025-04-01T14:35:46.000Z"
type: Doc
format: lake
---

# 单位创建帮助类-UnitCreateHelper

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/igaotdou4iz2d55u
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/helper

:::success
作者: dreamness

:::

用于快速构建单位的帮助模块

在该帮助类中, 有参数waypoint_name, 这个路径点可以是一个临时的路径点, 想要更改相关参数如角度和位置, 可以直接对返回值(Unit)操作



例程: [16 创建单位](https://www.yuque.com/muzeqaq/ra3mapwiki/mrm89fohi51g94ng)

# 函数
#### create_unit_at_waypoint
在路径点创建单位, 返回被创建的Unit对象

返回: Unit

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| unit_name | string | 名字，可以使用UnitHelper.get_unit_auto_name()快速获取一个名字 |
| thing | ThingEnum | 物体名 |
| team_name | TeamEnum | 队伍名字, 如果不想分配队伍, 可以通过TeamModule.from_player_name(xx)获取默认的队伍名 |
| waypoint_name | string | 路径点名字 |
| attrs | AttributeModifierEnum | AttributeModifierEnum[] | nil | BUFF或BUFF数组 |
| attrs_frame_cnt | number | nil | BUFF持续时间(帧数), nil为默认值; 想要一直生效可以设置为一个非常大的数 |
| statuses | StatusEnum | StatusEnum[] | nil | 状态或状态数组 |
| | | |


#### create_refinery
在矿脉上建造矿场

返回值: Unit

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| ore_node_unit | Unit | 矿脉Unit |
| faction_name | FactionEnum | 派系 |
| player_name | PlayerEnum | 玩家名 |
| waypoint_name | string | 一个临时的路径点, 在哪不重要 |
|  |  |  |
|  |  |  |


#### create_unit_with_garrison
创建有乘客的单位

返回值: Unit(母单位), Unit[] (子单位列表)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| parent_thing_name | ThingEnum | 母物体名 |
| child_thing_names | ThingEnum[] | 子物体名 |
| team_name | TeamEnum | 所属队伍名 |
| waypoint_name | string | 生成单位的路径点 |
|  |  |  |
