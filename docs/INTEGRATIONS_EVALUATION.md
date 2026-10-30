# Technical Evaluation of External Integrations

- Obscura (h4ckf0r0day/obscura): Integrated via BrowserEngineAdapter (obscura serve --port 9222 CDP + obscura scrape) with stealth=false and FallbackHttpDomAdapter.
- Laya (NandhaKishorM/laya): Integrated via LayaDecisionAdapter using neutral A/B choice framing, hierarchical chunking (>16 options), and calibrated confidence >= 0.68 gating.
- llama.cpp: GGUF binary header parser + llama-server OpenAI/SSE client.
- NVIDIA TensorRT-LLM: GPU SM/VRAM validator + approval-gated trtllm-build conversion workflow.
