/**
 * Base abstract class for all skill registries.
 */
export class SkillRegistry {
  constructor(name) {
    if (this.constructor === SkillRegistry) {
      throw new Error("SkillRegistry is abstract and cannot be instantiated directly.");
    }
    this.name = name;
  }

  /**
   * Search registry for matching skills.
   * @param {string} query
   * @param {object} options
   * @returns {Promise<Array<object>>}
   */
  async search(query, options = {}) {
    throw new Error("search() must be implemented by subclass.");
  }

  /**
   * Get metadata for a specific skill by identifier.
   */
  async getMetadata(id) {
    throw new Error("getMetadata() must be implemented by subclass.");
  }

  /**
   * Get available versions for a skill.
   */
  async getVersions(id) {
    throw new Error("getVersions() must be implemented by subclass.");
  }

  /**
   * Resolve source repository / location details.
   */
  async resolveSource(id) {
    throw new Error("resolveSource() must be implemented by subclass.");
  }

  /**
   * Retrieve skill instructions / content when authorized.
   */
  async retrieveSkillContent(id) {
    throw new Error("retrieveSkillContent() must be implemented by subclass.");
  }

  /**
   * Verify skill origin and provenance.
   */
  async verifyProvenance(id) {
    throw new Error("verifyProvenance() must be implemented by subclass.");
  }
}
