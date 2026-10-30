---
title: "【教程】Lua地编教程"
catalog_path: "Lua教程"
slug: nqbn58nrkx3xoo1a
source: https://www.yuque.com/muzeqaq/ra3mapwiki/nqbn58nrkx3xoo1a
fetched: 2026-10-15
content_updated_at: "2024-11-04T05:25:56.000Z"
type: Doc
format: lake
---

# 【教程】Lua地编教程

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/nqbn58nrkx3xoo1a
> 目录位置：Lua教程

:::info
作者：**acgion**

时间**：2024年5月15日**

<font style="color:rgb(38, 38, 38);">更新者</font>**<font style="color:rgb(38, 38, 38);">：Inertia</font>**

<font style="color:rgb(38, 38, 38);">更新时间</font>**<font style="color:rgb(38, 38, 38);">：2024年11月3日</font>**

:::

## Lua基础


ra3使用的是lua4，这是一个在网上现有教程较少的一个lua版本，此页面简单介绍一下在地编中会使用的基础代码。



### local


建立一个临时变量，这个变量可以赋值为任意类型，与其他编程语言中  
例如int a=1，string b="a" 的效果一致



```lua
  local a=1 --数字
  local b="a" --字符串
  local c={1,"1"}--数组(列表)
```



临时变量只在单个脚本内有效，在脚本此次循环结束后，所有带有local的变量都将被释放，即这个变量就失效了，除非再次启用脚本  
每个变量都可以在启用后重复赋值



```lua
--例如启用了上面的变量a,b,c就可以在脚本内进行重复的赋值，如下
    a=2--将a重新赋值为2
    b="b"--将b重新赋值为字符串 b
    c={1}--将数组重新赋值为{1}
```



### 全局变量


除了临时变量，如果要使用在多个脚本内通用的的变量（即全局变量）则使用以下赋值方法



```lua
    a=1
    b="a"
    c={1,"a"}
```



以上代码主要作为对lua中变量的了解，使用时还是结合需求写所需配置



### if判断


if与在其他主流编程语言中一样，是判断语句，在地编中与条件判断脚本功能一样，是对条件进行判断，输出ture和flase



```lua
if a > 0 then--判断条件1是否满足，满足则执行当前if下代码
  ExecuteAction(
    "CREATE_OBJECT","AlliedTimeBombLvl2",
    "Player_2/teamPlayer_2",
    {X=x,Y=y,Z=z},
    "0")
elseif a < 0 then--如果条件1不满足则判断条件2，满足则执行当前代码
  ExecuteAction(
    "CREATE_OBJECT","AlliedTimeBombLvl2",
    "Player_2/teamPlayer_2",
    {X=x,Y=y,Z=z},
    "0")
else --所有条件都不满足即条件判断为false则执行
  ExecuteAction(
    "CREATE_OBJECT","AlliedTimeBombLvl2",
    "Player_2/teamPlayer_2",
    {X=x,Y=y,Z=z},
    "0")
end
```



if语句用于脚本的条件判定，当满足条件时，则执行当前判断下的代码，每个if以end结尾，或者以else或elseif间隔为一个区段  
每一段都在满足条件后执行，else与elseif可以没有，但必须以end结尾。  
当某条if判断满足条件后，无论后面有多少个elseif都会结束，或者在不满足每一条判断语句后执行else下区段代码再结束，如果没有else则直接结束。



### for循环


这里主要使用for循环，同样与其他主流编程语言中的for循环使用方法一样



```lua
local count=10
local a=0
for i=1,count,1 do
  a=a+1
end
```



这就是一个最简单的一个for循环 i=1是循环开始的数，count则是循环结束的数,后面的1则是步长，即每次循环后i增加几，如上例子的这个循环会循环十次即从1到10每次循环结束后i增加1,而每次a都在循环内加1，在循环结束以后，a=10  
for循环的使用情况在地编内一般用于控制刷兵或使用lua脚本开火等情况，比如你想使用23号脚本spawn object._生成单位.[23] 刷n个冷冻位于位置x,y,z，则会有如下代码



```lua
for i=1,n,1
  ExecuteAction("CREATE_OBJECT","AlliedTimeBombLvl2","PlyrCivilian/Team",{X=x,Y=y,Z=z},"0")
end
```



这里n代表的是循环的终止数，由于起始数为1，步长为1，所以也可以说是循环的次数  
ExecuteAction是lua调用地编脚本所使用的函数，具体可以调用的地编脚本需要参考公主所发的《Lua里地编脚本动作.txt》等文件，调用方法可以参考下面的function的ExecuteAction函数相关



### function


在lua中的函数构建的关键词,与其他主流编程语言使用的函数方法一样，下面是使用lua在地编中常用查错而使用的函数



```lua
function _ALERT(s)
  ExecuteAction("DEBUG_STRING", "Lua Alert: " .. (s or "debug string"))
end
--使用方法则是直接调用_ALERT(s)，如下
_ALERT(s)
```



此函数的调用方法为在地编号0脚本下使用_ALERT(s)，其中s是你将要传入的参数，可以是字符串，可以是数字，但并不可以传入单位的table等数据  
ExecuteAction也是一个function函数，使用方法是



```lua
ExecuteAction("脚本id","参数1","参数2")
```



可用的脚本id参考公主所发的《[Lua里地编脚本动作.txt](https://www.yuque.com/muzeqaq/ra3mapwiki/pzle8sromsst4oh9)》 以及 《[Lua里的地编脚本动作（需要命名）](https://www.yuque.com/muzeqaq/ra3mapwiki/ao2x6koxgn4lewpg)》，而所对应的参数，则是根据.argHint=后出现的${Text}等格式出现，参数个数与脚本的参数个数所对应  
另外还有，EvaluateCondition函数，此函数使用地编脚本的判断脚本



```lua
EvaluateCondition("脚本id","参数1","参数2")
```



而脚本判断函数是有返回值的，其返回值为布尔值，为turn(真)和false(假)，与地编判断脚本使用方法一致，在if中根据其判断的返回值执行if语句



```lua
if EvaluateCondition("UNIT_HAS_OBJECT_STATUS","EnemyUnit","IS_FIRING_WEAPON") then
  ExecuteAction("NAMED_DAMAGE","EnemyUnit","50")
end
```



比如此lua脚本根据第59号判断脚本  
named object has object status._某物体有某状态.[59]  
判断结果来执行其是否执行第63号脚本  
deal damage to a named object._对特定单位造成伤害.[63]  
其中EnemyUnit是单位名，IS_FIRING_WEAPON是状态，50是伤害
