---
title: "Lua快速导入工具(遗弃)"
catalog_path: "地编伴侣/Lua快速导入工具"
slug: xsai4129ssh84srd
source: https://www.yuque.com/muzeqaq/ra3mapwiki/xsai4129ssh84srd
fetched: 2026-10-15
content_updated_at: "2026-03-08T06:19:34.000Z"
type: Doc
format: lake
---

# Lua快速导入工具(遗弃)

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/xsai4129ssh84srd
> 目录位置：地编伴侣/Lua快速导入工具

:::danger
旧的方式, 不再建议使用了

:::

# 一 安装插件
在"设置" -> "新地编联动" 中, 配置新地编路径即可  
![](https://cdn.nlark.com/yuque/0/2024/png/34558013/1732391066489-4c38854f-43a2-4135-b9bd-f9c0fb5ee87d.png)

# 二 配置lua库
进入Lua导入工具

![](https://cdn.nlark.com/yuque/0/2024/png/34558013/1733933101455-4ae3eef1-b93f-4159-a12f-e8a42773095a.png)

## 第一列, 配置lua库
:::color4
库的文件夹以及lua文件不能是中文! 否则会损坏地图文件!

:::

库的上下关系决定了地编中脚本文件夹的上下关系

## 第二列, 预览lua库
预览lua库, 上下关系决定了在地编中的上下关系

## 第三列, 配置脚本信息
配置第二列选中的文件或文件夹的信息

如果关闭了"包含到库中", 则该文件或文件夹不会被导入



# 三 地编导入
:::danger
导入前, 确保对应地图, 已经打开"Lua导入工具". 否则可能会导入失败或导入错误的库文件

:::

在地编中打开该地图, 点击"新功能" ->"快捷代码", 选中RA3MapUtil_LuaImporter, 即可导入.

如果在VSCode等IDE对lua进行了修改, 重新点击RA3MapUtil_LuaImporter即可快速导入.

 会有一点卡, 稍等即可.

![](https://cdn.nlark.com/yuque/0/2024/png/34558013/1731352983746-cd000a65-c314-4fef-b08e-08b3a29fbf88.png)



# 四 可能遇到的错误
1. 导入时弹窗, 失败

![](https://cdn.nlark.com/yuque/0/2025/png/34558013/1737124652191-4cb87049-26b9-4e52-94c4-30c7bcf30056.png)

请确保对应地图, 已经打开"Lua导入工具"

2. 导入没有反应, 没有导入正确的代码

对应地图, 已经打开"Lua导入工具"

3. 导入的lua被截断了

可能是lua脚本过长导致的, 在"设置"->"Lua导入工具"->"冗余字符因子"中将参数调大

![](https://cdn.nlark.com/yuque/0/2024/png/34558013/1732393323425-47b1c656-9a25-4a6c-87d5-940bc64e4bf5.png)

4. 安装问题

显示安装失败: 尝试关闭地编后再次安装.

5. 如何重新安装插件?

在设置中点击"打开插件文件夹", 进入"scripts"文件夹, 删除"RA3MapUtil_LuaImporter", 然后再在工具中重新安装插件
