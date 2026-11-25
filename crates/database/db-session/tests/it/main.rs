//! Sessions on real databases. SQLite runs in every test run; PostgreSQL
//! and Oracle tests are ignored and need the containers of
//! `docker-compose.yml` (see docs/testing-databases.md):
//!
//! ```text
//! docker compose up -d postgres oracle
//! cargo nextest run -p db-session --run-ignored all
//! ```
//!
//! Hosts, ports and the password come from `OXSHIFT_TEST_PG_HOST`,
//! `OXSHIFT_TEST_PG_PORT` (default `55432`), `OXSHIFT_TEST_ORACLE_HOST`,
//! `OXSHIFT_TEST_ORACLE_PORT` (default `1521`),
//! `OXSHIFT_TEST_ORACLE_SERVICE` (default `FREEPDB1`) and
//! `OXSHIFT_DB_PASSWORD` (default `oxshift`).

#[path = "../../../../../tests/support/product_env.rs"]
mod product_env;

mod oracle;
mod pager;
mod postgres;
mod refresh;
mod sqlite;
mod support;
mod transfer;
