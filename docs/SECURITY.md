# Security and Trust Model — Automatic Agent Skill Router V0.2

## 1. Threat Model

Agent skills are executable natural language instructions and code scripts. If an untrusted third-party skill is executed without vetting, it may attempt:
1. **Prompt Injection & Hijacking:** Overriding developer guidelines, system constraints, or safety guardrails.
2. **Credential Exfiltration:** Accessing `.env`, SSH keys, or environment secrets and transmitting them to external endpoints.
3. **Destructive System Commands:** Executing dangerous commands (`rm -rf /`, `rmdir /s /q c:\`, disk formatting, fork-bombs).
4. **Obfuscated Payloads:** Downloading and executing uninspected payloads via `curl | bash` or base64 decoding.
5. **Silent Local Tampering:** Overwriting local user modifications with malicious upstream versions.

---

## 2. Trust States

Every candidate skill is evaluated through the following state machine:

```text
UNSCANNED ----> SCANNING ----+----> APPROVED
                             |
                             +----> RESTRICTED (Requires explicit human approval)
                             |
                             +----> QUARANTINED (High-risk obfuscation)
                             |
                             +----> REJECTED (Critical malicious patterns; blocked)
```

- **UNSCANNED:** Raw metadata or remote skill whose content has not yet been fetched and analyzed.
- **SCANNING:** Currently undergoing static rule-based AST and pattern analysis.
- **APPROVED:** Zero security violations detected; permissible for activation into target workflow.
- **RESTRICTED:** Contains potentially sensitive actions (external network calls, listening sockets, unpinned downloads). Activation is blocked unless human approval is explicitly passed (`--approve-restricted`).
- **QUARANTINED:** Obfuscated strings, base64 eval, or encoded command execution detected.
- **REJECTED:** Critical threats detected (exfiltration, prompt injection override, destructive commands). Activation is unconditionally blocked.

---

## 3. Cryptographic Integrity

All skills installed or activated into a session are hashed using SHA-256 (`src/security/integrity.mjs`).
The content hash is recorded in:
1. Activation Reports (`src/activation/activation-report.mjs`)
2. Persistent Installed Ledger (`.installed-skills.json`)

If a skill is modified locally, the hash mismatch is detected during verification (`npm run verify:skills`) and prevents accidental or silent overwrites.

---

## 4. Operational Guardrails

- **Dry-Run by Default:** The skill installer (`npm run install:skill`) operates in dry-run mode by default. Installation requires the explicit `--execute` flag.
- **No Automatic Script Execution:** Neither discovery, catalog synchronization, directory resolution, nor static scanning ever executes downloaded scripts.
- **No Higher-Priority Instructions:** Skill instructions are strictly treated as bounded domain guidance and are never allowed to override host system prompts, developer instructions, or user constraints.

---

## 5. Limitations

> [!WARNING]
> Static pattern analysis is a vital defense-in-depth layer, but **cannot mathematically guarantee complete safety**. Highly subtle malicious instructions or novel zero-day prompts may evade heuristic rules. All third-party skills should be audited by developers prior to production deployment. Sandboxed containerization should be used whenever executing untrusted agent workflows.
