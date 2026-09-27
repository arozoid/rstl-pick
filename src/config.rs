use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

/// Keys the picker itself uses, so a menu cannot bind them.
const RESERVED_KEYS: [char; 3] = ['q', 'j', 'k'];

/// A single row in the menu, ready for the picker to use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuEntry {
    pub label: String,
    pub key: char,
    pub command: String,
    pub hint: String,
    /// When true, the entry's stdout is copied to the clipboard instead of
    /// launching interactively (the "icons" row).
    pub clip_stdout: bool,
    /// When set, the row hides unless at least one of these is on PATH.
    pub needs: Option<Vec<String>>,
}

impl MenuEntry {
    /// Whether the row should be shown, given what is installed.
    pub fn available(&self) -> bool {
        match &self.needs {
            None => true,
            Some(tools) => tools.iter().any(|tool| on_path(tool)),
        }
    }
}

/// Raw shape of the config file, as deserialized straight from TOML.
#[derive(Debug, Deserialize)]
struct ConfigFile {
    /// How many rows the picker shows at once before the list scrolls.
    #[serde(default = "default_display")]
    display: usize,
    entries: Vec<EntryToml>,
}

#[derive(Debug, Deserialize)]
struct EntryToml {
    label: String,
    #[serde(default)]
    key: String,
    command: String,
    #[serde(default)]
    hint: String,
    #[serde(default)]
    mode: String,
    #[serde(default)]
    needs: Vec<String>,
}

/// A parsed menu: the entries plus the picker's presentation settings.
pub struct Menu {
    pub entries: Vec<MenuEntry>,
    /// How many rows the picker shows at once before the list scrolls.
    pub display: usize,
}

/// Default for `display` when the config does not set it. Keeps the
/// historical panel: search box + notice + 9 list rows.
fn default_display() -> usize {
    9
}

/// Parse and validate the config text into menu entries.
pub fn parse(source: &str) -> Result<Menu, String> {
    let file: ConfigFile = toml::from_str(source).map_err(|e| format!("bad config: {e}"))?;

    if file.entries.is_empty() {
        return Err("config has no entries".into());
    }

    let display = file.display;
    if display == 0 {
        return Err("display must be at least 1".into());
    }

    let mut seen = HashSet::new();
    let mut entries = Vec::with_capacity(file.entries.len());
    for e in file.entries {
        let label = e.label.trim().to_string();
        if label.is_empty() {
            return Err("every entry needs a non-empty label".into());
        }

        let mut chars = e.key.chars();
        let key = match (chars.next(), chars.next()) {
            (Some(c), None) if !c.is_control() => c,
            _ => return Err(format!("entry \"{label}\": key must be a single character")),
        };
        if RESERVED_KEYS.contains(&key) {
            return Err(format!(
                "entry \"{label}\": key '{key}' is reserved for routing the picker itself"
            ));
        }
        if !seen.insert(key) {
            return Err(format!("two entries share the key '{key}'"));
        }

        let command = e.command.trim().to_string();
        if command.is_empty() {
            return Err(format!("entry \"{label}\": command must not be empty"));
        }

        let hint = e.hint.trim().to_string();
        let hint = if hint.is_empty() {
            command.clone()
        } else {
            hint
        };

        let clip_stdout = match e.mode.trim() {
            "" | "normal" => false,
            "clipboard" => true,
            other => {
                return Err(format!(
                    "entry \"{label}\": unknown mode \"{other}\" (use \"normal\" or \"clipboard\")"
                ))
            }
        };

        let needs = e.needs;
        if !needs.is_empty() {
            for tool in &needs {
                if tool.trim().is_empty() {
                    return Err(format!(
                        "entry \"{label}\": needs list contains an empty tool"
                    ));
                }
            }
        }
        let needs = if needs.is_empty() { None } else { Some(needs) };

        entries.push(MenuEntry {
            label,
            key,
            command,
            hint,
            clip_stdout,
            needs,
        });
    }
    Ok(Menu { entries, display })
}

/// The default menu, written out when no config file exists yet.
pub fn default_toml() -> &'static str {
    r#"# rstl-pick menu configuration.
#   display  how many rows the picker shows at once before it scrolls (default 9)
# Each [[entries]] block is one row in the picker:
#   label   shown in the list
#   key     one character; press it in the picker to launch this entry
#   command shell command run when the row is picked
#   hint    short text next to the label (optional, defaults to the command)
#   mode    "normal" runs the command, "clipboard" copies its stdout instead
#   needs   optional list of programs; the row hides unless one is installed

display = 9

[[entries]]
label = "network"
command = "nmtui"
key = "n"
needs = ["nmtui"]

[[entries]]
label = "audio"
command = "wiremix"
key = "a"
needs = ["wiremix"]

[[entries]]
label = "clipboard"
command = "clipse"
key = "c"
needs = ["clipse"]

[[entries]]
label = "bluetooth"
command = "bluetui"
key = "b"
needs = ["bluetui"]

[[entries]]
label = "icons"
command = "latuicon"
key = "i"
mode = "clipboard"
needs = ["latuicon"]

[[entries]]
label = "files"
command = "for m in spf rovr lf; do command -v \"$m\" >/dev/null 2>&1 || continue; \"$m\"; break; done"
key = "f"
hint = "spf / rovr / lf"
needs = ["spf", "rovr", "lf"]

[[entries]]
label = "calculator"
command = "eva"
key = "e"
needs = ["eva"]

[[entries]]
label = "music"
command = "for m in kew climp; do command -v \"$m\" >/dev/null 2>&1 || continue; \"$m\"; break; done"
key = "m"
hint = "kew / climp"
needs = ["kew", "climp"]

[[entries]]
label = "calendar"
command = "chroncal"
key = "d"
needs = ["chroncal"]
"#
}

/// The config path given on the command line, if any.
pub fn config_arg() -> Result<Option<String>, String> {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-C" | "--config" => {
                return args
                    .next()
                    .map(Some)
                    .ok_or("--config needs a path after it".into())
            }
            other if other.starts_with("--config=") => {
                return Ok(Some(other["--config=".len()..].to_string()))
            }
            other if other.starts_with("-C=") => return Ok(Some(other["-C=".len()..].to_string())),
            _ => {}
        }
    }
    Ok(None)
}

/// Resolve the config file: the -C path, or the default under the user's
/// config directory.
pub fn config_path(arg: Option<String>) -> PathBuf {
    if let Some(path) = arg {
        return PathBuf::from(path);
    }
    let base = env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("HOME").expect("HOME is not set");
            PathBuf::from(home).join(".config")
        });
    base.join("rstl-pick").join("config.toml")
}

/// Missing config path: write the default menu there, then return its entries.
pub fn ensure_default(path: &Path) -> Result<Menu, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    fs::write(path, default_toml()).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    parse(default_toml())
}

/// Whether `tool` is present on PATH.
fn on_path(tool: &str) -> bool {
    let tool = tool.trim();
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {tool} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_config() {
        let menu = parse(default_toml()).unwrap();
        assert_eq!(menu.display, 9);
        let entries = &menu.entries;
        assert_eq!(entries.len(), 9);
        assert_eq!(entries[0].key, 'n');
        assert_eq!(entries[5].label, "files");
        assert!(!entries[5].available() || entries[5].needs.is_some());
    }

    #[test]
    fn parses_custom_display() {
        let src = default_toml().replacen("display = 9", "display = 4", 1);
        let menu = parse(&src).unwrap();
        assert_eq!(menu.display, 4);
    }

    #[test]
    fn rejects_zero_display() {
        let src = default_toml().replacen("display = 9", "display = 0", 1);
        assert!(parse(&src).is_err());
    }

    #[test]
    fn rejects_duplicate_keys() {
        let src = default_toml().replacen("key = \"n\"", "key = \"x\"", 1);
        let src = src.replacen("key = \"a\"", "key = \"x\"", 1);
        assert!(parse(&src).is_err());
    }

    #[test]
    fn rejects_reserved_key() {
        let src = default_toml().replacen("key = \"n\"", "key = \"q\"", 1);
        assert!(parse(&src).is_err());
    }

    #[test]
    fn rejects_empty_entries() {
        assert!(parse("").is_err());
    }

    #[test]
    fn hints_default_to_command() {
        let src = r#"
[[entries]]
label = "test"
command = "echo hi"
key = "t"
"#;
        let menu = parse(src).unwrap();
        assert_eq!(menu.entries[0].hint, "echo hi");
    }
}
