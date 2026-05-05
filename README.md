# mnemo

一个用 Rust 构建的 AI Agent 框架，以微信作为第一个消息网关。

## 愿景

mnemo 的目标不是做一个更高效的助手，而是探索**类人的上下文工程**。

我们试图回答一个问题：如果一个 AI 的上下文里不只有指令和记忆，还有价值观、潜意识、短期意识、长期意识，它能不能像一个人一样，通过对话自我进化？

具体来说，mnemo 关注：

- **价值观** — 什么样的回应是"对的"，不是靠规则约束，而是内化为上下文的一部分
- **潜意识** — 不需要显式召回但持续影响行为的底层认知
- **短期意识** — 当下的对话状态、情绪感知、即时意图
- **长期意识** — 对一个人的长期理解、关系演变、记忆沉淀
- **自我进化** — Agent 通过对话不断修正自己的行为模式，而非依赖人工调参

前期不考虑 MCP、Skill、Tool Use 等效率导向的能力。mnemo 的第一优先级是**陪伴和情绪价值**，而非生产力工具。

## 技术栈

- **语言:** Rust (2024 edition)
- **运行时:** Tokio (async)
- **模型:** 小米 mimo-v2.5-pro (OpenAI 兼容 API)
- **网关:** 微信 iLink Bot 协议 (ilinkai.weixin.qq.com)
- **开发工具:** Claude Code

## 项目结构

```
mnemo/
├── src/                    # 主程序 (CLI + 编排)
│   ├── main.rs             # CLI 入口 (clap)
│   ├── cli.rs              # 命令定义
│   ├── config.rs           # ~/.mnemo/config.json 配置管理
│   ├── setup.rs            # 交互式配置向导
│   ├── run.rs              # Agent 运行时循环
│   └── llm.rs              # OpenAI 兼容的 LLM 客户端
│
├── mnemo-gateway/          # 消息网关层
│   └── wechat/             # 微信 iLink Bot 协议实现
│       ├── client.rs       # HTTP 客户端 (登录、轮询、发送)
│       ├── gateway.rs      # Gateway trait 实现
│       └── types.rs        # iLink API 类型定义
│
├── mnemo-core/             # Agent 核心逻辑
│   ├── agent.rs            # AgentLoop (消息 → LLM → 回复)
│   ├── message.rs          # MessageProcessor trait
│   └── types.rs            # AgentConfig, LlmRequest/Response
│
└── mnemo-memory/           # 上下文记忆层
    ├── memory.rs           # MemoryStore trait
    └── types.rs            # MemoryEntry, MemoryQuery
```

## 快速开始

### 1. 配置

```bash
cargo run setup
```

交互式配置：

- **模型** — API Base URL、API Key、可用模型列表、当前使用的模型
- **微信** — 扫码登录，Token 自动保存

配置保存在 `~/.mnemo/config.json`：

```json
{
  "wechat_token": {
    "token": "...",
    "base_url": "https://ilinkai.weixin.qq.com",
    "bot_id": "...",
    "user_id": "...",
    "saved_at": "..."
  },
  "model": {
    "base_url": "https://api.example.com",
    "api_key": "sk-...",
    "model_list": ["mimo-v2.5", "mimo-v2.5-pro"],
    "select_model": "mimo-v2.5-pro"
  }
}
```

### 2. 运行

```bash
cargo run
```

读取配置 → 连接微信（复用已保存的 Token）→ 开始轮询消息 → 调用 LLM → 自动回复。

### 3. CLI

```
mnemo - an AI agent

Usage: mnemo [COMMAND]

Commands:
  setup  Interactive setup: configure model and WeChat login
  help   Print this message or the help of the given subcommand(s)
```

## 工作流程

```
微信用户
    │
    ▼
┌─────────────────────┐
│   mnemo-gateway     │  长轮询 getupdates
│   (WeChat iLink)    │  ←────────────────────── ilinkai.weixin.qq.com
└────────┬────────────┘
         │ IncomingMessage (mpsc)
         ▼
┌─────────────────────┐
│   mnemo (run.rs)    │  消息循环
│   收消息 → 调 LLM   │
│   → 回复            │
└────────┬────────────┘
         │
         ▼
┌─────────────────────┐
│   LLM (mimo-v2.5)   │  POST /v1/chat/completions
└─────────────────────┘
```

## 路线图

**第一阶段 — 基础通路**
- [x] 微信网关 (iLink Bot 协议)
- [x] 交互式 CLI 配置
- [x] 单轮 LLM 对话
- [ ] 多轮对话 (上下文窗口)
- [ ] 持久化记忆 (mnemo-memory)

**第二阶段 — 上下文工程**
- [ ] System Prompt 作为"人格基底" (价值观与性格)
- [ ] 短期意识 (当前对话状态、情绪感知)
- [ ] 长期意识 (记忆召回、关系演变)
- [ ] 潜意识层 (隐式行为模式)

**第三阶段 — 自我进化**
- [ ] 自我反思: Agent 评估自己的回复质量
- [ ] 行为进化: 根据对话反馈调整行为模式
- [ ] 身份延续: 跨会话的一致人格
