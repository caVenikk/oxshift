# 0023. Release distribution without paid signing

- Status: accepted
- Date: 2026-11-25

## Context

Issue #94 requires distributable desktop builds without paid Apple Developer ID
or Windows code-signing certificates. Two products share version 2026.1 but have
separate executables, artwork, profiles and macOS bundle identifiers. Packaging
must be testable before any tag or public release exists.

## Decision

- Package both products on native macOS arm64/x64, Linux arm64/x64 and Windows
  x64 runners. Formats are DMG and tar archives on macOS, tar/Debian/AppImage on
  Linux, and installer/ZIP on Windows. Windows ARM64 and macOS universal builds
  are outside the current matrix.
- Keep artifact construction in a read-only reusable workflow. Preview actions
  artifacts and release publication are separate operations. The tag workflow
  gates on the exact commit's successful CI and creates a draft only; the
  maintainer accepts native builds before tagging and publishes the draft.
- macOS uses ad hoc signing by default, with explicit stable identifiers. The
  identifier-only designated requirement is weaker than a certificate-backed
  requirement: another ad hoc program can use the same identifier. It is not a
  trusted developer identity or notarization. Local certificate signing is an
  optional packaging argument. Windows packages are unsigned.
- Explain Gatekeeper, SmartScreen and enterprise policy in the release guide.
  Provide checksum-checked tar installers and rendered Homebrew formulas, but
  do not automatically publish a tap or rely on quarantine bypass as trust.
- Include pinned ConPTY in both Windows products; debugpy only in the Python
  product. Include license notices, canonical icons and SHA-256 checksums.
  AppImage build tool and embedded runtime are both explicitly pinned.
- ZIP carries the portable marker and memory-only secrets seed. Installers have
  no portable marker. Runtime support owns the path routing, while projects,
  external tools and OS-managed data remain outside the portable guarantee.

## Consequences

Native fixtures prove packaging structure, not installation or GUI behavior.
All native targets and formats require build/installation acceptance before a
release. Linux AppImages still require compatible system graphics libraries.
A future paid certificate/notarization path changes trust and keychain identity;
it requires a separate acceptance pass. See [release.md](../release.md) and
[code-signing.md](../code-signing.md) for operational details.
