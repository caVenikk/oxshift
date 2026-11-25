# Clean-room rules

Allowed sources:
1. Public documentation, specifications and protocols (LSP, DAP, Oracle data
   dictionary docs, Docker Engine API, Python docs).
2. Open-source code under compatible licenses (Apache-2.0, MIT, BSD), with a
   file in `notices/` for anything copied or closely adapted (one file per
   entry, see ADR 0019).
3. **Observed behaviour** of tools a contributor is licensed to use: run a
   scenario, record inputs and outputs, write them as tests under
   `tests/scenarios/`. Never copy text, icons, layout assets or strings.
4. Users' own configuration files (exported keymaps, colour schemes, live
   templates) as an import format. User themes and colour schemes (`.icls` XML,
   `.theme.json`, keymap and code-style XML) are plain data: the importer is
   written from documented formats and sample files. Plugin archives
   (`.jar`/zip) may be opened **only to extract such data resources**
   (including the plugin descriptor `META-INF/plugin.xml`, read to resolve a
   theme's id and name);
   `.class` files and anything executable are never opened, decompiled or analysed.

Forbidden:
- Decompiled, disassembled or otherwise extracted code from proprietary
  products, in any form (including "translated" or "paraphrased").
- Proprietary assets, trademarks, logos, strings or sound, except for the
  factual import-source identification described below.
- Pasting such material into prompts, issues, commits or docs.
- Committing third-party theme files whose license does not allow it. Keep them
  in the git-ignored `tests/fixtures/themes-local/`.

When behaviour is unclear, write the *question* and an observation script
(scenario -> expected output) in `tests/scenarios/`, never the internals of
the other tool.

## Visual fidelity
The UI aims to match the reference IDE's current UI closely, down to small
details, so its users keep their habits: layout, metrics, fonts, behaviour and
keyboard. Look and metrics are reproduced from observation of a licensed copy.
Icons, themes, metric constants and UI strings may be taken from the
open-source IntelliJ Platform Community code base (Apache-2.0), with a
`notices/` entry naming the source, commit and files. Assets, strings and code
of proprietary products and plugins (for example the database tools) are never
copied; their look is rebuilt from observation with original assets. Logos and
product names of reference products appear only under the narrow import-source
exception below.

## Factual import-source identification

An import UI may identify a supported source by its actual product name,
known edition and version. This is a statement of configuration compatibility,
not the application's identity or a claim of endorsement. Unknown folders retain
neutral labels and icons. Python imports only the PyCharm family; this exception
does not authorize unrelated product presets or product branding elsewhere.

An official icon may accompany a confirmed source only when the owner's public
terms permit this use. Obtain it from public brand assets, never an installed
proprietary binary; preserve the original file, proportions and colours, including
in dark themes and disabled states. Follow size, spacing, attribution and link
requirements. Record the source and applicable terms in a dedicated notice.
Use a neutral import icon when those conditions cannot be met. This exception
permits no proprietary code, screenshots, themes or other extracted assets.

The PyCharm source icon is covered by
[its notice](../notices/import-pycharm-brand.md), including the scope and limitations
of the public brand permission.
