# Third-party notices

Kurage Aquarium's Rust source is MIT licensed; see LICENSE.

The embedded Braille coverage masks and the test font come from DejaVu Sans.
DejaVu changes are public domain; the underlying Bitstream Vera font license
is reproduced in `assets/render/LICENSE-DejaVu.txt` (also shipped as
`licenses/DejaVu.txt` in binary archives).

Custom font rendering uses FreeType, compiled into the executables.
Portions of this software are copyright (c) The FreeType Project
(https://www.freetype.org/). All rights reserved. The FreeType Project
License is reproduced in `assets/render/LICENSE-FreeType.txt` (also shipped
as `licenses/FreeType.txt`). The freetype-rs and freetype-sys bindings are MIT
licensed. Cargo.lock records the versions of the other Rust dependencies.

The default glyph masks and two scene palettes were generated from the
Python v0.3.0 renderer (`c5d795a`) using `tools/oracle.py`. They are embedded
data, not Python code. Neither building nor running the generator invokes
Python, NumPy, or Pillow.
