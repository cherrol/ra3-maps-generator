---
title: "建造模块-BuildModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: wzvuw8ivk5m3nrvg
source: https://www.yuque.com/muzeqaq/ra3mapwiki/wzvuw8ivk5m3nrvg
fetched: 2026-10-15
content_updated_at: "2025-11-11T15:13:40.000Z"
type: Doc
format: lake
---

# 建造模块-BuildModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/wzvuw8ivk5m3nrvg
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::

### 函数
#### allow_build_building
是否允许玩家建造建筑(所有)

注: 帝国的核心不是建筑，所以不受影响

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum |  |
| is_allowed | boolean |  |


#### allow_build_unit
是否允许玩家建造单位(所有)

 注: 帝国的核心属于单位, 会受影响

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum |  |
| is_allowed | boolean |  |


#### allow_build_a_unit
允许/禁止玩家建造一个单位

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum |  |
| thing_name | ThingEnum |  |
| is_allowed | boolean |  |
