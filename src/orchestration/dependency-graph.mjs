/**
 * Dependency Graph for work packages: builds DAG, computes topological order,
 * calculates parallel execution waves, and creates sequential execution schedules.
 */

export class DependencyGraph {
  constructor(workPackages = []) {
    this.workPackages = workPackages;
    this.packageMap = new Map(workPackages.map(wp => [wp.id, wp]));
  }

  buildAdjacency() {
    const adj = new Map();
    const inDegree = new Map();

    for (const wp of this.workPackages) {
      if (!adj.has(wp.id)) adj.set(wp.id, []);
      if (!inDegree.has(wp.id)) inDegree.set(wp.id, 0);
    }

    for (const wp of this.workPackages) {
      for (const depId of (wp.dependencies || [])) {
        if (!this.packageMap.has(depId)) {
          throw new Error(`Work package '${wp.id}' references non-existent dependency '${depId}'`);
        }
        adj.get(depId).push(wp.id);
        inDegree.set(wp.id, inDegree.get(wp.id) + 1);
      }
    }

    return { adj, inDegree };
  }

  /**
   * Compute topological sort and execution waves (Kahn's algorithm).
   */
  schedule() {
    const { adj, inDegree } = this.buildAdjacency();
    const queue = [];
    const waves = [];
    const inDegreeCopy = new Map(inDegree);

    // Initial wave: in-degree 0
    for (const [id, deg] of inDegreeCopy.entries()) {
      if (deg === 0) queue.push(id);
    }

    let processedCount = 0;
    const executionOrder = [];

    while (queue.length > 0) {
      const currentWave = [...queue];
      waves.push(currentWave.map(id => this.packageMap.get(id)));
      queue.length = 0;

      for (const id of currentWave) {
        executionOrder.push(this.packageMap.get(id));
        processedCount++;

        for (const neighbor of adj.get(id) || []) {
          const nextDeg = inDegreeCopy.get(neighbor) - 1;
          inDegreeCopy.set(neighbor, nextDeg);
          if (nextDeg === 0) {
            queue.push(neighbor);
          }
        }
      }
    }

    if (processedCount !== this.workPackages.length) {
      throw new Error("Circular dependency detected in work package graph.");
    }

    return {
      executionOrder,
      parallelWaves: waves,
      isParallelizable: waves.some(w => w.length > 1)
    };
  }

  /**
   * Plan execution based on requested mode and runtime capability.
   */
  createExecutionPlan(mode, runtimeCapability = { supported: false }) {
    const scheduled = this.schedule();

    // Mode A: Single isolated package
    if (this.workPackages.length <= 1 || mode === "single-agent") {
      return {
        selectedMode: "Mode A: Single-agent",
        actualExecution: "sequential-single-agent",
        parallelEnabled: false,
        waves: [this.workPackages],
        linearOrder: this.workPackages,
        explanation: "Task is executed sequentially by a single agent one step at a time.",
        integrationStep: {
          id: "INT-VALIDATION",
          objective: "Run automated unit and integration tests to verify changes",
          dependencies: this.workPackages.map(wp => wp.id)
        }
      };
    }

    // Mode B: Multi-agent (only if runtime truly supports sub-agents)
    if (mode === "multi-agent" && runtimeCapability.supported) {
      return {
        selectedMode: "Mode B: Multi-agent",
        actualExecution: "multi-agent-parallel",
        parallelEnabled: true,
        waves: scheduled.parallelWaves,
        linearOrder: scheduled.executionOrder,
        explanation: "Independent work packages are dispatched to parallel specialist agents across dependency waves.",
        integrationStep: {
          id: "INT-MERGE-AND-VALIDATION",
          objective: "Integrator agent reconciles specialist outputs, merges changes, and validates end-to-end suite",
          dependencies: this.workPackages.map(wp => wp.id)
        }
      };
    }

    // Mode C: Sequential fallback (when task is complex/multi-agent requested, but parallel runtime is unavailable)
    return {
      selectedMode: "Mode C: Sequential fallback",
      actualExecution: "sequential-step-by-step",
      parallelEnabled: false,
      waves: scheduled.executionOrder.map(wp => [wp]),
      linearOrder: scheduled.executionOrder,
      explanation: "Task requires multi-step decomposition, but parallel sub-agents are unavailable in this environment. Falling back honestly to sequential specialist work packages.",
      downgradeNotice: runtimeCapability.reason || "Sub-agent concurrency is not active.",
      integrationStep: {
        id: "INT-VALIDATION",
        objective: "Perform final integration verification and regression testing",
        dependencies: this.workPackages.map(wp => wp.id)
      }
    };
  }
}
