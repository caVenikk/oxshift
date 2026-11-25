//! Database dialect context for the reusable SQL highlighter.

pub use sql_highlight::{SqlHighlighter, SqlLexer};

impl sql_highlight::DialectSource for super::hub::Hub {
    fn epoch(&self) -> u64 {
        self.epoch()
    }

    fn dialect(&self, path: &std::path::Path) -> sql_syntax::Dialect {
        self.dialect(path)
    }
}
