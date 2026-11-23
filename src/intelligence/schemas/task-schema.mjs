/**
 * JSON Schema and validator for Sort-The-Issue Triage output.
 */

export const TRIAGE_SCHEMA = {
  $id: "https://skills.antigravity.dev/schemas/triage-v0.2.json",
  type: "object",
  required: [
    "outcome",
    "class",
    "requirements",
    "constraints",
    "non_goals",
    "acceptance_criteria",
    "affected_areas",
    "dependencies",
    "risks",
    "unknowns",
    "complexity",
    "workflow",
    "work_packages"
  ],
  properties: {
    outcome: { type: "string" },
    class: { type: "array", items: { type: "string" } },
    requirements: { type: "array", items: { type: "string" } },
    constraints: { type: "array", items: { type: "string" } },
    non_goals: { type: "array", items: { type: "string" } },
    acceptance_criteria: { type: "array", items: { type: "string" } },
    affected_areas: { type: "array", items: { type: "string" } },
    dependencies: { type: "array", items: { type: "string" } },
    risks: { type: "array", items: { type: "string" } },
    unknowns: { type: "array", items: { type: "string" } },
    complexity: {
      type: "object",
      required: ["scope_breadth", "technical_coupling", "uncertainty", "risk", "parallelism", "total_score", "band"],
      properties: {
        scope_breadth: { type: "integer", minimum: 0, maximum: 2 },
        technical_coupling: { type: "integer", minimum: 0, maximum: 2 },
        uncertainty: { type: "integer", minimum: 0, maximum: 2 },
        risk: { type: "integer", minimum: 0, maximum: 2 },
        parallelism: { type: "integer", minimum: 0, maximum: 2 },
        total_score: { type: "integer", minimum: 0, maximum: 10 },
        band: { type: "string", enum: ["LOW", "MEDIUM", "HIGH"] }
      }
    },
    workflow: { type: "string", enum: ["single-agent", "multi-agent", "single-agent-with-subagents"] },
    work_packages: {
      type: "array",
      items: {
        type: "object",
        required: ["id", "title", "objective", "component", "dependencies", "independent", "skill_query"],
        properties: {
          id: { type: "string" },
          title: { type: "string" },
          objective: { type: "string" },
          component: { type: "string" },
          dependencies: { type: "array", items: { type: "string" } },
          independent: { type: "boolean" },
          skill_query: { type: "string" }
        }
      }
    }
  }
};

export function validateTriageSchema(data) {
  const errors = [];
  if (!data || typeof data !== "object") {
    return { valid: false, errors: ["Data must be a non-null object"] };
  }

  for (const field of TRIAGE_SCHEMA.required) {
    if (data[field] === undefined) {
      errors.push(`Missing required field: '${field}'`);
    }
  }

  if (typeof data.outcome !== "string" || !data.outcome.trim()) {
    errors.push("Field 'outcome' must be a non-empty string");
  }

  if (!Array.isArray(data.class) || data.class.length === 0) {
    errors.push("Field 'class' must be a non-empty array of strings");
  }

  const arrayFields = ["requirements", "constraints", "non_goals", "acceptance_criteria", "affected_areas", "dependencies", "risks", "unknowns"];
  for (const af of arrayFields) {
    if (!Array.isArray(data[af])) {
      errors.push(`Field '${af}' must be an array`);
    }
  }

  if (data.complexity) {
    const compFields = ["scope_breadth", "technical_coupling", "uncertainty", "risk", "parallelism", "total_score"];
    for (const cf of compFields) {
      const val = data.complexity[cf];
      if (typeof val !== "number" || !Number.isInteger(val)) {
        errors.push(`Field 'complexity.${cf}' must be an integer`);
      }
    }
    if (!["LOW", "MEDIUM", "HIGH"].includes(data.complexity.band)) {
      errors.push(`Field 'complexity.band' must be LOW, MEDIUM, or HIGH`);
    }
  }

  if (!["single-agent", "multi-agent", "single-agent-with-subagents"].includes(data.workflow)) {
    errors.push(`Field 'workflow' must be one of: single-agent, multi-agent, single-agent-with-subagents`);
  }

  if (!Array.isArray(data.work_packages) || data.work_packages.length === 0) {
    errors.push("Field 'work_packages' must be a non-empty array");
  } else {
    for (const [idx, wp] of data.work_packages.entries()) {
      if (!wp.id || typeof wp.id !== "string") errors.push(`work_packages[${idx}].id is missing or not string`);
      if (!wp.title || typeof wp.title !== "string") errors.push(`work_packages[${idx}].title is missing or not string`);
      if (!wp.objective || typeof wp.objective !== "string") errors.push(`work_packages[${idx}].objective is missing or not string`);
      if (!Array.isArray(wp.dependencies)) errors.push(`work_packages[${idx}].dependencies must be an array`);
      if (typeof wp.independent !== "boolean") errors.push(`work_packages[${idx}].independent must be boolean`);
    }
  }

  return {
    valid: errors.length === 0,
    errors
  };
}
