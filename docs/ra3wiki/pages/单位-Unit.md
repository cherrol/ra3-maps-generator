---
title: "单位-Unit"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/object"
slug: ozofvf18pu9gxs2g
source: https://www.yuque.com/muzeqaq/ra3mapwiki/ozofvf18pu9gxs2g
fetched: 2026-10-15
content_updated_at: "2025-10-15T13:21:13.000Z"
type: Doc
format: lake
---

# 单位-Unit

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/ozofvf18pu9gxs2g
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/object

:::success
作者: dreamness

:::

Unit是一个类, 不能直接使用. 需要使用UnitHelper来构建, 或者是其他函数的返回值



# 字段
| 字段名 | 数据类型 | 备注 |
| --- | --- | --- |
| id | number | 单位的id |
| name | string | nil | 单位的name, 除非指定, 否则会自动生成 |
| unit_table | SystemUnitTable | 单位的unit_table |


# 方法
:::color3
这里的方法需要使用冒号来调用, 如:

test_unit:get_owner_player()

:::

### 系统
#### set_name
给单位命名

除非有特殊需求, 一般不需要调用, Unit会自动命名

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| name | string |  |




#### equals
判断与另一个unit是否为同一单位

返回值: boolean

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| other_unit | Unit |  |




#### is_thing
单位是否是指定的物体

返回值: boolean

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| thing_name | ThingEnum | 物体名 |


### 单位所属
例程:[1_脑车](https://www.yuque.com/muzeqaq/ra3mapwiki/nka8b2x5fkewmo0r)

#### get_owner_player
获取所属玩家, 不能在第一帧调用

返回值: PlayerEnum



#### set_owner_player
设置所属玩家

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_name | PlayerEnum | 玩家名 |


### 单位存活
#### delete
删除单位, 不会触发死亡事件, 不建议使用

#### kill
杀死单位

#### is_alive
单位是否存活

返回值: boolean



### 单位使用技能
#### use_ability
单位使用技能

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| ability_name | AbilityEnum | 技能名 |




#### use_ability_at_waypoint
单位在路径点使用技能

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| ability_name | AbilityEnum | 技能名 |
| waypoint | string | 路径点名字 |




#### use_ability_on_unit
在其他单位上使用技能

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| ability_name | AbilityEnum | 技能名 |
| target_unit | Unit | 目标单位 |




### 单位状态
#### add_status
添加单位状态

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| statuses | StatusEnum|StatusEnum[] | 状态 或 状态数组 |


#### remove_status
移除单位状态

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| statuses | StatusEnum|StatusEnum[] | 状态 或 状态数组 |




#### is_has_status
单位是否拥有该状态

返回值: boolean

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| status | StatusEnum | 状态名 |


### 单位属性(buff)
#### add_modifier
添加buff

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| modifiers | AttributeModifierEnum | AttributeModifierEnum[] | 属性修改器 或 属性修改器数组 |
| frame_cnt | number | nil | 持续帧数, nil为默认值 |


#### remove_modifier
移除buff

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| modifiers | AttributeModifierEnum | AttributeModifierEnum[] | 属性修改器 或 属性修改器数组 |




### 单位模型状态
#### add_model_state
添加模型状态

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| model_states | ModuleStateEnum | ModuleStateEnum[] | 模型状态 或 模型状态数组 |
| duration_seconds | number | nil | 持续秒数, nil为默认值 |


#### remove_model_state
移除模型状态

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| model_states | ModuleStateEnum | ModuleStateEnum[] | 模型状态 或 模型状态数组 |


#### is_has_model_state
单位是否拥有模型状态

返回值: boolean

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| model_states | ModuleStateEnum | ModuleStateEnum[] | 模型状态 或 模型状态数组 |




### 单位健康
#### get_max_health
获取单位最大命值

返回值: number

#### set_max_health
设置单位的最大生命值

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| max_health | number |  |


#### get_health
获取单位生命值

返回值: number

#### set_health
设置单位的生命值

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| health | number |  |


#### set_health_percentage
设置单位的生命值百分比

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| percentage | number 整数,  | 相对于最大生命的百分比 |




#### suffer_damage
单位承受伤害

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| point | number | 单位承受的伤害值 |


### 武器
#### fire_with_temp_weapon
使用临时武器开火, 例程: [1_脑车](https://www.yuque.com/muzeqaq/ra3mapwiki/nka8b2x5fkewmo0r)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| weapon_name | WeaponEnum | 武器名字 |
| x | number |  |
| y | number |  |
| z | number |  |
| target_unit | Unit | nil | 目标单位 |


#### get_weapon_ammo_cnt
获取单位武器弹药量

返回值: number 弹药量

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| weapon_slot_id | number | 武器槽位id |
| weapon_ording | number | 武器排序 |
| weapon_index | number | 武器索引 |


#### set_weapon_ammo_cnt
设置单位武器弹药量

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| weapon_slot_id | number | 武器槽位id |
| weapon_ording | number | 武器排序 |
| weapon_index | number | 武器索引 |
| ammo_cnt | number | 弹药数量 |


#### get_weapon_max_ammo_cnt
获取单位武器最大弹药量

返回值: number 弹药量

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| weapon_slot_id | number | 武器槽位id |
| weapon_ording | number | 武器排序 |
| weapon_index | number | 武器索引 |




### 进驻
#### garrison_other_unit
进驻可进驻单位(建筑物,ifv等), 例程: [13 单位进驻](https://www.yuque.com/muzeqaq/ra3mapwiki/xpqpqpu91gkt3kvb)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| parent_unit | Unit | 要进驻的单位 |


#### cancel_garrison
离开可进驻单位(建筑物,ifv等)

#### empty_garrison_inside
清空已进驻的单位

#### get_parent_unit
获取单位的父单位

返回值: Unit

#### get_child_units
获取子单位列表

返回值: Unit[] 子单位列表, number 子单位数量

### 升级
#### is_enabled_upgrade
是否拥有某项升级

返回值:boolean

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| upgrade_name | UpgradeEnum |  |


#### enable_upgrade
允许/禁止单位升级

返回值:boolean

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| upgrade_name | UpgradeEnum |  |
| is_allow | boolean |  |




### 视野
#### set_fog_clearing_distance
设置单位清雾距离

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| distance | number, 整数 | 距离 |


#### set_vision_range
设置单位视线距离

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| distance | number, 整数 | 距离 |


### 心灵控制
#### get_temporary_owned_unit
获取当前物体正在控制的单位的id  (此函数针对有控制技能的物体， 例如 JapanPsychicInhibitor )

返回值: Unit

### 攻击
#### attack_unit
攻击单位

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| target_unit | Unit | 目标单位单位 |
| is_force_attack | boolean | 是否强制攻击, 用于攻击友军等 |


#### attack_waypoint
攻击路径点

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| waypoint_name | string | 路径点 |


#### move_attack_to_waypoint
移动攻击到路径点

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| waypoint_name | string | 路径点 |


#### move_attack_to_nearest_thing
移动攻击到最近的某类型目标

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| nearby_thing | ThingEnum |  |


#### hunt
捕猎, 自动搜寻地图上的敌人

注意, 不能用于人类玩家的单位



#### get_target_unit
获取单位目标单位

返回值: Unit | nil  如果没有目标则返回nil

### 物理信息
#### get_position
获取单位坐标

返回值: number x, number y, number z

#### get_position_vec
获取单位坐标向量

返回值: Vector

#### set_position
设置单位坐标

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| x | number |  |
| y | number |  |
| z | number | nil | 可以缺省为nil |


#### set_position_by_vec
设置单位坐标向量

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| vec | Vector |  |


#### move_to
单位移动到指定坐标, 参数为整数. 涉及到游戏状态机, 慎用

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| x | number |  |
| y | number |  |
| z | number |  |


#### get_distance_2D
获取与另一个单位的2D距离

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| other_unit | Unit | 目标单位 |


#### get_distance_3D
获取与另一个单位的3D距离

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| other_unit | Unit | 目标单位 |


#### set_model_radius
设置单位模型体积

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| radius | number, 整数 | 半径 |


#### get_speed
获取单位速度

返回值: number

#### set_speed
设置单位速度（注意，这个速度在单位下海，上陆地，变形，回到机场 之后会失效）

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| speed | number |  |


#### set_fixed_scale
设置物体缩放（永久缩放，和地编那个一样）

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| scale | number |  |




### 齐次坐标相关
使用教程: [齐次坐标矩阵研究](https://www.yuque.com/muzeqaq/ra3mapwiki/mvato6ot39cg8s8y)

#### get_homogeneous_coordinates
获取单位的齐次坐标矩阵

返回值: Matrix

#### set_homogeneous_coordinates
设置单位的齐次坐标矩阵

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| matrix | Matrix | 齐次坐标矩阵 |


#### get_direction_vec
获取单位的方向向量

返回值: Vector

#### translate
平移单位(全局坐标)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| vec | Vector |  |


#### translate_relative
平移单位： [1-齐次坐标-平移](https://www.yuque.com/muzeqaq/ra3mapwiki/ftxfgl5v83smph69)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| vec | Vector |  |


#### rotate_by_euler
欧拉角旋转, 具体的旋转方向可使用右手定则确定: [3-齐次坐标-旋转](https://www.yuque.com/muzeqaq/ra3mapwiki/nhyqsa4wbuodfz7d)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| roll_angle | number |  |
| pitch_angle | number |  |
| yaw_angle | number |  |


#### set_scale
缩放单位: [2-齐次坐标-缩放](https://www.yuque.com/muzeqaq/ra3mapwiki/mh3xv48u7ba29a4b)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| scale | number | 缩放比例 |


#### get_scale
获取单位的缩放比例

返回值: number 在x轴的缩放比例, number 在y轴的缩放比例, number 在z轴的缩放比例

#### set_mirror
单位镜像, 模型可能会出问题!

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| x | boolean | 是否在x轴镜像 |
| y | boolean | 是否在y轴镜像 |
| z | boolean | 是否在z轴镜像 |


### 高亮
例程: [19 单位高亮](https://www.yuque.com/muzeqaq/ra3mapwiki/gg61vgne1dpfgdxz)

#### highlight_white_flash
单位白色闪烁

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| seconds | number | 闪烁时间(秒) |


#### highlight_flash
单位闪烁

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| seconds | number | 闪烁时间(秒) |


#### highlight_outline
单位闪烁

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| outline_type | UnitOutlineTypeEnum | 轮廓类型 |


### 头顶文字
例程: [18 单位头顶文字/漂浮数字](https://www.yuque.com/muzeqaq/ra3mapwiki/dirwwu7pavuyqt5c)， [11_文本框](https://www.yuque.com/muzeqaq/ra3mapwiki/oigo98mvdi9d4m96)

#### set_top_text
在单位上方添加或更新文字

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| content | string | 文字内容 |
| z_offset | number | z轴偏移量 |
| color | Color | 颜色 |
| player_name_list | string|string[]|nil | 玩家名字列表，nil表示所有玩家 |


#### remove_top_text
删除单位顶部文字

#### show_float_int_number
显示浮动整数数字

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| value | number 整数 |  |


#### show_msg_box
显示单位头顶消息框

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| msg | msg | 消息内容 |
| seconds | seconds | 持续时间(秒) |
| img | ButtonTextureEnum|nil | 图片名称 |


#### remove_msg_box
移除单位头顶消息框

### 头顶按钮
例程: [17 单位头顶按钮](https://www.yuque.com/muzeqaq/ra3mapwiki/ev5rzdes28gxlpxa)

#### add_top_button
添加顶部按钮

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| offset | number | z轴偏移量 |
| player_list | string|string[]|nil | 玩家名字列表, nil表示所有玩家 |


#### remove_top_button
删除顶部按钮

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| player_list | string|string[]|nil | 玩家名字列表, nil表示所有玩家 |


#### set_top_button_visible
设置顶部按钮是否可见

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| visible | boolean | 是否可见 |
| player_list | string|string[]|nil | 玩家名字列表, nil表示所有玩家 |


### 造价
#### get_money_value
获取单位造价

返回值: number

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| producer_unit | Unit | nil | 生产建筑 |


### 集结点
#### get_rally_pos
获取单位集结点坐标

返回值: number, number, number



#### set_rally_pos
设置单位集结点坐标

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| x | number |  |
| y | number |  |
| z | number |  |


### 附着物
#### get_attachers
获取单位附着物体

返回值: Unit[] 附着物体列表, number 附着物体数量



#### get_attachee
获取单位被附着物体

返回值: Unit[] 被附着物体列表, number 被附着物体数量



#### attach_to_other_unit
附着到其他物体

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| be_attached_unit | Unit |  |


#### add_attacher
添加附着物体

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| attachable_unit | Unit |  |


#### add_attacher_by_name
添加附着物体

返回值: Unit

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| attachable_thing_name | AttachableThingEnum | 附着物体名称 |
| tmp_waypoint_name | string | 临时路径点名称 |
