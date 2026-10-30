---
title: "结盟模块-AllegianceModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: idlmb4g3sfabhpi0
source: https://www.yuque.com/muzeqaq/ra3mapwiki/idlmb4g3sfabhpi0
fetched: 2026-10-15
content_updated_at: "2025-04-27T00:49:26.000Z"
type: Doc
format: lake
---

# 结盟模块-AllegianceModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/idlmb4g3sfabhpi0
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::

:::info
稳定, 未来不会变更

:::



用于设置玩家间的结盟关系

# 函数
#### set_allegiance_single_way
单向设置关系

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name1 | PlayerEnum | |
| player_name2 | PlayerEnum | |
| relationship | AllegianceEnum | 关系 |




#### set_allegiance_both_way
双向设置关系

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name1 | PlayerEnum | |
| player_name2 | PlayerEnum | |
| relationship | AllegianceEnum | 关系 |


#### get_player_relation
获取玩家关系

返回值: AllegianceEnum

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name1 | PlayerEnum | |
| player_name2 | PlayerEnum | |
