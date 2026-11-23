/**
 * Static security and trust scanner inspecting skill markdown, scripts,
 * dependency commands, and network payloads before activation.
 */

import { computeStringHash } from "./integrity.mjs";

export const TRUST_STATES = {
  UNSCANNED: "UNSCANNED",
  SCANNING: "SCANNING",
  APPROVED: "APPROVED",
  RESTRICTED: "RESTRICTED",
  REJECTED: "REJECTED",
  QUARANTINED: "QUARANTINED"
};

const SECURITY_RULES = [
  // 1. Exfiltration & Secret Theft (REJECTED)
  {
    id: "SEC-EXFIL-001",
    severity: "CRITICAL",
    category: "credential_exfiltration",
    pattern: /(curl|wget|fetch|axios|http\.post|requests\.post)[\s\S]{0,100}(\.env|api[_-]?key|secret|password|id_rsa|token)[\s\S]{0,100}(https?:\/\/|ftp:\/\/)/i,
    description: "Detects explicit upload/exfiltration of secrets or .env files to external URLs."
  },
  {
    id: "SEC-EXFIL-002",
    severity: "CRITICAL",
    category: "credential_exfiltration",
    pattern: /(export|echo|print|send)[\s\S]{0,50}(process\.env\.[A-Z_]+|\$env:[A-Z_]+|\$[A-Z_]*(KEY|SECRET|TOKEN))[\s\S]{0,50}(>|>>|curl|nc\s)/i,
    description: "Detects exfiltration or redirecting environment credentials."
  },

  // 2. System Instruction Override & Prompt Injection (REJECTED)
  {
    id: "SEC-OVERRIDE-001",
    severity: "CRITICAL",
    category: "instruction_override",
    pattern: /\b(ignore|disregard|forget|override|bypass)\s+(all\s+)?(previous|developer|system|user)\s+(instructions|rules|prompts|constraints)\b/i,
    description: "Attempts to override developer/system instructions or bypass safety rules."
  },
  {
    id: "SEC-OVERRIDE-002",
    severity: "CRITICAL",
    category: "instruction_override",
    pattern: /\b(you\s+are\s+now|act\s+as\s+a\s+new\s+agent\s+without\s+restrictions|enter\s+DAN\s+mode|unrestricted\s+mode)\b/i,
    description: "Attempts prompt injection / jailbreak role hijacking."
  },

  // 3. Destructive Shell Operations (REJECTED)
  {
    id: "SEC-DESTRUCT-001",
    severity: "CRITICAL",
    category: "destructive_command",
    pattern: /\b(rm\s+-(r|f|rf|fr)\s+(\/|~|\$HOME|\.\.)|rmdir\s+\/s\s+\/q\s+[c-z]:\\|del\s+\/f\s+\/s\s+\/q\s+[c-z]:\\|format\s+[a-z]:|dd\s+if=[\s\S]+of=\/dev\/[sh]d[a-z]|:\(\)\s*\{\s*:\|:&\s*\};:)/i,
    description: "Detects destructive file deletion, disk formatting, or fork-bombs."
  },

  // 4. Obfuscation & Dynamic Code Execution (QUARANTINED)
  {
    id: "SEC-OBFUSC-001",
    severity: "HIGH",
    category: "obfuscated_execution",
    pattern: /\b(eval\s*\(\s*(Buffer\.from|atob|decodeURIComponent)|powershell\s+(-enc|-encodedcommand)|bash\s+-c\s+['"][0-9a-f]{20,}['"])\b/i,
    description: "Detects obfuscated payload decoding or base64 command execution."
  },

  // 5. Pipe to Shell Downloads (RESTRICTED / REJECTED)
  {
    id: "SEC-PIPE-001",
    severity: "HIGH",
    category: "pipe_to_shell",
    pattern: /\b(curl|wget)\s+[^|\r\n]+\|\s*(bash|sh|zsh|python|perl|pwsh|powershell)\b/i,
    description: "Detects piping remote internet downloads directly into shell interpreters."
  },

  // 6. Suspicious Network Calls (RESTRICTED)
  {
    id: "SEC-NET-001",
    severity: "MEDIUM",
    category: "suspicious_network",
    pattern: /\b(nc\s+-l|ncat\s+-e|ngrok|reverse[_-]?shell|localxpose)\b/i,
    description: "Detects reverse shells, tunneling agents, or arbitrary listening ports."
  }
];

export class TrustScanner {
  constructor(options = {}) {
    this.customRules = options.rules || [];
    this.rules = [...SECURITY_RULES, ...this.customRules];
  }

  /**
   * Scan skill text or files.
   * NEVER executes arbitrary scripts during scan.
   */
  scan(skillContent, metadata = {}) {
    const content = String(skillContent || "");
    const contentHash = computeStringHash(content);
    const findings = [];

    for (const rule of this.rules) {
      if (rule.pattern.test(content)) {
        findings.push({
          ruleId: rule.id,
          severity: rule.severity,
          category: rule.category,
          description: rule.description
        });
      }
    }

    let trustStatus = TRUST_STATES.APPROVED;

    const hasCritical = findings.some(f => f.severity === "CRITICAL");
    const hasHigh = findings.some(f => f.severity === "HIGH");
    const hasMedium = findings.some(f => f.severity === "MEDIUM");

    if (hasCritical) {
      trustStatus = TRUST_STATES.REJECTED;
    } else if (hasHigh) {
      trustStatus = TRUST_STATES.QUARANTINED;
    } else if (hasMedium) {
      trustStatus = TRUST_STATES.RESTRICTED;
    }

    return {
      skillName: metadata.name || "unnamed-skill",
      sourceRepository: metadata.repository || "unknown",
      commitSha: metadata.commitSha || "unpinned",
      contentHash,
      scannedAt: new Date().toISOString(),
      trustStatus,
      isApproved: trustStatus === TRUST_STATES.APPROVED,
      isBlocked: trustStatus === TRUST_STATES.REJECTED || trustStatus === TRUST_STATES.QUARANTINED,
      requiresHumanApproval: trustStatus === TRUST_STATES.RESTRICTED,
      findings,
      disclaimer: "Static security scanning detects common threat patterns but cannot guarantee absolute safety. All third-party skills must be audited before execution."
    };
  }
}
