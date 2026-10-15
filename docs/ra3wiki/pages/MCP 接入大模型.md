---
title: "MCP 接入大模型"
catalog_path: "地编伴侣"
slug: gvdnzevp57yfqu6k
source: https://www.yuque.com/muzeqaq/ra3mapwiki/gvdnzevp57yfqu6k
fetched: 2026-10-15
content_updated_at: "2025-11-10T03:47:34.000Z"
type: Doc
format: lake
---

# MCP 接入大模型

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/gvdnzevp57yfqu6k
> 目录位置：地编伴侣

启动地编伴侣后, 会开启mcp服务, 可对接大模型

地址: http://127.0.0.1:30033/mcp



# 对接 Cherry Studio
cherry studio是一个大模型对话客户端, 即可以与在线的deepseek, chatgpt等模型对话, 也可以与本地模型对话

1. 在设置中配置mcp

![](https://cdn.nlark.com/yuque/0/2025/png/34558013/1762745601111-22d61f27-9651-478f-8bcc-2f0dfd3bc962.png)

2. 直接对话! (注意在对话框中勾选mcp)

![](https://cdn.nlark.com/yuque/0/2025/png/34558013/1762745660714-b29394a9-e6c3-4fab-8d49-2008ec047b1a.png)

![](https://cdn.nlark.com/yuque/0/2025/png/34558013/1762745676248-6ac8c502-19c7-42f8-87ff-dd87e60c3e6f.png)

# 
# 对接 lm studio
lm studio可以本地运行自己的大模型, 接入mcp后, 可以用自己的模型操作ra3地图

注意, 使用下方的"developer"模式, 添加mcp配置即可

![](https://cdn.nlark.com/yuque/0/2025/png/34558013/1762745799643-d9668644-e29f-4a3c-b452-1a368c6fa01e.png)

```json
{
  "mcpServers": {
    "Ra3MapUtils": {
      "url": "http://127.0.0.1:30033/mcp"
    }
  }
}
```

![](https://cdn.nlark.com/yuque/0/2025/png/34558013/1762745893962-dbd87b4e-af80-46f9-a0ca-ad4790216110.png)

![](https://cdn.nlark.com/yuque/0/2025/png/34558013/1762745917465-75d83e4f-9b47-477a-8718-c1b73b8314b0.png)

# 对接cursor
cursor是著名的ai agent, 强大的agent可处理复杂的代码问题

![](https://cdn.nlark.com/yuque/0/2025/png/34558013/1762746044869-dcb40e6a-51cc-439d-9c15-d8e6815f9510.png)

```json
{
  "mcpServers": {
    "Ra3MapUtils": {
      "url": "http://127.0.0.1:30033/mcp"
    }
  }
}
```

配置后

![](https://cdn.nlark.com/yuque/0/2025/png/34558013/1762746095220-ca400bf5-8602-467b-9bfe-533ca98d82f5.png)

![](https://cdn.nlark.com/yuque/0/2025/png/34558013/1762746169478-7322c3de-549e-4ad4-82e1-ed1c61b04b4f.png)
