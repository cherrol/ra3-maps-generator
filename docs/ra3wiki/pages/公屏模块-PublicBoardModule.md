---
title: "公屏模块-PublicBoardModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: qi8m8azq407kmuat
source: https://www.yuque.com/muzeqaq/ra3mapwiki/qi8m8azq407kmuat
fetched: 2026-10-15
content_updated_at: "2025-11-11T15:43:34.000Z"
type: Doc
format: lake
---

# 公屏模块-PublicBoardModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/qi8m8azq407kmuat
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::



该模块用于在公屏输出文字

使用方法可参考[4_公屏](https://www.yuque.com/muzeqaq/ra3mapwiki/tui2u3351oecffbm)



# 函数
#### show_text
在公屏输出文字

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| text | string | 文本内容 |
| duration | number | 持续时间(秒), 最长30秒 |
| player_name_list | PlayerEnum|PlayerEnum[]|nil | 玩家名字列表, nil则针对所有玩家 |


#### set_color
设置公屏文字的颜色(是整个公屏文字的颜色)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| color | Color |  |
