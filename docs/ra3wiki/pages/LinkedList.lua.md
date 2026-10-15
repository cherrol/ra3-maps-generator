---
title: "LinkedList.lua"
catalog_path: "Lua例子/【日冕】太刀新弹射护盾的相关 Lua 代码"
slug: xxwr6h3pcqgz18a1
source: https://www.yuque.com/muzeqaq/ra3mapwiki/xxwr6h3pcqgz18a1
fetched: 2026-10-15
content_updated_at: "2024-09-30T04:08:41.000Z"
type: Doc
format: lake
---

# LinkedList.lua

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/xxwr6h3pcqgz18a1
> 目录位置：Lua例子/【日冕】太刀新弹射护盾的相关 Lua 代码

```lua
-- 该文件可以被任何其他 Lua 文件调用，以提供双向链表的功能。
-- 调用方式：RunRA3Lua("Data\\Scripts\\LinkedList.lua")

if LinkedList ~= nil then
    -- 假如双向链表已经可以使用了，那么就不需要再次加载了。
    return
end

-- 调用此函数，可以得到一个空的双向链表。
function LinkedList()
    -- 链表是由一系列连接的节点（Node）组成的。
    -- 所有的节点都有一个 Next，指向下一个节点。
    -- 以及 Previous，指向上一个节点。
    -- 为了方便，链表本身也是一个节点，称为“锚点” (anchor)。
    -- 锚点与普通的节点类似，都是有 Previous 和 Next 的。
    -- 锚点的 Next 指向链表的第一个节点，Previous 指向链表最后一个节点。
    -- 因此，链表的第一个节点的 Previous 指向锚点，
    -- 而链表最后一个节点的 Next 也指向锚点。
    -- 假如链表是空的（刚刚创建的链表即是如此），那么 Next 和 Previous 都指向锚点自身。
    -- 假如链表只有一个节点，那么链表的 Next 和 Previous 都指向第一个节点，
    -- 而该节点的 Next 和 Previous 都指向链表的锚点。

    -- 假如要遍历链表，可以这样写：
    -- local anchor = (某个链表).Anchor
    -- local node = anchor.Next
    -- while node ~= anchor do
    --     -- do something
    --     node = node.Next
    -- end

    -- 为了保证不会写出 BUG，请不要直接修改 Next 和 Last 的值。
    -- 读取肯定是可以的，但是贸然修改，假如写错了就麻烦了
    -- 假如想要修改，请使用 AppendNode 在尾部添加链表，或者用 RemoveNode 删除链表中的节点。
    -- 这样就不会出错了。

    -- 但是依然需要注意三点：
    -- 1. 不要操作锚点。
    --      list:AppendNode(list.Next) 在 list 为空的时候会出错，因为此时 list.Next 是锚点。
    -- 2. 一个节点不能同时存在于两个链表中。
    --      list1:AppendNode(self)
    --      list2:AppendNode(self) 会出错。
    -- 3. 节点从链表中移除之后，Next 和 Previous 会被置为 nil。
    --      list:RemoveNode(self)
    --      local y = self.Next.XXX 会出错，因为此时 self.Next 为 nil。

    -- 用例：
    -- local anchor = (某个链表).Anchor
    -- local node = anchor.Next
    -- while node ~= anchor do
    --     local next = node.Next
    --     if (某个条件) then
    --         (某个链表):RemoveNode(node)
    --     end
    --     node = next
    -- end
    -- (某个链表):AppendNode(self)
    -- -- 调用之后，self.Previous 指向原来的最后一个节点，self.Next 指向锚点。

    local anchor = {}
    anchor.Previous = anchor
    anchor.Next = anchor
    anchor.Anchor = anchor

    anchor.AppendNode = function(anchor, node)
        if node.Previous ~= nil or node.Next ~= nil then
            error("LinkedList.AppendNode: node is already in a list.")
        end
        local previous = anchor.Previous
        previous.Next = node
        node.Next = anchor
        node.Previous = previous
        anchor.Previous = node
        -- 假如要添加的节点将会是链表的第一个节点，那么除了 anchor.Previous 之外，anchor.Next 也需要修改。
        -- 但是由于 anchor.Previous 指向 anchor 自身，因此 previous.Next 已经被修改了。
    end

    anchor.RemoveNode = function(anchor, node)
        if anchor == node then
            error("LinkedList.RemoveNode: cannot remove anchor from list.")
        end
        if node.Previous == nil or node.Next == nil then
            error("LinkedList.RemoveNode: node is not in a list.")
        end
        local previous = node.Previous
        local nextNode = node.Next
        previous.Next = nextNode
        nextNode.Previous = previous
        node.Previous = nil
        node.Next = nil
        -- 假如要被删除的节点是链表的最后一个节点，那么 anchor.Previous 也需要修改。
        -- 但是由于最后一个节点的 Next 指向 anchor，因此 nextNode.Previous 已经被修改了。
        -- 同理，假如要被删除的节点是链表的第一个节点，那么 anchor.Next 也需要修改。
        -- 但是由于第一个节点的 Previous 指向 anchor，因此 previous.Next 已经被修改了。
    end

    return anchor
end
```
