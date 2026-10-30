---
title: "BUFF模块-AttributeModifierModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: sggiian7wo9o9ctf
source: https://www.yuque.com/muzeqaq/ra3mapwiki/sggiian7wo9o9ctf
fetched: 2026-10-15
content_updated_at: "2025-04-02T15:04:12.000Z"
type: Doc
format: lake
---

# BUFF模块-AttributeModifierModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/sggiian7wo9o9ctf
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::

BUFF模块



# 函数
#### add_modifier
添加BUFF

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| unit_table | SystemUnitTable | |
| modifiers | AttributeModifierEnum | AttributeModifierEnum[] | BUFF或BUFF数组 |
| frame_cnt | number|nil | 帧数, nil为默认 |


#### remove_modifier
移除BUFF

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| unit_table | SystemUnitTable | |
| modifiers | AttributeModifierEnum | AttributeModifierEnum[] | BUFF或BUFF数组 |
