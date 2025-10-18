use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::process::{Command, Stdio};

/// scorewrap: run a JS engine and print "edges:<N>" for the fuzzer to parse.
/// Two modes:
///   1) --edges-file /tmp/kre8_edges.txt     (engine writes a count there)
///   2) --score-regex 'edges:(\\d+)'         (parse stdout/stderr if no file)
#[derive(Parser, Debug)]
struct Args {
    /// The real engine binary (e.g., an instrumented d8)
    #[arg(long)]
    engine: String,

    /// Pass-through args for the engine (repeatable)
    #[arg(long, num_args=0.., value_delimiter=' ')]
    engine_args: Vec<String>,

    /// Optional: file where engine writes the coverage count (either "edges:<N>" or "<N>")
    #[arg(long)]
    edges_file: Option<String>,

    /// Optional: regex to parse a numeric score from stdout/stderr
    #[arg(long, default_value = "")]
    score_regex: String,

    /// JS file path (last arg, provided by the fuzzer)
    js: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // 1) Run the engine on the JS file
    let mut cmd = Command::new(&args.engine);
    cmd.args(&args.engine_args)
        .arg(&args.js)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = cmd.output().with_context(|| "failed to run engine")?;

    // 2) Prefer file-based edges if provided
    if let Some(p) = args.edges_file.as_ref() {
        let s = fs::read_to_string(p).with_context(|| format!("read {}", p))?;
        // Accept either "edges:<N>" or just "<N>"
        if let Some(num) = s.trim().strip_prefix("edges:") {
            println!("edges:{}", num.trim());
        } else {
            println!("edges:{}", s.trim());
        }
        return Ok(());
    }

    // 3) Else parse stdout/stderr with regex if provided
    if !args.score_regex.is_empty() {
        let re = regex::Regex::new(&args.score_regex).context("invalid score_regex")?;
        let mut sum: u64 = 0;

        let so = String::from_utf8_lossy(&out.stdout);
        let se = String::from_utf8_lossy(&out.stderr);

        for cap in re.captures_iter(&so) {
            for i in 1..cap.len() {
                if let Ok(v) = cap[i].replace('_', "").parse::<u64>() { sum += v; }
            }
        }
        for cap in re.captures_iter(&se) {
            for i in 1..cap.len() {
                if let Ok(v) = cap[i].replace('_', "").parse::<u64>() { sum += v; }
            }
        }

        println!("edges:{sum}");
        return Ok(());
    }

    // 4) Fallback: no score known
    println!("edges:0");
    Ok(())
}
