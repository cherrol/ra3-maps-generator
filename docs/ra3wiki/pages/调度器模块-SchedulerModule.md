---
title: "调度器模块-SchedulerModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: upgtkdc43pqaep1x
source: https://www.yuque.com/muzeqaq/ra3mapwiki/upgtkdc43pqaep1x
fetched: 2026-10-15
content_updated_at: "2025-11-11T15:44:37.000Z"
type: Doc
format: lake
---

# 调度器模块-SchedulerModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/upgtkdc43pqaep1x
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::

:::info
稳定, 未来不会变更

:::

定时调度模块



使用方法可参考[9_定时任务](https://www.yuque.com/muzeqaq/ra3mapwiki/pp579z0wbo0dehd3)

# 函数
#### call_every_x_frame
每N帧调用一次某函数

返回值: number 调度id

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| callback | function | 回调函数 |
| interval_frame_num | number | 间隔帧数, 大于0 (15帧=1秒) |
| repeat_num | number| nil | 复次数, nil表示无限次, 0或小于0表示不调用 |
| arguments | table|nil | 回调函数参数, 数组. 元素个数需要和回调函数参数个数一致; nil等同于{} |




#### delay_call
N帧后调用函数

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| callback | function | 回调函数 |
| interval_frame_num | number | 间隔帧数, 大于0 (15帧=1秒) |
| arguments | table|nil | 回调函数参数, 数组. 元素个数需要和回调函数参数个数一致; nil等同于{} |


#### pause_scheduler
暂停某个调度

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| id | number | 调度id |
|  |  |  |


#### resume_scheduler
恢复某个调度(不会重置已运行次数)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| id | number | 调度id |
|  |  |  |


#### restart_scheduler
重启调度器(会重置已运行次数)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| id | number | 调度id |
|  |  |  |


#### get_config
获取调度器配置, 修改后会对调度有影响, 需要谨慎修改

返回值: SchedulerConfig

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| id | number | 调度id |
|  |  |  |


##### SchedulerConfig
###### 字段
| 字段名 | 数据类型 | 备注 |
| --- | --- | --- |
| callback | function | 回调函数 |
| aready_called_times | number | 已经调用的次数 |
| repeat_times | number | 重复次数 |
| interval_frame_num | number | 间隔帧数 |
| passed_frames | number | 已经过去的帧数 |
| is_active | boolean | 是否激活 |
