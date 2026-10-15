---
title: "状态模块-StatusModel"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: obe5veop3plh3a32
source: https://www.yuque.com/muzeqaq/ra3mapwiki/obe5veop3plh3a32
fetched: 2026-10-15
content_updated_at: "2024-12-22T11:03:08.000Z"
type: Doc
format: lake
---

# 状态模块-StatusModel

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/obe5veop3plh3a32
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::



# 函数
#### is_unit_has_status
单位是否拥有状态

返回值: boolean

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| unit_table_or_name | SystemUnitTable|string  | |
| status | StatusEnum | |


#### unit_change_status
改变单位状态

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| unit_table_or_name | SystemUnitTable|string  | |
| statuses | StatusEnum | StatusEnum[] | 状态或状态数组 |
| is_active | boolean | 添加/删除 |
