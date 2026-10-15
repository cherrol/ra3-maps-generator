---
title: "矩阵工具类 MatrixUtil"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_util"
slug: dgk7k0cakd7i4qg5
source: https://www.yuque.com/muzeqaq/ra3mapwiki/dgk7k0cakd7i4qg5
fetched: 2026-10-15
content_updated_at: "2025-04-02T14:54:09.000Z"
type: Doc
format: lake
---

# 矩阵工具类 MatrixUtil

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/dgk7k0cakd7i4qg5
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_util

:::success
作者: dreamness

:::

### 定义
Matrix是一个二维数组



### 函数
#### dot
矩阵点乘

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| 。... | Matrix | 变长参数, 接受多个矩阵连续点乘 |


#### dot_vector
矩阵与向量相乘

返回值: Vector

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| m | Matrix |  |
| v | Vector |  |


#### transpose
转置矩阵

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| m | Matrix |  |


#### add
矩阵相加

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| 。... | Matrix | 变长参数, 接受多个矩阵连续相加 |


#### m_tostring
矩阵转字符串, 用于日志输出

返回值: string

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| m | Matrix |  |


#### unit_matrix
获取单位矩阵

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| rank | number | 秩 |


#### multiply
矩阵乘法

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| m | Matrix |  |
| num | number |  |
