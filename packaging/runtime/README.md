# Runtime third-party license bundle

`THIRD-PARTY-LICENSES.txt` is a shipped release asset for both desktop products
and their companion CLIs. It covers the union of macOS ARM64/x64, Linux ARM64/x64
and Windows x64. `cargo-about` resolves the normal dependency graph for the four
release packages with default features. Dependencies used exclusively for build
scripts or tests are excluded. Native code distributed inside runtime crates is
covered by their original recursive LICENSE, COPYING, COPYRIGHT and NOTICE files.

The report retains package version, declared SPDX expression, repository and
published authors. `cargo-about` chooses an accepted OR alternative and retains
all required AND terms. Its SPDX fallback text is supplemented with the original
crate distribution documents and pinned upstream documents in `upstream/`.
Some upstreams provide authors and an SPDX declaration without a separate
copyright notice; the report preserves those declarations rather than inventing
copyright years or owners. Supplementary historical/current upstream documents
are identified by immutable URL; they do not override the package declaration.
MPL source repositories and package versions are identified in the report.

## Regenerate

Use Cargo with the committed lockfile and Python 3.9 or newer:

```sh
cargo install cargo-about --version 0.9.3 --locked --features cli
python3 scripts/runtime-notices.py generate
python3 scripts/runtime-notices.py check
python3 scripts/test-package.py
```

Generation may fetch registry metadata and source packages. Review changes to
package scope, declared licenses and attribution documents before committing the
updated bundle and manifest. New upstream supplements must use official project
sources and immutable revision URLs; record their SHA-256 in `upstream/sources.json`.
`original_sha256` hashes the downloaded bytes, while `sha256` hashes the stored
text after trailing-space and final-newline normalization. License wording is
preserved. Do not add private paths or replace original copyright notices.

Normal package builds and `package.py check` are offline: they check the bundle
SHA-256 and the Cargo.lock, manifests, generator, configuration and supplement
hashes. A dependency change requires regeneration. Every package includes the
same reviewed bundle; archive and DMG verification compares its exact bytes.
Existing desktop CI runs these checks through `package.py`, without a separate
download or license-generation step.
