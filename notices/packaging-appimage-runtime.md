- AppImage runtime embedded in Linux `.AppImage` packages, unmodified:
  https://github.com/AppImage/type2-runtime @
  dd6cebedcbddde9c82f89b011e8e1d40b6e43868 (release 20251108).
  MIT, Copyright (c) 2004-23 probonopd; its upstream license and linked
  dependency terms are in `packaging/licenses/appimage-runtime.txt`, copied
  into the packages. The upstream repository contains the runtime build
  scripts and pinned dependency sources. The build-only appimagetool 1.9.1
  comes from https://github.com/AppImage/appimagetool; it is not bundled.
  Both tool and runtime downloads are checked against pinned SHA-256 values
  in `scripts/fetch-appimage-tools.py`; no upstream code is adapted.
