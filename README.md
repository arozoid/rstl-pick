# rstl-pick

compact fuzzy-search TUI picker for launching terminal apps of the
[`rstl.sway`](https://github.com/arozoid/rstl.sway) rice.

type to filter, arrows / `j`/`k` to move, `enter` to run. the chosen app
launches in-place and the picker returns when it exits. pressing the letter
shown next to an entry (e.g. `n` for network, `e` for calculator) launches
that entry directly. entries whose program is not installed on the system are
hidden from the list, and their key does nothing.

## configuration

the picker is defined by a TOML file; every launched program, key binding, and
even the menu itself is yours to personalize.

- default config: `${XDG_CONFIG_HOME:-~/.config}/rstl-pick/config.toml`
- override with `-C <path>` / `--config <path>` (also `-C=path` / `--config=path`)
- when the file does not exist, rstl-pick writes the default menu there and
  uses it; a malformed file is rejected with an error

each `[[entries]]` block is one row. the defaults (written on first run):

- `nmtui`: network
- `wiremix`: audio
- `clipse`: clipboard
- `bluetui`: bluetooth
- `latuicon`: icons (the picked icon is copied to the clipboard via
  `wl-copy`, falling back to `xclip`/`xsel`)
- `spf` / `rovr` / `lf`: files (runs the first of them that is installed,
  in that order; `spf` is the superfile binary)
- `eva`: calculator
- `kew`: music
- `chroncal`: calendar

top-level option (before the `[[entries]]` blocks):

| field     | meaning |
|-----------|---------|
| `display` | how many rows the picker shows at once before the list scrolls (default 9) |

fields, per entry:

| field     | meaning |
|-----------|---------|
| `label`   | shown in the list |
| `key`     | one character; press it in the picker to launch this entry (`q`/`j`/`k` are reserved) |
| `command` | shell command run when the row is picked |
| `hint`    | short text next to the label (optional, defaults to the command) |
| `mode`    | `"normal"` runs the command, `"clipboard"` copies its stdout to the clipboard instead |
| `needs`   | optional list of programs; the row hides unless one is installed |

example:

```toml
[[entries]]
label = "ping tool"
command = "gping"
key = "g"
needs = ["gping"]

[[entries]]
label = "password"
command = "pass show work/laptop"
key = "p"
mode = "clipboard"
```