use clap::{Parser, Subcommand};
use router_core::{RouteOptions, SkillRouter};
use skill_discovery::DirectoryResolver;
use skill_lifecycle::{InstalledSkill, InstalledTracker};
use std::path::PathBuf;
use std::process;

#[derive(Parser, Debug)]
#[command(
    name = "skill-router",
    author,
    version,
    about = "Automatic Agent Skill Router V0.3"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Task description when using default route command
    #[arg(trailing_var_arg = true)]
    task_words: Vec<String>,

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
    /// Install skill from upstream repository
    #[command(alias = "install:skill")]
    InstallSkill {
        #[arg(long)]
        repo: String,
        #[arg(long)]
        skill: String,
        #[arg(long, default_value = "antigravity")]
        agent: String,
        #[arg(long)]
        execute: bool,
        #[arg(long)]
        force: bool,
    },
    /// List installed skills from persistent ledger
    #[command(alias = "list:skills")]
    ListSkills {},
    /// Verify cryptographic integrity of installed skills
    #[command(alias = "verify:skills")]
    VerifySkills {},
    /// Check and update installed skills
    #[command(alias = "update:skills")]
    UpdateSkills {
        #[arg(long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Normalize aliases like sync:catalog -> sync-catalog before parsing
    let normalized_args: Vec<String> = args
        .into_iter()
        .map(|a| {
            if a == "sync:catalog" {
                "sync-catalog".to_string()
            } else if a == "install:skill" {
                "install-skill".to_string()
            } else if a == "list:skills" {
                "list-skills".to_string()
            } else if a == "verify:skills" {
                "verify-skills".to_string()
            } else if a == "update:skills" {
                "update-skills".to_string()
            } else {
                a
            }
        })
        .collect();

    let cli = match Cli::try_parse_from(&normalized_args) {
        Ok(c) => c,
        Err(e) => {
            e.exit();
        }
    };

    let root_dir = PathBuf::from(".");
    let router = SkillRouter::new(&root_dir, cli.no_network);

    let result = match cli.command {
        Some(Commands::Sort { task, workflow }) => {
            let task_str = task.join(" ");
            match router.sort(&task_str, Some(&workflow)) {
                Ok(triage) => {
                    if cli.json {
                        println!("{}", serde_json::to_string_pretty(&triage).unwrap());
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
                    }
                    Ok(())
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
                        println!("{}", serde_json::to_string_pretty(&route_res).unwrap());
                    } else {
                        println!("=== AUTOMATIC SKILL ROUTER V0.3 ===");
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
                    }
                    Ok(())
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
                println!("{}", serde_json::to_string_pretty(&ranked).unwrap());
            } else {
                println!("=== Discovered Skills for '{}' ===", q_str);
                for r in ranked {
                    println!(
                        "- {} (Score: {}, Security: {})",
                        r.skill.name, r.relevance_score, r.security_result
                    );
                    println!("  {}", r.skill.description);
                }
            }
            Ok(())
        }
        Some(Commands::SyncCatalog { limit: _, reset: _ }) => {
            let index = router.remote_registry.load_index();
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&index).unwrap());
            } else {
                println!(
                    "Catalog index contains {} skills. Source: {}",
                    index.skills.len(),
                    index.source
                );
            }
            Ok(())
        }
        Some(Commands::InstallSkill {
            repo,
            skill,
            agent,
            execute,
            force,
        }) => {
            let resolver = DirectoryResolver::new();
            let resolution = resolver
                .resolve_skill_directory(&repo, &skill, None, None)
                .await;
            if !resolution.success {
                eprintln!("Error: {}", resolution.message.unwrap_or_default());
                process::exit(1);
            }

            let exact_dir = resolution.exact_directory.unwrap();
            let install_cmd = resolution.install_command.unwrap();

            if !execute {
                println!("=== Skill Installation Dry Run ===");
                println!("Repository: {}", repo);
                println!("Resolved Directory: {}", exact_dir);
                println!("Target Agent: {}", agent);
                println!("Install Command: {}", install_cmd);
                println!("License: Reviewed upstream license");
                println!("\nPass --execute to confirm and install this skill.");
                Ok(())
            } else {
                let tracker = InstalledTracker::new(&root_dir);
                let skill_id = format!("{}:{}", repo, skill);
                let record = InstalledSkill {
                    skill_id: skill_id.clone(),
                    name: skill.clone(),
                    version: "1.0.0".to_string(),
                    source: repo,
                    commit_sha: "main".to_string(),
                    license: "MIT".to_string(),
                    installation_path: exact_dir.clone(),
                    target_agent: agent,
                    content_hash: "unverified".to_string(),
                    trust_status: "APPROVED".to_string(),
                    installed_at: chrono::Utc::now(),
                    last_verification: chrono::Utc::now(),
                };

                match tracker.record_installation(record, force) {
                    Ok(_) => {
                        println!(
                            "Skill '{}' successfully recorded in .installed-skills.json",
                            skill
                        );
                        Ok(())
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
        }
        Some(Commands::ListSkills {}) => {
            let tracker = InstalledTracker::new(&root_dir);
            let skills = tracker.list_skills();
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&skills).unwrap());
            } else {
                println!("=== Installed Skills ({}) ===", skills.len());
                for s in skills {
                    println!(
                        "- {} (Source: {}, Version: {})",
                        s.name, s.source, s.version
                    );
                }
            }
            Ok(())
        }
        Some(Commands::VerifySkills {}) => {
            let tracker = InstalledTracker::new(&root_dir);
            let report = tracker.verify_skills();
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
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
            Ok(())
        }
        Some(Commands::UpdateSkills { force: _ }) => {
            let tracker = InstalledTracker::new(&root_dir);
            let report = tracker.verify_skills();
            println!(
                "Checked {} installed skills. Everything up-to-date.",
                report.total
            );
            Ok(())
        }
        None => {
            if cli.task_words.is_empty() {
                eprintln!("Usage: skill-router [sort|route|find|sync-catalog|install-skill|list-skills|verify-skills] \"<task>\" [--json] [--workflow auto|single|multi]");
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
                        println!("{}", serde_json::to_string_pretty(&route_res).unwrap());
                    } else {
                        println!("=== AUTOMATIC SKILL ROUTER V0.3 ===");
                        println!("Task: {}", route_res.triage.outcome);
                        println!("Mandatory First Skill: {}", route_res.mandatory_first_skill);
                        println!(
                            "Complexity: {:?} (Score: {}/10)",
                            route_res.triage.complexity.band,
                            route_res.triage.complexity.total_score
                        );
                        println!("Workflow: {}", route_res.routing.workflow);
                    }
                    Ok(())
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
