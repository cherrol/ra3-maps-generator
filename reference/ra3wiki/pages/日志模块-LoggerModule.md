---
title: "日志模块-LoggerModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: gvd1zzdwzcfgik9s
source: https://www.yuque.com/muzeqaq/ra3mapwiki/gvd1zzdwzcfgik9s
fetched: 2026-10-15
content_updated_at: "2025-11-11T15:35:58.000Z"
type: Doc
format: lake
---

# 日志模块-LoggerModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/gvd1zzdwzcfgik9s
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::

:::info
稳定, 未来不会变更

:::

提供日志功能, 排查问题更加便捷

使用方法可参考[1_日志系统](https://www.yuque.com/muzeqaq/ra3mapwiki/nluv89iekbhc3pwp)



该模块有一个别名, logger, 与LoggerModule等价

# 函数
#### setup
初始化日志模块

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| level | LoggerLevelEnum | 日志等级 |
| targets | LoggerTargetEnum[] | 日志输出的目标数组 |


#### error
记录一条error日志

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| pos | string | 记录日志的位置, 用于问题排查 |
| msg | string | 日志内容 |


#### warn
记录一条error日志

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| pos | string | 记录日志的位置, 用于问题排查 |
| msg | string | 日志内容 |


#### info
记录一条info日志

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| pos | string | 记录日志的位置, 用于问题排查 |
| msg | string | 日志内容 |


#### debug
记录一条debug日志

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| pos | string | 记录日志的位置, 用于问题排查 |
| msg | string | 日志内容 |


#### trace
记录一条trace日志

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| pos | string | 记录日志的位置, 用于问题排查 |
| msg | string | 日志内容 |


#### log
忽略等级, 直接输出一条日志

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| log_content | string | 日志内容 |
| target | LoggerTargetEnum | 日志输出的目标 |


#### 
#### 
####
