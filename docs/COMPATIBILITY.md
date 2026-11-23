# Runtime Compatibility Guide — V0.2

This document details compatibility between the Skill Router, host coding agents, and operating systems.

## 1. Supported Agent Runtimes

| Agent Runtime | Skill Support | Sub-agent Support | Skill Installation Path | Orchestration Support |
|---|---|---|---|---|
| **Antigravity** | Native | Runtime-dependent (Supported via `invoke_subagent` when live) | `.agents/skills/` | Mode A, Mode B, Mode C fallback |
| **Claude Code** | Native | Runtime-dependent | `.claude/skills/` | Mode A, Mode C fallback |
| **Codex** | Native | Sequential session | `.agents/skills/` | Mode A, Mode C fallback |
| **Cursor** | Native | Single-session | `.agents/skills/` | Mode A, Mode C fallback |
| **Gemini CLI** | Native | Single-session | `.agents/skills/` | Mode A, Mode C fallback |
| **GitHub Copilot** | Native | Single-session | `.agents/skills/` | Mode A, Mode C fallback |
| **Windsurf** | Native | Single-session | `.windsurf/skills/` | Mode A, Mode C fallback |

---

## 2. Platform & Engine Requirements

- **Node.js:** Node.js `>= 20.0.0` (LTS 20 or LTS 22 recommended).
- **Operating Systems:**
  - Linux (Ubuntu 20.04+, Debian, Fedora, Arch)
  - macOS (macOS 12+ Apple Silicon and Intel)
  - Windows (Windows 10, Windows 11, PowerShell, and cmd.exe)
- **Official Skills CLI:** Node.js `>= 22.20` is required if executing upstream `skills add` via Vercel Skills CLI.

---

## 3. Sub-Agent Execution Policy

1. **Truthful Reporting:** If the router is executed from a CLI session without a live subagent bridge (e.g. standalone Node.js process), the router will plan Mode B waves if requested, but will execute via **Mode C: Sequential Fallback** and explicitly log the downgrade reason.
2. **Never Emulate Concurrency:** The router never writes fake concurrent prompts and claims multi-agent parallelism occurred when it did not.
