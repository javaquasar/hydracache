//! Source inventory only: parse code, never expand or execute it.
use proc_macro2::{TokenStream, TokenTree};
use serde_json::{json, Value};
use syn::parse::Parser;
use syn::visit::{self, Visit};

fn path(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|v| v.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}
fn expression(value: &syn::Expr) -> String {
    match value {
        syn::Expr::Lit(v) => match &v.lit {
            syn::Lit::Str(v) => format!("literal:{}", v.value()),
            _ => "literal-other".into(),
        },
        syn::Expr::Path(v) => format!("path:{}", path(&v.path)),
        syn::Expr::Call(v) => format!("call:{}", expression(&v.func)),
        syn::Expr::Macro(v) => format!("macro:{}({})", path(&v.mac.path), v.mac.tokens),
        syn::Expr::If(_) => "conditional-expression".into(),
        _ => "computed-expression".into(),
    }
}
#[derive(Default)]
struct Inventory {
    scope: Vec<String>,
    sites: Vec<Value>,
}
impl Inventory {
    fn site(&mut self, kind: &str, detail: String) {
        self.sites
            .push(json!({"scope":self.scope.join("::"),"kind":kind,"detail":detail}));
    }
    fn tokens(&mut self, tokens: TokenStream) {
        let values: Vec<_> = tokens.into_iter().collect();
        for window in values.windows(5) {
            if let [TokenTree::Ident(a), TokenTree::Punct(b), TokenTree::Punct(c), TokenTree::Ident(d), TokenTree::Group(args)] =
                window
            {
                if a == "Command" && b.as_char() == ':' && c.as_char() == ':' && d == "new" {
                    self.site(
                        "macro-command-constructor",
                        syn::parse2::<syn::Expr>(args.stream())
                            .map(|v| expression(&v))
                            .unwrap_or_else(|_| "macro-parameter".into()),
                    );
                }
            }
        }
        for window in values.windows(3) {
            if let [TokenTree::Punct(dot), TokenTree::Ident(method), TokenTree::Group(_)] = window {
                if dot.as_char() == '.'
                    && matches!(
                        method.to_string().as_str(),
                        "spawn" | "output" | "status" | "kill"
                    )
                {
                    self.site("macro-process-method-candidate", method.to_string());
                }
            }
        }
        for token in values {
            if let TokenTree::Group(group) = token {
                self.tokens(group.stream());
            }
        }
    }
}
impl<'ast> Visit<'ast> for Inventory {
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.scope.push(format!("impl::{}", node.sig.ident));
        visit::visit_impl_item_fn(self, node);
        self.scope.pop();
    }
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.scope.push(node.sig.ident.to_string());
        for attr in &node.attrs {
            if matches!(path(attr.path()).as_str(), "test" | "ignore" | "cfg") {
                self.site(
                    "function-attribute",
                    match &attr.meta {
                        syn::Meta::Path(v) => path(v),
                        syn::Meta::List(v) => format!("{}({})", path(&v.path), v.tokens),
                        _ => "attribute-value".into(),
                    },
                );
            }
        }
        visit::visit_item_fn(self, node);
        self.scope.pop();
    }
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        self.scope.push(node.ident.to_string());
        if node.content.is_none() {
            let mut literal = None;
            for attr in &node.attrs {
                if attr.path().is_ident("path") {
                    if let syn::Meta::NameValue(v) = &attr.meta {
                        literal = Some(expression(&v.value));
                    }
                }
            }
            self.site(
                "module",
                literal.unwrap_or_else(|| "default-module-path".into()),
            );
        }
        visit::visit_item_mod(self, node);
        self.scope.pop();
    }
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(function) = &*node.func {
            let name = path(&function.path);
            self.site("function-call", name.clone());
            if name.ends_with("Command::new") {
                self.site(
                    "command-constructor",
                    node.args.first().map(expression).unwrap_or_default(),
                );
            } else if name.ends_with("::command")
                || name.ends_with("::execute_entry")
                || name.ends_with("::execute_gate_with_identity")
                || name.ends_with("::run")
                || name.ends_with("::remove_dir_all")
                || name.ends_with("::remove_file")
            {
                self.site("indirect-or-removal-call", name);
            }
        }
        visit::visit_expr_call(self, node);
    }
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if matches!(
            node.method.to_string().as_str(),
            "spawn" | "output" | "status" | "kill" | "kill_on_drop"
        ) {
            self.site("process-method-candidate", node.method.to_string());
        }
        visit::visit_expr_method_call(self, node);
    }
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let name = path(&node.path);
        if name == "include" {
            self.site("include", node.tokens.to_string());
        }
        self.scope.push(format!("macro:{name}"));
        self.tokens(node.tokens.clone());
        if matches!(
            name.as_str(),
            "assert"
                | "assert_eq"
                | "assert_ne"
                | "debug_assert"
                | "debug_assert_eq"
                | "debug_assert_ne"
                | "format"
                | "println"
                | "eprintln"
        ) {
            if let Ok(expressions) =
                syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
                    .parse2(node.tokens.clone())
            {
                for expression in expressions {
                    self.visit_expr(&expression);
                }
            }
        }
        self.scope.pop();
        visit::visit_macro(self, node);
    }
}

pub fn inventory(raw: &str) -> Result<Vec<Value>, syn::Error> {
    let file = syn::parse_file(raw)?;
    let mut inventory = Inventory::default();
    inventory.visit_file(&file);
    Ok(inventory.sites)
}
