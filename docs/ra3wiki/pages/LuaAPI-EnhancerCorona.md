---
title: "LuaAPI-EnhancerCorona"
catalog_path: "Lua API "
slug: eksx1whrfix7tlno
source: https://www.yuque.com/muzeqaq/ra3mapwiki/eksx1whrfix7tlno
fetched: 2026-10-15
content_updated_at: "2026-07-04T03:06:32.000Z"
type: Doc
format: lake
---

# LuaAPI-EnhancerCorona

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/eksx1whrfix7tlno
> 目录位置：Lua API 

:::info
作者：物

日期：2024年12月14日

可读性与格式化：Inertia

更新日期：2024年12月19日

:::

:::info
日冕V3.22x新增的lua功能由两个dll提供，分别是EnhancerCorona（本文内容）和[LuaBridge](https://www.yuque.com/muzeqaq/ra3mapwiki/rg4gm1ibq054y6o4)。

:::

EnhancerCorona主要提供了4大类函数，分别为通用，键盘输入，地图视效，UI和按钮。

> **注意，下面列出的API本质上是注释格式，直接复制进地图将无法通过语法检查。**
>



如果你没有相关经验，请先参阅教程：

[Lua 教程（适用于日冕地图）](https://www.yuque.com/muzeqaq/ra3mapwiki/qllxru0p99ilqosx)

[日冕Lua的Hello World](https://www.yuque.com/muzeqaq/ra3mapwiki/lx066ch0tq9h13ve)

## 通用
```lua
-- 由EnhancerCorona提供

return 1;

-- 打印日志  建议只在测试时使用，发布时删去，避免影响游戏性能 日志在日冕根目录/CoronaData/Logs 里的EnhancerCorona开头的文件里
exPrintln("内容")



--计数器
-- 设置计数器 ，如果计数器还不存在会自动创建
exCounterSetByName("计数器名字", 100);
-- 获取计数器的值
local counterValue = exCounterGetByName("计数器名字");

--在左下方消息区添加新的消息
exMessageAppendToMessageArea("消息内容")




-- 在单位上方显示漂浮数字（整数，效果类似油井的那个数字）
exShowFloatingIntAtObject(int objectId, int value)
  -- objectId 单位id
  -- value 数字

-- 单位头顶显示文字相关， 当单位不存在后，会自动移除这个文字
-- 在单位头顶显示文字
exObjectShowTextAtTop(objectId, content, offset, color);
   -- objectId 单位id
   -- content 文字内容，单行
   -- offset 相对于单位z的偏移值
   -- color  颜色值，比如255 (0x0000FF)

-- 更新头顶文字
exObjectUpdateTextAtTop(objectId, content, offset， color)

--删除指定单位头顶的文字
exObjectDeleteTextAtTop(objectId)

-- 玩家特化版本
exObjectShowTextAtTopForPlayer(playerName, objectId, content, offset, color);
exObjectUpdateTextAtTopForPlayer(playerName, objectId, content, offset， color)
exObjectDeleteTextAtTopForPlayer(playerName, objectId)



-- 对物体造成冰冻伤害
-- 注意当冰冻伤害达到单位的最大血量时，单位会被冻结。但是最大冰冻值为最大血量的两倍，这允许单位被冻结后继续积攒冷冻条。
-- objectId物体id
-- amount 伤害量（整数），正数表示增加冰冻量，负数表示解冻量
int exObjectDealSecondaryDamage(int objectId, int amount)

-- 获取单位当前冰冻量数值--- 返回整数
int exObjectGetCurrentSecondaryDamage(int objectId)

-- 获取单位当前血量 --- 该函数消耗约为 ObjectGetCurrentHealth 的5倍
float exObjectGetCurrentHealth(int objectId)

-- 获取单位最大血量 --- 该函数消耗约为 ObjectGetInitialHealth 的5倍
-- objectId 单位id
float exObjectGetMaxHealth(int objectId)



-- 移动单位到某个位置  坐标都是整数
exObjectMoveTo(objectId, int x, int y, int z);
--- 注意，该命令消耗较高，在需要寻路时每次消耗30,000-50,000ns，在路径没有变化时消耗约5,000-10,000ns。
--- 作为对比，专用LUA函数更新物体状态仅需50ns。


-- 获取指定玩家当前选中的物体   比使用ObjectFilter获取要快很多
exPlayerGetSelectedObjectIds(playerName);
-- 使用例子
-- local selectedObjects, count = exPlayerGetSelectedObjects("Player_1");  -- selectedObjects是单位id数组，count是单位数量
-- for i = 1, count, 1 do
--  local selectedObjectId = selectedObjects[i]
--  -- ...
-- end



-- 获取指定玩家当前资金
exPlayerGetCurrentMoney(playerName);

-- 获取指定玩家当前产生的电力
exPlayerGetProducePower(playerName);

-- 获取指定玩家当前消耗的电力
exPlayerGetConsumePower(playerName);

-- 获取物体instanceId   每种物体都有自己唯一的instanceId。例如所有的标准模式阿库拉潜艇的id是 Ox8D0A384A, 而扩展模式的阿库拉潜艇则是OxED23C07C
exObjectGetInstanceId(objectId);

-- 获取经济作弊倍数
exModeGetCheatMultiplier();

```

## 键盘、聊天事件
```lua
--处理用户键盘事件
--首先注册按键
exRegisterHotKey(keyCode, assistKeyCode, commandCode);
  --keyCode  按键码 参考https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
  --assistKeyCode  辅助按键码，如果不需要，传0即可,仅支持VK_SHIFT,VK_TAB,VK_CONTROL   具体数值参考https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
  --commandCode  你自己定义的命令码 在处理事件回调里，会把这个命令码传给你,以便你区分是哪个功能
  -- 例如要注册 ctrl+L  热键  ，可以这样注册 exRegisterHotKey(76, 17, 1)   76是L的按键码，17是ctrl的按键码，1是你自己定义的命令码
  -- 如果不需要辅助按键码 , 比如注册 K 热键  ，可以这样注册 exRegisterHotKey(75, 0, 2)
  -- 每次检测到按键事件后，会遍历注册的事件看看是否还有其他的热键被点击了，但最多额外支持2个 （也就是每一帧最多可以检测到3个键盘事件）

--然后定义一个固定名字的函数，在收到用户键盘事件后会调用这个函数
function onUserHotKeyEvent(playerName, commandCode, mouseWorldPos)
 -- playerName  玩家脚本名字
 -- commandCode 命令码
 -- mouseWorldPos  鼠标在游戏里的位置，获取位置使用mouseWorldPos.X, mouseWorldPos.Y, mouseWorldPos.Z， 如果鼠标不在游戏窗口内，这个值是nil
 --你的处理逻辑
end




--处理用户聊天信息事件
--触发方式， Player1~6 在聊天框输入信息，信息内容以/开头
--并且/后面的信息只能有字母和数字，以及!, @, #, $, %, ^, &, *, (, ), -, _, =, +, {, }, [, ], :, ;, ", ', <, >, ,, ., ?, /, |, \,  ~和空格
--并且/后面的内容长度不超过45（也就是不超过45字节）
--违反上面这样两条规则任意一个，都不会触发这个事件
-- 定义一个固定名字的函数
function onUserShortChatMsg(playerName, msg)
  -- playerName  玩家名字
  -- msg 信息内容
end

```

## 地图视效
```lua
-- FX特效相关
--在指定地点播放特效FXParticleSystemTemplate
exFXShowFXTemplate("特效名字", x,y,z);

--在指定地点播放特效FXShowFXList
exFXShowFXList("特效名字", x,y,z);

--地形相关
-- 获取指定地点的地面高度
local height = exTerrainGetHeightByPos(1000, 3000);
-- 获取指定地点的地面贴图名字
local textureName = exTerrainGetTextureNameByPos(1000, 3000);

--改变地图光照
exLightSetMainLightColor(float r, float g, float b)  --主光源颜色
exLightSetAccent1Color(float x, float y, float z)  --补光1颜色
exLightSetAccent2Color(float x, float y, float z)  --补光2颜色
exLightSetMainLightDirection(float x, float y, float z)  --主光源方向  这里输入的是光源方向向量
exLightSetAccent1Direction(float x, float y, float z)   --补光1方向
exLightSetAccent2Direction(float x, float y, float z)   --补光2方向




--物体染色或闪烁着色
exObjectTintColor(int objectId, int R, int G, int B, float preColorTime, float PostColorTime, float SustainedColorTime, float Frequency, float Amplitude)
 -- objectId 物体id
 -- R G B 颜色的三个分量， 范围0到255
 -- preColorTime 颜色渐变到目标颜色的时间
 -- PostColorTime 颜色渐变回原始颜色的时间
 -- SustainedColorTime 持续时间
 -- Frequency 频率
 -- Amplitude 振幅  后面这两个参数我也不是太懂
 -- 例子
 -- 将物体染色为固定一个颜色 exObjectTintColor(83, 2,0,0, 0, 0, 10, 1, 0);
 -- 将物体染色为闪烁的颜色 exObjectTintColor(83, 2,0,0, 0, 0, 10, 1, 1);



--修改相机参数
-- 针对全体玩家
exCameraAdjustPos(float x, float y, float z); --设置相机位置
exCameraAdjustRotation(float value); --设置相机旋转
exCameraAdjustPitch(float value); --设置相机俯仰
exCameraAdjustRoll(float value); --设置相机滚转

exCameraAdjustPosBy(float dx, float dy, float dz); --设置相机位置相对于当前位置的偏移
exCameraAdjustRotationBy(float value); --设置相机旋转改变量（相对于当前值)
exCameraAdjustPitchBy(float value); --设置相机俯仰改变量（相对于当前值)
exCameraAdjustRollBy(float value); --设置相机滚转改变量（相对于当前值)

-- 针对某个玩家
exCameraAdjustPosForPlayer(playerName, float x, float y, float z); --设置某玩家相机位置
exCameraAdjustRotationForPlayer(playerName, float value); --设置某玩家相机旋转
exCameraAdjustPitchForPlayer(playerName, float value); --设置某玩家相机俯仰
exCameraAdjustRollForPlayer(playerName, float value); --设置某玩家相机滚转

exCameraAdjustPosByForPlayer(playerName, float dx, float dy, float dz); --设置某玩家相机位置相对于当前位置的偏移
exCameraAdjustRotationByForPlayer(playerName, float value); --设置某玩家相机旋转改变量（相对于当前值)
exCameraAdjustPitchByForPlayer(playerName, float value); --设置某玩家相机俯仰改变量（相对于当前值)
exCameraAdjustRollByForPlayer(playerName, float value); --设置某玩家相机滚转改变量（相对于当前值)

-- 相机动画，跟随相机相关
-- 开启相机插值
exEnableCameraInterpolate();
-- 关闭相机插值
exDisableCameraInterpolate();
-- 设置相机各个参数
exSetFollowCameraParam(playerName, float posX, float posY, float posZ, float yaw, float pitch, float roll);
  -- 使用方法，要进行连续的相机动画，或者跟随某单位，首先exEnableCameraInterpolate，然后每一帧调用一次SetFollowCameraParam（必须有一次且只能调用一次）
  -- 然后使用 exDisableCameraInterpolate 结束
  -- 注意，如果地图不需要相机动画或者跟随物体，不要调用exEnableCameraInterpolate
  -- 这里的插值，是自动在渲染帧预测和设置相机参数，来实现平稳的相机连续动作（因为lua设置参数只能在逻辑帧）
```

## UI和按钮
### 未注释API
```lua
-- 为某个玩家显示正上方的提示框，只有那个玩家能看到    是脚本 SHOW_MILITARY_CAPTION 的增强版本
exInfoBoxTopCenterShowForPlayer("玩家名字", "提示内容", time)
-- time 时间
-- 比如 exInfoBoxTopCenterShowForPlayer("Player_1", "Script:Des1", 10)

-- 隐藏右下角的技能栏区域
exAptHideObjectDetailsArea()
-- 显示右下角的技能栏区域
exAptShowObjectDetailsArea()

-- 隐藏左下角的协议按钮
exAptHidePlayerTechButton();

-- 显示左下角的协议按钮
exAptShowPlayerTechButton();

-- 协议转换相关
-- 为某个玩家解锁所有阵营的基础科技（如果要切换某个玩家的协议面板，需要先调用这个）
exPlayerTechUnlockBaseTechForPlayer(playerName)
-- 切换某个玩家的协议面板为指定阵营的协议面板
-- factionTech 只能是以下字符串 (注意自己加一下双引号)
  -- PlayerTechStore_Allied 盟军协议
  -- PlayerTechStore_Soviet 苏联协议
  -- PlayerTechStore_Japan 帝国协议
  -- PlayerTechStore_Neutral 神州协议
exPlayerTechChangeTechFaction(playerName, factionTech)




-- 公屏文字相关， 公屏最多16行
-- 添加一条公屏文本，如果有换行符，会被拆陈多行
exAddTextToPublicBoard(string content, float time)
  -- content 文本内容
  -- time 显示持续时间， 最多30秒
-- 玩家特化版本
exAddTextToPublicBoardForPlayer(playerName, string content, float time)



-- 单位顶部按钮相关
exObjectShowBtnAtTop(objectId, offset);   -- 添加按钮
exObjectChangeBtnAtTopVisibility(objectId, visible);  -- visible  1是可见，0是隐藏
exObjectDeleteBtnAtTop(objectId);  -- 删除按钮
-- 玩家特化版本
exObjectShowBtnAtTopForPlayer(playerName, objectId, offset);
exObjectChangeBtnAtTopVisibilityForPlayer(playerName, objectId, visible);
exObjectDeleteBtnAtTopForPlayer(playerName, objectId);

-- 单位头顶按钮点击事件
-- 定义一个固定名字的函数
function onUnitTopBtnClickEvent(playerName, unitId)
-- unitId 单位id
end


-- 界面顶部按钮相关
-- ui  目前顶部按钮最多6个， index 取值 1~6
-- textureName 图片名字，对应源码里 PackedTextureImage 类型的id, 例如
-- "Button_AlliedAntiGroundAircraft_on"
-- "Button_AlliedAntiInfantryInfantry_on"
-- "Button_AlliedAntiNavyShipTech1_on"
-- "AUA_Tank_TargetPainter"
-- "Button_PlayerPower_TimeBomb3"
-- "CelestialEMSuppressField_Lv1"

-- 设置整个顶部栏按钮区域的可见性, visible  1可见，0不可见
exCenterTopBtnsChangeVisibility(visible);
-- 修改指定位置的图片，既是创建也是更新，注意，目前这个函数创建后需要等待1或2帧后才能调用其他的ui函数，不然没有作用  desc是描述文本（鼠标放到按钮上时出现）
exCenterTopBtnShow(index,  textureName, desc);
-- 是否启用高亮动画 ,enable 1启用 0关闭
exCenterTopBtnToggleHighLight(index, int enable);
-- 设置时间overlay层可见性, visible 1可见，0隐藏
exCenterTopBtnSetTimerOverlayVisibility(index, visible);
-- 设置时间overlay层的百分比，value范围 0-99  注意先要让时间overlay层可见
exCenterTopBtnSetTimerOverlayValue(index, value);
-- 设置血条可见性, visible 1可见，0隐藏
exCenterTopBtnSetHealthBarValue(index, visiblev);
-- 设置血条的百分比，value范围 1-100  注意先要让血条可见
exCenterTopBtnSetHealthBarVisibility(index, value);
-- 设置某个位置按钮的可见性
exCenterTopBtnSetVisibility(index);
-- 设置是否选中态，selected 1选中， 0未选中
exCenterTopBtnSetSelected(index, selected);
-- 设置是否选启用，enable 1可用， 0不可用
exCenterTopBtnSetEnable(index, enable);

-- 玩家特化版本
exCenterTopBtnShowForPlayer(playerName, index, textureName, desc);
exCenterTopBtnToggleHighLightForPlayer(playerName, index, int enable);
exCenterTopBtnSetTimerOverlayVisibilityForPlayer(playerName, index, visible);
exCenterTopBtnSetTimerOverlayValueForPlayer(playerName, index, value);
exCenterTopBtnSetHealthBarValueForPlayer(playerName, index, visiblev);
exCenterTopBtnSetHealthBarVisibilityForPlayer(playerName, index, value);
exCenterTopBtnSetVisibilityForPlayer(playerName, index);
exCenterTopBtnSetSelectedForPlayer(playerName, index, selected);
exCenterTopBtnsChangeVisibilityForPlayer(playerName, visible);
exCenterTopBtnSetEnableForPlayer(playerName, index, enable);

-- 顶部按钮点击事件函数, 这个函数将在顶部按钮被点击时被游戏引擎调用 （放在非循环脚本里，定义一次就行了）
-- 定义一个固定名字的函数 (必须是这个名字，参数也要一样)
function onCenterTopBtnClickEvent(playerName, btnIndex)
-- btnIndex 按钮编号，第一个按钮是1，第二个按钮是2，以此类推
  --- TODO: 在这里定义你的函数
end



```

### 已注释API
下面这些API代码已按照LuaCATS格式注释，可供VS Code等代码编辑器使用，用于协助插件（如Emma Lua和Lua Language Server）[自动检测语法和类型错误](https://www.yuque.com/muzeqaq/ra3mapwiki/rge16id4wsoloub8)。

例子:

```lua
local dialogId = 1
local content "我是对话框"
local btn1_text = "按钮1"
local btn2_text = "按钮2"
local btn3_text = ""
exShowLongTextDialog(dialogId, content, btn1_text, btn2_text, btn3_text)
```

当用户使用了错误的参数类型，例如content参数使用数字，依靠LuaCATS格式注释，插件将在出错位置显示波浪线警告。

![](https://cdn.nlark.com/yuque/0/2024/png/50038568/1734606397665-7f9d419b-db2f-44c0-87e3-6c794addde03.png)



```lua
--- @alias Player
--- |"<1st Human Player's Allies incl Self>"
--- |"<All Players>"
--- | "PlyrCreeps"
--- | "PlyrCivilian"
--- | "Player_1"
--- | "Player_2"
--- | "Player_3"
--- | "Player_4"
--- | "Player_5"
--- | "Player_6"


------------------------ 长文本对话框-----------------------------------------------------------
--- 长文本对话框只有3个按钮，哪个按钮传空字符串，那么那个按钮就会隐藏

--- 显示长文本对话框, 按钮字符串为空时不显示对应按钮
---@param dialogId integer
---@param content string 长文本，支持多行
---@param btn1_text string
---@param btn2_text string
---@param btn3_text string
function exShowLongTextDialog(dialogId, content, btn1_text, btn2_text, btn3_text) end

--- 为特定玩家显示长文本对话框, 按钮字符串为空时不显示对应按钮
---@param playerName Player
function exShowLongTextDialogForPlayer(playerName, dialogId, content, btn1_text, btn2_text, btn3_text) end

--- 隐藏长文本对话框
function exHideLongTextDialog() end

--- 隐藏长文本对话框, 特定玩家
--- @param playerName Player
function exHideLongTextDialogForPlayer(playerName) end

-- 监听长文本弹框的点击事件
-- 定义一个固定名字的函数
function onUserLongTextDialogEvent(playerName, btnIndex, dialogId)
-- btnIndex 按钮编号，第一个按钮是1，第二个按钮是2，以此类推 从左到右
-- dialogId  弹框id
end


------------------------ 按钮选项弹框-----------------------------------------------------------
-- 显示按钮选项弹框(最多7个按钮竖着排）
-- 弹框全局只有一个，如果之前的弹框还未消失，则会覆盖之前的弹框数据和ui
-- 弹框按钮点击后，弹框自动消失
-- 如果不需要那么多按钮，就传空字符串，比如第3个传空字符串，那么后面的按钮都会被隐藏
--- @param dialogId integer 弹框id， 用户点击按钮后，回调函数里会传过去
--- @param title string 弹框文本
--- @param btn1_text string 按钮1文本, 7个按钮均为string类型， 注释略去
function exShowCustomBtnChoiceDialog(dialogId, title, 
    btn1_text, btn2_text, btn3_text, btn4_text, btn5_text, btn6_text, btn7_text) end

--- 为特定玩家显示按钮选项弹框(最多7个按钮竖着排）
--- @param playerName Player 
function exShowCustomBtnChoiceDialogForPlayer(playerName, dialogId, title, 
    btn1_text, btn2_text, btn3_text, btn4_text, btn5_text, btn6_text, btn7_text) end

--- 关闭按钮选项弹框
function exHideCustomBtnChoiceDialog() end

--- 为特定玩家关闭按钮选项弹框
--- @param playerName Player
function exHideCustomBtnChoiceDialogForPlayer(playerName) end

--- 按钮选项弹框的点击事件函数, 该函数将在按钮被点击时被调用
--- btnIndex 按钮编号，第一个按钮是1，第二个按钮是2，以此类推
--- dialogId  弹框id
function onUserBtnChoiceDialogEvent(playerName, btnIndex, dialogId)
  --- 在这里定义你的函数
  --- 注意，这个函数需要被定义
  --- TODO: 在这里定义你的函数
end
```

Enhancer相关已知bug见[LuaAPI中的bug](https://www.yuque.com/muzeqaq/ra3mapwiki/ih1z2puyfrwb0gxq)

界面顶部按钮，原版图片参数可参考[538号脚本图片参数](https://www.yuque.com/muzeqaq/ra3mapwiki/hd2b4xwe6mgn89ux)

日冕新增图片参数清单见 [日冕新增PackedTextureImage 清单](https://www.yuque.com/muzeqaq/ra3mapwiki/xm64gw2nfzy2i35y#6edd)



## 3224更新 
左下角消息区，公屏区 新增 支持显示玩家昵称支持昵称，$p1Name，$p2Name，$p3Name，$p4Name，$p5Name，$p6Name将被自动替换为对应的玩家昵称（对应Player_1~Player_6的昵称）

```lua
-- 修改Waypoint点的坐标位置  坐标都需要是整数
exWaypointSetPos(name, x, y, z)

-- 给GenericRankTech（玩家协议面板不能一开始就点完所有科技就是因为有这个东西，开局给level=1的,每隔一段时间给下一个level，最多到10，
-- level取值2~10, 如果给2，并且协议点数给2，一开始就能点到第二行协议，如果给10，那么就能点完所有协议
exUnlockPlayerGenericRankTech(playerName, level)

-- 计分板相关  计分版有7行3列  标题和文字都支持昵称，$p1Name，$p2Name，$p3Name，$p4Name，$p5Name，$p6Name将被自动替换为对应的玩家昵称（对应Player_1~Player_6的昵称）
-- 设置计分板可进行, visible  1可见，0不可见
exScoreBoardSetVisibility(visible)
-- 设置标题文字
exScoreBoardSetTitle(title)
-- 设置某行某列的文字
-- row 行  范围1~7
-- col 列  范围1~3
exScoreBoardSetText(text, row, col)

-- 玩家特化版本
exScoreBoardSetVisibilityForPlayer(playerName, visible)
exScoreBoardSetTitleForPlayer(playerName, title)
exScoreBoardSetTextForPlayer(playerName, text, row, col)

-- 设置公屏文字的颜色(是整个公屏文字的颜色)
exSetPublicBoardColor(color)

-- 获取玩家颜色 
-- local color, count = exPlayerGetColor("Player_1")
-- color[1]   color[2]   color[3]  分别对应 R  G  B
-- 如果玩家非法，R,G,B均是负数
exPlayerGetColor(playerName);

-- 获取ai经济作弊倍数   注意：负数表示不指定
exModeGetAICheatMultiplier();

-- 启用地编脚本
exEnableWBScript(scriptName);
-- 禁用地编脚本
exDisableWBScript(scriptName);


-- 下面的功能因为bug，目前不可用
--注册某种单位死亡事件   name是单位种类名字
exObjectRegisterDieEvent(name)
--然后定义一个固定名字的函数
function onUnitDieEvent(dyingObjId, attackerId, dyingObjInstanceId, attackerInstanceId)
    -- dyingObjId 即将死亡的物体id （注意，避免对死亡物体做任何操作，包括设置位置，设置状态等，可能会引发难以预料的后果，获取死亡物体的位置是可以的）
    -- attackedId 攻击者id 注意不一定有攻击者，这时这个参数是nil
    -- dyingObjInstanceId 死亡物体的InstanceId  (每种单位都有唯一的InstanceId)
    -- attackerInstanceId 攻击者的InstanceId  注意不一定有攻击者，这时这个参数是nil
end

```



## 3227更新 
exCounter计数器函数闪退问题修复

单位死亡事件修复，现在可以使用了

```lua
--注册某种单位死亡事件   name是单位种类名字
exObjectRegisterDieEvent(name)
--然后定义一个固定名字的函数
function onUnitDieEvent(dyingObjId, attackerId, dyingObjInstanceId, attackerInstanceId, ownerPlayerName)
    -- dyingObjId 即将死亡的物体id （注意，避免对死亡物体做任何操作，包括设置位置，设置状态等，可能会引发难以预料的后果，获取死亡物体的位置是可以的）
    -- attackedId 攻击者id 注意不一定有攻击者，这时这个参数是nil
    -- dyingObjInstanceId 死亡物体的InstanceId  (每种单位都有唯一的InstanceId)
    -- attackerInstanceId 攻击者的InstanceId  注意不一定有攻击者，这时这个参数是nil
    -- ownerPlayerName  死亡单位的玩家名字
end


--注册某种单位创建事件   name是单位种类名字
exObjectRegisterCreateEvent(name)
--然后定义一个固定名字的函数
-- 注意，不要在这里面摧毁创建的单位，很可能会造成未知崩溃等问题
function onUnitCreateEvent(createdObjId, createdObjInstanceId, ownerPlayerName)
-- createdObjId 创建的物体id
-- createdObjInstanceId 创建的物体的InstanceId  (每种单位都有唯一的InstanceId)
-- ownerPlayerName 创建的物体的玩家脚本名字  例如"Player_1"
end

-- 鼠标位置相关
-- 建议只在必要的时间段开启监听，然后及时取消监听，避免频繁发送事件
-- 监听某个玩家的鼠标位置事件
exEnableMousePosEvent(playerName);
-- 取消监听某个玩家的鼠标位置事件
exDisableMousePosEvent(playerName);
-- 然后定义一个固定名字的函数
function onUserMousePosEvent(playerName, screenPos, worldPos)
  -- 如果鼠标不在屏幕内  screenPos和worldPos都是nil
  -- screenPos 鼠标屏幕坐标, 获取位置使用mouseWorldPos.X, mouseWorldPos.Y,  
  -- mouseWorldPos  鼠标在游戏里的位置，获取位置使用mouseWorldPos.X, mouseWorldPos.Y, mouseWorldPos.Z， 如果鼠标不在游戏窗口内，这个值是nil
end

-- 下面3个函数建议不要在固定名字的回调函数里使用，可能会导致未知问题
-- 设置物体清雾距离  distance需要是整数
exObjectSetShroudClearingDistance(objectId, distance);
-- 设置物体视线距离  distance需要是整数   和警戒距离有关
exObjectSetVisionRange(objectId, distance);
-- 设置物体模型体积相关  radius需要是整数 (据观察, radius越小，单位挨得越近，大于20并且小于40的值会被统一视作40)
exObjectSetGeometryMajorRadius(objectId, radius);


--获取当前物体正在控制的单位的id , 如果返回的是0，则表示没有控制的物体 (此函数针对有控制技能的物体， 例如 JapanPsychicInhibitor )
exObjectGetTemporaryOwnedObjectId(objectId);

```

## 3230更新 
热键注册有所修改，目前辅助键不可以注册鼠标左右键和鼠标中键盘，但主键可以直接注册鼠标左右键了

单位头顶文本和单位头顶按钮的绘制优化，更加流畅

```lua
-- 获取玩家之间的关系
-- 返回值枚举: 0(Enemy) 1(Neutral) 2(Friend)
exPlayerGetRelationShip(playerName1, playerName2)；

-- 将一个可附着物体 附着到 其他物体 
-- 第一个参数是可以附着到其他物体的物体（也即必须有attach模块物体）；第二个参数无要求，可以是任何物体
exObjectAttachToObject(objectId, beAttachedObjectId)
-- 关于附着物体相关的api，参考LuaBridge那里的api   这里一并贴过来
-- [[21]]
-- 返回贴在当前单位身上贴着的单位的列表。
-- 例如，假如 self 是矿车，那么可能返回包含蜻蜓以及信标、钱套子之类的列表
-- local table, count = ObjectGetAttachers(self)
-- for i = 1, count, 1 do
--   ObjectDescription(table[i])
-- end
-- -- 返回贴在当前单位正在贴的单位。
-- -- 例如，假如 self 是蜻蜓，那么可能返回包含矿车的列表。
-- -- 一般来说这个列表只会有 0 个或者 1 个单位，除非 self 有多个 AttachUpdate 模块
-- local table, count = ObjectGetAttachees(self)
-- for i = 1, count, 1 do
--   ObjectDescription(table[i])
-- end
-- 日冕3230新增的可附加的物体有
-- Map_CustomAttaches01
-- Map_CustomAttaches02
-- Map_CustomAttaches_DogStun_Small
-- Map_CustomAttaches_DogStun_Big
-- Map_CustomAttaches04
-- Map_CustomAttaches_SelfHeal_Per2
-- Map_CustomAttaches_SelfHeal_Per5
-- Map_CustomAttaches_RangeArmorUp_400
-- Map_CustomAttaches_RangeDamageUp_400
-- Map_CustomAttaches_RangeFireSpeedUp_400
-- Map_CustomAttaches_RangeHeal_Per3
-- Map_CustomAttaches_RangeHeal_Per6
-- Map_CustomAttaches_TextureCelestial_Small
-- Map_CustomAttaches_TextureCelestial_Big
-- Map_CustomAttaches_TextureNatasha_Small
-- Map_CustomAttaches_TextureNatasha_Big
-- Map_CustomAttaches_TextureTanYa_Small
-- Map_CustomAttaches_TextureTanYa_Big


-- 设置物体缩放（永久缩放，和地编那个一样）
exObjectSetFixedScale(objectid, scale)

-- 获取指定路径点的位置
exWaypointGetPos(name)

-- 集结点相关  集结点指的是生产建筑的集结点，比如兵营重工这些
-- 获取物体的集结点（如果物体不存在，或者此物体还没设置过集结点，那么坐标都是0）
exObjectGetRallyPointPos(objectid)
-- local pos, count = exObjectGetRallyPointPos(114);
-- pos[1]  pos[2]  pos[3]  就是坐标

-- 设置物体的集结点  
-- 这个函数建议不要在固定名字的回调函数里使用
exObjectSetRallyPointPos(objectid, x, y, z)

-- 设置单位速度（注意，这个速度在单位下海，上陆地，变形，回到机场 之后会失效）
exObjectSetCurrentLocomotorSpeed(objectId, speedValue)

-- 取消特定commandCode的热键注册
exUnregisterHotKey(commandCode);

-- 获取单位当前速度
exObjectGetCurrentLocomotorSpeed(objectId);
```



## 3232更新
```lua
-- 在指定地点显示任意内容的漂浮文字
exShowTextAtPos(text, x, y, z, color)
  -- text 文字内容

-- 玩家特化版本
exShowTextAtPosForPlayer(playerName, text, x, y, z, color)

-- 获取玩家指定编号的队伍  index范围0~9
local memers, count = exPlayerGetFormationTeam(playerName, index)

-- 获取指定名字的队伍的成员
local memers, count = exTeamGetMembers(teamName)

-- 获取指定物体指定技能的剩余冷却时间
exObjectGetSpecialCountDownFrame(objectId, specialPowerName)

-- 开启或关闭监听玩家获取协议事件
exTogglePlayerGetTechEvent(enable)

-- 然后定义一个固定名字的函数
function onPlayerGetTechEvent(playerName, techInstanceId)
  -- 具体逻辑
end

-- 监听玩家某个升级完成的事件
exRegisterUpgradeCompleteEvent(upgradeName)

-- 然后定义一个固定名字的函数
function onPlayerUpgradeCompleteEvent(playerName, upgradeInstanceId, objectId)
  -- objectId是完成这个升级的物体的id，比如某个盟军基地完成了中级许可
  -- 具体逻辑
end

```



## 3234更新


ObjectTestModelCondition

ObjectSetObjectStatus

ObjectCreateAndFireTempWeapon

以上三个函数支持传物体id

```lua
-- 创建物体
-- 参数必须要有 ObjectTypeName, TeamName
-- 返回值是物体id  注意！！如果id是0，表示创建失败
local objectId = exCreateObject({
    ObjectType = FastHash("JapanInterceptorAircraft"),
    TeamName = "Player_1/teamPlayer_1",
})
-- 更多可选参数
local objectId = exCreateObject({
    ObjectType = FastHash("JapanInterceptorAircraft"),
    TeamName = "Player_1/teamPlayer_1",
    Position = {X = 300, Y = 2100, Z = 210},
    Angle = 90,
    Health = 30000,
    Scale = 3.0,
})


-- 创建多个物体，返回物体id数组和个数
local objects, count = exCreateObjects({
    {
        ObjectType = FastHash("JapanAntiInfantryInfantry"),
        TeamName = "Player_1/teamPlayer_1",
        Position = {X = 600, Y = 2900, Z = 210},
        Angle = 90,
        Health = 30000,
        Scale = 2.0,
    },
    {
        ObjectType = FastHash("JapanAntiInfantryInfantry"),
        TeamName = "Player_1/teamPlayer_1",
        Position = {X = 900, Y = 2900, Z = 210},
        Angle = 120,
        Health = 30000,
        Scale = 3.0,
    },
})
for i = 1, count, 1 do
  local current = objects[i]
  -- 对每个物体进行操作， 注意！！如果current是0，表示创建失败
end


-- 设置物体当前生命值（整数）
exObjectSetHealth(objectId, health)


-- 自定义按钮（建造栏的那种按钮）每个按钮大小是50
-- 创建
-- GroupIndex是可选参数(默认0)，AlignX和AlignY也是可选参数（但建议都传，不传默认为绝对坐标，有可能会超出屏幕）其他事必须传
-- AlignX 是按钮对齐画面左右的方式  可以传 "left" 或者  "right" ，分别是靠左或靠右  （这里靠左靠右均已考虑屏幕左右黑边）
-- AlignY 是按钮对齐画面上下的方式  可以传 "top" 或者  "bottom"  ,分别是靠上或者靠下
-- X Y 
    -- 不传AlignX和AlignY则为按钮相对于屏幕左上角的坐标
    -- 传了AlignX "left" 则x表示距离画面左边的距离，  "right"  则表示距离画面右边的距离
    -- 传了AlignY "top" 则x表示距离画面上边的距离，  "bottom"  则表示距离画面下边的距离
-- GroupIndex可以将多个按钮联系起来，方便统一设置他们的相关属性
-- 注意，如果后续创建了同一个index的按钮，则会销毁之前的按钮，然后新建一个
exCreateCustomButton({
    Index = 3,
    TextureName = "Button_AlliedAntiInfantryInfantry_on",
    Desc = "局部毁灭武器(全局仅限一次)\n摧毁一大片地区的所有敌方单位",
    X = 400,
    Y = 200,
    GroupIndex = 1,
    AlignX = "right",
    AlignY = "top",
})
exCreateCustomButtons({
    {
        Index = 4,
        TextureName = "Button_AlliedAntiInfantryInfantry_on",
        Desc = "局部毁灭武器(全局仅限一次)\n摧毁一大片地区的所有敌方单位",
        X = 400,
        Y = 200,
        GroupIndex = 1,
        AlignX = "right",
        AlignY = "top",
    },
    {
        Index = 5,
        TextureName = "Button_AlliedAntiInfantryInfantry_on",
        Desc = "局部毁灭武器(全局仅限一次)\n摧毁一大片地区的所有敌方单位",
        X = 450,
        Y = 200,
        GroupIndex = 1,
    },
    {
        Index = 6,
        TextureName = "Button_AlliedAntiInfantryInfantry_on",
        Desc = "局部毁灭武器(全局仅限一次)\n摧毁一大片地区的所有敌方单位",
        X = 500,
        Y = 200,
        GroupIndex = 1,
    }
})

-- 用法与顶部按钮类似
exCustomBtnToggleHighLight(index, enable)
exCustomBtnSetTimerOverlayValue
exCustomBtnSetTimerOverlayVisibility
exCustomBtnSetHealthBarValue
exCustomBtnSetHealthBarVisibility
exCustomBtnSetVisibility
exCustomBtnSetSelected
exCustomBtnSetEnable

-- 按钮组设置，会将同一个groupIndex的按钮统一设置（相当于上面那些函数的批量操作版本）
-- 参数只是将按钮index换成了groupIndex，其他都一样
exCustomBtnGroupToggleHighLight(groupIndex, enable)
exCustomBtnGroupSetTimerOverlayValue
exCustomBtnGroupSetTimerOverlayVisibility
exCustomBtnGroupSetHealthBarValue
exCustomBtnGroupSetHealthBarVisibility
exCustomBtnGroupSetVisibility
exCustomBtnGroupSetSelected
exCustomBtnGroupSetEnable

--组定按钮玩家特化版本（命名类似，都是最后加个ForPlayer）
exCreateCustomButtonForPlayer("Player_1", {
    Index = 4,
    TextureName = "Button_AlliedAntiInfantryInfantry_on",
    Desc = "局部毁灭武器(全局仅限一次)\n摧毁一大片地区的所有敌方单位",
    X = 400,
    Y = 200,
    GroupIndex = 1,
})
exCreateCustomButtonsForPlayer("Player_1", {
    {
        Index = 4,
        TextureName = "Button_AlliedAntiInfantryInfantry_on",
        Desc = "局部毁灭武器(全局仅限一次)\n摧毁一大片地区的所有敌方单位",
        X = 400,
        Y = 200,
        GroupIndex = 1,
    },
    {
        Index = 5,
        TextureName = "Button_AlliedAntiInfantryInfantry_on",
        Desc = "局部毁灭武器(全局仅限一次)\n摧毁一大片地区的所有敌方单位",
        X = 450,
        Y = 200,
        GroupIndex = 1,
    },
    {
        Index = 6,
        TextureName = "Button_AlliedAntiInfantryInfantry_on",
        Desc = "局部毁灭武器(全局仅限一次)\n摧毁一大片地区的所有敌方单位",
        X = 500,
        Y = 200,
        GroupIndex = 1,
    }
})

--多了个玩家名字参数在最前面
exCustomBtnToggleHighLightForPlayer(playerName, index, enable)
exCustomBtnSetTimerOverlayValueForPlayer
exCustomBtnSetTimerOverlayVisibilityForPlayer
exCustomBtnSetHealthBarValueForPlayer
exCustomBtnSetHealthBarVisibilityForPlayer
exCustomBtnSetVisibilityForPlayer
exCustomBtnSetSelectedForPlayer
exCustomBtnSetEnableForPlayer

exCustomBtnGroupToggleHighLightForPlayer(playerName, groupIndex, enable)
exCustomBtnGroupSetTimerOverlayValueForPlayer
exCustomBtnGroupSetTimerOverlayVisibilityForPlayer
exCustomBtnGroupSetHealthBarValueForPlayer
exCustomBtnGroupSetHealthBarVisibilityForPlayer
exCustomBtnGroupSetVisibilityForPlayer
exCustomBtnGroupSetSelectedForPlayer
exCustomBtnGroupSetEnableForPlayer

--自定义按钮点击事件
--定义固定名字的函数
function onCustomBtnClickEvent(playerName, index)
  
end
```

## 3240更新
**自定义按钮新增特性**：如果配置的描述为空，那么鼠标放上去不会出现那个描述框

```lua
-- 自定义文本相关   目前自定义文本（3241版本）存在严重问题，请不要使用

-- 创建自定义文本
-- size是文本大小
-- 自动换行长度取决于第一行的长度
-- 假如文本本身没有任何换行，那么 exCreateCustomText 也绝不会自行自动换行
-- 假如文本本身有换行，然后假如第二行比第一行长,那么第二行会自动换行、它不会超过第一行的长度
exCreateCustomText({
    Index = 3,
    Content = "这是测试文字",
    X = 200,
    Y = 200,
    Color = 16711680,
    Size = 16,
    AlignX = "left",
    AlignY = "top",
})

-- 更新文本 content是新的文本内容
exCustomTextUpdateText(index, content)
-- 更新文本位置
exCustomTextUpdatePos(index, x, y, AlignX, AlignY)
-- 更新文本可见性  
exCustomTextUpdateVisibility(index, visibility)

-- 玩家特化版本
exCreateCustomTextForPlayer("Player_1", {
    Index = 3,
    Content = "这是测试文字",
    X = 200,
    Y = 200,
    Color = 16711680,
    Size = 16,
    AlignX = "left",
    AlignY = "top",
})
exCustomTextUpdateTextForPlayer(playerName, index, content)
exCustomTextUpdatePosForPlayer(playerName, index, x, y, AlignX, AlignY)
exCustomTextUpdateVisibilityForPlayer(playerName, index, visibility)


-- 是否启用了散矿模式，0没启用，1启用
exIsLooseOreEnable()

--右上角ui只保留小地图
exHideTopRightUIComponents()

--右上角ui只保留小地图  玩家特化版本
exHideTopRightUIComponentsForPlayer(playerName)
```

## 3243更新
```lua
-- 自定义按钮 删除指定自定义按钮
exCustomBtnRemove(index)

-- 自定义按钮点击事件函数 新增鼠标左键是否按下的参数isLeftMouse  1表示鼠标左键按下了，0表示鼠标左键没有按下
function onCustomBtnClickEvent(playerName, index, leftMouseDown)
  
end


-- 散矿矿石相关   每个矿石的价值 = 每个矿石含有的box * 每个box的价值   目前日冕每个box的价值是20
-- 获取矿石剩余box
exTiberiumCrystalGetLeftBoxes(objectId)
-- 获取矿石最大box (源码里设定的)
exTiberiumCrystalGetMaxBoxes(objectId)
-- 设置矿石剩余box   boxNum指定box数量
exTiberiumCrystalSetLeftBoxes(objectId, boxNum)
```



## 3245
日新增6个中立ai，名字是

<font style="color:#333333;">AIPlayer01</font>

<font style="color:#333333;">AIPlayer02</font>

<font style="color:#333333;">AIPlayer03</font>

<font style="color:#333333;">AIPlayer04</font>

<font style="color:#333333;">AIPlayer05</font>

<font style="color:#333333;">AIPlayer06</font>

```lua
-- 取消注册单位死亡事件
exObjectUnRegisterDieEvent

-- 取消注册单位创造事件
exObjectUnRegisterCreateEvent

--玩家特化版的 移除某个自定义按钮
exCustomBtnRemoveForPlayer

-- 获取单位的清雾距离
exObjectGetShroudClearingDistance(objectId)

-- 自定义buff
local stackingLimit = 3 -- 最大层数
local attrId = exAttributeModifierCreate({
    HEALTH_MULT = 100,
    DAMAGE_MULT = 25.5,
}, stackingLimit)
-- 创建好后直接使用即可
ObjectLoadAttributeModifier(unit, attrId, 999)

```

自定义贴图

```lua
-- 先从工具中获取图片的base64, 支持jpg/png/dds/webp格式
local base = "xxxxxxxxxxx"
-- 压缩算法
-- DXT1 默认.最省资源
-- DXT5 适中, 支持半透明
-- A8R8G8B8 质量最好, 内存消耗高
local compression = 'DXT1'
local generateMips = 0
local textureId = exTextureCreateFromBase64(base64, compression, generateMips)
-- 可以忽略后两个参数
local textureId = exTextureCreateFromBase64(base64)
-- 如果图片文件格式是dds, 则后两个参数无效


-- 定义自定义贴图
local imageId = exPackedTextureImageCreate({
    Texture = textureId,
    -- 贴图在纹理中的位置
    CoordsX = 1, CoordsY = 1,
    -- 贴图的尺寸
    DimensionsX = 64, DimensionsY = 64,
    -- 源纹理的尺寸
    TextureDimensionsX = 512, TextureDimensionsY = 512,
    -- 旋转角度
    Rotated = 0
})

-- 在按钮中使用这个贴图
exCreateCustomButton({
                Index = 1,
                TextureName = imageId, -- 替换TextureName
                X = 10,
                Y = 10,
                GroupIndex = 1,
                AlignX = "left",
                AlignY = "top",
})
```

## 3254
新增对图片格式webp的支持

### 自定义按钮
```lua
-- 自定义按钮也支持自定义大小
-- 注意：假如使用了自定义大小，则该按钮被视为一个可点击的 customImage
-- 在那种情况下，大部分现有的按钮操作（例如设置生命值、设置冷却进度条等）都不再适用
-- 依然可以使用 exCustomBtnSetVisibility、exCustomBtnSetEnable
-- 注意：自定义图片中的透明部分依然可以被鼠标点击
exCreateCustomButton({
    Index = 300,
    TextureName = "Button_AlliedAntiInfantryInfantry_on",
    X = 0,
    Y = 0,
    AlignX = "right",
    AlignY = "bottom",
    SizeX = 100, -- 自定义宽度
    SizeY = 60, -- 自定义高度
    Desc = "大按钮",
})
```

### 自定义图像
```lua
-- 创建带自定义尺寸的自定义图像
-- 自定义图像和自定义按钮实际上是同一个控件，只是按钮有点击事件，图像没有点击事件
-- 因此自定义图像也是用 exCustomBtnRemove 来移除
-- 也可以使用 exCustomBtnSetVisibility 以及对应的分组操作
-- 不支持其他按钮操作
local ret, err = exCreateCustomImage({
    Index = 100,
    TextureName = "Button_AlliedAntiInfantryInfantry_on",
    X = 200,
    Y = 150,
    AlignX = "left",
    AlignY = "top",
    GroupIndex = 1,
    SizeX = 80,   -- 自定义宽度
    SizeY = 80,   -- 自定义高度
})
if err then
    _ALERT("创建失败: " .. err)
end

-- 批量创建图像
local images = {
    { Index = 101, TextureName = "Button_AlliedAntiInfantryInfantry_on", X = 10, Y = 10, GroupIndex = 2, SizeX = 32, SizeY = 32 },
    { Index = 102, TextureName = "Button_AlliedAntiInfantryInfantry_off", X = 10, Y = 50, GroupIndex = 2, SizeX = 32, SizeY = 32 },
}
exCreateCustomImages(images)

-- 为指定玩家创建图像
exCreateCustomImageForPlayer("Player_1", {
    Index = 200,
    TextureName = "Button_AlliedAntiInfantryInfantry_on",
    X = 500,
    Y = 300,
    SizeX = 48,
    SizeY = 48,
})
exCreateCustomImagesForPlayer("Player_1", {
    -- 略
})

```

### 图片设置
```lua
-- 设置图像或按钮的颜色（ARGB: 0xAARRGGBB）
-- 注意：假如修改按钮的颜色，即使把按钮改成透明，按钮也依然可被点击
-- 假如想要让按钮不可见而且不可被点击，应该使用 exCustomBtnSetVisibility 设置按钮不可见
-- 半透明红色
exCustomImageSetColor(100, HexToUInt32("0x80FF0000"))
-- 不透明蓝色
exCustomImageGroupSetColor(2, HexToUInt32("0xFF0000FF"))

-- 为指定玩家的图像或按钮设置颜色
exCustomImageSetColorForPlayer("Player_1", 200, HexToUInt32("0xFFFFCC00"))  -- 不透明橙色

-- 深度排序
-- 将分组2内所有按钮/图像，按索引（Index）顺序排列深度
-- 例如：Index=2 的按钮将会覆盖显示在 Index=1 的按钮之上
exCustomBtnGroupSortDepthByIndex(2)

-- 让分组1显示在分组2之上；同一个分组内的按钮或图片按照索引（Index）顺序排列深度
exCustomBtnGroupSortDepthAboveAnotherGroup(1, 2)
```
