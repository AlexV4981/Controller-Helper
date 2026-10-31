use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub path: String,
    pub content: String,
    pub note: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub id: String,
    pub changed: usize,
    pub files: Vec<String>,
}
#[derive(Serialize, Deserialize)]
struct Entry {
    path: String,
    before: Option<String>,
    after: String,
}
#[derive(Serialize, Deserialize)]
struct Backup {
    id: String,
    entries: Vec<Entry>,
}
const STATE: &str = ".local/state/controller-studio";
pub fn allowed(path: &str) -> bool {
    let p = Path::new(path);
    if p.components().any(|c| !matches!(c, Component::Normal(_))) {
        return false;
    }
    if [
        "io.github.punesemu.puNES",
        "net.kuribo64.melonDS",
        "org.azahar_emu.Azahar",
        "org.DolphinEmu.dolphin-emu",
        "info.cemu.Cemu",
    ]
    .iter()
    .any(|a| path == format!(".local/share/flatpak/overrides/{a}"))
    {
        return true;
    }
    if let Some(id) = path
        .strip_prefix(".var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/GameSettings/")
        .and_then(|s| s.strip_suffix(".ini"))
    {
        return id.len() == 6
            && id
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    }
    for game in crate::games::catalog().iter().filter(|g| g.system == "ds") {
        if crate::games::ds_config(&game.id).as_deref() == Ok(path) {
            return true;
        }
    }
    let pre = ".var/app/";
    [
        "io.github.punesemu.puNES/config/puNES/input.cfg",
        "net.kuribo64.melonDS/config/melonDS/melonDS.toml",
        "org.azahar_emu.Azahar/config/azahar-emu/qt-config.ini",
        "org.DolphinEmu.dolphin-emu/config/dolphin-emu/WiimoteNew.ini",
        "org.DolphinEmu.dolphin-emu/config/dolphin-emu/GCPadNew.ini",
        "org.DolphinEmu.dolphin-emu/config/dolphin-emu/Dolphin.ini",
        "org.DolphinEmu.dolphin-emu/config/dolphin-emu/GFX.ini",
    ]
    .iter()
    .any(|s| path == format!("{pre}{s}"))
        || [
            (
                ".var/app/io.github.punesemu.puNES/config/puNES/jsc/",
                ".jsc",
            ),
            (
                ".var/app/org.DolphinEmu.dolphin-emu/config/dolphin-emu/Profiles/GCPad/",
                ".ini",
            ),
            (
                ".var/app/org.DolphinEmu.dolphin-emu/config/dolphin-emu/Profiles/Wiimote/",
                ".ini",
            ),
            (
                ".var/app/info.cemu.Cemu/config/Cemu/controllerProfiles/",
                ".xml",
            ),
        ]
        .iter()
        .any(|(prefix, suffix)| {
            path.starts_with(prefix)
                && !path[prefix.len()..].contains('/')
                && path.ends_with(suffix)
        })
}
pub fn safe_path(home: &Path, rel: &str) -> Result<PathBuf, String> {
    if rel.is_empty()
        || Path::new(rel)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Invalid relative file path".into());
    }
    if fs::symlink_metadata(home)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("Home directory is a symlink".into());
    }
    let mut p = home.to_path_buf();
    for part in Path::new(rel).components() {
        p.push(part.as_os_str());
        match fs::symlink_metadata(&p) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(
                    "A configuration path contains a symlink; no files were changed".into(),
                );
            }
            Ok(m) if m.is_file() && p != home.join(rel) => {
                return Err("A parent path is not a directory".into());
            }
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(p)
}
pub fn read(home: &Path, rel: &str) -> Result<Option<String>, String> {
    let p = safe_path(home, rel)?;
    match fs::metadata(&p) {
        Ok(m) if m.len() > 2_000_000 => Err("Configuration exceeds safe size limit".into()),
        Ok(_) => fs::read_to_string(p).map(Some).map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
fn nonce() -> String {
    format!(
        "{}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        std::process::id()
    )
}
fn write_atomic(home: &Path, rel: &str, data: &[u8]) -> Result<(), String> {
    let p = safe_path(home, rel)?;
    let parent = p.parent().ok_or("Missing parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    safe_path(home, rel)?;
    let temp = parent.join(format!(".controller-studio-{}", nonce()));
    let result = (|| {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        f.write_all(data)
            .and_then(|_| f.sync_all())
            .map_err(|e| e.to_string())?;
        fs::rename(&temp, &p).map_err(|e| e.to_string())?;
        fs::File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(())
    })();
    if temp.exists() {
        let _ = fs::remove_file(&temp);
    }
    result
}
pub fn save_settings(home: &Path, settings: &crate::model::Settings) -> Result<(), String> {
    write_atomic(
        home,
        &format!("{STATE}/preferences.json"),
        &serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?,
    )
}
fn upgrade_sideways(b: &mut crate::model::Bindings) {
    if !b.contains_key("B") {
        b.insert("B".into(), "lt".into());
        if b.get("A").map(String::as_str) == Some("south")
            && b.get("2").map(String::as_str) == Some("south")
        {
            b.insert("A".into(), "east".into());
        }
    }
}
pub fn load_settings(home: &Path) -> Result<crate::model::Settings, String> {
    match read(home, &format!("{STATE}/preferences.json"))? {
        Some(s) => {
            let mut v: crate::model::Settings = serde_json::from_str(&s)
                .map_err(|e| format!("Saved preferences could not be read: {e}"))?;
            if v.applied_revision > v.desired_revision || v.desired_revision > 9_007_199_254_740_990
            {
                return Err("Invalid settings revision".into());
            }
            if v.pending_apply && v.desired_revision == v.applied_revision {
                v.desired_revision += 1;
            }
            if v.schema != 1 {
                return Err("Unsupported preference version".into());
            }
            if let Some(id) = &v.last_game {
                crate::games::find(id)?;
            }
            for (id, display) in &v.ds_display {
                if id != "shared" && crate::games::find(id)?.system != "ds" {
                    return Err("Not a DS display profile".into());
                }
                if !["dual", "focus", "top"].contains(&display.as_str()) {
                    return Err("Unknown DS display layout".into());
                }
            }
            for p in v.devices.values_mut() {
                for (sys, b) in &mut p.mappings {
                    if sys == "wii-sideways" {
                        upgrade_sideways(b);
                    }
                }
                for (id, layout) in &mut p.games {
                    if crate::games::find(id)?.system == "wii-sideways" {
                        upgrade_sideways(&mut layout.bindings);
                    }
                }

                if p.games.len() > crate::games::catalog().len() {
                    return Err("Too many saved game layouts".into());
                }
                for (id, layout) in &p.games {
                    let game = crate::games::find(id)?;
                    crate::model::validate(&layout.bindings, &game.system)?;
                }

                if !(1..=4).contains(&p.player) {
                    return Err("Invalid saved player assignment".into());
                }
                for (sys, b) in &p.mappings {
                    crate::model::validate(b, sys)?;
                }
            }
            Ok(v)
        }
        None => Ok(crate::model::Settings::default()),
    }
}
pub fn running_emulators() -> Vec<String> {
    let names = [
        "puNES",
        "punes",
        "melonDS",
        "melonds",
        "azahar",
        "azahar-qt",
        "dolphin-emu",
        "Cemu",
        "cemu",
    ];
    let mut found = Vec::new();
    if let Ok(dirs) = fs::read_dir("/proc") {
        for entry in dirs.flatten() {
            if !entry
                .file_name()
                .to_string_lossy()
                .chars()
                .all(|c| c.is_ascii_digit())
            {
                continue;
            }
            if let Ok(name) = fs::read_to_string(entry.path().join("comm")) {
                let name = name.trim();
                if names.contains(&name) && !found.contains(&name.to_owned()) {
                    found.push(name.to_owned());
                }
            }
        }
    }
    found
}
pub fn apply(home: &Path, changes: &[Change]) -> Result<Receipt, String> {
    recover_interrupted(home)?;
    let running = running_emulators();
    if !running.is_empty() {
        return Err(format!(
            "Close {} to apply controls. Your choices are remembered.",
            running.join(", ")
        ));
    }
    let mut entries = Vec::new();
    let mut paths = std::collections::BTreeSet::new();
    for c in changes {
        if !allowed(&c.path) || c.content.len() > 2_000_000 || !paths.insert(&c.path) {
            return Err("Profile path or size failed validation".into());
        }
        let before = read(home, &c.path)?;
        if before.as_deref() != Some(&c.content) {
            entries.push(Entry {
                path: c.path.clone(),
                before,
                after: c.content.clone(),
            });
        }
    }
    if entries.is_empty() {
        return Ok(Receipt {
            id: String::new(),
            changed: 0,
            files: vec![],
        });
    }
    let id = nonce();
    let backup = Backup {
        id: id.clone(),
        entries,
    };
    // Save the backup before changing emulator files.
    let record = format!("{STATE}/backups/{id}.json");
    let encoded = serde_json::to_vec_pretty(&backup).map_err(|e| e.to_string())?;
    if encoded.len() > 2_000_000 {
        return Err(
            "The complete backup exceeds the recovery size limit; no emulator files were changed"
                .into(),
        );
    }
    write_atomic(home, &record, &encoded)?;
    fs::set_permissions(
        home.join(STATE).join("backups"),
        fs::Permissions::from_mode(0o700),
    )
    .map_err(|e| e.to_string())?;
    write_atomic(home, &format!("{STATE}/pending-transaction"), id.as_bytes())?;
    for (done, entry) in backup.entries.iter().enumerate() {
        if let Err(error) = write_atomic(home, &entry.path, entry.after.as_bytes()) {
            let mut rollback_failed = false;
            for old in backup.entries.iter().take(done + 1) {
                if put_back(home, old).is_err() {
                    rollback_failed = true;
                }
            }
            return Err(format!(
                "Save stopped: {error}. {} Recovery record: {id}",
                if rollback_failed {
                    "Some files need recovery."
                } else {
                    "Earlier writes were rolled back."
                }
            ));
        }
    }
    write_atomic(home, &format!("{STATE}/last-backup"), id.as_bytes())?;
    fs::remove_file(safe_path(home, &format!("{STATE}/pending-transaction"))?)
        .map_err(|e| e.to_string())?;
    Ok(Receipt {
        id,
        changed: backup.entries.len(),
        files: backup.entries.iter().map(|e| e.path.clone()).collect(),
    })
}
fn put_back(home: &Path, entry: &Entry) -> Result<(), String> {
    match &entry.before {
        Some(data) => write_atomic(home, &entry.path, data.as_bytes()),
        None => {
            let p = safe_path(home, &entry.path)?;
            if p.exists() {
                fs::remove_file(p).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
    }
}
pub fn recover_interrupted(home: &Path) -> Result<(), String> {
    let Some(id) = read(home, &format!("{STATE}/pending-transaction"))? else {
        return Ok(());
    };
    if !running_emulators().is_empty() {
        return Err("Close emulators to recover an interrupted setup".into());
    }
    if id.is_empty() || id.len() > 80 || !id.chars().all(|c| c.is_ascii_digit() || c == '-') {
        return Err("Invalid pending recovery record".into());
    }
    let pending = safe_path(home, &format!("{STATE}/pending-transaction"))?;
    if read(home, &format!("{STATE}/last-backup"))?.as_deref() == Some(&id) {
        fs::remove_file(pending).map_err(|e| e.to_string())?;
        return Ok(());
    }
    let text = read(home, &format!("{STATE}/backups/{id}.json"))?
        .ok_or("Pending recovery record is missing")?;
    let backup: Backup = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let mut paths = std::collections::BTreeSet::new();
    if backup.id != id {
        return Err("Recovery identity mismatch".into());
    }
    for entry in &backup.entries {
        if !allowed(&entry.path) || !paths.insert(&entry.path) {
            return Err("Invalid pending recovery target".into());
        }
        let current = read(home, &entry.path)?;
        if current != entry.before && current.as_deref() != Some(&entry.after) {
            return Err(
                "Interrupted setup files were edited afterward; recovery stopped to preserve them"
                    .into(),
            );
        }
    }
    for entry in &backup.entries {
        put_back(home, entry)?;
    }
    fs::remove_file(pending).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn restore(home: &Path) -> Result<usize, String> {
    if !running_emulators().is_empty() {
        return Err("Close the emulators before restoring".into());
    }
    let id = read(home, &format!("{STATE}/last-backup"))?
        .ok_or("There is no earlier setup to restore")?;
    if id.len() > 80 || !id.chars().all(|c| c.is_ascii_digit() || c == '-') {
        return Err("Invalid recovery record".into());
    }
    let text =
        read(home, &format!("{STATE}/backups/{id}.json"))?.ok_or("Recovery record is missing")?;
    let backup: Backup = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if backup.id != id {
        return Err("Recovery identity mismatch".into());
    }
    let mut paths = std::collections::BTreeSet::new();
    for entry in &backup.entries {
        if !allowed(&entry.path) || !paths.insert(&entry.path) {
            return Err("Invalid recovery target".into());
        }
        let current = read(home, &entry.path)?;
        if current != entry.before && current.as_deref() != Some(&entry.after) {
            return Err("An emulator configuration changed since this backup. Restore stopped to preserve those changes.".into());
        }
    }
    for entry in &backup.entries {
        put_back(home, entry)?;
    }
    let last = safe_path(home, &format!("{STATE}/last-backup"))?;
    fs::remove_file(last).map_err(|e| e.to_string())?;
    Ok(backup.entries.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Home(PathBuf);
    impl Home {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!("controller-studio-test-{}", nonce()));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Home {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    const FILE: &str = ".var/app/net.kuribo64.melonDS/config/melonDS/melonDS.toml";
    #[test]
    fn apply_and_restore_preserve_original_contents() {
        let h = Home::new();
        let p = h.0.join(FILE);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, "original keyboard + video").unwrap();
        let r = apply(
            &h.0,
            &[Change {
                path: FILE.into(),
                content: "new keyboard + pad + video".into(),
                note: String::new(),
            }],
        )
        .unwrap();
        assert_eq!(r.changed, 1);
        assert_eq!(
            fs::read_to_string(&p).unwrap(),
            "new keyboard + pad + video"
        );
        assert_eq!(restore(&h.0).unwrap(), 1);
        assert_eq!(fs::read_to_string(p).unwrap(), "original keyboard + video");
    }
    #[test]
    fn reject_all_changes_before_following_a_symlink() {
        let h = Home::new();
        std::os::unix::fs::symlink("/tmp", h.0.join(".var")).unwrap();
        assert!(
            apply(
                &h.0,
                &[Change {
                    path: FILE.into(),
                    content: "bad".into(),
                    note: String::new()
                }]
            )
            .is_err()
        );
    }
    #[test]
    fn restoration_does_not_overwrite_later_emulator_changes() {
        let h = Home::new();
        apply(
            &h.0,
            &[Change {
                path: FILE.into(),
                content: "new".into(),
                note: String::new(),
            }],
        )
        .unwrap();
        fs::write(h.0.join(FILE), "later edit").unwrap();
        assert!(restore(&h.0).is_err());
        assert_eq!(fs::read_to_string(h.0.join(FILE)).unwrap(), "later edit");
    }
    #[test]
    fn interrupted_transaction_recovers_mixed_before_and_after_files() {
        let h = Home::new();
        let other = ".var/app/org.DolphinEmu.dolphin-emu/config/dolphin-emu/GCPadNew.ini";
        let id = "123-456";
        let record = Backup {
            id: id.into(),
            entries: vec![
                Entry {
                    path: FILE.into(),
                    before: Some("old ds".into()),
                    after: "new ds".into(),
                },
                Entry {
                    path: other.into(),
                    before: Some("old gc".into()),
                    after: "new gc".into(),
                },
            ],
        };
        write_atomic(&h.0, FILE, b"new ds").unwrap();
        write_atomic(&h.0, other, b"old gc").unwrap();
        write_atomic(
            &h.0,
            &format!("{STATE}/backups/{id}.json"),
            &serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
        write_atomic(&h.0, &format!("{STATE}/pending-transaction"), id.as_bytes()).unwrap();
        recover_interrupted(&h.0).unwrap();
        assert_eq!(read(&h.0, FILE).unwrap().as_deref(), Some("old ds"));
        assert_eq!(read(&h.0, other).unwrap().as_deref(), Some("old gc"));
        assert!(!h.0.join(STATE).join("pending-transaction").exists());
    }
    #[test]
    fn traversal_and_unrelated_files_are_rejected() {
        assert!(!allowed("../../.ssh/authorized_keys"));
        assert!(!allowed(".var/app/other/settings.ini"));
        assert!(!allowed(
            ".var/app/info.cemu.Cemu/config/Cemu/controllerProfiles/../bad.xml"
        ));
    }
}
