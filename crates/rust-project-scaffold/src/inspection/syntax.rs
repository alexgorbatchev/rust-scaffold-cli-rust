use crate::{Result, invalid};
use std::{collections::BTreeMap, fs, path::Path};
use syn::{
    parse::Parser,
    visit::{self, Visit},
};

#[derive(Default)]
pub(super) struct SourceChecks {
    pub agent: bool,
    pub clap: bool,
    pub help_template: bool,
    pub tree_branch: bool,
    pub tree_end: bool,
    pub skill_command: bool,
    pub skill_file: bool,
    pub library: bool,
    pub tests: bool,
}
pub(super) fn inspect(target: &Path, clap_names: &[String]) -> Result<SourceChecks> {
    let mut result = SourceChecks::default();
    for directory in ["src", "tests"] {
        let root = target.join(directory);
        if !root.try_exists()? {
            continue;
        }
        for entry in walkdir::WalkDir::new(root).follow_links(false) {
            let entry = entry.map_err(|error| invalid(error.to_string()))?;
            if !entry.file_type().is_file()
                || entry
                    .path()
                    .extension()
                    .is_none_or(|extension| extension != "rs")
            {
                continue;
            }
            let text = fs::read_to_string(entry.path())?;
            let Ok(file) = syn::parse_file(&text) else {
                continue;
            };
            if entry.path() == target.join("src/lib.rs") {
                result.library = !file.items.is_empty();
            }
            let mut visitor = SourceVisitor {
                result: &mut result,
                imports: BTreeMap::new(),
                clap_names,
                target,
                source: entry.path(),
                production: directory == "src",
                error: None,
            };
            visitor.visit_file(&file);
            if let Some(error) = visitor.error {
                return Err(error.into());
            }
        }
    }
    Ok(result)
}
fn collect_use(tree: &syn::UseTree, prefix: &[String], output: &mut BTreeMap<String, Vec<String>>) {
    let mut path = prefix.to_vec();
    match tree {
        syn::UseTree::Path(item) => {
            path.push(item.ident.to_string());
            collect_use(&item.tree, &path, output);
        }
        syn::UseTree::Name(item) => {
            let name = item.ident.to_string();
            if name != "self" {
                path.push(name.clone());
            }
            let alias = if name == "self" {
                path.last().cloned().unwrap_or(name)
            } else {
                name
            };
            output.insert(alias, path);
        }
        syn::UseTree::Rename(item) => {
            path.push(item.ident.to_string());
            output.insert(item.rename.to_string(), path);
        }
        syn::UseTree::Group(item) => {
            for child in &item.items {
                collect_use(child, prefix, output);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}
struct SourceVisitor<'a> {
    result: &'a mut SourceChecks,
    imports: BTreeMap<String, Vec<String>>,
    clap_names: &'a [String],
    target: &'a Path,
    source: &'a Path,
    production: bool,
    error: Option<std::io::Error>,
}
impl SourceVisitor<'_> {
    fn resolved(&self, path: &syn::Path) -> Vec<String> {
        let mut segments = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        if let Some(alias) = segments.first().and_then(|first| self.imports.get(first)) {
            let mut resolved = alias.clone();
            resolved.extend(segments.drain(1..));
            resolved
        } else {
            segments
        }
    }
    fn is_clap(&self, path: &syn::Path, suffix: &str) -> bool {
        let path = self.resolved(path);
        path.first()
            .is_some_and(|segment| self.clap_names.contains(segment))
            && path.last().is_some_and(|segment| segment == suffix)
    }
}
impl<'ast> Visit<'ast> for SourceVisitor<'_> {
    fn visit_file(&mut self, file: &'ast syn::File) {
        for item in &file.items {
            if let syn::Item::Use(item) = item {
                collect_use(&item.tree, &[], &mut self.imports);
            }
        }
        visit::visit_file(self, file);
    }
    fn visit_block(&mut self, block: &'ast syn::Block) {
        let previous = self.imports.clone();
        for statement in &block.stmts {
            if let syn::Stmt::Item(syn::Item::Use(item)) = statement {
                collect_use(&item.tree, &[], &mut self.imports);
            }
        }
        visit::visit_block(self, block);
        self.imports = previous;
    }
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if item.attrs.iter().any(|attr| {
            attr.path()
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "test")
        }) {
            self.result.tests = true;
            return;
        }
        visit::visit_item_fn(self, item);
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let previous = std::mem::take(&mut self.imports);
        // Inline modules have their own import namespace; their parent's use
        // declarations are not implicitly inherited.
        if let Some((_, items)) = &item.content {
            for item in items {
                if let syn::Item::Use(item) = item {
                    collect_use(&item.tree, &[], &mut self.imports);
                }
            }
        }
        if item.attrs.iter().any(|attr| {
            attr.path().is_ident("cfg")
                && attr
                    .parse_args::<syn::Ident>()
                    .is_ok_and(|ident| ident == "test")
        }) {
            let production = self.production;
            self.production = false;
            visit::visit_item_mod(self, item);
            self.production = production;
        } else {
            visit::visit_item_mod(self, item);
        }
        self.imports = previous;
    }
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if self.production
            && let syn::Expr::Path(function) = call.func.as_ref()
        {
            let name = self.resolved(&function.path).join("::");
            if call.args.len() == 1
                && let Some(syn::Expr::Lit(argument)) = call.args.first()
                && let syn::Lit::Str(argument) = &argument.lit
            {
                if matches!(name.as_str(), "std::env::var" | "std::env::var_os")
                    && argument.value() == "AGENT"
                {
                    self.result.agent = true;
                }
                if self.is_clap(&function.path, "new") && name.ends_with("::Command::new") {
                    self.result.clap = true;
                    if argument.value() == "skill" {
                        self.result.skill_command = true;
                    }
                }
            }
        }
        visit::visit_expr_call(self, call);
    }
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if self.production && call.method == "help_template" && call.args.len() == 1 {
            self.result.help_template = true;
        }
        visit::visit_expr_method_call(self, call);
    }
    fn visit_lit_str(&mut self, literal: &'ast syn::LitStr) {
        if self.production {
            self.result.tree_branch |= literal.value().contains('├');
            self.result.tree_end |= literal.value().contains(['╰', '└']);
        }
    }
    fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
        if self.production
            && attribute.path().is_ident("derive")
            && let Ok(paths) = attribute.parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            )
        {
            self.result.clap |= paths.iter().any(|path| self.is_clap(path, "Parser"));
        }
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if self.production
            && mac.path.is_ident("include_str")
            && let Ok(literal) = syn::parse2::<syn::LitStr>(mac.tokens.clone())
            && let Ok(path) = crate::options::absolute(
                &self
                    .source
                    .parent()
                    .unwrap_or(self.target)
                    .join(literal.value()),
            )
            && let Ok(relative) = path.strip_prefix(self.target)
            && relative.starts_with("skills")
            && path.file_name().is_some_and(|name| name == "SKILL.md")
        {
            match super::regular(&path) {
                Ok(exists) => self.result.skill_file |= exists,
                Err(error) => self.error = Some(error),
            }
        }
        if let Ok(arguments) =
            syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
                .parse2(mac.tokens.clone())
        {
            for argument in &arguments {
                self.visit_expr(argument);
            }
        }
    }
}
