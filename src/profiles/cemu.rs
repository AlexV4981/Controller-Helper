use super::{Bindings, Device, bind};
fn cemu_code(physical: &str) -> Option<u32> {
    Some(match physical {
        "south" => 0,
        "east" => 1,
        "west" => 2,
        "north" => 3,
        "back" => 4,
        "guide" => 5,
        "start" => 6,
        "ls" => 7,
        "rs" => 8,
        "lb" => 9,
        "rb" => 10,
        "up" => 11,
        "down" => 12,
        "left" => 13,
        "right" => 14,
        "lt" => 42,
        "rt" => 43,
        "lx+" => 38,
        "ly+" => 39,
        "rx+" => 40,
        "ry+" => 41,
        "lx-" => 44,
        "ly-" => 45,
        "rx-" => 46,
        "ry-" => 47,
        _ => return None,
    })
}
fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub(super) fn cemu(d: &Device, b: &Bindings, player: usize, pro: bool) -> String {
    let mut entries = String::new();
    let base = [
        ("A", 1),
        ("B", 2),
        ("X", 3),
        ("Y", 4),
        ("L", 5),
        ("R", 6),
        ("ZL", 7),
        ("ZR", 8),
        ("Start", 9),
        ("Select", 10),
        ("Up", 11),
        ("Down", 12),
        ("Left", 13),
        ("Right", 14),
        ("Left stick click", 15),
        ("Right stick click", 16),
        ("Left stick up", 17),
        ("Left stick down", 18),
        ("Left stick left", 19),
        ("Left stick right", 20),
        ("Right stick up", 21),
        ("Right stick down", 22),
        ("Right stick left", 23),
        ("Right stick right", 24),
    ];
    for (action, mut mapping) in base {
        if pro && mapping >= 11 {
            mapping += 1;
        }
        if let Some(button) = cemu_code(bind(b, action)) {
            entries +=
                &format!("<entry><mapping>{mapping}</mapping><button>{button}</button></entry>\n");
        }
    }
    let keyboard = if pro {
        String::new()
    } else {
        "<controller><api>Keyboard</api><uuid>keyboard</uuid><display_name>Keyboard</display_name><mappings><entry><mapping>25</mapping><button>109</button></entry><entry><mapping>26</mapping><button>65289</button></entry></mappings></controller>".into()
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<emulated_controller><type>{}</type><profile>ControllerStudio-P{player}</profile><toggle_display>0</toggle_display><controller><api>SDLController</api><uuid>{}_{}</uuid><display_name>{}</display_name><rumble>0.35</rumble><axis><deadzone>0.16</deadzone><range>1</range></axis><rotation><deadzone>0.16</deadzone><range>1</range></rotation><trigger><deadzone>0.05</deadzone><range>1</range></trigger><mappings>{entries}</mappings></controller>{keyboard}</emulated_controller>\n",
        if pro {
            "Wii U Pro Controller"
        } else {
            "Wii U GamePad"
        },
        d.guid_index,
        d.guid,
        xml(&d.name)
    )
}
