---
title: "组合式AttributeModifier清单"
catalog_path: "Lua API "
slug: wpz5fwgb181qgy4r
source: https://www.yuque.com/muzeqaq/ra3mapwiki/wpz5fwgb181qgy4r
fetched: 2026-10-15
content_updated_at: "2026-04-27T13:32:06.000Z"
type: Sheet
format: lakesheet
---

# 组合式AttributeModifier清单

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/wpz5fwgb181qgy4r
> 目录位置：Lua API 
> 说明：表格页（lakesheet），共 1 个工作表；原始表格数据见 pages/组合式AttributeModifier清单.sheet.json

## Sheet1

| BUFF |  | 相关说明 |
| --- | --- | --- |
| 必看：①有些BUFF并未归入其中。 | ②使用BUFF时，最好手动添加相应时间，如600帧，30帧。 | ③相同名称BUFF不能叠加，但是不同名称BUFF的同一种效果能叠加，如3个不同加速BUFF可以叠加。 |
| 缩小 | AttributeMod_AlliedSupportAircraftShrinkRay |  |
| 单位外表类 | AttributeMod_ChronoRiftEffect | 时空裂隙特效，可开火，可被攻击，可移动，无其他效果 |
|  | AttributeMod_ChronoRiftEffect_Contained | 超时空裂隙效果，本体无法开火，可自由移动，不可被其他单位选中攻击，不会被AOE武器造成伤害 |
|  | AttributeMod_ChronoRiftEffect_NoFX | 貌似只有超时空裂隙特效，可开火，可被攻击，可移动 |
|  | AttributeMod_HyperSpaceMove | 先锋开F的特效，可攻击，可自由移动，也可被攻击，貌似其他属性变化(不算副作用的副作用：有先锋开F的音效） |
|  | AttributeMod_lightningTroop_Lv1 | 神州的雷铸天兵协议，相应单位属性的改变请在下方寻找(还挺酷炫的) |
|  | AttributeMod_lightningTroop_Lv2 | 同上 |
|  | AttributeMod_lightningTroop_Lv3 | 同上 |
|  | AttributeModifer_CryoBeam_10 | 冷冻特效,阵营色+1 |
| 定身 | AttributeMod_TankdestroyerParalyzeWeapon |  |
|  | AttributeMod_CelestialNukeVehicleLaunchSelfParalyze | 类似束缚状态，什么都干不了 |
|  | AttributeMod_FrozenrayAttachDrone | 被冻住的效果(无视觉特效）[但只是单位护甲降低，并不会被秒杀] |
|  | AttributeModifer_MovieSpecialAnimationBuff | 定在原地，能开火或转向 |
|  | AttributeModifer_CelestialShield | 定在原地，某些单位不能开火，或者需要开F技能使其能开火，会启用单位模型状态中的USER_6 |
|  | AttributeModifier_AttributeModifier_PrismAA | 定在原地，能开火或转向 |
|  | AttributeModifier_CelestialRepairDroneSeedImmobile |  |
| 铁幕 | AttributeModifier_INVULNERABLE | 无铁幕的特效 |
|  | AttributeModifier_IronCurtain | 有铁幕的特效 |
| 隐藏单位血条类(指Shift+空格也看不见） | AttributeMod_ContainedTurretREALLYDAMAGED | 无单位属性上的修改，对敌我双方都有效，血条都看不见（或许某些地图作者能用上此BUFF） |
|  | AttributeModifier_AlliedCommandoDetonationFX_DragonShip |  |
| 易伤效果 | AttributeMod_TargetPainted_HighTechnology |  |
| 打不准的导弹 | AttributeMod_StopMissileGuiding | AttributeModifier_SmokeBomb |
| 未测试效果 | AttributeMod_StopSpecialPower | AttributeMod_StopAttacking |
|  | AttributeMod_StopSpecialPower02 |  |
| 未知BUFF | AttributeMod_ZhuRongWeaponCool | 壹:该BUFF能使得某些单位切换武器，比如该BUFF加在幻影上能使得幻影普攻变为技能武器的攻击(射程也与技能武器射程相同）[某些单位会无法攻击] |
|  | AttributeMod_ZhuRongWeaponCool_Upgrade | 大致与壹一致，速度更快一点，伤害没测，基本没其他效果了 |
|  | AttributeModifer_CelestialMechaRush | 大致与壹一致，速度更更快一点，伤害没测，基本没其他效果了 |
|  | AttributeModifer_CelestialSpeedUpdate | 会启用单位模型状态的USER_1，貌似只能给神州特定单位。 |
|  | AttributeModifier_AlliedCommandoDetonationWeaponOwner_DragonShip | 貌似同壹 |
|  | AttributeModifier_CelestialPowerParalyze_Individual_Special | 单位无法开火，可以移动 |
|  | AttributeModifier_DefenseEffect_0 | 单位无法开火，可以移动，无法被其他单位攻击，但是能吃AOE伤害 |
|  | AttributeModifier_EndRising | 变成装饰品了233 |
|  | AttributeModifier_JANSTier3CombatMode | 有速度加成，会切换武器(类似壹) |
|  | AttributeModifier_JapanAntiVehicleVehicleTech1EnergizedArmor | 有速度加成，会切换武器(类似壹)，有护甲加成，会启用USER_6 |
| 免疫负面效果的BUFF | AttributeModifer_JapanMechaRushAntiDebuff | 只测了一部分(能免疫天使F,水翼F等等），应该跟鬼王免控那些差不多 |
|  | AttributeModifer_VacuumBombAntiDebuff | 貌似免疫的效果跟上面一样，不能免疫内爆弹 |
| ※潜艇上浮(非常有用) | AttributeMod_Submarine_Float |  |
| 电磁炮 | AttributeModifer_CelestialEMG | 貌似只能给神州指定的单位 |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
| 箱子特效（小琪） | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
|  | {"class": "image", "src": "https://cdn.nlark.com/yuque/0/2024/png/50038568/1734391612820-68bddd10-3a36-40ba-ba05-2cf072de30ba.png"} |  |
| 探测距离变近 | AttributeModifier_Blind | 即使全图视野，射程也是减小状态 |
|  | AttributeModifier_CelestialSmokeBombDebuff | 即使全图视野，射程也是减小状态 |
|  | AttributeModifier_DefenderTankSmokeEffect | 即使全图视野，射程也只有贴脸 |
|  | AttributeModifier_DefenderTankSmokeEffect |  |
|  | AttributeModifier_DefenderTankSmokeEffect |  |
|  | AttributeModifier_DefenderTankSmokeEffect |  |
|  | AttributeModifier_DefenderTankSmokeEffect |  |
| 探测潜艇 | AttributeMod_SubmarineDetection | 该BUFF能使得潜水的潜艇能在小地图显示(强制加在隐身的幻影身上没效果2333） |
|  | AttributeMod_SubmarineDetection |  |
| 萎了 | AttributeModifier_CelestialLowPowerDebuff | DEBUFF |
|  | AttributeModifier_CelestialLowPowerDebuff |  |
| ※加速BUFF大类 | AttributeModifier_JapanBattleShipShield | 将军盾，会开启单位的USER_1 |
| 1.25 | AttributeModifier_JapanAntiVehicleVehicleTech1EnergizedArmor | 极不推荐，慎用！！！副作用:单位无法开火，且会启动单位模型状态中的USER_6 |
| 1.5 | AttributeModifier_JapanEngineerSprint | 也不算副作用的副作用:启用某些单位的模型状态中的USER_1 |
| 2 | AttributeMod_AlliedSupportAircraftShrinkRay | 缩小（副作用多），不推荐 |
| 2 | AttributeMod_AlliedSupportAircraftShrinkRay_HighTechnology | 也是缩小（副作用多），不推荐 |
| 1.5 | AttributeModifier_JapanAntiInfantryInfantryBonzai | 也不算副作用的副作用:启用某些单位的模型状态中的USER_1 |
| 2 | AttributeModifier_JapanAntiVehicleVehicleTech3RushAttack | 单纯的加速 |
| 2 | AttributeModifier_JapanAntiStructureShipRammingSpeed | 单纯的加速 |
| 1.25 | AttributeModifer_JapanNavalPower_JapanAntiVehicleShip | 有护甲加成 |
| 1.25 | AttributeModifer_JapanNavalPower_JapanAntiStructureShip | 有护甲加成 |
| 1.25 | AttributeModifer_JapanNavalPower_JapanAntiAirShip | 有护甲加成 |
| 1.25 | AttributeModifer_JapanNavalPower_JapanNavyScoutShip | 有护甲加成 |
| 1.25 | AttributeModifer_JapanNavalPower_JapanFortressShip | 有护甲加成 |
| 1.25 | AttributeModifer_EmperorsStatue | 射速加成200％，伤害貌似会降低一点？ |
| 1.25 | AttributeModifier_ReturnToBase | 飞机回机场的加速BUFF,无任何负面效果 |
|  | AttributeModifier_BoxSpeedSuperUp | 不清楚加成多少，反正灰常多，无负面 |
|  | AttributeModifier_BoxSpeedUp | 不清楚加成多少，反正还好，无负面 |
| 1.5 | AttributeModifier_KamikazeAttack | 慎用！！无负面，但是模型状态会切换到USER_5(某些单位会出现无法开火)[但是这些单位的某一部分能通过开启F技能使其正常化] |
|                        目测至少300% | AttributeMod_CelestialCoreBOSSAIUnitCheat | 貌似只有速度上的改变 |
|                      目测大约有200% | AttributeMod_CelestialEggRange1 | 貌似只有速度上的改变 |
|  | AttributeMod_CelestialEggRange2 | 与上面加成相同且能一起叠加 |
|                            大概110% | AttributeMod_CenturionUpgradeLv1 | 攻击造成伤害变为1.1倍 |
|                            大概125% | AttributeMod_CenturionUpgradeLv2 | 攻击造成伤害变为1.25倍 |
|                            大概125% | AttributeMod_CenturionUpgradeLv3 | 攻击造成伤害变为1.25倍 [没错与Lv2几乎一样] |
|                            大概125% | AttributeMod_GenericBOSSAIUnitCheat | 攻击造成伤害变为3倍，生命值变为1.2倍，射程提升一点 |
|                            大概135% | AttributeMod_GenericSpeedSet | 貌似只加成速度 |
|                             | AttributeMod_GenericSpeedSet1000 | 非常变态，基佬夫能开启跃迁引擎，幻影能类瞬移到目标位置 |
|  | AttributeMod_GenericSpeedSet200 |  |
|  | AttributeMod_GenericSpeedSet150 |  |
|  | AttributeMod_GenericSpeedSet400 |  |
| 2 | AttributeMod_JapanEggBOSSAIUnitCheat |  |
| 1.15 | AttributeMod_JapanMechaRushUpgrade | 这个就不用说了，帝国高科的升级，不过用在其他阵营没有使受到的伤害降低的效果 |
| 1.5 | AttributeMod_JapanShipBOSSAIUnitCheat | 貌似射程提升了1/3 |
| 1.25 | AttributeMod_lightningTroop_Lv1 | 攻击造成伤害变为1.25倍 |
| 1.25 | AttributeMod_lightningTroop_Lv2 | 攻击造成伤害变为1.5倍 |
| 1.25 | AttributeMod_lightningTroop_Lv3 | 攻击造成伤害变为1.75倍 |
| 2 | AttributeMod_LongYanWeaponBOSSAICheat | 射速至少为200％ |
| 1.25 | AttributeMod_RhinoTankBOSSAIUnitCheat | 生命值变为1.5倍 |
| 1.5 | AttributeMod_TanyaBOSSAIUnitCheat | 生命值变为1.75倍 |
| 1.5 | AttributeMod_TsunamiTankBOSSAIUnitCheat | 除速度加成的效果外，其他效果会被星级效果覆盖。射程略提升，速度变为150%,1.14倍左右的伤害 |
| 2 | AttributeModifier_Buff1 | 貌似只有加速效果 |
|  | AttributeModifier_QuickMove | 会启用USER_6 |
|  | AttributeModifier_SovietGrinderTriggerNitrousBoost | 会启用USER_6 |
|  | AttributeModifier_SovietGrinderTriggerNitrousBoost |  |
|  | AttributeModifier_SovietGrinderTriggerNitrousBoost |  |
|  | AttributeModifier_SovietGrinderTriggerNitrousBoost |  |
|  | AttributeModifier_SovietGrinderTriggerNitrousBoost |  |
|  | AttributeModifier_SovietGrinderTriggerNitrousBoost |  |
| ※减速BUFF大类(非静止） | AttributeModifier_BoxSpeedSuperDown | 不清楚加成多少 |
|  | AttributeModifier_BoxSpeedDown | 不清楚加成多少 |
| 0.5 | AttributeModifier_RiotShield | 会启用模型状态USER_6和部分单位F技能特效(日冕3.202用幻影测试这BUFF能启用原版的幻影F技能的特效？） |
| 0.5 | AttributeModifier_ArmorUp | 因其有护甲加成所以同种单位下，该BUFF比白板单位能抗4倍的揍 |
| 0.5 | AttributeMod_JapanScoutInfantrySlowAttached | 蜻蜓附着的减速debuff，仅有减速效果 |
|  | AttributeMod_ATConeDieSpeedReduced3s | 龟速行驶，接近静止 |
| 未知值 | AttributeMod_AthenaWeaponBOSSAICheat | 减速效果，其他效果未知 |
| 未知值 | AttributeMod_EMSuppressDebuff_Lv1 | 未知其他效果 |
| 未知值 | AttributeMod_EMSuppressDebuff_Lv2 | 未知其他效果 |
| 未知值 | AttributeMod_EMSuppressDebuff_Lv3 | 未知其他效果 |
| 未知值 | AttributeMod_CelestialChainGunBOSSAICheat | 护甲也会降低 |
| 未知值 | AttributeMod_FrozenRayDroneSlowDown | 貌似只降低速度 |
| 未知值 | AttributeMod_GenericShipDecelerate | 貌似只降低速度 |
|  | AttributeMod_GenericSpeedSet2 | 龟速 |
|  | AttributeMod_GenericSpeedSet10 | 龟速 |
|  | AttributeMod_GenericSpeedSet50 | 龟速 |
|  | AttributeMod_HardSpeedReduced2s | 龟速 |
|  | AttributeMod_JapanCommandoSpeedReduce | 龟速 |
|  | AttributeMod_JapanCommandoSpeedReduce02 | 与楼上相比，不那么龟速一点 |
| 未知值 | AttributeMod_JapanAntiAirShipUnderWaterAttackingSpeedReduce | 反正不是龟速 |
| 未知值 | AttributeMod_SovietAdvancedTankTech4TeslaGun_BOSSAI | 与原版盟军战役---低调的天启坦克---速度差不多 |
| 未知值 | AttributeMod_SovietAdvancedTankTech4TeslaGun_SpeedDebuff | 比楼上快一点 |
|  | AttributeMod_SpeedReduced | 护甲会降低 |
|  | AttributeMod_SpeedReduced_01 | 护甲会降低 |
|  | AttributeMod_SpeedReduced_02 | 护甲会降低 |
|  | AttributeMod_SpeedReduced5s | 护甲不变，但是龟速 |
|  | AttributeMod_SpeedReducedLong | 护甲不变 |
|  | AttributeMod_SpeedReducedShort | 护甲不变 |
|  | AttributeMod_SUMineSpeedReduce | 貌似就只有减速 |
|  | AttributeModifier_CelestialPowerParalyze_Individual |  |
|  | AttributeModifier_CelestialZhuRongSpeedReduce |  |
|  | AttributeModifier_InfantryVehicle_Slower |  |
|  | AttributeModifier_SentinelVehicleSpeedReduce |  |
|  | AttributeModifier_SentinelVehicleSpeedReduce |  |
|  | AttributeModifier_SentinelVehicleSpeedReduce |  |
| ※射速BUFF类(正面)            | 说明：射速BUFF能加快武器的开火速度，不能加速某些单位武器充能速度，因此推荐与设置单位弹药的lua脚本和物体状态OVERCHARGING_WEAPON配合使用以达到最好效果。 |  |
| 3 | AttributeModifier_RapidLaunch | 无畏的F效果，该BUFF适用于所有可开火单位 |
|  | AttributeModifier_DreadnoughtRapidLaunch | 楼上BUFF的青春版 |
| 1.25 | AttributeModifer_JapanEmperorsResolve_L1 | 速度变为75％ |
| 1.5 | AttributeModifer_JapanEmperorsResolve_L2 | 速度变为50％ |
| 1.75 | AttributeModifer_JapanEmperorsResolve_L3 | 速度变为25％ |
| 2 | AttributeModifer_EmperorsStatue | 副作用：单位造成的伤害变为75％ |
| 未知值 | AttributeModifier_BoxRateOfFireUp |  |
| ※※※※※非常恐怖EX | AttributeMod_Buff_Grappling | ※警告：此BUFF自带武器充能效果，该buff类似原版超频，非常变态，并且多数单位只能攻击一次，打完就萎了，建议配合设置弹药类lua脚本使用，非常变态变态变态 |
|                            至少200% | AttributeMod_LongYanWeaponBOSSAICheat | 速度变为200% |
|                            至少200% | AttributeModifier_CelestialInterceptorRapidFire |  |
|                            至少150% | AttributeModifier_CelestialZhuRongRapidFire | 射程会增加一部分 |
|  | AttributeModifier_CelestialZhuRongRapidFire |  |
| ※伤害加成类（正面） | AttributeModifier_CelestialZhuRongRapidFire |  |
| 10倍伤害 | AttributeModifier_BoxAttackSuperUp |  |
| 1.5倍伤害 | AttributeMod_AlliedPrismAttackBoost |  |
| 2倍伤害 | AttributeMod_AlliedTeslaBoost |  |
| 1.2倍伤害 | HardAISinglePlayer_Bonus |  |
| 1.2倍伤害 | Unit_Veteran | 如果单位等级大于等于2级，那么该BUFF作废 |
| 1.5倍伤害 | AttributeMod_DestroyerBOSSAICheat |  |
| 1.1倍伤害 | AttributeMod_CenturionUpgradeLv1 | 会提高单位一点点速度(大概110％) |
| 1.25倍伤害 | AttributeMod_CenturionUpgradeLv2 | 会提高单位一点速度(大概125％) |
| 1.25倍伤害 | AttributeMod_CenturionUpgradeLv3 | 会提高单位一点速度(大概125％) [没错与Lv2几乎一样] |
| 3倍伤害 | AttributeMod_GenericBOSSAIUnitCheat | 速度大概变为125%，生命值变为1.2倍，射程提升一点 |
| 1.25倍伤害 | AttributeMod_lightningTroop_Lv1 | 速度变为125% |
| 1.5倍伤害 | AttributeMod_lightningTroop_Lv2 | 速度变为125% |
| 1.75倍伤害 | AttributeMod_lightningTroop_Lv3 | 速度变为125% |
| 1.14倍左右的伤害 | AttributeMod_TsunamiTankBOSSAIUnitCheat | 除速度加成的效果外，其他效果会被星级效果覆盖，也就是此BUFF作废。射程略提升，速度变为150% |
| 1.14倍左右的伤害 | AttributeModifer_CelestialTechRANK1 | 1.14倍左右伤害，会启用单位模型状态USER_1，射程增加了一点，生命值1.25倍(星级状态下，除了生命值上限效果，其他效果会被代替） |
| 2倍伤害 | AttributeModifier_BoxDamageMultUp |  |
| 2倍伤害 | AttributeModifier_Buff3 |  |
| 1.35倍伤害 | AttributeModifier_CelestialAAVehicleAttackMoveEffect |  |
| 1.14倍左右的伤害 | AttributeModifier_CelestialPowerExamine | 射程增加了一点（星级状态下，该BUFF所有效果被替代） |
|  | AttributeModifier_CelestialPowerExamine |  |
|  | AttributeModifier_CelestialPowerExamine |  |
|  | AttributeModifier_CelestialPowerExamine |  |
|  | AttributeModifier_CelestialPowerExamine |  |
| ※生命值上限大类 | AttributeModifier_CelestialPowerExamine |  |
| 1.25倍 | AttributeMod_CenturionUpgradeLeaderLv1 | 此BUFF及下面2个可一起叠加(貌似对单位其他属性无改变） |
| 1.25倍 | AttributeMod_CenturionUpgradeLeaderLv2 |  |
| 1.25倍 | AttributeMod_CenturionUpgradeLeaderLv3 |  |
| 1.2倍 | AttributeMod_GenericBOSSAIUnitCheat | 攻击造成伤害变为3倍，速度大概变为125%，射程提升一点 |
| 1.5倍 | AttributeMod_GunshipBOSSAIUnitCheat |  |
| 2倍 | AttributeMod_JapanEggBOSSAIUnitCheat | 护甲略微提升，速度变为200% |
| 1.5倍 | AttributeMod_RhinoTankBOSSAIUnitCheat | 速度变为125% |
| 2倍 | AttributeMod_SovietAntiGroundAttackerBOSSAIUnitCheat | 貌似没其他效果 |
| 1.75倍 | AttributeMod_TanyaBOSSAIUnitCheat | 速度变为150% |
| 1.5倍 | AttributeMod_TeslaTrooperBOSSAIUnitCheat | 貌似没其他效果 |
| 1.25倍 | AttributeModifer_CelestialTechRANK4 | 貌似没其他效果 |
| 1.25倍 | AttributeModifer_CelestialTechRANK3 | 貌似没其他效果 |
| 1.25倍 | AttributeModifer_CelestialTechRANK2 | 貌似没其他效果 |
| 1.25倍 | AttributeModifer_CelestialTechRANK1 | 1.14倍左右伤害，会启用单位模型状态USER_1，射程增加了一点，生命值1.25倍(星级状态下，除了生命值上限效果，其他效果会被代替） |
| 1.25倍 | AttributeModifer_JapanTech4_JapanBarracks | 貌似没其他效果 |
| 1.25倍 | AttributeModifer_JapanTech4_JapanNavalYard | 貌似没其他效果 |
| 1.25倍 | AttributeModifer_JapanTech4_JapanWarFactory | 貌似没其他效果 |
| 极大幅度下降 | AttributeModifier_BoxHealSuperDown |  |
| 1.25倍 | AttributeModifier_CelestialMissilePodUpgrade |  |
| 1.2倍 | AttributeModifier_CelestialTech1HealthBuff |  |
| 1.25倍 | AttributeModifier_CelestialTech2HealthBuff |  |
| 1.3倍 | AttributeModifier_CelestialTech3HealthBuff |  |
| 1.25倍 | AttributeModifier_CelestialTech4HealthBuff |  |
|  | AttributeModifier_CelestialTech4HealthBuff |  |
|  | AttributeModifier_CelestialTech4HealthBuff |  |
|  | AttributeModifier_CelestialTech4HealthBuff |  |
| ※射程大类（正面） | AttributeModifier_CelestialTech4HealthBuff |  |
|  | AttributeMod_GenericBOSSAIUnitCheat | 攻击造成伤害变为3倍，速度大概变为125%，生命值变为1.2倍 |
|  | AttributeMod_TsunamiTankBOSSAIUnitCheat | 除速度加成的效果外，其他效果会被星级效果覆盖，也就是此BUFF作废。射程略提升，速度变为150%,1.14倍左右的伤害 |
|  | AttributeModifer_CelestialTechRANK1 | 1.14倍左右伤害，会启用单位模型状态USER_1，射程增加了一点，生命值1.25倍(星级状态下，除了生命值上限效果，其他效果会被代替） |
|  | AttributeModifier_AlliedAntiGroundAircraftBombRangeUp | 射程加了一点 |
| 变态射程 | AttributeModifier_BoxRangeSuperUp | 此BUFF与下面BUFF叠加会让几乎所有攻城陆地单位或者海面单位的射程覆盖整张图 |
|  | AttributeModifier_BoxRangeUp |  |
|  | AttributeModifier_CelestialZhuRongRapidFire | 射程增加一点，有射速加成 |
|  | AttributeModifier_Garrisoned |  |
|  | AttributeModifier_Garrisoned_02 |  |
|  | AttributeModifier_Garrisoned_03 |  |
|  | AttributeModifier_Garrisoned_03 |  |
|  | AttributeModifier_Garrisoned_03 |  |
|  | AttributeModifier_Garrisoned_03 |  |
|  | AttributeModifier_Garrisoned_03 |  |
| ※射程大类（负面） | AttributeModifier_Garrisoned_03 |  |
|  | AttributeMod_HarbingerGunshipVision_GunMode | 武器射程减少，但是单位本身会移动到本身默认的射程的位置去攻击敌人（也就是需要手动靠近敌人才能开火）[补充：攻击地面不会出现这种情况] |
|  | AttributeMod_ZhuRongBOSSAICheat | 武器射程大幅度减少，护甲较多幅度减少 |
|  | AttributeMod_ZhuRongBOSSAICheat |  |
|  | AttributeMod_ZhuRongBOSSAICheat |  |
|  | AttributeMod_ZhuRongBOSSAICheat |  |
|  | AttributeMod_ZhuRongBOSSAICheat |  |
|  | AttributeMod_ZhuRongBOSSAICheat |  |
|  | AttributeMod_ZhuRongBOSSAICheat |  |
|  | AttributeMod_ZhuRongBOSSAICheat |  |
| ※护甲大类（正面） | AttributeMod_ZhuRongBOSSAICheat | 无特别说明基本都通用↓ |
|  | AttributeMod_SovietCompositeArmor |  |
|  | AttributeMod_SovietCompositeArmorForKirov |  |
|  | AttributeMod_SovietCompositeArmorForLargeUnit |  |
|  | AttributeModifier_BoxArmorUp |  |
|  | AttributeModifier_ArmorUp | 速度变为50%，会启用USER_6 |
|  | AttributeModifier_ArmorUp |  |
|  | AttributeModifier_ArmorUp |  |
|  | AttributeModifier_ArmorUp |  |
|  | AttributeModifier_ArmorUp |  |
|  | AttributeModifier_ArmorUp |  |
|  | AttributeModifier_ArmorUp |  |
|  | AttributeModifier_ArmorUp |  |
|  | AttributeModifier_ArmorUp |  |
|  | AttributeModifier_ArmorUp |  |
|  | AttributeModifier_ArmorUp |  |
|  | AttributeModifier_ArmorUp |  |
|  | AttributeModifier_ArmorUp |  |
|  | AttributeModifier_ArmorUp |  |
|  | AttributeModifier_ArmorUp |  |
|  | AttributeModifier_ArmorUp |  |
|  | AttributeModifier_ArmorUp |  |
| 子弹吸收 | AttributeModifier_MagneticArmor |  |
|  | AttributeModifier_MagneticArmor |  |
|  | AttributeModifier_MagneticArmor |  |
|  | AttributeModifier_MagneticArmor |  |
|  | AttributeModifier_MagneticArmor |  |
|  | AttributeModifier_MagneticArmor |  |
|  | AttributeModifier_MagneticArmor |  |
| ※护甲大类（负面）[懒得测了 | AttributeModifier_MagneticArmor |  |
|  | AttributeMod_SpeedReduced | 速度会降低 |
|  | AttributeMod_ArmorDecline |  |
|  | AttributeMod_TargetPainted_HighTechnology |  |
|  | AttributeMod_TargetPainted |  |
|  | AttributeMod_ZhuRongBOSSAICheat | 武器射程大幅度减少，护甲较多幅度减少 |
|  | AttributeModifier_BoxArmorDown |  |
|  | AttributeModifier_BoxArmorDown |  |
| 自定义buff | lua支持自定义buff, 请参考 https://www.yuque.com/muzeqaq/ra3mapwiki/chqogqlxe9pzb0k1 |  |
|  | lua支持自定义buff, 请参考 https://www.yuque.com/muzeqaq/ra3mapwiki/chqogqlxe9pzb0k1 |  |
| 单项buff | lua支持自定义buff, 请参考 https://www.yuque.com/muzeqaq/ra3mapwiki/chqogqlxe9pzb0k1 |  |
|  | AttributeMod_MAP_ObjectsFrozen | 单位变成冷冻模型，验证有效 |
|  | AttributeMod_MAP_FormationPreview | 单位变成 队列模型 |
|  | AttributeMod_MAP_ObjectsIronCurtain | 单位变成 铁幕模型 |
|  | AttributeMod_MAP_ObjectsTeslaSingularity | 单位变成 磁力奇点模型 |
|  | AttributeMod_MAP_ObjectsTeslaSingularity |  |
|  | AttributeModifier_MAP_ARMOR_Up | 注意，为减弱护甲 |
|  | AttributeModifier_MAP_ARMOR_Down | 注意，为加强护甲 |
|  | AttributeModifier_MAP_DAMAGE_ADD_Up | 不可用，未见伤害增加 |
|  | AttributeModifier_MAP_DAMAGE_ADD_Down |  |
|  | AttributeModifier_MAP_DAMAGE_MULT_Up | 可用，按百分比增加伤害。例如叠50次伤害就增加500% |
|  | AttributeModifier_MAP_DAMAGE_MULT_Down |  |
|  | AttributeModifier_MAP_RESIST_FEAR_Up |  |
|  | AttributeModifier_MAP_RESIST_FEAR_Down |  |
|  | AttributeModifier_MAP_RESIST_TERROR_Up |  |
|  | AttributeModifier_MAP_RESIST_TERROR_Down |  |
|  | AttributeModifier_MAP_EXPERIENCE_Up |  |
|  | AttributeModifier_MAP_EXPERIENCE_Down |  |
|  | AttributeModifier_MAP_RANGE_Up | 注意，AttributeModifier_MAP_RANGE_Up需要结合AttributeModifier_MAP_VISION_Up， 同步增加相同的百分比。AttributeModifier_MAP_RANGE_Up修改的只是单位的射程，射程的增加必须配合“索敌视野”的同步增加才能生效（AttributeModifier_MAP_VISION_Up）。即如果只使用AttributeModifier_MAP_RANGE_Up增加射程，那么单位就会在进入射程时停下, 但因为“看不见”目标，导致无法攻击。 |
|  | AttributeModifier_MAP_RANGE_Down |  |
|  | AttributeModifier_MAP_SPEED_Up | 可用，加快最大移动速度 |
|  | AttributeModifier_MAP_SPEED_Down |  |
|  | AttributeModifier_MAP_CRUSH_DECELERATE_Up |  |
|  | AttributeModifier_MAP_CRUSH_DECELERATE_Down |  |
|  | AttributeModifier_MAP_RESIST_KNOCKBACK_Up |  |
|  | AttributeModifier_MAP_RESIST_KNOCKBACK_Down |  |
|  | AttributeModifier_MAP_SPELL_DAMAGE_Up |  |
|  | AttributeModifier_MAP_SPELL_DAMAGE_Down |  |
|  | AttributeModifier_MAP_RECHARGE_TIME_Up | 充能时间未见明显变化? |
|  | AttributeModifier_MAP_RECHARGE_TIME_Down |  |
|  | AttributeModifier_MAP_PRODUCTION_Up | 可用，生产速度加快, (UI不变) |
|  | AttributeModifier_MAP_PRODUCTION_Down |  |
|  | AttributeModifier_MAP_PRODUCTION_COST_Up |  可用, 建造费用 -10%  (ui上不会改变)  |
|  | AttributeModifier_MAP_PRODUCTION_COST_Down | 可用, 建造费用 +10%  (ui上不会改变)  |
|  | AttributeModifier_MAP_HEALTH_Up | 可用, 生命值增加1点 |
|  | AttributeModifier_MAP_HEALTH_Down |  |
|  | AttributeModifier_MAP_HEALTH_MULT_Up | 可用,生命值增加10% |
|  | AttributeModifier_MAP_HEALTH_MULT_Down |  |
|  | AttributeModifier_MAP_VISION_Up | 注意VISION的增加会同步增大单位的警戒/自动攻击范围 |
|  | AttributeModifier_MAP_VISION_Down |  |
|  | AttributeModifier_MAP_BOUNTY_PERCENTAGE_Up |  |
|  | AttributeModifier_MAP_BOUNTY_PERCENTAGE_Down |  |
|  | AttributeModifier_MAP_MIN_CRUSH_VELOCITY_PERCENTAGE_Up |  |
|  | AttributeModifier_MAP_MIN_CRUSH_VELOCITY_PERCENTAGE_Down |  |
|  | AttributeModifier_MAP_AUTO_HEAL_Up |  |
|  | AttributeModifier_MAP_AUTO_HEAL_Down |  |
|  | AttributeModifier_MAP_SHROUD_CLEARING_Up |  |
|  | AttributeModifier_MAP_SHROUD_CLEARING_Down |  |
|  | AttributeModifier_MAP_RATE_OF_FIRE_Up | 可用，加快射速 |
|  | AttributeModifier_MAP_RATE_OF_FIRE_Down |  |
|  | AttributeModifier_MAP_DAMAGE_STRUCTURE_BOUNTY_ADD_Up |  |
|  | AttributeModifier_MAP_DAMAGE_STRUCTURE_BOUNTY_ADD_Down |  |
|  | AttributeModifier_MAP_CRUSHER_LEVEL_Up |  |
|  | AttributeModifier_MAP_CRUSHER_LEVEL_Down |  |
|  | AttributeModifier_MAP_COMMAND_POINT_BONUS_Up |  |
|  | AttributeModifier_MAP_COMMAND_POINT_BONUS_Down |  |
|  | AttributeModifier_MAP_CRUSHABLE_LEVEL_Up |  |
|  | AttributeModifier_MAP_CRUSHABLE_LEVEL_Down |  |
|  | AttributeModifier_MAP_CRUSHED_DECELERATE_Up |  |
|  | AttributeModifier_MAP_CRUSHED_DECELERATE_Down |  |
|  | AttributeModifier_MAP_INVULNERABLE_Up |  |
|  | AttributeModifier_MAP_INVULNERABLE_Down |  |
|  | AttributeModifier_MAP_SUPPRESSABILITY_Up |  |
|  | AttributeModifier_MAP_SUPPRESSABILITY_Down |  |
|  | AttributeModifier_MAP_RESIST_EMP_Up |  |
|  | AttributeModifier_MAP_RESIST_EMP_Down |  |
|  | AttributeModifier_MAP_POWER_BOOST_Up | 和电力貌似无关? |
|  | AttributeModifier_MAP_POWER_BOOST_Down |  |
|  | AttributeModifier_MAP_AREA_OF_EFFECT_Up |  |
|  | AttributeModifier_MAP_AREA_OF_EFFECT_Down |  |
|  | AttributeModifier_MAP_COLLISION_GEOMETRY_SIZE_MULT_Up |  |
|  | AttributeModifier_MAP_COLLISION_GEOMETRY_SIZE_MULT_Down | 无法用来降低大船的碰撞体积以优化移动寻路，但会改变大船的移动行为（变得像小船一样） |
|  | AttributeModifier_MAP_BROADCAST_RANGE_Up |  |
|  | AttributeModifier_MAP_BROADCAST_RANGE_Down |  |
