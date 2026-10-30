---
title: "过滤器构造器-FilterBuilder"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/object"
slug: afkp6v2oyzr3wm4k
source: https://www.yuque.com/muzeqaq/ra3mapwiki/afkp6v2oyzr3wm4k
fetched: 2026-10-15
content_updated_at: "2025-05-31T04:40:44.000Z"
type: Doc
format: lake
---

# 过滤器构造器-FilterBuilder

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/afkp6v2oyzr3wm4k
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/object

:::success
作者: dreamness

:::

:::info
稳定, 未来不会变更

:::

这是一个类, 需要通过FilterHelper.new_filter_builder()来获取一个实例

支持链式调用, 如:

```lua
FilterHelper.new_filter_builder()
    :set_rule(FilterRuleEnum.ALL)
    :add_relationships({FilterRelationshipEnum.ENEMIES, FilterRelationshipEnum.NEUTRAL})
    :add_include_kind_ofs({KindOfEnum.VEHICLE, KindOfEnum.INFANTRY})
    :add_exclude_statuses(DemoBrainVehicle.controlled_status_name)
    :add_exclude_things(DemoBrainVehicle.immune_things)
    :build()
```



使用方法可参考 [7_过滤器](https://www.yuque.com/muzeqaq/ra3mapwiki/cky92r3frg6zfega)  [1_脑车](https://www.yuque.com/muzeqaq/ra3mapwiki/nka8b2x5fkewmo0r)

### 方法
#### build
构建Filter对象

#### add_include_things
添加过滤器包含的单位

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| things | ThingEnum[] | ThingEnum | 单位的种类, 接受单个或者数组 |
| | | |


#### add_exclude_things
添加过滤器排除的单位

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| things | ThingEnum[] | ThingEnum | 单位的种类, 接受单个或者数组 |
| | | |


#### add_include_statuses
添加包含的状态

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| statuses | StatusEnum[]|StatusEnum | 状态, 接受单个或者数组 |
| | | |


#### add_exclude_statuses
添加排除的状态

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| statuses | StatusEnum[]|StatusEnum | 状态, 接受单个或者数组 |
| | | |


#### add_include_kind_ofs
添加包含的种类

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| kind_ofs | KindOfEnum[]|KindOfEnum | 单位的类别, 接受单个或者数组 |
| | | |


#### add_exclude_kind_ofs
添加排除的种类

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| kind_ofs | KindOfEnum[]|KindOfEnum | 单位的类别, 接受单个或者数组 |
| | | |


#### add_relationships
添加关系过滤

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| relationships | FilterRelationshipEnum[]|FilterRelationshipEnum | 关系, 接受单个或者数组 |
| | | |


#### set_rule
设置过滤器的过滤规则

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| rule | FilterRuleEnum | 设置过滤器的过滤规则 |
| | | |
