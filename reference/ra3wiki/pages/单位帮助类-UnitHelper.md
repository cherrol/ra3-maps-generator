---
title: "单位帮助类-UnitHelper"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/helper"
slug: aebufv1tgvpl5ell
source: https://www.yuque.com/muzeqaq/ra3mapwiki/aebufv1tgvpl5ell
fetched: 2026-10-15
content_updated_at: "2024-12-22T02:05:25.000Z"
type: Doc
format: lake
---

# 单位帮助类-UnitHelper

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/aebufv1tgvpl5ell
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/helper

:::success
作者: dreamness

:::

:::info
稳定, 未来不会变更

:::

用于构建Unit实例的帮助模块

# 函数
#### get_unit_auto_name
获取一个自增的字符串, 可以用于单位命名

返回值: string

#### get_unit_from_id
根据id获取单位对象

返回值: Unit

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| id | number | |
| | | |


#### get_unit_from_name
据name获取单位对象

返回值: Unit

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| name | string | |
| | | |


#### get_unit_from_table
据unit_table获取单位对象

返回值: Unit

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| unit_table | SystemUnitTable | |
| | | |
