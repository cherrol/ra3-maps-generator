---
title: "摄像机模块-CameraModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: lrgcmwfyo1emtwt7
source: https://www.yuque.com/muzeqaq/ra3mapwiki/lrgcmwfyo1emtwt7
fetched: 2026-10-15
content_updated_at: "2025-04-02T15:26:40.000Z"
type: Doc
format: lake
---

# 摄像机模块-CameraModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/lrgcmwfyo1emtwt7
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

:::success
作者: dreamness

:::

### 函数
#### enable_camera_interpolate
开启/关闭摄像机插值 (相机跟随相关)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| is_enable | boolean | 是否启用 (真: 启用, 假: 禁用) |


#### set_player_follow
设置相机位置, 进行相机动画, 每帧调用一次(需要先开启摄像机插值)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum | 玩家名字 |
| pos_x | number | 相机位置x |
| pos_y | number | 相机位置y |
| pos_z | number | 相机位置z |
| yaw | number | 相机旋转 |
| pitch | number | 相机俯仰 |
| roll | number | 相机滚转 |


#### set_camera_pos
设置相机位置

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum|nil | 玩家名字, nil表示所有玩家 |
| pos_x | number | 相机位置x |
| pos_y | number | 相机位置y |
| pos_z | number | 相机位置z |


#### set_camera_pos_by_vec
设置相机位置

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum|nil | 玩家名字, nil表示所有玩家 |
| pos_vec | Vector | 相机位置向量 |


#### rotate_by_euler
根据欧拉角旋转相机

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum|nil | 玩家名字, nil表示所有玩家 |
| yaw | number | 相机旋转 |
| pitch | number | 相机俯仰 |
| roll | number | 相机滚转 |


#### translate_relative
相对于单位进行平移

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum|nil | 玩家名字, nil表示所有玩家 |
| x | number |  |
| y | number |  |
| z | number |  |


#### set_camera_rotation
设置相机角度(相对于{0,1,0})

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum|nil | 玩家名字, nil表示所有玩家 |
| yaw | number | nil | 相机旋转， nil为不变化 |
| pitch | number | nil | 相机俯仰， nil为不变化 |
| roll | number | nil | 相机滚转， nil为不变化 |


#### reset_camera
重置相机, 到达指定路径点

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| waypoint_name | string | 目标路径点 |
| duration | number | 总时长(秒) |
| ease_in | number | 开始移动时, 从静止到运动的时间(秒) |
| ease_out | number | 结束移动时, 从运动到静止的时间(秒) |


#### is_camera_moving_finished
相机是否已经完成移动

返回值: boolean

#### enable_camera_letter_mode
开启/关闭电影模式

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| is_enable | boolean |  |
| with_radar | boolean | 是否带雷达 |


#### enable_camera_bloom_effect
开启/关闭辉光特效

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| is_enable | boolean |  |


####
