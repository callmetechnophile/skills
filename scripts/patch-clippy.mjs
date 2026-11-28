/**
 * Patch script: fix 3 Clippy warnings (-D warnings level)
 *
 * Fix 1 (skill-discovery): Replace manual ceiling division with .div_ceil()
 * Fix 2 (skill-activation): Replace Result<T,T> large-err with Result<T, Box<T>>
 * Fix 3 (skill-activation): Add #[allow(clippy::too_many_arguments)] above activate()
 */
import { readFileSync, writeFileSync } from "node:fs";

function toWinPath(url) {
  return new URL(url, import.meta.url).pathname.replace(/^\/([A-Z]:)/, "$1");
}

// ─── Fix 1: skill-discovery/src/lib.rs ───────────────────────────────────────
{
  const p = toWinPath("../crates/skill-discovery/src/lib.rs");
  let src = readFileSync(p, "utf8");

  // Replace manual ceiling div pattern
  src = src.replace(
    /let max_pages = if page_size == 0 \{\s*0\s*\} else \{\s*\(limit \+ page_size - 1\) \/ page_size\s*\};/s,
    "let max_pages = limit.div_ceil(page_size);"
  );

  // Ensure usize::div_ceil is stable in Rust 1.73+; no extra use needed.
  writeFileSync(p, src);
  console.log("✅ skill-discovery: replaced manual ceiling div with .div_ceil()");
}

// ─── Fix 2 & 3: skill-activation/src/lib.rs ──────────────────────────────────
{
  const p = toWinPath("../crates/skill-activation/src/lib.rs");
  let src = readFileSync(p, "utf8");

  // Fix 2: Box the Err variant in the return type to reduce size
  src = src.replace(
    ") -> Result<SkillActivationRecord, SkillActivationRecord> {",
    ") -> Result<SkillActivationRecord, Box<SkillActivationRecord>> {"
  );
  // Update the two Err(SkillActivationRecord { ... }); calls to Err(Box::new(...))
  // Both Err returns have the same shape — replace `return Err(SkillActivationRecord {` → `return Err(Box::new(SkillActivationRecord {`
  // and the closing `});` → `}));`
  // We'll do a targeted string replacement for the two Err blocks.
  src = src.replace(
    /return Err\(SkillActivationRecord \{/g,
    "return Err(Box::new(SkillActivationRecord {"
  );
  // Close the Box::new( — each Err block ends with `        });` (8-space indent + `});`)
  // We need to change `        });` that immediately follows a `return Err(Box::new(` to `        }));`
  // Safest: replace the closing pattern contextually. Since there are exactly 2 such blocks,
  // we'll count and replace both occurrences.
  let count = 0;
  src = src.replace(/(\s+note: None,\s+\}\);)/g, (match) => {
    count++;
    if (count <= 2) return match.replace("});", "}));");
    return match;
  });

  // Fix 3: Add allow attribute above pub fn activate(
  src = src.replace(
    "    pub fn activate(\n        work_package_id: &str,",
    "    #[allow(clippy::too_many_arguments)]\n    pub fn activate(\n        work_package_id: &str,"
  );

  writeFileSync(p, src);
  console.log("✅ skill-activation: boxed Err variant + added #[allow(too_many_arguments)]");
}

console.log("\n✅ All Clippy fixes applied.");
