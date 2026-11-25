//! Syntax-only file types shared by desktop products. Language tools are
//! registered separately by the product; this pack starts no processes.

use editor::{SyntaxHighlighter, WrapCondition, WrapRule};
use std::sync::Arc;
use ui::icons::{IconName, IconSvg};
use workbench::{FileType, Product};

/// Installs the shared syntax defaults after product modules have registered
/// their enhanced file types. Existing types and icons win by name, and their
/// order and file associations stay unchanged. The SQL factory is supplied by
/// the product so the language pack does not depend on the database module.
pub fn with_shared_syntax(mut product: Product, sql_default: impl FnOnce() -> FileType) -> Product {
    let mut missing: Vec<_> = file_types()
        .into_iter()
        .filter(|default| !product.file_types.iter().any(|ty| ty.name == default.name))
        .collect();
    if !product.file_types.iter().any(|ty| ty.name == "SQL") {
        missing.push(sql_default());
    }
    missing.append(&mut product.file_types);
    product.file_types = missing;

    let mut missing_icons: Vec<_> = icons()
        .into_iter()
        .filter(|default| !product.icons.iter().any(|icon| icon.name == default.name))
        .collect();
    missing_icons.append(&mut product.icons);
    product.icons = missing_icons;
    product
}

/// File types with the grammars shipped by this pack.
pub fn file_types() -> Vec<FileType> {
    vec![python()]
}

/// Icons required by this pack's file types.
pub fn icons() -> Vec<IconSvg> {
    vec![IconSvg {
        name: "python/file",
        light: include_bytes!("../assets/icons/python.svg"),
        dark: Some(include_bytes!("../assets/icons/python_dark.svg")),
    }]
}

/// Python source files (and `#!` scripts naming Python).
fn python() -> FileType {
    FileType {
        hash_bangs: &["python"],
        // A block opens after a colon at the end of a line and ends after
        // a statement that leaves it.
        indent_after: &[':'],
        dedent_after: &["return", "pass", "break", "continue", "raise"],
        line_comment: Some("# "),
        quotes: &['"', '\''],
        triple_quotes: true,
        // String prefixes: raw, bytes, formatted, template, unicode.
        string_prefixes: &['r', 'R', 'b', 'B', 'f', 'F', 't', 'T', 'u', 'U'],
        highlighter: Some(Arc::new(SyntaxHighlighter::factory(py_syntax::language()))),
        language_id: Some("python"),
        wrap_rules: PYTHON_WRAP_RULES,
        ..FileType::plain("Python", &["py", "pyi", "pyw"], IconName("python/file"))
    }
}

/// Where a soft-wrapped Python line may wrap when no space is near: after
/// a comma (preferred a little) or an opening bracket, around spaces and
/// tabs (the reference's Python line wrap strategy).
const PYTHON_WRAP_RULES: &[WrapRule] = &[
    WrapRule::weighted(
        ',',
        WrapCondition::After,
        WrapRule::DEFAULT_WEIGHT * 11 / 10,
    ),
    WrapRule::new(' ', WrapCondition::Both),
    WrapRule::new('\t', WrapCondition::Both),
    WrapRule::new('(', WrapCondition::After),
    WrapRule::new('[', WrapCondition::After),
    WrapRule::new('{', WrapCondition::After),
];

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    use gpui as _;
    use std::path::Path;
    use workbench::file_types::{Detected, FileTypes};

    #[test]
    fn shared_file_icons_rasterize() {
        for icon in icons() {
            assert!(ui::icons::rasterize(icon.light, 32).is_some());
            assert!(ui::icons::rasterize(icon.dark.unwrap(), 32).is_some());
        }
    }

    #[test]
    fn python_syntax_is_available_without_python_tools() {
        let pack = file_types();
        let types = FileTypes::with_associations(&pack, &[]);
        for name in ["main.py", "types.pyi", "window.pyw"] {
            let file_type = types.for_path(Path::new(name)).unwrap();
            assert_eq!(file_type.language_id, Some("python"));
            assert!(file_type.highlighter.is_some());
            assert!(file_type.triple_quotes);
            assert_eq!(file_type.indent_after, &[':']);
            assert!(icons().iter().any(|icon| icon.name == file_type.icon.0));
        }
        assert_eq!(
            types.content_type(b"#!/usr/bin/env python3\n"),
            Detected::Type("Python")
        );
    }
}
