# Oxshift product family

**Oxshift** is the language-independent platform. **Oxshift Python** is the first IDE. Product display names use the platform name followed by the language.

The platform and each language IDE have a distinct palette. The platform and Python IDE share a deep navy face, a full-bleed square gradient frame and a folded ribbon as the central mark. The inset has four identical rounded corners. On macOS the operating system shapes the outer corners.

| Product | Approved mark | Frame gradient samples | Light fold or highlight sample |
| --- | --- | --- | --- |
| Platform | Four equal rounded rays around an open four-point star | `#391EFD` → `#7F31FC` → `#E64FFD` | `#FDCEFE` |
| Python IDE | One S-shaped ribbon snake with a rounded tail, a mint head and one dark eye | `#002FFC` → `#00D1F7` → `#02F7CC` | `#C9FCF0` |
| Go IDE | Open | `#039247` → `#47D13D` → `#CEF52A` | `#B6FBBE` |
| Rust IDE | Open | `#D80538` → `#FA2F2A` → `#FD9411` | `#FEBE87` |
| C# IDE | Open | `#C70487` → `#E720A7` → `#FD4CBC` | `#FC92D0` |

The values are representative sRGB samples from the selected artwork. The platform face measures `#010D3E` in its dark interior. Only color is settled for Go, Rust and C#. Their symbols remain a future design decision.

## Application icon assets

The [platform mark](../assets/brand/oxshift.svg) identifies Oxshift itself. Use the [Python mark](../assets/brand/oxshift-python.svg) for the Python application's launcher and bundle. Each SVG fills its square canvas to the edges. The adjacent files provide a multi-resolution ICNS for macOS, a multi-resolution ICO for Windows and a 512-pixel PNG for Linux.

On macOS, compile the matching Icon Composer source ([platform](../assets/brand/Oxshift.icon) or [Python IDE](../assets/brand/OxshiftPython.icon)) and use the icon name emitted by the asset compiler. Each source references the same SVG as the other desktop exports.
