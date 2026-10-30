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
