---
title: "《Lua里的地编动作脚本》"
catalog_path: "Lua API "
slug: pzle8sromsst4oh9
source: https://www.yuque.com/muzeqaq/ra3mapwiki/pzle8sromsst4oh9
fetched: 2026-10-15
content_updated_at: "2024-12-16T23:55:26.000Z"
type: Doc
format: lake
---

# 《Lua里的地编动作脚本》

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/pzle8sromsst4oh9
> 目录位置：Lua API 

以下地编脚本可在 Lua 里通过 ExecuteAction 调用。

例子：

```lua
--- @type StandardUnitType
local unit = GetObjectByScriptName("UNIT") --- 假设地图上存在名为UNIT的单位，存活状态
ExecuteAction("NAMED_HUNT", unit) --- 单位开始猎杀
```

std::vector<WBScriptItem>

{



{

    .index = "0",

    .id = "DEBUG_MESSAGE_BOX",

    .name = R"#(Scripting/Debug/Display message and pause[0])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Show debug string and pause: ${Text})#",

},

{

    .index = "3",

    .id = "VICTORY",

    .name = R"#(Win & Loss/Announce victory.[3])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Show 'Victorious' window and end game)#",

},

{

    .index = "4",

    .id = "DEFEAT",

    .name = R"#(Win & Loss/Announce defeat.[4])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Show 'Defeated' window and end game)#",

},

{

    .index = "7",

    .id = "PLAY_SOUND_EFFECT",

    .name = R"#(Media/Audio/Sound Effect/Play 2D sound effect.[7])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Play ${SoundName} as though it was a 2D sound (NOT RECOMMENDED for 3D sounds).)#",

},

{

    .index = "11",

    .id = "PLAY_SOUND_EFFECT_AT",

    .name = R"#(Media/Audio/Sound Effect/Play sound effect at waypoint.[11])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Play ${SoundName} at ${WaypointName}.)#",

},

{

    .index = "12",

    .id = "DAMAGE_MEMBERS_OF_TEAM",

    .name = R"#(Team/Damage or Remove/Damage the members of Team.[12])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Damage ${TeamName}, amount=${RealNumber} (-1==kill).)#",

},

{

    .index = "13",

    .id = "MOVE_TEAM_TO",

    .name = R"#(Team/Move/Set team to move to a location.[13])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Move ${TeamName} to ${WaypointName}.)#",

},

{

    .index = "14",

    .id = "MOVE_CAMERA_TO",

    .name = R"#(Cinematics/Camera/Move the camera to a location.[14])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Move camera to ${CameraLocation} in ${RealNumber} seconds, camera shutter ${RealNumber} seconds, ease-in ${RealNumber} seconds, ease-out ${RealNumber} seconds.)#",

},

{

    .index = "17",

    .id = "MOVE_CAMERA_ALONG_SPLINE_PATH",

    .name = R"#(Cinematics/Camera/Move along a spline path.[17])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Move along ${WaypointPathName} in ${RealNumber} seconds, camera shutter ${RealNumber} seconds, ease-in ${RealNumber} seconds, ease-out ${RealNumber} seconds, delay start ${RealNumber} seconds. )#",

},

{

    .index = "18",

    .id = "ROTATE_CAMERA",

    .name = R"#(Cinematics/Camera/Rotate around the current viewpoint.[18])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Rotate ${RealNumber} times, taking ${RealNumber} seconds, ease-in ${RealNumber} seconds, ease-out ${RealNumber} seconds.)#",

},

{

    .index = "19",

    .id = "RESET_CAMERA",

    .name = R"#(Cinematics/Camera/Reset to the default view.[19])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Reset to ${WaypointName}, taking ${RealNumber} seconds, ease-in ${RealNumber} seconds, ease-out ${RealNumber} seconds.)#",

},

{

    .index = "20",

    .id = "RESET_CAMERA_TUTORIAL_STATE",

    .name = R"#(Tutorial/Reset camera tutorial state.[20])#",

    .help = R"#(Makes it so that script commands that try to detect camera rotation, etc can be accurate from this point.)#",

    .argHint = R"#(Reset camera tutorial state.)#",

},

{

    .index = "22",

    .id = "CAMERA_MOD_FREEZE_TIME",

    .name = R"#(Cinematics/Settings/Freeze time during the camera movement.[22])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Freeze time during the camera movement.)#",

},

{

    .index = "23",

    .id = "CREATE_OBJECT",

    .name = R"#(Unused or Obsolete/Object Type/Spawn/Spawn object.[23])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Spawn object ${ObjectName} in ${TeamName} at position (${PositionCoordinate}), rotated ${Angle} .)#",

},

{

    .index = "24",

    .id = "SUSPEND_BACKGROUND_SOUNDS",

    .name = R"#(Media/Audio/All Sounds/Suspend all sounds.[24])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Suspend background sounds.)#",

},

{

    .index = "25",

    .id = "RESUME_BACKGROUND_SOUNDS",

    .name = R"#(Media/Audio/All Sounds/Resume all sounds.[25])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Resume background sounds.)#",

},

{

    .index = "26",

    .id = "CAMERA_MOD_SET_FINAL_ZOOM",

    .name = R"#(Cinematics/Camera/Set Final zoom for camera movement.[26])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Adjust zoom to ${RealNumber} (1.0==max height, 0.0==in the ground) ${Percentage} ease-in ${Percentage} ease-out.)#",

},

{

    .index = "27",

    .id = "CAMERA_MOD_SET_FINAL_PITCH",

    .name = R"#(Cinematics/Camera/Set Final pitch for camera movement.[27])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Adjust pitch to ${RealNumber} (1.0==default, 0.0==toward horizon, >1 = toward ground) ${Percentage} ease-in ${Percentage} ease-out.)#",

},

{

    .index = "28",

    .id = "CAMERA_MOD_LOOK_TOWARD",

    .name = R"#(Cinematics/Camera/Camera look toward point while moving.[28])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Look toward ${WaypointName} during the camera movement.)#",

},

{

    .index = "29",

    .id = "TEAM_ATTACK_TEAM",

    .name = R"#(Team/Attack/Set team to attack another team.[29])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} begin attack on ${TeamName})#",

},

{

    .index = "30",

    .id = "CREATE_REINFORCEMENT_TEAM",

    .name = R"#(Team/Spawn, Build, or Merge/Spawn a reinforcement team at location.[30])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Spawn an instance of ${TeamName} at ${WaypointName}.)#",

},

{

    .index = "31",

    .id = "MOVE_CAMERA_TO_SELECTION",

    .name = R"#(Cinematics/Camera/End movement at selected unit.[31])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(End movement at selected unit.)#",

},

{

    .index = "32",

    .id = "TEAM_FOLLOW_WAYPOINTS",

    .name = R"#(Team/Move/Set team to follow a waypoint path.[32])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Have ${TeamName} follow ${WaypointPathName} , as a team is ${Boolean} , in formation is ${Boolean})#",

},

{

    .index = "33",

    .id = "MOVE_NAMED_UNIT_TO",

    .name = R"#(Object Type/Move/Named object move to a location.[33])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Move ${UnitName} to ${WaypointName}.)#",

},

{

    .index = "34",

    .id = "NAMED_ATTACK_NAMED",

    .name = R"#(Object Type/Attack/Set unit to attack another unit.[34])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} begin attack on ${UnitName})#",

},

{

    .index = "35",

    .id = "CREATE_NAMED_ON_TEAM_AT_WAYPOINT",

    .name = R"#(Object Type/Spawn/Spawn a named object on a team at a waypoint.[35])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Spawn ${UnitName} of type ${ObjectName} on ${TeamName} at ${WaypointName})#",

},

{

    .index = "36",

    .id = "CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT",

    .name = R"#(Object Type/Spawn/Spawn an unnamed object on a team at a waypoint.[36])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Spawn unit of type ${ObjectName} on ${TeamName} at ${WaypointName})#",

},

{

    .index = "39",

    .id = "SET_BASE_CONSTRUCTION_SPEED",

    .name = R"#(Player/Build/Set the delay between building teams.[39])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} seconds between building teams.${Integer})#",

},

{

    .index = "40",

    .id = "NAMED_ATTACK_AREA",

    .name = R"#(Object Type/Attack/Named object attacks an area.[40])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} attacks anything in ${TriggerAreaName})#",

},

{

    .index = "41",

    .id = "NAMED_ATTACK_AREA_FOR_SECONDS",

    .name = R"#(Object Type/Attack/Named object attacks an area for a given number of seconds.[41])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} attacks anything in ${TriggerAreaName} for ${Integer} seconds.)#",

},

{

    .index = "42",

    .id = "NAMED_ATTACK_TEAM",

    .name = R"#(Object Type/Attack/Named object attacks Team.[42])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} attacks ${TeamName})#",

},

{

    .index = "43",

    .id = "TEAM_ATTACK_NAMED",

    .name = R"#(Team/Attack/Team attacks a named object.[43])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} attacks ${UnitName})#",

},

{

    .index = "44",

    .id = "TEAM_LOAD_TRANSPORTS",

    .name = R"#(Unused or Obsolete/Team/Transport/Transport -- automatically load.[44])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} load into transports.)#",

},

{

    .index = "47",

    .id = "NAMED_EXIT_ALL",

    .name = R"#(Unused or Obsolete/Object Type/Transport/Transport -- unload units from specific.[47])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} unloads.)#",

},

{

    .index = "48",

    .id = "TEAM_EXIT_ALL",

    .name = R"#(Unused or Obsolete/Team/Transport/Transport -- unload team from all.[48])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} unload.)#",

},

{

    .index = "49",

    .id = "NAMED_FOLLOW_WAYPOINTS",

    .name = R"#(Object Type/Move/Named object follow a waypoint path.[49])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} follow ${WaypointPathName})#",

},

{

    .index = "50",

    .id = "NAMED_GUARD",

    .name = R"#(Object Type/Guard/Named object guards.[50])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} begins guarding.)#",

},

{

    .index = "51",

    .id = "TEAM_GUARD",

    .name = R"#(Team/Guard/Team guard current location.[51])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} begins guarding.)#",

},

{

    .index = "52",

    .id = "NAMED_HUNT",

    .name = R"#(Object Type/Attack/Hunt (choose enemy target, move towards it, and attack when within range).[52])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} begins hunting.)#",

},

{

    .index = "53",

    .id = "TEAM_HUNT",

    .name = R"#(Team/Attack/Hunt (choose enemy target, move towards it, and attack when within range).[53])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} begins hunting.)#",

},

{

    .index = "54",

    .id = "SELL_OBJECT",

    .name = R"#(Object Type/Damage or Remove/Sell off named Object.[54])#",

    .help = R"#(This script will attempt to sell the named object regardless who it belongs to even if the player is a human.  Also, it should only be called on sellable objects.  Be careful!)#",

    .argHint = R"#(Sell off ${UnitName})#",

},

{

    .index = "55",

    .id = "PLAYER_SELL_EVERYTHING",

    .name = R"#(Player/Remove, Sell, or Transfer All/Player sells everything.[55])#",

    .help = R"#(This script will attempt to sell all of a players structures, even if the player is a human.  Be careful!)#",

    .argHint = R"#(Sell everything belonging to ${PlayerName})#",

},

{

    .index = "56",

    .id = "PLAYER_DISABLE_BASE_CONSTRUCTION",

    .name = R"#(Player/Build/Permissions/Player is not allowed to build buildings.[56])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} is not allowed to build buildings.)#",

},

{

    .index = "57",

    .id = "PLAYER_DISABLE_FACTORIES",

    .name = R"#(Player/Build/Player cannot build from an object type.[57])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} is not allowed to build from ${ObjectName})#",

},

{

    .index = "58",

    .id = "PLAYER_DISABLE_UNIT_CONSTRUCTION",

    .name = R"#(Player/Build/Permissions/Player is not allowed to build units.[58])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} is not allowed to build units.)#",

},

{

    .index = "59",

    .id = "PLAYER_ENABLE_BASE_CONSTRUCTION",

    .name = R"#(Player/Build/Permissions/Player is allowed to build buildings.[59])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} is allowed to build buildings.)#",

},

{

    .index = "60",

    .id = "PLAYER_ENABLE_FACTORIES",

    .name = R"#(Player/Build/Player can build from an object type.[60])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} is allowed to build from ${ObjectName})#",

},

{

    .index = "61",

    .id = "PLAYER_ENABLE_UNIT_CONSTRUCTION",

    .name = R"#(Player/Build/Permissions/Player is allowed to build units.[61])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} is able to build units.)#",

},

{

    .index = "62",

    .id = "BUILD_TEAM",

    .name = R"#(Team/Spawn, Build, or Merge/Start building Team.[62])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Start building ${TeamName})#",

},

{

    .index = "63",

    .id = "NAMED_DAMAGE",

    .name = R"#(Object Type/Damage or Remove/Deal damage to a named object.[63])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} takes ${Integer} points of damage.)#",

},

{

    .index = "64",

    .id = "NAMED_DELETE",

    .name = R"#(Object Type/Damage or Remove/Delete a named object.[64])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} is removed from the world.)#",

},

{

    .index = "65",

    .id = "TEAM_DELETE",

    .name = R"#(Team/Damage or Remove/Delete Team.[65])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} is removed from the world.)#",

},

{

    .index = "66",

    .id = "NAMED_SHOCK",

    .name = R"#(Unused or Obsolete/Object Type/Damage or Remove/Shock a specific unit.[66])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} will be shocked dir (0-360) ${RealNumber}, force ${RealNumber}, zMult ${RealNumber}, with FX named ${Text} .)#",

},

{

    .index = "67",

    .id = "NAMED_KILL",

    .name = R"#(Object Type/Damage or Remove/Kill a named object.[67])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName}is dealt a lethal amount of damage.)#",

},

{

    .index = "68",

    .id = "TEAM_KILL",

    .name = R"#(Team/Damage or Remove/Kill an entire team.[68])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} is dealt a lethal amount of damage.)#",

},

{

    .index = "69",

    .id = "PLAYER_KILL",

    .name = R"#(Player/Remove, Sell, or Transfer All/Kill Player.[69])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(All of ${PlayerName}'s buildings and units are dealt a lethal amount of damage.)#",

},

{

    .index = "70",

    .id = "NAMED_FLASH",

    .name = R"#(Object Type/Flash or Color/Flash a specific unit for a specified amount of time.[70])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} flashes for ${Integer} seconds.)#",

},

{

    .index = "71",

    .id = "TEAM_FLASH",

    .name = R"#(Team/Flash/Flash Team for a specified amount of time.[71])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} flashes for ${Integer} seconds.)#",

},

{

    .index = "72",

    .id = "MOVIE_PLAY_FULLSCREEN",

    .name = R"#(Media/Video/Play a movie in fullscreen mode.[72])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${MovieName} plays fullscreen but doesn't play for low detail ( ${Boolean} ))#",

},

{

    .index = "73",

    .id = "MOVIE_PLAY_RADAR",

    .name = R"#(Media/Video/Play a movie in the radar.[73])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${MovieName} plays in the radar window.)#",

},

{

    .index = "74",

    .id = "SOUND_PLAY_NAMED",

    .name = R"#(Media/Audio/Sound Effect/Play a sound as though coming from a named object.[74])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${SoundName} plays as though coming from ${UnitName})#",

},

{

    .index = "75",

    .id = "SPEECH_PLAY",

    .name = R"#(Unused or Obsolete/Media/Audio/Sound Effect/Play a speech file.[75])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${SpeechName} plays, allowing overlap ${Boolean} (true to allow, false to disallow).)#",

},

{

    .index = "76",

    .id = "PLAYER_TRANSFER_OWNERSHIP_PLAYER",

    .name = R"#(Player/Remove, Sell, or Transfer All/Transfer assets from one player to another player.[76])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(All assets of ${PlayerName} are transferred to ${PlayerName})#",

},

{

    .index = "77",

    .id = "NAMED_TRANSFER_OWNERSHIP_PLAYER",

    .name = R"#(Object Type/Allegiance/Transfer a named object to the control of a Player.[77])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} is transferred to the command of ${PlayerName})#",

},

{

    .index = "78",

    .id = "PLAYER_RELATES_PLAYER",

    .name = R"#(Player/Allegiance/Change how Player relates to another player.[78])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} considers ${PlayerName} to be ${Relation})#",

},

{

    .index = "79",

    .id = "OPTIONAL_VICTORY",

    .name = R"#(Unused or Obsolete/Win & Loss/End the mission in victory if the user chooses to[79])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(End game in victory if the user indicates that they wish to.)#",

},

{

    .index = "80",

    .id = "RADAR_CREATE_EVENT",

    .name = R"#(Interface/Radar/Create a radar event at a specified location.[80])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(A radar event occurs at ${PositionCoordinate} of type ${RadarEventType})#",

},

{

    .index = "81",

    .id = "MAP_REVEAL_AT_WAYPOINT",

    .name = R"#(Map/Fog or Reveal/Reveal map at waypoint -- fog.[81])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(The map is revealed at ${WaypointName} with a radius of ${RealNumber} feet for ${PlayerName}.)#",

},

{

    .index = "82",

    .id = "TEAM_MERGE_INTO_TEAM",

    .name = R"#(Team/Spawn, Build, or Merge/Merge Team into another team.[82])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} merges onto ${TeamName})#",

},

{

    .index = "83",

    .id = "DISABLE_INPUT",

    .name = R"#(Interface/Input/User input -- disable.[83])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Disable mouse and keyboard input.)#",

},

{

    .index = "84",

    .id = "ENABLE_INPUT",

    .name = R"#(Interface/Input/User input -- enable.[84])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Enable mouse and keyboard input.)#",

},

{

    .index = "85",

    .id = "SOUND_AMBIENT_PAUSE",

    .name = R"#(Media/Audio/Sound Effect/Suspend the ambient sounds.[85])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Pause the ambient sounds.)#",

},

{

    .index = "86",

    .id = "SOUND_AMBIENT_RESUME",

    .name = R"#(Media/Audio/Sound Effect/Resume the ambient sounds.[86])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Resume the ambient sounds.)#",

},

{

    .index = "87",

    .id = "MUSIC_SET_TRACK",

    .name = R"#(Media/Audio/Music/Play a music track.[87])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Play ${MusicName} using fadeout (${Boolean}) and fadein (${Boolean}).)#",

},

{

    .index = "88",

    .id = "DEBUG_STRING",

    .name = R"#(Scripting/Debug/Display string[88])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Show debug string without pausing: ${Text})#",

},

{

    .index = "89",

    .id = "MAP_REVEAL_ALL",

    .name = R"#(Unused or Obsolete/Map/Shroud or Reveal/Reveal the entire map for Player.[89])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(The world is revealed for ${PlayerName}.)#",

},

{

    .index = "90",

    .id = "TEAM_GARRISON_SPECIFIC_BUILDING",

    .name = R"#(Team/Garrison/Garrison a named object with Team.[90])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} enters into ${UnitName})#",

},

{

    .index = "92",

    .id = "TEAM_GARRISON_NEAREST_BUILDING",

    .name = R"#(Team/Garrison/Garrison a nearby structure with Team.[92])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} garrison a nearby object.)#",

},

{

    .index = "93",

    .id = "TEAM_EXIT_ALL_BUILDINGS",

    .name = R"#(Team/Garrison/Exit all objects a team is in.[93])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} exits all objects.)#",

},

{

    .index = "95",

    .id = "NAMED_GARRISON_NEAREST_BUILDING",

    .name = R"#(Object Type/Garrison/Garrison a nearby structure with a named object.[95])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} garrison a nearby structure.)#",

},

{

    .index = "96",

    .id = "NAMED_EXIT_BUILDING",

    .name = R"#(Object Type/Garrison/Named object exit whatever object it's in.[96])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} leaves it's garrison.)#",

},

{

    .index = "97",

    .id = "PLAYER_EXIT_ALL_BUILDINGS",

    .name = R"#(Player/Garrison/All of Player's units leave their garrisons.[97])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(All of ${PlayerName} units leave their garrisons.)#",

},

{

    .index = "98",

    .id = "TEAM_WANDER",

    .name = R"#(Team/Move/Follow a waypoint path -- wander.[98])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Have ${TeamName} wander along ${WaypointPathName})#",

},

{

    .index = "99",

    .id = "TEAM_PANIC",

    .name = R"#(Cinematics/Force Emotion/Follow a waypoint path -- panic.[99])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Have ${TeamName} move in panic along ${WaypointPathName})#",

},

{

    .index = "100",

    .id = "SETUP_CAMERA",

    .name = R"#(Cinematics/Camera/Set up the camera.[100])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Position camera at ${WaypointName}, zoom = ${RealNumber}(0.0 to 1.0), pitch = ${RealNumber}(1.0==default), looking towards ${WaypointName}.)#",

},

{

    .index = "101",

    .id = "CAMERA_LETTERBOX_BEGIN",

    .name = R"#(Cinematics/Settings/Start letterbox mode.[101])#",

    .help = R"#(This will hide ALL UI elements including the radar.)#",

    .argHint = R"#(Start letterbox mode (hide UI, add border).)#",

},

{

    .index = "102",

    .id = "CAMERA_LETTERBOX_END",

    .name = R"#(Cinematics/Settings/End letterbox mode.[102])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(End letterbox mode (show UI, remove border).)#",

},

{

    .index = "103",

    .id = "ZOOM_CAMERA",

    .name = R"#(Cinematics/Camera/Change the camera zoom.[103])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Change camera zoom to ${RealNumber} in ${RealNumber} seconds, ease-in ${RealNumber} seconds, ease-out ${RealNumber} seconds.)#",

},

{

    .index = "104",

    .id = "PITCH_CAMERA",

    .name = R"#(Cinematics/Camera/Change the camera pitch.[104])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Change camera pitch to ${RealNumber} in ${RealNumber} seconds, ease-in ${RealNumber} seconds, ease-out ${RealNumber} seconds.)#",

},

{

    .index = "105",

    .id = "CAMERA_FOLLOW_NAMED",

    .name = R"#(Cinematics/Camera/Follow a specific unit.[105])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Have the camera follow ${UnitName}. Snap camera to object is ${Boolean}. Camera will be ${RealNumber} higher than the unit.)#",

},

{

    .index = "113",

    .id = "CAMERA_STOP_FOLLOW",

    .name = R"#(Cinematics/Camera/Stop following any units.[113])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Stop following any units.)#",

},

{

    .index = "114",

    .id = "FREEZE_TIME",

    .name = R"#(Cinematics/Settings/Freeze time.[114])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Freeze time.)#",

},

{

    .index = "115",

    .id = "UNFREEZE_TIME",

    .name = R"#(Cinematics/Settings/Unfreeze time.[115])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Unfreeze time.)#",

},

{

    .index = "116",

    .id = "SHOW_MILITARY_CAPTION",

    .name = R"#(Tutorial/Show windowed text caption.[116])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Show ${LocalizedStringName} for ${RealNumber} seconds.)#",

},

{

    .index = "117",

    .id = "SET_STOPPING_DISTANCE",

    .name = R"#(Unused or Obsolete/Team/Settings/Set stopping distance for each unit's current locomotor.[117])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set stopping distances for ${TeamName} to ${RealNumber}.)#",

},

{

    .index = "118",

    .id = "NAMED_SET_STOPPING_DISTANCE",

    .name = R"#(Unused or Obsolete/Object Type/Settings/Set stopping distance for current locomotor.[118])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set stopping distance for ${UnitName} to ${RealNumber}.)#",

},

{

    .index = "119",

    .id = "MUSIC_SET_VOLUME",

    .name = R"#(Media/Audio/Volume/Set the current music volume.[119])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set the desired music volume to ${RealNumber}%. (0-100))#",

},

{

    .index = "120",

    .id = "MAP_SHROUD_ALL",

    .name = R"#(Unused or Obsolete/Map/Shroud or Reveal/Shroud the entire map for Player.[120])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(The world is shrouded for ${PlayerName}.)#",

},

{

    .index = "127",

    .id = "TEAM_TRANSFER_TO_PLAYER",

    .name = R"#(Team/Allegiance/Transfer control of Team to Player.[127])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Control of ${TeamName} transfers to ${PlayerName})#",

},

{

    .index = "128",

    .id = "PLAYER_SET_MONEY",

    .name = R"#(Player/Money/Set Player's money.[128])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set ${PlayerName}'s money to $${Integer})#",

},

{

    .index = "129",

    .id = "PLAYER_GIVE_MONEY",

    .name = R"#(Player/Money/Increase or Decrease Player's money.[129])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} gets $${Integer})#",

},

{

    .index = "130",

    .id = "DISPLAY_COUNTDOWN_TIMER",

    .name = R"#(Interface/Counters and Timers/Specific timer -- display.[130])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Show ${CounterName} with text ${LocalizedStringName})#",

},

{

    .index = "131",

    .id = "HIDE_COUNTDOWN_TIMER",

    .name = R"#(Interface/Counters and Timers/Specific timer -- hide.[131])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Hide ${CounterName})#",

},

{

    .index = "132",

    .id = "ENABLE_COUNTDOWN_TIMER_DISPLAY",

    .name = R"#(Interface/Counters and Timers/All timers -- display.[132])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Enables timer display.)#",

},

{

    .index = "133",

    .id = "DISABLE_COUNTDOWN_TIMER_DISPLAY",

    .name = R"#(Interface/Counters and Timers/All timers -- hide.[133])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Disables timer display.)#",

},

{

    .index = "134",

    .id = "NAMED_STOP_SPECIAL_POWER_COUNTDOWN",

    .name = R"#(Object Type/Special Power/Countdown timer -- pause.[134])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Pause ${UnitName}'s ${SpecialPowerName} countdown.)#",

},

{

    .index = "135",

    .id = "NAMED_START_SPECIAL_POWER_COUNTDOWN",

    .name = R"#(Object Type/Special Power/Countdown timer -- resume.[135])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Resume ${UnitName}'s ${SpecialPowerName} countdown.)#",

},

{

    .index = "136",

    .id = "NAMED_SET_SPECIAL_POWER_COUNTDOWN",

    .name = R"#(Object Type/Special Power/Countdown timer -- set.[136])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set ${UnitName}'s ${SpecialPowerName} to ${Integer} seconds.)#",

},

{

    .index = "137",

    .id = "NAMED_FIRE_SPECIAL_POWER_AT_WAYPOINT",

    .name = R"#(Object Type/Special Power/Special Power -- fire at location.[137])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} fires ${SpecialPowerName} at ${WaypointName}.)#",

},

{

    .index = "138",

    .id = "CAMERA_TETHER_NAMED",

    .name = R"#(Cinematics/Camera/Begin tether camera to a specific unit.[138])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Have the camera tethered to ${UnitName}.  Snap camera to object is ${Boolean}.  Amount of play is ${RealNumber}.)#",

},

{

    .index = "139",

    .id = "CAMERA_STOP_TETHER_NAMED",

    .name = R"#(Cinematics/Camera/End tether to any units.[139])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Stop tether to any units.)#",

},

{

    .index = "140",

    .id = "NAMED_STOP",

    .name = R"#(Object Type/Move/Unit stops.[140])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} stops.)#",

},

{

    .index = "141",

    .id = "TEAM_STOP",

    .name = R"#(Team/Move/Team stops.[141])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} stops.)#",

},

{

    .index = "142",

    .id = "TEAM_STOP_AND_DISBAND",

    .name = R"#(Team/Move/Team stops, then disbands.[142])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} stops, then disbands.)#",

},

{

    .index = "143",

    .id = "RECRUIT_TEAM",

    .name = R"#(Unused or Obsolete/Team/Spawn, Build, or Merge/Recruit Team.[143])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Recruit an instance of ${TeamName}, maximum recruiting distance (feet):${RealNumber}.)#",

},

{

    .index = "144",

    .id = "RECRUIT_TEAM_AT_TEAM",

    .name = R"#(Unused or Obsolete/Team/Spawn, Build, or Merge/Recruit Team at location of another team.[144])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Recruit an instance of ${TeamName}, maximum recruiting distance (feet):${RealNumber} from the team ${TeamName}.)#",

},

{

    .index = "145",

    .id = "TEAM_SET_OVERRIDE_RELATION_TO_TEAM",

    .name = R"#(Team/Allegiance/Override Team's relationship to a Team.[145])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} considers ${TeamName} to be ${Relation} (rather than using the the player relationship).)#",

},

{

    .index = "146",

    .id = "TEAM_REMOVE_OVERRIDE_RELATION_TO_TEAM",

    .name = R"#(Team/Allegiance/Remove an override to Team's relationship to a Team.[146])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} uses the player relationship to ${TeamName})#",

},

{

    .index = "147",

    .id = "TEAM_REMOVE_ALL_OVERRIDE_RELATIONS",

    .name = R"#(Team/Allegiance/Remove all overrides to Team's relationship to Teams and Players.[147])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} uses the player relationship to all other teams and players.)#",

},

{

    .index = "148",

    .id = "CAMERA_LOOK_TOWARD_OBJECT",

    .name = R"#(Cinematics/Camera/Rotate toward unit.[148])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Rotate toward ${UnitName}, taking ${RealNumber} seconds and holding ${RealNumber} seconds, ease-in ${RealNumber} seconds, ease-out ${RealNumber} seconds with z-offset of ${RealNumber}.)#",

},

{

    .index = "149",

    .id = "TEAM_SET_OVERRIDE_RELATION_TO_PLAYER",

    .name = R"#(Team/Allegiance/Override Team's relationship to a Player.[149])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} considers ${PlayerName} to be ${Relation} (rather than using the the player relationship).)#",

},

{

    .index = "150",

    .id = "PLAYER_SET_OVERRIDE_RELATION_TO_TEAM",

    .name = R"#(Player/Allegiance/Override Player's relationship to another team.[150])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} considers ${TeamName} to be ${Relation} (rather than using the the player relationship).)#",

},

{

    .index = "151",

    .id = "UNIT_EXECUTE_SEQUENTIAL_SCRIPT",

    .name = R"#(Unused or Obsolete/Object Type/SequentialScript/Set a specific unit to execute a script sequentially.[151])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} executes ${ScriptName} sequentially.)#",

},

{

    .index = "152",

    .id = "UNIT_EXECUTE_SEQUENTIAL_SCRIPT_LOOPING",

    .name = R"#(Unused or Obsolete/Object Type/SequentialScript/Set a specific unit to execute a looping sequential script.[152])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} executes ${ScriptName} sequentially, ${Integer} times. (0=forever))#",

},

{

    .index = "153",

    .id = "UNIT_STOP_SEQUENTIAL_SCRIPT",

    .name = R"#(Object Type/SequentialScript/Set a named object to stop executing a sequential script.[153])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} stops executing.)#",

},

{

    .index = "154",

    .id = "TEAM_EXECUTE_SEQUENTIAL_SCRIPT",

    .name = R"#(Unused or Obsolete/Team/SequentialScript/Execute script sequentially -- start.[154])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} executes ${ScriptName} sequentially.)#",

},

{

    .index = "155",

    .id = "TEAM_EXECUTE_SEQUENTIAL_SCRIPT_LOOPING",

    .name = R"#(Unused or Obsolete/Team/SequentialScript/Execute script sequentially -- looping.[155])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} executes ${ScriptName} sequentially, ${Integer} times. (0=forever))#",

},

{

    .index = "156",

    .id = "TEAM_STOP_SEQUENTIAL_SCRIPT",

    .name = R"#(Team/SequentialScript/Execute script sequentially -- stop.[156])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} stops executing.)#",

},

{

    .index = "157",

    .id = "UNIT_GUARD_FOR_FRAMECOUNT",

    .name = R"#(Unused or Obsolete/Object Type/SequentialScript/Set to guard for some number of frames.[157])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} guards for ${Integer} frames.)#",

},

{

    .index = "158",

    .id = "UNIT_IDLE_FOR_FRAMECOUNT",

    .name = R"#(Object Type/SequentialScript/Named object idles for some number of frames.[158])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} idles for ${Integer} frames.)#",

},

{

    .index = "159",

    .id = "TEAM_GUARD_FOR_FRAMECOUNT",

    .name = R"#(Unused or Obsolete/Team/SequentialScript/Guard -- number of frames.[159])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} guards for ${Integer} frames.)#",

},

{

    .index = "160",

    .id = "TEAM_IDLE_FOR_FRAMECOUNT",

    .name = R"#(Team/SequentialScript/Idle for some number of frames.[160])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} idles for ${Integer} frames.)#",

},

{

    .index = "162",

    .id = "NAMED_USE_COMMANDBUTTON_ABILITY_AT_WAYPOINT",

    .name = R"#(Object Type/CommandButton/Use commandbutton ability at a waypoint.[162])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} use ${UnitAbilityName} at ${WaypointName}.)#",

},

{

    .index = "163",

    .id = "MAP_SWITCH_BORDER",

    .name = R"#(Map/Modify/Change the active border of the map.[163])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${BoundaryName} becomes the active border.)#",

},

{

    .index = "164",

    .id = "TEAM_GUARD_POSITION",

    .name = R"#(Team/Guard/Team guard at location.[164])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} begins guarding at ${WaypointName})#",

},

{

    .index = "165",

    .id = "TEAM_GUARD_OBJECT",

    .name = R"#(Team/Guard/Team guard a named object.[165])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} begins guarding ${UnitName})#",

},

{

    .index = "166",

    .id = "TEAM_GUARD_AREA",

    .name = R"#(Team/Guard/Team guard an area.[166])#",

    .help = R"#(Please note that units guarding an area that is fogged will not be able to see a unit that enters the area and thus not attack so be sure that the area they are guarding is not fogged.)#",

    .argHint = R"#( ${TeamName} begins guarding ${TriggerAreaName})#",

},

{

    .index = "167",

    .id = "CAMERA_LOOK_TOWARD_WAYPOINT",

    .name = R"#(Cinematics/Camera/Rotate to look at a waypoint.[167])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Rotate to look at ${WaypointName}, taking ${RealNumber} seconds, ease-in ${RealNumber} seconds, ease-out ${RealNumber} seconds, reverse rotation ${Boolean}.)#",

},

{

    .index = "168",

    .id = "UNIT_DESTROY_ALL_CONTAINED",

    .name = R"#(Object Type/Damage or Remove/Kill all units contained within a specific transport or structure.[168])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(All units inside ${UnitName} are killed.)#",

},

{

    .index = "169",

    .id = "SCREEN_SHAKE",

    .name = R"#(Cinematics/Camera/Shake Screen.[169])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(The screen will shake with ${CameraShakeIntensity})#",

},

{

    .index = "170",

    .id = "TECHTREE_MODIFY_BUILDABILITY_OBJECT",

    .name = R"#(Player/Build/Adjust the tech tree for a specific object type.[170])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${ObjectName} becomes ${Buildability})#",

},

{

    .index = "172",

    .id = "OBJECT_CREATE_RADAR_EVENT",

    .name = R"#(Interface/Radar/Create a radar event at a named object.[172])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(A radar event occurs at ${UnitName} of type ${RadarEventType})#",

},

{

    .index = "173",

    .id = "TEAM_CREATE_RADAR_EVENT",

    .name = R"#(Interface/Radar/Create a radar event at a Team.[173])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(A radar event occurs at ${TeamName} of type ${RadarEventType})#",

},

{

    .index = "175",

    .id = "SOUND_DISABLE_TYPE",

    .name = R"#(Media/Audio/Sound Events/Sound Events -- disable type.[175])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${AudioName} is disabled.)#",

},

{

    .index = "176",

    .id = "SOUND_ENABLE_TYPE",

    .name = R"#(Media/Audio/Sound Events/Sound Events -- enable type.[176])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${AudioName} is enabled.)#",

},

{

    .index = "177",

    .id = "SOUND_ENABLE_ALL",

    .name = R"#(Media/Audio/Sound Events/Sound Events -- enable all.[177])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Enable all sound events.)#",

},

{

    .index = "178",

    .id = "AUDIO_OVERRIDE_VOLUME_TYPE",

    .name = R"#(Unused or Obsolete/Media/Audio/Sound Events/Sound Events -- override volume -- type.[178])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${AudioName} play at ${RealNumber}% of full volume.)#",

},

{

    .index = "179",

    .id = "AUDIO_RESTORE_VOLUME_TYPE",

    .name = R"#(Unused or Obsolete/Media/Audio/Sound Events/Sound Events -- restore volume -- type.[179])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${AudioName} play at normal volume.)#",

},

{

    .index = "180",

    .id = "AUDIO_RESTORE_VOLUME_ALL_TYPE",

    .name = R"#(Unused or Obsolete/Media/Audio/Sound Events/Sound Events -- restore volume -- all.[180])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(All sound events play at normal volume.)#",

},

{

    .index = "181",

    .id = "NAMED_SET_HELD",

    .name = R"#(Unused or Obsolete/Object Type/Settings/Set unit to be held in place, ignoring Physics, Locomotors, etc.[181])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set Held status for ${UnitName} to ${Boolean}.)#",

},

{

    .index = "182",

    .id = "NAMED_SET_TOPPLE_DIRECTION",

    .name = R"#(Object Type/Settings/Set topple direction.[182])#",

    .help = R"#(Topples towards the specified coordinate.)#",

    .argHint = R"#( ${UnitName} will topple towards ${PositionCoordinate} if destroyed.)#",

},

{

    .index = "183",

    .id = "MAP_REVEAL_ALL_PERM",

    .name = R"#(Map/Fog or Reveal/Reveal the entire map permanently for Player.[183])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(The world is revealed permanently for ${PlayerName}.)#",

},

{

    .index = "184",

    .id = "MAP_REVEAL_ALL_UNDO_PERM",

    .name = R"#(Map/Fog or Reveal/Un-Reveal the entire map permanently for Player.[184])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Undo the permanent reveal for ${PlayerName}.  This will mess things up badly if called when there has been no permanent reveal.)#",

},

{

    .index = "185",

    .id = "TEAM_WANDER_IN_PLACE",

    .name = R"#(Team/Move/Set to wander around current location.[185])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Have ${TeamName} wander around it's current location.)#",

},

{

    .index = "186",

    .id = "TEAM_INCREASE_PRIORITY",

    .name = R"#(Unused or Obsolete/Team/Spawn, Build, or Merge/Change priority by Success Priority Increase amount.[186])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Increase the AI priority for${TeamName}  by its Success Priority Increase amount.)#",

},

{

    .index = "187",

    .id = "TEAM_DECREASE_PRIORITY",

    .name = R"#(Unused or Obsolete/Team/Spawn, Build, or Merge/Change priority by Failure Priority Decrease amount.[187])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Reduce the AI priority for${TeamName}  by its Failure Priority Decrease amount.)#",

},

{

    .index = "188",

    .id = "DISPLAY_COUNTER",

    .name = R"#(Interface/Counters and Timers/Specific counter -- display.[188])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Show ${CounterName} with text ${LocalizedStringName})#",

},

{

    .index = "189",

    .id = "HIDE_COUNTER",

    .name = R"#(Interface/Counters and Timers/Specific counter -- hide.[189])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Hide ${CounterName})#",

},

{

    .index = "191",

    .id = "TEAM_USE_COMMANDBUTTON_ABILITY_AT_WAYPOINT",

    .name = R"#(Team/CommandButton/Use commandbutton ability at a waypoint.[191])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} use ${TeamAbilityName} at ${WaypointName}.)#",

},

{

    .index = "192",

    .id = "NAMED_USE_COMMANDBUTTON_ABILITY",

    .name = R"#(Object Type/CommandButton/Use commandbutton ability.[192])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} use ${UnitAbilityName}.)#",

},

{

    .index = "193",

    .id = "TEAM_USE_COMMANDBUTTON_ABILITY",

    .name = R"#(Team/CommandButton/Use commandbutton ability.[193])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} use ${TeamAbilityName}.)#",

},

{

    .index = "194",

    .id = "NAMED_FLASH_WHITE",

    .name = R"#(Object Type/Flash or Color/Flash a named object white for a specified amount of time.[194])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} flashes white for ${Integer} seconds.)#",

},

{

    .index = "195",

    .id = "TEAM_FLASH_WHITE",

    .name = R"#(Team/Flash/Flash Team white for a specified amount of time.[195])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} flashes white for ${Integer} seconds.)#",

},

{

    .index = "196",

    .id = "SKIRMISH_FOLLOW_APPROACH_PATH",

    .name = R"#(Unused or Obsolete/Team/Move/Team follow approach path.[196])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Have ${TeamName} approach the enemy using path ${SkirmishApproachPath}, as a team is ${Boolean})#",

},

{

    .index = "197",

    .id = "IDLE_ALL_UNITS",

    .name = R"#(Cinematics/Force Emotion/Idle all units for all players.[197])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Idle all units for all players.)#",

},

{

    .index = "198",

    .id = "NAMED_CUSTOM_COLOR",

    .name = R"#(Object Type/Flash or Color/Set a named object to use a special indicator color.[198])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} uses the color ${Color} .)#",

},

{

    .index = "199",

    .id = "SKIRMISH_MOVE_TO_APPROACH_PATH",

    .name = R"#(Unused or Obsolete/Team/Move/Team move to approach path.[199])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Have ${TeamName} move to the start of enemy path ${SkirmishApproachPath}.)#",

},

{

    .index = "200",

    .id = "SKIRMISH_FIRE_SPECIAL_POWER_AT_MOST_COST",

    .name = R"#(Player/Special Power/Special power -- fire at enemy's highest cost area.[200])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} fire ${SpecialPowerName} at enemy's most costly area.)#",

},

{

    .index = "201",

    .id = "NAMED_RECEIVE_UPGRADE",

    .name = R"#(Object Type/Settings/Give upgrade to named object.[201])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Give ${UpgradeName} to ${UnitName}.)#",

},

{

    .index = "204",

    .id = "TEAM_ALL_USE_COMMANDBUTTON_ON_NEAREST_ENEMY_UNIT",

    .name = R"#(Team/CommandButton/Use command ability -- all -- nearest enemy unit[204])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} use ${TeamAbilityName}  on nearest enemy unit.)#",

},

{

    .index = "205",

    .id = "TEAM_ALL_USE_COMMANDBUTTON_ON_NEAREST_GARRISONED_BUILDING",

    .name = R"#(Team/CommandButton/Use command ability -- all -- nearest enemy garrisoned building.[205])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} use ${TeamAbilityName}  on nearest enemy garrisoned building.)#",

},

{

    .index = "206",

    .id = "TEAM_ALL_USE_COMMANDBUTTON_ON_NEAREST_KINDOF",

    .name = R"#(Team/CommandButton/Use command ability -- all -- nearest enemy object with kind of.[206])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} use ${TeamAbilityName}  on nearest enemy with ${UnitOrStructureKind})#",

},

{

    .index = "207",

    .id = "TEAM_ALL_USE_COMMANDBUTTON_ON_NEAREST_ENEMY_BUILDING",

    .name = R"#(Team/CommandButton/Use command ability -- all -- nearest enemy building.[207])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} use ${TeamAbilityName}  on nearest enemy building.)#",

},

{

    .index = "208",

    .id = "TEAM_ALL_USE_COMMANDBUTTON_ON_NEAREST_ENEMY_BUILDING_CLASS",

    .name = R"#(Team/CommandButton/Use command ability -- all -- nearest enemy building kindof.[208])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} use ${TeamAbilityName}  on nearest enemy building with ${UnitOrStructureKind})#",

},

{

    .index = "209",

    .id = "TEAM_ALL_USE_COMMANDBUTTON_ON_NEAREST_OBJECTTYPE",

    .name = R"#(Team/CommandButton/Use command ability -- all -- nearest object type.[209])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} use ${TeamAbilityName} on nearest object of type ${ObjectName}.)#",

},

{

    .index = "210",

    .id = "TEAM_PARTIAL_USE_COMMANDBUTTON",

    .name = R"#(Team/CommandButton/Use command ability -- partial -- self.[210])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${RealNumber}% of ${TeamName} perform ${TeamAbilityName}.)#",

},

{

    .index = "211",

    .id = "TEAM_CAPTURE_NEAREST_UNOWNED_FACTION_UNIT",

    .name = R"#(Team/Capture/Capture unowned faction unit -- nearest.[211])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} capture the nearest unowned faction unit.)#",

},

{

    .index = "213",

    .id = "TEAM_FOLLOW_WAYPOINTS_EXACT",

    .name = R"#(Team/Move/Set to EXACTLY follow a waypoint path.[213])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Have ${TeamName} EXACTLY follow ${WaypointPathName} , as a team is ${Boolean})#",

},

{

    .index = "214",

    .id = "NAMED_FOLLOW_WAYPOINTS_EXACT",

    .name = R"#(Object Type/Move/Named object follow a waypoint path -- exactly.[214])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} EXACTLY follow ${WaypointPathName})#",

},

{

    .index = "215",

    .id = "AI_PLAYER_BUILD_UPGRADE",

    .name = R"#(Unused or Obsolete/Player/Upgrades and Sciences/AI Player build an Upgrade.[215])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Have AI ${PlayerName} build this upgrade: ${UpgradeName})#",

},

{

    .index = "216",

    .id = "OBJECTLIST_ADDOBJECTTYPE",

    .name = R"#(Scripting/Object List/Add Object Type to Object List.[216])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Add ${ObjectName} to ${ObjectTypeListName})#",

},

{

    .index = "217",

    .id = "OBJECTLIST_REMOVEOBJECTTYPE",

    .name = R"#(Scripting/Object List/Remove Object Type from Object List.[217])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Remove ${ObjectName} from ${ObjectTypeListName})#",

},

{

    .index = "218",

    .id = "MAP_REVEAL_PERMANENTLY_AT_WAYPOINT",

    .name = R"#(Map/Fog or Reveal/Reveal map at waypoint -- permanently.[218])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(The map is permanently revealed at ${WaypointName} with a radius of ${RealNumber} for ${PlayerName}. (Afterwards referred to as ${MapRevealName}).)#",

},

{

    .index = "219",

    .id = "MAP_UNDO_REVEAL_PERMANENTLY_AT_WAYPOINT",

    .name = R"#(Map/Fog or Reveal/Reveal map at waypoint -- undo permanently.[219])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${MapRevealName} is undone.)#",

},

{

    .index = "220",

    .id = "NAMED_SET_STEALTH_ENABLED",

    .name = R"#(Object Type/Settings/Stealth -- enable or disable.[220])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set ${UnitName} stealth ability to ${Boolean}.)#",

},

{

    .index = "221",

    .id = "EVA_SET_ENABLED_DISABLED",

    .name = R"#(Media/Audio/EVA/Enable or Disable EVA.[221])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set EVA to be enabled ${Boolean} (False to disable.))#",

},

{

    .index = "222",

    .id = "OPTIONS_SET_OCCLUSION_MODE",

    .name = R"#(Cinematics/Settings/Enable or Disable Occlusion (Drawing Behind Buildings).[222])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set Occlusion to be enabled ${Boolean} (False to disable.))#",

},

{

    .index = "223",

    .id = "LOCALDEFEAT",

    .name = R"#(Win & Loss/Announce local defeat.[223])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Show 'Game Over' window)#",

},

{

    .index = "224",

    .id = "OPTIONS_SET_DRAWICON_UI_MODE",

    .name = R"#(Cinematics/Settings/Enable or Disable Draw-icon UI.[224])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set Draw-icon UI to be enabled ${Boolean} (False to disable.))#",

},

{

    .index = "225",

    .id = "OPTIONS_SET_PARTICLE_CAP_MODE",

    .name = R"#(Cinematics/Settings/Enable or Disable Particle Cap.[225])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set Particle Cap to be enabled ${Boolean} (False to disable.))#",

},

{

    .index = "226",

    .id = "UNIT_AFFECT_OBJECT_PANEL_FLAGS",

    .name = R"#(Object Type/Settings/Change Object flags.[226])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} changes the value of ${ObjectPanelFlag} to ${Boolean}.)#",

},

{

    .index = "227",

    .id = "TEAM_AFFECT_OBJECT_PANEL_FLAGS",

    .name = R"#(Team/Settings/Change Object flags - all.[227])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} change the value of ${ObjectPanelFlag} to ${Boolean}.)#",

},

{

    .index = "228",

    .id = "PLAYER_SELECT_SKILLSET",

    .name = R"#(Unused or Obsolete/Player/Upgrades and Sciences/Set the skillset for an AI Player.[228])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} uses skillset number ${Integer} (1-5).)#",

},

{

    .index = "229",

    .id = "SCRIPTING_OVERRIDE_HULK_LIFETIME",

    .name = R"#(Object Type/Hulk set override lifetime.[229])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Override hulk lifetime to ${RealNumber} seconds. Negative value reverts to normal behavior.)#",

},

{

    .index = "231",

    .id = "NAMED_FACE_WAYPOINT",

    .name = R"#(Object Type/Move/Face a waypoint.[231])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} begin facing ${WaypointName})#",

},

{

    .index = "233",

    .id = "TEAM_FACE_WAYPOINT",

    .name = R"#(Team/Move/Face a waypoint.[233])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} begin facing ${WaypointName})#",

},

{

    .index = "235",

    .id = "UNIT_SPAWN_NAMED_LOCATION_ORIENTATION",

    .name = R"#(Unused or Obsolete/Object Type/Spawn/Spawn -- named object on Team at a position with an orientation.[235])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Spawn ${UnitName} of type ${ObjectName} on ${TeamName} at position (${PositionCoordinate}), rotated ${Angle} .)#",

},

{

    .index = "236",

    .id = "PLAYER_EXCLUDE_FROM_SCORE_SCREEN",

    .name = R"#(Player/Scoring/Exclude this player from the score screen.[236])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Exclude ${PlayerName} from the score screen.)#",

},

{

    .index = "237",

    .id = "TEAM_GUARD_SUPPLY_CENTER",

    .name = R"#(Team/Guard/Guard a supply source.[237])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Have ${TeamName} guard attacked or closest supply source with at least ${Integer} available resources)#",

},

{

    .index = "238",

    .id = "ENABLE_SCORING",

    .name = R"#(Unused or Obsolete/Map/Gameplay/Turn on scoring.[238])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Turn on scoring.)#",

},

{

    .index = "239",

    .id = "DISABLE_SCORING",

    .name = R"#(Unused or Obsolete/Map/Gameplay/Turn off scoring.[239])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Turn off scoring.)#",

},

{

    .index = "240",

    .id = "SOUND_SET_VOLUME",

    .name = R"#(Media/Audio/Volume/Set the current sound volume.[240])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set the desired sound volume to ${RealNumber}%. (0-100))#",

},

{

    .index = "241",

    .id = "SPEECH_SET_VOLUME",

    .name = R"#(Unused or Obsolete/Media/Audio/Volume/Set the current speech volume.[241])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set the desired speech volume to ${RealNumber}%. (0-100))#",

},

{

    .index = "242",

    .id = "DISABLE_BORDER_SHROUD",

    .name = R"#(Map/Fog or Reveal/Border Shroud is turned off.[242])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Shroud off the map edges is turned off.)#",

},

{

    .index = "243",

    .id = "ENABLE_BORDER_SHROUD",

    .name = R"#(Map/Fog or Reveal/Border Shroud is turned on.[243])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Shroud off the map edges is turned on.)#",

},

{

    .index = "244",

    .id = "OBJECT_ALLOW_BONUSES",

    .name = R"#(Map/Gameplay/Adjust Object Bonuses based on difficulty.[244])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Enable Object Bonuses based on difficulty ${Boolean} (true to enable, false to disable).)#",

},

{

    .index = "245",

    .id = "SOUND_REMOVE_TYPE",

    .name = R"#(Unused or Obsolete/Media/Audio/Sound Events/Sound Events -- remove type.[245])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${AudioName} is removed.)#",

},

{

    .index = "246",

    .id = "TEAM_GUARD_IN_TUNNEL_NETWORK",

    .name = R"#(Unused or Obsolete/Team/Guard/Guard -- from inside tunnel network.[246])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} Enter and guard from tunnel network.)#",

},

{

    .index = "247",

    .id = "QUICKVICTORY",

    .name = R"#(Win & Loss/Announce quick win[247])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(End game in victory immediately.)#",

},

{

    .index = "248",

    .id = "UNIT_SET_MODELCONDITION",

    .name = R"#(Object Type/Settings/Change Custom Model Condition flag.[248])#",

    .help = R"#(Sets the USER_x model state up to 10 currently on a given object.)#",

    .argHint = R"#( ${UnitName} sets custom model condition number (1-10) ${Integer} to ${Boolean}.)#",

},

{

    .index = "249",

    .id = "TEAM_SET_MODELCONDITION",

    .name = R"#(Team/Settings/Change Custom Model Condition flag - all.[249])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} sets custom model condition number (1-4) ${Integer} to ${Boolean}.)#",

},

{

    .index = "250",

    .id = "TEAM_DELETE_LIVING",

    .name = R"#(Team/Damage or Remove/Delete Team, but ignore dead guys.[250])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Each living member of ${TeamName} is removed from the world. (Dead or dying members will remain))#",

},

{

    .index = "251",

    .id = "DELETE_ALL_UNMANNED",

    .name = R"#(Object Type/Delete all unmanned (sniped) vehicles.[251])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Delete all unmanned (sniped) vehicles.)#",

},

{

    .index = "252",

    .id = "CHOOSE_VICTIM_ALWAYS_USES_NORMAL",

    .name = R"#(Unused or Obsolete/Map/Gameplay/Force ChooseVictim to ignore game difficulty and always use Normal setting.[252])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Force ChooseVictim to ignore game difficulty and always use Normal setting ${Boolean} (true to enable, false to disable).)#",

},

{

    .index = "253",

    .id = "SET_CAMERA_CLIP_DEPTH_MULTIPLIER",

    .name = R"#(Cinematics/Camera/Change the camera depth of field.[253])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Modify normal depth of Frustrum by ${RealNumber} (1.0 = normal, < 1.0 shortens frustrum, > 1.0 lengthens.))#",

},

{

    .index = "254",

    .id = "CAMERA_ADD_SHAKER_AT",

    .name = R"#(Cinematics/Camera/Add Camera Shaker Effect at.[254])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Add Camera Shaker Effect at waypoint ${WaypointName} with Amplitude ${RealNumber} Duration (seconds) ${RealNumber} Radius.${RealNumber})#",

},

{

    .index = "255",

    .id = "ENABLE_OBJECT_SOUND",

    .name = R"#(Unused or Obsolete/Media/Audio/Sound Effect/Enable object's ambient sound[255])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Enable (or trigger) ${UnitName}'s ambient sound.)#",

},

{

    .index = "256",

    .id = "DISABLE_OBJECT_SOUND",

    .name = R"#(Unused or Obsolete/Media/Audio/Sound Effect/Disable object's ambient sound[256])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Disable ${UnitName}'s ambient sound.)#",

},

{

    .index = "257",

    .id = "ROTATE_CAMERA_LOCKED",

    .name = R"#(Cinematics/Camera/Rotate the camera in place.[257])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Rotate in place ${RealNumber} times, taking ${RealNumber} seconds, ease-in ${RealNumber} seconds, ease-out ${RealNumber} seconds.)#",

},

{

    .index = "258",

    .id = "CAMERA_BLOOM_EFFECT_BEGIN",

    .name = R"#(Cinematics/Lighting/Begin bloom lighting effect.[258])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set Bloom Filter to be enabled)#",

},

{

    .index = "259",

    .id = "CAMERA_BLOOM_EFFECT_END",

    .name = R"#(Cinematics/Lighting/End bloom lighting effect.[259])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set Bloom Filter to be disabled)#",

},

{

    .index = "260",

    .id = "UNIT_SET_FLAME_STATUS",

    .name = R"#(Unused or Obsolete/Object Type/Settings/Ignite or quench aflame status.[260])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} is now on fire (T-F) ${Boolean} .)#",

},

{

    .index = "261",

    .id = "TEAM_SET_FLAME_STATUS",

    .name = R"#(Unused or Obsolete/Team/Settings/Ignite or quench aflame status - all.[261])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} is now on fire (T-F) ${Boolean} .)#",

},

{

    .index = "262",

    .id = "ROLL_CAMERA",

    .name = R"#(Cinematics/Camera/Change the camera roll.[262])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Change camera roll to ${RealNumber} degrees in ${RealNumber} seconds, ease-in ${RealNumber} seconds, ease-out ${RealNumber} seconds.)#",

},

{

    .index = "263",

    .id = "HIDE_UI",

    .name = R"#(Cinematics/Settings/Hide UI[263])#",

    .help = R"#(Like letterbox mode, but without the borders)#",

    .argHint = R"#(Hide UI (like letterbox mode, but with no border).)#",

},

{

    .index = "264",

    .id = "SHOW_UI",

    .name = R"#(Cinematics/Settings/Show UI[264])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Show UI)#",

},

{

    .index = "265",

    .id = "TEAM_GIVE_EXPERIENCE_LEVEL",

    .name = R"#(Unused or Obsolete/Team/Experience/Gives the team an experience level.[265])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Gives ${TeamName} experience level ${ExperienceLevel}.)#",

},

{

    .index = "266",

    .id = "TEAM_GIVE_EXPERIENCE_POINTS",

    .name = R"#(Team/Experience/Gives the team experience points.[266])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Gives ${TeamName} experience points ${Integer}.)#",

},

{

    .index = "267",

    .id = "TEAM_SET_EXPERIENCE_POINTS",

    .name = R"#(Team/Experience/Sets the team's experience points.[267])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Sets ${TeamName} experience points' to ${Integer}.)#",

},

{

    .index = "268",

    .id = "UNIT_GIVE_EXPERIENCE_LEVEL",

    .name = R"#(Unused or Obsolete/Object Type/Experience/Gives a unit an experience level.[268])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Gives ${UnitName} experience level ${ExperienceLevel}.)#",

},

{

    .index = "269",

    .id = "UNIT_GIVE_EXPERIENCE_POINTS",

    .name = R"#(Object Type/Experience/Gives a named object experience points.[269])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Gives ${UnitName} experience points ${Integer}.)#",

},

{

    .index = "270",

    .id = "UNIT_SET_EXPERIENCE_POINTS",

    .name = R"#(Object Type/Experience/Sets a named object's experience points.[270])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Sets ${UnitName} experience points to ${Integer}.)#",

},

{

    .index = "272",

    .id = "PLAY_MOVIE_IN_GAME",

    .name = R"#(Unused or Obsolete/PlayMovie/Plays a movie in game.[272])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Play Movie named ${MovieName} allow cancel ${Boolean} .)#",

},

{

    .index = "273",

    .id = "MAP_CHANGE_CLOUD_SPEED",

    .name = R"#(Unused or Obsolete/Map/Modify/Change the speed of the CloudEffect.[273])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Modify the default CloudEffect speed to be ${Percentage} )#",

},

{

    .index = "274",

    .id = "TERRAIN_RENDER_DISABLE",

    .name = R"#(Unused or Obsolete/Camera/Terrain/Enable or Disable Terrain Render.[274])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Disable terrain render ${Boolean} (True to disable.))#",

},

{

    .index = "278",

    .id = "SET_UNIT_REFERENCE",

    .name = R"#(Unused or Obsolete/Scripting/Reference/Set reference to a unit.[278])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set ${UnitReference} to reference unit ${UnitName})#",

},

{

    .index = "279",

    .id = "SET_TEAM_REFERENCE",

    .name = R"#(Unused or Obsolete/Scripting/Reference/Set reference to a team.[279])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set ${TeamReference} to reference team ${TeamName})#",

},

{

    .index = "280",

    .id = "SET_UNIT_REFERENCE_TO_REFERENCE",

    .name = R"#(Unused or Obsolete/Scripting/Reference/Set reference to another unit reference.[280])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set ${UnitReference} to reference another unit reference ${UnitReference})#",

},

{

    .index = "281",

    .id = "SET_TEAM_REFERENCE_TO_REFERNECE",

    .name = R"#(Unused or Obsolete/Scripting/Reference/ Set reference to another team reference.[281])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set ${TeamReference} to reference another team reference ${TeamReference})#",

},

{

    .index = "282",

    .id = "TEAM_GIVE_NEAREST_TEAM_UPGRADE",

    .name = R"#(Unused or Obsolete/Team/Upgrades/Team will give nearest team upgrade (Giving Team must have upgrade)[282])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} will give nearest team upgrade)#",

},

{

    .index = "283",

    .id = "ALLOW_DISALLOW_ONE_BUILDING",

    .name = R"#(Player/Build/Allow or Disallow one building of a specific type[283])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} allowed to build ${ObjectName} is ${Boolean}.)#",

},

{

    .index = "284",

    .id = "ALLOW_DISALLOW_ALL_BUILDINGS",

    .name = R"#(Unused or Obsolete/Base/Allow or Disallow all buildings[284])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} is allowed to build ALL buildings is ${Boolean} . )#",

},

{

    .index = "285",

    .id = "FOCAL_LENGTH_CAMERA",

    .name = R"#(Cinematics/Camera/Change the camera's focal length (mm).[285])#",

    .help = R"#(Typical values range from 10mm (wide) to 400mm (extreme zoom). Normal camera is 25mm)#",

    .argHint = R"#(Change camera focal length to ${RealNumber}mm in ${RealNumber} seconds, ease-in ${RealNumber} seconds, ease-out ${RealNumber} seconds.)#",

},

{

    .index = "286",

    .id = "MOVE_CAMERA_LOCATOR_ALONG_SPLINE_PATH",

    .name = R"#(Cinematics/Camera/Order camera's locator to follow a spline path.[286])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Look at point moving along ${WaypointPathName} in ${RealNumber} seconds, camera shutter ${RealNumber} seconds, ease-in ${RealNumber} seconds, ease-out ${RealNumber} seconds, delay start ${RealNumber} seconds. )#",

},

{

    .index = "287",

    .id = "TEAM_MOVE_TO_NEAREST_OBJECT_OF_KINDOF",

    .name = R"#(Team/Move/Team will move to the nearest object that matches kindof specified.[287])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} will move to the nearest object of ${UnitOrStructureKind}.)#",

},

{

    .index = "288",

    .id = "TEAM_MOVE_TO_NEAREST_OBJECT_OF_KINDOF_OWNED_BY_PLAYER",

    .name = R"#(Team/Move/Team will move to the nearest object that matches both kindof and player specified.[288])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} will move to the nearest object of ${UnitOrStructureKind} that is owned by the ${PlayerName}.)#",

},

{

    .index = "289",

    .id = "TEAM_MOVE_TO_NEAREST_OBJECT_OF_TYPE",

    .name = R"#(Team/Move/Team will move to the nearest object that matches type specified.[289])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} will move to the nearest object of type ${ObjectType}.)#",

},

{

    .index = "290",

    .id = "TEAM_MOVE_TO_NEAREST_OBJECT_OF_TYPE_OWNED_BY_PLAYER",

    .name = R"#(Team/Move/Team will move to the nearest object that matches both type and player specified.[290])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} will move to the nearest object of type ${ObjectType} that is owned by ${PlayerName}.)#",

},

{

    .index = "291",

    .id = "COUNTER_MATH_COUNTER",

    .name = R"#(Scripting/Counters/Counter do math opertation with another Counter.[291])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Counter ${CounterName} , ${MathOperator} with counter ${CounterName})#",

},

{

    .index = "292",

    .id = "COUNTER_MATH_VALUE",

    .name = R"#(Scripting/Counters/Counter do math opertation with value.[292])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Counter ${CounterName} , ${MathOperator} with value ${Integer})#",

},

{

    .index = "293",

    .id = "TEAM_RECRUIT_UNITS",

    .name = R"#(Unused or Obsolete/Team/Capture-Recruit/Team recruits from same player and nearby allied AI teams.[293])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} will recruit ${Integer} units of type ${ObjectType} from nearby recruitable allied teams.)#",

},

{

    .index = "294",

    .id = "TEAM_RECRUIT_UNITS_FROM_TEAM",

    .name = R"#(Unused or Obsolete/Team/Capture-Recruit/Team recruits from another specific team.[294])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} will recruit ${Integer} units of type ${ObjectType} from ${TeamName} .)#",

},

{

    .index = "295",

    .id = "TEAM_GUARD_TEAM",

    .name = R"#(Team/Guard/Team guard a specific team.[295])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} begins guarding ${TeamName})#",

},

{

    .index = "296",

    .id = "TEAM_GUARD_NEAREST_KINDOF",

    .name = R"#(Team/Guard/Team guard nearest kindof.[296])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} begins guarding nearest ${UnitOrStructureKind})#",

},

{

    .index = "297",

    .id = "UNIT_GUARD_NEAREST_KINDOF",

    .name = R"#(Object Type/Guard/Named object guards nearest kindof.[297])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} begins guarding nearest ${UnitOrStructureKind})#",

},

{

    .index = "298",

    .id = "TEAM_GIVE_TEAM_UPGRADE",

    .name = R"#(Unused or Obsolete/Team/Upgrades/Team will give team upgrade (Giving Team must have upgrade)[298])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} will give ${TeamName} upgrade )#",

},

{

    .index = "299",

    .id = "TEAM_UPGRADE",

    .name = R"#(Team/Upgrades/Give Team the specified upgrade[299])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Give Team ${TeamName} the upgrade ${UpgradeName}.)#",

},

{

    .index = "300",

    .id = "SET_PLAYER_MONEY_TO_COUNTER",

    .name = R"#(Scripting/Counters/Set players money to counter.[300])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Store amount of  ${PlayerName} money to the counter ${CounterName})#",

},

{

    .index = "301",

    .id = "SET_UNIT_EXPERIENCE_TO_COUNTER",

    .name = R"#(Scripting/Counters/Set units experience points to counter.[301])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Store amount of ${UnitName} experience points to the counter ${CounterName})#",

},

{

    .index = "302",

    .id = "SET_PLAYER_COMMAND_POINTS_USED_TO_COUNTER",

    .name = R"#(ThreeSixty/Set command points used by player to counter.[302])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Store amount of command points used by player ${PlayerName} to the counter ${CounterName})#",

},

{

    .index = "303",

    .id = "SET_PLAYER_COMMAND_POINTS_AVAILABLE_TO_COUNTER",

    .name = R"#(ThreeSixty/Set command points available to player to counter.[303])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Store amount of command points available to player ${PlayerName} to the counter ${CounterName})#",

},

{

    .index = "304",

    .id = "SET_PLAYER_COMMAND_POINTS_TOTAL_TO_COUNTER",

    .name = R"#(ThreeSixty/Set command points total of player to counter.[304])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Store total number of command point of player ${PlayerName} to the counter ${CounterName})#",

},

{

    .index = "305",

    .id = "SET_COMMAND_POINTS_TO_BUILD_TEAM_TO_COUNTER",

    .name = R"#(ThreeSixty/Set command points required to build team to counter.[305])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Store the amount of command points required to build ${TeamName} to the counter ${CounterName})#",

},

{

    .index = "306",

    .id = "UNIT_GUARD_FOR_SECONDS",

    .name = R"#(Unused or Obsolete/Object Type/SequentialScript/Set to guard for some number of seconds.[306])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} guards for ${Integer} seconds.)#",

},

{

    .index = "307",

    .id = "UNIT_IDLE_FOR_SECONDS",

    .name = R"#(Object Type/SequentialScript/Named object idles for some number of seconds.[307])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} idles for ${Integer} seconds.)#",

},

{

    .index = "308",

    .id = "TEAM_GUARD_FOR_SECONDS",

    .name = R"#(Unused or Obsolete/Team/SequentialScript/Guard -- number of seconds.[308])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} guards for ${Integer} seconds.)#",

},

{

    .index = "309",

    .id = "TEAM_IDLE_FOR_SECONDS",

    .name = R"#(Team/SequentialScript/Idle for some number of seconds.[309])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} idles for ${Integer} seconds.)#",

},

{

    .index = "312",

    .id = "AUDIO_FADE_VOLUME",

    .name = R"#(Media/Audio/Volume/Fade all audio[312])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Fade (0-1) all audio from volume ${RealNumber} to volume ${RealNumber}. Take ${RealNumber} seconds to increase, hold for ${RealNumber} seconds, and decrease ${RealNumber} seconds.)#",

},

{

    .index = "313",

    .id = "TOGGLE_AUTO_BUILD",

    .name = R"#(Unused or Obsolete/Player/Build/Turn on or off a player's structures that auto build units.  (Starts ON)[313])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} toggle auto build ${Boolean})#",

},

{

    .index = "314",

    .id = "GIVE_PLAYER_UPGRADE",

    .name = R"#(Player/Upgrades and Sciences/Give Player an Upgrade.[314])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Give ${PlayerName} this upgrade: ${UpgradeName})#",

},

{

    .index = "315",

    .id = "SET_PLAYER_KILLS_OF_TYPE_TO_COUNTER",

    .name = R"#(Scripting/Counters/Set number of player kills of a type to counter.[315])#",

    .help = R"#(Now takes an object type or type list instead of just an object type.  Total amount will reflect the total number of units killed by type or by the entirety of the list.)#",

    .argHint = R"#(Store the amount of kills by ${PlayerName} of the type of object(s) ${ObjectType} to ${CounterName})#",

},

{

    .index = "316",

    .id = "SET_PLAYER_KILLS_OF_KINDOF_TO_COUNTER",

    .name = R"#(Scripting/Counters/Set number of player kills of a kindof to counter.[316])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Store the amount of kills by ${PlayerName} of the ${UnitOrStructureKind} to ${CounterName})#",

},

{

    .index = "317",

    .id = "SET_PLAYER_OWNERSHIP_OF_TYPE_COUNTER",

    .name = R"#(Scripting/Counters/Set number of object types owned by player to counter.[317])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Store the number of ${ObjectType} owned by ${PlayerName} to ${CounterName})#",

},

{

    .index = "318",

    .id = "TEAM_SET_HEALTH",

    .name = R"#(Team/Damage or Remove/Set the health of the team members (in percentage).[318])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set health of ${TeamName} to percentage (0.0 - 100.0) ${RealNumber})#",

},

{

    .index = "319",

    .id = "NAMED_USE_COMMANDBUTTON_ON_NEAREST_ENEMY_UNIT",

    .name = R"#(Object Type/CommandButton/Use commandbutton ability at nearest enemy unit[319])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} use ${TeamAbilityName}  on nearest enemy unit.)#",

},

{

    .index = "320",

    .id = "NAMED_USE_COMMANDBUTTON_ON_NEAREST_GARRISONED_BUILDING",

    .name = R"#(Object Type/CommandButton/Use commandbutton ability at nearest enemy garrisoned building.[320])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} use ${TeamAbilityName}  on nearest enemy garrisoned building.)#",

},

{

    .index = "321",

    .id = "NAMED_USE_COMMANDBUTTON_ON_NEAREST_KINDOF",

    .name = R"#(Object Type/CommandButton/Use commandbutton ability at nearest enemy object with kind of.[321])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} use ${TeamAbilityName}  on nearest enemy with ${UnitOrStructureKind})#",

},

{

    .index = "322",

    .id = "NAMED_USE_COMMANDBUTTON_ON_NEAREST_ENEMY_BUILDING",

    .name = R"#(Object Type/CommandButton/Use commandbutton ability at nearest enemy building.[322])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} use ${TeamAbilityName}  on nearest enemy building.)#",

},

{

    .index = "323",

    .id = "NAMED_USE_COMMANDBUTTON_ON_NEAREST_ENEMY_BUILDING_CLASS",

    .name = R"#(Object Type/CommandButton/Use commandbutton ability at nearest enemy building kindof.[323])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} use ${TeamAbilityName}  on nearest enemy building with ${UnitOrStructureKind})#",

},

{

    .index = "324",

    .id = "NAMED_USE_COMMANDBUTTON_ON_NEAREST_OBJECTTYPE",

    .name = R"#(Object Type/CommandButton/Use commandbutton ability at nearest object type.[324])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} use ${TeamAbilityName} on nearest object of type ${ObjectName}.)#",

},

{

    .index = "325",

    .id = "NAMED_SET_STRICT_CONTROL_ENABLED",

    .name = R"#(Unused or Obsolete/Object Type/Settings/Strict control -- enable or disable.[325])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set ${UnitName} strict control to ${Boolean}.)#",

},

{

    .index = "326",

    .id = "TEAM_SET_STRICT_CONTROL_ENABLED",

    .name = R"#(Unused or Obsolete/Team/Status/Strict Control set enabled or disabled.[326])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set ${TeamName} strict control to ${Boolean}.)#",

},

{

    .index = "327",

    .id = "UNIT_SET_MODELCONDITION_FOR_DURATION",

    .name = R"#(Object Type/Settings/Set a model condition flag on a named object for duration. % specifies # of members in horde that get this.[327])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} sets ${ModelCondition} for ${RealNumber} seconds. Percentage (hordes) ${Percentage}.)#",

},

{

    .index = "328",

    .id = "UNIT_CLEAR_MODELCONDITION",

    .name = R"#(Object Type/Settings/Clear model condition flag on a named object.[328])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} clears ${ModelCondition}.)#",

},

{

    .index = "329",

    .id = "TEAM_SET_MODELCONDITION_FOR_DURATION",

    .name = R"#(Unused or Obsolete/Team/Settings/Set a model condition flag on a team (and horde members) for duration. % specifies # of members in team that get this.[329])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} sets model condition ${ModelCondition} for ${RealNumber} seconds. Percentage ${Percentage}.)#",

},

{

    .index = "330",

    .id = "TEAM_CLEAR_MODELCONDITION",

    .name = R"#(Team/Settings/Clear model condition flag on a team.[330])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} clears model condition ${ModelCondition}.)#",

},

{

    .index = "331",

    .id = "KILL_HORDE_MEMBERS",

    .name = R"#(Unused or Obsolete/Object Type/Damage or Remove/Kill off a percentage of the Horde[331])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Horde ${UnitName} kill off  ${RealNumber} percent. )#",

},

{

    .index = "334",

    .id = "AUDIO_MAKE_SOUND_IMMUNE_TO_FADE",

    .name = R"#(Media/Audio/Volume/Make sound immune to fade[334])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Make the ${AudioName} ignore the fade levels set by 'fade all audio.')#",

},

{

    .index = "335",

    .id = "AUDIO_MAKE_SOUND_SUBJECT_TO_FADE",

    .name = R"#(Media/Audio/Volume/Make sound subject to fade[335])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Make the ${AudioName} subject to the fade levels set by 'fade all audio.')#",

},

{

    .index = "336",

    .id = "AUDIO_MAKE_ALL_SOUNDS_SUBJECT_TO_FADE",

    .name = R"#(Media/Audio/Volume/Make all sounds subject to fade[336])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Make all sounds subject to the fade levels set by 'fade all audio.')#",

},

{

    .index = "338",

    .id = "FIRE_SPECIAL_POWER_ON_NEAREST_OBJECTTYPE",

    .name = R"#(Player/Special Power/Special power -- fire on nearest object type to team.[338])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} fire ${SpecialPowerName} on nearest object of ${ObjectType} to ${TeamName})#",

},

{

    .index = "339",

    .id = "FIRE_SPECIAL_POWER_ON_NEAREST_OBJECTTYPE_BY_PLAYER",

    .name = R"#(Player/Special Power/Special power -- fire on nearest object type to team owned by player.[339])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} fire ${SpecialPowerName} on nearest object of ${ObjectType} to ${TeamName} owned by ${PlayerName})#",

},

{

    .index = "340",

    .id = "FIRE_SPECIAL_POWER_ON_TEAM",

    .name = R"#(Player/Special Power/Special power -- fire on team.[340])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} fire ${SpecialPowerName} on ${TeamName})#",

},

{

    .index = "341",

    .id = "AUDIO_SET_REVERB_ROOM_TYPE",

    .name = R"#(Media/Audio/All Sounds/Set global reverb room type.[341])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Use global reverb room type ${ReverbRoomType})#",

},

{

    .index = "342",

    .id = "AUDIO_REMOVE_REVERB",

    .name = R"#(Media/Audio/All Sounds/Remove all reverb effects[342])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Do not use reverb effects)#",

},

{

    .index = "343",

    .id = "AUDIO_PUSH_MUSIC",

    .name = R"#(Media/Audio/Music/Push music track.[343])#",

    .help = R"#(Undone by Pop music track.)#",

    .argHint = R"#(Push (stack) ${MusicName} atop the current music track using fadeout (${Boolean}) and fadein (${Boolean}).)#",

},

{

    .index = "344",

    .id = "AUDIO_POP_MUSIC",

    .name = R"#(Media/Audio/Music/Pop music track.[344])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Pop the current music track off the stack using fadeout (${Boolean}) and fadein (${Boolean}).)#",

},

{

    .index = "345",

    .id = "UNIT_GUARD_POSITION",

    .name = R"#(Object Type/Guard/Named object guard at location.[345])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} begins guarding at ${WaypointName})#",

},

{

    .index = "346",

    .id = "UNIT_GUARD_OBJECT",

    .name = R"#(Object Type/Guard/Named object guard another named object.[346])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} begins guarding ${UnitName})#",

},

{

    .index = "347",

    .id = "UNIT_GUARD_AREA",

    .name = R"#(Object Type/Guard/Named object guard area.[347])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} begins guarding ${TriggerAreaName})#",

},

{

    .index = "348",

    .id = "MOVE_TEAM_HOME",

    .name = R"#(Team/Move/Set team to move to home position (designated in team template).[348])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Move ${TeamName} Home. )#",

},

{

    .index = "352",

    .id = "TEAM_SET_PLAYERS_NEAREST_UNIT_OF_TYPE_TO_REFERENCE",

    .name = R"#(Scripting/Reference/Set a reference to an object nearest-to-team, owned by player.[352])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Find an object matching a type in the list ${ObjectType} nearest the ${TeamName} (and owned by the teams Player) and set its reference to ${UnitReference})#",

},

{

    .index = "353",

    .id = "GATE_OPEN",

    .name = R"#(Unused or Obsolete/Object Type/gate/open a gate[353])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Open gate ${UnitName})#",

},

{

    .index = "354",

    .id = "GATE_CLOSE",

    .name = R"#(Unused or Obsolete/Object Type/gate/ close a gate[354])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Close gate.${UnitName})#",

},

{

    .index = "355",

    .id = "TEAM_HARVEST",

    .name = R"#(Unused or Obsolete/Team/Attack/Harvest around waypoint.[355])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} begins harvesting around ${WaypointName}.)#",

},

{

    .index = "356",

    .id = "MUSIC_PLAY_TRACK_FINITE_TIMES",

    .name = R"#(Media/Audio/Music/Play a music track a certain number of times.[356])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Play ${MusicName} ${Integer} times using fadeout (${Boolean}) and fadein (${Boolean}).)#",

},

{

    .index = "357",

    .id = "MUSIC_PLAY_TRACK_FINITE_TIMES_AND_NOTIFY",

    .name = R"#(Media/Audio/Music/Play a music track a certain number of times, then set a flag.[357])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Play ${MusicName} ${Integer} times using fadeout (${Boolean}) and fadein (${Boolean}), then set ${FlagName} to true.)#",

},

{

    .index = "358",

    .id = "MUSIC_PUSH_TRACK_FINITE_TIMES",

    .name = R"#(Media/Audio/Music/Push music track and play it a certain number of times.[358])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Push (stack) ${MusicName} atop the current music track and play it ${Integer} times using fadeout (${Boolean}) and fadein (${Boolean}). (Does NOT pop the music off the stack upon completion.) )#",

},

{

    .index = "359",

    .id = "MUSIC_PUSH_TRACK_FINITE_TIMES_AND_NOTIFY",

    .name = R"#(Media/Audio/Music/Push music track, play it a certain number of times, then set a flag.[359])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Push (stack) ${MusicName} atop the current music track and play it ${Integer} times using fadeout (${Boolean}) and fadein (${Boolean}), then set ${FlagName} to true. (Does NOT pop the music off the stack upon completion.) )#",

},

{

    .index = "360",

    .id = "REPAIR_OBJECT",

    .name = R"#(Object Type/Repair/Repair named Object.[360])#",

    .help = R"#(This script will attempt to start or stop repair on the named object.  It should only be called on repairable objects, and will automatically deduct the cost from the player that owns the object unless specified to be free.)#",

    .argHint = R"#(Repair named object ${UnitName} ${Boolean}, and it is ${Boolean} that this should be free.)#",

},

{

    .index = "361",

    .id = "TEAM_REPAIR_NEREST",

    .name = R"#(Unused or Obsolete/Team/Repair/Repair nearest building that needs it.[361])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Have Team ${TeamName} repair the nearest building within the radius ${RealNumber})#",

},

{

    .index = "363",

    .id = "TEAM_SET_AUTO_ABILITY",

    .name = R"#(Unused or Obsolete/Team/CommandButton/Team set auto ability to on or off.[363])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Have Team ${TeamName} set auto ability ${CommandButtonName} to ${Boolean})#",

},

{

    .index = "364",

    .id = "SET_LOGIC_FOG_STATE",

    .name = R"#(Unused or Obsolete/Fog or Reveal/Toggle Fog of War.[364])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set presence of Fog of War to ${Boolean}. (Default true.))#",

},

{

    .index = "365",

    .id = "CAMERA_RESTRICT_TO_AREA",

    .name = R"#(Cinematics/Camera/Restrict camera movement to area.[365])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Restrict camera movement to ${TriggerAreaName}.)#",

},

{

    .index = "366",

    .id = "CAMERA_REMOVE_AREA_RESTRICTION",

    .name = R"#(Unused or Obsolete/Camera/Move/Restrict camera movement to area -- REMOVE.[366])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Restrict camera movement to area -- REMOVE.)#",

},

{

    .index = "367",

    .id = "UNIT_GUARD_AREA_FROM_POSITION",

    .name = R"#(Object Type/Guard/Named object guard an area from a location.[367])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} begins guarding ${TriggerAreaName} from ${WaypointName})#",

},

{

    .index = "368",

    .id = "TEAM_GUARD_AREA_FROM_POSITION",

    .name = R"#(Team/Guard/Team guard an area from a location.[368])#",

    .help = R"#(Please note that units guarding an area that is fogged will not be able to see a unit that enters the area and thus not attack so be sure that the area they are guarding is not fogged.)#",

    .argHint = R"#( ${TeamName} begins guarding ${TriggerAreaName} from ${WaypointName})#",

},

{

    .index = "369",

    .id = "FIND_HOME_BASE_OF_PLAYER",

    .name = R"#(Scripting/Reference/Find home base of player[369])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Find the home base owned by ${PlayerName} and set ${UnitReference} to reference it. (Building must be inside map boundaries: ${Boolean}))#",

},

{

    .index = "370",

    .id = "SET_REF_TO_NEREST_TEAM_OF_TYPE_OWNED_BY_PLAYER",

    .name = R"#(Scripting/Reference/Set reference to object type of player nearest team.[370])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Find ${ObjectType} Owned by ${PlayerName} nearest ${TeamName} and reference as ${UnitReference})#",

},

{

    .index = "371",

    .id = "SET_REF_TO_NEREST_TEAM_OF_UNNAMED_TYPE_OWNED_BY_PLAYER",

    .name = R"#(Scripting/Reference/Set reference to unnamed object owned by player [371])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Find unnamed ${ObjectType} Owned by ${PlayerName} nearest ${TeamName} and reference as ${UnitReference})#",

},

{

    .index = "372",

    .id = "MAP_EXIT",

    .name = R"#(Map/Modify/Exit Map[372])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Exits map back to the main menu.)#",

},

{

    .index = "374",

    .id = "TEAM_NEEDS_OPEN_GATE",

    .name = R"#(Unused or Obsolete/Team/Misc/Set or clear a team-needs-gate open flag [374])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Team ${TeamName} wants gate ${UnitName} open is ${Boolean})#",

},

{

    .index = "375",

    .id = "UNIT_SET_TEAM",

    .name = R"#(Object Type/Allegiance/Force named object to join team.[375])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} will join ${TeamName}.)#",

},

{

    .index = "376",

    .id = "UNIT_STAND_GROUND",

    .name = R"#(Unused or Obsolete/Object Type/Status/Unit Set Stand Ground Status.[376])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set Stand Ground Status of ${UnitName} to ${Boolean})#",

},

{

    .index = "377",

    .id = "TEAM_STAND_GROUND",

    .name = R"#(Unused or Obsolete/Object Type/Status/Team Set Stand Ground Status.[377])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set Stand Ground Status of ${TeamName} to ${Boolean})#",

},

{

    .index = "378",

    .id = "UNIT_TELEPORT_TO_WAYPOINT",

    .name = R"#(Object Type/Move/Teleport named object to waypoint.[378])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Teleport ${UnitName} to ${WaypointName}.)#",

},

{

    .index = "379",

    .id = "TEAM_TELEPORT_TO_WAYPOINT",

    .name = R"#(Team/Move/Teleport team to waypoint.[379])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Teleport ${TeamName} to ${WaypointName}.)#",

},

{

    .index = "380",

    .id = "OVERRIDE_PLAYER_COMMAND_POINTS",

    .name = R"#(ThreeSixty/Override player command points.[380])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Override command points for ${PlayerName} to total: ${Integer} maximum: ${Integer})#",

},

{

    .index = "381",

    .id = "SELECT_OBJECT",

    .name = R"#(Tutorial/Player selects object.[381])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(For ${PlayerName} select object ${UnitName})#",

},

{

    .index = "382",

    .id = "DESELECT",

    .name = R"#(Tutorial/Deselects everything.[382])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Deselects everything )#",

},

{

    .index = "383",

    .id = "HUMAN_IMPASSABLE_AREA",

    .name = R"#(Map/Make area impassable for human player.[383])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( Make area ${TriggerAreaName} impassable for human player: ${Boolean})#",

},

{

    .index = "384",

    .id = "SET_COUNTER_TO_NUMBER_OBJECTS_PLAYER_OWNES_WITH_MODELCONDITION",

    .name = R"#(Scripting/Counters/Set counter to number of objects owned by player with a model condition[384])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Store the number of objects owned by ${PlayerName} with ${ModelCondition} ON to the counter ${CounterName})#",

},

{

    .index = "385",

    .id = "PLAY_SOUND_EFFECT_AT_TEAM",

    .name = R"#(Media/Audio/Sound Effect/Play sound effect from team.[385])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Play ${SoundName} as though coming from a member of ${TeamName}.)#",

},

{

    .index = "386",

    .id = "VICTORY_SCREEN",

    .name = R"#(Win & Loss/ Show Victory Screen[386])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Shows the victory screen, but does not behave as a victory.)#",

},

{

    .index = "387",

    .id = "LOCK_CAMERA",

    .name = R"#(Tutorial/Lock or unlock camera.[387])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set the camera lock to ${Boolean} .)#",

},

{

    .index = "389",

    .id = "TEAM_ENABLE_HOUSE_COLOR",

    .name = R"#(Team/Settings/Enables or disables the house color[389])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Enable the house color for ${TeamName} to ${Boolean} .)#",

},

{

    .index = "390",

    .id = "ENABLE_HOUSE_COLOR",

    .name = R"#(Map/Modify/Enables or disables the house color for all units[390])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Enable the house color for all units ${Boolean} .)#",

},

{

    .index = "391",

    .id = "UNIT_FORCE_EMOTION",

    .name = R"#(Cinematics/Force Emotion/Force a named object to have an emotion for duration.[391])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Force ${UnitName} to have ${Emotion} for ${RealNumber} seconds.)#",

},

{

    .index = "392",

    .id = "TEAM_FORCE_EMOTION",

    .name = R"#(Cinematics/Force Emotion/Force a team to have an emotion for duration.[392])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Force ${TeamName} to have ${Emotion} for ${RealNumber} seconds.)#",

},

{

    .index = "393",

    .id = "PLAYER_FORCE_EMOTION",

    .name = R"#(Cinematics/Force Emotion/Force all of player's units to have an emotion for duration.[393])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Force all units of ${PlayerName} to have ${Emotion} for ${RealNumber} seconds.)#",

},

{

    .index = "394",

    .id = "SET_PLAYER_OWNERSHIP_OF_TYPE_COUNTER_INCLUDE_DEAD",

    .name = R"#(Scripting/Counters/Set number of object types owned by player to a counter including dead ones.[394])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Store the number of objects (include dead ones)${ObjectType} owned by ${PlayerName} to ${CounterName})#",

},

{

    .index = "395",

    .id = "UNIT_GAIN_LEVEL",

    .name = R"#(Object Type/Experience/Named object levels up with visual FX.[395])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} gains a level with FX turned ${Boolean})#",

},

{

    .index = "396",

    .id = "TEAM_GAIN_LEVEL",

    .name = R"#(Team/Experience/Team levels up with visual FX.[396])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} gains a level with FX turned ${Boolean})#",

},

{

    .index = "397",

    .id = "AUDIO_SET_REVERB_SUPPRESSION_POLYGON",

    .name = R"#(Media/Audio/All Sounds/Suppress reverb within an area[397])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Lower reverb of all sounds in ${TriggerAreaName} to ${Percentage} of normal.)#",

},

{

    .index = "398",

    .id = "AUDIO_REMOVE_REVERB_SUPPRESSION_POLYGON",

    .name = R"#(Media/Audio/All Sounds/Unsuppress reverb in an area[398])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Return the reverb of sounds in ${TriggerAreaName} to normal.)#",

},

{

    .index = "399",

    .id = "AUDIO_REMOVE_ALL_REVERB_SUPPRESSION_POLYGONS",

    .name = R"#(Media/Audio/All Sounds/Unsuppress all reverb[399])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Remove all reverb-suppression areas.)#",

},

{

    .index = "400",

    .id = "LOCK_CAMERA_ANGLE_AND_HEIGHT",

    .name = R"#(Tutorial/Lock or unlock camera height and angle (allow scrolling).[400])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set the camera angle&  height lock to ${Boolean} .)#",

},

{

    .index = "401",

    .id = "ROTATE_CAMERA_TO_ANGLE",

    .name = R"#(Cinematics/Camera/Rotate to a specific view angle.[401])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Rotate camera view to ${RealNumber} degrees, taking ${RealNumber} seconds, ease-in ${RealNumber} seconds, ease-out ${RealNumber} seconds.)#",

},

{

    .index = "402",

    .id = "NAMED_SET_CAMERA_FADING",

    .name = R"#(Unused or Obsolete/Object Type/Settings/Camera fading -- enable or disable.[402])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set ${UnitName} camera fading to ${Boolean}.)#",

},

{

    .index = "403",

    .id = "SHOW_MISSION_OBJECTIVE",

    .name = R"#(Unused or Obsolete/Interface/Objectives/Show mission objective[403])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Make mission objective '${MissionObjective}' visible.)#",

},

{

    .index = "404",

    .id = "HIDE_MISSION_OBJECTIVE",

    .name = R"#(Unused or Obsolete/Interface/Objectives/Hide mission objective[404])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Hide mission objective '${MissionObjective}'.)#",

},

{

    .index = "405",

    .id = "LINK_MISSION_OBJECTIVE_TO_OBJECT",

    .name = R"#(Interface/Objectives/Link mission objective to object[405])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set mission objective '${MissionObjective}' linked to ${UnitName} to ${Boolean}.)#",

},

{

    .index = "406",

    .id = "LINK_MISSION_OBJECTIVE_TO_WAYPOINT",

    .name = R"#(Interface/Objectives/Link mission objective to waypoint[406])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set mission objective '${MissionObjective}' linked to ${WaypointName} to ${Boolean}.)#",

},

{

    .index = "407",

    .id = "MARK_MISSION_OBJECTIVE_COMPLETED",

    .name = R"#(Interface/Objectives/Mark mission objective as completed[407])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Mark mission objective '${MissionObjective}' as completed.)#",

},

{

    .index = "408",

    .id = "MARK_MISSION_OBJECTIVE_NOT_COMPLETED",

    .name = R"#(Interface/Objectives/Mark mission objective as not completed[408])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Mark mission objective '${MissionObjective}' as not completed.)#",

},

{

    .index = "411",

    .id = "UNIT_CHANGE_OBJECT_STATUS",

    .name = R"#(Object Type/Settings/Change object status.[411])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(For ${UnitName} set ${ObjectStatus} to ${Boolean}.)#",

},

{

    .index = "412",

    .id = "TEAM_CHANGE_OBJECT_STATUS",

    .name = R"#(Team/Settings/Change object status.[412])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(For all units in ${TeamName} set ${ObjectStatus} to ${Boolean}.)#",

},

{

    .index = "413",

    .id = "ATTACK_MOVE_TEAM_TO",

    .name = R"#(Team/Attack/Set team to attack-move to a location.[413])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(AttackMove ${TeamName} to ${WaypointName}.)#",

},

{

    .index = "414",

    .id = "TEAM_ATTACK_MOVE_FOLLOW_WAYPOINTS",

    .name = R"#(Team/Attack/Set team to attack-move along a waypoint path.[414])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Have ${TeamName} AttackMove along ${WaypointPathName} , as a team is ${Boolean} , in formation is ${Boolean})#",

},

{

    .index = "415",

    .id = "ATTACK_MOVE_NAMED_UNIT_TO",

    .name = R"#(Object Type/Attack/Named object attack-move to a location.[415])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(AttackMove ${UnitName} to ${WaypointName}.)#",

},

{

    .index = "416",

    .id = "NAMED_ATTACK_FOLLOW_WAYPOINTS",

    .name = R"#(Object Type/Attack/Named object attack-move along a waypoint path.[416])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} AttackMove follows waypoints, beginning at ${WaypointPathName})#",

},

{

    .index = "417",

    .id = "UNIT_ATTACK_MOVE_TOWARDS_NEAREST_OBJECT_TYPE",

    .name = R"#(Object Type/Attack/Move unit towards the nearest object of a specific type[417])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} will AttackMove towards the nearest ${ObjectName})#",

},

{

    .index = "418",

    .id = "TEAM_ATTACK_MOVE_TOWARDS_NEAREST_OBJECT_TYPE",

    .name = R"#(Team/Attack/Move team towards the nearest object of a specific type[418])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} will AttackMove towards the nearest ${ObjectName})#",

},

{

    .index = "419",

    .id = "MOVE_CAMERA_BY_ANIMATION",

    .name = R"#(Cinematics/Camera/Move using camera animation.[419])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Move using ${CameraAnimation}.)#",

},

{

    .index = "420",

    .id = "MAP_REVEAL_IN_TRIGGER",

    .name = R"#(Map/Fog or Reveal/Reveal map in trigger -- fog.[420])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(The map is revealed inside ${TriggerAreaName} for ${PlayerName}.)#",

},

{

    .index = "421",

    .id = "MAP_REVEAL_PERMANENTLY_IN_TRIGGER",

    .name = R"#(Map/Fog or Reveal/Reveal map in polytrigger -- permanently.[421])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(The map is permanently revealed in trigger ${TriggerAreaName} for ${PlayerName}. (Afterwards referred to as ${MapRevealName}).)#",

},

{

    .index = "422",

    .id = "MAP_UNDO_REVEAL_PERMANENTLY_IN_TRIGGER",

    .name = R"#(Map/Fog or Reveal/Reveal map in polytrigger -- undo permanently.[422])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${MapRevealName} is undone.)#",

},

{

    .index = "423",

    .id = "TEAM_INCREASE_PRIORITY_BY_VALUE",

    .name = R"#(Team/Spawn, Build, or Merge/Increase priority by specific amount.[423])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Increase the build priority for ${TeamName}  by ${Integer} .)#",

},

{

    .index = "424",

    .id = "TEAM_DECREASE_PRIORITY_BY_VALUE",

    .name = R"#(Team/Spawn, Build, or Merge/Decrease priority by specific amount.[424])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Decrease the build priority for ${TeamName}  by ${Integer} .)#",

},

{

    .index = "425",

    .id = "ATTACK_MOVE_TEAM_TO_NAMED_OBJECT",

    .name = R"#(Team/Attack/Attack move a team to a named object.[425])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(AttackMove ${TeamName} to ${UnitName}.)#",

},

{

    .index = "426",

    .id = "ATTACK_MOVE_TEAM_TO_NEAREST_OBJECT_OF_TYPE_OF_PLAYER",

    .name = R"#(Team/Attack/Move team towards the nearest object of a specific type owned by a player[426])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} will AttackMove towards the nearest ${ObjectType} owned by ${PlayerName})#",

},

{

    .index = "427",

    .id = "TEAM_SET_AI_RECRUITABLE_FLAG",

    .name = R"#(Team/Settings/Change status of the Scripted AI Recruitable flag for team members.[427])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set the Scripted AI Recruitable flag for ${TeamName} to ${Boolean})#",

},

{

    .index = "430",

    .id = "PLAYER_FIRE_SPECIAL_POWER_AT_WAYPOINT",

    .name = R"#(Player/Special Power/Special power -- fire at waypoint.[430])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} fires ${SpecialPowerName} at ${WaypointName}.)#",

},

{

    .index = "431",

    .id = "NAMED_BUILD_STRUCTURE_AT_WAYPOINT",

    .name = R"#(Unused or Obsolete/Object Type/CommandButton/Build thing at waypoint.[431])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} build ${ObjectName} at angle ${Angle} at ${WaypointName}.)#",

},

{

    .index = "432",

    .id = "TEAM_GARRISON_TEAM_INSTANTLY",

    .name = R"#(Team/Garrison/Garrison a team instantly with a team.[432])#",

    .help = R"#(Note that this Script will not do any proper checks to make sure the objects container is valid for the unit entering it.  If you add a unit that is not valid for the container the behavior thereafter is undefined and could possibly crash.)#",

    .argHint = R"#( ${TeamName} garrison's ${TeamName} instantly.)#",

},

{

    .index = "433",

    .id = "LOCK_CAMERA_ZOOM",

    .name = R"#(Tutorial/Lock or unlock camera zoom.[433])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set the camera zoom lock to ${Boolean} .)#",

},

{

    .index = "434",

    .id = "LOCK_CAMERA_ROTATION",

    .name = R"#(Tutorial/Lock or unlock camera rotation.[434])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set the camera rotation lock to ${Boolean} .)#",

},

{

    .index = "435",

    .id = "LOCK_CAMERA_SCROLL",

    .name = R"#(Tutorial/Lock or unlock camera scroll.[435])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set the camera scroll lock to ${Boolean} .)#",

},

{

    .index = "436",

    .id = "LOCK_CAMERA_RESET",

    .name = R"#(Tutorial/Lock or unlock camera reset.[436])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set the camera reset lock to ${Boolean} .)#",

},

{

    .index = "437",

    .id = "PLAYER_SET_SPECIAL_POWER_COUNTDOWN",

    .name = R"#(Player/Special Power/Special power -- set countdown timer.[437])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set ${PlayerName}'s ${SpecialPowerName} to ${Integer} seconds.)#",

},

{

    .index = "438",

    .id = "ENABLE_PLANNING_MODE",

    .name = R"#(Tutorial/Enable planning mode[438])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Enable planning mode.)#",

},

{

    .index = "439",

    .id = "DISABLE_PLANNING_MODE",

    .name = R"#(Tutorial/Disable planning mode[439])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Disable planning mode.)#",

},

{

    .index = "440",

    .id = "CREATE_NAMED_ON_TEAM_AT_OBJECTTYPE",

    .name = R"#(Unused or Obsolete/Object Type/Spawn/Spawn -- named unit on Team at an object type.[440])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Spawn ${UnitName} of type ${ObjectName} on ${TeamName} at type ${ObjectName})#",

},

{

    .index = "441",

    .id = "UNIT_SET_MAX_LEVEL",

    .name = R"#(Unused or Obsolete/Object Type/Experience/Sets the unit's maximum level.[441])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Sets ${UnitName} maximum level to ${Integer}.)#",

},

{

    .index = "442",

    .id = "PLAYER_AI_CHANGEBASEBUILDER",

    .name = R"#(SkirmishAI/Turn on or off AI Base Builder.[442])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Turn AI Base Builder for ${PlayerName} ${NewBoolean})#",

},

{

    .index = "443",

    .id = "PLAYER_AI_CHANGEUNITBUILDER",

    .name = R"#(SkirmishAI/Turn on or off AI Unit Builder.[443])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Turn AI Unit Builder for ${PlayerName} ${NewBoolean})#",

},

{

    .index = "444",

    .id = "PLAYER_AI_CHANGETEAMBUILDER",

    .name = R"#(SkirmishAI/Turn on or off AI Party Builder.[444])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Turn AI Party Builder for ${PlayerName} ${NewBoolean})#",

},

{

    .index = "445",

    .id = "PLAYER_AI_CHANGE_ECONOMY_MANAGER",

    .name = R"#(SkirmishAI/Turn on or off AI Economy Manager.[445])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Turn AI Economy Builder for ${PlayerName} ${NewBoolean})#",

},

{

    .index = "446",

    .id = "PLAYER_AI_CHANGEWALLBUILDER",

    .name = R"#(SkirmishAI/Turn on or off AI Wall Builder.[446])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Turn AI Wall Builder for ${PlayerName} ${NewBoolean})#",

},

{

    .index = "447",

    .id = "PLAYER_AI_CHANGEUNITUPGRADER",

    .name = R"#(SkirmishAI/Turn on or off AI Unit Upgrader.[447])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Turn AI Unit Upgrader for ${PlayerName} ${NewBoolean})#",

},

{

    .index = "448",

    .id = "PLAYER_AI_CHANGESCIENCEUPGRADER",

    .name = R"#(SkirmishAI/Turn on or off AI Science Upgrader.[448])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Turn AI Science Upgrader for ${PlayerName} ${NewBoolean})#",

},

{

    .index = "449",

    .id = "PLAYER_AI_CHANGETACTICAL",

    .name = R"#(SkirmishAI/Turn on or off Tactical AI.[449])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Turn Tactical AI for ${PlayerName} ${NewBoolean})#",

},

{

    .index = "450",

    .id = "TEAM_AI_TRANSFER",

    .name = R"#(SkirmishAI/Transfer control of Team to or from Skirmish AI.[450])#",

    .help = R"#(Ensure you have needed components of Skirmish AI turned on.)#",

    .argHint = R"#(Turn Skirmish AI for ${TeamName} ${NewBoolean})#",

},

{

    .index = "452",

    .id = "UNIT_AI_TRANSFER_ALL",

    .name = R"#(SkirmishAI/Transfer control of all player's units to or from Skirmish AI -- Enable Skirmish AI.[452])#",

    .help = R"#(Transfers control of all players objects to or from the Skirmish AI, it also starts the AI timers and opening moves(turns Skirmish AI On, if transferring to the Skirmish AI). Ensure you have needed components of Skirmish AI turned on in the player list.)#",

    .argHint = R"#(Turn Skirmish AI for ${PlayerName} ${NewBoolean} starting opening moves and timers.)#",

},

{

    .index = "453",

    .id = "NAMED_SET_SLEEP_STATUS",

    .name = R"#(Unused or Obsolete/Object Type/Settings/Set the sleep status of a specific unit.[453])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} changes his sleep status to ${Boolean})#",

},

{

    .index = "454",

    .id = "TEAM_SET_SLEEP_STATUS",

    .name = R"#(Unused or Obsolete/Team/Settings/Set the Sleep status of Team.[454])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${TeamName} change their sleep status to ${Boolean})#",

},

{

    .index = "455",

    .id = "BUILD_LIST_MOVE_TO_TOP",

    .name = R"#(Base Builder/Move structure to top of build list.[455])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Move ${UnitName} to top of build list)#",

},

{

    .index = "456",

    .id = "BUILD_LIST_ELIMINATE_FOOTPRINT",

    .name = R"#(Base Builder/Eliminate building footprint.[456])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Eliminate footprint for ${UnitName} .)#",

},

{

    .index = "457",

    .id = "BUILD_LIST_ELIMINATE_FOOTPRINT_ALL",

    .name = R"#(Base Builder/Eliminate all building footprints.[457])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Eliminate all building footprints for ${PlayerName} .)#",

},

{

    .index = "458",

    .id = "UNIT_CONTEXT_SENSITIVE_ATTACK",

    .name = R"#(Object Type/Attack/Set named object to capture a named object.[458])#",

    .help = R"#(This script should be used to tell Engineer and Infiltration type units to capture/infiltrate another object.)#",

    .argHint = R"#( ${UnitName} begin context-sensitive attack on ${UnitName})#",

},

{

    .index = "459",

    .id = "SKIRMISH_AI_SET_FAVORED_ENEMY",

    .name = R"#(SkirmishAI/Enemy Target Control/Set Skirmish AI favored enemy[459])#",

    .help = R"#(Sets the given enemy's priority to 10, and everyone else's priority to 0)#",

    .argHint = R"#(Set favored enemy of ${PlayerName} to ${PlayerName})#",

},

{

    .index = "460",

    .id = "SKIRMISH_AI_USE_REBUILD_SYSTEM",

    .name = R"#(SkirmishAI/Use Skirmish AI Structure Rebuilds[460])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Use structure rebuilds for ${PlayerName}: ${Boolean})#",

},

{

    .index = "462",

    .id = "TEAM_ALLOW_AI_AUTOACQUIRE",

    .name = R"#(SkirmishAI/Allow AI Auto-Acquire for team.[462])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Use AI AutoAcquire for ${TeamName}: ${Boolean})#",

},

{

    .index = "463",

    .id = "UNIT_ALLOW_AI_AUTOACQUIRE",

    .name = R"#(SkirmishAI/Allow AI Auto-Acquire for named object.[463])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Use AI AutoAcquire for ${UnitName}: ${Boolean})#",

},

{

    .index = "466",

    .id = "CREATE_NAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION",

    .name = R"#(Object Type/Spawn/Spawn a named object on a team at a waypoint with an orientation.[466])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Spawn ${UnitName} of type ${ObjectName} on ${TeamName} at ${WaypointName}, rotated ${Angle}.)#",

},

{

    .index = "467",

    .id = "CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION",

    .name = R"#(Object Type/Spawn/Spawn an unnamed object on a team at a waypoint with an orientation.[467])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Spawn unit of type ${ObjectName} on ${TeamName} at ${WaypointName}, rotated ${Angle}.)#",

},

{

    .index = "468",

    .id = "CREATE_NAMED_ON_TEAM_AT_OBJECTTYPE_WITH_ORIENTATION",

    .name = R"#(Unused or Obsolete/Object Type/Spawn/Spawn -- named unit on Team at an object type with specified orientation.[468])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Spawn ${UnitName} of type ${ObjectName} on ${TeamName} at type ${ObjectName}, rotated ${Angle}.)#",

},

{

    .index = "469",

    .id = "SET_CAMPAIGN_FLAG",

    .name = R"#(Campaign/Set or clear a campaign flag[469])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set campaign flag ${CampaignFlag} to ${Boolean})#",

},

{

    .index = "470",

    .id = "PLAYER_SPECIAL_POWER_AVAILABILITY",

    .name = R"#(Player/Special Power/Set special power availability.[470])#",

    .help = R"#(This script will prevent a player from casting a player power, even if it's not locked.)#",

    .argHint = R"#( ${PlayerName} set ${SpecialPowerName} availability to ${Availability}.)#",

},

{

    .index = "471",

    .id = "PLAYER_UNIT_TYPE_SPECIAL_POWER_IGNORE_REQUIREMENTS",

    .name = R"#(Player/Special Power/Player's object's special power ignores requirements.[471])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(The ${SpecialPowerName} for ${ObjectTypeListName} owned by ${PlayerName} ignores requirements: ${Boolean}.)#",

},

{

    .index = "472",

    .id = "SKIRMISH_AI_MAKE_BASE",

    .name = R"#(SkirmishAI/Create a skirmish AI base[472])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Player ${PlayerName} has a base at ${WaypointName}.)#",

},

{

    .index = "473",

    .id = "UNIT_SET_STANCE",

    .name = R"#(Object Type/Settings/Sets a unit's current stance[473])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Sets ${UnitName} stance to ${ObjectStance}.)#",

},

{

    .index = "474",

    .id = "DISPLAY_TUTORIAL_TEXT",

    .name = R"#(Tutorial/Display tutorial text[474])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Show tutorial text: ${LocalizedStringName})#",

},

{

    .index = "475",

    .id = "HIDE_TUTORIAL_TEXT",

    .name = R"#(Tutorial/Hide tutorial text[475])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Hide the tutorial text.)#",

},

{

    .index = "476",

    .id = "FLASH_MOUSE_HUD_FIXED_ELEMENT",

    .name = R"#(Tutorial/Flash PC HUD fixed element[476])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set flash PC HUD fixed element ${MouseHud} to ${Boolean}.)#",

},

{

    .index = "477",

    .id = "FLASH_UI_PLAYER_POWER_BUTTON",

    .name = R"#(Tutorial/Flash UI player power button[477])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set flash UI player power button ${SpecialPowerName} to ${Boolean}.)#",

},

{

    .index = "478",

    .id = "FLASH_UI_UNIT_ABILITY_BUTTON",

    .name = R"#(Tutorial/Flash named object's UI user ability button[478])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(For ${UnitName} set flash UI user ability button ${UnitAbilityName} to ${Boolean}.)#",

},

{

    .index = "479",

    .id = "FLASH_UI_BUILD_BUTTON",

    .name = R"#(Tutorial/Flash UI build button[479])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set flash UI build '${ObjectName}' button to ${Boolean}.)#",

},

{

    .index = "481",

    .id = "SKIRMISH_AI_USE_REPAIRS",

    .name = R"#(SkirmishAI/Skirmish AI may repair buildings[481])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Use structure repair for ${PlayerName}: ${Boolean})#",

},

{

    .index = "482",

    .id = "DISABLE_MOUSE_HUD_FIXED_ELEMENT",

    .name = R"#(Tutorial/Disable PC HUD fixed element[482])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set disable PC HUD fixed element ${MouseHud} to ${Boolean}.)#",

},

{

    .index = "483",

    .id = "DISABLE_UI_PLAYER_POWER_BUTTON",

    .name = R"#(Tutorial/Disable UI player power button[483])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set disable UI player power button ${SpecialPowerName} to ${Boolean}.)#",

},

{

    .index = "484",

    .id = "DISABLE_UI_UNIT_ABILITY_BUTTON",

    .name = R"#(Tutorial/Disable named object's UI user ability button[484])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(For ${UnitName} set disable UI user ability button ${UnitAbilityName} to ${Boolean}.)#",

},

{

    .index = "485",

    .id = "DISABLE_UI_BUILD_BUTTON",

    .name = R"#(Tutorial/Disable UI build button[485])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set disable UI build '${ObjectName}' button to ${Boolean}.)#",

},

{

    .index = "487",

    .id = "PLAYER_SET_BASE_POWER",

    .name = R"#(Player/Power/Set the base power for the player.[487])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set base power for ${PlayerName} to ${Integer})#",

},

{

    .index = "488",

    .id = "TEAM_CONTEXT_SENSITIVE_ATTACK",

    .name = R"#(Team/Attack/Set team to capture a named object.[488])#",

    .help = R"#(This script should be used to tell a team of Engineer's and/or Infiltration type units to capture/infiltrate another object.)#",

    .argHint = R"#( ${TeamName} begin context-sensitive attack on ${UnitName})#",

},

{

    .index = "489",

    .id = "SET_UNIT_HEALTH_PERCENTAGE_TO_COUNTER",

    .name = R"#(Scripting/Counters/Set unit's health as percentage to counter.[489])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Store ${UnitName}'s health as percentage to the counter ${CounterName})#",

},

{

    .index = "490",

    .id = "PLAYER_SET_MONEY_CAP",

    .name = R"#(Player/Money/Set player's money cap.[490])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set ${PlayerName}'s money cap to $${Integer})#",

},

{

    .index = "491",

    .id = "SWITCH_TO_PRODUCTION_QUEUE_TAB",

    .name = R"#(Tutorial/Switch to production queue tab[491])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Switch to ${ProductionQueueTab}.)#",

},

{

    .index = "492",

    .id = "SET_MISSION_OBJECTIVE_STATUS",

    .name = R"#(Interface/Objectives/Set mission objective status[492])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set status of mission objective '${MissionObjective}' to ${MissionObjectiveStatus}.)#",

},

{

    .index = "493",

    .id = "SET_COUNTER_TO_NUMBER_OBJECTS_PLAYER_OWNS_WITH_KINDOF",

    .name = R"#(Scripting/Counters/Set counter to number of objects owned by player with kindof[493])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Store the number of objects owned by ${PlayerName} of ${UnitOrStructureKind} to ${CounterName}, count dead : ${Boolean}, count under construction : ${Boolean})#",

},

{

    .index = "494",

    .id = "REGISTER_OBJECT_VS_OBJECT_KILL_RECORDING",

    .name = R"#(Scripting/Data Tracking/Record object vs. object kills[494])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Record object type ${ObjectName} belonging to ${PlayerName} killed object type ${ObjectName} belonging to ${PlayerName}.)#",

},

{

    .index = "495",

    .id = "STORE_OBJECT_KILL_COUNT_TO_COUNTER",

    .name = R"#(Scripting/Data Tracking/Store object vs. object kills to counter[495])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Store num of object type ${ObjectName} belonging to ${PlayerName} killed object type ${ObjectName} belonging to ${PlayerName} to counter ${CounterName}.)#",

},

{

    .index = "496",

    .id = "PLAY_SOUND_AS_IF_IT_WAS_AN_EVA_EVENT",

    .name = R"#(Media/Audio/EVA/Play a sound effect as if it were an EVA event.[496])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${SoundName} plays as if it was an EVA event)#",

},

{

    .index = "497",

    .id = "PLAY_SOUND_AS_IF_IT_WAS_AN_EVA_EVENT_WITH_SUBTITLE",

    .name = R"#(Media/Audio/EVA/Play a sound effect as if it were an EVA event, and subtitle it.[497])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${SoundName} plays as if it was an EVA event, while displaying ${LocalizedStringName} for ${RealNumber} seconds.)#",

},

{

    .index = "498",

    .id = "PLAY_SPEECH_AS_IF_IT_WAS_AN_EVA_EVENT",

    .name = R"#(Unused or Obsolete/Media/Audio/Sound Effect/Play a speech file as if it were an EVA event.[498])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${SpeechName} plays as if it was an EVA event)#",

},

{

    .index = "499",

    .id = "PLAY_SPEECH_AS_IF_IT_WAS_AN_EVA_EVENT_WITH_SUBTITLE",

    .name = R"#(Unused or Obsolete/Media/Audio/Sound Effect/Play a speech file as if it were an EVA event, and subtitle it.[499])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${SpeechName} plays as if it was an EVA event, while displaying military briefing ${LocalizedStringName} for ${RealNumber} seconds.)#",

},

{

    .index = "500",

    .id = "UNIT_KILL_ALL_IN_AREA",

    .name = R"#(Object Type/Damage or Remove/Kill all units belonging to the specified player in the specified area.[500])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Kill all units belonging to ${PlayerName} in ${TriggerAreaName}.)#",

},

{

    .index = "501",

    .id = "SKIRMISH_AI_REBUILD_SYSTEM_SCAN",

    .name = R"#(SkirmishAI/Skirmish AI Structure Rebuilds Rescan[501])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Scan for new structures to rebuild for ${PlayerName})#",

},

{

    .index = "502",

    .id = "NAMED_REMOVE_UPGRADE",

    .name = R"#(Object Type/Settings/Remove upgrade from named object.[502])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Remove ${UpgradeName} from ${UnitName}.)#",

},

{

    .index = "503",

    .id = "MOVE_CAMERA_TO_NAMED",

    .name = R"#(Cinematics/Camera/Move the camera to a named object's location.[503])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Move camera to ${UnitName} in ${RealNumber} seconds, camera shutter ${RealNumber} seconds, ease-in ${RealNumber} seconds, ease-out ${RealNumber} seconds.)#",

},

{

    .index = "504",

    .id = "REMOVE_PLAYER_UPGRADE",

    .name = R"#(Player/Upgrades and Sciences/Remove upgrade from Player.[504])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Remove ${UpgradeName} from ${PlayerName})#",

},

{

    .index = "505",

    .id = "MUSIC_PLAY_TRACK_FINITE_TIMES_AS_SPECIAL_END_OF_LEVEL_MUSIC",

    .name = R"#(Media/Audio/Music/Play SPECIAL END-OF-LEVEL music a certain number of times.[505])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Play ${MusicName} as SPECIAL END-OF-LEVEL MUSIC ${Integer} times using fadeout (${Boolean}) and fadein (${Boolean}).)#",

},

{

    .index = "506",

    .id = "LINK_MISSION_HOT_SPOT_TO_OBJECT",

    .name = R"#(Interface/Mission Hot-Spot/Link mission hot-spot to object[506])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set mission hot-spot ${MissionHotSpot} linked to object ${UnitName} to ${Boolean}.)#",

},

{

    .index = "507",

    .id = "LINK_MISSION_HOT_SPOT_TO_WAYPOINT",

    .name = R"#(Interface/Mission Hot-Spot/Link mission hot-spot to waypoint[507])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set mission hot-spot ${MissionHotSpot} linked to waypoint ${WaypointName} to ${Boolean}.)#",

},

{

    .index = "508",

    .id = "SET_MISSION_HOT_SPOT_VISIBILITY",

    .name = R"#(Interface/Mission Hot-Spot/Set visibility of mission hot-spot[508])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set visibility of mission hot-spot ${MissionHotSpot} to ${Boolean}.)#",

},

{

    .index = "509",

    .id = "PLAYER_SET_ALLY_BUILDABILITY",

    .name = R"#(Player/Build/Player cannot build in a specific ally's buildability radii (ground control).[509])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} is allowed to build in ${PlayerName} controlled territory: ${NewBoolean})#",

},

{

    .index = "510",

    .id = "FADE_OUT_TO_BLACK",

    .name = R"#(Interface/Fade/Fade-out to black[510])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Fade-out the entire screen to black in ${RealNumber} seconds.)#",

},

{

    .index = "511",

    .id = "FADE_IN_FROM_BLACK",

    .name = R"#(Interface/Fade/Fade-in from black[511])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Fade-in the entire screen from black in ${RealNumber} seconds.)#",

},

{

    .index = "512",

    .id = "NAMED_OUTLINE",

    .name = R"#(Object Type/Flash or Color/Outline a specific unit with an outline type.[512])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${UnitName} is set to ${ObjectOutline} outline type.)#",

},

{

    .index = "513",

    .id = "MAP_SHROUD_SET_RESHROUD_TIME",

    .name = R"#(Map/Fog or Reveal/Set Re-Fog delay.[513])#",

    .help = R"#(This will set the length of time it takes for the fog of war to come back after a unit leaves the area.  Defined in seconds, can use decimals like .5 for half a second.)#",

    .argHint = R"#(Set Fog Delay(In Seconds) to ${RealNumber})#",

},

{

    .index = "514",

    .id = "SET_MISSION_HOT_SPOT_ACTIVENESS",

    .name = R"#(Interface/Mission Hot-Spot/Set mission hot-spot to be active or inactive[514])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set 'active' status of mission hot-spot ${MissionHotSpot} to ${Boolean}.)#",

},

{

    .index = "515",

    .id = "SKIRMISH_AI_SET_ENEMY_PLAYER_PRIORITY",

    .name = R"#(SkirmishAI/Enemy Target Control/Set enemy player priority[515])#",

    .help = R"#(AI's target heuristics will always choose a higher-priority enemy's stuff over a lower-priority enemy's stuff, if possible. Priority of zero means the target heuristics will never consider that enemy.)#",

    .argHint = R"#(Skirmish AI for ${PlayerName}: Give enemy player ${PlayerName} a target priority of ${Integer})#",

},

{

    .index = "516",

    .id = "SKIRMISH_AI_CLEAR_ENEMY_PLAYER_PRIORITY",

    .name = R"#(SkirmishAI/Enemy Target Control/Clear enemy player priority[516])#",

    .help = R"#(AI's target heuristics will will never target this enemy's objects.)#",

    .argHint = R"#(Skirmish AI for ${PlayerName}: Does not target enemy player ${PlayerName})#",

},

{

    .index = "517",

    .id = "SKIRMISH_AI_SET_ALL_ENEMY_PLAYERS_PRIORITIES",

    .name = R"#(SkirmishAI/Enemy Target Control/Set all enemy players' priority[517])#",

    .help = R"#(AI's target heuristics will always choose a higher-priority enemy's stuff over a lower-priority enemy's stuff, if possible. Priority of zero means the target heuristics will never consider that enemy.)#",

    .argHint = R"#(Skirmish AI for ${PlayerName}: Give all enemy players a target priority of ${Integer})#",

},

{

    .index = "518",

    .id = "SKIRMISH_AI_CLEAR_ALL_ENEMY_PLAYERS_PRIORITIES",

    .name = R"#(SkirmishAI/Enemy Target Control/Clear all enemy players' priority[518])#",

    .help = R"#(AI's target heuristics will not target any enemies until 'Set enemy player priority' is called )#",

    .argHint = R"#(Skirmish AI for ${PlayerName}: Does not target any enemy player.)#",

},

{

    .index = "519",

    .id = "SKIRMISH_AI_USE_AUTOMATIC_ENEMY_PLAYER_PRIORITY_SCHEME",

    .name = R"#(SkirmishAI/Enemy Target Control/Use automatic enemy player prioritization scheme[519])#",

    .help = R"#(AI automatically prioritizes enemy players using the given scheme)#",

    .argHint = R"#(Skirmish AI for ${PlayerName}: Choose enemy players automatically using ${SkimishPriorityScheme})#",

},

{

    .index = "520",

    .id = "SKIRMISH_AI_USE_DEFAULT_AUTOMATIC_ENEMY_PLAYER_PRIORITY_SCHEME",

    .name = R"#(SkirmishAI/Enemy Target Control/Use default automatic enemy player prioritization scheme[520])#",

    .help = R"#(AI automatically prioritizes enemy players using the default scheme for the AI's difficulty & personality)#",

    .argHint = R"#(Skirmish AI for ${PlayerName}: Choose enemy players automatically using default prioritization scheme)#",

},

{

    .index = "521",

    .id = "REDALERT_DISABLE_THREAT_LEVEL_BONUS",

    .name = R"#(Player/Red Alerts/Disable Threat Level Bonus[521])#",

    .help = R"#(This will disable the attribute modifiers associated with each area of the red alert system(yellow, orange, red).  Currently these give modifiers to this players objects experience gain.)#",

    .argHint = R"#(Disable ${PlayerName} threat level bonus.)#",

},

{

    .index = "522",

    .id = "SET_COOP_AI_COMMAND_UI_ENABLED",

    .name = R"#(Interface/Enable or disable the co-op AI command panel[522])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set the enabled status of the co-op AI command panel to ${Boolean})#",

},

{

    .index = "523",

    .id = "SKIRMISH_AI_CONSIDER_ENEMY_WHEN_BUILDING",

    .name = R"#(SkirmishAI/Build Control/Consider specific enemy when deciding what to build[523])#",

    .help = R"#(AI considers the units owned by the given enemy when evaluating BEST_UNIT_EXPECTED, BEST_UNIT_WEIGHTED, WORST_UNIT, FACTORY, and base defense build heuristics. This enemy's units will be weighed the same as those of all the other enemies the AI has been told to consider. By default, the AI considers all enemies, so this is only useful after telling the AI to not consider all the enemies.)#",

    .argHint = R"#(Skirmish AI for ${PlayerName}: Consider ${PlayerName}'s units when deciding what the best or worst unit to build is.)#",

},

{

    .index = "524",

    .id = "SKIRMISH_AI_DONT_CONSIDER_ENEMY_WHEN_BUILDING",

    .name = R"#(SkirmishAI/Build Control/Don't consider specific enemy when deciding what to build[524])#",

    .help = R"#(AI does not consider the units owned by the given enemy when evaluating BEST_UNIT_EXPECTED, BEST_UNIT_WEIGHTED, WORST_UNIT, FACTORY, and base defense build heuristics. )#",

    .argHint = R"#(Skirmish AI for ${PlayerName}: Does not consider ${PlayerName}'s units when deciding what the best or worst unit to build is.)#",

},

{

    .index = "525",

    .id = "SKIRMISH_AI_CONSIDER_ALL_ENEMIES_WHEN_BUILDING",

    .name = R"#(SkirmishAI/Build Control/Consider all enemies when deciding what to build[525])#",

    .help = R"#(AI considers the units owned by the all its enemies equally when evaluating BEST_UNIT_EXPECTED, BEST_UNIT_WEIGHTED, WORST_UNIT, FACTORY, and base defense build heuristics. This is the default behavior.)#",

    .argHint = R"#(Skirmish AI for ${PlayerName}: Considers all enemies' units when deciding what the best or worst unit to build is.)#",

},

{

    .index = "526",

    .id = "SKIRMISH_AI_DONT_CONSIDER_ANY_ENEMIES_WHEN_BUILDING",

    .name = R"#(SkirmishAI/Build Control/Don't consider any enemies when deciding what to build[526])#",

    .help = R"#(AI does not consider the units owned by the any its enemies when evaluating BEST_UNIT_EXPECTED, BEST_UNIT_WEIGHTED, WORST_UNIT, FACTORY, and base defense build heuristics. Without some enemy units to consider, the AI will have no idea what to build, and will just build randomly. It is expected that you will call 'Consider specific enemy when deciding what to build' after this to give the AI something to build against.)#",

    .argHint = R"#(Skirmish AI for ${PlayerName}: Do not consider any enemies' units when deciding what the best or worst unit to build is.)#",

},

{

    .index = "528",

    .id = "PLAYER_GRANT_SPECIAL_POWER",

    .name = R"#(Player/Special Power/Grant power to player.[528])#",

    .help = R"#(This will grant a power to the player, it will grant whatever tech level it needs to access the power.)#",

    .argHint = R"#(Grant ${SpecialPowerName}to player ${PlayerName})#",

},

{

    .index = "529",

    .id = "SKIRMISH_AI_TRANSFER_ALL",

    .name = R"#(SkirmishAI/Transfer ownership of all player's units to or from Skirmish AI -- Don't Enable Skirmish AI.[529])#",

    .help = R"#(Transfers control of all a players units to or from the Skirmish AI, note it does not start its timers or opening moves (doesn't turn the AI on if transferring to the AI).)#",

    .argHint = R"#(Turn Skirmish AI for ${PlayerName} ${NewBoolean} without starting opening moves and timers.)#",

},

{

    .index = "530",

    .id = "SKIRMISH_AI_START",

    .name = R"#(SkirmishAI/Start AI Opening moves and timers(Turn on the Skirmish AI).[530])#",

    .help = R"#(Units that have been transferred to the SkirmishAI will now start executing their opening moves and timers.  Ensure you have needed components of Skirmish AI turned on in the player list.)#",

    .argHint = R"#(Start SkirmishAI for side: ${PlayerName} )#",

},

{

    .index = "531",

    .id = "PLAYER_CAMERA_MOVE_TO",

    .name = R"#(Cinematics/Camera/Move the camera for player to a location.[531])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Move ${PlayerName} camera to ${CameraLocation} in ${RealNumber} seconds, camera shutter ${RealNumber} seconds, ease-in ${RealNumber} seconds, ease-out ${RealNumber} seconds.)#",

},

{

    .index = "532",

    .id = "PLAYER_CAMERA_MOVE_TO_NAMED",

    .name = R"#(Cinematics/Camera/Move the camera for player to a named object's location.[532])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Move ${PlayerName} camera to ${UnitName} in ${RealNumber} seconds, camera shutter ${RealNumber} seconds, ease-in ${RealNumber} seconds, ease-out ${RealNumber} seconds.)#",

},

{

    .index = "533",

    .id = "PLAYER_SET_MAX_PLAYER_POWER_POINTS",

    .name = R"#(Player/Special Power/Set maximum player power points.[533])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} set maximum power points to ${Integer}.)#",

},

{

    .index = "534",

    .id = "PLAYER_LOCK_PLAYER_TECH",

    .name = R"#(Player/Special Power/Lock or Unlock a player tech.[534])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} set ${PlayerTech} locked to ${Boolean}.)#",

},

{

    .index = "535",

    .id = "ALLOW_DISALLOW_ONE_UPGRADE",

    .name = R"#(Player/Upgrades and Sciences/Allow or Disallow one upgrade[535])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} allowed to build ${UpgradeName} is ${Boolean}.)#",

},

{

    .index = "536",

    .id = "NAMED_SET_DISABLED",

    .name = R"#(Object Type/Settings/Disable -- enable or disable.[536])#",

    .help = R"#(Enable/Disable a unit by type.  The types you should most be interested in are UNMANNED(pilot sniped), HELD(Standard disable), EMP(No Power).  The others can be used but the ones listed here are probably the most popular.)#",

    .argHint = R"#(Set  ${UnitName} ${ObjectDisableType} to ${Boolean}.)#",

},

{

    .index = "537",

    .id = "TEAM_SET_STANCE",

    .name = R"#(Team/Settings/Set a Team's current stance[537])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Sets ${TeamName} stance to ${ObjectStance}.)#",

},

{

    .index = "538",

    .id = "NAMED_SHOW_INFOBOX",

    .name = R"#(Interface/Info Box/Show Info Box[538])#",

    .help = R"#(Displays an info box over the top of the named object.  Optionally the 4th parameter takes a string of an image id to display an image.  If the image is found it will display this image in the info box.  Note that a time of 0 will result in a never ending info box that will only be removed when the script Remove Info Box is used on the unit.)#",

    .argHint = R"#( ${UnitName} Show info box text ${LocalizedStringName} for ${RealNumber} seconds.  Display Image from button ${Text})#",

},

{

    .index = "539",

    .id = "NAMED_REMOVE_INFOBOX",

    .name = R"#(Interface/Info Box/Remove Info Box[539])#",

    .help = R"#(Removes.....wait for it....  The info box on this unit.)#",

    .argHint = R"#( ${UnitName} remove info box)#",

},

{

    .index = "540",

    .id = "PLAYER_TOGGLE_RESOURCE_SHARING",

    .name = R"#(Player/Money/Toggle allied resource sharing on-off.[540])#",

    .help = R"#(Toggles resource sharing for this player on/off.  When on this player will share all gathered resources with it's allies.)#",

    .argHint = R"#( ${PlayerName} turn resource sharing:  ${NewBoolean})#",

},

{

    .index = "541",

    .id = "TEAM_FOLLOW_WAYPOINTS_STARTING_AT_WAYPOINT",

    .name = R"#(Team/Move/Set team to follow a waypoint path starting at waypoint.[541])#",

    .help = R"#(Please note that this script will allow you to specify a Waypoint to start on but doesn't check if that Waypoint is part of the path you specified.  Please be careful with this.)#",

    .argHint = R"#(Have ${TeamName} follow ${WaypointPathName} , as a team is ${Boolean} , in formation is ${Boolean} , starting at waypoint ${WaypointName})#",

},

{

    .index = "542",

    .id = "NAMED_FOLLOW_WAYPOINTS_STARTING_AT_WAYPOINT",

    .name = R"#(Object Type/Move/Named object follow a waypoint path starting at waypoint.[542])#",

    .help = R"#(Please note that this script will allow you to specify a Waypoint to start on but doesn't check if that Waypoint is part of the path you specified.  Please be careful with this.)#",

    .argHint = R"#( ${UnitName} follow ${WaypointPathName} starting at ${WaypointName})#",

},

{

    .index = "543",

    .id = "PATH_MUSIC_PLAY_EVENT",

    .name = R"#(Media/Audio/PathfinderMusic/Play Event.[543])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Play a specific Path Music Event(${PathMusic}), also Suspends Dynamic State System( if running ).)#",

},

{

    .index = "544",

    .id = "PATH_MUSIC_DISABLE_DYNAMIC_SYSTEM",

    .name = R"#(Media/Audio/PathfinderMusic/Disable Dynamic System.[544])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Disable the Path Music Dynamic State System)#",

},

{

    .index = "545",

    .id = "PATH_MUSIC_ENABLE_DYNAMIC_SYSTEM",

    .name = R"#(Media/Audio/PathfinderMusic/Enable Standard Dynamic System.[545])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Enable the Standard Path Music Dynamic State System)#",

},

{

    .index = "546",

    .id = "PATH_MUSIC_STOP_MUSIC",

    .name = R"#(Media/Audio/PathfinderMusic/Stop Music.[546])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Stop any Path Music currently playing.)#",

},

{

    .index = "547",

    .id = "UNIT_SET_MODELCONDITION_GENERIC",

    .name = R"#(Object Type/Settings/Set a model condition flag on a named object for duration(optional).[547])#",

    .help = R"#(Sets a model state on the given object. Can optionally set the state to expire after (x) seconds, 0 sets the state permanently until manually cleared via the cleear model state script.)#",

    .argHint = R"#( ${UnitName} sets ${ModelCondition} for ${RealNumber} seconds(optional). )#",

},

{

    .index = "548",

    .id = "PLAYER_SET_PLAYER_POWER_POINTS",

    .name = R"#(Player/Special Power/Set the players current power point total.[548])#",

    .help = R"#(Sets the current total of points for this player.  Note you cannot set this value to be greater than the maximum amount of points set via script or below 0.)#",

    .argHint = R"#( ${PlayerName} set power points to ${Integer}.)#",

},

{

    .index = "549",

    .id = "NAMED_FORCE_ATTACK_NAMED",

    .name = R"#(Object Type/Attack/Named Object force attack a named object.[549])#",

    .help = R"#(Sets a named object to force attack another named object. Useful to make friendly units attack each other.)#",

    .argHint = R"#( ${UnitName} force attacks ${UnitName})#",

},

{

    .index = "550",

    .id = "NAMED_FORCE_ATTACK_WAYPOINT",

    .name = R"#(Object Type/Attack/Named Object force attack a waypoint.[550])#",

    .help = R"#(Sets a named object to force attack a waypoint. Useful to make a unit bombard the ground at a particular location.)#",

    .argHint = R"#( ${UnitName} force attacks ${WaypointName})#",

},

{

    .index = "552",

    .id = "LINK_MISSION_OBJECTIVE_TO_WAYPOINT_WITH_CATEGORY",

    .name = R"#(Interface/Objectives/Link mission objective to waypoint using icon[552])#",

    .help = R"#(Links an objective to a waypoint and specifies a particular icon type to display.)#",

    .argHint = R"#(Set mission objective '${MissionObjective}' linked to ${WaypointName} to ${Boolean} using icon ${MissionObjectiveCategory}.)#",

},

{

    .index = "553",

    .id = "LINK_MISSION_OBJECTIVE_TO_OBJECT_WITH_CATEGORY",

    .name = R"#(Interface/Objectives/Link mission objective to object using icon[553])#",

    .help = R"#(Links an objective to an object and specifies a particular icon type to display.)#",

    .argHint = R"#(Set mission objective '${MissionObjective}' linked to ${UnitName} to ${Boolean} using icon ${MissionObjectiveCategory}.)#",

},

{

    .index = "554",

    .id = "CAMPAIGN_CREATE_AUTO_SAVE",

    .name = R"#(Campaign/Create campaign auto save checkpoint.[554])#",

    .help = R"#(Create a new auto save checkpoint with a description. This script will work both in single player with an AI buddy and in Co-Op.)#",

    .argHint = R"#(Create auto save checkpoint with description: ${LocalizedStringName}.)#",

},

{

    .index = "555",

    .id = "PATH_MUSIC_ENABLE_SPECIFIC_DYNAMIC_SYSTEM",

    .name = R"#(Media/Audio/PathfinderMusic/Enable Specific Dynamic System.[555])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Enables a specific Path Music Dynamic State System(${PathMusicDynamicSystem}))#",

},

{

    .index = "556",

    .id = "PATH_MUSIC_ENABLE_RESUME_SYSTEM",

    .name = R"#(Media/Audio/PathfinderMusic/Resume Dynamic System.[556])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Resume a Suspended Path Music Dynamic State System)#",

},

{

    .index = "557",

    .id = "FLASH_JOYPAD_COMMANDBAR_BUTTON",

    .name = R"#(Tutorial/Flash console command menu button[557])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set flash console command menu button ${CommandMenuButton} to ${Boolean}.)#",

},

{

    .index = "558",

    .id = "DISABLE_JOYPAD_COMMANDBAR_BUTTON",

    .name = R"#(Tutorial/Disable console command menu button[558])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set disable console command menu button ${CommandMenuButton} to ${Boolean}.)#",

},

{

    .index = "559",

    .id = "FLASH_JOYPAD_HUD_FIXED_ELEMENT",

    .name = R"#(Tutorial/Flash console HUD fixed element[559])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set flash console HUD fixed element ${FixedHUD} to ${Boolean}.)#",

},

{

    .index = "560",

    .id = "DISABLE_JOYPAD_HUD_FIXED_ELEMENT",

    .name = R"#(Tutorial/Disable console HUD fixed element[560])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Set disable console HUD fixed element ${FixedHUD} to ${Boolean}.)#",

},

{

    .index = "561",

    .id = "SHOW_UNIT_INTRO",

    .name = R"#(Interface/Show unit intro[561])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Show unit intro for ${ObjectName}.)#",

},

{

    .index = "562",

    .id = "PLAYER_SET_ALL_UNITS_TO_FLEE",

    .name = R"#(Player/Retreat/Set all units on player to flee off map.[562])#",

    .help = R"#(Units that need to flee are required to have a RunOffMapBehavior in their object definition for the unit to be affected.  Any units without it will ignore this script.  The RunOffMapBehavior has two modes of operation one is to have the unit run to a waypoint specified in the behavior or go 1000 units in the direction the unit is currently facing, if it can't path to either point the unit will kill itself.  Please be careful with this as some units may not be able to go to whatever exit point and will kill themselves at their current location.)#",

    .argHint = R"#(Set ${PlayerName} units to flee off map)#",

},

{

    .index = "563",

    .id = "CAMERA_LETTERBOX_BEGIN_WITH_RADAR",

    .name = R"#(Cinematics/Settings/Start letterbox mode(shows radar).[563])#",

    .help = R"#(This will display letterbox mode and leave the radar up.)#",

    .argHint = R"#(Start letterbox mode (hide UI, add border), but keep the Radar visible.)#",

},

{

    .index = "564",

    .id = "PATH_MUSIC_FADE_OUT_MUSIC",

    .name = R"#(Media/Audio/PathfinderMusic/Fade Out Music.[564])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Fade Out Path Music Volume( Stop Music after fade ) over (${RealNumber}) seconds.)#",

},

{

    .index = "565",

    .id = "PATH_MUSIC_FADE_IN_MUSIC",

    .name = R"#(Media/Audio/PathfinderMusic/Fade In Music.[565])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#(Fade In Path Music Volume over (${RealNumber}) seconds.)#",

},

{

    .index = "566",

    .id = "PLAYER_CAMERA_TETHER_NAMED",

    .name = R"#(Cinematics/Camera/Begin tether player's camera to a specific unit.[566])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${PlayerName} have player camera tethered to ${UnitName}.  Snap camera to object is ${Boolean}.  Amount of play is ${RealNumber}.)#",

},

{

    .index = "567",

    .id = "PLAYER_ALLOWED_TOBE_COOPAI",

    .name = R"#(Interface/Toggle CoOp AI interface.[567])#",

    .help = R"#(Toggle the CoOp AI's interface on and off.  It defaults to on.)#",

    .argHint = R"#(Set ${PlayerName} CoOp AI's interface to ${Boolean})#",

},

{

    .index = "568",

    .id = "PLAYER_TOGGLE_SKIRMISH_AI",

    .name = R"#(SkirmishAI/Toggle SkirmishAI On-Off.[568])#",

    .help = R"#(The SkirmishAI is on by default.  Toggling it off will shut it down completely.  This is different than the scripts which transfer assets between the default player team and the default skirmish team.  It is only necessary to use this for performance reasons.)#",

    .argHint = R"#(Set ${PlayerName} to toggle SkirmishAI ${Boolean})#",

},

{

    .index = "569",

    .id = "PLAY_SOUND_AS_AI_COCOMMANDER_DIALOG",

    .name = R"#(Media/Audio/EVA/Play a sound effect as AI co-commander dialog.[569])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${SoundName} plays as AI co-commander dialog)#",

},

{

    .index = "570",

    .id = "PLAY_SOUND_AS_AI_COCOMMANDER_DIALOG_WITH_SUBTITLE",

    .name = R"#(Media/Audio/EVA/Play a sound effect as AI co-commander dialog, and subtitle it.[570])#",

    .help = R"#(No help text has been defined for this script. Contact your local help text provider for details. Have a nice day!)#",

    .argHint = R"#( ${SoundName} plays as AI co-commander dialog, while displaying ${LocalizedStringName} for ${RealNumber} seconds.)#",

},

{

    .index = "571",

    .id = "PLACE_AI_BEACON_LOCATION",

    .name = R"#(SkirmishAI/Beacon/Spawn an AI Beacon at a waypoint.[571])#",

    .help = R"#(Spawns an AI Beacon of a given type at a waypoint and informs the AI of it's placement.)#",

    .argHint = R"#( ${PlayerName} place beacon type: ${AiBeaconType} at waypoint: ${WaypointName})#",

},

{

    .index = "572",

    .id = "PLACE_AI_BEACON_OBJECT",

    .name = R"#(SkirmishAI/Beacon/Spawn an AI Beacon on a named object[572])#",

    .help = R"#(Spawns an AI Beacon of a given type and attaches it to a named object.)#",

    .argHint = R"#( ${PlayerName} place beacon type: ${AiBeaconType} on unit: ${UnitName})#",

},

        };

    }

}
