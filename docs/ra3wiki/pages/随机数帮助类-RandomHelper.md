---
title: "随机数帮助类-RandomHelper"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/helper"
slug: zxnzy2158fd6qpuf
source: https://www.yuque.com/muzeqaq/ra3mapwiki/zxnzy2158fd6qpuf
fetched: 2026-10-15
content_updated_at: "2024-12-22T08:26:57.000Z"
type: Doc
format: lake
---

# 随机数帮助类-RandomHelper

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/zxnzy2158fd6qpuf
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/helper

:::success
作者: dreamness

:::

:::info
稳定, 未来不会变更

:::



使用方法可参考[8_随机](https://www.yuque.com/muzeqaq/ra3mapwiki/iydqsvinltcu4drh)



# 函数
#### random_float
获取一个0-1之间的随机浮点数

返回值: number (float)

#### randon_range_float
获取一个min_num和max_num之间的随机浮点数, _**左闭右开**_

返回值: number (float)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| min_num | number | 最小值, 包含 |
| max_num | number | 最大值, 不包含 |


#### random_range_int
获取一个min_num和max_num之间的随机整数, _**左闭右开**_

返回值: number (int)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| min_num | number | 最小值, 包含 |
| max_num | number | 最大值, 不包含 |


#### choice
从数组中随机选择一个元素

返回值: any 任意一个元素

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| array | table | 包含若干元素的数组 |
|  |  |  |


#### choices
从数组中随机选择N个不相同的元素

返回值: table (包含N个元素)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| array | table | 包含若干元素的数组 |
| n | number | 选择的元素个数 |


#### shuffle
将数组打乱, 返回新数组; 不会更改原数组

返回值: table

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| array | table | 包含若干元素的数组 |
|  |  |  |
