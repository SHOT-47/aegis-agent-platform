# Security, Workspace Boundary, Command Approval & Risk Policy Model

- Risk Tiers: Low (auto within workspace), Elevated (requires explicit user approval), Prohibited (blocked).
- Shell Command Approval: Bound to SHA256(argv || cwd || env_deltas). Any parameter change invalidates approval.
- SSRF & Obscura Safety: Blocks loopback/private/link-local IPs across DNS resolution and HTTP redirects; enforces Obscura stealth=false.
- Secret Redaction: Scrubs OpenAI, GitHub, AWS, Bearer, PEM, and Vault secrets before SQLite persistence or prompt assembly.
