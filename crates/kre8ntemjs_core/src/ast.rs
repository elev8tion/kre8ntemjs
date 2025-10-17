use tree_sitter::{Language, Node, Parser, Tree};

pub struct JsAst {
    parser: Parser,
}

impl Default for JsAst {
    fn default() -> Self {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_javascript::language()).expect("set JS grammar");
        Self { parser }
    }
}

impl JsAst {
    pub fn parse(&mut self, src: &str) -> Option<Tree> {
        self.parser.parse(src, None)
    }
}

/// Collect statement nodes (safe insertion boundaries).
pub fn collect_statement_nodes<'a>(tree: &'a Tree, src: &str) -> Vec<Node<'a>> {
    let mut out = Vec::new();
    let root = tree.root_node();
    let mut cursor = root.walk();
    let mut stack = vec![root];

    while let Some(n) = stack.pop() {
        // Common statement-ish node types in TS-JS grammar
        let ty = n.kind();
        if matches!(ty,
            "statement_block"
            | "variable_declaration"
            | "lexical_declaration"
            | "expression_statement"
            | "if_statement"
            | "for_statement"
            | "for_in_statement"
            | "for_of_statement"
            | "while_statement"
            | "do_statement"
            | "return_statement"
            | "throw_statement"
            | "try_statement"
            | "function_declaration"
            | "class_declaration"
        ) {
            out.push(n);
        }
        if n.child_count() > 0 {
            cursor.reset(n);
            for i in 0..n.child_count() {
                if let Some(c) = n.child(i) { stack.push(c); }
            }
        }
    }
    if out.is_empty() { out.push(root); }
    out
}

/// Insert text at a node boundary (before the node's start byte).
pub fn insert_at_node(src: &str, node: Node<'_>, insertion: &str) -> String {
    let b = node.start_byte();
    let (head, tail) = src.split_at(b);
    let mut s = String::with_capacity(src.len() + insertion.len() + 1);
    s.push_str(head);
    // ensure we land at a line boundary
    if !head.ends_with('\n') { s.push('\n'); }
    s.push_str(insertion);
    if !insertion.ends_with('\n') { s.push('\n'); }
    s.push_str(tail);
    s
}
