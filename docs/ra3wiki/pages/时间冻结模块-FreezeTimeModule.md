---
title: "时间冻结模块-FreezeTimeModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: ei80z994qyh5rbdb
source: https://www.yuque.com/muzeqaq/ra3mapwiki/ei80z994qyh5rbdb
fetched: 2026-10-15
content_updated_at: "2025-04-27T00:46:47.000Z"
type: Doc
format: lake
---

# 时间冻结模块-FreezeTimeModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/ei80z994qyh5rbdb
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::

### 函数
#### enable_freeze_time
不是真的冻结, 只是单位不动, 单位还是会移动; 恢复后显示在移动后的位置, 而不是冻结时的位置

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| is_enable | boolean | 是否启用冻结时间 |
| only_for_camera_moving | boolean | 是否只用于相机移动 |
