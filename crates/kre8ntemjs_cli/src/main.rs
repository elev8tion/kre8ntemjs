
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

fn score_with_regex(out: &str, err: &str, re: &Regex) -> u64 {
    let mut sum = 0u64;
    for cap in re.captures_iter(out) {
        for i in 1..cap.len() {
            if let Ok(v) = cap[i].replace('_', "").parse::<u64>() {
                sum += v;
            }
        }
    }
    for cap in re.captures_iter(err) {
        for i in 1..cap.len() {
            if let Ok(v) = cap[i].replace('_', "").parse::<u64>() {
                sum += v;
            }
        }
    }
    sum
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

    /// Optional: extra args for the scoring pass (run #2)
    #[arg(long, value_delimiter=' ', num_args=0..)]
    score_cmd_args: Vec<String>,

    /// Regex to extract numeric coverage score from stdout+stderr of scoring pass.
    /// All captures are parsed as integers and summed.
    #[arg(long, default_value="")]
    score_regex: String,

    /// Keep only coverage-increasing inputs (requires score_regex).
    #[arg(long, default_value_t=false)]
    keep_only_increasing: bool,

    /// Minimizer mode: "signature" or "coverage"
    #[arg(long, default_value="signature")]
    minimize_by: String,
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
    let mut best_score: u64 = 0;
    let score_re = if !args.score_regex.is_empty() {
        Some(Regex::new(&args.score_regex).expect("invalid --score-regex"))
    } else { None };

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

        // Optional coverage scoring pass
        let mut cov_score: Option<u64> = None;
        if let Some(re) = &score_re {
            let score_run = eng.run_js_with_args(&prog, &args.score_cmd_args)?;
            let s = score_with_regex(&score_run.stdout, &score_run.stderr, re);
            cov_score = Some(s);
        }

        // Decide whether this is "interesting enough" to save/promote
        let is_increasing = if let Some(s) = cov_score { s > best_score } else { false };

        if outcome.timed_out {
            // Gate on increasing coverage if requested
            if args.keep_only_increasing && score_re.is_some() && !is_increasing {
                // skip saving; not increasing coverage
            } else {
                if let Some(s) = cov_score {
                    if s > best_score { best_score = s; }
                }
                timeouts += 1;
                let sig = crash_signature(&outcome.stderr);
                if seen.insert(sig.clone()) {
                    let path = args.out.join(format!("timeout_iter{}_sig{}.js", i, &sig[..8]));
                    std::fs::write(&path, &prog)?;
                    let stderr_path = args.out.join(format!("timeout_iter{}_sig{}.stderr.txt", i, &sig[..8]));
                    std::fs::write(&stderr_path, &outcome.stderr)?;
                }
            }
        } else if outcome.status != 0 && !is_syntax_error {
            let boring = outcome.stderr.contains("ReferenceError:") && !outcome.stderr.contains("prototype");
            if !boring {
                // Gate on increasing coverage if requested
                if args.keep_only_increasing && score_re.is_some() && !is_increasing {
                    // skip saving; not increasing coverage
                } else {
                    if let Some(s) = cov_score {
                        if s > best_score { best_score = s; }
                    }
                    let sig = crash_signature(&outcome.stderr);
                    if seen.insert(sig.clone()) {
                        crashes += 1;
                        // Minimize the crash
                        let minimized = if args.minimize_by == "coverage" && score_re.is_some() {
                            let score_run = eng.run_js_with_args(&prog, &args.score_cmd_args)?;
                            let target = score_with_regex(&score_run.stdout, &score_run.stderr, score_re.as_ref().unwrap());
                            kre8ntemjs_core::minimize_preserving_coverage(
                                &prog, &eng, &args.score_cmd_args, score_re.as_ref().unwrap(), target
                            ).unwrap_or(prog.clone())
                        } else {
                            // signature mode
                            kre8ntemjs_core::minimizer::minimize_by(&prog, &eng, |stderr| crash_signature(stderr) == sig)
                                .unwrap_or(prog.clone())
                        };
                        let path = args.out.join(format!("crash_iter{}_sig{}.js", i, &sig[..8]));
                        std::fs::write(&path, &minimized)?;
                        let stderr_path = args.out.join(format!("crash_iter{}_sig{}.stderr.txt", i, &sig[..8]));
                        std::fs::write(&stderr_path, &outcome.stderr)?;
                    }
                }
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
