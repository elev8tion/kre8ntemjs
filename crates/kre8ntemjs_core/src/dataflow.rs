use anyhow::Result;
use std::collections::HashMap;
use tree_sitter::{Node, Parser, Tree, Language};

extern "C" { fn tree_sitter_javascript() -> Language; }

#[derive(Default)]
pub struct JsDf {
    parser: Parser,
}

impl Default for JsDf {
    fn default() -> Self {
        let mut parser = Parser::new();
        let lang = unsafe { tree_sitter_javascript() };
        parser.set_language(lang).expect("set JS grammar");
        Self { parser }
    }
}

pub struct DfReport {
    pub def_count: HashMap<String, usize>,
    pub use_count: HashMap<String, usize>,
}

impl JsDf {
    pub fn analyze(&mut self, src: &str) -> Result<DfReport> {
        let tree = self.parser.parse(src, None).ok_or_else(|| anyhow::anyhow!("parse failed"))?;
        Ok(collect_defs_uses(&tree, src))
    }
}

/// Very simple def/use accounting:
/// - DEF: identifiers that are declared (let/const/var/function/class) or are LHS of assignment.
/// - USE: identifiers appearing in expressions not being declared.
/// NOTE: This is intentionally lightweight; you can refine it later.
fn collect_defs_uses(tree: &Tree, _src: &str) -> DfReport {
    use std::collections::HashSet;
    let root = tree.root_node();
    let mut defs: HashMap<String, usize> = HashMap::new();
    let mut uses: HashMap<String, usize> = HashMap::new();

    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        let k = n.kind();

        // Declarations
        if k == "variable_declarator" {
            if let Some(id) = n.child_by_field_name("name") {
                if id.kind() == "identifier" {
                    incr(&mut defs, id.utf8_text(_src.as_bytes()).unwrap_or("").to_string(), 1);
                }
            }
        } else if k == "function_declaration" || k == "class_declaration" {
            if let Some(id) = n.child_by_field_name("name") {
                if id.kind() == "identifier" {
                    incr(&mut defs, id.utf8_text(_src.as_bytes()).unwrap_or("").to_string(), 1);
                }
            }
        } else if k == "assignment_expression" {
            if let Some(lhs) = n.child_by_field_name("left") {
                if lhs.kind() == "identifier" {
                    incr(&mut defs, lhs.utf8_text(_src.as_bytes()).unwrap_or("").to_string(), 1);
                }
            }
        }

        // Uses: any identifiers not caught above (rough heuristic)
        if k == "identifier" {
            // if parent is a declarator name or function/class name, skip (already counted as def)
            if let Some(p) = n.parent() {
                let pk = p.kind();
                let is_decl_name = pk == "variable_declarator"
                    || pk == "function_declaration"
                    || pk == "class_declaration";
                if !is_decl_name {
                    incr(&mut uses, n.utf8_text(_src.as_bytes()).unwrap_or("").to_string(), 1);
                }
            }
        }

        // DFS
        for i in 0..n.child_count() {
            if let Some(c) = n.child(i) {
                stack.push(c);
            }
        }
    }

    DfReport { def_count: defs, use_count: uses }
}

fn incr(map: &mut HashMap<String, usize>, k: String, by: usize) {
    *map.entry(k).or_insert(0) += by;
}

/// DFComp(v) = DefCount(v) + UseCount(v)
pub fn dfcomp(report: &DfReport) -> HashMap<String, usize> {
    let mut out = report.def_count.clone();
    for (k, u) in &report.use_count {
        *out.entry(k.clone()).or_insert(0) += *u;
    }
    out
}
