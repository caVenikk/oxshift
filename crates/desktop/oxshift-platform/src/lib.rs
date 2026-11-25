pub mod brand;

use ui::icons::{IconName, IconSvg};
use workbench::Product;

/// A desktop workbench with the platform's editor, project tree, Git and terminal.
pub fn product() -> Product {
    let mut icons = syntax_pack::icons();
    icons.push(IconSvg {
        name: "product/platform",
        light: brand::ICON,
        dark: None,
    });
    let mut file_types = syntax_pack::file_types();
    file_types.push(sql_highlight::file_type());
    Product {
        name: brand::INFO.name,
        app_info: brand::INFO,
        icons,
        file_types,
        welcome: workbench::welcome::WelcomeHook {
            icon: Some(IconName("product/platform")),
            ..Default::default()
        },
        ..Product::default()
    }
}

/// Start the standalone desktop using the same folder/file arguments as the workbench.
pub fn launch() {
    app_info::install(brand::INFO);
    let paths = std::env::args_os().skip(1).map(Into::into).collect();
    workbench::run(product(), paths);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standalone_has_no_language_product_capabilities() {
        let product = product();
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
