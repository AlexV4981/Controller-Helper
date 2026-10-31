use crate::{
    model::{self, Bindings, Device, Settings},
    storage::{self, Change},
};
use serde::Serialize;
use std::path::Path;
mod cemu;
mod dolphin;
mod native;
use cemu::cemu;
use dolphin::{dolphin_alias, dolphin_profile, keyboard_only};
use native::{az_button, az_stick, pu_button, pu_guid};
#[derive(Default, Serialize)]
pub struct Plan {
    pub changes: Vec<Change>,
    pub warnings: Vec<String>,
    pub applications: Vec<String>,
}

pub fn patch_ini(text: &str, section: &str, values: &Bindings) -> String {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut pending = values.clone();
    let mut found = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if current == section {
                for (k, v) in &pending {
                    out.push(format!("{k} = {v}"));
                }
                pending.clear();
            }
            current = trimmed[1..trimmed.len() - 1].to_owned();
            if current == section {
                found = true;
            }
        }
        if current == section
            && !trimmed.starts_with('#')
            && !trimmed.starts_with(';')
            && let Some((key, _)) = line.split_once('=')
            && values.contains_key(key.trim())
        {
            if let Some(value) = pending.remove(key.trim()) {
                out.push(format!("{} = {value}", key.trim()));
            }
            continue;
        }
        out.push(line.to_owned());
    }
    if !found {
        out.push(format!("[{section}]"));
    }
    for (k, v) in pending {
        out.push(format!("{k} = {v}"));
    }
    out.join("\n") + "\n"
}
pub(crate) fn section(text: &str, name: &str) -> Bindings {
    let mut active = false;
    let mut out = Bindings::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            active = &line[1..line.len() - 1] == name;
            continue;
        }
        if active
            && !line.starts_with(['#', ';'])
            && let Some((k, v)) = line.split_once('=')
        {
            out.insert(k.trim().into(), v.trim().into());
        }
    }
    out
}
fn mapping_key(physical: &str) -> Option<(&'static str, Option<bool>)> {
    Some(match physical {
        "south" => ("a", None),
        "east" => ("b", None),
        "west" => ("x", None),
        "north" => ("y", None),
        "lb" => ("leftshoulder", None),
        "rb" => ("rightshoulder", None),
        "back" => ("back", None),
        "start" => ("start", None),
        "guide" => ("guide", None),
        "ls" => ("leftstick", None),
        "rs" => ("rightstick", None),
        "up" => ("dpup", None),
        "down" => ("dpdown", None),
        "left" => ("dpleft", None),
        "right" => ("dpright", None),
        "lt" => ("lefttrigger", Some(true)),
        "rt" => ("righttrigger", Some(true)),
        "lx-" => ("leftx", Some(false)),
        "lx+" => ("leftx", Some(true)),
        "ly-" => ("lefty", Some(false)),
        "ly+" => ("lefty", Some(true)),
        "rx-" => ("rightx", Some(false)),
        "rx+" => ("rightx", Some(true)),
        "ry-" => ("righty", Some(false)),
        "ry+" => ("righty", Some(true)),
        _ => return None,
    })
}
pub fn ds_raw(mapping: &str, physical: &str) -> Result<i32, String> {
    if physical == "none" {
        return Ok(-1);
    }
    let (key, direction) = mapping_key(physical).ok_or("Unknown physical control")?;
    let value = mapping
        .split(',')
        .filter_map(|s| s.split_once(':'))
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
        .ok_or("SDL did not report a raw binding for this button")?;
    let value = value.trim();
    if let Some(n) = value.strip_prefix('b') {
        return n
            .parse::<i32>()
            .ok()
            .filter(|n| (0..256).contains(n))
            .ok_or("Invalid raw button".into());
    }
    if let Some(h) = value.strip_prefix('h')
        && let Some((hat, dir)) = h.split_once('.')
    {
        let hat = hat.parse::<i32>().map_err(|_| "Invalid hat")?;
        let dir = dir.parse::<i32>().map_err(|_| "Invalid hat direction")?;
        if (0..16).contains(&hat) && [1, 2, 4, 8].contains(&dir) {
            return Ok(0x100 | (hat << 4) | dir);
        }
    }
    let mut positive = direction.unwrap_or(true);
    if value.starts_with('-') {
        positive = !positive;
    }
    if value.ends_with('~') {
        positive = !positive;
    }
    let axis = value.trim_start_matches(['+', '-']).trim_end_matches('~');
    if let Some(n) = axis.strip_prefix('a') {
        let n = n.parse::<i32>().map_err(|_| "Invalid axis")?;
        if (0..16).contains(&n) {
            let kind = if matches!(physical, "lt" | "rt") && !value.starts_with(['+', '-']) {
                2
            } else if positive {
                0
            } else {
                1
            };
            return Ok((n << 24) | (kind << 20) | 0x10000 | 0xffff);
        }
    }
    Err("Unsupported raw SDL binding; this console was left unchanged".into())
}
pub fn ds_keyboard_profile(text: &str) -> Result<String, String> {
    let mut values = Bindings::new();
    for action in model::actions("ds") {
        values.insert(action.into(), "-1".into());
    }
    ds_config_patch(text, &values, -1)
}
pub fn ds_display_profile(text: &str, display: &str) -> Result<String, String> {
    let (layout, sizing) = match display {
        "dual" => (2, 0),
        "focus" => (2, 1),
        "top" => (0, 4),
        _ => return Err("Unknown DS display layout".into()),
    };
    let mut v: toml::Value =
        toml::from_str(text).map_err(|e| format!("Invalid melonDS TOML: {e}"))?;
    for (path, entries) in [
        (
            "Instance0.Window0",
            vec![
                ("ScreenLayout", layout),
                ("ScreenSizing", sizing),
                ("ScreenGap", 0),
            ],
        ),
        ("Instance0.Keyboard", vec![("HK_SwapScreens", 16777217)]),
    ] {
        let mut table = &mut v;
        for part in path.split('.') {
            table = table
                .as_table_mut()
                .ok_or("Invalid DS display table")?
                .entry(part)
                .or_insert_with(|| toml::Value::Table(Default::default()));
        }
        for (k, n) in entries {
            table
                .as_table_mut()
                .ok_or("Invalid DS display table")?
                .insert(k.into(), toml::Value::Integer(n));
        }
    }
    toml::to_string_pretty(&v).map_err(|e| e.to_string())
}
fn ds_profile(text: &str, d: &Device, bindings: &Bindings) -> Result<String, String> {
    let mut values = Bindings::new();
    for action in model::actions("ds") {
        values.insert(
            action.into(),
            ds_raw(&d.sdl_mapping, bind(bindings, action))?.to_string(),
        );
    }
    ds_config_patch(text, &values, d.sdl_index as i64)
}
fn ds_config_patch(text: &str, values: &Bindings, index: i64) -> Result<String, String> {
    let mut config: toml::Value =
        toml::from_str(text).map_err(|e| format!("Invalid melonDS TOML: {e}"))?;
    for (path, entries) in [
        ("Instance0.Joystick", values.clone()),
        (
            "Instance0",
            [("JoystickID".into(), index.to_string())].into(),
        ),
        (
            "Instance0.Window0",
            [
                ("IntegerScaling".into(), "false".into()),
                ("ScreenLayout".into(), "0".into()),
                ("ScreenSizing".into(), "0".into()),
            ]
            .into(),
        ),
        ("Screen", [("UseGL".into(), "true".into())].into()),
        (
            "Instance0.Keyboard",
            [("HK_FullscreenToggle".into(), "16777274".into())].into(),
        ),
    ] {
        let mut table = &mut config;
        for part in path.split('.') {
            table = table
                .as_table_mut()
                .ok_or("melonDS setting is not a table")?
                .entry(part)
                .or_insert_with(|| toml::Value::Table(Default::default()));
        }
        let table = table
            .as_table_mut()
            .ok_or("melonDS setting is not a table")?;
        for (key, value) in entries {
            let value = match value.as_str() {
                "true" => toml::Value::Boolean(true),
                "false" => toml::Value::Boolean(false),
                _ => toml::Value::Integer(
                    value.parse::<i64>().map_err(|_| "Invalid DS input value")?,
                ),
            };
            table.insert(key, value);
        }
    }
    toml::to_string_pretty(&config).map_err(|e| e.to_string())
}

fn bind<'a>(b: &'a Bindings, key: &str) -> &'a str {
    b.get(key).map(String::as_str).unwrap_or("none")
}
fn add(
    plan: &mut Plan,
    home: &Path,
    path: &str,
    content: String,
    note: &str,
) -> Result<(), String> {
    if !storage::allowed_at(home, path) {
        return Err(format!("Unapproved profile target: {path}"));
    }
    if let Some(old) = plan.changes.iter().find(|c| c.path == path) {
        if old.content != content {
            return Err("Identical NES pads share one hardware profile. Use the same NES controls for both.".into());
        }
        return Ok(());
    }
    // Check parent folders too so symlinks cannot redirect the write.
    storage::safe_path(home, path)?;
    plan.changes.push(Change {
        path: path.into(),
        content,
        note: note.into(),
    });
    Ok(())
}
const NES: &str = ".var/app/io.github.punesemu.puNES/config/puNES/";
const DS: &str = ".var/app/net.kuribo64.melonDS/config/melonDS/melonDS.toml";
const AZ: &str = ".var/app/org.azahar_emu.Azahar/config/azahar-emu/qt-config.ini";
const DOL: &str = ".var/app/org.DolphinEmu.dolphin-emu/config/dolphin-emu/";
const CEMU: &str = ".var/app/info.cemu.Cemu/config/Cemu/controllerProfiles/";
pub fn build(home: &Path, devices: &[Device], settings: &Settings) -> Result<Plan, String> {
    if !["wii-nunchuk", "wii-sideways", "wii-classic"].contains(&settings.wii_style.as_str()) {
        return Err("Unknown Wii preset".into());
    }
    let mut plan = Plan::default();
    for d in devices {
        if d.family == model::Family::Generic {
            plan.warnings.push(format!(
                "{} has no known button-symbol family. Check the assignments before playing.",
                d.name
            ));
        }
    }
    if devices.is_empty() {
        plan.warnings
            .push("Connect a controller to prepare profiles".into());
        return Ok(plan);
    }
    let mut ordered: Vec<_> = devices.iter().collect();
    ordered.sort_by_key(|d| settings.devices.get(&d.key).map(|p| p.player).unwrap_or(1));
    let first = ordered[0];
    for d in &ordered {
        for sys in model::SYSTEMS {
            model::validate(&settings.bindings(d, sys), sys)?;
        }
    }
    if let Some(mut input) = storage::read(home, &format!("{NES}input.cfg"))? {
        plan.applications.push("NES".into());
        for d in &ordered {
            let player = settings.devices.get(&d.key).map(|p| p.player).unwrap_or(1);
            if player > 2 {
                continue;
            }
            let b = settings.bindings(d, "nes");
            let mut values = Bindings::new();
            let mut failed = None;
            for a in model::actions("nes") {
                match pu_button(d, bind(&b, a)) {
                    Ok(v) => {
                        values.insert(a.replace(' ', ""), v);
                    }
                    Err(e) => {
                        failed = Some(e);
                        break;
                    }
                }
            }
            if let Some(e) = failed {
                plan.warnings.push(format!("NES, player {player}: {e}"));
                continue;
            }
            let guid = pu_guid(d);
            let target = format!(
                "{NES}jsc/{}.jsc",
                guid.chars()
                    .filter(|c| c.is_ascii_hexdigit())
                    .collect::<String>()
            );
            let old = storage::read(home, &target)?.unwrap_or_default();
            let content = patch_ini(
                &patch_ini(&old, "standard controller", &values),
                "system",
                &[
                    ("Deadzone".into(), "16".into()),
                    ("Buttons enabled".into(), "0xFFFFFFFFFFFFFFFF".into()),
                    ("Axes enabled".into(), "0xFFFFFF".into()),
                ]
                .into(),
            );
            add(&mut plan, home, &target, content, "NES printed buttons")?;
            input = patch_ini(
                &input,
                &format!("port {player}"),
                &[
                    (format!("P{player}J GUID"), guid),
                    (format!("controller {player}"), "standard".into()),
                ]
                .into(),
            );
        }
        add(
            &mut plan,
            home,
            &format!("{NES}input.cfg"),
            input,
            "NES player assignment",
        )?;
    }
    if let Some(text) = storage::read(home, DS)? {
        let updated = ds_profile(&text, first, &settings.bindings(first, "ds"))?;
        let updated = ds_display_profile(
            &updated,
            settings
                .ds_display
                .get("shared")
                .map(String::as_str)
                .unwrap_or("focus"),
        )?;
        add(
            &mut plan,
            home,
            DS,
            updated,
            "DS highest-priority controller, scaling and keyboard retained",
        )?;
        let library = crate::library::Library::load(home)?;
        for game in library.catalog().iter().filter(|g| {
            g.system == "ds"
                && (!crate::library::valid_id(&g.id)
                    || settings.ds_display.contains_key(&g.id)
                    || settings
                        .devices
                        .values()
                        .any(|p| p.games.contains_key(&g.id)))
        }) {
            let path = crate::games::ds_config(&game.id)?;
            let inherited = library
                .aliases
                .iter()
                .find(|(_, id)| **id == game.id)
                .map(|(alias, _)| crate::games::ds_config(alias))
                .transpose()?
                .map(|p| storage::read(home, &p))
                .transpose()?
                .flatten();
            let original = storage::read(home, &path)?
                .or(inherited)
                .unwrap_or_else(|| text.clone());
            let updated = ds_profile(&original, first, &settings.game_bindings_for(first, game))?;
            let updated = ds_display_profile(
                &updated,
                settings
                    .ds_display
                    .get(&game.id)
                    .map(String::as_str)
                    .unwrap_or(if game.id == "ASME" { "top" } else { "dual" }),
            )?;
            add(
                &mut plan,
                home,
                &path,
                updated,
                "Isolated DS game controls; launch from Controller Studio",
            )?;
        }
        plan.applications.push("DS".into());
    }
    if let Some(mut text) = storage::read(home, AZ)? {
        let mut values = section(&text, "Controls");
        let mut count = values
            .get("profiles\\size")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        if count > 30 {
            return Err("Too many existing 3DS profiles".into());
        }
        let mut active = 0;
        for d in &ordered {
            let player = settings.devices.get(&d.key).map(|p| p.player).unwrap_or(1);
            let name = format!("Controller Studio P{player}");
            let index = (1..=count)
                .find(|i| values.get(&format!("profiles\\{i}\\name")) == Some(&name))
                .unwrap_or_else(|| {
                    count += 1;
                    count
                });
            if d.key == first.key {
                active = index - 1;
            }
            let prefix = format!("profiles\\{index}\\");
            let b = settings.bindings(d, "3ds");
            let mut fields = Bindings::new();
            fields.insert("name".into(), name);
            fields.insert("input_maptype".into(), "2".into());
            for a in [
                "A", "B", "X", "Y", "L", "R", "ZL", "ZR", "Select", "Start", "Up", "Down", "Left",
                "Right",
            ] {
                fields.insert(
                    format!("button_{}", a.to_lowercase()),
                    format!("\"{}\"", az_button(d, bind(&b, a))),
                );
            }
            fields.insert(
                "circle_pad".into(),
                format!("\"{}\"", az_stick(d, &b, false)),
            );
            fields.insert("c_stick".into(), format!("\"{}\"", az_stick(d, &b, true)));
            fields.insert("touch_device".into(), "engine:emu_window".into());
            fields.insert(
                "motion_device".into(),
                "\"engine:motion_emu,update_period:100,sensitivity:0.01,tilt_clamp:90.0\"".into(),
            );
            for (key, value) in fields {
                values.insert(format!("{prefix}{key}"), value);
                values.insert(format!("{prefix}{key}\\default"), "false".into());
            }
        }
        values.insert("profiles\\size".into(), count.to_string());
        values.insert("profile".into(), active.to_string());
        values.insert("profile\\default".into(), "false".into());
        text = patch_ini(&text, "Controls", &values);
        add(
            &mut plan,
            home,
            AZ,
            text,
            "3DS controller profiles; keyboard retained",
        )?;
        plan.applications.push("3DS".into());
    }
    if let Some(mut wiimotes) = storage::read(home, &format!("{DOL}WiimoteNew.ini"))? {
        let mut gc = storage::read(home, &format!("{DOL}GCPadNew.ini"))?.unwrap_or_default();
        let mut global = storage::read(home, &format!("{DOL}Dolphin.ini"))?.unwrap_or_default();
        global = patch_ini(
            &global,
            "Interface",
            &[("ShowActiveTitle".into(), "True".into())].into(),
        );
        global = patch_ini(
            &global,
            "Display",
            &[
                ("Fullscreen".into(), "True".into()),
                ("RenderToMain".into(), "False".into()),
                ("FullscreenDisplayRes".into(), "Auto".into()),
            ]
            .into(),
        );
        // Keep controls working after Alt-Tab.
        global = patch_ini(
            &global,
            "Input",
            &[("BackgroundInput".into(), "True".into())].into(),
        );
        for player in 1..=4 {
            if !ordered.iter().any(|d| {
                settings
                    .devices
                    .get(&d.key)
                    .is_some_and(|p| p.player == player)
            }) {
                global = patch_ini(
                    &global,
                    "Core",
                    &[(format!("SIDevice{}", player - 1), "0".into())].into(),
                );
                wiimotes = patch_ini(
                    &wiimotes,
                    &format!("Wiimote{player}"),
                    &[("Source".into(), "0".into())].into(),
                );
                for style in ["wii-nunchuk", "wii-sideways", "wii-classic"] {
                    let path =
                        format!("{DOL}Profiles/Wiimote/ControllerStudio-P{player}-{style}.ini");
                    add(
                        &mut plan,
                        home,
                        &path,
                        "[Profile]\nDevice = \nSource = 0\n".into(),
                        "Inactive disconnected player preset",
                    )?;
                }
            }
        }
        for d in &ordered {
            let player = settings.devices.get(&d.key).map(|p| p.player).unwrap_or(1);
            if player > 4 {
                continue;
            }
            // SDL indexes follow connection order, not player number.
            let family = dolphin_alias(d);
            let ordinal = devices
                .iter()
                .take_while(|prior| prior.key != d.key)
                .filter(|prior| dolphin_alias(prior) == family)
                .count();
            for sys in ["gamecube", "wii-nunchuk", "wii-sideways", "wii-classic"] {
                let b = settings.bindings(d, sys);
                let mut v = dolphin_profile(d, &b, sys, ordinal);
                if player == 1 && sys == "wii-sideways" {
                    for (action, key) in [
                        ("Buttons/A", "C"),
                        ("Buttons/B", "V"),
                        ("Buttons/1", "Z"),
                        ("Buttons/2", "X"),
                        ("Buttons/+", "Return"),
                        ("Buttons/-", "BackSpace"),
                        ("Buttons/Home", "Home"),
                        ("D-Pad/Up", "Up"),
                        ("D-Pad/Down", "Down"),
                        ("D-Pad/Left", "Left"),
                        ("D-Pad/Right", "Right"),
                        ("Shake/X", "space"),
                        ("Shake/Y", "space"),
                        ("Shake/Z", "space"),
                        ("Tilt/Left", "Q"),
                        ("Tilt/Right", "E"),
                        ("Tilt/Forward", "R"),
                        ("Tilt/Backward", "F"),
                    ] {
                        if let Some(binding) = v.get_mut(action) {
                            *binding =
                                format!("{binding} | `XInput2/0/Virtual core pointer:{key}`");
                        }
                    }
                }
                if player == 1 && sys == "wii-classic" {
                    for (action, key) in [
                        ("Buttons/A", "X"),
                        ("Buttons/B", "Z"),
                        ("Buttons/X", "S"),
                        ("Buttons/Y", "A"),
                        ("Buttons/+", "Return"),
                        ("Buttons/-", "BackSpace"),
                        ("Triggers/L", "Q"),
                        ("Triggers/R", "W"),
                        ("Buttons/ZL", "E"),
                        ("Buttons/ZR", "R"),
                        ("Left Stick/Up", "Up"),
                        ("Left Stick/Down", "Down"),
                        ("Left Stick/Left", "Left"),
                        ("Left Stick/Right", "Right"),
                        ("D-Pad/Up", "T"),
                        ("D-Pad/Down", "G"),
                        ("D-Pad/Left", "F"),
                        ("D-Pad/Right", "H"),
                    ] {
                        if let Some(binding) = v.get_mut(&format!("Classic/{action}")) {
                            *binding =
                                format!("{binding} | `XInput2/0/Virtual core pointer:{key}`");
                        }
                    }
                }
                if player == 1
                    && sys == "wii-classic"
                    && let Some(binding) = v.get_mut("Buttons/A")
                {
                    *binding = format!("{binding} | `XInput2/0/Virtual core pointer:X`");
                }
                let folder = if sys == "gamecube" {
                    "GCPad"
                } else {
                    "Wiimote"
                };
                let target = format!("{DOL}Profiles/{folder}/ControllerStudio-P{player}-{sys}.ini");
                add(
                    &mut plan,
                    home,
                    &target,
                    patch_ini("", "Profile", &v),
                    "Selectable controller preset",
                )?;
                if sys == "gamecube" || sys == settings.wii_style {
                    let (file, group) = if sys == "gamecube" {
                        (&mut gc, format!("GCPad{player}"))
                    } else {
                        (&mut wiimotes, format!("Wiimote{player}"))
                    };
                    let previous = section(file, &group);
                    let device = previous
                        .get("Device")
                        .map(String::as_str)
                        .unwrap_or("XInput2/0/Virtual core pointer");
                    let mut v = v;
                    if player == 1 {
                        for (key, value) in &mut v {
                            if value.starts_with('(')
                                && let Some(k) =
                                    previous.get(key).and_then(|s| keyboard_only(s, device))
                            {
                                *value = format!("{value} | {k}");
                            }
                        }
                    }
                    *file = patch_ini(file, &group, &v);
                }
            }
            global = patch_ini(
                &global,
                "Core",
                &[(format!("SIDevice{}", player - 1), "6".into())].into(),
            );
        }
        add(
            &mut plan,
            home,
            &format!("{DOL}WiimoteNew.ini"),
            wiimotes,
            "Wii controls and emulated ports",
        )?;
        add(
            &mut plan,
            home,
            &format!("{DOL}GCPadNew.ini"),
            gc,
            "GameCube controls",
        )?;
        add(
            &mut plan,
            home,
            &format!("{DOL}Dolphin.ini"),
            global,
            "Enable connected GameCube ports",
        )?;
        // Load this game's controller style without changing the shared preset.
        for (id, style) in [
            ("RMCE01", "wii-classic"),
            ("RSBE01", "wii-classic"),
            ("SOJE41", "wii-classic"),
            ("SNCP8P", "wii-classic"),
            ("RSVE8P", "wii-classic"),
            ("SMNE01", "wii-sideways"),
            ("R8PE01", "wii-sideways"),
            ("RMGE01", "wii-nunchuk"),
            ("SB4E01", "wii-nunchuk"),
            ("RM8E01", "wii-nunchuk"),
            ("R4QE01", "wii-nunchuk"),
        ] {
            let path = format!(
                ".var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/GameSettings/{id}.ini"
            );
            let old = storage::read(home, &path)?.unwrap_or_default();
            let mut values = Bindings::new();
            for index in 0..4 {
                let player = index + 1;
                let connected = ordered.iter().any(|d| {
                    settings
                        .devices
                        .get(&d.key)
                        .is_some_and(|p| p.player == player)
                });
                values.insert(format!("PadType{index}"), "0".into());
                values.insert(
                    format!("WiimoteSource{index}"),
                    if connected { "1" } else { "0" }.into(),
                );
                let target = format!("{DOL}Profiles/Wiimote/ControllerStudio-{id}-P{player}.ini");
                let content = if let Some(d) = ordered.iter().find(|d| {
                    settings
                        .devices
                        .get(&d.key)
                        .is_some_and(|p| p.player == player)
                }) {
                    let base_path =
                        format!("{DOL}Profiles/Wiimote/ControllerStudio-P{player}-{style}.ini");
                    let base = plan
                        .changes
                        .iter()
                        .find(|c| c.path == base_path)
                        .ok_or("Shared Wii preset missing")?
                        .content
                        .clone();
                    if settings
                        .devices
                        .get(&d.key)
                        .is_some_and(|p| p.games.contains_key(id))
                    {
                        let ordinal = devices
                            .iter()
                            .filter(|other| dolphin_alias(other) == dolphin_alias(d))
                            .position(|other| other.key == d.key)
                            .unwrap_or(0);
                        let mut v =
                            dolphin_profile(d, &settings.game_bindings(d, id)?, style, ordinal);
                        if player == 1 {
                            for (key, value) in &mut v {
                                if value.starts_with('(')
                                    && let Some(k) =
                                        section(&base, "Profile").get(key).and_then(|old| {
                                            keyboard_only(old, "XInput2/0/Virtual core pointer")
                                        })
                                {
                                    *value = format!("{value} | {k}");
                                }
                            }
                        }
                        patch_ini("[Profile]\n", "Profile", &v)
                    } else {
                        base
                    }
                } else {
                    "[Profile]\nDevice = \nSource = 0\n".into()
                };
                add(
                    &mut plan,
                    home,
                    &target,
                    content,
                    "Independent per-game player profile",
                )?;
                values.insert(
                    format!("WiimoteProfile{player}"),
                    format!("ControllerStudio-{id}-P{player}"),
                );
            }
            add(
                &mut plan,
                home,
                &path,
                patch_ini(&old, "Controls", &values),
                "Game-specific Wii controller style and independent players",
            )?;
        }
        plan.applications.push("Wii / GameCube".into());
    }
    if storage::read(home, &format!("{CEMU}controller0.xml"))?.is_some() {
        for slot in 0..4 {
            let player = slot + 1;
            let Some(d) = ordered.iter().find(|d| {
                settings
                    .devices
                    .get(&d.key)
                    .is_some_and(|p| p.player == player)
            }) else {
                let path = format!("{CEMU}controller{slot}.xml");
                if storage::read(home, &path)?.is_some() {
                    add(
                        &mut plan,
                        home,
                        &path,
                        "<?xml version=\"1.0\"?><emulated_controller/>\n".into(),
                        "Disable disconnected managed player",
                    )?;
                }
                continue;
            };
            let b = settings.bindings(d, "wiiu");
            let pro = slot != 0;
            let text = cemu(d, &b, player, pro);
            add(
                &mut plan,
                home,
                &format!("{CEMU}ControllerStudio-P{player}.xml"),
                text.clone(),
                "Selectable Wii U controller",
            )?;
            add(
                &mut plan,
                home,
                &format!("{CEMU}controller{slot}.xml"),
                text,
                "Wii U connected player",
            )?;
        }
        plan.applications.push("Wii U".into());
    }

    for app in [
        "io.github.punesemu.puNES",
        "net.kuribo64.melonDS",
        "org.azahar_emu.Azahar",
        "org.DolphinEmu.dolphin-emu",
        "info.cemu.Cemu",
    ] {
        if home.join(".var/app").join(app).is_dir() {
            let path = format!(".local/share/flatpak/overrides/{app}");
            let old = storage::read(home, &path)?.unwrap_or_default();
            add(
                &mut plan,
                home,
                &path,
                patch_ini(
                    &old,
                    "Environment",
                    &[("SDL_GAMECONTROLLER_USE_BUTTON_LABELS".into(), "0".into())].into(),
                ),
                "Consistent physical input; printed-label policy handled by the helper",
            )?;
        }
    }
    if plan.applications.is_empty() {
        plan.warnings.push("No supported emulator configurations in this home. Open each installed emulator once, then apply again.".into());
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_xbox_a_binding_exports_raw_button_zero() {
        assert_eq!(ds_raw("id,Pad,a:b0,b:b1,dpup:h0.1,", "south").unwrap(), 0);
    }
    #[test]
    fn dpad_hat_is_not_confused_with_a_button() {
        assert_eq!(ds_raw("id,Pad,a:b0,dpup:h0.1,", "up").unwrap(), 0x101);
    }
    #[test]
    fn unrelated_video_and_keyboard_settings_survive_a_patch() {
        let text = "[Video]\nScale = 3\n[Keys]\nA = 88\n[Pad]\nA = -1\n";
        let b = [("A".into(), "0".into())].into();
        let p = patch_ini(text, "Pad", &b);
        assert!(p.contains("Scale = 3"));
        assert!(p.contains("A = 88"));
        assert!(p.contains("[Pad]\nA = 0"));
    }
    #[test]
    fn negative_and_positive_axes_keep_their_direction() {
        assert_eq!(ds_raw("id,Pad,leftx:a0,", "lx-").unwrap(), 0x11ffff);
        assert_eq!(ds_raw("id,Pad,leftx:a0,", "lx+").unwrap(), 0x1ffff);
    }
    #[test]
    fn unsupported_raw_mappings_are_refused_instead_of_guessed() {
        assert!(ds_raw("id,Pad,a:unknown,", "south").is_err());
    }
}
