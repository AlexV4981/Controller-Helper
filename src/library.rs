//! A bounded, content-addressed DS library. Availability is advisory; launches revalidate.
use crate::{
    games::{self, Game},
    storage,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::CString,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const INDEX: &str = ".local/state/controller-studio/ds-library.json";
const ROOT: &str = "Games/Nintendo/DS";
const MAX_RECORDS: usize = 512;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub id: String,
    pub title: String,
    pub code: String,
    pub relative: String,
    pub available: bool,
    stamp: Vec<i128>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Library {
    schema: u8,
    pub records: BTreeMap<String, Record>,
    #[serde(default)]
    pub aliases: BTreeMap<String, String>,
    #[serde(default)]
    pub issues: Vec<String>,
}
impl Default for Library {
    fn default() -> Self {
        Self {
            schema: 1,
            records: BTreeMap::new(),
            aliases: BTreeMap::new(),
            issues: Vec::new(),
        }
    }
}
pub fn valid_id(id: &str) -> bool {
    id.strip_prefix("ds-").is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
pub fn config(id: &str) -> Result<String, String> {
    if !valid_id(id) {
        return Err("Invalid DS content identity".into());
    }
    Ok(format!(
        ".var/app/net.kuribo64.melonDS/config/controller-studio-games/{id}/melonDS/melonDS.toml"
    ))
}
fn relative(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 1024
        && Path::new(s)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
// Walk from an anchored directory descriptor. No component may follow a link,
// and O_NONBLOCK prevents special files from blocking before type validation.
fn open_under(home: &Path, rel: &Path) -> Result<File, String> {
    let mut dir = File::open(home).map_err(|e| e.to_string())?;
    if !dir.metadata().map_err(|e| e.to_string())?.is_dir() {
        return Err("Home is not a directory".into());
    }
    let parts: Vec<_> = rel.components().collect();
    for (i, part) in parts.iter().enumerate() {
        let Component::Normal(name) = part else {
            return Err("Unsafe library path".into());
        };
        let name = CString::new(name.as_bytes()).map_err(|_| "Invalid path")?;
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | libc::O_NONBLOCK
            | if i + 1 < parts.len() {
                libc::O_DIRECTORY
            } else {
                0
            };
        // SAFETY: dir and name remain alive for openat; ownership of a successful fd transfers once.
        let fd = unsafe { libc::openat(dir.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        dir = unsafe { File::from_raw_fd(fd) };
    }
    Ok(dir)
}
fn stamp(m: &fs::Metadata) -> Vec<i128> {
    vec![
        m.dev() as i128,
        m.ino() as i128,
        m.len() as i128,
        m.mtime() as i128,
        m.mtime_nsec() as i128,
        m.ctime() as i128,
        m.ctime_nsec() as i128,
    ]
}
fn digest(file: &File) -> Result<String, String> {
    let mut input = file.try_clone().map_err(|e| e.to_string())?;
    input.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let out = Command::new("/usr/bin/timeout")
        .args(["2", "/usr/bin/sha256sum"])
        .env_clear()
        .stdin(Stdio::from(input))
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("ROM digest timed out or failed".into());
    }
    let hash = String::from_utf8(out.stdout)
        .map_err(|_| "Invalid digest output")?
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_owned();
    let id = format!("ds-{hash}");
    if !valid_id(&id) {
        return Err("Invalid ROM digest".into());
    }
    Ok(id)
}
fn header(file: &mut File) -> Result<(String, String), String> {
    let m = file.metadata().map_err(|e| e.to_string())?;
    if !m.is_file() || m.len() < 512 || m.len() > 536_870_912 {
        return Err("Expected a regular DS ROM between 512 bytes and 512 MiB".into());
    }
    let mut h = [0u8; 512];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut h))
        .map_err(|_| "Truncated DS header")?;
    if h[18] != 0 {
        return Err("Only standard DS ROMs are supported; use melonDS directly for DSi".into());
    }
    let word = |n: usize| u32::from_le_bytes(h[n..n + 4].try_into().unwrap()) as u64;
    for (start, size, required) in [
        (0x20, 0x2c, true),
        (0x30, 0x3c, true),
        (0x40, 0x44, false),
        (0x48, 0x4c, false),
        (0x50, 0x54, false),
        (0x58, 0x5c, false),
    ] {
        let (off, len) = (word(start), word(size));
        if required && len == 0
            || len > 0 && (off < 512 || off.checked_add(len).is_none_or(|end| end > m.len()))
        {
            return Err("DS executable or table range is outside the ROM".into());
        }
    }
    let banner = word(0x68);
    if banner != 0 && (banner < 512 || banner.checked_add(0x840).is_none_or(|end| end > m.len())) {
        return Err("DS banner is truncated".into());
    }
    if h[..12].iter().any(|b| *b != 0 && b.is_ascii_control()) {
        return Err("Invalid DS title control character".into());
    }
    let title = String::from_utf8_lossy(&h[..12])
        .trim_matches('\0')
        .trim()
        .to_owned();
    if title.chars().any(char::is_control) {
        return Err("Invalid DS title".into());
    }
    let code = String::from_utf8_lossy(&h[12..16]).into_owned();
    if !code.bytes().all(|c| c.is_ascii_graphic()) {
        return Err("Invalid DS game code".into());
    }
    Ok((title, code))
}
impl Library {
    pub fn load(home: &Path) -> Result<Self, String> {
        match fs::symlink_metadata(home.join(INDEX)) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.to_string()),
            Ok(_) => (),
        }
        let mut file = open_under(home, Path::new(INDEX))?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file()
            || metadata.len() > 2_000_000
            || metadata.uid() != fs::metadata(home).map_err(|e| e.to_string())?.uid()
            || metadata.mode() & 0o022 != 0
        {
            return Err("DS index must be a bounded regular file owned by this user and not writable by others".into());
        }
        let mut text = String::new();
        (&mut file)
            .take(2_000_001)
            .read_to_string(&mut text)
            .map_err(|e| e.to_string())?;
        if text.len() > 2_000_000
            || stamp(&metadata) != stamp(&file.metadata().map_err(|e| e.to_string())?)
        {
            return Err("DS index changed while being read".into());
        }
        let l: Self =
            serde_json::from_str(&text).map_err(|e| format!("Invalid DS library index: {e}"))?;
        if l.schema != 1 || l.records.len() > MAX_RECORDS || l.issues.len() > 128 {
            return Err("Unsupported or oversized DS library index".into());
        }
        for (alias, id) in &l.aliases {
            if !games::find(alias).is_ok_and(|g| g.system == "ds") || !l.records.contains_key(id) {
                return Err("Invalid legacy DS alias".into());
            }
        }
        for (id, r) in &l.records {
            if !valid_id(id)
                || r.id != *id
                || !relative(&r.relative)
                || r.title.len() > 512
                || r.title.chars().any(char::is_control)
                || r.code.len() != 4
                || !r.code.bytes().all(|c| c.is_ascii_graphic())
                || r.stamp.len() != 7
            {
                return Err("Invalid DS library record".into());
            }
        }
        Ok(l)
    }
    pub fn save(&self, home: &Path) -> Result<(), String> {
        storage::write_atomic(
            home,
            INDEX,
            &serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
    }
    pub fn migrate_settings(&self, settings: &mut crate::model::Settings) -> bool {
        let mut changed = false;
        for (legacy, id) in &self.aliases {
            if !settings.ds_migrations.insert(id.clone()) {
                continue;
            }
            changed = true;
            for p in settings.devices.values_mut() {
                if !p.games.contains_key(id)
                    && let Some(layout) = p.games.get(legacy).cloned()
                {
                    p.games.insert(id.clone(), layout);
                    changed = true;
                }
            }
            if !settings.ds_display.contains_key(id) {
                let display =
                    settings
                        .ds_display
                        .get(legacy)
                        .cloned()
                        .unwrap_or(if legacy == "ASME" {
                            "top".into()
                        } else {
                            "dual".into()
                        });
                settings.ds_display.insert(id.clone(), display);
                changed = true;
            }
        }
        changed
    }
    pub fn find(&self, id: &str) -> Result<Game, String> {
        if let Some(r) = self.records.get(id) {
            return Ok(Game {
                id: r.id.clone(),
                title: r.title.clone(),
                system: "ds".into(),
                folder: "DS".into(),
                filename: r.relative.clone(),
            });
        }
        games::find(id)
    }
    pub fn catalog(&self) -> Vec<Game> {
        let mut list = games::catalog();
        list.extend(self.records.values().map(|r| Game {
            id: r.id.clone(),
            title: format!("{} · {}", r.title, &r.id[3..11]),
            system: "ds".into(),
            folder: "DS".into(),
            filename: r.relative.clone(),
        }));
        list
    }
    pub fn available(&self, home: &Path, id: &str) -> bool {
        self.records
            .get(id)
            .map(|r| r.available)
            .unwrap_or_else(|| {
                games::find(id).is_ok_and(|g| {
                    home.join("Games/Nintendo")
                        .join(g.folder)
                        .join(g.filename)
                        .is_file()
                })
            })
    }
    pub fn resolve(&self, home: &Path, id: &str) -> Result<PathBuf, String> {
        let Some(r) = self.records.get(id) else {
            return games::resolve(home, id);
        };
        if !r.available {
            return Err("ROM is unavailable or ambiguous; check library issues and refresh".into());
        }
        let mut file = open_under(home, &Path::new(ROOT).join(&r.relative))?;
        let before = stamp(&file.metadata().map_err(|e| e.to_string())?);
        header(&mut file)?;
        if digest(&file)? != id || before != stamp(&file.metadata().map_err(|e| e.to_string())?) {
            return Err("ROM changed since discovery; refresh the library".into());
        }
        Ok(home.join(ROOT).join(&r.relative))
    }
    pub fn refresh(&mut self, home: &Path) -> Result<(), String> {
        let started = Instant::now();
        let mut next = self.clone();
        next.issues.clear();
        for r in next.records.values_mut() {
            r.available = false;
        }
        let mut dirs = vec![PathBuf::new()];
        let mut entries = 0;
        let mut seen = BTreeSet::new();
        while let Some(rel) = dirs.pop() {
            if started.elapsed() > Duration::from_secs(8) {
                next.issues
                    .push("Scan time limit reached; refresh again to retry".into());
                break;
            }
            let dir = match open_under(home, &Path::new(ROOT).join(&rel)) {
                Ok(f) => f,
                Err(e) => {
                    if next.issues.len() < 100 {
                        next.issues
                            .push(format!("DS folder {}: {e}", rel.display()));
                    }
                    continue;
                }
            };
            if !dir.metadata().map_err(|e| e.to_string())?.is_dir() {
                next.issues
                    .push("DS library root is not a directory".into());
                break;
            }
            let listing = fs::read_dir(format!("/proc/self/fd/{}", dir.as_raw_fd()))
                .map_err(|e| e.to_string())?;
            for entry in listing {
                entries += 1;
                if entries > 4096 || started.elapsed() > Duration::from_secs(8) {
                    next.issues.push("DS scan limit reached".into());
                    dirs.clear();
                    break;
                }
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        if next.issues.len() < 100 {
                            next.issues.push(e.to_string());
                        }
                        continue;
                    }
                };
                let path = rel.join(entry.file_name());
                let Some(name) = path.to_str().filter(|s| relative(s)) else {
                    if next.issues.len() < 100 {
                        next.issues
                            .push("Skipped a non-UTF-8 or oversized path".into());
                    }
                    continue;
                };
                let result = (|| -> Result<(), String> {
                    let mut file = open_under(home, &Path::new(ROOT).join(&path))?;
                    let meta = file.metadata().map_err(|e| e.to_string())?;
                    if meta.is_dir() {
                        if path.components().count() < 12 {
                            dirs.push(path.clone());
                        } else {
                            return Err("Subfolder depth limit reached".into());
                        }
                        return Ok(());
                    }
                    if !path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("nds"))
                    {
                        return Ok(());
                    }
                    let (title, code) = header(&mut file)?;
                    let before = stamp(&meta);
                    let cached = self
                        .records
                        .values()
                        .find(|r| r.relative == name && r.stamp == before);
                    let id = match cached {
                        Some(r) => r.id.clone(),
                        None => digest(&file)?,
                    };
                    if stamp(&file.metadata().map_err(|e| e.to_string())?) != before {
                        return Err("ROM changed during scanning".into());
                    }
                    if !seen.insert(id.clone()) {
                        if let Some(record) = next.records.get_mut(&id) {
                            record.available = false;
                        }
                        return Err("Duplicate ROM content: launch disabled to avoid selecting the wrong save location; remove one copy and refresh".into());
                    }
                    if !next.records.contains_key(&id) && next.records.len() >= MAX_RECORDS {
                        return Err(
                            "DS library record limit reached; existing preferences retained".into(),
                        );
                    }
                    next.records.insert(
                        id.clone(),
                        Record {
                            id,
                            title: if title.is_empty() {
                                path.file_stem()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .into()
                            } else {
                                title
                            },
                            code,
                            relative: name.into(),
                            available: true,
                            stamp: before,
                        },
                    );
                    Ok(())
                })();
                if let Err(e) = result
                    && next.issues.len() < 100
                {
                    next.issues.push(format!("{name}: {e}"));
                }
            }
        }
        for game in games::catalog().into_iter().filter(|g| g.system == "ds") {
            if next.aliases.contains_key(&game.id) {
                continue;
            }
            let wide = "ds-a5a3c793c95087996c71f7f9a91ef17c7344ee45a815a413de3b9296ddee47a5";
            let candidates: Vec<_> = next
                .records
                .values()
                .filter(|r| {
                    r.available
                        && r.code == game.id
                        && (r.relative == game.filename || game.id == "ASME" && r.id == wide)
                })
                .collect();
            let selected = candidates
                .iter()
                .find(|r| r.id == wide)
                .or_else(|| candidates.first());
            if let Some(record) = selected {
                next.aliases.insert(game.id, record.id.clone());
            }
        }
        next.save(home)?;
        *self = next;
        Ok(())
    }
}
