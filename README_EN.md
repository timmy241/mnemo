# mnemo

An AI agent framework built in Rust, with WeChat as the first messaging gateway.

## Vision

mnemo isn't about building a more efficient assistant. It's about exploring **human-like context engineering**.

We're trying to answer a question: if an AI's context contains not just instructions and memories, but also values, a subconscious, short-term consciousness, and long-term consciousness — can it evolve on its own through conversation, like a human does?

mnemo focuses on:

- **Values** — knowing what the "right" response is, not through rule enforcement, but through internalization as part of context
- **Subconscious** — underlying cognition that doesn't need explicit recall but continuously shapes behavior
- **Short-term consciousness** — current conversation state, emotional awareness, immediate intent
- **Long-term consciousness** — deep understanding of a person, relationship evolution, memory sedimentation
- **Self-evolution** — the agent continuously refines its own behavioral patterns through dialogue, rather than relying on manual tuning

We're not considering MCP, Skill, Tool Use, or other efficiency-oriented capabilities in the early stages. mnemo's first priority is **companionship and emotional value**, not productivity.

## Tech Stack

- **Language:** Rust (2024 edition)
- **Runtime:** Tokio (async)
- **LLM:** Xiaomi mimo-v2.5-pro (OpenAI-compatible API)
- **Gateway:** WeChat iLink Bot protocol (ilinkai.weixin.qq.com)
- **Dev Tool:** Claude Code

## Project Structure

```
mnemo/
├── src/                    # Main binary (CLI + orchestration)
│   ├── main.rs             # CLI entry (clap)
│   ├── cli.rs              # Command definitions
│   ├── config.rs           # ~/.mnemo/config.json management
│   ├── setup.rs            # Interactive setup wizard
│   ├── run.rs              # Agent runtime loop
│   └── llm.rs              # OpenAI-compatible LLM client
│
├── mnemo-gateway/          # Messaging gateway layer
│   └── wechat/             # WeChat iLink Bot protocol implementation
│       ├── client.rs       # HTTP client (login, polling, send)
│       ├── gateway.rs      # Gateway trait implementation
│       └── types.rs        # iLink API types
│
├── mnemo-core/             # Agent loop logic
│   ├── agent.rs            # AgentLoop (message → LLM → response)
│   ├── message.rs          # MessageProcessor trait
│   └── types.rs            # AgentConfig, LlmRequest/Response
│
└── mnemo-memory/           # Context memory layer
    ├── memory.rs           # MemoryStore trait
    └── types.rs            # MemoryEntry, MemoryQuery
```

## Quick Start

### 1. Setup

```bash
cargo run setup
```

Interactive configuration:

- **Model** — API base URL, API key, available models, selected model
- **WeChat** — QR code scan login, token auto-saved

Config is saved to `~/.mnemo/config.json`:

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

### 2. Run

```bash
cargo run
```

Reads config → connects to WeChat (reuses saved token) → starts polling for messages → queries LLM → replies automatically.

### 3. CLI

```
mnemo - an AI agent

Usage: mnemo [COMMAND]

Commands:
  setup  Interactive setup: configure model and WeChat login
  help   Print this message or the help of the given subcommand(s)
```

## How It Works

```
WeChat User
    │
    ▼
┌─────────────────────┐
│   mnemo-gateway     │  Long-polling getupdates
│   (WeChat iLink)    │  ←────────────────────── ilinkai.weixin.qq.com
└────────┬────────────┘
         │ IncomingMessage (mpsc)
         ▼
┌─────────────────────┐
│   mnemo (run.rs)    │  Message loop
│   receive → LLM     │
│   → send_reply      │
└────────┬────────────┘
         │
         ▼
┌─────────────────────┐
│   LLM (mimo-v2.5)   │  POST /v1/chat/completions
└─────────────────────┘
```

## Roadmap

**Phase 1 — Foundation**
- [x] WeChat gateway (iLink Bot protocol)
- [x] Interactive CLI setup
- [x] Single-turn LLM query
- [ ] Multi-turn conversation (context window)
- [ ] Persistent memory (mnemo-memory)

**Phase 2 — Context Engineering**
- [ ] System prompt as "personality substrate" (values & personality)
- [ ] Short-term consciousness (conversation state, emotion tracking)
- [ ] Long-term consciousness (memory recall, relationship evolution)
- [ ] Subconscious layer (implicit behavioral patterns)

**Phase 3 — Self-Evolution**
- [ ] Self-reflection: agent evaluates its own responses
- [ ] Behavioral evolution: adjust patterns based on conversation feedback
- [ ] Identity continuity: consistent persona across sessions
