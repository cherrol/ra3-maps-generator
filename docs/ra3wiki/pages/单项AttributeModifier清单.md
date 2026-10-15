---
title: "单项AttributeModifier清单"
catalog_path: "Lua API "
slug: chqogqlxe9pzb0k1
source: https://www.yuque.com/muzeqaq/ra3mapwiki/chqogqlxe9pzb0k1
fetched: 2026-10-15
content_updated_at: "2026-04-27T13:26:30.000Z"
type: Doc
format: lake
---

# 单项AttributeModifier清单

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/chqogqlxe9pzb0k1
> 目录位置：Lua API 

:::info
作者：物

日期：2024年12月14日

更新者：Inertia, <font style="color:#2F4BDA;">dreamness</font>

更新日期：2026年04月27日

:::

# 自定义buff
```lua
local stackingLimit = 3 -- 最大层数
local attrId = exAttributeModifierCreate({
    HEALTH_MULT = 100,
    DAMAGE_MULT = 25.5,
}, stackingLimit)
-- 创建好后直接使用即可
ObjectLoadAttributeModifier(unit, attrId, 999)
```

其中, 自定义buff的效果名字如下:

ARMOR

DAMAGE_ADD

DAMAGE_MULT

RESIST_FEAR

RESIST_TERROR

EXPERIENCE

RANGE

SPEED

CRUSH_DECELERATE

RESIST_KNOCKBACK

SPELL_DAMAGE

RECHARGE_TIME

PRODUCTION

PRODUCTION_COST

HEALTH

HEALTH_MULT

VISION

BOUNTY_PERCENTAGE

MIN_CRUSH_VELOCITY_PERCENTAGE

AUTO_HEAL

SHROUD_CLEARING

RATE_OF_FIRE

DAMAGE_STRUCTURE_BOUNTY_ADD

CRUSHER_LEVEL

COMMAND_POINT_BONUS

CRUSHABLE_LEVEL

CRUSHED_DECELERATE

INVULNERABLE

SUPPRESSABILITY

RESIST_EMP

POWER_BOOST

AREA_OF_EFFECT

COLLISION_GEOMETRY_SIZE_MULT

BROADCAST_RANGE

# 单项Buffer
下面列出的AttributeModifier为`ObjectLoadAttributeModifier`的参数，见[(日冕V3.22x)LuaAPI-LuaBridge](https://www.yuque.com/muzeqaq/ra3mapwiki/rg4gm1ibq054y6o4) 第38条。

这些AttributeModifier直接对单位的单个属性进行修改，通过合适的组合，能够替代 [组合式AttributeModifier清单](https://www.yuque.com/muzeqaq/ra3mapwiki/wpz5fwgb181qgy4r#6edd) 中的各类仅修改单位属性的组合式AttributeModifier。



修改模型的attr，无其他效果

| <font style="color:#74B602;">AttributeMod_MAP_ObjectsFrozen</font> | <font style="color:#74B602;">单位变成冷冻模型，验证有效</font> |
| --- | --- |
| AttributeMod_MAP_FormationPreview | 单位变成 队列模型 |
| AttributeMod_MAP_ObjectsIronCurtain | 单位变成 铁幕模型 |
| AttributeMod_MAP_ObjectsTeslaSingularity | 单位变成 磁力奇点模型 |


下面是一系列attr，覆盖所有种类修改（护甲、速度、视野、单位体积，建造速度等等），分为增加和减少（Up和Down），增加是110%, 减少是90%，都可以叠99层（给同一个单位设置多次AttributeModifier）

AttributeModifier_MAP_ARMOR_Up

**注意，为减弱护甲**

AttributeModifier_MAP_ARMOR_Down

**注意，为加强护甲**

<font style="color:#DF2A3F;">AttributeModifier_MAP_DAMAGE_ADD_Up</font>

<font style="color:#DF2A3F;">不可用，未见伤害增加</font>

AttributeModifier_MAP_DAMAGE_ADD_Down

<font style="color:#74B602;">AttributeModifier_MAP_DAMAGE_MULT_Up</font>

<font style="color:#74B602;">可用，按百分比增加伤害。例如叠50次伤害就增加500%</font>

AttributeModifier_MAP_DAMAGE_MULT_Down

AttributeModifier_MAP_RESIST_FEAR_Up

AttributeModifier_MAP_RESIST_FEAR_Down

AttributeModifier_MAP_RESIST_TERROR_Up

AttributeModifier_MAP_RESIST_TERROR_Down

AttributeModifier_MAP_EXPERIENCE_Up

AttributeModifier_MAP_EXPERIENCE_Down

<font style="color:#ECAA04;">AttributeModifier_MAP_RANGE_Up</font>

<font style="color:#ECAA04;">注意，AttributeModifier_MAP_RANGE_Up需要结合AttributeModifier_MAP_VISION_Up， 同步增加相同的百分比。</font>

<font style="color:#ECAA04;">AttributeModifier_MAP_RANGE_Up修改的只是单位的射程，射程的增加必须配合“索敌视野”的同步增加才能生效（AttributeModifier_MAP_VISION_Up）。即如果只使用AttributeModifier_MAP_RANGE_Up增加射程，那么单位就会在进入射程时停下, 但因为“看不见”目标，导致无法攻击。</font>

AttributeModifier_MAP_RANGE_Down

<font style="color:#74B602;">AttributeModifier_MAP_SPEED_Up</font>

<font style="color:#74B602;">可用，加快最大移动速度</font>

AttributeModifier_MAP_SPEED_Down

AttributeModifier_MAP_CRUSH_DECELERATE_Up

AttributeModifier_MAP_CRUSH_DECELERATE_Down

AttributeModifier_MAP_RESIST_KNOCKBACK_Up

AttributeModifier_MAP_RESIST_KNOCKBACK_Down

AttributeModifier_MAP_SPELL_DAMAGE_Up

AttributeModifier_MAP_SPELL_DAMAGE_Down

AttributeModifier_MAP_RECHARGE_TIME_Up 充能时间未见明显变化?

AttributeModifier_MAP_RECHARGE_TIME_Down

<font style="color:#74B602;">AttributeModifier_MAP_PRODUCTION_Up</font>

<font style="color:#74B602;">可用，生产速度加快, (UI不变)</font>

AttributeModifier_MAP_PRODUCTION_Down

<font style="color:#2F4BDA;">AttributeModifier_MAP_PRODUCTION_COST_Up  可用, 建造费用 -10%  (ui上不会改变) </font>

<font style="color:#2F4BDA;">AttributeModifier_MAP_PRODUCTION_COST_Down 可用, 建造费用 +10%  (ui上不会改变) </font>

<font style="color:#2F4BDA;">AttributeModifier_MAP_HEALTH_Up  可用, 生命值增加1点</font>

AttributeModifier_MAP_HEALTH_Down

<font style="color:#2F4BDA;">AttributeModifier_MAP_HEALTH_MULT_Up  可用,生命值增加10%</font>

AttributeModifier_MAP_HEALTH_MULT_Down

<font style="color:#ECAA04;">AttributeModifier_MAP_VISION_Up</font>

<font style="color:#ECAA04;">注意VISION的增加会同步增大单位的警戒/自动攻击范围</font>

AttributeModifier_MAP_VISION_Down

AttributeModifier_MAP_BOUNTY_PERCENTAGE_Up

AttributeModifier_MAP_BOUNTY_PERCENTAGE_Down

AttributeModifier_MAP_MIN_CRUSH_VELOCITY_PERCENTAGE_Up

AttributeModifier_MAP_MIN_CRUSH_VELOCITY_PERCENTAGE_Down

AttributeModifier_MAP_AUTO_HEAL_Up

AttributeModifier_MAP_AUTO_HEAL_Down

AttributeModifier_MAP_SHROUD_CLEARING_Up

AttributeModifier_MAP_SHROUD_CLEARING_Down

<font style="color:#74B602;">AttributeModifier_MAP_RATE_OF_FIRE_Up</font>

<font style="color:#74B602;">可用，加快射速</font>

AttributeModifier_MAP_RATE_OF_FIRE_Down

AttributeModifier_MAP_DAMAGE_STRUCTURE_BOUNTY_ADD_Up

AttributeModifier_MAP_DAMAGE_STRUCTURE_BOUNTY_ADD_Down

AttributeModifier_MAP_CRUSHER_LEVEL_Up

AttributeModifier_MAP_CRUSHER_LEVEL_Down

AttributeModifier_MAP_COMMAND_POINT_BONUS_Up

AttributeModifier_MAP_COMMAND_POINT_BONUS_Down

AttributeModifier_MAP_CRUSHABLE_LEVEL_Up

AttributeModifier_MAP_CRUSHABLE_LEVEL_Down

AttributeModifier_MAP_CRUSHED_DECELERATE_Up

AttributeModifier_MAP_CRUSHED_DECELERATE_Down

AttributeModifier_MAP_INVULNERABLE_Up

AttributeModifier_MAP_INVULNERABLE_Down

AttributeModifier_MAP_SUPPRESSABILITY_Up

AttributeModifier_MAP_SUPPRESSABILITY_Down

AttributeModifier_MAP_RESIST_EMP_Up

AttributeModifier_MAP_RESIST_EMP_Down

AttributeModifier_MAP_POWER_BOOST_Up 和电力貌似无关?

AttributeModifier_MAP_POWER_BOOST_Down

AttributeModifier_MAP_AREA_OF_EFFECT_Up

AttributeModifier_MAP_AREA_OF_EFFECT_Down

AttributeModifier_MAP_COLLISION_GEOMETRY_SIZE_MULT_Up

<font style="color:#ECAA04;">AttributeModifier_MAP_COLLISION_GEOMETRY_SIZE_MULT_Down</font>

<font style="color:#ECAA04;">无法用来降低大船的碰撞体积以优化移动寻路，但会改变大船的移动行为（变得像小船一样）</font>

AttributeModifier_MAP_BROADCAST_RANGE_Up

AttributeModifier_MAP_BROADCAST_RANGE_Down



#
