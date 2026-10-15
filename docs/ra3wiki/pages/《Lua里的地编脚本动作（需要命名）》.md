---
title: "《Lua里的地编脚本动作（需要命名）》"
catalog_path: "Lua API "
slug: ao2x6koxgn4lewpg
source: https://www.yuque.com/muzeqaq/ra3mapwiki/ao2x6koxgn4lewpg
fetched: 2026-10-15
content_updated_at: "2024-11-22T22:48:20.000Z"
type: Doc
format: lake
---

# 《Lua里的地编脚本动作（需要命名）》

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/ao2x6koxgn4lewpg
> 目录位置：Lua API 

:::info
作者：岚依公主

更新者：Inertia

更新时间：2024年11月22日

:::

以下地编脚本可在 Lua 里通过 ExecuteAction 调用。

这些脚本必须使用字符串名称（而不是 Lua table）来代表单位。

单位的字符串名称可以在地图编辑器的单位属性面板里直接设置，也可以用 ExecuteAction SET_UNIT_REFERENCE 脚本为一个 lua table 添加一个字符串引用。



（一般人不需要关心的小细节：字符串名称和字符串引用实际上会被游戏转换为 FNV-1 哈希，因此有极小概率出现哈希碰撞导致一些问题，但实际上应该不太可能发生）



std::vector<WBScriptItem>

{



{

    .index = "45",

    .id = "NAMED_ENTER_NAMED",

    .name = R"#(Unused or Obsolete/Object Type/Transport/Transport -- load unit into specific.[45])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} loads into ${UnitName})#",

},

{

    .index = "46",

    .id = "TEAM_ENTER_NAMED",

    .name = R"#(Unused or Obsolete/Team/Transport/Transport -- load team into specific.[46])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} attempt to load into ${UnitName})#",

},

{

    .index = "91",

    .id = "EXIT_SPECIFIC_BUILDING",

    .name = R"#(Object Type/Garrison/Empty a named object.[91])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} empties.)#",

},

{

    .index = "94",

    .id = "NAMED_GARRISON_SPECIFIC_BUILDING",

    .name = R"#(Object Type/Garrison/Garrison a named object with a named object.[94])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} garrison ${UnitName})#",

:::info
注意该函数中被进驻的单位或建筑必须使用字符串命名参数（UnitTable无效），进驻者（例如步兵）可以使用字符串参数或者UnitTable。

:::

},

{

    .index = "161",

    .id = "NAMED_USE_COMMANDBUTTON_ABILITY_ON_NAMED",

    .name = R"#(Object Type/CommandButton/Use commandbutton ability on a specific Unit.[161])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} use ${UnitAbilityName} on ${UnitName}.)#",

},

{

    .index = "171",

    .id = "WAREHOUSE_SET_VALUE",

    .name = R"#(Unused or Obsolete/Map/Gameplay/Supply Source -- set cash value.[171])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Warehouse named ${UnitName} is set to having ${Integer} dollars worth of boxes. )#",

},

{

    .index = "190",

    .id = "TEAM_USE_COMMANDBUTTON_ABILITY_ON_NAMED",

    .name = R"#(Team/CommandButton/Use commandbutton ability on a named object.[190])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} use ${TeamAbilityName} on ${UnitName}.)#",

},

{

    .index = "230",

    .id = "NAMED_FACE_NAMED",

    .name = R"#(Object Type/Move/Face another unit.[230])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} begin facing ${UnitName})#",

},

{

    .index = "232",

    .id = "TEAM_FACE_NAMED",

    .name = R"#(Team/Move/Face another unit.[232])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} begin facing ${UnitName})#",

},

{

    .index = "337",

    .id = "CREATE_REINFORCEMENT_TEAM_AT_UNIT_POSITION",

    .name = R"#(Team/Spawn, Build, or Merge/Spawn a reinforcement team at named object's position.[337])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Spawn an instance of ${TeamName} at ${UnitName}.)#",

},

{

    .index = "362",

    .id = "NAME_SET_AUTO_ABILITY",

    .name = R"#(Unused or Obsolete/Object Type/CommandButton/Team set CommandButton auto ability to on or off.[362])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Have Team ${UnitName} set auto ability ${CommandButtonName} to ${Boolean})#",

},

{

    .index = "388",

    .id = "UNIT_ENABLE_HOUSE_COLOR",

    .name = R"#(Object Type/Settings/Enables or disables the house color[388])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Enable house color for ${UnitName} to ${Boolean} .)#",

},

{

    .index = "428",

    .id = "NAMED_GARRISON_SPECIFIC_BUILDING_INSTANTLY",

    .name = R"#(Object Type/Garrison/Garrison a named object instantly with a named object.[428])#",

    .help = R"#(Note that this Script will not do any proper checks to make sure the objects container is valid for the unit entering it.  If you add a unit that is not valid for the container the behavior thereafter is undefined and could possibly crash.)#",

    .argHint = R"#( ${UnitName} garrison's ${UnitName} instantly.)#",

:::info
注意该函数中被进驻的单位或建筑必须使用字符串命名参数（UnitTable无效），进驻者（例如步兵）可以使用字符串参数或者UnitTable。

:::

},

{

    .index = "429",

    .id = "TEAM_GARRISON_SPECIFIC_BUILDING_INSTANTLY",

    .name = R"#(Team/Garrison/Garrison a named object instantly with a team.[429])#",

    .help = R"#(Note that this Script will not do any proper checks to make sure the objects container is valid for the unit entering it.  If you add a unit that is not valid for the container the behavior thereafter is undefined and could possibly crash.)#",

    .argHint = R"#( ${TeamName} garrison's ${UnitName} instantly.)#",

},

{

    .index = "451",

    .id = "UNIT_AI_TRANSFER",

    .name = R"#(SkirmishAI/Transfer control of Unit to or from Skirmish AI.[451])#",

    .help = R"#(Ensure you have needed components of Skirmish AI turned on.)#",

    .argHint = R"#(Turn Skirmish AI for ${UnitName} ${NewBoolean})#",

},

{

    .index = "461",

    .id = "PLAYER_ALLOW_AI_AUTOACQUIRE",

    .name = R"#(SkirmishAI/Allow AI Auto-Acquire for Player.[461])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Use AI AutoAcquire for ${PlayerName}: ${Boolean})#",

},

{

    .index = "486",

    .id = "UNIT_SET_HEALTH",

    .name = R"#(Object Type/Damage or Remove/Set the health of the unit (in percentage).[486])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set health of ${UnitName} to percentage (0.0 - 100.0) ${RealNumber})#",

},

{

    .index = "527",

    .id = "UNIT_SPAWN_NAMED_OBJECT_ON_TEAM_AT_NAMED_OBJECT_LOCATION",

    .name = R"#(Object Type/Spawn/Spawn -- named object on Team at a named object's position.[527])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Spawn ${UnitName} of type ${ObjectName} on ${TeamName} at named object ${UnitName} .)#",

},

{

    .index = "551",

    .id = "NAMED_SET_MAX_HEALTH",

    .name = R"#(Object Type/Settings/Set the max health of a unit.[551])#",

    .help = R"#(Sets the maximum health of a unit to the value specified and optionally sets the units current health to this value as well. (Note if setting the max health lower than the current health the current health will automatically be set to max.)#",

    .argHint = R"#(Set health of ${UnitName} to ${Integer} set current health to max ${Boolean})#",

},



};
