---
title: "生产队列模块-ProductionQueueModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: ddcuwmozmc35106y
source: https://www.yuque.com/muzeqaq/ra3mapwiki/ddcuwmozmc35106y
fetched: 2026-10-15
content_updated_at: "2024-12-22T13:49:27.000Z"
type: Doc
format: lake
---

# 生产队列模块-ProductionQueueModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/ddcuwmozmc35106y
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::



# 函数
#### get_queues
获取生产队列

返回值: ProductionQueueInfo[], number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| unit_table | SystemUnitTable | 生产建筑的unit_table |
| | | |


##### ProductionQueueInfo
| 字段名 | 数据类型 | 备注 |
| --- | --- | --- |
| Id | number | |
| ModuleId | number | XML 里模块 ID 的哈希值, 使用GameModule.get_string_fast_hash获取 |
| QueueLength | number | |
| Queue | ProductionQueueItem[] | |


##### ProductionQueueItem
| 字段名 | 数据类型 | 备注 |
| --- | --- | --- |
| Type | ProductionQueueItemTypeEnum | |
| InstanceId | number | 正在建造的单位或者升级的哈希值, 使用GameModule.get_string_fast_hash获取 |
| ProductionPercentage | number | 0-100 |
| ProductionFrames | number | |
| BuildCost | number | 花费 |
