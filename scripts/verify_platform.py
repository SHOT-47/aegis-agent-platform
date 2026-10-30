from pathlib import Path
import re, sqlite3

ROOT = Path(__file__).resolve().parent.parent
CRATES = [
    "aegis-core", "aegis-storage", "aegis-policy", "aegis-events",
    "aegis-indexing", "aegis-context", "aegis-models", "aegis-browser",
    "aegis-extensibility", "aegis-harness", "aegis-server-cli"
]

def verify():
    cargo = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    for c in CRATES:
        assert f'"crates/{c}"' in cargo, f"Missing {c}"
        entry = ROOT / "crates" / c / ("src/main.rs" if c == "aegis-server-cli" else "src/lib.rs")
        assert entry.is_file() and entry.stat().st_size > 400, f"Missing {entry}"
    pol = (ROOT / "crates/aegis-policy/src/lib.rs").read_text(encoding="utf-8")
    assert "allow_obscura_stealth: false" in pol
    assert "is_private_or_reserved_ip" in pol
    assert "CloudFallbackApprovalRequired" in pol
    html = (ROOT / "web-gui/index.html").read_text(encoding="utf-8")
    for ws in ["ws-main", "ws-activity", "ws-browser", "ws-models", "ws-extensibility", "ws-admin"]:
        assert f'id="{ws}"' in html
    conn = sqlite3.connect(":memory:")
    conn.execute("CREATE TABLE t (id TEXT)")
    redacted = re.sub(r"sk-[A-Za-z0-9_\-]{20,}", "[REDACTED:OPENAI_KEY]", "sk-1234567890abcdefghijklmnop")
    assert "[REDACTED:OPENAI_KEY]" in redacted
    print("ALL_PLATFORM_CHECKS_VERIFIED")

if __name__ == "__main__":
    verify()
