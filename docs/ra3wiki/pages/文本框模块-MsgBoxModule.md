---
title: "文本框模块-MsgBoxModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: zaeicggzut461apt
source: https://www.yuque.com/muzeqaq/ra3mapwiki/zaeicggzut461apt
fetched: 2026-10-15
content_updated_at: "2024-12-22T13:49:18.000Z"
type: Doc
format: lake
---

# 文本框模块-MsgBoxModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/zaeicggzut461apt
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::

该模块所显示的文本均需要在map.str中配置, 并注意编码为utf8

是否用方法可参考 [11_文本框](https://www.yuque.com/muzeqaq/ra3mapwiki/oigo98mvdi9d4m96)



图片参数可参考 [538号脚本图片参数，Show Info Box, Button Image 参数](https://www.yuque.com/muzeqaq/ra3mapwiki/hd2b4xwe6mgn89ux)

# 函数
#### show_msg_box_on_unit
在单位上显示文本框, 并可以带图片

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| unit_name_or_table | string|SystemUnitTable | |
| msg | string | 信息(map.str中的key) |
| seconds | number | 持续时长(s) |
| img | ButtonTextureEnum|nil | 图片 |


#### remove_msg_box_on_unit
移除单位上的信息框 

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| unit_name_or_table | string|SystemUnitTable | |


#### show_msg_caption
显示标题文字

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| msg | string | 信息(map.str中的key) |
| seconds | number | 持续时长(s) |
| player_name | PlayerEnum|nil | 玩家名字, nil为全体玩家 |
