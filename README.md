# Aegis -- High-Performance Rust-Based Autonomous AI Coding & Research Agent

Aegis provides a persistent autonomous coding and web research harness with two synchronized interfaces:
1. Terminal Interface (CLI + Ratatui TUI): `aegis tui` and `aegis <subcommand>`
2. Web Browser GUI (6 Workspaces): `aegis serve --bind 127.0.0.1:18420`

## Quick Start
    python3 scripts/verify_platform.py
    cargo build --workspace --release
    ./target/release/aegis doctor
    ./target/release/aegis serve --bind 127.0.0.1:18420
    ./target/release/aegis tui


Awesome — `project.zip` (`8acb1ef6...`, all 40 files verified) is built and ready in `C:\Users\0\Downloads\project.zip`, and the unpacked source tree is ready in `C:\Users\0\Downloads\aegis-agent-platform`!

### How to Run & Use Your Project on Windows

1. **Open the Web Browser GUI immediately (no compile needed to preview the UI):**
   ```powershell
   Start-Process "C:\Users\0\Downloads\aegis-agent-platform\web-gui\index.html"
   ```

2. **Compile and run the native Rust runtime & CLI/TUI (requires [Rust / Cargo](https://rustup.rs)):**
   ```powershell
   cd C:\Users\0\Downloads\aegis-agent-platform

   # Build all 11 Rust crates in release mode
   cargo build --workspace --release

   # Run platform diagnostics (models, router, Laya threshold)
   .\target\release\aegis.exe doctor

   # Index the codebase incrementally with SHA-256 Merkle tracking
   .\target\release\aegis.exe index --query "PolicyEngine"

   # Launch the Terminal TUI
   .\target\release\aegis.exe tui

   # Or start the localhost REST + WebSocket API daemon (127.0.0.1:18420)
   .\target\release\aegis.exe serve --bind 127.0.0.1:18420
   ```

3. **Or run the Windows PowerShell installer:**
   ```powershell
   .\scripts\install.ps1
   ```

Let me know if you want to expand any crate, add new tools/plugins, or customize anything else!
