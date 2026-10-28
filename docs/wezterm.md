# WezTerm background on macOS and Linux

Use the pre-rendered aquarium with WezTerm. Playback needs WezTerm, the GIF, and
the Lua configuration files. Python, NumPy, Pillow, and FFmpeg are only needed
when generating a new animation.

## Install WezTerm

On macOS, with Homebrew:

```sh
brew install --cask wezterm
```

The [official macOS guide](https://wezterm.org/install/macos.html) also describes
installation without Homebrew. If the `wezterm` command is not in your PATH,
use `/Applications/WezTerm.app/Contents/MacOS/wezterm` in the commands below.

On Ubuntu and other Linux distributions, follow the
[official Linux guide](https://wezterm.org/install/linux.html) for a native
package or AppImage. When using an AppImage, substitute its executable path for
`wezterm` below. Installing WezTerm does not require changing your default
terminal application.

## Download the background bundle

Download `kurage-wezterm.tar.gz` from the
[v0.2.0 release](https://github.com/Hosi121/kurage-aquarium/releases/tag/v0.2.0).
In the folder containing the download:

```sh
tar -xzf kurage-wezterm.tar.gz
mkdir -p "$HOME/.config/kurage"
cp kurage-wezterm/aquarium.gif kurage-wezterm/kurage-background.lua kurage-wezterm/wezterm.lua "$HOME/.config/kurage/"
```

The bundle contains the full 1280 × 720, 30 fps, 38.4-second aquarium. The GIF is
losslessly optimized to 9.41 MiB. It has the same decoded frames and timing as
the normal export. The smaller GIF in the repository's `assets/` folder is a
640 × 360, 15 fps preview.

If you already generated your own GIF, copy it to
`~/.config/kurage/aquarium.gif` and copy the two Lua files from `examples/`
instead of downloading the bundle.

## Try it in a separate window

```sh
wezterm --config-file "$HOME/.config/kurage/wezterm.lua" start --always-new-process
```

This launches WezTerm using the supplied configuration without writing to your
existing WezTerm configuration. The aquarium is centered, keeps its aspect
ratio, and fills the window without tiling. Some of the image can be cropped
when the window has a different aspect ratio. Image opacity is 70%, over a
dark blue background.

WezTerm animates GIF backgrounds while the window has focus. When you switch
to another application, the animation may pause; this is WezTerm behavior.
See the [background reference](https://wezterm.org/config/lua/config/background.html).

## Add it to an existing configuration

Find your active configuration: usually `~/.wezterm.lua`, or
`~/.config/wezterm/wezterm.lua`. WezTerm also honors `XDG_CONFIG_HOME` and
explicit configuration-file overrides; see the
[configuration guide](https://wezterm.org/config/files.html).

Add this line before the existing `return config`:

```lua
dofile(wezterm.home_dir .. '/.config/kurage/kurage-background.lua').apply(config)
```

Keep your existing `local wezterm = require 'wezterm'`, configuration table,
fonts, colors, key bindings, and final return statement. The module replaces
only `config.background`. If your table has another name, use that name in
place of `config`.

For a first configuration, use:

```lua
local wezterm = require 'wezterm'
local config = wezterm.config_builder()
dofile(wezterm.home_dir .. '/.config/kurage/kurage-background.lua').apply(config)
return config
```

WezTerm reloads configuration changes automatically. You can also press
Ctrl+Shift+R to reload them.

To use another GIF location, pass its absolute path as the second argument:

```lua
dofile(wezterm.home_dir .. '/.config/kurage/kurage-background.lua').apply(
  config, '/absolute/path/to/aquarium.gif'
)
```

To remove the background, remove the `apply` line. To adjust the image strength,
edit `opacity = 0.7` in `kurage-background.lua` and reload your configuration.

## Troubleshooting

- If WezTerm reports a missing Lua file or GIF, confirm that all three files
  are in `~/.config/kurage/`, on the computer running WezTerm. An SSH server's
  filesystem is not the local terminal's filesystem.
- If an application paints an opaque background over the aquarium, try
  `config.text_background_opacity = 0.85` in your configuration. This changes
  application background blending and can affect readability, so it is not
  applied by the module.
- If your existing configuration does not change, check which configuration
  file WezTerm loads. The separate-window command above selects it explicitly.
- GNOME Terminal and macOS Terminal are different applications. These Lua
  settings are loaded by WezTerm, not by those terminals.

The configuration is intended for macOS and Linux. Validation status is listed
in the repository README; native GUI playback on macOS remains unverified.
