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
