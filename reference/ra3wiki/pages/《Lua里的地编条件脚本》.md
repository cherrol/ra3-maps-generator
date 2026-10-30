---
title: "《Lua里的地编条件脚本》"
catalog_path: "Lua API "
slug: eaht4xli7pd51gzk
source: https://www.yuque.com/muzeqaq/ra3mapwiki/eaht4xli7pd51gzk
fetched: 2026-10-15
content_updated_at: "2024-12-20T04:19:21.000Z"
type: Doc
format: lake
---

# 《Lua里的地编条件脚本》

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/eaht4xli7pd51gzk
> 目录位置：Lua API 

以下地编脚本可在 Lua 里通过 EvaluateCondition 调用，并将返回布尔值。

例子：

```lua
--- @type StandardUnitType
local unit = GetObjectByScriptName("UNIT") --- 假设地图上存在名为UNIT的单位，存活状态
--- @type boolean
local bool = EvaluateCondition("NAMED_DESTROYED", unit) --- 布尔值为假，未被摧毁

--- 假设玩家1在之前任意时刻为1架金乌使用了1次切换挂载技能
local bool2 = EvaluateCondition("PLAYER_TRIGGERED_SPECIAL_POWER", "Player_1", "SpecialPower_CUBomberSwitchWeapon")
--- 布尔值为真，检测到使用了技能（同时内部检测计数器值-1）

```

std::vector<WBScriptItem>

{



{

    .index = "5",

    .id = "TEAM_INSIDE_AREA_PARTIALLY",

    .name = R"#(Team/Area or Path/Team has units in an area.[5])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} has one or more units in ${TriggerAreaName} (${SurfaceType}).)#",

},

{

    .index = "6",

    .id = "TEAM_DESTROYED",

    .name = R"#(Team/Attacked, Destroyed, or Exists/Team is destroyed.[6])#",

    .help = R"#(This condition is only true for the single logic frame when the team dies.)#",

    .argHint = R"#( ${TeamName} has been destroyed.)#",

},

{

    .index = "7",

    .id = "CAMERA_MOVEMENT_FINISHED",

    .name = R"#(Camera/Camera movement finished.[7])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(The camera movement has finished.)#",

},

{

    .index = "8",

    .id = "TEAM_HAS_UNITS",

    .name = R"#(Team/Attacked, Destroyed, or Exists/Team has units.[8])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} has one or more units.)#",

},

{

    .index = "9",

    .id = "NAMED_INSIDE_AREA",

    .name = R"#(Object Type/Area or Path/Named object is inside an area.[9])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} is inside ${TriggerAreaName})#",

},

{

    .index = "10",

    .id = "NAMED_OUTSIDE_AREA",

    .name = R"#(Object Type/Area or Path/Named object is outside an area.[10])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} is outside ${TriggerAreaName})#",

},

{

    .index = "11",

    .id = "NAMED_DESTROYED",

    .name = R"#(Object Type/Attacked, Destroyed, or Exists/Named object is destroyed.[11])#",

    .help = R"#(This will return true if the unit is dead or does not exist.  It will also return true if the unit doesn)#",

    .argHint = R"#( ${UnitName} has been destroyed.)#",

},

{

    .index = "12",

    .id = "NAMED_NOT_DESTROYED",

    .name = R"#(Object Type/Attacked, Destroyed, or Exists/Named object exists and is alive.[12])#",

    .help = R"#(This checks to see if the object exists on the level, and is not effectively dead.  This means it will fire if the unit is around and not dead, a unit in its slow death will cause this to return false.)#",

    .argHint = R"#( ${UnitName} exists and is alive.)#",

},

{

    .index = "13",

    .id = "TEAM_INSIDE_AREA_ENTIRELY",

    .name = R"#(Team/Area or Path/Team is completely inside an area.[13])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} is all inside ${TriggerAreaName} (${SurfaceType}).)#",

},

{

    .index = "14",

    .id = "TEAM_OUTSIDE_AREA_ENTIRELY",

    .name = R"#(Team/Area or Path/Team is completely outside an area.[14])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} is completely outside ${TriggerAreaName} (${SurfaceType}).)#",

},

{

    .index = "15",

    .id = "NAMED_ATTACKED_BY_OBJECTTYPE",

    .name = R"#(Object Type/Attacked, Destroyed, or Exists/Named object is attacked by a specific object type.[15])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} has been attacked by a(n) ${ObjectType})#",

},

{

    .index = "16",

    .id = "TEAM_ATTACKED_BY_OBJECTTYPE",

    .name = R"#(Team/Attacked, Destroyed, or Exists/Team is attacked by a specific object type.[16])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} has been attacked by a(n) ${ObjectType})#",

},

{

    .index = "17",

    .id = "NAMED_ATTACKED_BY_PLAYER",

    .name = R"#(Object Type/Attacked, Destroyed, or Exists/Named object has been attacked by Player.[17])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} has been attacked by ${PlayerName})#",

},

{

    .index = "18",

    .id = "TEAM_ATTACKED_BY_PLAYER",

    .name = R"#(Team/Attacked, Destroyed, or Exists/Team has been attacked by Player.[18])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} has been attacked by ${PlayerName})#",

},

{

    .index = "19",

    .id = "BUILT_BY_PLAYER",

    .name = R"#(Player/Owns/Player has built an object type.[19])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${ObjectType} has been built by ${PlayerName})#",

},

{

    .index = "20",

    .id = "NAMED_CREATED",

    .name = R"#(Unused or Obsolete/Unit has been created.[20])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} has been created.)#",

},

{

    .index = "21",

    .id = "TEAM_CREATED",

    .name = R"#(Team/Attacked, Destroyed, or Exists/Team has been created.[21])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} has been created.)#",

},

{

    .index = "22",

    .id = "PLAYER_HAS_CREDITS",

    .name = R"#(Player/Owns/Player has (comparison) number of credits.[22])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has ${Comparison} ${Integer} credits.)#",

},

{

    .index = "23",

    .id = "NAMED_DISCOVERED",

    .name = R"#(Player/Sighted/Player has sighted named object.[23])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has seen ${UnitName})#",

},

{

    .index = "24",

    .id = "TEAM_DISCOVERED",

    .name = R"#(Player/Sighted/Player has sighted Team.[24])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has seen ${TeamName}.)#",

},

{

    .index = "25",

    .id = "NAMED_OWNED_BY_PLAYER",

    .name = R"#(Player/Owns/Player owns Unit.[25])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} owns ${UnitName}.)#",

},

{

    .index = "26",

    .id = "TEAM_OWNED_BY_PLAYER",

    .name = R"#(Player/Owns/Player owns Team.[26])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} is owned by ${TeamName}.)#",

},

{

    .index = "27",

    .id = "PLAYER_HAS_POWER",

    .name = R"#(Player/Power/Player's base currently has power.[27])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} buildings are powered.)#",

},

{

    .index = "28",

    .id = "NAMED_REACHED_WAYPOINTS_END",

    .name = R"#(Object Type/Area or Path/Named object has reached the end of a specific waypoint path. (Unreliable)[28])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} has reached the end of ${WaypointPathName})#",

},

{

    .index = "29",

    .id = "TEAM_REACHED_WAYPOINTS_END",

    .name = R"#(Team/Area or Path/Team has reached the end of a specific waypoint path. (Unreliable)[29])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} has reached the end of ${WaypointPathName})#",

},

{

    .index = "30",

    .id = "NAMED_SELECTED",

    .name = R"#(Object Type/Selected/Named object is currently selected.[30])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} is currently selected.)#",

},

{

    .index = "31",

    .id = "NAMED_ENTERED_AREA",

    .name = R"#(Unused or Obsolete/Unit enters an area.[31])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} enters ${TriggerAreaName})#",

},

{

    .index = "32",

    .id = "NAMED_EXITED_AREA",

    .name = R"#(Object Type/Area or Path/Named object exits an area.[32])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} exits ${TriggerAreaName})#",

},

{

    .index = "33",

    .id = "TEAM_ENTERED_AREA_ENTIRELY",

    .name = R"#(Unused or Obsolete/Team entirely enters an area.[33])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} all enter ${TriggerAreaName} (${SurfaceType}).)#",

},

{

    .index = "34",

    .id = "TEAM_ENTERED_AREA_PARTIALLY",

    .name = R"#(Team/Area or Path/One unit from team enters an area.[34])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(One unit from ${TeamName} enters ${TriggerAreaName} (${SurfaceType}).)#",

},

{

    .index = "35",

    .id = "TEAM_EXITED_AREA_ENTIRELY",

    .name = R"#(Team/Area or Path/Team entirely exits an area.[35])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} has completely exited ${TriggerAreaName} (${SurfaceType}).)#",

},

{

    .index = "36",

    .id = "TEAM_EXITED_AREA_PARTIALLY",

    .name = R"#(Team/Area or Path/One unit from team exits an area.[36])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(One unit from ${TeamName} exits ${TriggerAreaName} (${SurfaceType}).)#",

},

{

    .index = "37",

    .id = "MULTIPLAYER_ALLIED_VICTORY",

    .name = R"#(Unused or Obsolete/Multiplayer allied victory.[37])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(The multiplayer game has ended in victory for the local player and his allies.)#",

},

{

    .index = "38",

    .id = "MULTIPLAYER_ALLIED_DEFEAT",

    .name = R"#(Unused or Obsolete/Multiplayer allied defeat.[38])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(The multiplayer game has ended in defeat for the local player and his allies.)#",

},

{

    .index = "39",

    .id = "MULTIPLAYER_PLAYER_DEFEAT",

    .name = R"#(Player/Win & Loss/Multiplayer local player defeat check.[39])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Everything belonging to the local player has been destroyed, but his allies may or may not have been defeated.)#",

},

{

    .index = "40",

    .id = "PLAYER_HAS_NO_POWER",

    .name = R"#(Player/Power/Player's base currently has no power.[40])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} buildings are not powered.)#",

},

{

    .index = "41",

    .id = "HAS_FINISHED_VIDEO",

    .name = R"#(Movie/Movie in the radar has completed playing.[41])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Movie ${MovieName} has completed playing.)#",

},

{

    .index = "42",

    .id = "HAS_FINISHED_SPEECH",

    .name = R"#(Unused or Obsolete/Speech has completed playing.[42])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${SpeechName} has completed playing.)#",

},

{

    .index = "43",

    .id = "HAS_FINISHED_AUDIO",

    .name = R"#(Audio/Sound has completed playing.[43])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${SoundName} has completed playing.)#",

},

{

    .index = "44",

    .id = "BUILDING_ENTERED_BY_PLAYER",

    .name = R"#(Player/Garrisoned/Player has garrisoned a specific building.[44])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has garrisoned building named ${UnitName})#",

},

{

    .index = "45",

    .id = "ENEMY_SIGHTED",

    .name = R"#(Object Type/Sighted/Unit has sighted a friendly, neutral, or enemy unit belonging to Player.[45])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} sees a(n) ${Relation} unit belonging to ${PlayerName}.)#",

},

{

    .index = "46",

    .id = "UNIT_HEALTH",

    .name = R"#(Object Type/Attacked, Destroyed, or Exists/Named object's health % compared to a value.[46])#",

    .help = R"#(Compares the unit's current health relative to the initial health (which may be modified in World Builder) to the value.)#",

    .argHint = R"#( ${UnitName} health is ${Comparison} ${Integer} percent.)#",

},

{

    .index = "47",

    .id = "NAMED_DYING",

    .name = R"#(Object Type/Attacked, Destroyed, or Exists/Named object is dying. (Cinematic)[47])#",

    .help = R"#(Returns true if the unit is currently dead, if the unit doesn)#",

    .argHint = R"#( ${UnitName} has been killed, but is still on screen.)#",

},

{

    .index = "48",

    .id = "NAMED_TOTALLY_DEAD",

    .name = R"#(Object Type/Attacked, Destroyed, or Exists/Named object is finished dying. (Cinematic)[48])#",

    .help = R"#(This will return true if the unit no longer exists but did in the past.  If the unit currently exists it will return false.)#",

    .argHint = R"#( ${UnitName} has been killed, and is finished dying.)#",

},

{

    .index = "49",

    .id = "PLAYER_HAS_OBJECT_COMPARISON",

    .name = R"#(Player/Owns/Player has (comparison) object type.[49])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has ${Comparison} ${Integer} object of type ${ObjectType}.)#",

},

{

    .index = "50",

    .id = "PLAYER_TRIGGERED_SPECIAL_POWER",

    .name = R"#(Player/Special Power/Player starts using a special power.[50])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} starts using ${SpecialPowerName}.)#",

},

{

    .index = "51",

    .id = "PLAYER_COMPLETED_SPECIAL_POWER",

    .name = R"#(Player/Special Power/Player completed using a special power.[51])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} completed using ${SpecialPowerName}.)#",

},

{

    .index = "52",

    .id = "PLAYER_TRIGGERED_SPECIAL_POWER_FROM_NAMED",

    .name = R"#(Player/Special Power/Player start using a special power from a named object.[52])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} starts using ${SpecialPowerName} from ${UnitName}.)#",

},

{

    .index = "53",

    .id = "PLAYER_BUILT_UPGRADE",

    .name = R"#(Player/Upgrades and Sciences/Player built an Upgrade.[53])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has built ${UpgradeName}.)#",

},

{

    .index = "54",

    .id = "PLAYER_DESTROYED_N_BUILDINGS_PLAYER",

    .name = R"#(Player/Attacked or Destroyed/Player destroyed N or more of an opponent's buildings.[54])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has destroyed ${Integer} or more buildings owned by ${PlayerName}.)#",

},

{

    .index = "55",

    .id = "PLAYER_HAS_COMPARISON_UNIT_TYPE_IN_TRIGGER_AREA",

    .name = R"#(Player/Area/Player has (comparison) object type in an area.[55])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has ${Comparison} ${Integer} object of type ${ObjectType} in the ${TriggerAreaName}.)#",

},

{

    .index = "56",

    .id = "PLAYER_HAS_COMPARISON_UNIT_KIND_IN_TRIGGER_AREA",

    .name = R"#(Player/Area/Player has (comparison) object types with kindof in an area.[56])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has ${Comparison} ${Integer} objects with ${UnitOrStructureKind} in the ${TriggerAreaName}.)#",

},

{

    .index = "57",

    .id = "TYPE_SIGHTED",

    .name = R"#(Object Type/Sighted/Named object has sighted a type of object belonging to Player.[57])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} sees a(n) ${ObjectType} belonging to ${PlayerName}.)#",

},

{

    .index = "58",

    .id = "NAMED_BUILDING_IS_EMPTY",

    .name = R"#(Object Type/Garrisoned/A specific building is empty.[58])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} is empty.)#",

},

{

    .index = "59",

    .id = "UNIT_HAS_OBJECT_STATUS",

    .name = R"#(Object Type/Object Status/Named object has object status.[59])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} has ${ObjectStatus})#",

},

{

    .index = "60",

    .id = "TEAM_ALL_HAS_OBJECT_STATUS",

    .name = R"#(Team/Object Status/Team has object status - all.[60])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} has ${ObjectStatus})#",

},

{

    .index = "61",

    .id = "TEAM_SOME_HAVE_OBJECT_STATUS",

    .name = R"#(Team/Object Status/Team has object status - partial.[61])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} has ${ObjectStatus})#",

},

{

    .index = "62",

    .id = "PLAYER_POWER_COMPARE_PERCENT",

    .name = R"#(Player/Power/Player has (comparison) percent power supply to consumption.[62])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has ${Comparison} ${Integer} percent power supply ratio.)#",

},

{

    .index = "63",

    .id = "PLAYER_EXCESS_POWER_COMPARE_VALUE",

    .name = R"#(Player/Power/Player has (comparison) kilowatts excess power supply.[63])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has ${Comparison} ${Integer} excess kilowatts power supply.)#",

},

{

    .index = "64",

    .id = "SKIRMISH_SPECIAL_POWER_READY",

    .name = R"#(Player/Special Power/Player's special power is ready to fire.[64])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} is ready to fire ${SpecialPowerName}.)#",

},

{

    .index = "65",

    .id = "SKIRMISH_VALUE_IN_AREA",

    .name = R"#(Unused or Obsolete/Player has (comparison) threat level in area.[65])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has ${Comparison} ${RealNumber} threat within area ${TriggerAreaName})#",

},

{

    .index = "66",

    .id = "SKIRMISH_PLAYER_FACTION",

    .name = R"#(Player/Faction and Start Position/Player is faction.[66])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} is ${FactionName})#",

},

{

    .index = "67",

    .id = "SKIRMISH_SUPPLIES_VALUE_WITHIN_DISTANCE",

    .name = R"#(Unused or Obsolete/Supplies are within specified distance.[67])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has supplies within ${RealNumber} of ${TriggerAreaName} worth at least ${RealNumber})#",

},

{

    .index = "68",

    .id = "SKIRMISH_COMMAND_BUTTON_READY_ALL",

    .name = R"#(Team/CommandButton/Team's CommandButton ability is ready - all.[68])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName}'s ${TeamName} are ready to use ${TeamAbilityName} (all applicable members).)#",

},

{

    .index = "69",

    .id = "SKIRMISH_COMMAND_BUTTON_READY_PARTIAL",

    .name = R"#(Team/CommandButton/Team's CommandButton ability is ready - partial[69])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName}'s ${TeamName} are ready to use ${TeamAbilityName} (at least one member).)#",

},

{

    .index = "70",

    .id = "SKIRMISH_UNOWNED_FACTION_UNIT_EXISTS",

    .name = R"#(Object Type/Unowned/There are (comparison) unowned faction objects.[70])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName}. There are ${Comparison} ${Integer} unowned faction objects.)#",

},

{

    .index = "71",

    .id = "SKIRMISH_PLAYER_HAS_COMPARISON_GARRISONED",

    .name = R"#(Player/Garrisoned/Player has (comparison) garrisoned buildings.[71])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has ${Comparison} ${Integer} garrisoned buildings.)#",

},

{

    .index = "72",

    .id = "SKIRMISH_PLAYER_HAS_UNITS_IN_AREA",

    .name = R"#(Player/Area/Player has units in an area[72])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has units in ${TriggerAreaName}.)#",

},

{

    .index = "73",

    .id = "SKIRMISH_PLAYER_HAS_BEEN_ATTACKED_BY_PLAYER",

    .name = R"#(Player/Attacked or Destroyed/Player has been attacked by Player.[73])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has been attacked by ${PlayerName}.)#",

},

{

    .index = "74",

    .id = "SKIRMISH_PLAYER_IS_OUTSIDE_AREA",

    .name = R"#(Unused or Obsolete/Player doesn't have units in an area.[74])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has doesn't have units in ${TriggerAreaName}.)#",

},

{

    .index = "75",

    .id = "SKIRMISH_PLAYER_HAS_DISCOVERED_PLAYER",

    .name = R"#(Player/Sighted/Player has sighted another player.[75])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has seen ${PlayerName}.)#",

},

{

    .index = "76",

    .id = "PLAYER_LOST_OBJECT_TYPE",

    .name = R"#(Player/Owns/Player has lost an object of type.[76])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has lost an object of type ${ObjectType}.)#",

},

{

    .index = "77",

    .id = "SUPPLY_SOURCE_SAFE",

    .name = R"#(Unused or Obsolete/Supply source is safe.[77])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} closest supply src with at least ${Integer} available resources is SAFE from enemy influence.)#",

},

{

    .index = "78",

    .id = "SUPPLY_SOURCE_ATTACKED",

    .name = R"#(Unused or Obsolete/Supply source is attacked.[78])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} supply source is under attack.)#",

},

{

    .index = "79",

    .id = "START_POSITION_IS",

    .name = R"#(Player/Faction and Start Position/Start Position.[79])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} starting position is ${Integer} .)#",

},

{

    .index = "80",

    .id = "PLAYER_HAS_COMPARISON_UNIT_TYPE_IN_TRIGGER_AREA_WITH_UPGRADE",

    .name = R"#(Player/Area/Player has (comparison) object type in an area with a specified upgrade.[80])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has ${Comparison} ${Integer} object of type with ${ObjectType} in the ${TriggerAreaName} with ${UpgradeName}.)#",

},

{

    .index = "81",

    .id = "COUNTER_COUNTER",

    .name = R"#(Scripting/Counter compared to another Counter.[81])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Counter ${CounterName} IS ${Comparison} counter ${CounterName})#",

},

{

    .index = "82",

    .id = "COUNTER_SECONDS",

    .name = R"#(Scripting/Counter compared to time in seconds.[82])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Counter ${CounterName} IS ${Comparison} time ${RealNumber} secs. )#",

},

{

    .index = "83",

    .id = "DISTANCE_BETWEEN_OBJ",

    .name = R"#(Scripting/Compare distance between 2 named objects.[83])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Distance between ${UnitName} and ${UnitName} is ${Comparison} ${RealNumber}.)#",

},

{

    .index = "84",

    .id = "DISTANCE_BETWEEN_TEAM",

    .name = R"#(Scripting/Compare distance between 2 teams.[84])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Distance between ${TeamName} and ${TeamName} is ${Comparison} ${RealNumber}.)#",

},

{

    .index = "85",

    .id = "UNIT_HAS_GAINED_LEVEL",

    .name = R"#(Object Type/Experience/Named object has gained experience level.[85])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} has gained a level.)#",

},

{

    .index = "86",

    .id = "UNIT_IS_AT_LEVEL",

    .name = R"#(Unused or Obsolete/Experience/Named object is at experience level.[86])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} is at experience level ${Text})#",

},

{

    .index = "87",

    .id = "CAMERA_HIT_SPECIFIC_SPLINE_WAYPOINT",

    .name = R"#(Camera/Camera following spline path hits specified waypoint.[87])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Camera following spline path hits ${WaypointName}.)#",

},

{

    .index = "88",

    .id = "HAS_COMMAND_POINTS_TO_BUILD_TEAM",

    .name = R"#(Player/CommandPoints/Player has enough command points to build a team. (May be broken. -JB)[88])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has enough command points to build ${TeamName}.)#",

},

{

    .index = "89",

    .id = "HAS_COMMAND_POINTS_TO_BUILD_UNIT",

    .name = R"#(Player/CommandPoints/Player has enough command points to build object type.[89])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has enough command points to build ${ObjectType}.)#",

},

{

    .index = "90",

    .id = "PLAYER_HAS_KILLED_KINDOF_UNITS",

    .name = R"#(Player/Attacked or Destroyed/Player has destroyed N or more objects of kindof.[90])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has killed ${Integer} or more objects of ${UnitOrStructureKind}.)#",

},

{

    .index = "91",

    .id = "PLAYER_HAS_KILLED_TYPE_UNITS",

    .name = R"#(Player/Attacked or Destroyed/Player has destroyed N or more objects of type.[91])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has killed ${Integer} or more ${ObjectType}.)#",

},

{

    .index = "92",

    .id = "NAMED_DESTROYED_BY_OBJECTTYPE",

    .name = R"#(Object Type/Attacked, Destroyed, or Exists/Named object is destroyed by a specific object type.[92])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} has been destroyed by a(n) ${ObjectType})#",

},

{

    .index = "93",

    .id = "PLAYER_HAS_N_OR_FEWER_BASES",

    .name = R"#(Unused or Obsolete/Player currently controls N or fewer bases.[93])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} currently controls ${Integer} or fewer bases.)#",

},

{

    .index = "94",

    .id = "CAMERA_ENTERED_AREA",

    .name = R"#(Camera/Camera entered trigger area.[94])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Camera enters ${TriggerAreaName}.)#",

},

{

    .index = "95",

    .id = "ENEMY_SIGHTED_BY_TEAM",

    .name = R"#(Team/Sighted/Team has sighted a friendly, neutral, or enemy unit belonging to Player.[95])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} sees a(n) ${Relation} unit belonging to ${PlayerName}.)#",

},

{

    .index = "96",

    .id = "COMPARISON_TREES_IN_TRIGGER_AREA",

    .name = R"#(Unused or Obsolete/There are (comparison) trees in an area.[96])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(There are ${Comparison} ${Integer} trees in the ${TriggerAreaName} .)#",

},

{

    .index = "97",

    .id = "GATE_IS_OPEN",

    .name = R"#(Unused or Obsolete/Check if a gate is open. (False, if not a gate, or closed)[97])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( The unit (presumed a gate) referenced as${UnitName} is OPEN.)#",

},

{

    .index = "98",

    .id = "UNIT_CAN_PATH_TO_WAYPOINT",

    .name = R"#(Object Type/Area or Path/Named object can path to waypoint.[98])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} can path to ${WaypointName}.)#",

},

{

    .index = "99",

    .id = "UNIT_CAN_PATH_TO_OBJECT",

    .name = R"#(Object Type/Area or Path/Named object can path to named object.[99])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} can path to ${UnitName}.)#",

},

{

    .index = "100",

    .id = "UNIT_CAN_PATH_INTO_PLAYERS_NEAREST_BASE",

    .name = R"#(Unused or Obsolete/Named object can path into (Player's) nearest base or is already inside it.[100])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} is inside, or can path into the nearest base owned by player, ${PlayerName})#",

},

{

    .index = "101",

    .id = "TEAM_CAN_PATH_TO_WAYPOINT",

    .name = R"#(Team/Area or Path/Team can path to waypoint.[101])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(The first unit of ${TeamName} can path to ${WaypointName}.)#",

},

{

    .index = "102",

    .id = "TEAM_CAN_PATH_TO_OBJECT",

    .name = R"#(Team/Area or Path/Team can path to named object.[102])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(The first unit of ${TeamName} can path to ${UnitName}.)#",

},

{

    .index = "103",

    .id = "TEAM_CAN_PATH_INTO_PLAYERS_NEAREST_BASE",

    .name = R"#(Unused or Obsolete/Team can path into (Player's) nearest base or is already inside it.[103])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(The first unit of team ${TeamName} is inside, or can path into the nearest base owned by player, ${PlayerName})#",

},

{

    .index = "104",

    .id = "MUSIC_IS_PLAYING_FROM_SCRIPT",

    .name = R"#(Audio/Music/Playing Music is from Script[104])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( The music scripting system is on.)#",

},

{

    .index = "105",

    .id = "UNIT_COMPARE_RANK",

    .name = R"#(Object Type/Attacked, Destroyed, or Exists/Named object's veterancy level compared to a value.[105])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Does ${UnitName} have veterancy level ${Comparison} ${Integer}.)#",

},

{

    .index = "106",

    .id = "PLAYER_HAS_OBJECT_OF_VETERANCY",

    .name = R"#(Player/Experience/Player has object of veterancy level compared to a value.[106])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has ${ObjectType} with veterancy level ${Comparison} ${Integer}.)#",

},

{

    .index = "107",

    .id = "PLAYER_HAS_NUMBER_OBJECTS_WITH_MODELCONDITION",

    .name = R"#(Player/Owns/Player has (comparison) objects with a model condition.[107])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has ${Comparison} ${Integer} objects with ${ModelCondition}.)#",

},

{

    .index = "108",

    .id = "PLAYER_HAS_NUMBER_UNITS_DISTANCE_FROM_OBJECT",

    .name = R"#(Player/Owns/Player has (comparison) obejcts more than x distance from named object.[108])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Player ${PlayerName} has ${Comparison} ${Integer} objects at least ${RealNumber} feet from ${UnitName}.)#",

},

{

    .index = "109",

    .id = "UNIT_HAS_PASSENGER",

    .name = R"#(Object Type/Garrison/Evaluates if named object is currently holding passengers.[109])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Evaluate if ${UnitName} is containing any passengers.)#",

},

{

    .index = "110",

    .id = "UNIT_IS_ATTACKED_AND_CANNOT_RETALIATE",

    .name = R"#(Object Type/Attacked, Destroyed, or Exists/Evaluate if a named object is being attacked but cannot retaliate[110])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Evaluate if ${UnitName} is being attacked and cannot retaliate.)#",

},

{

    .index = "111",

    .id = "TEAM_IS_ATTACKED_AND_CANNOT_RETALIATE_ANY",

    .name = R"#(Team/Attacked, Destroyed, or Exists/Evaluate if any member of a team is being attacked but cannot retaliate[111])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Evaluate if any team member of ${TeamName} is being attacked and cannot retaliate.)#",

},

{

    .index = "112",

    .id = "TEAM_IS_ATTACKED_AND_CANNOT_RETALIATE_ALL",

    .name = R"#(Team/Attacked, Destroyed, or Exists/Evaluate if all members of a team being attacked cannot retaliate[112])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Evaluate if all team members of ${TeamName} are being attacked and cannot retaliate.)#",

},

{

    .index = "113",

    .id = "IS_GAME_IN_SKIRMISH_OR_MULTIPLAYER",

    .name = R"#(Unused or Obsolete/Is game in a skirmish or multiplayer game.[113])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Is the game in a skirmish or multiplayer mode == ${Boolean})#",

},

{

    .index = "114",

    .id = "COMPARE_NUM_PLAYERS_IN_GAME",

    .name = R"#(Game Type/Compares the number of players in game to a value.[114])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Is this a ${Comparison} ${Integer} player game.)#",

},

{

    .index = "115",

    .id = "TEAM_HAS_FEWER_THAN_X_UNITS_WITH_KINDOF",

    .name = R"#(Team/Attacked, Destroyed, or Exists/Team has fewer than X units left of KindOf.[115])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} has fewer than ${Integer} units left of type ${UnitOrStructureKind}.)#",

},

{

    .index = "116",

    .id = "SKIRMISH_FIRE_SPECIAL_POWER_ON_TEAM",

    .name = R"#(Player/Special Power/Player can perform special power on team location.[116])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} can perform ${SpecialPowerName} at ${TeamName}'s location.)#",

},

{

    .index = "117",

    .id = "NAMED_RANK_LEVEL",

    .name = R"#(Unused or Obsolete/Named object is rank level or above.[117])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} is veterancy ${Integer} or above.)#",

},

{

    .index = "118",

    .id = "EVAL_TEAM_HEALTH",

    .name = R"#(Team/Health/Team health level compared to a value.[118])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Does ${TeamName} have ${Comparison} ${Integer} percent health.)#",

},

{

    .index = "120",

    .id = "NUM_UNITS_LEVELED_UP",

    .name = R"#(Player/Owns/Num objects leveled up during game.[120])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Does ${PlayerName} have ${Integer} objects that have gained veterancy this game.)#",

},

{

    .index = "121",

    .id = "PLAYER_HAS_NUM_UNITS_WITH_UPGRADE",

    .name = R"#(Player/Owns/Num objects has specified upgrade.[121])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Does ${PlayerName} have ${Integer} units that have ${UpgradeName}.)#",

},

{

    .index = "122",

    .id = "PLAYER_HAS_NUM_UNITS_LOADED_WITH_OBJECT",

    .name = R"#(Player/Owns/Num objects of type loaded on transport.[122])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Does ${PlayerName} have ${Integer} objects of type ${ObjectName} loaded on transport of type ${ObjectName}.)#",

},

{

    .index = "124",

    .id = "UNIT_USING_AUTOPICKUP",

    .name = R"#(Unused or Obsolete/Does Player have Units that have triggered auto pickup.[124])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Does ${PlayerName} have ${ObjectName} that has triggered auto pickup.)#",

},

{

    .index = "125",

    .id = "UNIT_IN_ALT_FORMATION",

    .name = R"#(Unused or Obsolete/Does Player have Units that are in alternate formation.[125])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Does ${PlayerName} have ${ObjectName} that is currently in alternate formation.)#",

},

{

    .index = "126",

    .id = "HAS_EVA_EVENT_PLAYED_IN_LAST_N_SECONDS",

    .name = R"#(Player/Eva/Has EVA event played recently.[126])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Has ${EVA} played with in the last ${RealNumber} seconds.)#",

},

{

    .index = "127",

    .id = "IS_NUM_OF_UNITS_BELONGING_TO_PLAYER_NEAR_EVA_EVENT_LAST_PLAYED_LOCATION_COMPARISON_INT",

    .name = R"#(Unused or Obsolete/Is number of units belonging to player near EVA event's last played location greater than, less than, or equal to a fixed value.[127])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Is the number of units belong to ${PlayerName} within ${RealNumber} feet of the location where ${EVA} last played ${Comparison} ${Integer})#",

},

{

    .index = "128",

    .id = "PLAYER_HAS_COMPARISON_UNIT_TYPE_IN_TRIGGER_AREA_COMPLETELY_BUILT",

    .name = R"#(Player/Area/Player has (comparison) completely built unit types in an area.[128])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has ${Comparison} ${Integer} completely built objects of type ${ObjectType} in the ${TriggerAreaName}.)#",

},

{

    .index = "129",

    .id = "CAMERA_SCROLL_DISTANCE",

    .name = R"#(Tutorial/Camera/Camera scroll distance.[129])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Player has scrolled camera ${RealNumber} units.)#",

},

{

    .index = "130",

    .id = "CAMERA_ROTATE_DISTANCE",

    .name = R"#(Tutorial/Camera/Camera rotate distance.[130])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Player has rotated camera ${RealNumber} degrees.)#",

},

{

    .index = "131",

    .id = "CAMERA_ZOOM_DISTANCE",

    .name = R"#(Tutorial/Camera/Camera zoom distance.[131])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Player has zoomed camera ${RealNumber} units.)#",

},

{

    .index = "132",

    .id = "CAMERA_RESET",

    .name = R"#(Tutorial/Camera/Camera reset.[132])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Player has reset camera)#",

},

{

    .index = "133",

    .id = "UNIT_USING_STANCE",

    .name = R"#(Object Type/Stance/Named object is using stance.[133])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Is ${UnitName} in the ${ObjectStance}.)#",

},

{

    .index = "134",

    .id = "PLAYER_ISSUED_FORMATION_ORDER",

    .name = R"#(Tutorial/Player/Player has issued a formation order.[134])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Player has issued formation order)#",

},

{

    .index = "135",

    .id = "TYPE_SELECTED",

    .name = R"#(Object Type/Selected/Object Type is currently selected.[135])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${ObjectType} is currently selected.)#",

},

{

    .index = "136",

    .id = "UNIT_TEST_OBJECT_PANEL_FLAGS",

    .name = R"#(Object Type/Settings/Test object flags.[136])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Test the value of ${ObjectPanelFlag} for ${UnitName}.)#",

},

{

    .index = "137",

    .id = "TEST_CAMPAIGN_FLAG",

    .name = R"#(Campaign/Test a campaign flag.[137])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Test the value of campaign flag ${CampaignFlag}.)#",

},

{

    .index = "138",

    .id = "UNIT_HAS_UPGRADE",

    .name = R"#(Object Type/Upgrades/Unit has upgrade.[138])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} has ${UpgradeName}.)#",

},

{

    .index = "139",

    .id = "UNIT_SPECIAL_POWER_READY",

    .name = R"#(Object Type/Special Power/Unit's special power is ready to fire.[139])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} is ready to fire ${SpecialPowerName}.)#",

},

{

    .index = "140",

    .id = "PLAYER_HAS_NUMBER_OBJECTS_WITH_OBJECT_STATUS",

    .name = R"#(Player/Owns/Player has (comparison) objects with object status[140])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has ${Comparison} ${Integer} with ${ObjectStatus}.)#",

},

{

    .index = "141",

    .id = "PLAYER_TRIGGERED_SCRIPT_EVENT",

    .name = R"#(Player/Player has triggered game event[141])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has triggered ${ScriptEvent}.)#",

},

{

    .index = "142",

    .id = "BUILDING_GARRISONNED_BY_OBJECT_TYPE",

    .name = R"#(Object Type/Garrison/Named building is garrisoned by object type owned by specific player.[142])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} is garrisoned by object type ${ObjectType} owned by ${PlayerName}.)#",

},

{

    .index = "143",

    .id = "PLAYER_TRIGGERED_SPECIAL_POWER_IN_AREA",

    .name = R"#(Player/Special Power/Player starts using a special power in trigger area.[143])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} starts using ${SpecialPowerName} in ${TriggerAreaName}.)#",

},

{

    .index = "144",

    .id = "MISSION_HOT_SPOT_ACTIVE",

    .name = R"#(Interface/Mission Hot-Spot/Mission hot-spot has been activated.[144])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Mission hot-spot ${MissionHotSpot} has been activated.)#",

},

{

    .index = "145",

    .id = "PLAYER_RED_ALERT_ACTIVATED",

    .name = R"#(Player/Red Alerts/Player activated their Red Alert.[145])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Player has activated their Red Alert.${PlayerName})#",

},

{

    .index = "146",

    .id = "PLAYER_IS_HUMAN_OR_AI_PERSONALITY",

    .name = R"#(Player/Controlled By/Is player controlled by human or AI Personality.[146])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} is controlled by ${Personality})#",

},

{

    .index = "147",

    .id = "PLAYER_HAS_KILLED_NAMED_UNIT",

    .name = R"#(Player/Attacked or Destroyed/Player has killed Named Unit[147])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has killed object named: ${UnitName})#",

},

{

    .index = "148",

    .id = "TEAM_COMPARE_NUM_MEMBERS",

    .name = R"#(Team/Attacked, Destroyed, or Exists/Team has (comparison) number of objects.[148])#",

    .help = R"#(Compares the number of objects in a team to the passed in comparison values.  Note that this script does not count dead, under construction, projectiles, or units with the IGNORE_FOR_VICTORY kind of.)#",

    .argHint = R"#( ${TeamName} has${Comparison} ${Integer} members.)#",

},

{

    .index = "149",

    .id = "NAMED_HAS_KILLED_COMPARISON_UNIT_TYPE",

    .name = R"#(Object Type/Attacked, Destroyed, or Exists/Named object has killed (comparison) number of object type or type list.[149])#",

    .help = R"#(Script will compare if a unit has killed (x) number of object type(s).)#",

    .argHint = R"#( ${UnitName} has killed  ${Comparison} ${Integer} ${ObjectType})#",

},

{

    .index = "150",

    .id = "UNIT_HAS_MODELCONDITION",

    .name = R"#(Object Type/Settings/Named object has model condition.[150])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} has ${ModelCondition})#",

},

{

    .index = "151",

    .id = "PLAYER_HAS_COMPARISON_POWER_POINTS",

    .name = R"#(Player/Special Power/Player has <comparison><n> power points.[151])#",

    .help = R"#(Determines if a player has x amount of player power points.)#",

    .argHint = R"#( ${PlayerName} has ${Comparison} ${Integer} power points.)#",

},

{

    .index = "152",

    .id = "UI_STATE_PROPOSITION_STATE",

    .name = R"#(Tutorial/UI state proposition is true.[152])#",

    .help = R"#(The condition is only guaranteed to work correctly in a tutorial game)#",

    .argHint = R"#( ${UiState} is true.)#",

},

{

    .index = "153",

    .id = "PLAYER_HAS_PLAYER_TECH",

    .name = R"#(Player/Special Power/Player has player tech.[153])#",

    .help = R"#(Determines if a player has purchased a player tech.)#",

    .argHint = R"#( ${PlayerName} has ${PlayerTech} player tech.)#",

},

{

    .index = "154",

    .id = "CURRENT_PLATFORM",

    .name = R"#(Platform/Is game running on platform.[154])#",

    .help = R"#(Returns true if running on the chosen platform.  Current options are PC/360/PS3)#",

    .argHint = R"#(Is game running on ${Platform})#",

},

{

    .index = "155",

    .id = "PLAYER_HAS_COMPARISON_UNIT_KIND",

    .name = R"#(Player/Owns/Player has (comparison) object types with kindof.[155])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} has ${Comparison} ${Integer} objects with ${UnitOrStructureKind}.)#",

},



};
