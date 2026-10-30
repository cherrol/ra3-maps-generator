---
title: "升级模块-UpgradeModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: aplzl104n6eihlpn
source: https://www.yuque.com/muzeqaq/ra3mapwiki/aplzl104n6eihlpn
fetched: 2026-10-15
content_updated_at: "2025-11-11T15:53:41.000Z"
type: Doc
format: lake
---

# 升级模块-UpgradeModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/aplzl104n6eihlpn
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::



该模块用于单位/玩家的升级



# 函数
#### unit_enable_upgrade
给单位升级/取消升级

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| unit_name_or_table | string | SystemUnitTable | |
| upgrade_name | UpgradeEnum | |
| is_enable | boolean | |


#### unit_has_upgrade
单位是否有升级

返回值: boolean

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| unit_name_or_table | string | SystemUnitTable | |
| upgrade_name | UpgradeEnum | |




#### has_upgrade
玩家是否拥有某升级

返回值: boolean

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | string | |
| upgrade_name | UpgradeEnum | |


#### allow_upgrade
允许/禁止玩家升级 

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | string | |
| upgrade_name | UpgradeEnum | |
| is_allow | boolean | |


#### enable_upgrade
给予/剥夺 玩家某升级

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | string | |
| upgrade_name | UpgradeEnum | |
| is_allow | boolean | |


#### enable_upgrade_complete_event
开启监听玩家某个升级完成的事件

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| upgrade_name | UpgradeEnum | |


#### register_upgrade_complete_callback
注册玩家某个升级完成的事件回调

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| callback_func | function | 回调函数(player_name, upgrade_instance_id, object_id) |
