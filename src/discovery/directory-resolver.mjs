/**
 * Resolves exact upstream skill directory by inspecting the repository tree
 * and locating valid SKILL.md files. Rejects ambiguous matches and never fabricates names.
 */

export class DirectoryResolver {
  constructor(options = {}) {
    this.fetchFn = options.fetchFn || globalThis.fetch;
    this.treeCache = new Map();
  }

  /**
   * Fetch repository tree using GitHub Git Trees API or tree provider.
   */
  async fetchRepositoryTree(owner, repo, branch = "main", options = {}) {
    const cacheKey = `${owner}/${repo}@${branch}`;
    if (this.treeCache.has(cacheKey) && !options.noCache) {
      return this.treeCache.get(cacheKey);
    }

    const url = `https://api.github.com/repos/${owner}/${repo}/git/trees/${branch}?recursive=1`;
    const headers = {
      accept: "application/vnd.github.v3+json",
      "user-agent": "automatic-skill-router/0.2.0"
    };
    if (options.githubToken || process.env.GITHUB_TOKEN) {
      headers.authorization = `Bearer ${options.githubToken || process.env.GITHUB_TOKEN}`;
    }

    let res;
    try {
      res = await this.fetchFn(url, {
        headers,
        signal: AbortSignal.timeout(options.timeoutMs || 10000)
      });
    } catch (err) {
      throw new Error(`Failed to connect to repository host for ${owner}/${repo}: ${err.message}`);
    }

    if (!res.ok) {
      // If branch main failed, try master
      if (branch === "main" && (res.status === 404 || res.status === 422)) {
        return this.fetchRepositoryTree(owner, repo, "master", options);
      }
      throw new Error(`Repository tree lookup failed for ${owner}/${repo}: HTTP ${res.status}`);
    }

    const data = await res.json();
    if (!Array.isArray(data.tree)) {
      throw new Error(`Malformed repository tree response for ${owner}/${repo}`);
    }

    this.treeCache.set(cacheKey, data.tree);
    return data.tree;
  }

  /**
   * Locate all directories containing a valid SKILL.md file.
   */
  discoverSkillDirectoriesFromTree(tree) {
    const skillDirs = [];
    for (const item of tree) {
      if (item.type === "blob" && /(^|\/)SKILL\.md$/i.test(item.path)) {
        const dirPath = item.path.replace(/\/?SKILL\.md$/i, "");
        skillDirs.push({
          skillMdPath: item.path,
          directory: dirPath || ".",
          basename: dirPath ? dirPath.split("/").pop() : "root"
        });
      }
    }
    return skillDirs;
  }

  /**
   * Resolve exact upstream skill directory.
   */
  async resolveSkillDirectory(repository, targetSkill, options = {}) {
    const parts = repository.split("/");
    if (parts.length !== 2 || !parts[0] || !parts[1]) {
      return {
        success: false,
        error: "INVALID_REPOSITORY",
        message: `Invalid repository format '${repository}'. Expected owner/name.`
      };
    }

    const [owner, repo] = parts;
    let tree;

    // Support injecting pre-resolved tree (for tests or local repos)
    if (options.treeFixture) {
      tree = options.treeFixture;
    } else {
      try {
        tree = await this.fetchRepositoryTree(owner, repo, options.branch || "main", options);
      } catch (err) {
        return {
          success: false,
          error: "TREE_FETCH_ERROR",
          message: err.message
        };
      }
    }

    const discovered = this.discoverSkillDirectoriesFromTree(tree);

    if (discovered.length === 0) {
      return {
        success: false,
        error: "NO_SKILLS_FOUND",
        message: `No SKILL.md files found in repository ${repository}.`
      };
    }

    const query = String(targetSkill || "").trim().toLowerCase();

    // 1. Exact match on directory path (e.g. "skills/core/sort-the-issue")
    const exactPathMatch = discovered.find(d => d.directory.toLowerCase() === query);
    if (exactPathMatch) {
      return this._buildSuccessResult(repository, exactPathMatch, options);
    }

    // 2. Exact match on folder basename (e.g. "sort-the-issue")
    const exactBasenameMatches = discovered.filter(d => d.basename.toLowerCase() === query);
    if (exactBasenameMatches.length === 1) {
      return this._buildSuccessResult(repository, exactBasenameMatches[0], options);
    }
    if (exactBasenameMatches.length > 1) {
      return {
        success: false,
        error: "AMBIGUOUS_MATCH",
        message: `Target skill '${targetSkill}' is ambiguous; found ${exactBasenameMatches.length} matching directories.`,
        candidates: exactBasenameMatches.map(m => m.directory)
      };
    }

    // 3. Normalized slug matching
    const slugQuery = query.replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
    const slugMatches = discovered.filter(d =>
      d.basename.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") === slugQuery
    );

    if (slugMatches.length === 1) {
      return this._buildSuccessResult(repository, slugMatches[0], options);
    }
    if (slugMatches.length > 1) {
      return {
        success: false,
        error: "AMBIGUOUS_MATCH",
        message: `Target skill '${targetSkill}' is ambiguous; multiple candidates match normalized slug.`,
        candidates: slugMatches.map(m => m.directory)
      };
    }

    return {
      success: false,
      error: "SKILL_NOT_FOUND",
      message: `Skill '${targetSkill}' not found in repository ${repository}.`,
      availableSkills: discovered.map(d => d.directory)
    };
  }

  _buildSuccessResult(repository, match, options) {
    const agent = options.agent || "antigravity";
    const installArgs = ["-y", "skills", "add", repository, "--skill", match.directory, "--agent", agent, "-y"];
    const installCommand = `npx ${installArgs.join(" ")}`;

    return {
      success: true,
      repository,
      exactDirectory: match.directory,
      skillMdPath: match.skillMdPath,
      basename: match.basename,
      installCommand,
      installArgs
    };
  }
}
