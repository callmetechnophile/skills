use clap::{Parser, Subcommand};
use router_core::{RouteOptions, SkillRouter};
use serde::Serialize;
use skill_discovery::DirectoryResolver;
use skill_lifecycle::{InstallOptions, InstalledTracker};
use std::path::PathBuf;
use std::process;

#[derive(Parser, Debug)]
#[command(
    name = "skill-router",
    author,
    version,
    about = "Automatic Agent Skill Router V0.3.2"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Task description when using default route command
    #[arg(trailing_var_arg = true)]
    task_words: Vec<String>,

    /// Workspace root directory
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,

    /// Execution workflow mode (auto, single, multi)
    #[arg(long, global = true, default_value = "auto")]
    workflow: String,

    /// Target agent runtime environment
    #[arg(long, global = true, default_value = "antigravity")]
    agent: String,

    /// Disable remote network requests
    #[arg(long, global = true)]
    no_network: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Execute mandatory sort-the-issue triage
    Sort {
        /// Task description to analyze
        #[arg(required = true)]
        task: Vec<String>,
        #[arg(long, default_value = "auto")]
        workflow: String,
    },
    /// Route task through triage, ranking, security, and activation planning
    Route {
        /// Task description to route
        #[arg(required = true)]
        task: Vec<String>,
        #[arg(long, default_value = "auto")]
        workflow: String,
        #[arg(long, default_value = "antigravity")]
        agent: String,
        #[arg(long)]
        no_network: bool,
    },
    /// Find and rank skills across local and remote registries
    Find {
        /// Search query
        #[arg(required = true)]
        query: Vec<String>,
        #[arg(long, default_value = "antigravity")]
        agent: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Synchronize skills directory catalog
    #[command(alias = "sync:catalog")]
    SyncCatalog {
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(long)]
        reset: bool,
    },
    /// Install skill from upstream repository (dry-run by default unless --execute is passed)
    #[command(alias = "install:skill")]
    InstallSkill {
        #[arg(long)]
        repo: String,
        #[arg(long)]
        skill: String,
        #[arg(long, default_value = "antigravity")]
        agent: String,
        /// Execute installation and write files/ledger
        #[arg(long)]
        execute: bool,
        /// Preview installation plan without modifying files (default)
        #[arg(long)]
        dry_run: bool,
        /// Confirm installation of RESTRICTED skills or overwrite of modified skills
        #[arg(long)]
        force: bool,
    },
    /// List installed skills from persistent ledger
    #[command(alias = "list:skills")]
    ListSkills {},
    /// Verify cryptographic integrity of installed skills
    #[command(alias = "verify:skills")]
    VerifySkills {},
    /// Check upstream repositories for available updates (read-only; does not install)
    #[command(visible_alias = "update-skills", alias = "update:skills")]
    CheckUpdates {},
}

fn print_json<T: Serialize>(value: &T) -> Result<(), String> {
    let serialized = serde_json::to_string_pretty(value)
        .map_err(|e| format!("Failed to serialize JSON output: {}", e))?;
    println!("{}", serialized);
    Ok(())
}

#[tokio::main]
async fn main() {
    let raw_args: Vec<String> = std::env::args().collect();
    let used_deprecated_update_alias = raw_args
        .iter()
        .any(|a| a == "update:skills" || a == "update-skills");

    // Normalize legacy colon aliases before clap parsing
    let normalized_args: Vec<String> = raw_args
        .into_iter()
        .map(|a| match a.as_str() {
            "sync:catalog" => "sync-catalog".to_string(),
            "install:skill" => "install-skill".to_string(),
            "list:skills" => "list-skills".to_string(),
            "verify:skills" => "verify-skills".to_string(),
            "update:skills" => "check-updates".to_string(),
            _ => a,
        })
        .collect();

    let cli = match Cli::try_parse_from(&normalized_args) {
        Ok(c) => c,
        Err(e) => e.exit(),
    };

    if used_deprecated_update_alias {
        eprintln!(
            "Warning: 'update-skills' / 'update:skills' is a deprecated alias for 'check-updates' (check-only; does not install updates)."
        );
    }

    let root_dir = cli.root.clone();
    let router = SkillRouter::new(&root_dir, cli.no_network);

    let result: Result<(), String> = match cli.command {
        Some(Commands::Sort { task, workflow }) => {
            let task_str = task.join(" ");
            match router.sort(&task_str, Some(&workflow)) {
                Ok(triage) => {
                    if cli.json {
                        print_json(&triage)
                    } else {
                        println!("=== SORT-THE-ISSUE TRIAGE ===");
                        println!("Outcome: {}", triage.outcome);
                        println!("Class: {}", triage.class.join(", "));
                        println!(
                            "Complexity: {:?} (Score: {}/10)",
                            triage.complexity.band, triage.complexity.total_score
                        );
                        println!("Recommended Workflow: {}", triage.workflow);
                        println!("Work Packages ({}):", triage.work_packages.len());
                        for wp in &triage.work_packages {
                            println!(
                                "  - [{}] {} (Deps: {})",
                                wp.id,
                                wp.title,
                                if wp.dependencies.is_empty() {
                                    "none".to_string()
                                } else {
                                    wp.dependencies.join(", ")
                                }
                            );
                        }
                        Ok(())
                    }
                }
                Err(e) => Err(e.to_string()),
            }
        }
        Some(Commands::Route {
            task,
            workflow,
            agent,
            no_network,
        }) => {
            let task_str = task.join(" ");
            let options = RouteOptions {
                workflow: Some(workflow),
                agent,
                no_network: no_network || cli.no_network,
                local_only: false,
                strict_compatibility: true,
            };
            match router.route(&task_str, &options).await {
                Ok(route_res) => {
                    if cli.json {
                        print_json(&route_res)
                    } else {
                        println!("=== AUTOMATIC SKILL ROUTER V0.3.2 ===");
                        println!("Task: {}", route_res.triage.outcome);
                        println!("Mandatory First Skill: {}", route_res.mandatory_first_skill);
                        println!(
                            "Complexity: {:?} (Score: {}/10)",
                            route_res.triage.complexity.band,
                            route_res.triage.complexity.total_score
                        );
                        println!("Workflow: {}", route_res.routing.workflow);
                        if let Some(plan) = &route_res.execution_plan {
                            println!("Execution Mode: {}", plan.selected_mode);
                            if let Some(notice) = &plan.downgrade_notice {
                                println!("Notice: {}", notice);
                            }
                        }
                        println!("\nActivations:");
                        for act in &route_res.activation.activations {
                            println!(
                                "  - [{}] {} (Package: {}, Trust: {})",
                                act.status, act.skill_name, act.work_package_id, act.trust_status
                            );
                        }
                        if !route_res.activation.failures.is_empty() {
                            println!("\nBlocked / Unresolved:");
                            for f in &route_res.activation.failures {
                                println!(
                                    "  - [{}] {}: {}",
                                    f.status,
                                    f.skill_name,
                                    f.message.as_deref().unwrap_or("")
                                );
                            }
                        }
                        Ok(())
                    }
                }
                Err(e) => Err(e.to_string()),
            }
        }
        Some(Commands::Find {
            query,
            agent,
            limit,
        }) => {
            let q_str = query.join(" ");
            let ranked = router.find(&q_str, &agent, limit).await;
            if cli.json {
                print_json(&ranked)
            } else {
                println!("=== Discovered Skills for '{}' ===", q_str);
                for r in ranked {
                    println!(
                        "- {} (Score: {}, Security: {})",
                        r.skill.name, r.relevance_score, r.security_result
                    );
                    println!("  {}", r.skill.description);
                }
                Ok(())
            }
        }
        Some(Commands::SyncCatalog { limit, reset }) => {
            match router.remote_registry.sync_catalog(limit, reset).await {
                Ok(result) => {
                    if cli.json {
                        print_json(&result)
                    } else {
                        println!(
                            "Registry sync complete: {} skills indexed across {} page(s).",
                            result.total_indexed, result.synced_pages
                        );
                        if !result.complete {
                            println!(
                                "Partial sync saved at next offset {}. Re-run sync-catalog to resume.",
                                result.next_offset
                            );
                        }
                        Ok(())
                    }
                }
                Err(e) => Err(format!("Sync error: {}", e)),
            }
        }
        Some(Commands::InstallSkill {
            repo,
            skill,
            agent,
            execute,
            dry_run,
            force,
        }) => {
            if execute && dry_run {
                Err("Cannot specify both --execute and --dry-run.".to_string())
            } else {
                let resolver =
                    DirectoryResolver::with_config("https://api.github.com", 15, cli.no_network);
                let resolution = resolver
                    .resolve_skill_directory(&repo, &skill, None, None)
                    .await;
                if !resolution.success {
                    if cli.json {
                        let _ = print_json(&resolution);
                    }
                    Err(resolution
                        .message
                        .unwrap_or_else(|| "Directory resolution failed.".to_string()))
                } else {
                    let exact_dir = resolution.exact_directory.unwrap_or_default();
                    let install_cmd = resolution.install_command.unwrap_or_default();

                    if !execute || dry_run {
                        if cli.json {
                            let preview = serde_json::json!({
                                "status": "DRY_RUN",
                                "repository": repo,
                                "targetSkill": skill,
                                "exactDirectory": exact_dir,
                                "installCommand": install_cmd,
                                "targetAgent": agent,
                                "license": "UNSPECIFIED",
                                "action": "Re-run with --execute to perform installation."
                            });
                            print_json(&preview)
                        } else {
                            println!("=== Skill Installation Plan (Dry Run) ===");
                            println!("Repository: {}", repo);
                            println!("Requested Skill: {}", skill);
                            println!("Resolved Directory: {}", exact_dir);
                            println!("Target Agent: {}", agent);
                            println!("Install Command: {}", install_cmd);
                            println!("License: UNSPECIFIED (verify upstream before executing)");
                            println!("\nDry run only. Re-run with --execute after reviewing the upstream source.");
                            Ok(())
                        }
                    } else {
                        let ledger_path = root_dir.join(".installed-skills.json");
                        let tracker = InstalledTracker::with_config(
                            &root_dir,
                            &ledger_path,
                            "https://api.github.com",
                            "https://raw.githubusercontent.com",
                            15,
                            cli.no_network,
                        );
                        let options = InstallOptions {
                            repo,
                            skill,
                            agent,
                            force,
                            ..Default::default()
                        };

                        match tracker.install_skill(&options).await {
                            Ok(installed) => {
                                if cli.json {
                                    print_json(&installed)
                                } else {
                                    println!(
                                        "\nSkill '{}' successfully installed and recorded in .installed-skills.json.",
                                        installed.name
                                    );
                                    println!("  Path: {}", installed.installation_path);
                                    println!("  Commit SHA: {}", installed.commit_sha);
                                    println!("  Directory SHA-256: {}", installed.content_hash);
                                    println!("  Trust Status: {}", installed.trust_status);
                                    Ok(())
                                }
                            }
                            Err(e) => Err(format!("Error during installation: {}", e)),
                        }
                    }
                }
            }
        }
        Some(Commands::ListSkills {}) => {
            let tracker = InstalledTracker::new(&root_dir);
            match tracker.list_skills() {
                Ok(skills) => {
                    if cli.json {
                        print_json(&skills)
                    } else if skills.is_empty() {
                        println!("No skills currently recorded in .installed-skills.json.");
                        Ok(())
                    } else {
                        println!("=== Installed Skills ({}) ===", skills.len());
                        for s in skills {
                            println!(
                                "- {} (Source: {}, Version: {}, Commit: {})",
                                s.name, s.source, s.version, s.commit_sha
                            );
                        }
                        Ok(())
                    }
                }
                Err(e) => Err(format!("Error loading ledger: {}", e)),
            }
        }
        Some(Commands::VerifySkills {}) => {
            let tracker = InstalledTracker::new(&root_dir);
            match tracker.verify_skills() {
                Ok(report) => {
                    let has_failures = report.modified > 0 || report.missing > 0;
                    if cli.json {
                        if let Err(e) = print_json(&report) {
                            eprintln!("Error: {}", e);
                            process::exit(1);
                        }
                    } else {
                        println!("=== Skill Integrity Report ===");
                        println!(
                            "Total: {}, Intact: {}, Modified: {}, Missing: {}",
                            report.total, report.intact, report.modified, report.missing
                        );
                        for d in &report.details {
                            println!(
                                "- [{}]: {} ({})",
                                d.status,
                                d.name,
                                if d.verified { "Passed" } else { "Failed" }
                            );
                        }
                    }
                    if has_failures {
                        process::exit(1);
                    }
                    Ok(())
                }
                Err(e) => Err(format!("Error verifying skills: {}", e)),
            }
        }
        Some(Commands::CheckUpdates {}) => {
            let ledger_path = root_dir.join(".installed-skills.json");
            let tracker = InstalledTracker::with_config(
                &root_dir,
                &ledger_path,
                "https://api.github.com",
                "https://raw.githubusercontent.com",
                15,
                cli.no_network,
            );
            match tracker.check_for_updates().await {
                Ok(updates) => {
                    if cli.json {
                        let payload = serde_json::json!({
                            "total": updates.len(),
                            "updates": updates
                        });
                        print_json(&payload)
                    } else if updates.is_empty() {
                        println!("No installed skills tracked.");
                        Ok(())
                    } else {
                        println!("=== INSTALLED SKILLS UPDATE STATUS (CHECK ONLY) ===");
                        for u in &updates {
                            println!(
                                "- {} (Source: {}, Current SHA: {}, Latest SHA: {}) -> {}",
                                u.name,
                                u.source,
                                u.current_commit_sha,
                                u.latest_commit_sha.as_deref().unwrap_or("UNKNOWN"),
                                u.status
                            );
                        }
                        Ok(())
                    }
                }
                Err(e) => Err(format!("Error checking updates: {}", e)),
            }
        }
        None => {
            if cli.task_words.is_empty() {
                eprintln!(
                    "Usage: skill-router [sort|route|find|sync-catalog|install-skill|list-skills|verify-skills|check-updates] \"<task>\" [--json] [--workflow auto|single|multi]"
                );
                process::exit(2);
            }
            let task_str = cli.task_words.join(" ");
            let options = RouteOptions {
                workflow: Some(cli.workflow),
                agent: cli.agent,
                no_network: cli.no_network,
                local_only: false,
                strict_compatibility: true,
            };
            match router.route(&task_str, &options).await {
                Ok(route_res) => {
                    if cli.json {
                        print_json(&route_res)
                    } else {
                        println!("=== AUTOMATIC SKILL ROUTER V0.3.2 ===");
                        println!("Task: {}", route_res.triage.outcome);
                        println!("Mandatory First Skill: {}", route_res.mandatory_first_skill);
                        println!(
                            "Complexity: {:?} (Score: {}/10)",
                            route_res.triage.complexity.band,
                            route_res.triage.complexity.total_score
                        );
                        println!("Workflow: {}", route_res.routing.workflow);
                        Ok(())
                    }
                }
                Err(e) => Err(e.to_string()),
            }
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
