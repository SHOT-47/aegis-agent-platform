# Aegis Architecture Specification

## 11-Crate Modular Rust Workspace
1. aegis-core: 22 AgentActivityState variants, plans, tasks, SHA-256 command approval, error taxonomy.
2. aegis-storage: SQLite WAL persistence, crash-recovery checkpoints, credential vault, secret redactor.
3. aegis-policy: Centralized 3-tier risk policy engine (Low, Elevated, Prohibited), workspace jail, SSRF guard.
4. aegis-events: Broadcast live activity event bus, heartbeat monitor, stage-aware stuck detector.
5. aegis-indexing: Incremental SHA-256 Merkle codebase indexer, symbol/call-graph extractor, hybrid search.
6. aegis-context: Continuous Token Cleanup Loop + Token Assist Engine + SubwordBpeTokenizer + 13 protected anchors.
7. aegis-models: llama.cpp GGUF adapter, NVIDIA TensorRT-LLM validator, Laya System-1 adapter, ModelRouter.
8. aegis-browser: Obscura CDP/CLI browser adapter (stealth=false), fallback extractor, SSRF-safe batch scraper.
9. aegis-extensibility: MCP stdio/HTTP-SSE client, sandboxed plugins, progressive-disclosure skills, 13-stage hooks.
10. aegis-harness: 12-step continuous autonomous harness, 12 specialist agents, conflict reconciler, self-recovery.
11. aegis-server-cli: Localhost Axum REST/WebSocket server, Ratatui TUI, and scriptable CLI.
