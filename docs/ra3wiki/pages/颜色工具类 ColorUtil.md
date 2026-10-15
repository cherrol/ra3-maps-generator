---
title: "颜色工具类 ColorUtil"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_util"
slug: rn3hl4gye0ru2213
source: https://www.yuque.com/muzeqaq/ra3mapwiki/rn3hl4gye0ru2213
fetched: 2026-10-15
content_updated_at: "2025-05-11T15:51:31.000Z"
type: Doc
format: lake
---

# 颜色工具类 ColorUtil

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/rn3hl4gye0ru2213
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_util

:::success
作者: dreamness

:::

## 定义Color
Color是一个table, 包含以下字段:

| 字段名 | 数据类型 | 备注 |
| --- | --- | --- |
| r | number | 红色值 |
| g | number | 绿色值 |
| b | number | 蓝色值 |
| hex_value | string | 十六进制值 |
| dec_value | number | 十进制值 |


## 函数
#### get_color_from_rgb
从rgb值获取颜色

返回值: Color

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| r | number | 红色值 |
| g | number | 绿色值 |
| b | number | 蓝色值 |


#### get_color_from_hex
从十六进制值获取颜色

返回值: Color

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| hex_value | string | 十六进制值 |




#### is_color_equal
颜色判等

返回值: boolean

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| color1 | Color |  |
| color2 | Color |  |
