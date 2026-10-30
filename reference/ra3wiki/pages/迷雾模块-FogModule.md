---
title: "迷雾模块-FogModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: tcgy0qutxig4ntiz
source: https://www.yuque.com/muzeqaq/ra3mapwiki/tcgy0qutxig4ntiz
fetched: 2026-10-15
content_updated_at: "2025-11-11T15:21:25.000Z"
type: Doc
format: lake
---

# 迷雾模块-FogModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/tcgy0qutxig4ntiz
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::

### 函数
#### reveal_map
为玩家展开全图视野 

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum | nil | nil针对所有玩家 |
| is_enable | boolean |  |


#### is_unit_in_sight
判断单位是否在玩家的视野中 

返回值: boolean

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum | nil | nil针对所有玩家 |
| unit_name_or_table | string | SystemUnitTable |  |


#### reveal_area
为玩家揭示区域中的视野

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum|nil | nil针对所有玩家 |
| area_name | string |  |
| reveal_name | string |  |
