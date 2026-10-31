use crate::games;
use serde::Serialize;
use std::{collections::BTreeSet, fs, os::unix::fs::MetadataExt, process::Command};

#[derive(Clone, Default, Serialize)]
pub struct Runtime {
    pub game: Option<String>,
    pub message: String,
}
pub fn application_phase(requested: bool, blocked: bool, failed: bool) -> &'static str {
    if failed {
        "error"
    } else if requested && blocked {
        "waiting"
    } else if requested {
        "applying"
    } else {
        "idle"
    }
}
pub fn identify_titles(titles: &[String]) -> Option<String> {
    let mut ids = BTreeSet::new();
    let normalize = |s: &str| {
        s.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    };
    for title in titles {
        let words: Vec<_> = title.split(|c: char| !c.is_alphanumeric()).collect();
        let catalog = games::catalog();
        let exact: Vec<_> = catalog
            .iter()
            .filter(|g| words.contains(&g.id.as_str()))
            .collect();
        if !exact.is_empty() {
            ids.extend(exact.into_iter().map(|g| g.id.clone()));
            continue;
        }
        let title = format!(" {} ", normalize(title));
        let mut matches: Vec<_> = catalog
            .iter()
            .filter(|g| title.contains(&format!(" {} ", normalize(&g.title))))
            .collect();
        matches.sort_by_key(|g| std::cmp::Reverse(g.title.len()));
        if let Some(g) = matches.first() {
            ids.insert(g.id.clone());
        }
    }
    if ids.len() == 1 {
        ids.into_iter().next()
    } else {
        None
    }
}

fn xprop(args: &[&str]) -> Option<String> {
    let out = Command::new("/usr/bin/timeout")
        .args(["0.15", "/usr/bin/xprop"])
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() || out.stdout.len() > 128_000 {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}
pub fn detect() -> Runtime {
    let uid = fs::metadata("/proc/self")
        .map(|m| m.uid())
        .unwrap_or(u32::MAX);
    let mut pids = BTreeSet::new();
    if let Ok(dirs) = fs::read_dir("/proc") {
        for e in dirs.flatten() {
            if !e
                .file_name()
                .to_string_lossy()
                .bytes()
                .all(|b| b.is_ascii_digit())
                || !fs::metadata(e.path()).is_ok_and(|m| m.uid() == uid)
            {
                continue;
            }
            let name = fs::read_to_string(e.path().join("comm")).unwrap_or_default();
            if !["dolphin-emu", "melonDS", "melonds"].contains(&name.trim()) {
                continue;
            }
            pids.insert(e.file_name().to_string_lossy().into_owned());
            if let Ok(status) = fs::read_to_string(e.path().join("status"))
                && let Some(line) = status.lines().find(|l| l.starts_with("NSpid:"))
            {
                pids.extend(line.split_whitespace().skip(1).map(str::to_owned));
            }
        }
    }
    if pids.is_empty() {
        return Runtime {
            game: None,
            message: "No Dolphin or melonDS session running".into(),
        };
    }
    let Some(windows) = xprop(&["-root", "_NET_CLIENT_LIST"]) else {
        return Runtime { game: None, message: "Emulator open; live game detection needs an X11 session and xprop. Choose the game manually.".into() };
    };
    if windows
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|w| w.starts_with("0x"))
        .count()
        > 64
    {
        return Runtime {
            game: None,
            message: "Too many windows to identify the game safely; choose a game manually.".into(),
        };
    }
    let mut titles = Vec::new();
    for window in windows
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|w| w.starts_with("0x"))
        .take(64)
    {
        let Some(properties) = xprop(&[
            "-id",
            window,
            "_NET_WM_PID",
            "WM_CLASS",
            "_NET_WM_NAME",
            "WM_NAME",
        ]) else {
            continue;
        };
        let pid = properties
            .lines()
            .find(|l| l.starts_with("_NET_WM_PID(CARDINAL)"))
            .and_then(|l| l.split('=').nth(1))
            .map(str::trim);
        if !pid.is_some_and(|p| pids.contains(p)) {
            continue;
        }
        let class = properties
            .lines()
            .find(|l| l.starts_with("WM_CLASS("))
            .unwrap_or("")
            .to_lowercase();
        if !class.contains("dolphin") && !class.contains("melonds") {
            continue;
        }
        titles.extend(
            properties
                .lines()
                .filter(|l| l.starts_with("_NET_WM_NAME(") || l.starts_with("WM_NAME("))
                .map(str::to_owned),
        );
    }
    let game = identify_titles(&titles);
    Runtime {
        message: if game.is_some() {
            "Game identified from emulator window · editing selection only; active controls are not verified".into()
        } else {
            "Emulator open; game unknown or multiple games detected. Choose a game to edit.".into()
        },
        game,
    }
}
