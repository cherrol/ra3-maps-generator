---
title: "xml脚本规范"
catalog_path: "地编脚本列表/xml脚本规范"
slug: zbnpgq7gs0mbvl1k
source: https://www.yuque.com/muzeqaq/ra3mapwiki/zbnpgq7gs0mbvl1k
fetched: 2026-10-15
content_updated_at: "2024-05-11T13:17:03.000Z"
type: Doc
format: lake
---

# xml脚本规范

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/zbnpgq7gs0mbvl1k
> 目录位置：地编脚本列表/xml脚本规范

```xml
<?xml version="1.0" encoding="utf-8"?>
<Ra3Map xmlns="uri:wu.com:ra3map">
  <MapScript>
    <ScriptList
      Name="">     <!--        文件夹名字，不要增加或者修改-->

      <!--        脚本，每个脚本必须有Name,其他属性可以不设置，有默认值，默认值和地编一样-->
      <!--        Name 脚本名字 必须要有-->
      <!--        Conmment 脚本注释  可选，默认空-->
      <!--        ConditionComment 脚本条件注释  可选，默认空-->
      <!--        ActionComment 脚本动作指注释  可选，默认空-->
      <!--        IsSubroutine 是否子脚本  可选，默认否-->
      <!--        DeactivateUponSuccess 执行完脚本之后是否停止执行  可选，默认是-->
      <!--        isActive 是否启用脚本 可选，默认是-->
      <!--        ActiveInEasy 是否在该ai简单度启用 默认是-->
      <!--        ActiveInMedium 是否在该ai中等度启用 默认是-->
      <!--        ActiveInHard 是否在该ai困难度启用 默认是-->
      <!--        EvaluationInterval 循环次数，需要开启LoopActions才生效 默认0-->
      <!--        LoopActions 是否循环脚本 默认否-->
      <!--        ActionsFireSequentially 是否顺序脚本 默认否-->
      <!--        LoopCount 顺序脚本执行次数 默认0-->
      <!--        SequentialTargetName 脚顺序脚本目标名称 默认空-->
      <Script
        Name="banInfantry"        
        Conmment=""
        ConditionComment=""
        ActionComment=""
        IsSubroutine="false"
        DeactivateUponSuccess="true"
        isActive="true"
        ActiveInEasy="true"
        ActiveInMedium="true"
        ActiveInHard="true"
        EvaluationInterval="0"
        LoopActions="false"
        ActionsFireSequentially="false"
        LoopCount="0"
        SequentialTargetName="">   
        <If>      <!--        条件-->
          <OrCondition>        <!--        每个or条件组-->
            <CONDITION_TRUE />      <!--        具体的条件-->
          </OrCondition>
        </If>
        <Then>  <!--        条件为真执行的动作-->
          <ALLOW_DISALLOW_ONE_BUILDING>            <!--        具体的动作-->
            <PlayerName_0 value="Player_4"/>             <!--        这里面是参数-->
            <ObjectName_1 value="sovietairfield"/>
            <Boolean_2 value="0"/>
          </ALLOW_DISALLOW_ONE_BUILDING>
        </Then>
        <Else />    <!--        条件为假执行的动作-->
      </Script>

      <!--        脚本组，每个脚本组必须有Name,其他属性可以不设置，有默认值，默认值和地编一样-->
      <!--        Name 脚本组名字 必须要有-->
      <!--        IsSubroutine 是否子脚本  可选，默认否-->
      <!--        IsActive 是否启用脚本组  可选，默认是-->
      <ScriptGroup 
        Name="脚本组名字"
        IsSubroutine="false"
        IsActive="true">
        <Script Name="脚本名字">
          <If>
            <OrCondition>
              <CONDITION_TRUE/>
            </OrCondition>
          </If>
          <Then/>
          <Else/>
        </Script>
      </ScriptGroup>
    </ScriptList>
  </MapScript>
</Ra3Map>
```
