//! Source file parsing for cargo-fa.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Find all Rust source files in a directory.
pub fn find_rust_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    for entry in WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| !is_hidden(e) && !is_target_dir(e))
    {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().map(|e| e == "rs").unwrap_or(false) {
            files.push(path.to_path_buf());
        }
    }
    
    Ok(files)
}

/// Parse a Rust source file into a syn AST.
pub fn parse_file(path: &Path) -> Result<syn::File> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    
    syn::parse_file(&content)
        .with_context(|| format!("failed to parse {}", path.display()))
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

fn is_target_dir(entry: &walkdir::DirEntry) -> bool {
    entry.file_name() == "target"
}

/// Extract span information from a syn span.
pub fn span_to_location(span: proc_macro2::Span, file: &Path) -> crate::diagnostics::Location {
    crate::diagnostics::Location {
        file: file.to_path_buf(),
        line: span.start().line,
        column: span.start().column + 1,
        end_line: Some(span.end().line),
        end_column: Some(span.end().column + 1),
    }
}

/// Visitor trait for AST traversal.
pub trait AstVisitor {
    fn visit_file(&mut self, _file: &syn::File) {}
    fn visit_item(&mut self, _item: &syn::Item) {}
    fn visit_fn(&mut self, _func: &syn::ItemFn) {}
    fn visit_impl(&mut self, _impl_block: &syn::ItemImpl) {}
    fn visit_expr(&mut self, _expr: &syn::Expr) {}
    fn visit_stmt(&mut self, _stmt: &syn::Stmt) {}
}

/// Walk an AST with a visitor.
pub fn walk_file<V: AstVisitor>(visitor: &mut V, file: &syn::File) {
    visitor.visit_file(file);
    
    for item in &file.items {
        walk_item(visitor, item);
    }
}

fn walk_item<V: AstVisitor>(visitor: &mut V, item: &syn::Item) {
    visitor.visit_item(item);
    
    match item {
        syn::Item::Fn(func) => {
            visitor.visit_fn(func);
            walk_block(visitor, &func.block);
        }
        syn::Item::Impl(impl_block) => {
            visitor.visit_impl(impl_block);
            for item in &impl_block.items {
                if let syn::ImplItem::Fn(method) = item {
                    walk_block(visitor, &method.block);
                }
            }
        }
        syn::Item::Mod(module) => {
            if let Some((_, items)) = &module.content {
                for item in items {
                    walk_item(visitor, item);
                }
            }
        }
        _ => {}
    }
}

fn walk_block<V: AstVisitor>(visitor: &mut V, block: &syn::Block) {
    for stmt in &block.stmts {
        walk_stmt(visitor, stmt);
    }
}

fn walk_stmt<V: AstVisitor>(visitor: &mut V, stmt: &syn::Stmt) {
    visitor.visit_stmt(stmt);
    
    match stmt {
        syn::Stmt::Local(local) => {
            if let Some(init) = &local.init {
                walk_expr(visitor, &init.expr);
            }
        }
        syn::Stmt::Expr(expr, _) => {
            walk_expr(visitor, expr);
        }
        syn::Stmt::Item(item) => {
            walk_item(visitor, item);
        }
        syn::Stmt::Macro(_) => {}
    }
}

fn walk_expr<V: AstVisitor>(visitor: &mut V, expr: &syn::Expr) {
    visitor.visit_expr(expr);
    
    match expr {
        syn::Expr::Block(block) => {
            walk_block(visitor, &block.block);
        }
        syn::Expr::Loop(loop_expr) => {
            walk_block(visitor, &loop_expr.body);
        }
        syn::Expr::While(while_expr) => {
            walk_expr(visitor, &while_expr.cond);
            walk_block(visitor, &while_expr.body);
        }
        syn::Expr::ForLoop(for_expr) => {
            walk_expr(visitor, &for_expr.expr);
            walk_block(visitor, &for_expr.body);
        }
        syn::Expr::If(if_expr) => {
            walk_expr(visitor, &if_expr.cond);
            walk_block(visitor, &if_expr.then_branch);
            if let Some((_, else_branch)) = &if_expr.else_branch {
                walk_expr(visitor, else_branch);
            }
        }
        syn::Expr::Match(match_expr) => {
            walk_expr(visitor, &match_expr.expr);
            for arm in &match_expr.arms {
                walk_expr(visitor, &arm.body);
            }
        }
        syn::Expr::Call(call) => {
            walk_expr(visitor, &call.func);
            for arg in &call.args {
                walk_expr(visitor, arg);
            }
        }
        syn::Expr::MethodCall(call) => {
            walk_expr(visitor, &call.receiver);
            for arg in &call.args {
                walk_expr(visitor, arg);
            }
        }
        syn::Expr::Await(await_expr) => {
            walk_expr(visitor, &await_expr.base);
        }
        syn::Expr::Closure(closure) => {
            walk_expr(visitor, &closure.body);
        }
        syn::Expr::Async(async_expr) => {
            walk_block(visitor, &async_expr.block);
        }
        _ => {}
    }
}
