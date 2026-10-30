---
title: "过滤器-Filter"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/object"
slug: qche37h4qyg295gq
source: https://www.yuque.com/muzeqaq/ra3mapwiki/qche37h4qyg295gq
fetched: 2026-10-15
content_updated_at: "2025-04-01T14:30:05.000Z"
type: Doc
format: lake
---

# 过滤器-Filter

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/qche37h4qyg295gq
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/object

:::success
作者: dreamness

:::

:::info
稳定, 未来不会变更

:::

Filter是一个类, 不能直接使用, 需要通过FilterBuilder进行构建



使用方法可参考 [7_过滤器](https://www.yuque.com/muzeqaq/ra3mapwiki/cky92r3frg6zfega)  [1_脑车](https://www.yuque.com/muzeqaq/ra3mapwiki/nka8b2x5fkewmo0r) [20 记分板(可替代计数器)](https://www.yuque.com/muzeqaq/ra3mapwiki/ddqkagi921kz0r7s)

# 字段
| 字段名 | 数据类型 | 备注 |
| --- | --- | --- |
| system_filter | | 存储实际ra3中的filter对象 |
| | | |


# 方法
:::color3
这里的方法需要使用冒号来调用, 如:

air_unit_filter:filter_units(nil, nil)

:::

#### filter_units(area_config, relation_reference_unit)
在战场上过滤单位

返回值: Unit[](符合过滤器的单位列表), number(单位个数)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| area_config | AreaConfig | nil | 过滤的区域, 全局搜索填入nil |
| relation_reference_unit | Unit | nil | 来参考关系的单位. 如果构建filter时配置了relationship就必须填入该值 |




#### is_pass(unit, relation_reference_unit)
单位是否符合过滤器

返回值: boolean

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| unit | Unit | |
| relation_reference_unit | Unit | nil | 来参考关系的单位. 如果构建filter时配置了relationship就必须填入该值 |
