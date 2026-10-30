use controller_studio::{
    model::{Device, Mode, Settings},
    profiles, storage,
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
struct Home(PathBuf);
impl Home {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "studio-profile-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn put(&self, path: &str, data: &str) {
        let p = self.0.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, data).unwrap();
    }
}
impl Drop for Home {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn device() -> Device {
    serde_json::from_str(include_str!("fixtures/xbox.json")).unwrap()
}
#[test]
fn printed_labels_reach_real_emulator_formats_and_preserve_other_settings() {
    let h = Home::new();
    let d = device();
    let mut prefs = Settings::default();
    prefs.assign(std::slice::from_ref(&d));
    h.put(
        ".var/app/io.github.punesemu.puNES/config/puNES/input.cfg",
        "[port 1]\nP1J GUID=NULL\nP1K A=S\n",
    );
    h.put(".var/app/net.kuribo64.melonDS/config/melonDS/melonDS.toml","[Instance0]\nJoystickID=0\n[Instance0.Keyboard]\nA=88\n[Instance0.Joystick]\nA=-1\n[Video]\nScale=3\n");
    h.put(".var/app/org.azahar_emu.Azahar/config/azahar-emu/qt-config.ini","[Renderer]\nresolution_factor=2\n[Controls]\nprofiles\\size=1\nprofiles\\1\\name=Default\nprofiles\\1\\button_a=engine:keyboard,code:65\n[Unrelated]\nsentinel=preserved\n");
    h.put(".var/app/org.DolphinEmu.dolphin-emu/config/dolphin-emu/WiimoteNew.ini","[Wiimote1]\nDevice=XInput2/0/Virtual core pointer\nButtons/A=(`SDL/0/Xbox One Controller:Button B`) | `XInput2/0/Virtual core pointer:Click 1`\n");
    h.put(
        ".var/app/org.DolphinEmu.dolphin-emu/config/dolphin-emu/GCPadNew.ini",
        "[GCPad1]\nDevice=XInput2/0/Virtual core pointer\nButtons/A=X\n",
    );
    h.put(
        ".var/app/info.cemu.Cemu/config/Cemu/controllerProfiles/controller0.xml",
        "<emulated_controller><type>Wii U GamePad</type></emulated_controller>",
    );
    h.put(
        ".local/share/flatpak/overrides/info.cemu.Cemu",
        "[Context]\nfilesystems=/home/titus/Games/Nintendo;\n",
    );
    let plan = profiles::build(&h.0, std::slice::from_ref(&d), &prefs).unwrap();
    assert!(plan.warnings.is_empty(), "{:?}", plan.warnings);
    assert_eq!(plan.applications.len(), 5);
    let receipt = storage::apply(&h.0, &plan.changes).unwrap();
    assert!(receipt.changed > 10);
    let ds =
        fs::read_to_string(h.0.join(".var/app/net.kuribo64.melonDS/config/melonDS/melonDS.toml"))
            .unwrap();
    assert!(ds.contains("A = 0"));
    assert!(ds.contains("A = 88"));
    assert!(ds.contains("Scale = 3"));
    let nes = plan
        .changes
        .iter()
        .find(|c| c.path.ends_with(".jsc"))
        .unwrap();
    assert!(nes.content.contains("A = BTN01"));
    assert!(nes.content.contains("B = BTN02"));
    let az = fs::read_to_string(
        h.0.join(".var/app/org.azahar_emu.Azahar/config/azahar-emu/qt-config.ini"),
    )
    .unwrap();
    assert!(az.contains("resolution_factor=2"));
    assert!(az.contains("sentinel=preserved"));
    assert!(az.contains("profiles\\1\\button_a = engine:keyboard,code:65"));
    assert!(az.contains("button:0"));
    let wii = plan
        .changes
        .iter()
        .find(|c| c.path.ends_with("WiimoteNew.ini"))
        .unwrap();
    let a = wii
        .content
        .lines()
        .find(|l| l.starts_with("Buttons/A ="))
        .unwrap();
    assert!(a.contains("Button S"));
    assert!(!a.contains("Button B"));
    assert!(a.contains("Click 1"));
    let overrides =
        fs::read_to_string(h.0.join(".local/share/flatpak/overrides/info.cemu.Cemu")).unwrap();
    assert!(overrides.contains("filesystems=/home/titus/Games/Nintendo;"));
    assert!(overrides.contains("SDL_GAMECONTROLLER_USE_BUTTON_LABELS = 0"));
    assert_eq!(storage::restore(&h.0).unwrap(), receipt.changed);
    assert!(
        fs::read_to_string(h.0.join(".var/app/io.github.punesemu.puNES/config/puNES/input.cfg"))
            .unwrap()
            .contains("P1J GUID=NULL")
    );
}
#[test]
fn position_matching_is_optional_and_exports_the_other_face_button() {
    let h = Home::new();
    let d = device();
    let mut s = Settings::default();
    s.assign(std::slice::from_ref(&d));
    s.devices.get_mut(&d.key).unwrap().mode = Mode::Positions;
    h.put(
        ".var/app/net.kuribo64.melonDS/config/melonDS/melonDS.toml",
        "[Instance0.Joystick]\nA=-1\n",
    );
    let p = profiles::build(&h.0, &[d], &s).unwrap();
    assert!(
        p.changes
            .iter()
            .find(|c| c.path.ends_with("melonDS.toml"))
            .unwrap()
            .content
            .contains("A = 1")
    );
}
#[test]
fn reconnecting_does_not_lose_the_saved_player() {
    let d = device();
    let mut other = d.clone();
    other.key = "another-controller".into();
    let mut s = Settings::default();
    s.assign(&[d.clone(), other.clone()]);
    assert_ne!(s.devices[&d.key].player, s.devices[&other.key].player);
    let saved = s.devices[&other.key].player;
    s.assign(&[other.clone()]);
    assert_eq!(s.devices[&other.key].player, saved);
    s.assign(&[d.clone(), other.clone()]);
    assert_eq!(s.devices[&other.key].player, saved);
}

#[test]
fn swapping_players_keeps_dolphin_bound_to_the_physical_controller() {
    let h = Home::new();
    let first = device();
    let mut second = first.clone();
    second.key = "second".into();
    second.name = "Microsoft X-Box One pad".into();
    second.guid = "03003d6b5e040000d102000001010000".into();
    second.version = 0x101;
    second.instance = 1;
    let mut s = Settings::default();
    s.assign(&[first.clone(), second.clone()]);
    s.devices.get_mut(&first.key).unwrap().player = 2;
    s.devices.get_mut(&second.key).unwrap().player = 1;
    h.put(
        ".var/app/org.DolphinEmu.dolphin-emu/config/dolphin-emu/WiimoteNew.ini",
        "[Wiimote1]\nDevice=XInput2/0/Virtual core pointer\n",
    );
    let plan = profiles::build(&h.0, &[first, second], &s).unwrap();
    let p = plan
        .changes
        .iter()
        .find(|c| c.path.ends_with("ControllerStudio-P1-gamecube.ini"))
        .unwrap();
    assert!(p.content.contains("SDL/1/Xbox One Controller:Button S"));
    assert!(p.content.contains("Device = SDL/1/Xbox One Controller"));
    assert!(!p.content.contains("SDL/0/Xbox One Controller:Button S"));
}
#[test]
fn reconnecting_an_old_pad_resolves_a_conflicting_player_slot() {
    let first = device();
    let mut second = first.clone();
    second.key = "new-controller".into();
    let mut s = Settings::default();
    s.assign(std::slice::from_ref(&first));
    s.assign(std::slice::from_ref(&second));
    s.assign(&[second.clone(), first.clone()]);
    assert_ne!(s.devices[&first.key].player, s.devices[&second.key].player);
}

#[test]
fn disconnected_dolphin_players_are_disabled_and_do_not_share_the_survivor() {
    let h = Home::new();
    let a = device();
    let mut b = a.clone();
    b.key = "other".into();
    b.name_index = 1;
    let mut s = Settings::default();
    s.assign(&[a.clone(), b.clone()]);
    h.put(
        ".var/app/org.DolphinEmu.dolphin-emu/config/dolphin-emu/WiimoteNew.ini",
        "[Wiimote1]\nSource=1\n",
    );
    let plan = profiles::build(&h.0, &[a, b.clone()], &s).unwrap();
    storage::apply(&h.0, &plan.changes).unwrap();
    b.name_index = 0;
    s.assign(std::slice::from_ref(&b));
    let plan = profiles::build(&h.0, &[b], &s).unwrap();
    let global = &plan
        .changes
        .iter()
        .find(|c| c.path.ends_with("Dolphin.ini"))
        .unwrap()
        .content;
    assert!(global.contains("SIDevice0 = 0"), "{global}");
    assert!(global.contains("SIDevice1 = 6"));
    let w = &plan
        .changes
        .iter()
        .find(|c| c.path.ends_with("WiimoteNew.ini"))
        .unwrap()
        .content;
    assert!(w.contains("[Wiimote1]\nSource = 0"), "{w}");
}
#[test]
fn duplicate_managed_ini_keys_cannot_override_the_saved_value() {
    let result = profiles::patch_ini(
        "[Pad]\nA=stale\nA=last-stale\n[Other]\nKeep=yes\n[Pad]\nA=also-stale\n",
        "Pad",
        &[("A".into(), "new".into())].into(),
    );
    assert_eq!(
        result
            .lines()
            .filter(|l| l.starts_with("A"))
            .collect::<Vec<_>>(),
        vec!["A = new"]
    );
    assert!(result.contains("Keep=yes"));
}
#[test]
fn kart_and_brawl_get_independent_classic_ports_and_keyboard_fallback() {
    let h = Home::new();
    let a = device();
    let mut b = a.clone();
    b.key = "second".into();
    b.name_index = 1;
    let mut s = Settings::default();
    s.assign(&[a.clone(), b.clone()]);
    h.put(
        ".var/app/org.DolphinEmu.dolphin-emu/config/dolphin-emu/WiimoteNew.ini",
        "[Wiimote1]\n",
    );
    let p = profiles::build(&h.0, &[a, b], &s).unwrap();
    for id in ["RMCE01", "RSBE01"] {
        let c = &p
            .changes
            .iter()
            .find(|c| c.path.ends_with(&format!("GameSettings/{id}.ini")))
            .expect("game override")
            .content;
        assert!(c.contains(&format!("WiimoteProfile1 = ControllerStudio-{id}-P1")));
        assert!(c.contains(&format!("WiimoteProfile2 = ControllerStudio-{id}-P2")));
        assert!(c.contains("WiimoteSource1 = 1"));
        assert!(c.contains("PadType0 = 0"));
    }
    let c = &p
        .changes
        .iter()
        .find(|c| c.path.ends_with("ControllerStudio-P1-wii-classic.ini"))
        .unwrap()
        .content;
    assert!(c.contains("XInput2/0/Virtual core pointer:X"));
    assert!(
        c.lines().any(|line| line.starts_with("Buttons/A =")
            && line.contains("Button S")
            && line.contains("Virtual core pointer:X")),
        "Classic A must also wake the underlying Wii Remote"
    );
}
#[test]
fn playstation_symbols_and_two_players_are_explicit() {
    use controller_studio::model::{self, Family};
    for (vendor, name, product) in [
        (0x054c, "DualSense Wireless Controller", 0x0ce6),
        (0x054c, "Wireless Controller", 0x09cc),
    ] {
        let mut d = device();
        d.vendor = vendor;
        d.product = product;
        d.name = name.into();
        d.family = model::family(vendor, name);
        d.buttons = model::labels(d.family);
        assert_eq!(d.family, Family::PlayStation);
        assert_eq!(d.buttons["south"], "✕");
        assert_eq!(d.buttons["east"], "○");
        let h = Home::new();
        h.put(
            ".var/app/org.DolphinEmu.dolphin-emu/config/dolphin-emu/WiimoteNew.ini",
            "[Wiimote1]\n",
        );
        let mut other = d.clone();
        other.key = "ps2".into();
        other.name_index = 1;
        let mut s = Settings::default();
        s.assign(&[d.clone(), other.clone()]);
        let p = profiles::build(&h.0, &[d, other], &s).unwrap();
        let first = &p
            .changes
            .iter()
            .find(|c| c.path.ends_with("ControllerStudio-P1-wii-classic.ini"))
            .unwrap()
            .content;
        let second = &p
            .changes
            .iter()
            .find(|c| c.path.ends_with("ControllerStudio-P2-wii-classic.ini"))
            .unwrap()
            .content;
        assert!(first.contains("SDL/0/PS"));
        assert!(second.contains("SDL/1/PS"));
        assert!(!second.contains("SDL/0/PS"));
    }
}
#[test]
fn cemu_surviving_second_player_stays_in_second_slot() {
    let h = Home::new();
    let a = device();
    let mut b = a.clone();
    b.key = "b".into();
    b.guid_index = 1;
    let mut s = Settings::default();
    s.assign(&[a.clone(), b.clone()]);
    h.put(
        ".var/app/info.cemu.Cemu/config/Cemu/controllerProfiles/controller0.xml",
        "<emulated_controller/>",
    );
    let p = profiles::build(&h.0, &[a, b.clone()], &s).unwrap();
    storage::apply(&h.0, &p.changes).unwrap();
    b.guid_index = 0;
    let p = profiles::build(&h.0, &[b], &s).unwrap();
    let first = &p
        .changes
        .iter()
        .find(|c| c.path.ends_with("/controller0.xml"))
        .unwrap()
        .content;
    let second = &p
        .changes
        .iter()
        .find(|c| c.path.ends_with("/controller1.xml"))
        .unwrap()
        .content;
    assert!(!first.contains("<api>SDL"));
    assert!(second.contains("<api>SDL"));
}

#[test]
fn saved_primary_survives_reverse_connection_order_and_secondary_only() {
    let mut primary = device();
    primary.key = "stable:primary".into();
    let mut secondary = primary.clone();
    secondary.key = "stable:secondary".into();
    let mut prefs = Settings::default();
    prefs.assign(&[primary.clone(), secondary.clone()]);
    let encoded = serde_json::to_string(&prefs).unwrap();
    let mut restored: Settings = serde_json::from_str(&encoded).unwrap();
    restored.assign(&[secondary.clone(), primary.clone()]);
    assert_eq!(restored.devices[&primary.key].player, 1);
    assert_eq!(restored.devices[&secondary.key].player, 2);
    let mut newcomer = primary.clone();
    newcomer.key = "stable:new".into();
    restored.assign(&[secondary.clone(), newcomer.clone()]);
    assert_eq!(restored.devices[&secondary.key].player, 2);
    assert_eq!(
        restored.devices[&newcomer.key].player, 3,
        "an absent primary keeps its role"
    );
}

#[test]
fn upgrade_keeps_existing_saved_roles_and_custom_buttons() {
    let mut d = device();
    let mut prefs = Settings::default();
    prefs.assign(&[d.clone()]);
    prefs.devices.get_mut(&d.key).unwrap().player = 2;
    let custom = controller_studio::model::defaults(d.family, Mode::Positions, "ds");
    prefs
        .devices
        .get_mut(&d.key)
        .unwrap()
        .mappings
        .insert("ds".into(), custom.clone());
    d.key = format!("stable:{}:port:abcd", d.guid);
    prefs.assign(&[d.clone()]);
    assert_eq!(prefs.devices[&d.key].player, 2);
    assert_eq!(prefs.bindings(&d, "ds"), custom);
    assert_eq!(prefs.devices.len(), 1);
}

#[test]
fn dolphin_input_does_not_depend_on_window_focus_and_game_presets_stay_independent() {
    let h = Home::new();
    let d = device();
    let mut prefs = Settings::default();
    prefs.assign(std::slice::from_ref(&d));
    h.put(
        ".var/app/org.DolphinEmu.dolphin-emu/config/dolphin-emu/WiimoteNew.ini",
        "[Wiimote1]\n",
    );
    h.put(".var/app/org.DolphinEmu.dolphin-emu/config/dolphin-emu/Dolphin.ini", "[Input]\nBackgroundInput = False\n[Core]\nEmulationSpeed = 1.0\n[Interface]\nLanguageCode = en\n");
    let plan = profiles::build(&h.0, &[d], &prefs).unwrap();
    let global = &plan
        .changes
        .iter()
        .find(|c| c.path.ends_with("/Dolphin.ini"))
        .unwrap()
        .content;
    assert!(global.contains("BackgroundInput = True"));
    assert!(global.contains("EmulationSpeed = 1.0"));
    assert!(global.contains("LanguageCode = en"));
    let mario = &plan
        .changes
        .iter()
        .find(|c| c.path.ends_with("GameSettings/SMNE01.ini"))
        .unwrap()
        .content;
    assert!(mario.contains("WiimoteProfile1 = ControllerStudio-SMNE01-P1"));
    let brawl = &plan
        .changes
        .iter()
        .find(|c| c.path.ends_with("GameSettings/RSBE01.ini"))
        .unwrap()
        .content;
    assert!(brawl.contains("WiimoteProfile1 = ControllerStudio-RSBE01-P1"));
}

#[test]
fn performance_settings_can_be_restored_with_the_controller_backup() {
    let h = Home::new();
    let path = ".var/app/org.DolphinEmu.dolphin-emu/config/dolphin-emu/GFX.ini";
    h.put(path, "[Settings]\nMSAA = 8\n");
    storage::apply(
        &h.0,
        &[storage::Change {
            path: path.into(),
            content: "[Settings]\nMSAA = 1\n".into(),
            note: "Performance baseline".into(),
        }],
    )
    .unwrap();
    storage::restore(&h.0).unwrap();
    assert_eq!(
        fs::read_to_string(h.0.join(path)).unwrap(),
        "[Settings]\nMSAA = 8\n"
    );
}

#[test]
fn game_layout_isolated_from_shared_and_disconnected_slots_are_cleared() {
    let h = Home::new();
    let a = device();
    let mut b = a.clone();
    b.key = "other".into();
    b.name_index = 1;
    let mut s = Settings::default();
    s.assign(&[a.clone(), b.clone()]);
    let shared = s.bindings(&a, "wii-classic");
    let mut custom = shared.clone();
    custom.insert("A".into(), "north".into());
    s.devices.get_mut(&a.key).unwrap().games.insert(
        "RSBE01".into(),
        controller_studio::model::GamePreference {
            mode: Mode::Labels,
            bindings: custom,
        },
    );
    h.put(
        ".var/app/org.DolphinEmu.dolphin-emu/config/dolphin-emu/WiimoteNew.ini",
        "[Wiimote1]\n",
    );
    let p = profiles::build(&h.0, &[a.clone(), b.clone()], &s).unwrap();
    assert_eq!(s.bindings(&a, "wii-classic"), shared);
    let profile = &p
        .changes
        .iter()
        .find(|c| c.path.ends_with("ControllerStudio-RSBE01-P1.ini"))
        .expect("isolated profile")
        .content;
    assert!(
        profile
            .lines()
            .any(|l| l.starts_with("Classic/Buttons/A =") && l.contains("Button N"))
    );
    let kart = &p
        .changes
        .iter()
        .find(|c| c.path.ends_with("GameSettings/RMCE01.ini"))
        .unwrap()
        .content;
    assert!(kart.contains("ControllerStudio-RMCE01-P1"));
    let p = profiles::build(&h.0, &[b], &s).unwrap();
    let inactive = &p
        .changes
        .iter()
        .find(|c| c.path.ends_with("ControllerStudio-RSBE01-P1.ini"))
        .unwrap()
        .content;
    assert!(!inactive.contains("Button N"));
    assert!(inactive.contains("Device ="));
}
#[test]
fn ds_uses_original_sdl_index_of_highest_priority_pad_and_scales() {
    let h = Home::new();
    let mut a = device();
    a.sdl_index = 4;
    let mut b = a.clone();
    b.key = "secondary".into();
    b.sdl_index = 1;
    let mut s = Settings::default();
    s.assign(&[a.clone(), b.clone()]);
    h.put(".var/app/net.kuribo64.melonDS/config/melonDS/melonDS.toml","[Instance0]\nJoystickID=1\n[Instance0.Keyboard]\nA=88\n[Instance0.Window0]\nIntegerScaling=true\n[Screen]\nUseGL=false\n");
    let p = profiles::build(&h.0, &[b, a], &s).unwrap();
    let text = &p
        .changes
        .iter()
        .find(|c| c.path.ends_with("config/melonDS/melonDS.toml"))
        .unwrap()
        .content;
    assert!(text.contains("JoystickID = 4"));
    assert!(text.contains("IntegerScaling = false"));
    assert!(text.contains("UseGL = true"));
    assert!(text.contains("HK_FullscreenToggle = 16777274"));
    assert!(text.contains("A = 88"));
}
#[test]
fn ds_game_config_is_separate_and_keeps_rom_adjacent_save_defaults() {
    let h = Home::new();
    let d = device();
    let mut s = Settings::default();
    s.assign(std::slice::from_ref(&d));
    let mut mapping = s.bindings(&d, "ds");
    mapping.insert("A".into(), "north".into());
    s.devices.get_mut(&d.key).unwrap().games.insert(
        "ASME".into(),
        controller_studio::model::GamePreference {
            mode: Mode::Labels,
            bindings: mapping,
        },
    );
    h.put(
        ".var/app/net.kuribo64.melonDS/config/melonDS/melonDS.toml",
        "[Instance0]\nSaveFilePath=\"\"\n[Instance0.Joystick]\nA=0\n[Emu]\nConsoleType=0\n",
    );
    let p = profiles::build(&h.0, std::slice::from_ref(&d), &s).unwrap();
    let text = &p
        .changes
        .iter()
        .find(|c| c.path.contains("controller-studio-games/ASME/"))
        .expect("isolated DS config")
        .content;
    assert!(text.contains("A = 3"));
    assert!(text.contains("SaveFilePath = \"\""));
    let shared = &p
        .changes
        .iter()
        .find(|c| c.path.ends_with("config/melonDS/melonDS.toml"))
        .unwrap()
        .content;
    assert!(shared.contains("A = 0"));
    assert!(controller_studio::games::find("../../bad").is_err());
    assert!(!storage::allowed(
        ".var/app/net.kuribo64.melonDS/config/controller-studio-games/../../etc/passwd"
    ));
}

#[test]
fn ds_trigger_and_inverted_stick_directions_match_melonds_format() {
    let m = "guid,name,lefttrigger:a2,leftx:-a0,";
    assert_eq!(
        profiles::ds_raw(m, "lt").unwrap(),
        (2 << 24) | (2 << 20) | 0x1ffff
    );
    assert_ne!(
        profiles::ds_raw(m, "lx-").unwrap(),
        profiles::ds_raw(m, "lx+").unwrap()
    );
}

#[test]
fn ds_toml_comments_do_not_bypass_firmware_guards_and_keyboard_launch_clears_stale_pad() {
    let h = Home::new();
    h.put(
        ".var/app/net.kuribo64.melonDS/config/melonDS/melonDS.toml",
        "[Emu] # valid TOML comment\nExternalBIOSEnable = true # also valid\nConsoleType = 0\n",
    );
    assert!(controller_studio::games::check_ds_launch(&h.0, "ASME").is_err());
    let text=profiles::ds_keyboard_profile("[Instance0] # existing\nJoystickID=4\n[Instance0.Joystick]\nA=2\n[Instance0.Keyboard]\nA=88\n").unwrap();
    let config: toml::Value = toml::from_str(&text).unwrap();
    assert_eq!(config["Instance0"]["JoystickID"].as_integer(), Some(-1));
    assert_eq!(config["Instance0"]["Joystick"]["A"].as_integer(), Some(-1));
    assert_eq!(config["Instance0"]["Keyboard"]["A"].as_integer(), Some(88));
}
#[test]
fn game_lookup_rejects_header_mismatch_and_symlinked_library() {
    let h = Home::new();
    let game = controller_studio::games::find("ASME").unwrap();
    let rel = format!("Games/Nintendo/DS/{}", game.filename);
    h.put(&rel, "not a matching game");
    assert!(controller_studio::games::resolve(&h.0, "ASME").is_err());
    let mut bytes = vec![0u8; 96];
    bytes[12..16].copy_from_slice(b"ASME");
    fs::write(h.0.join(&rel), bytes).unwrap();
    assert!(controller_studio::games::resolve(&h.0, "ASME").is_ok());
    fs::remove_file(h.0.join(&rel)).unwrap();
    std::os::unix::fs::symlink("/etc/passwd", h.0.join(&rel)).unwrap();
    assert!(controller_studio::games::resolve(&h.0, "ASME").is_err());
}
