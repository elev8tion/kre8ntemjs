use crate::Engine;
use anyhow::Result;

/// Greedy line-drop reducer: removes lines if the crash signature (caller-provided)
/// remains the same. Keep it tiny and fast—good enough for triage.
pub fn minimize_by<F>(prog: &str, engine: &Engine, same_bug: F) -> Result<String>
where
    F: Fn(&str) -> bool,
{
    let mut lines: Vec<&str> = prog.lines().collect();
    let mut i = 0usize;
    while i < lines.len() && lines.len() > 1 {
        let candidate = {
            let mut c = lines.clone();
            c.remove(i);
            c.join("\n")
        };
        let out = engine.run_js(&candidate)?;
        if same_bug(&out.stderr) {
            lines.remove(i); // accept the deletion
            // do not advance; try the same index again
        } else {
            i += 1;
        }
    }
    Ok(lines.join("\n"))
}

/// Coverage-preserving minimizer: keep candidate if scorer(candidate) >= target.
pub fn minimize_preserving_coverage(
    prog: &str,
    engine: &Engine,
    score_args: &[String],
    scorer: &regex::Regex,
    target_score: u64,
) -> Result<String> {
    let mut lines: Vec<&str> = prog.lines().collect();
    let mut i = 0usize;
    while i < lines.len() && lines.len() > 1 {
        let candidate = {
            let mut c = lines.clone();
            c.remove(i);
            c.join("\n")
        };
        let out = engine.run_js_with_args(&candidate, score_args)?;
        let s = {
            let mut sum = 0u64;
            for cap in scorer.captures_iter(&out.stdout) {
                for j in 1..cap.len() {
                    if let Ok(v) = cap[j].replace('_', "").parse::<u64>() { sum += v; }
                }
            }
            for cap in scorer.captures_iter(&out.stderr) {
                for j in 1..cap.len() {
                    if let Ok(v) = cap[j].replace('_', "").parse::<u64>() { sum += v; }
                }
            }
            sum
        };
        if s >= target_score {
            lines.remove(i); // accept deletion
        } else {
            i += 1;
        }
    }
    Ok(lines.join("\n"))
}
