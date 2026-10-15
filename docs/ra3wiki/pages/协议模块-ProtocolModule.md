---
title: "协议模块-ProtocolModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: ol3cr1gppapstbtu
source: https://www.yuque.com/muzeqaq/ra3mapwiki/ol3cr1gppapstbtu
fetched: 2026-10-15
content_updated_at: "2025-11-11T15:42:48.000Z"
type: Doc
format: lake
---

# 协议模块-ProtocolModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/ol3cr1gppapstbtu
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::



# 函数
#### grant_protocol
授权玩家协议

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum | |
| protocol_name | ProtocolEnum | |




#### set_max_point
设置能获取的协议点数

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum | |
| cnt | number | |


#### set_point
设置协议点数

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum | |
| cnt | number | |


#### is_ready
协议是否就绪

返回值: boolean

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum | |
| protocol_name | ProtocolEnum | |


#### set_player_protocol_tech_faction
覆盖玩家的协议面板

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum | |
| faction_name | FactionEnum | |




#### unlock_rank_tech
解锁协议等级(第几行)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum | |
| level | number | 等级 (2-10) |
