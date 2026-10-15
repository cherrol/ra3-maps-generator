---
title: "路径点模块-WaypointModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: vc55uanxs7m59ev9
source: https://www.yuque.com/muzeqaq/ra3mapwiki/vc55uanxs7m59ev9
fetched: 2026-10-15
content_updated_at: "2025-04-26T12:26:25.000Z"
type: Doc
format: lake
---

# 路径点模块-WaypointModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/vc55uanxs7m59ev9
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::

### 函数
#### set_position
修改Waypoint点的坐标位置  坐标都需要是整数

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| waypoint_name | string |  |
| x | number |  |
| y | number |  |
| z | number |  |




#### get_position
获取路径点坐标

返回值: number x, number y, number  z 如果是非法的路径点, 返回 nil, nil, nil

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| waypoint_name | string |  |
