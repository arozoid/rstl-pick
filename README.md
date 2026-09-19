# rstl-pick

compact fuzzy-search TUI picker for launching the terminal apps of the
[`rstl.sway`](https://github.com/arozoid/rstl.sway) rice.

type to filter, arrows / `j`/`k` to move, `enter` to run. the chosen app
launches in-place and the picker returns when it exits. pressing the letter
shown next to an entry (e.g. `n` for network, `e` for calculator) launches
that entry directly. entries whose program is not installed on the system are
hidden from the list, and their key does nothing.

picks between:

- `nmtui`: network
- `wiremix`: audio
- `clipse`: clipboard
- `bluetui`: bluetooth
- `latuicon`: icons (the picked icon is copied to the clipboard via
  `wl-copy`, falling back to `xclip`/`xsel`)
- `spf` / `rovr` / `lf`: files (runs the first of them that is installed,
  in that order; `spf` is the superfile binary)
- `eva`: calculator