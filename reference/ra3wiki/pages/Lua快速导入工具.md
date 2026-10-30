---
title: "Lua快速导入工具"
catalog_path: "地编伴侣"
slug: ofa6ufyd9pmwwgnm
source: https://www.yuque.com/muzeqaq/ra3mapwiki/ofa6ufyd9pmwwgnm
fetched: 2026-10-15
content_updated_at: "2026-03-08T17:47:21.000Z"
type: Doc
format: lake
---

# Lua快速导入工具

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/ofa6ufyd9pmwwgnm
> 目录位置：地编伴侣

:::color2
作者: dreamness

:::

# 一 配置lua库
![](https://cdn.nlark.com/yuque/0/2026/png/34558013/1772950828519-a5f9abef-57dc-4919-ad7f-3b58ab510276.png)

进入Lua导入工具

![](https://cdn.nlark.com/yuque/0/2024/png/34558013/1733933101455-4ae3eef1-b93f-4159-a12f-e8a42773095a.png)

## 第一列, 配置lua库
库的上下关系决定了地编中脚本文件夹的上下关系

## 第二列, 预览lua库
预览lua库, 上下关系决定了在地编中的上下关系

## 第三列, 配置脚本信息
配置第二列选中的文件或文件夹的信息

如果关闭了"包含到库中", 则该文件或文件夹不会被导入



# 三 地编导入
:::danger
新地编版本需要 >= 21

:::

![](https://cdn.nlark.com/yuque/0/2026/png/34558013/1772991877562-df10e85f-7bf6-4edc-9a19-95986255f301.png)

### 微程序: 导入LUA脚本
在地编伴侣中, 为该地图配置好lua lib后, 使用该选项可直接导入

### 微程序: 导入LUA脚本, 使用当前配置
将正在打开的"LUA导入工具"窗口中的配置, 做为本次导入的配置

# 四 可能遇到的错误
1. 导入的lua被截断了

可能是lua脚本过长导致的, 在"设置"->"Lua导入工具"->"冗余字符因子"中将参数调大

![](https://cdn.nlark.com/yuque/0/2024/png/34558013/1732393323425-47b1c656-9a25-4a6c-87d5-940bc64e4bf5.png)
