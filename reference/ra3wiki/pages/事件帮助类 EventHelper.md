---
title: "事件帮助类 EventHelper"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/helper"
slug: eacbxyzczkczlgk5
source: https://www.yuque.com/muzeqaq/ra3mapwiki/eacbxyzczkczlgk5
fetched: 2026-10-15
content_updated_at: "2025-04-01T14:13:30.000Z"
type: Doc
format: lake
---

# 事件帮助类 EventHelper

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/eacbxyzczkczlgk5
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/helper

:::success
作者: dreamness

:::

本模块中包含对各个事件进行封装, 更加易用



## 函数
#### register_hot_key
注册键盘热键,  例程: [3_键盘事件](https://www.yuque.com/muzeqaq/ra3mapwiki/ufmb7w6aicly63ac)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| key_code_hex | KeyboardKeyCodeEnum | 按键code |
| assist_key_code_hex | KeyboardKeyCodeEnum | 辅助键code (alt,ctrl等) |
| callback_func | function | 回调函数, 参数列表: <br/>(player_name, mouse_x, mouse_y, mouse_z) |




#### register_unit_die_event
注册单位死亡事件, 例程: [26 出生与死亡事件](https://www.yuque.com/muzeqaq/ra3mapwiki/osvmc0h8wg0ngfdd)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| thing | ThingEnum | 单位名 |
| callback_func | function | 回调函数, 参数列表: <br/>(dying_unit, attacker_unit <可能为nil>) |


#### register_unit_create_event
注册单位创建事件, 例程: [26 出生与死亡事件](https://www.yuque.com/muzeqaq/ra3mapwiki/osvmc0h8wg0ngfdd)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| thing | ThingEnum | 单位名 |
| callback_func | function | 回调函数, 参数列表: <br/>(dying_unit, attacker_unit <可能为nil>) |




#### register_unit_top_button_click_event
注册单位顶部按钮, 例程: [17 单位头顶按钮](https://www.yuque.com/muzeqaq/ra3mapwiki/ev5rzdes28gxlpxa)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| unit | Unit | 单位Unit |
| callback_func | function | 回调函数, 参数列表: <br/>(player_name, unit) |




#### register_top_button_click_event
注册屏幕顶部按钮点击事件, 例程: [22 顶部按钮](https://www.yuque.com/muzeqaq/ra3mapwiki/bmqbtrup56qyv1fu)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| pos_index | number 整形 1-6 | 顶部第几个按钮 |
| callback_func | function | 回调函数, 参数列表: <br/>(player_name, shift_pressed) |


#### register_mission_hotspot_event
注册任务热点检测事件, 例程: [23 战役相关](https://www.yuque.com/muzeqaq/ra3mapwiki/dzz8lp9tyc19stdq)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| hotspot_id | string  | 任务热点的名字, 在地编中设置 |
| callback_func | function | 回调函数, 参数列表: <br/>(hotspot_id) |


#### register_mouse_position_event
注册鼠标位置事件, 例程: [25 鼠标位置](https://www.yuque.com/muzeqaq/ra3mapwiki/tmhlienmcv37eiqr)

该事件非常耗费性能, 请及时关闭

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum  | 玩家名字 |
| is_enable | boolean | 是否启用 |
