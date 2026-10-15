---
title: "聊天框模块-ChatMsgModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: zh6fp9t9qimniwhf
source: https://www.yuque.com/muzeqaq/ra3mapwiki/zh6fp9t9qimniwhf
fetched: 2026-10-15
content_updated_at: "2024-12-22T06:31:49.000Z"
type: Doc
format: lake
---

# 聊天框模块-ChatMsgModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/zh6fp9t9qimniwhf
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::info
作者: dreamness

:::

该模块可以在聊天框输出文字, 检测玩家文字



使用方法可参考 [2_多人游戏的聊天框](https://www.yuque.com/muzeqaq/ra3mapwiki/rz2oftuit1n1m5i6)

# 函数
#### print_on_chat
在聊天框输出文字

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| message | string | 文本内容 |
| | | |




#### register_chat_listener
注册聊天监听函数

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| func | function | 回调函数, 参数列表: player_name, content |
| | | |
