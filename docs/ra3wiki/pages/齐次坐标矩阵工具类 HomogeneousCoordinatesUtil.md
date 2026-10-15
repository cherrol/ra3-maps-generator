---
title: "齐次坐标矩阵工具类 HomogeneousCoordinatesUtil"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_util"
slug: ferf04icsvsi7h7c
source: https://www.yuque.com/muzeqaq/ra3mapwiki/ferf04icsvsi7h7c
fetched: 2026-10-15
content_updated_at: "2025-04-02T14:53:18.000Z"
type: Doc
format: lake
---

# 齐次坐标矩阵工具类 HomogeneousCoordinatesUtil

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/ferf04icsvsi7h7c
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_util

:::success
作者: dreamness

:::

用于单位的矩阵转换, 例程: [齐次坐标矩阵研究](https://www.yuque.com/muzeqaq/ra3mapwiki/mvato6ot39cg8s8y)

### 函数
:::info
这里的所有函数都是没有副作用的(不会更改参数的值)

:::

#### system_matrix_to_homogeneous_coordinates
将ra3的矩阵转换为标准齐次坐标矩阵

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| m | Matrix | Ra3中的单位矩阵 |


#### system_matrix_to_homogeneous_coordinates
将ra3的矩阵转换为标准齐次坐标矩阵

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| m | Matrix | 标准齐次坐标矩阵 |


#### copy
深拷贝齐次坐标矩阵

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| m | Matrix | 标准齐次坐标矩阵 |


#### get_transform_matrix_by_hc
齐次坐标矩阵转换为变换矩阵(3x3)

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| m | Matrix | 标准齐次坐标矩阵 |


#### get_translation_matrix_by_vec
获取齐次坐标矩阵的平移矩阵

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| vec | Vector | 平移向量 |


#### get_axis_vecs_by_hc
根据齐次坐标矩阵获取单位的x, y, z轴的方向向量

返回值: Vector 在x轴的方向向量, Vector 在y轴的方向向量, Vector 在z轴的方向向量

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| m | Matrix | 标准齐次坐标矩阵 |


#### get_direction_vec
获取当前方向向量

返回值: Vector

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| m | Matrix | 标准齐次坐标矩阵 |


#### get_move_origin_translation_matrix_by_hc
获取平移到原点的齐次坐标矩阵

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| m | Matrix | 标准齐次坐标矩阵 |


#### get_move_back_translation_matrix_by_hc
获取从原点平移回去的齐次坐标矩阵

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| m | Matrix | 标准齐次坐标矩阵 |


#### get_scale_matrix
获取齐次坐标矩阵的缩放矩阵

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| x | number | x轴缩放比例 |
| y | number | y轴缩放比例 |
| z | number | z轴缩放比例 |


#### get_uniform_scale_matrix
获取齐次坐标矩阵的等比缩放矩阵

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| s | number | 缩放比例 |


#### get_scale_by_hc
获取齐次坐标矩阵的缩放比例

返回值: Matrix

| 字段名 | 数据类型 | 备注 |
| --- | --- | --- |
| m | Matrix | 标准齐次坐标矩阵 |


#### get_rotation_matrix_by_vec
获取绕任意向量旋转的矩阵

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| vec | Vector | 向量 |
| angle | number | 角度(度数) |


#### get_mirror_matrix
获取齐次坐标矩阵的镜像矩阵

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| x | boolean | 是否x轴镜像 |
| y | boolean | 是否y轴镜像 |
| z | boolean | 是否z轴镜像 |


#### get_position_vec_by_hc
根据齐次坐标矩阵获取位置

返回值: Vector

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| m | Matrix | 标准齐次坐标矩阵 |


#### set_position_vec
根据齐次坐标矩阵获取位置

返回值: Matrix

| 字段名 | 数据类型 | 备注 |
| --- | --- | --- |
| m | Matrix | 标准齐次坐标矩阵 |
| vec | Vector | 位置坐标向量 |


#### merge
合并矩阵和位置向量为标准齐次坐标矩阵

返回值: Matrix

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| transform_matrix | Matrix | 变换矩阵 |
| position_vector | Vector | 位置坐标向量 |
