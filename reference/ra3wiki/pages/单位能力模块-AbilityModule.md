---
title: "单位能力模块-AbilityModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: cwassez1gxwtyl1t
source: https://www.yuque.com/muzeqaq/ra3mapwiki/cwassez1gxwtyl1t
fetched: 2026-10-15
content_updated_at: "2025-04-02T15:03:19.000Z"
type: Doc
format: lake
---

# 单位能力模块-AbilityModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/cwassez1gxwtyl1t
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::



不建议直接使用, 建议使用[单位-Unit](https://www.yuque.com/muzeqaq/ra3mapwiki/ozofvf18pu9gxs2g)

# 函数
#### unit_use_ability
使用技能

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| unit_table_or_name | string|SystemUnitTable | 单位table或单位名 |
| ability_name | AbilityEnum | 技能名 |


#### unit_use_ability_at_waypoint
使用技能

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| unit_table_or_name | string|SystemUnitTable | 单位table或单位名 |
| ability_name | AbilityEnum | 技能名 |
| waypoint | string | 路径点 |


#### unit_use_ability_on_unit
使用技能

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| unit_name | string | 单位名 |
| ability_name | AbilityEnum | 技能名 |
| target_unit_name | string | 目标单位名 |
