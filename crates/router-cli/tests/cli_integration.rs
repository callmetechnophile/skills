use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_skill-router")
}

fn setup_temp_workspace(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cli_int_{}_{}", name, uuid::Uuid::new_v4()));
    std::fs::create_dir_all(dir.join("skills/core/sort-the-issue")).unwrap();
    std::fs::write(
        dir.join("skills/core/sort-the-issue/SKILL.md"),
        "---\nname: sort-the-issue\ndescription: Triage task complexity and decompose\n---\n# Sort the issue",
    )
    .unwrap();
    dir
}

#[test]
fn test_cli_sort_and_route_and_find_json() {
    let root = setup_temp_workspace("sort_route_find");

    // 1. sort --json
    let out = Command::new(bin())
        .args([
            "--root",
            root.to_str().unwrap(),
            "--json",
            "sort",
            "Fix",
            "login",
            "bug",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "sort failed: {:?}", out);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v.get("outcome").is_some());

    // 2. route --json --no-network
    let out_route = Command::new(bin())
        .args([
            "--root",
            root.to_str().unwrap(),
            "--json",
            "--no-network",
            "route",
            "Fix",
            "login",
            "bug",
        ])
        .output()
        .unwrap();
    assert!(out_route.status.success(), "route failed: {:?}", out_route);
    let vr: serde_json::Value = serde_json::from_slice(&out_route.stdout).unwrap();
    assert_eq!(vr["mandatory_first_skill"], "sort-the-issue");

    // 3. find --json --no-network
    let out_find = Command::new(bin())
        .args([
            "--root",
            root.to_str().unwrap(),
            "--json",
            "--no-network",
            "find",
            "triage",
        ])
        .output()
        .unwrap();
    assert!(out_find.status.success(), "find failed: {:?}", out_find);
    let vf: serde_json::Value = serde_json::from_slice(&out_find.stdout).unwrap();
    assert!(vf.is_array());

    // 4. Invalid flag fails cleanly
    let out_bad_flag = Command::new(bin())
        .args(["--nonexistent-flag", "sort", "test"])
        .output()
        .unwrap();
    assert!(!out_bad_flag.status.success());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn test_cli_sync_catalog_and_install_skill_guards() {
    let root = setup_temp_workspace("sync_install");

    // 1. sync-catalog --limit 0 fails nonzero
    let out_zero = Command::new(bin())
        .args([
            "--root",
            root.to_str().unwrap(),
            "sync-catalog",
            "--limit",
            "0",
        ])
        .output()
        .unwrap();
    assert!(!out_zero.status.success());

    // 2. sync-catalog --no-network --json succeeds with 0 pages
    let out_off = Command::new(bin())
        .args([
            "--root",
            root.to_str().unwrap(),
            "--json",
            "--no-network",
            "sync-catalog",
            "--limit",
            "10",
        ])
        .output()
        .unwrap();
    assert!(out_off.status.success());
    let vs: serde_json::Value = serde_json::from_slice(&out_off.stdout).unwrap();
    assert_eq!(vs["syncedPages"], 0);

    // 3. install-skill --no-network fails without touching files
    let out_inst_off = Command::new(bin())
        .args([
            "--root",
            root.to_str().unwrap(),
            "--no-network",
            "install-skill",
            "--repo",
            "owner/repo",
            "--skill",
            "demo",
        ])
        .output()
        .unwrap();
    assert!(!out_inst_off.status.success());
    assert!(!root.join(".agents/skills/demo").exists());

    // 4. install-skill --execute --dry-run conflicts
    let out_conflict = Command::new(bin())
        .args([
            "--root",
            root.to_str().unwrap(),
            "install-skill",
            "--repo",
            "owner/repo",
            "--skill",
            "demo",
            "--execute",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(!out_conflict.status.success());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn test_cli_list_verify_and_check_updates() {
    let root = setup_temp_workspace("lifecycle_cli");

    // 1. list-skills --json on empty workspace
    let out_list = Command::new(bin())
        .args(["--root", root.to_str().unwrap(), "--json", "list-skills"])
        .output()
        .unwrap();
    assert!(out_list.status.success());
    let vl: serde_json::Value = serde_json::from_slice(&out_list.stdout).unwrap();
    assert_eq!(vl.as_array().unwrap().len(), 0);

    // 2. Populate a valid installed skill + ledger
    let skill_dir = root.join(".agents/skills/demo-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), "# Demo Skill").unwrap();
    let hash = skill_registry::compute_directory_sha256(&skill_dir).unwrap();

    let ledger = serde_json::json!({
        "schema_version": "1.0",
        "last_updated": "2026-01-01T00:00:00Z",
        "installed": [{
            "skillId": "owner/repo:demo-skill",
            "name": "demo-skill",
            "version": "1.0.0",
            "source": "owner/repo",
            "commitSha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "license": "MIT",
            "installationPath": ".agents/skills/demo-skill",
            "targetAgent": "antigravity",
            "contentHash": hash,
            "trustStatus": "APPROVED",
            "installedAt": "2026-01-01T00:00:00Z",
            "lastVerification": "2026-01-01T00:00:00Z"
        }]
    });
    std::fs::write(
        root.join(".installed-skills.json"),
        serde_json::to_string_pretty(&ledger).unwrap(),
    )
    .unwrap();

    // 3. verify-skills --json passes
    let out_ver = Command::new(bin())
        .args(["--root", root.to_str().unwrap(), "--json", "verify-skills"])
        .output()
        .unwrap();
    assert!(out_ver.status.success());
    let vv: serde_json::Value = serde_json::from_slice(&out_ver.stdout).unwrap();
    assert_eq!(vv["intact"], 1);
    assert_eq!(vv["modified"], 0);

    // 4. check-updates --no-network --json reports UNKNOWN (never UP_TO_DATE!)
    let out_upd = Command::new(bin())
        .args([
            "--root",
            root.to_str().unwrap(),
            "--json",
            "--no-network",
            "check-updates",
        ])
        .output()
        .unwrap();
    assert!(out_upd.status.success());
    let vu: serde_json::Value = serde_json::from_slice(&out_upd.stdout).unwrap();
    assert_eq!(vu["updates"][0]["status"], "UNKNOWN");
    assert_eq!(vu["updates"][0]["updateAvailable"], false);

    // 5. Deprecated update:skills alias warns on stderr
    let out_alias = Command::new(bin())
        .args([
            "--root",
            root.to_str().unwrap(),
            "--json",
            "--no-network",
            "update:skills",
        ])
        .output()
        .unwrap();
    assert!(out_alias.status.success());
    let stderr_str = String::from_utf8_lossy(&out_alias.stderr);
    assert!(stderr_str.contains("deprecated"));

    // 6. Tamper file -> verify-skills exits nonzero
    std::fs::write(skill_dir.join("SKILL.md"), "# Tampered").unwrap();
    let out_tamper = Command::new(bin())
        .args(["--root", root.to_str().unwrap(), "--json", "verify-skills"])
        .output()
        .unwrap();
    assert!(!out_tamper.status.success());
    let vt: serde_json::Value = serde_json::from_slice(&out_tamper.stdout).unwrap();
    assert_eq!(vt["modified"], 1);

    let _ = std::fs::remove_dir_all(&root);
}
