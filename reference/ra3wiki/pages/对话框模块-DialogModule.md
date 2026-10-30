---
title: "对话框模块-DialogModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: yao4cedy2oh3dihv
source: https://www.yuque.com/muzeqaq/ra3mapwiki/yao4cedy2oh3dihv
fetched: 2026-10-15
content_updated_at: "2025-04-05T12:14:14.000Z"
type: Doc
format: lake
---

# 对话框模块-DialogModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/yao4cedy2oh3dihv
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::

该模块提供显示对话框, 注册按钮点击事件的功能



使用方法可参考[10_对话框](https://www.yuque.com/muzeqaq/ra3mapwiki/ilbcm705w5ixsfu6)

# 函数
#### get_auto_dialog_id
获取一个自动的对话框id

返回值: number



#### show_choice_dialog
显示选择对话框, 最多支持7个选项

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| dialog_id | number | 弹框id, 不要重复 |
| title | string | 弹框标题 |
| button_contents | string[] | 按钮内容数组, 最多7个 |
| player_name_list | PlayerEnum|PlayerEnum[]|nil | 玩家名字列表, nil则针对所有玩家 |


#### hide_choice_dialog
隐藏选择对话框

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name_list | PlayerEnum|PlayerEnum[]|nil | 玩家名字列表, nil则针对所有玩家 |




#### show_long_text_dialog
显示长文本弹框, 弹框全局只有一个，如果之前的弹框还未消失，则会覆盖之前的弹框数据和ui, 弹框按钮点击后，弹框自动消失

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| dialog_id | number | 弹框id, 不要重复 |
| title | string | 弹框标题 |
| button_contents | string[] | 按钮内容数组, 最多7个 |
| player_name_list | PlayerEnum|PlayerEnum[]|nil | 玩家名字列表, nil则针对所有玩家 |


#### hide_long_text_dialog
隐藏选择对话框

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name_list | PlayerEnum|PlayerEnum[]|nil | 玩家名字列表, nil则针对所有玩家 |






#### register_dialog_click_listener
注册对话框按钮点击事件

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| func | function|nil | 回调函数, 参数列表: player_name, btn_index, dialog_id |
