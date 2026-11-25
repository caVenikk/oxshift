pub mod brand;

use ui::icons::{IconName, IconSvg};
use workbench::Product;

/// A desktop workbench with the platform's editor, project tree, Git and terminal.
pub fn product() -> Product {
    let product = Product {
        name: brand::INFO.name,
        app_info: brand::INFO,
        icons: vec![IconSvg {
            name: "product/platform",
            light: brand::ICON,
            dark: None,
        }],
        welcome: workbench::welcome::WelcomeHook {
            icon: Some(IconName("product/platform")),
            ..Default::default()
        },
        ..Product::default()
    };
    syntax_pack::with_shared_syntax(product, sql_highlight::file_type)
}

/// Start the standalone desktop using the same folder/file arguments as the workbench.
pub fn launch() {
    app_info::install(brand::INFO);
    // Windows sends Ctrl+C through this executable before any UI starts.
    process::run_interrupt_helper_if_requested();
    process::set_interrupt_helper(process::InterruptHelper::current_exe());
    let paths = std::env::args_os().skip(1).map(Into::into).collect();
    workbench::run(product(), paths);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standalone_has_no_language_product_capabilities() {
        let product = product();
        let file_types = workbench::file_types::FileTypes::new(&product.file_types);
        for (file, expected) in [("demo.py", "python"), ("query.sql", "sql")] {
            let ty = file_types.for_path(std::path::Path::new(file)).unwrap();
            assert_eq!(ty.language_id, Some(expected));
            assert!(ty.highlighter.is_some());
            assert!(ty.key_context.is_none());
            assert!(ty.context_menu.is_none());
        }
        assert!(product.run.is_none());
        assert!(product.debug.is_none());
        assert!(product.code_intel.is_empty());
        assert!(product.status_widgets.is_empty());
        assert!(product.settings_pages.is_empty());
        assert!(product.terminal.is_none());
        assert!(product.new_items.is_empty());
        assert!(product.tool_windows.is_empty());
        assert_eq!(product.app_info.profile_dir, "oxshift-platform");
        assert_eq!(product.app_info.bundle_id, "dev.oxshift.platform");
    }
}
