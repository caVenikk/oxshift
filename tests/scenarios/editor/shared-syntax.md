# Shared syntax without language tools

1. Launch each desktop product on a folder containing `sample.py`, `sample.pyi`,
   `sample.pyw` and `query.sql`, without configuring an interpreter or a database.
2. Open each Python file with a function, docstring, string escape and a comment.
   Expect the same syntax colours and Python indentation, quote and wrap rules.
3. Open SQL with `select name from customers where id = 42; -- note`.
   Expect keyword, identifier, number and comment colours without a connection.
4. Edit a Python function header into incomplete syntax. Subsequent valid lines
   retain their lexical colours; restoring the header restores structural roles.
5. In the Python product, assign a SQL dialect through Database. Expect the
   existing dialect-aware highlighting, execution and intelligence to remain.

The shared pack adds no interpreter, runner, debugger, language server, database
connection, execution command or SQL toolbar to the neutral desktop product.
JSON, shell and TOML remain recognised file types with no syntax grammar.
