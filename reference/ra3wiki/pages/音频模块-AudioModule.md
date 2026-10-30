---
title: "音频模块-AudioModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: dl0pcyz82b9fbczm
source: https://www.yuque.com/muzeqaq/ra3mapwiki/dl0pcyz82b9fbczm
fetched: 2026-10-15
content_updated_at: "2025-10-16T09:21:15.000Z"
type: Doc
format: lake
---

# 音频模块-AudioModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/dl0pcyz82b9fbczm
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::



例程: [24 音频](https://www.yuque.com/muzeqaq/ra3mapwiki/vucxymsgake86bgs)

### 函数
#### 音效
##### set_sound_volume
设置音效音量

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| sound_name | SoundEnum | nil | 音效名称, nil表示全局音量 |
| volume | number|nil | 音量, 0-100, nil表示恢复默认音量 |


##### enable_sound
启用/禁用音效

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| sound_name | SoundEnum | nil | 音效名称, nil表示所有音效, 只能用于启用所有音效, 不能用于禁用所有音效 |
| is_enable | boolean | 是否启用 (真: 启用, 假: 禁用) |


##### play_sound
播放音效

后三个参数全部为nil, 或只能有一个不为nil

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| sound_name | SoundEnum | 音效名称 |
| from_unit_table_or_name | SystemUnitTable | string | nil | 播放音效的单位 |
| from_waypoint_name | string | nil | 播放音效的路径点 |
| from_team_name | string | nil | 播放音效的队伍 |


#### EVA
##### enable_eva
启用/禁用音效

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| is_enable | boolean | 是否启用 (真: 启用, 假: 禁用) |


##### play_eva_sound
播放EVA音效, 后两个参数为选填项, 显示字幕

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| sound_name | EvaSoundEnum | 音效EVA音效名称 |
| is_from_ai_commander | boolean | 是否来自AI指挥官 (假: 来源于EVA) |
| map_str_key | map_str_key string | nil | 字幕内容的key(在map.str中定义) |
| duration | number | nil | 字幕持续时间(秒)  |


#### 音乐
##### play_music
播放音乐

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| music_name | MusicEnum | 音乐名称 |


##### stop_music
停止音乐

##### resume_dynamic_music
恢复动态音乐系统
