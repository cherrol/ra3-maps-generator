---
title: "Table工具类 TableUtil"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_util"
slug: hcn7wog1sk47935y
source: https://www.yuque.com/muzeqaq/ra3mapwiki/hcn7wog1sk47935y
fetched: 2026-10-15
content_updated_at: "2025-04-02T14:54:36.000Z"
type: Doc
format: lake
---

# Table工具类 TableUtil

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/hcn7wog1sk47935y
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_util

:::success
作者: dreamness

:::

### 函数
#### get_or_default
根据索引获取table的值

返回值: any

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| t | table |  |
| key_or_index | string|number |  |
| default_value | any | 默认值, 如果取不到值, 就返回该值 |


#### to_string
table转字符串, 用于输出日志

返回值: string

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| t | table |  |


#### arr_to_string
数组转字符串, 用于输出日志

返回值: string

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| t | table |  |


#### dict_to_string
字典转字符串, 用于输出日志

返回值: string

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| t | table |  |


#### is_ele_in_array
判断元素是否在数组中

返回值: booleanb

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| t | table |  |
| ele | any |  |


#### array_shallow_copy
数组浅拷贝

返回值: table

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| t | table |  |


#### dict_shallow_copy
字典浅拷贝

返回值: table

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| t | table |  |


#### make_array
构造数组

返回值: table

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| ele | any |  |
| cnt | num | 元素个数 |


#### merge_array
合并数组

返回值: table

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| ... | table | 合并多个数组 |
