---
title: "存档模块-ArchiveModule"
catalog_path: "Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module"
slug: xabcd2kcbrp6ieg8
source: https://www.yuque.com/muzeqaq/ra3mapwiki/xabcd2kcbrp6ieg8
fetched: 2026-10-15
content_updated_at: "2025-11-11T15:09:35.000Z"
type: Doc
format: lake
---

# 存档模块-ArchiveModule

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/xabcd2kcbrp6ieg8
> 目录位置：Lua库 (推荐使用)（先看安装使用流程！！）/api文档/base_module

用于支持存档功能, 使用方法参考 [27 游戏存档](https://www.yuque.com/muzeqaq/ra3mapwiki/td7zxzt9eme7scyo)



# 函数
#### set_auto_load_lua_files
设置自动加载的lua文件

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| lua_script_names | string|string[] | 需要自动加载的lua文件名或文件名列表 |




#### set_save_data_function
设置保存数据函数

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| save_data_function | function | 保存数据函数 没有参数, 返回值为table(需要保存的数据) |




#### fetch_and_clean_saved_data
读取存档中已保存的数据, 只能获取调用一次, 之后会自动释放

返回值: any



#### disable_save_feature
禁用保存功能 (不可逆), 可优化性能



#### allow_save
允许/禁止保存功能 (默认允许)

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| is_allow | boolean | 是否允许玩家保存 |


#### set_save_data_callback
设置回调函数, 在触发保存时调用

| 参数名 | 数据类型 | 备注 |
| --- | --- | --- |
| 设置回调函数, 在触发保存时调用 | function |  没有参数 |


#### get_last_load_data_frame_number
获取上一次加载数据的时间帧序号. 主要用于判断当前帧是否正在加载数据

返回值: number 上一次加载数据的时间帧序号, 如果没有加载数据则返回nil
