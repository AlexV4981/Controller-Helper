use super::{Bindings, Device, bind};
fn clean_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || " -_().".contains(*c))
        .take(100)
        .collect()
}
pub(super) fn dolphin_alias(d: &Device) -> String {
    // Only add aliases for identified models; never bind a PS5 profile to a PS4,
    // or an Xbox One profile to an unrelated Xbox 360 controller.
    match (d.vendor, d.product) {
        (0x045e, 0x02d1 | 0x02dd) => "Xbox One Controller".into(),
        (0x045e, 0x028e) => "Xbox 360 Controller".into(),
        (0x054c, 0x05c4 | 0x09cc | 0x0ba0) => "PS4 Controller".into(),
        (0x054c, 0x0ce6) => "PS5 Controller".into(),
        (0x057e, 0x2009) => "Nintendo Switch Pro Controller".into(),
        _ => clean_name(&d.name),
    }
}
fn dolphin(d: &Device, physical: &str, ordinal: usize) -> String {
    let name = match physical {
        "motor-left" => "Motor L",
        "motor-right" => "Motor R",
        "south" => "Button S",
        "east" => "Button E",
        "west" => "Button W",
        "north" => "Button N",
        "lb" => "Shoulder L",
        "rb" => "Shoulder R",
        "lt" => "Trigger L",
        "rt" => "Trigger R",
        "back" => "Back",
        "start" => "Start",
        "guide" => "Guide",
        "ls" => "Thumb L",
        "rs" => "Thumb R",
        "up" => "Pad N",
        "down" => "Pad S",
        "left" => "Pad W",
        "right" => "Pad E",
        "lx-" => "Left X-",
        "lx+" => "Left X+",
        "ly-" => "Left Y+",
        "ly+" => "Left Y-",
        "rx-" => "Right X-",
        "rx+" => "Right X+",
        "ry-" => "Right Y+",
        "ry+" => "Right Y-",
        _ => return "0".into(),
    };
    let mut aliases = vec![format!("SDL/{}/{}", d.name_index, clean_name(&d.name))];
    {
        let alias = dolphin_alias(d);
        let a = format!("SDL/{ordinal}/{alias}");
        if !aliases.contains(&a) {
            aliases.push(a);
        }
    }
    format!(
        "({})",
        aliases
            .into_iter()
            .map(|a| format!("`{a}:{name}`"))
            .collect::<Vec<_>>()
            .join(" | ")
    )
}
pub(super) fn dolphin_profile(d: &Device, b: &Bindings, system: &str, ordinal: usize) -> Bindings {
    let mut v = Bindings::new();
    for (action, physical) in b {
        let target = match action.as_str() {
            "A" | "B" | "X" | "Y" | "1" | "2" => format!("Buttons/{action}"),
            "Start" => format!(
                "Buttons/{}",
                if system == "gamecube" { "Start" } else { "+" }
            ),
            "Select" => "Buttons/-".into(),
            "Home" => "Buttons/Home".into(),
            "Up" | "Down" | "Left" | "Right" => format!("D-Pad/{action}"),
            "L" | "R" if system == "gamecube" || system == "wii-classic" => {
                format!("Triggers/{action}")
            }
            "L" | "R" | "ZL" | "ZR" => format!("Buttons/{action}"),
            "C" => "Nunchuk/Buttons/C".into(),
            "Z" if system == "wii-nunchuk" => "Nunchuk/Buttons/Z".into(),
            "Z" => "Buttons/Z".into(),
            _ => String::new(),
        };
        if !target.is_empty() {
            let target = if system == "wii-classic" {
                format!("Classic/{target}")
            } else {
                target
            };
            v.insert(target, dolphin(d, physical, ordinal));
        }
        for (prefix, group) in [
            (
                "Left stick ",
                if system == "gamecube" {
                    "Main Stick"
                } else if system == "wii-classic" {
                    "Classic/Left Stick"
                } else {
                    "Nunchuk/Stick"
                },
            ),
            (
                "Right stick ",
                if system == "gamecube" {
                    "C-Stick"
                } else {
                    "Classic/Right Stick"
                },
            ),
            ("Pointer ", "IR"),
            ("Tilt ", "Tilt"),
        ] {
            if let Some(dir) = action.strip_prefix(prefix)
                && ["up", "down", "left", "right", "forward", "backward"].contains(&dir)
            {
                let mut chars = dir.chars();
                let dir = chars.next().unwrap().to_uppercase().collect::<String>() + chars.as_str();
                v.insert(format!("{group}/{dir}"), dolphin(d, physical, ordinal));
            }
        }
    }
    if system == "gamecube" || system == "wii-classic" {
        let prefix = if system == "wii-classic" {
            "Classic/"
        } else {
            ""
        };
        for x in ["L", "R"] {
            v.insert(
                format!("{prefix}Triggers/{x}-Analog"),
                dolphin(d, bind(b, x), ordinal),
            );
        }
    }
    if system == "wii-nunchuk" {
        let modifier = dolphin(d, bind(b, "Tilt modifier"), ordinal);
        for (dir, source) in [
            ("Left", "Pointer left"),
            ("Right", "Pointer right"),
            ("Forward", "Pointer up"),
            ("Backward", "Pointer down"),
        ] {
            v.insert(
                format!("Tilt/{dir}"),
                format!("{} & {modifier}", dolphin(d, bind(b, source), ordinal)),
            );
        }
        for dir in ["Up", "Down", "Left", "Right"] {
            let key = format!("IR/{dir}");
            if let Some(value) = v.get_mut(&key) {
                *value = format!("{value} & !{modifier}");
            }
        }
    }
    if system == "wii-classic" {
        // Inactive emulated Wii Remotes reconnect only on core button bits;
        // Classic extension buttons alone cannot wake the Bluetooth device.
        v.insert("Buttons/A".into(), dolphin(d, bind(b, "A"), ordinal));
    }
    if let Some(shake) = b.get("Shake") {
        for dir in ["X", "Y", "Z"] {
            v.insert(format!("Shake/{dir}"), dolphin(d, shake, ordinal));
        }
    }
    if system != "gamecube" {
        v.insert("Source".into(), "1".into());
        v.insert(
            "Extension".into(),
            match system {
                "wii-nunchuk" => "Nunchuk",
                "wii-classic" => "Classic",
                _ => "None",
            }
            .into(),
        );
        v.insert(
            "Options/Sideways Wiimote".into(),
            (system == "wii-sideways").to_string(),
        );
    }
    for group in if system == "gamecube" {
        vec!["Main Stick", "C-Stick"]
    } else if system == "wii-classic" {
        vec!["Classic/Left Stick", "Classic/Right Stick"]
    } else {
        vec!["Nunchuk/Stick"]
    } {
        v.insert(format!("{group}/Dead Zone"), "16.0".into());
        v.insert(format!("{group}/Calibration"), "100.00".into());
    }
    v.insert(
        "Device".into(),
        format!("SDL/{ordinal}/{}", dolphin_alias(d)),
    );
    v.insert(
        "Rumble/Motor".into(),
        format!(
            "{} | {}",
            dolphin(d, "motor-left", ordinal),
            dolphin(d, "motor-right", ordinal)
        ),
    );
    v
}
pub(super) fn keyboard_only(value: &str, device: &str) -> Option<String> {
    if value.contains("SDL/") {
        let mut tokens = Vec::new();
        let mut parts = value.split('`');
        while let Some(_) = parts.next() {
            if let Some(token) = parts.next()
                && token.starts_with("XInput2/")
            {
                tokens.push(format!("`{token}`"));
            }
        }
        if tokens.is_empty() {
            None
        } else {
            Some(tokens.join(" | "))
        }
    } else if value.contains('`') {
        Some(
            value
                .split('`')
                .enumerate()
                .map(|(i, s)| {
                    if i % 2 == 1 && !s.contains(':') {
                        format!("{device}:{s}")
                    } else {
                        s.into()
                    }
                })
                .collect(),
        )
    } else if value
        .chars()
        .all(|c| c.is_alphanumeric() || " _".contains(c))
        && !value.is_empty()
    {
        Some(format!("`{device}:{value}`"))
    } else {
        None
    }
}
