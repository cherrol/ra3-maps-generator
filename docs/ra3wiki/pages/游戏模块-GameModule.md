---
title: "游戏模块-GameModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: hunq8oxgyr9cdlxs
source: https://www.yuque.com/muzeqaq/ra3mapwiki/hunq8oxgyr9cdlxs
fetched: 2026-10-15
content_updated_at: "2025-11-11T15:30:37.000Z"
type: Doc
format: lake
---

# 游戏模块-GameModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/hunq8oxgyr9cdlxs
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::

该模块提供一些和游戏流程和系统相关的函数



# 函数
#### get_frame_number
获取当前帧序号

返回值: number

#### get_string_fast_hash
获取字符串的快速哈希值

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| content | string | |


#### game_over_with_victory_screen
游戏结束, 显示胜利画面

#### game_over_with_defeat_screen
游戏结束, 显示失败画面

#### game_over_with_game_over_screen
游戏结束, 显示游戏结束画面

#### enable_script
启用/禁用脚本

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| script_name | string | |
| enable | boolean | |




#### get_duration_from_last_call_nanosecond
获取上一次调用该函数到这次调用的时间间隔，单位是纳秒

return: number



#### get_duration_from_last_call_millisecond
获取上一次调用该函数到这次调用的时间间隔，单位是毫秒

return: number



#### to_ra3_boolean
将lua的值, 转为ra3lua独有的boolean

reutrn: boolean

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| value | any | |


#### from_ra3_boolean
将ra3lua独有的boolean, 转为通用的lua4布尔值

return:  1 或 nil

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| value | boolean | |
