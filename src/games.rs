use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Game {
    pub id: String,
    pub title: String,
    pub system: String,
    pub folder: String,
    pub filename: String,
}
pub fn catalog() -> Vec<Game> {
    serde_json::from_str(include_str!("games.json")).expect("bundled catalog")
}
pub fn find(id: &str) -> Result<Game, String> {
    catalog()
        .into_iter()
        .find(|g| g.id == id)
        .ok_or_else(|| "Unknown game; choose one from the library".into())
}
pub fn ds_config(id: &str) -> Result<String, String> {
    if crate::library::valid_id(id) {
        return crate::library::config(id);
    }
    let g = find(id)?;
    if g.system != "ds" {
        return Err("Not a DS game".into());
    }
    Ok(format!(
        ".var/app/net.kuribo64.melonDS/config/controller-studio-games/{id}/melonDS/melonDS.toml"
    ))
}
pub fn resolve(home: &Path, id: &str) -> Result<PathBuf, String> {
    let g = find(id)?;
    let wide_name = "Super Mario 64 DS (USA) (Rev 1) (Widescreen).nds";
    let wide = if g.id == "ASME" {
        match fs::symlink_metadata(home.join("Games/Nintendo/DS").join(wide_name)) {
            Ok(_) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(e.to_string()),
        }
    } else {
        false
    };
    let rel = Path::new("Games/Nintendo").join(&g.folder).join(if wide {
        wide_name
    } else {
        &g.filename
    });
    let mut current = home.to_path_buf();
    for part in rel.components() {
        current.push(part.as_os_str());
        if current.is_symlink() {
            return Err("Game path contains a symbolic link".into());
        }
    }
    let path = current.canonicalize().map_err(|_| "Game file is missing")?;
    let base = home
        .join("Games/Nintendo")
        .canonicalize()
        .map_err(|_| "Game library is missing")?;
    if !path.starts_with(base) || !path.is_file() {
        return Err("Game path is outside the library".into());
    }
    let mut header = [0u8; 96];
    let mut file = fs::File::open(&path).map_err(|e| e.to_string())?;
    file.read_exact(&mut header)
        .map_err(|_| "Game header could not be read")?;
    if wide {
        if header[30] != 1 {
            return Err("Widescreen ROM revision is incorrect".into());
        }
        let output = std::process::Command::new("/usr/bin/sha256sum")
            .arg("--")
            .arg(&path)
            .output()
            .map_err(|e| e.to_string())?;
        let hash = String::from_utf8_lossy(&output.stdout);
        if !output.status.success()
            || hash.split_whitespace().next()
                != Some("a5a3c793c95087996c71f7f9a91ef17c7344ee45a815a413de3b9296ddee47a5")
        {
            return Err("Widescreen ROM failed verification; original game is unchanged".into());
        }
    }
    let actual = if g.system == "ds" {
        &header[12..16]
    } else {
        if &header[..4] != b"RVZ\x01" {
            return Err("Expected an RVZ game".into());
        }
        &header[88..94]
    };
    if actual != g.id.as_bytes() {
        return Err("Game header does not match its library entry".into());
    }
    Ok(path)
}

pub fn check_ds_launch(home: &Path, id: &str) -> Result<(), String> {
    let paths = [
        ".var/app/net.kuribo64.melonDS/config/melonDS/melonDS.toml".to_owned(),
        ds_config(id)?,
    ];
    for path in paths {
        if let Some(text) = crate::storage::read(home, &path)? {
            let config: toml::Value =
                toml::from_str(&text).map_err(|e| format!("Invalid DS configuration: {e}"))?;
            if config
                .get("Emu")
                .and_then(|e| e.get("ConsoleType"))
                .is_some_and(|v| v.as_integer() != Some(0))
                || config
                    .get("Emu")
                    .and_then(|e| e.get("ExternalBIOSEnable"))
                    .is_some_and(|v| v.as_bool() != Some(false))
            {
                return Err("Isolated DS layouts require DS mode and built-in BIOS. Use the shared melonDS launcher for external firmware or DSi.".into());
            }
            for key in ["SaveFilePath", "SavestatePath", "CheatFilePath"] {
                if let Some(v) = config.get("Instance0").and_then(|t| t.get(key)) {
                    let value = v.as_str().ok_or("Invalid DS save path type")?;
                    if !value.is_empty() && !Path::new(value).is_absolute() {
                        return Err("Use an absolute save/cheat path or the default ROM-adjacent path before launching a DS game layout".into());
                    }
                }
            }
        }
    }
    Ok(())
}
pub fn copy_wfc_once(home: &Path, id: &str) -> Result<(), String> {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    let source = crate::storage::safe_path(
        home,
        ".var/app/net.kuribo64.melonDS/config/melonDS/wfcsettings.bin",
    )?;
    if !source.exists() {
        return Ok(());
    }
    let metadata = source.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > 1_000_000 {
        return Err("Unexpected Wi-Fi settings file".into());
    }
    let target = ds_config(id)?.replace("melonDS.toml", "wfcsettings.bin");
    let target = crate::storage::safe_path(home, &target)?;
    if target.exists() {
        return Ok(());
    }
    let data = fs::read(source).map_err(|e| e.to_string())?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(target)
        .map_err(|e| e.to_string())?;
    file.write_all(&data)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}

// Flatpak initializes XDG paths itself, so set the game path inside its sandbox.
pub fn ds_launch_args(config: &Path, rom: &Path) -> Vec<std::ffi::OsString> {
    vec![
        "run".into(),
        "--command=env".into(),
        "net.kuribo64.melonDS".into(),
        format!("XDG_CONFIG_HOME={}", config.display()).into(),
        "melonDS".into(),
        "--fullscreen".into(),
        rom.as_os_str().to_owned(),
    ]
}

#[cfg(test)]
mod launch_tests {
    #[test]
    fn ds_sets_config_inside_flatpak_and_keeps_paths_single_arguments() {
        let config = std::path::Path::new("/home/a kid/config/ASME");
        let rom = std::path::Path::new("/home/a kid/Mario DS.nds");
        let args = super::ds_launch_args(config, rom);
        assert_eq!(
            args,
            [
                "run",
                "--command=env",
                "net.kuribo64.melonDS",
                "XDG_CONFIG_HOME=/home/a kid/config/ASME",
                "melonDS",
                "--fullscreen",
                "/home/a kid/Mario DS.nds"
            ]
        );
    }
}
