use controller_studio::library::Library;
use std::{fs, path::PathBuf};
struct Home(PathBuf);
impl Home {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "ds-library-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(p.join("Games/Nintendo/DS")).unwrap();
        Self(p)
    }
    fn rom(&self, name: &str, seed: u8) {
        let mut data = vec![seed; 4096];
        data[..12].copy_from_slice(b"HOMEBREW    ");
        data[12..16].copy_from_slice(b"####");
        data[18] = 0;
        for (off, value) in [
            (0x20, 0x200u32),
            (0x2c, 0x100),
            (0x30, 0x300),
            (0x3c, 0x100),
        ] {
            data[off..off + 4].copy_from_slice(&value.to_le_bytes());
        }
        for off in [0x40, 0x44, 0x48, 0x4c, 0x50, 0x54, 0x58, 0x5c, 0x68] {
            data[off..off + 4].fill(0);
        }
        let p = self.0.join("Games/Nintendo/DS").join(name);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, data).unwrap();
    }
}
impl Drop for Home {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn discovers_unknown_titles_preserves_rename_and_missing_identity() {
    let h = Home::new();
    h.rom("子供/first.nds", 1);
    h.rom("second.NDS", 2);
    let mut l = Library::load(&h.0).unwrap();
    l.refresh(&h.0).unwrap();
    assert_eq!(l.records.len(), 2);
    let id = l
        .records
        .values()
        .find(|r| r.relative.ends_with("first.nds"))
        .unwrap()
        .id
        .clone();
    assert_ne!(
        l.records.values().next().unwrap().id,
        l.records.values().last().unwrap().id
    );
    fs::rename(
        h.0.join("Games/Nintendo/DS/子供/first.nds"),
        h.0.join("Games/Nintendo/DS/renamed.nds"),
    )
    .unwrap();
    l.refresh(&h.0).unwrap();
    assert_eq!(l.records[&id].relative, "renamed.nds");
    fs::remove_file(h.0.join("Games/Nintendo/DS/renamed.nds")).unwrap();
    l.refresh(&h.0).unwrap();
    assert!(!l.records[&id].available);
    assert!(l.resolve(&h.0, &id).is_err());
    assert!(Library::load(&h.0).unwrap().records.contains_key(&id));
}
#[test]
fn rejects_symlinks_malformed_and_replaced_content() {
    let h = Home::new();
    h.rom("good.nds", 3);
    fs::write(h.0.join("Games/Nintendo/DS/bad.nds"), b"not a rom").unwrap();
    std::os::unix::fs::symlink("good.nds", h.0.join("Games/Nintendo/DS/link.nds")).unwrap();
    let mut l = Library::load(&h.0).unwrap();
    l.refresh(&h.0).unwrap();
    assert_eq!(l.records.len(), 1);
    assert!(l.issues.len() >= 2);
    let id = l.records.keys().next().unwrap().clone();
    h.rom("good.nds", 4);
    assert!(l.resolve(&h.0, &id).is_err());
    assert!(!controller_studio::storage::allowed_at(
        &h.0,
        ".var/app/net.kuribo64.melonDS/config/controller-studio-games/ds-000/melonDS/melonDS.toml"
    ));
}
#[test]
fn public_marker_identifies_content_and_rejects_ambiguity() {
    let h = Home::new();
    h.rom("a.nds", 1);
    h.rom("b.nds", 2);
    let mut l = Library::load(&h.0).unwrap();
    l.refresh(&h.0).unwrap();
    let ids: Vec<_> = l.records.keys().cloned().collect();
    let title = |id: &str| format!("[60/60] melonDS 1.1 [CS-ROM-v1:{}]", &id[3..]);
    assert_eq!(
        controller_studio::runtime::identify_ds_markers(&[title(&ids[0])], &l),
        Some(ids[0].clone())
    );
    assert_eq!(
        controller_studio::runtime::identify_ds_markers(&[title(&ids[0]), title(&ids[1])], &l),
        None
    );
    assert_eq!(
        controller_studio::runtime::identify_ds_markers(&["melonDS 1.1".into()], &l),
        None
    );
    assert_eq!(
        controller_studio::runtime::identify_ds_markers(
            &["[CS-ROM-v1:../../etc/passwd]".into()],
            &l
        ),
        None
    );
}

#[test]
fn dynamic_profiles_are_on_demand_and_restore_does_not_touch_roms() {
    let h = Home::new();
    h.rom("new game.nds", 7);
    let mut l = Library::load(&h.0).unwrap();
    l.refresh(&h.0).unwrap();
    let id = l.records.keys().next().unwrap().clone();
    let config =
        h.0.join(".var/app/net.kuribo64.melonDS/config/melonDS/melonDS.toml");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::write(&config,"[Emu]\nConsoleType=0\nExternalBIOSEnable=false\n[Instance0]\nJoystickID=0\n[Instance0.Keyboard]\nA=88\n").unwrap();
    let d: controller_studio::model::Device =
        serde_json::from_str(include_str!("fixtures/xbox.json")).unwrap();
    let mut s = controller_studio::model::Settings::default();
    s.assign(std::slice::from_ref(&d));
    let target = controller_studio::library::config(&id).unwrap();
    assert!(controller_studio::storage::allowed_at(&h.0, &target));
    let plan = controller_studio::profiles::build(&h.0, std::slice::from_ref(&d), &s).unwrap();
    assert!(!plan.changes.iter().any(|c| c.path == target));
    let mut bindings = s.bindings(&d, "ds");
    bindings.insert("A".into(), "north".into());
    s.devices.get_mut(&d.key).unwrap().games.insert(
        id.clone(),
        controller_studio::model::GamePreference {
            mode: controller_studio::model::Mode::Labels,
            bindings,
        },
    );
    controller_studio::storage::save_settings(&h.0, &s).unwrap();
    let plan = controller_studio::profiles::build(&h.0, std::slice::from_ref(&d), &s).unwrap();
    assert!(plan.changes.iter().any(|c| c.path == target));
    let original = fs::read(h.0.join("Games/Nintendo/DS/new game.nds")).unwrap();
    controller_studio::storage::apply(&h.0, &plan.changes).unwrap();
    assert_eq!(
        controller_studio::storage::load_settings(&h.0)
            .unwrap()
            .devices[&d.key]
            .games[&id]
            .bindings["A"],
        "north"
    );
    fs::remove_file(h.0.join("Games/Nintendo/DS/new game.nds")).unwrap();
    l.refresh(&h.0).unwrap();
    assert!(
        controller_studio::storage::load_settings(&h.0)
            .unwrap()
            .devices[&d.key]
            .games
            .contains_key(&id)
    );
    assert_eq!(original.len(), 4096);
}
#[test]
fn rejects_corrupt_index_and_unregistered_configuration_targets() {
    let h = Home::new();
    let path = h.0.join(".local/state/controller-studio/ds-library.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, r#"{"schema":99,"records":{},"issues":[]}"#).unwrap();
    assert!(Library::load(&h.0).is_err());
    let target = format!(
        ".var/app/net.kuribo64.melonDS/config/controller-studio-games/ds-{}/melonDS/melonDS.toml",
        "a".repeat(64)
    );
    assert!(!controller_studio::storage::allowed_at(&h.0, &target));
    assert!(
        controller_studio::storage::apply(
            &h.0,
            &[controller_studio::storage::Change {
                path: target,
                content: "anything".into(),
                note: "test".into()
            }]
        )
        .is_err()
    );
}
#[test]
fn a_symlinked_library_root_and_fifo_cannot_escape_or_block() {
    let h = Home::new();
    h.rom("a.nds", 1);
    let root = h.0.join("Games/Nintendo/DS");
    fs::rename(&root, h.0.join("elsewhere")).unwrap();
    std::os::unix::fs::symlink(h.0.join("elsewhere"), &root).unwrap();
    let mut l = Library::load(&h.0).unwrap();
    l.refresh(&h.0).unwrap();
    assert!(l.records.is_empty());
    assert!(!l.issues.is_empty());
    fs::remove_file(&root).unwrap();
    fs::create_dir(&root).unwrap();
    let name =
        std::ffi::CString::new(root.join("pipe.nds").as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let started = std::time::Instant::now();
    l.refresh(&h.0).unwrap();
    assert!(started.elapsed().as_secs() < 2);
    assert!(l.records.is_empty());
}
#[test]
fn rejects_bad_ranges_dsi_and_controls_but_accepts_trimmed_capacity() {
    let h = Home::new();
    h.rom("trimmed.nds", 9);
    let p = h.0.join("Games/Nintendo/DS/trimmed.nds");
    let original = fs::read(&p).unwrap();
    let mut bytes = original.clone();
    bytes[20] = 9;
    fs::write(&p, &bytes).unwrap();
    let mut l = Library::load(&h.0).unwrap();
    l.refresh(&h.0).unwrap();
    assert_eq!(l.records.len(), 1);
    for (name, offset, value) in [("dsi.nds", 18, 2), ("control.nds", 0, 10)] {
        let mut b = original.clone();
        b[offset] = value;
        fs::write(h.0.join("Games/Nintendo/DS").join(name), b).unwrap();
    }
    let mut overflow = original;
    overflow[0x20..0x24].copy_from_slice(&u32::MAX.to_le_bytes());
    fs::write(h.0.join("Games/Nintendo/DS/overflow.nds"), overflow).unwrap();
    l.refresh(&h.0).unwrap();
    assert_eq!(l.records.len(), 1);
    assert!(l.issues.len() >= 3);
}

#[test]
fn legacy_migration_runs_once_and_does_not_undo_inheritance() {
    let h = Home::new();
    h.rom("Super Mario 64 DS (USA).nds", 1);
    let path = h.0.join("Games/Nintendo/DS/Super Mario 64 DS (USA).nds");
    let mut bytes = fs::read(&path).unwrap();
    bytes[12..16].copy_from_slice(b"ASME");
    fs::write(&path, bytes).unwrap();
    let mut l = Library::load(&h.0).unwrap();
    l.refresh(&h.0).unwrap();
    let id = l.aliases["ASME"].clone();
    let d: controller_studio::model::Device =
        serde_json::from_str(include_str!("fixtures/xbox.json")).unwrap();
    let mut settings = controller_studio::model::Settings::default();
    settings.assign(std::slice::from_ref(&d));
    let binding = settings.bindings(&d, "ds");
    settings.devices.get_mut(&d.key).unwrap().games.insert(
        "ASME".into(),
        controller_studio::model::GamePreference {
            mode: controller_studio::model::Mode::Labels,
            bindings: binding,
        },
    );
    assert!(l.migrate_settings(&mut settings));
    assert!(settings.devices[&d.key].games.contains_key(&id));
    settings.devices.get_mut(&d.key).unwrap().games.remove(&id);
    assert!(!l.migrate_settings(&mut settings));
    assert!(!settings.devices[&d.key].games.contains_key(&id));
    controller_studio::storage::save_settings(&h.0, &settings).unwrap();
    assert!(controller_studio::storage::load_settings(&h.0).is_ok());
}
#[test]
fn missing_delimiter_unknown_and_removed_markers_do_not_follow() {
    let h = Home::new();
    h.rom("a.nds", 1);
    let mut l = Library::load(&h.0).unwrap();
    l.refresh(&h.0).unwrap();
    let id = l.records.keys().next().unwrap().clone();
    let hash = &id[3..];
    for title in [
        format!("[CS-ROM-v1:{hash}"),
        format!("[CS-ROM-v1:{}]", "a".repeat(64)),
        format!("[CS-ROM-v1:{hash}extra]"),
    ] {
        assert_eq!(
            controller_studio::runtime::identify_ds_markers(&[title], &l),
            None
        );
    }
    fs::remove_file(h.0.join("Games/Nintendo/DS/a.nds")).unwrap();
    l.refresh(&h.0).unwrap();
    assert_eq!(
        controller_studio::runtime::identify_ds_markers(&[format!("[CS-ROM-v1:{hash}]")], &l),
        None
    );
}
#[test]
fn duplicate_content_is_not_allowed_to_choose_a_save_location() {
    let h = Home::new();
    h.rom("a.nds", 8);
    fs::copy(
        h.0.join("Games/Nintendo/DS/a.nds"),
        h.0.join("Games/Nintendo/DS/b.nds"),
    )
    .unwrap();
    let mut l = Library::load(&h.0).unwrap();
    l.refresh(&h.0).unwrap();
    assert_eq!(l.records.len(), 1);
    let id = l.records.keys().next().unwrap().clone();
    assert!(!l.records[&id].available);
    assert!(l.resolve(&h.0, &id).is_err());
    fs::remove_file(h.0.join("Games/Nintendo/DS/b.nds")).unwrap();
    l.refresh(&h.0).unwrap();
    assert!(l.records[&id].available);
}
#[test]
fn index_cannot_be_a_link_or_writable_by_other_users() {
    use std::os::unix::fs::PermissionsExt;
    let h = Home::new();
    h.rom("a.nds", 2);
    let mut l = Library::load(&h.0).unwrap();
    l.refresh(&h.0).unwrap();
    let p = h.0.join(".local/state/controller-studio/ds-library.json");
    fs::set_permissions(&p, fs::Permissions::from_mode(0o666)).unwrap();
    assert!(Library::load(&h.0).is_err());
    fs::set_permissions(&p, fs::Permissions::from_mode(0o600)).unwrap();
    let moved = h.0.join("index.json");
    fs::rename(&p, &moved).unwrap();
    std::os::unix::fs::symlink(moved, &p).unwrap();
    assert!(Library::load(&h.0).is_err());
}
