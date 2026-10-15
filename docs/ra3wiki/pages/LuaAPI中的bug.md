---
title: "LuaAPI中的bug"
catalog_path: ""
slug: ih1z2puyfrwb0gxq
source: https://www.yuque.com/muzeqaq/ra3mapwiki/ih1z2puyfrwb0gxq
fetched: 2026-10-15
content_updated_at: "2025-05-10T02:19:00.000Z"
type: Doc
format: lake
---

# LuaAPI中的bug

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/ih1z2puyfrwb0gxq
> 目录位置：(未分组)

## Enhancer 现存bug：
### lua中所有固定名字的回调函数里无法使用带有player参数的ExecuteAction脚本
比如`onCenterTopBtnClickEvent`函数中，无法调用ExecuteAction("PLAYER_GIVE_MONEY")和ExecuteAction("PLAYER_SET_MONEY")修改玩家资金，原因是引擎bug，这里上下文没有相关环境参数，无法正确处理player参数。

```lua
-- 顶部按钮点击事件
-- 定义一个固定名字的函数
function onCenterTopBtnClickEvent(playerName, btnIndex)
-- btnIndex 按钮编号，第一个按钮是1，第二个按钮是2，以此类推
end
```

解决办法： 使用全局变量更新每帧玩家在该事件下应当改变的资金量，然后用单独的帧循环调用ExecuteAction("PLAYER_GIVE_MONEY") 更新玩家资金。





## Enhancer中的性能问题：
-- 获取单位当前血量 --- 该函数存在性能问题，消耗约为 ObjectGetCurrentHealth 的5倍

`exObjectGetCurrentHealth(objectId)`



-- 获取单位最大血量 --- 该函数存在性能问题，消耗约为 ObjectGetInitialHealth 的5倍

`exObjectGetMaxHealth(objectId)`
