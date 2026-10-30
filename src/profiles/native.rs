use super::{Bindings, Device, bind};
use crate::model;
pub(super) fn pu_guid(d: &Device) -> String {
    let w = [
        d.bus.wrapping_sub(500),
        0_u16.wrapping_sub(100),
        d.vendor,
        d.vendor.wrapping_sub(200),
        d.product,
        d.product.wrapping_sub(300),
        d.version,
        d.version.wrapping_sub(400),
    ];
    format!(
        "{{{:04X}{:04X}-{:04X}-{:04X}-{:04X}-{:04X}{:04X}{:04X}}}",
        w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]
    )
}
pub(super) fn pu_button(d: &Device, physical: &str) -> Result<String, String> {
    if physical == "none" {
        return Ok("NULL".into());
    }
    let west = if d.family == model::Family::Xbox {
        0x133
    } else {
        0x134
    };
    let north = if d.family == model::Family::Xbox {
        0x134
    } else {
        0x133
    };
    let button = match physical {
        "south" => Some(0x130),
        "east" => Some(0x131),
        "west" => Some(west),
        "north" => Some(north),
        "lb" => Some(0x136),
        "rb" => Some(0x137),
        "back" => Some(0x13a),
        "start" => Some(0x13b),
        "guide" => Some(0x13c),
        "ls" => Some(0x13d),
        "rs" => Some(0x13e),
        "lt" if d.native_buttons.contains(&0x138) => Some(0x138),
        "rt" if d.native_buttons.contains(&0x139) => Some(0x139),
        _ => None,
    };
    if let Some(code) = button {
        if !d.native_buttons.contains(&code) {
            return Err("NES native button information is unavailable".into());
        }
        let index = code - 0x130 + 1;
        return Ok(format!("BTN{index:02}"));
    }
    let (code, name, positive) = match physical {
        "up" => (17, "HAT0Y", false),
        "down" => (17, "HAT0Y", true),
        "left" => (16, "HAT0X", false),
        "right" => (16, "HAT0X", true),
        "lx-" => (0, "AXS X", false),
        "lx+" => (0, "AXS X", true),
        "ly-" => (1, "AXS Y", false),
        "ly+" => (1, "AXS Y", true),
        "rx-" => (3, "AXS RX", false),
        "rx+" => (3, "AXS RX", true),
        "ry-" => (4, "AXS RY", false),
        "ry+" => (4, "AXS RY", true),
        "lt" => (2, "AXS Z", true),
        "rt" => (5, "AXS RZ", true),
        _ => return Err("This NES button requires manual setup".into()),
    };
    if !d.native_axes.contains(&code) {
        return Err("This NES native axis requires manual setup".into());
    }
    Ok(format!("{name} {}", if positive { "PLS" } else { "MIN" }))
}
pub(super) fn az_button(d: &Device, physical: &str) -> String {
    let base = format!(
        "engine:sdl,api:controller,guid:{},port:{}",
        if d.native_guid.is_empty() {
            &d.guid
        } else {
            &d.native_guid
        },
        d.native_guid_index
    );
    let index = match physical {
        "south" => Some(0),
        "east" => Some(1),
        "west" => Some(2),
        "north" => Some(3),
        "back" => Some(4),
        "guide" => Some(5),
        "start" => Some(6),
        "ls" => Some(7),
        "rs" => Some(8),
        "lb" => Some(9),
        "rb" => Some(10),
        "up" => Some(11),
        "down" => Some(12),
        "left" => Some(13),
        "right" => Some(14),
        _ => None,
    };
    if let Some(i) = index {
        return format!("{base},button:{i}");
    }
    let (axis, negative) = match physical {
        "lt" => (4, false),
        "rt" => (5, false),
        "lx-" => (0, true),
        "lx+" => (0, false),
        "ly-" => (1, true),
        "ly+" => (1, false),
        "rx-" => (2, true),
        "rx+" => (2, false),
        "ry-" => (3, true),
        "ry+" => (3, false),
        _ => return "engine:null".into(),
    };
    format!(
        "{base},axis:{axis},direction:{},threshold:{}",
        if negative { "-" } else { "+" },
        if negative { "-0.5" } else { "0.5" }
    )
}
fn nested(value: &str) -> String {
    value
        .replace('$', "$2")
        .replace(':', "$0")
        .replace(',', "$1")
}
pub(super) fn az_stick(d: &Device, b: &Bindings, right: bool) -> String {
    let word = if right { "Right" } else { "Left" };
    let standard = if right {
        ["ry-", "ry+", "rx-", "rx+"]
    } else {
        ["ly-", "ly+", "lx-", "lx+"]
    };
    let controls: Vec<_> = ["up", "down", "left", "right"]
        .iter()
        .map(|dir| bind(b, &format!("{word} stick {dir}")))
        .collect();
    if controls == standard {
        format!(
            "engine:sdl,api:controller,guid:{},port:{},axis_x:{},axis_y:{},deadzone:0.16",
            if d.native_guid.is_empty() {
                &d.guid
            } else {
                &d.native_guid
            },
            d.native_guid_index,
            if right { 2 } else { 0 },
            if right { 3 } else { 1 }
        )
    } else {
        let mut params = vec![
            "engine:analog_from_button".to_owned(),
            "modifier_scale:0.5".to_owned(),
        ];
        for (dir, physical) in ["up", "down", "left", "right"].iter().zip(controls) {
            params.push(format!("{dir}:{}", nested(&az_button(d, physical))));
        }
        params.join(",")
    }
}
