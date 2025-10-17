
use clap::{Parser};
use std::path::PathBuf;
use std::time::Duration;
use std::collections::HashSet;
use rand::thread_rng;
use rand::Rng;
use sha1::{Digest, Sha1};
use regex::Regex;
use kre8ntemjs_core::{Extractor, Mutator, Concretizer, Engine, load_seed_paths, read_to_string};

fn crash_signature(stderr: &str) -> String {
    // Normalize unstable bits (paths, line numbers, hex ptrs)
    let re_hex = Regex::new(r"0x[0-9a-fA-F]+").unwrap();
    let re_line = Regex::new(r":\d+(:\d+)?").unwrap();
    let after_hex = re_hex.replace_all(stderr, "0xHEX");
    let norm = re_line.replace_all(&after_hex, ":LINE");
    let mut h = Sha1::new();
    h.update(norm.as_bytes());
    format!("{:x}", h.finalize())
}

/// Simple CLI for the MVP fuzzer.
#[derive(Parser, Debug)]
#[command(name="temujsx")]
#[command(version, about="Template-based JS fuzzer (from scratch)")]
struct Args {
    /// JS engine command (e.g., d8, jsc, js)
    #[arg(long)]
    engine_cmd: String,

    /// Additional args for the engine (repeatable)
    #[arg(long, num_args=0.., value_delimiter=' ')]
    engine_args: Vec<String>,

    /// Seed corpus directory (contains .js files)
    #[arg(long)]
    seeds: PathBuf,

    /// Output directory for artifacts
    #[arg(long)]
    out: PathBuf,

    /// Iterations
    #[arg(long, default_value_t=1000)]
    iters: u64,

    /// Timeout per run (e.g., 500ms, 2s). Defaults to 500ms.
    #[arg(long, default_value="500ms")]
    timeout: humantime::Duration,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    std::fs::create_dir_all(&args.out)?;

    let eng = Engine {
        cmd: args.engine_cmd,
        args: args.engine_args,
        timeout: Duration::from(args.timeout),
    };

    let seeds = load_seed_paths(&args.seeds)?;
    let extractor = Extractor::default();
    let mut rng = thread_rng();

    let mut seen: HashSet<String> = HashSet::new();
    let mut syntax_errors = 0usize;
    let mut crashes = 0usize;
    let mut timeouts = 0usize;

    for i in 0..args.iters {
        // pick a random seed and extract template
        let seed_path = seeds[rng.gen_range(0..seeds.len())].clone();
        let seed_src = read_to_string(&seed_path)?;
        let tpl_a = extractor.extract(&seed_src);

        // occasionally fuse with another template
        let tpl = if rng.gen_bool(0.2) {
            let other = &seeds[rng.gen_range(0..seeds.len())];
            let other_src = read_to_string(other)?;
            let tpl_b = extractor.extract(&other_src);
            kre8ntemjs_core::Mutator::fuse(&tpl_a, &tpl_b)
        } else {
            tpl_a
        };

        // choose a mutation op
        let mutated = match rng.gen_range(0..3) {
            0 => Mutator::insert_placeholder(&tpl, &mut rng),
            1 => Mutator::delete_placeholder(&tpl),
            _ => Mutator::substitute_placeholder(&tpl, &mut rng),
        };

        // concretize
        let prog = Concretizer::concretize(&mutated, &mut rng);

        // run
        let outcome = eng.run_js(&prog)?;

        // Filter out plain syntax errors; keep real crashes/timeouts.
        let is_syntax_error = outcome.stderr.contains("SyntaxError")
            || outcome.stderr.contains("Parse error")
            || outcome.stderr.contains("Unexpected token");

        if is_syntax_error {
            syntax_errors += 1;
        }

        if outcome.timed_out {
            timeouts += 1;
            let sig = crash_signature(&outcome.stderr);
            if seen.insert(sig.clone()) {
                let path = args.out.join(format!("timeout_iter{}_sig{}.js", i, &sig[..8]));
                std::fs::write(&path, &prog)?;
                let stderr_path = args.out.join(format!("timeout_iter{}_sig{}.stderr.txt", i, &sig[..8]));
                std::fs::write(&stderr_path, &outcome.stderr)?;
            }
        } else if outcome.status != 0 && !is_syntax_error {
            let sig = crash_signature(&outcome.stderr);
            if seen.insert(sig.clone()) {
                crashes += 1;
                // Minimize the crash
                let minimized = match kre8ntemjs_core::minimizer::minimize_by(&prog, &eng, |stderr| {
                    crash_signature(stderr) == sig
                }) {
                    Ok(m) => m,
                    Err(_) => prog.clone(),
                };
                let path = args.out.join(format!("crash_iter{}_sig{}.js", i, &sig[..8]));
                std::fs::write(&path, &minimized)?;
                let stderr_path = args.out.join(format!("crash_iter{}_sig{}.stderr.txt", i, &sig[..8]));
                std::fs::write(&stderr_path, &outcome.stderr)?;
            }
        }

        if i % 100 == 0 {
            eprintln!("iter {i} | syntax={syntax_errors} unique_crashes={crashes} timeouts={timeouts}");
        }
    }

    eprintln!("\n=== summary ===");
    eprintln!("syntax={syntax_errors} unique_crashes={crashes} timeouts={timeouts}");

    Ok(())
}
