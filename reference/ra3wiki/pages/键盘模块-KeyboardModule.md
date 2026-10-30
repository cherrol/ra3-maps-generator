---
title: "键盘模块-KeyboardModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: tx4gnukob0qorerq
source: https://www.yuque.com/muzeqaq/ra3mapwiki/tx4gnukob0qorerq
fetched: 2026-10-15
content_updated_at: "2025-11-11T15:35:05.000Z"
type: Doc
format: lake
---

# 键盘模块-KeyboardModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/tx4gnukob0qorerq
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::

该模块提供监控玩家键盘按键的能力

使用方法可参考[3_键盘事件](https://www.yuque.com/muzeqaq/ra3mapwiki/ufmb7w6aicly63ac)



# 函数
#### register_hot_key
注册热键

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| key_code_hex | KeyboardKeyCodeEnum | 按键码 |
| assist_key_code_hex | KeyboardKeyCodeEnum | 辅助按键码, 用于组合键 |
| command_code | number | 热键的标识, 不可重复 |


#### register_hot_key_listener
注册热键监听器

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| func | function | 回调函数, 参数列表: player_name, command_code, mouse_x, mouse_y, mouse_z |
| | | |




#### unregister_hot_key
取消注册热键

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| command_code | number | 热键的标识 |
