
use rand::{Rng, seq::SliceRandom};
use regex::Regex;
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::fs;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;
use walkdir::WalkDir;
use anyhow::{Result, Context};
use thiserror::Error;

pub mod ast;
pub mod minimizer;

#[derive(Debug, Error)]
pub enum FuzzError {
    #[error("engine failed with status: {0}")]
    EngineFailed(i32),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("other: {0}")]
    Other(String),
}

/// Placeholder variants (MVP). Extend as needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="k", content="v")]
pub enum Placeholder {
    Var,
    Integer,
    CodeStr,
    Range(i64, i64),
}

/// A simple string-backed template with placeholder tokens.
/// Example: "let <var> = <integer>;"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub source: String,
}

impl Template {
    pub fn from_source(src: &str) -> Self { Self { source: src.to_string() } }
    pub fn tokens() -> &'static [(&'static str, Placeholder)] {
        &[
            ("<var>", Placeholder::Var),
            ("<integer>", Placeholder::Integer),
            ("<code_str>", Placeholder::CodeStr),
        ]
    }
}

/// Extractor (MVP): replace obvious integers/idents with placeholders using regex.
pub struct Extractor {
    re_int: Regex,
    re_var_decl: Regex,
}

impl Default for Extractor {
    fn default() -> Self {
        Self {
            re_int: Regex::new(r"(?P<num>\b\d+\b)").unwrap(),
            re_var_decl: Regex::new(r"\blet\s+([a-zA-Z_]\w*)").unwrap(),
        }
    }
}

impl Extractor {
    pub fn extract(&self, src: &str) -> Template {
        // AST pass to find numeric literals & lexical decl ids
        let mut js = crate::ast::JsAst::default();
        if let Some(tree) = js.parse(src) {
            // Collect edits as (start_byte, end_byte, replacement)
            let mut edits: Vec<(usize, usize, &'static str)> = Vec::new();
            let root = tree.root_node();
            let mut cursor = root.walk();
            let mut stack = vec![root];
            while let Some(n) = stack.pop() {
                let kind = n.kind();
                match kind {
                    "number" => edits.push((n.start_byte(), n.end_byte(), "<integer>")),
                    "identifier" => {
                        if let Some(parent) = n.parent() {
                            // Only replace identifiers on the left side of decls
                            if matches!(parent.kind(), "variable_declarator" | "lexical_declaration") {
                                edits.push((n.start_byte(), n.end_byte(), "<var>"));
                            }
                        }
                    }
                    _ => {}
                }
                if n.child_count() > 0 {
                    cursor.reset(n);
                    for i in 0..n.child_count() {
                        if let Some(c) = n.child(i) { stack.push(c); }
                    }
                }
            }
            // Apply edits from right to left
            let mut s = src.to_string();
            edits.sort_by_key(|e| std::cmp::Reverse(e.0));
            for (start, end, rep) in edits {
                s.replace_range(start..end, rep);
            }
            return Template::from_source(&s);
        }
        // Fallback to regex if parse fails
        let mut s = self.re_int.replace_all(src, "<integer>").to_string();
        s = self.re_var_decl.replace_all(&s, "let <var>").to_string();
        Template::from_source(&s)
    }
}

/// Mutator: simple high-level operators over the string template.
pub struct Mutator;

impl Mutator {
    pub fn insert_placeholder<R: Rng>(tpl: &Template, rng: &mut R) -> Template {
        // AST-safe insertion using statement nodes
        let mut js = crate::ast::JsAst::default();
        if let Some(tree) = js.parse(&tpl.source) {
            let stmts = crate::ast::collect_statement_nodes(&tree, &tpl.source);
            let stmt_tmpls = [
                "let <var> = <integer>;",
                "const <var> = <integer>;",
                "function <var>(){ return <integer>; } <var>();",
                "try { <var> = <integer>; } catch (e) {}",
                "for (let <var> = 0; <var> < <integer>; <var>++) { }",
                "({ toString(){ return <code_str>; } });",
            ];
            let insertion = stmt_tmpls.choose(rng).unwrap();
            let target = stmts.choose(rng).copied();
            if let Some(node) = target {
                let s = crate::ast::insert_at_node(&tpl.source, node, insertion);
                return Template::from_source(&s);
            }
        }
        // Fallback to old line-boundary insertion if parse fails
        let mut lines: Vec<&str> = tpl.source.split('\n').collect();
        let idx = if lines.is_empty() { 0 } else { rng.gen_range(0..=lines.len()) };
        let stmt = "let <var> = <integer>;";
        lines.insert(idx, stmt);
        Template::from_source(&lines.join("\n"))
    }

    pub fn delete_placeholder(tpl: &Template) -> Template {
        let mut s = tpl.source.clone();
        for (tok, _) in Template::tokens() {
            if let Some(pos) = s.find(tok) {
                s.replace_range(pos..pos+tok.len(), "");
                break;
            }
        }
        Template::from_source(&s)
    }

    pub fn substitute_placeholder<R: Rng>(tpl: &Template, rng: &mut R) -> Template {
        let mut s = tpl.source.clone();
        let tokens: Vec<&str> = Template::tokens().iter().map(|(t, _)| *t).collect();
        if let Some((tok, _)) = Template::tokens().choose(rng) {
            s = s.replace(tok, tokens.choose(rng).unwrap());
        }
        Template::from_source(&s)
    }

    pub fn fuse(a: &Template, b: &Template) -> Template {
        // naive concatenation with a newline
        let s = format!("{}\n{}", a.source, b.source);
        Template::from_source(&s)
    }
}

/// Concretizer: generate concrete JS for placeholders.
pub struct Concretizer;

impl Concretizer {
    pub fn concretize<R: Rng>(tpl: &Template, rng: &mut R) -> String {
        let mut out = tpl.source.clone();
        // Replace <integer>
        while let Some(pos) = out.find("<integer>") {
            let val = rng.gen_range(-10_000..=10_000);
            out.replace_range(pos..pos+"<integer>".len(), &val.to_string());
        }
        // Replace <var>
        let names = ["a","b","c","x","y","z","tmp","obj","v"];
        while let Some(pos) = out.find("<var>") {
            let name = names.choose(rng).unwrap();
            out.replace_range(pos..pos+"<var>".len(), name);
        }
        // Replace <code_str> with quoted snippet
        while let Some(pos) = out.find("<code_str>") {
            let snippets = [
                r#"\"let k = 1;\""#,
                r#"\"class C{}\""#,
                r#"\"({a:1})\""#,
                r#"\"function f(){}\""#,
            ];
            let s = snippets.choose(rng).unwrap();
            out.replace_range(pos..pos+"<code_str>".len(), s);
        }
        out
    }
}

/// Engine runner: writes JS to temp file and executes engine command.
#[derive(Debug, Clone)]
pub struct Engine {
    pub cmd: String,
    pub args: Vec<String>,
    pub timeout: Duration,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunOutcome {
    pub status: i32,
    pub timed_out: bool,
    pub stderr: String,
    pub stdout: String,
}

impl Engine {
    pub fn run_js(&self, js: &str) -> Result<RunOutcome> {
        let mut tmp = NamedTempFile::new()?;
        fs::write(tmp.path(), js)?;
        let mut child = Command::new(&self.cmd)
            .args(self.args.clone())
            .arg(tmp.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to spawn engine {}", self.cmd))?;

        // crude timeout: poll with wait_timeout-like loop
        let start = std::time::Instant::now();
        let mut timed_out = false;
        loop {
            if let Some(status) = child.try_wait()? {
                let out = {
                    let mut s = String::new();
                    if let Some(mut so) = child.stdout.take() {
                        use std::io::Read;
                        so.read_to_string(&mut s).ok();
                    }
                    s
                };
                let err = {
                    let mut s = String::new();
                    if let Some(mut se) = child.stderr.take() {
                        use std::io::Read;
                        se.read_to_string(&mut s).ok();
                    }
                    s
                };
                return Ok(RunOutcome {
                    status: status.code().unwrap_or(-1),
                    timed_out,
                    stdout: out,
                    stderr: err,
                });
            }
            if start.elapsed() >= self.timeout {
                timed_out = true;
                child.kill().ok();
                let out = String::new();
                let err = "timeout".to_string();
                return Ok(RunOutcome { status: -1, timed_out, stdout: out, stderr: err });
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

/// Corpus utilities
pub fn load_seed_paths(seeds_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(seeds_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.path().extension().map(|e| e=="js").unwrap_or(false) {
            files.push(entry.into_path());
        }
    }
    if files.is_empty() {
        anyhow::bail!("no .js seeds found in {}", seeds_dir.display());
    }
    Ok(files)
}

pub fn read_to_string(p: &Path) -> Result<String> {
    Ok(fs::read_to_string(p)?)
}

