---
title: "Lua4.0进阶 - 使用tag模拟面向对象"
catalog_path: "Lua教程"
slug: glgtvl51wgb1zzdk
source: https://www.yuque.com/muzeqaq/ra3mapwiki/glgtvl51wgb1zzdk
fetched: 2026-10-15
content_updated_at: "2024-12-16T21:22:32.000Z"
type: Doc
format: lake
---

# Lua4.0进阶 - 使用tag模拟面向对象

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/glgtvl51wgb1zzdk
> 目录位置：Lua教程

:::success
作者: dreamness

:::

使用tag可以模拟面向对象

> Inertia注： 在现代的Lua版本中（如Lua5.0），tag 和 [tag method](https://www.lua.org/manual/4.0/manual.html#tag-method) 已被元表（[metatable](https://www.lua.org/manual/5.4/manual.html#2.4)）替代。  
>

```lua

-- 定义类
Animal = {}
function Animal:eat(food) end
function Animal:move(pos) end
function Animal:sleep(duration) end

-- 定义一个tag
animal_tag = newtag()

-- 使用tag, 新建对象
function new_animal(name)
    local obj = { name = name }

    settag(obj, animal_tag)

    settagmethod(animal_tag, "index", function(t, key)
        return Animal[key]
    end)

    return obj
end

-- 实例化并使用
cat = new_animal('cat')
cat.name  -- cat
cat:eat('meat')
cat:move(1)
cat:sleep(1000)

dog = new_animal('dog')
-- tag(x)函数会返回table的tag
tag(cat) == tag(dog) -- 真
tag(cat) == tag({}) -- 假
tag(cat) == animal_tag -- 真

```
