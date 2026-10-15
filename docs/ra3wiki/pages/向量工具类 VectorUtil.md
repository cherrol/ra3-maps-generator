---
title: "向量工具类 VectorUtil"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_util"
slug: ryx2tp2ikgy3yg0q
source: https://www.yuque.com/muzeqaq/ra3mapwiki/ryx2tp2ikgy3yg0q
fetched: 2026-10-15
content_updated_at: "2025-04-02T15:00:18.000Z"
type: Doc
format: lake
---

# 向量工具类 VectorUtil

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/ryx2tp2ikgy3yg0q
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_util

:::success
作者: dreamness

:::

### 定义
Vector是一个一维数组

### 函数
#### norm
向量的模

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| vec | Vector |  |


#### to_unit_vec
向量转为单位向量

返回值: Vector

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| vec | Vector |  |




#### dot
向量点乘

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| vec1 | Vector |  |
| vec2 | Vector |  |


#### angle
获取向量夹角

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| vec1 | Vector |  |
| vec2 | Vector |  |


#### cross3
3维向量叉乘

返回值: Vector

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| vec1 | Vector |  |
| vec2 | Vector |  |




#### cross2
2维向量叉乘

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| vec1 | Vector |  |
| vec2 | Vector |  |


#### multiply
向量乘法

返回值: Vector

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| vec | Vector |  |
| num | number |  |


#### add
向量加法

返回值: Vector

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| vec1 | Vector |  |
| vec2 | Vector |  |


#### subtract
向量减法

返回值: Vector

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| vec1 | Vector |  |
| vec2 | Vector |  |


#### to_string
向量转字符串, 用于日志输出

返回值: string

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| vec | Vector |  |


#### point_distance
两点之间的距离

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| p1 | Vector |  |
| p2 | Vector |  |


#### copy
复制向量

返回值: Vector

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| vec | Vector |  |
