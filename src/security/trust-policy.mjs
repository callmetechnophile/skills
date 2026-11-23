/**
 * Policy-based trust approval engine.
 */

import { TRUST_STATES } from "./trust-scanner.mjs";

export class TrustPolicy {
  constructor(options = {}) {
    this.allowRestrictedWithFlag = options.allowRestrictedWithFlag ?? true;
    this.allowedSources = options.allowedSources || ["local", "callmetechnophile/skills"];
  }

  evaluateActivation(scanResult, userOptions = {}) {
    if (!scanResult) {
      return {
        canActivate: false,
        reason: "MISSING_SCAN",
        message: "Skill has not undergone static security scanning."
      };
    }

    if (scanResult.trustStatus === TRUST_STATES.REJECTED) {
      return {
        canActivate: false,
        reason: "SECURITY_REJECTED",
        message: `Activation blocked: Skill failed security scan with critical violations: ${scanResult.findings.map(f => f.description).join("; ")}`
      };
    }

    if (scanResult.trustStatus === TRUST_STATES.QUARANTINED) {
      return {
        canActivate: false,
        reason: "SECURITY_QUARANTINED",
        message: "Activation blocked: Skill is quarantined due to high-risk obfuscation or execution patterns."
      };
    }

    if (scanResult.trustStatus === TRUST_STATES.RESTRICTED) {
      if (userOptions.approveRestricted === true) {
        return {
          canActivate: true,
          reason: "APPROVED_WITH_OVERRIDE",
          message: "Skill is restricted but human approval was explicitly granted."
        };
      }
      return {
        canActivate: false,
        reason: "REQUIRES_APPROVAL",
        message: "Activation requires explicit human approval for restricted operations (pass --approve-restricted)."
      };
    }

    return {
      canActivate: true,
      reason: "APPROVED",
      message: "Skill passed security scan and is approved for activation."
    };
  }
}
