---
title: "04 过滤器与buff"
catalog_path: "Lua例子/代码示例"
slug: wcxnhggfsc6i51g9
source: https://www.yuque.com/muzeqaq/ra3mapwiki/wcxnhggfsc6i51g9
fetched: 2026-10-15
content_updated_at: "2025-02-04T09:58:58.000Z"
type: Doc
format: lake
---

# 04 过滤器与buff

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/wcxnhggfsc6i51g9
> 目录位置：Lua例子/代码示例

本示例中, 指定一个单位, 给附近的载具加上铁幕buff, 忽略步兵



首先在地编中放置多个单位, 并给其中一个单位命名为"user_unit", 并修改所属方

![](https://cdn.nlark.com/yuque/0/2025/png/34558013/1738661883831-919f1de6-04bd-46a5-89d7-4999c95c2c58.png)

# 使用lua库
以下代码需要每帧运行

```lua
local filter = FilterHelper.new_filter_builder()
:set_rule(FilterRuleEnum.ANY)
:add_include_kind_ofs(KindOfEnum.VEHICLE)
:build()


local user_unit = UnitHelper.get_unit_from_name('user_unit')
local pos_x, pos_y, pos_z = user_unit:get_position()
-- 以单位位置位圆心, 50为半径的区域
local area_config = FilterHelper.new_area_config(pos_x, pos_y, pos_z, 50, AreaDistTypeEnum.CENTER_2D)

local target_units, target_unit_cnt = filter:filter_units(area_config, user_unit)
for i = 1, target_unit_cnt do
    local target_unit = target_units[i]
    -- 设置铁幕buff, 且持续1s(也就是说, 玩家的单位离开后铁幕会随即失效)
    target_unit:add_modifier(AttributeModifierEnum.IRON_CURTAIN, 12 * 1)
end
```



# 使用原生api
以下代码需要每帧运行

```lua
local filter_config = {
    Rule = "ANY",
    Include = "VEHICLE"
}

local filter = CreateObjectFilter(filter_config)

local user_unit_table = GetObjectByScriptName('user_unit')
local pos_x, pos_y, pos_z = ObjectGetPosition(user_unit_table)

local area_config = {
    X = pos_x, Y = pos_y, Z = pos_z,
    Radius = 50,
    DistType = 'CENTER_2D'
}
local target_unit_tables, target_unit_cnt =  ObjectFindObjects(user_unit_table, area_config, filter)

for i = 1, target_unit_cnt do
    local target_unit_table = target_unit_tables[i]
    ObjectLoadAttributeModifier(target_unit_table, 'AttributeModifier_IronCurtain',  12 * 1)
end
```
