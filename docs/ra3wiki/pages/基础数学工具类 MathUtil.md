---
title: "基础数学工具类 MathUtil"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_util"
slug: ss0olgid14fhtze7
source: https://www.yuque.com/muzeqaq/ra3mapwiki/ss0olgid14fhtze7
fetched: 2026-10-15
content_updated_at: "2025-04-02T14:53:45.000Z"
type: Doc
format: lake
---

# 基础数学工具类 MathUtil

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/ss0olgid14fhtze7
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_util

### 函数
#### sin
正弦

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| angle | number | 角度 |


#### cos
正弦

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| angle | number | 角度 |


#### tan
正切

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| angle | number | 角度 |


#### asin
反正弦

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| angle | number | 角度 |


#### acos
反余弦

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| angle | number | 角度 |


#### atan
反正切

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| angle | number | 角度 |


#### atan2
反正切

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| y | number |  |
| x | number |  |


#### dec_to_hex
十进制转十六进制

返回值: strings

| 字段名 | 数据类型 | 备注 |
| --- | --- | --- |
| dec | number | 十进制数 |


#### hex_to_dec
正弦

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| hex | string | 十六进制 |


#### is_integer
判断是否为整数

返回值: boolean

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| n | number |  |


#### is_in_error_range
判断是否在误差范围内

返回值: boolean

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| n | number | 被测试的值 |
| target | number | 目标的标准值 |
| tolerate | number | 允许的误差(正负) |
