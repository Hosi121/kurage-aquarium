# WezTerm background on macOS and Linux

Use the pre-rendered aquarium with WezTerm. Playback needs WezTerm, the GIF, and
the Lua configuration files. Generating a new animation uses the native Rust
generator and FFmpeg; Python is not needed.

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
[v0.3.0 release](https://github.com/Hosi121/kurage-aquarium/releases/tag/v0.3.0).
In the folder containing the download:

```sh
tar -xzf kurage-wezterm.tar.gz
mkdir -p "$HOME/.config/kurage"
cp kurage-wezterm/aquarium.gif kurage-wezterm/kurage-background.lua kurage-wezterm/wezterm.lua "$HOME/.config/kurage/"
```

The bundle contains the full 1280 × 720, 30 fps, 38.4-second aquarium, including
the giant foreground swimmer. The smaller GIF in the repository's `assets/`
folder is a 640 × 360, 15 fps preview.

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

### Scrolling in Codex recalls previous prompts

If the wheel changes the input text instead of scrolling the conversation,
the terminal may be translating wheel events into Up/Down keys. WezTerm does
this in the alternate screen when the application has not enabled mouse
reporting. The alternate screen also has no terminal scrollback; see the
[WezTerm wheel behavior reference](https://wezterm.org/config/lua/config/alternate_buffer_wheel_scroll_speed.html).

Run Codex in inline mode to keep the terminal's scrollback:

```sh
codex --no-alt-screen
```

To reopen a saved conversation, finish the current turn, exit Codex, then run:

```sh
codex resume --no-alt-screen
```

Select the conversation you want to continue. Using the picker avoids selecting
another chat when several Codex sessions are running.

To make this the default, set the following in Codex's `~/.codex/config.toml`.
If a `[tui]` table already exists, add the key to that table instead of adding
a second one:

```toml
[tui]
alternate_screen = "never"
```

This takes effect when Codex starts again. See the
[official Codex TUI options](https://learn.chatgpt.com/docs/config-file/config-advanced#tui-options).
The GIF background remains active in inline mode. The Kurage background module
only sets image layers and does not change mouse or keyboard bindings.

If you are still using the early tmux sidebar setup, mouse handling must also
be enabled in that tmux session to scroll its history. The current GIF setup
can be used directly without the sidebar launcher.

### Background setup

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
