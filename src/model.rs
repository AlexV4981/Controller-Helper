use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Xbox,
    #[serde(rename = "playstation")]
    PlayStation,
    Nintendo,
    Generic,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Labels,
    Positions,
}
pub type Bindings = BTreeMap<String, String>;

pub const SYSTEMS: &[&str] = &[
    "nes",
    "ds",
    "3ds",
    "gamecube",
    "wii-nunchuk",
    "wii-sideways",
    "wii-classic",
    "wiiu",
];
pub const CONTROLS: &[&str] = &[
    "none", "south", "east", "west", "north", "back", "guide", "start", "ls", "rs", "lb", "rb",
    "up", "down", "left", "right", "lt", "rt", "lx-", "lx+", "ly-", "ly+", "rx-", "rx+", "ry-",
    "ry+",
];
pub fn actions(system: &str) -> Vec<&'static str> {
    let mut a = match system {
        "nes" => vec![
            "A", "B", "Select", "Start", "Up", "Down", "Left", "Right", "Turbo A", "Turbo B",
        ],
        "ds" => vec![
            "A", "B", "X", "Y", "L", "R", "Select", "Start", "Up", "Down", "Left", "Right",
        ],
        "3ds" | "wiiu" => vec![
            "A",
            "B",
            "X",
            "Y",
            "L",
            "R",
            "ZL",
            "ZR",
            "Select",
            "Start",
            "Up",
            "Down",
            "Left",
            "Right",
            "Left stick up",
            "Left stick down",
            "Left stick left",
            "Left stick right",
            "Right stick up",
            "Right stick down",
            "Right stick left",
            "Right stick right",
        ],
        "gamecube" => vec![
            "A",
            "B",
            "X",
            "Y",
            "L",
            "R",
            "Z",
            "Start",
            "Up",
            "Down",
            "Left",
            "Right",
            "Left stick up",
            "Left stick down",
            "Left stick left",
            "Left stick right",
            "Right stick up",
            "Right stick down",
            "Right stick left",
            "Right stick right",
        ],
        "wii-nunchuk" => vec![
            "A",
            "B",
            "1",
            "2",
            "Select",
            "Start",
            "Home",
            "C",
            "Z",
            "Shake",
            "Up",
            "Down",
            "Left",
            "Right",
            "Left stick up",
            "Left stick down",
            "Left stick left",
            "Left stick right",
            "Pointer up",
            "Pointer down",
            "Pointer left",
            "Pointer right",
            "Tilt modifier",
        ],
        "wii-sideways" => vec![
            "A",
            "B",
            "1",
            "2",
            "Select",
            "Start",
            "Home",
            "Shake",
            "Up",
            "Down",
            "Left",
            "Right",
            "Tilt left",
            "Tilt right",
            "Tilt forward",
            "Tilt backward",
        ],
        "wii-classic" => vec![
            "A",
            "B",
            "X",
            "Y",
            "L",
            "R",
            "ZL",
            "ZR",
            "Select",
            "Start",
            "Home",
            "Up",
            "Down",
            "Left",
            "Right",
            "Left stick up",
            "Left stick down",
            "Left stick left",
            "Left stick right",
            "Right stick up",
            "Right stick down",
            "Right stick left",
            "Right stick right",
        ],
        _ => vec![],
    };
    if system == "wiiu" {
        a.extend(["Left stick click", "Right stick click"]);
    }
    a
}
pub fn defaults(family: Family, mode: Mode, system: &str) -> Bindings {
    let face = if mode == Mode::Labels && family == Family::Xbox {
        ["south", "east", "west", "north"]
    } else {
        ["east", "south", "north", "west"]
    };
    let base: Bindings = [
        ("A", face[0]),
        ("B", face[1]),
        ("X", face[2]),
        ("Y", face[3]),
        ("L", "lb"),
        ("R", "rb"),
        ("ZL", "lt"),
        ("ZR", "rt"),
        ("Z", "rb"),
        ("Select", "back"),
        ("Start", "start"),
        ("Home", "guide"),
        ("Up", "up"),
        ("Down", "down"),
        ("Left", "left"),
        ("Right", "right"),
        ("Left stick up", "ly-"),
        ("Left stick down", "ly+"),
        ("Left stick left", "lx-"),
        ("Left stick right", "lx+"),
        ("Right stick up", "ry-"),
        ("Right stick down", "ry+"),
        ("Right stick left", "rx-"),
        ("Right stick right", "rx+"),
        ("Left stick click", "ls"),
        ("Right stick click", "rs"),
        ("Pointer up", "ry-"),
        ("Pointer down", "ry+"),
        ("Pointer left", "rx-"),
        ("Pointer right", "rx+"),
        ("Tilt modifier", "rb"),
        ("Tilt left", "rx-"),
        ("Tilt right", "rx+"),
        ("Tilt forward", "ry-"),
        ("Tilt backward", "ry+"),
        ("1", "west"),
        ("2", "south"),
        ("C", "lb"),
        ("Shake", "rs"),
        ("Turbo A", "rb"),
        ("Turbo B", "lb"),
    ]
    .into_iter()
    .map(|(a, b)| (a.into(), b.into()))
    .collect();
    let mut out = Bindings::new();
    for a in actions(system) {
        out.insert(
            a.into(),
            base.get(a).cloned().unwrap_or_else(|| "none".into()),
        );
    }
    if system == "gamecube" {
        out.insert("L".into(), "lt".into());
        out.insert("R".into(), "rt".into());
    }
    if system == "wii-nunchuk" {
        out.insert("B".into(), "rt".into());
        out.insert("Z".into(), "lt".into());
    }
    if system == "wii-sideways" {
        out.insert("A".into(), "east".into());
        out.insert("B".into(), "lt".into());
        out.insert("Shake".into(), "rb".into());
    }
    out
}
pub fn validate(bindings: &Bindings, system: &str) -> Result<(), String> {
    if !SYSTEMS.contains(&system) {
        return Err("Unknown console profile".into());
    }
    let valid = actions(system);
    if bindings.len() != valid.len() {
        return Err("The profile must include every action".into());
    }
    for (action, physical) in bindings {
        if !valid.contains(&action.as_str()) || !CONTROLS.contains(&physical.as_str()) {
            return Err("Unknown button or action".into());
        }
    }
    Ok(())
}
pub fn family(vendor: u16, name: &str) -> Family {
    let lower = name.to_lowercase();
    if vendor == 0x045e || lower.contains("xbox") || lower.contains("x-box") {
        Family::Xbox
    } else if vendor == 0x054c
        || lower.contains("playstation")
        || lower.contains("dualsense")
        || lower.contains("dualshock")
    {
        Family::PlayStation
    } else if vendor == 0x057e || lower.contains("switch") || lower.contains("joy-con") {
        Family::Nintendo
    } else {
        Family::Generic
    }
}
pub fn labels(family: Family) -> Bindings {
    let face = match family {
        Family::Xbox => ["A", "B", "X", "Y"],
        Family::Nintendo => ["B", "A", "Y", "X"],
        Family::PlayStation => ["✕", "○", "□", "△"],
        Family::Generic => ["South", "East", "West", "North"],
    };
    let mut b: Bindings = [
        ("south", face[0]),
        ("east", face[1]),
        ("west", face[2]),
        ("north", face[3]),
        ("back", "Select / View"),
        ("start", "Start / Menu"),
        ("guide", "Home"),
        ("ls", "Left stick click"),
        ("rs", "Right stick click"),
        ("up", "D-pad ↑"),
        ("down", "D-pad ↓"),
        ("left", "D-pad ←"),
        ("right", "D-pad →"),
        ("lt", "LT"),
        ("rt", "RT"),
        ("lb", "LB"),
        ("rb", "RB"),
        ("none", "Unassigned"),
        ("lx-", "Left stick ←"),
        ("lx+", "Left stick →"),
        ("ly-", "Left stick ↑"),
        ("ly+", "Left stick ↓"),
        ("rx-", "Right stick ←"),
        ("rx+", "Right stick →"),
        ("ry-", "Right stick ↑"),
        ("ry+", "Right stick ↓"),
    ]
    .into_iter()
    .map(|(a, b)| (a.into(), b.into()))
    .collect();
    for (k, v) in match family {
        Family::PlayStation => vec![
            ("lb", "L1"),
            ("rb", "R1"),
            ("lt", "L2"),
            ("rt", "R2"),
            ("guide", "PS"),
            ("back", "Share / Create"),
            ("start", "Options"),
        ],
        Family::Nintendo => vec![
            ("lb", "L"),
            ("rb", "R"),
            ("lt", "ZL"),
            ("rt", "ZR"),
            ("guide", "Home"),
            ("back", "−"),
            ("start", "+"),
        ],
        _ => vec![],
    } {
        b.insert(k.into(), v.into());
    }
    b
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub identity_hint: String,
    #[serde(default)]
    pub usb_port: String,
    #[serde(default)]
    pub legacy_key: String,
    pub family: Family,
    pub guid: String,
    pub guid_index: usize,
    #[serde(default)]
    pub sdl_index: usize,
    #[serde(default)]
    pub name_index: usize,
    pub vendor: u16,
    pub product: u16,
    pub version: u16,
    pub bus: u16,
    pub path: String,
    pub instance: i32,
    pub sdl_mapping: String,
    pub buttons: Bindings,
    pub axes: Vec<f32>,
    pub values: BTreeMap<String, f32>,
    pub rumble: bool,
    pub native_buttons: Vec<u16>,
    pub native_axes: Vec<u16>,
    #[serde(default)]
    pub native_guid: String,
    #[serde(default)]
    pub native_guid_index: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePreference {
    pub mode: Mode,
    pub bindings: Bindings,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preference {
    pub games: BTreeMap<String, GamePreference>,
    pub player: usize,
    pub mode: Mode,
    pub mappings: BTreeMap<String, Bindings>,
}
impl Default for Preference {
    fn default() -> Self {
        Self {
            games: BTreeMap::new(),
            player: 1,
            mode: Mode::Labels,
            mappings: BTreeMap::new(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub schema: u32,
    pub auto_apply: bool,
    pub pending_apply: bool,
    pub desired_revision: u64,
    pub applied_revision: u64,
    pub last_game: Option<String>,
    pub ds_display: BTreeMap<String, String>,
    pub devices: BTreeMap<String, Preference>,
    pub wii_style: String,
    pub usb_labels: BTreeMap<String, String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: 1,
            auto_apply: true,
            pending_apply: false,
            desired_revision: 0,
            applied_revision: 0,
            last_game: None,
            ds_display: BTreeMap::new(),
            devices: BTreeMap::new(),
            wii_style: "wii-nunchuk".into(),
            usb_labels: BTreeMap::new(),
        }
    }
}
impl Settings {
    pub fn assign(&mut self, devices: &[Device]) {
        // Keep saved mappings when upgrading the old controller keys.
        for d in devices {
            if d.key.starts_with("stable:") && !self.devices.contains_key(&d.key) {
                let old = if d.legacy_key.is_empty() {
                    format!("{}:{}", d.guid, d.guid_index)
                } else {
                    d.legacy_key.clone()
                };
                if !devices.iter().any(|other| other.key == old)
                    && let Some(preference) = self.devices.remove(&old)
                {
                    self.devices.insert(d.key.clone(), preference);
                }
            }
        }
        let mut used = std::collections::BTreeSet::new();
        let mut needs_slot = Vec::new();
        for d in devices {
            match self.devices.get(&d.key) {
                Some(p) if used.insert(p.player) => (),
                _ => needs_slot.push(d.key.clone()),
            }
        }
        let reserved: std::collections::BTreeSet<_> =
            self.devices.values().map(|p| p.player).collect();
        for key in needs_slot {
            let player = (1..=4)
                .find(|n| !used.contains(n) && !reserved.contains(n))
                .or_else(|| (1..=4).find(|n| !used.contains(n)))
                .unwrap_or(4);
            used.insert(player);
            self.devices.entry(key).or_default().player = player;
        }
    }
    pub fn game_bindings(&self, d: &Device, id: &str) -> Result<Bindings, String> {
        let game = crate::games::find(id)?;
        Ok(self
            .devices
            .get(&d.key)
            .and_then(|p| p.games.get(id))
            .map(|p| p.bindings.clone())
            .unwrap_or_else(|| self.bindings(d, &game.system)))
    }
    pub fn bindings(&self, d: &Device, system: &str) -> Bindings {
        let p = self.devices.get(&d.key).cloned().unwrap_or_default();
        p.mappings
            .get(system)
            .cloned()
            .unwrap_or_else(|| defaults(d.family, p.mode, system))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xbox_printed_a_stays_a_in_nintendo_games() {
        let b = defaults(Family::Xbox, Mode::Labels, "ds");
        assert_eq!(b.get("A").map(String::as_str), Some("south"));
        assert_eq!(b.get("X").map(String::as_str), Some("west"));
    }
    #[test]
    fn position_mode_is_explicitly_different() {
        let b = defaults(Family::Xbox, Mode::Positions, "ds");
        assert_eq!(b.get("A").map(String::as_str), Some("east"));
        assert_eq!(b.get("X").map(String::as_str), Some("north"));
    }
    #[test]
    fn nintendo_printed_a_uses_the_east_button() {
        assert_eq!(
            defaults(Family::Nintendo, Mode::Labels, "ds")
                .get("A")
                .map(String::as_str),
            Some("east")
        );
    }
    #[test]
    fn all_default_layouts_are_valid_for_every_console() {
        for f in [
            Family::Xbox,
            Family::Nintendo,
            Family::PlayStation,
            Family::Generic,
        ] {
            for m in [Mode::Labels, Mode::Positions] {
                for sys in SYSTEMS {
                    assert!(validate(&defaults(f, m, sys), sys).is_ok());
                }
            }
        }
    }
    #[test]
    fn an_unknown_binding_cannot_be_written_to_an_emulator() {
        let mut b = defaults(Family::Xbox, Mode::Labels, "ds");
        b.insert("A".into(), "../bad\nprofile".into());
        assert!(validate(&b, "ds").is_err());
    }
}
