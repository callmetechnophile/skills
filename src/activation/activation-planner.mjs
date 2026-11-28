/**
 * Functional Skill Activation Planner.
 * Governs Discovery -> Selection -> Installation -> Activation -> Execution stages.
 */

import { LocalRegistry } from "../discovery/local-registry.mjs";
import { InstalledRegistry } from "../discovery/installed-registry.mjs";
import { SkillsDirectoryRegistry } from "../discovery/skillsdirectory-registry.mjs";
import { DirectoryResolver } from "../discovery/directory-resolver.mjs";
import { HybridRanker } from "../ranking/hybrid-ranker.mjs";
import { TrustScanner } from "../security/trust-scanner.mjs";
import { TrustPolicy } from "../security/trust-policy.mjs";
import { computeStringHash } from "../security/integrity.mjs";
import { DependencyGraph } from "../orchestration/dependency-graph.mjs";
import { getRuntimeAdapter } from "../orchestration/runtime-adapters/adapters.mjs";
import { ActivationReport } from "./activation-report.mjs";

export class ActivationPlanner {
  constructor(options = {}) {
    this.rootDir = options.rootDir || ".";
    this.localRegistry = options.localRegistry || new LocalRegistry({ rootDir: this.rootDir });
    this.installedRegistry = options.installedRegistry || new InstalledRegistry({ rootDir: this.rootDir });
    this.remoteRegistry = options.remoteRegistry || new SkillsDirectoryRegistry({ rootDir: this.rootDir, noNetwork: options.noNetwork });
    this.directoryResolver = options.directoryResolver || new DirectoryResolver(options);
    this.ranker = options.ranker || new HybridRanker(options);
    this.scanner = options.scanner || new TrustScanner();
    this.policy = options.policy || new TrustPolicy();
    this.agentConfigs = options.agentConfigs || {};
  }

  async planActivation(triageRecord, options = {}) {
    const agent = options.agent || "antigravity";
    const runtimeAdapter = getRuntimeAdapter(agent, this.agentConfigs);
    const subagentSupport = await runtimeAdapter.checkSubagentSupport();

    const workPackages = triageRecord.work_packages || [];
    const activations = [];
    const failures = [];

    // Pre-load local and installed skills
    const localSkills = await this.localRegistry.getAllSkills();
    const installedSkills = await this.installedRegistry.search("");

    for (const wp of workPackages) {
      const query = wp.skill_query || wp.objective || triageRecord.outcome;

      // Stage 1: Discovery across registries
      let candidates = [
        ...localSkills,
        ...installedSkills
      ];

      // If local candidates are scarce, query remote registry index
      if (!options.localOnly) {
        const remoteCandidates = await this.remoteRegistry.search(query, { limit: 10 });
        candidates.push(...remoteCandidates);
      }

      // Stage 2: Selection via Hybrid Ranker
      const rankingResult = await this.ranker.rank(candidates, query, {
        targetAgent: agent,
        strictCompatibility: options.strictCompatibility ?? true
      });
      const topMatch = rankingResult.selected;

      if (!topMatch) {
        failures.push({
          workPackageId: wp.id,
          skillName: query,
          status: "FAILED",
          reason: "NO_SUITABLE_SKILL",
          message: `No compatible or approved skill discovered for query '${query}'.`
        });
        continue;
      }

      const skill = topMatch.skill;
      const isLocal = skill.source === "local" || skill.registry === "local";
      const isInstalled = skill.registry === "installed";

      if (isLocal || isInstalled) {
        // Stage 3 & 4: Retrieve content, security scan, and activate
        let rawContent = "";
        try {
          rawContent = isLocal
            ? await this.localRegistry.retrieveSkillContent(skill.id)
            : await this.installedRegistry.retrieveSkillContent(skill.skillId || skill.id);
        } catch (err) {
          failures.push({
            workPackageId: wp.id,
            skillName: skill.name,
            status: "FAILED",
            reason: "CONTENT_RETRIEVAL_ERROR",
            message: `Could not read skill content: ${err.message}`
          });
          continue;
        }

        const scanResult = this.scanner.scan(rawContent, {
          name: skill.name,
          repository: skill.repository,
          commitSha: skill.commitSha
        });

        const policyEval = this.policy.evaluateActivation(scanResult, options);
        if (!policyEval.canActivate) {
          failures.push({
            workPackageId: wp.id,
            skillName: skill.name,
            status: "BLOCKED",
            reason: policyEval.reason,
            message: policyEval.message,
            scanResult
          });
          continue;
        }

        // Format instruction payload and verify integrity
        const instructionPayload = runtimeAdapter.formatInstructionPayload(wp, rawContent, skill);
        const actualHash = computeStringHash(rawContent);

        activations.push({
          workPackageId: wp.id,
          skillName: skill.name,
          status: "ACTIVATED",
          sourceType: isLocal ? "local-repository" : "locally-installed",
          sourceLocation: skill.path || skill.installationPath,
          trustStatus: scanResult.trustStatus,
          contentHash: actualHash,
          verified: actualHash === scanResult.contentHash,
          instructionPayload
        });
      } else {
        // Stage 3: Remote skill requiring installation
        // Guard: skill.repository must be a non-empty string before directory resolution
        if (!skill.repository || typeof skill.repository !== "string") {
          failures.push({
            workPackageId: wp.id,
            skillName: skill.name || skill.id || "unknown",
            status: "FAILED",
            reason: "MISSING_REPOSITORY",
            message: `Remote skill candidate has no 'repository' field. Cannot resolve install directory.`
          });
          continue;
        }
        // Check exact directory resolution before recommending install
        const resolution = await this.directoryResolver.resolveSkillDirectory(
          skill.repository,
          skill.skill_directory || skill.slug || skill.name,
          options
        );

        if (!resolution.success) {
          failures.push({
            workPackageId: wp.id,
            skillName: skill.name,
            status: "FAILED",
            reason: resolution.error,
            message: `Cannot activate remote skill: directory resolution failed: ${resolution.message}`
          });
          continue;
        }

        // Remote skills cannot be marked as ACTIVATED until actually installed and scanned
        activations.push({
          workPackageId: wp.id,
          skillName: skill.name,
          status: "REQUIRES_INSTALLATION",
          sourceType: "remote-registry",
          sourceLocation: skill.repository,
          exactDirectory: resolution.exactDirectory,
          installCommand: resolution.installCommand,
          trustStatus: "UNSCANNED",
          verified: false,
          note: "Skill must be installed and verified before it can be activated into the live session."
        });
      }
    }

    // Build Execution Plan via DependencyGraph
    const depGraph = new DependencyGraph(workPackages);
    const executionPlan = depGraph.createExecutionPlan(triageRecord.workflow, subagentSupport);

    return new ActivationReport({
      taskId: triageRecord.outcome,
      agent,
      workflowMode: triageRecord.workflow,
      activations,
      failures,
      executionPlan
    });
  }
}
