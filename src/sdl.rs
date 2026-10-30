//! SDL2 owns device discovery and standardization. All FFI stays in this module.
//! The shared library is loaded only from the system's absolute library paths.
use crate::model::{self, Device};
use libloading::Library;
use std::{
    collections::BTreeMap,
    ffi::{CStr, c_char, c_int, c_void},
};
#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
    data: [u8; 16],
}
type Pad = *mut c_void;
struct Api {
    _library: Library,
    init: unsafe extern "C" fn(u32) -> c_int,
    quit: unsafe extern "C" fn(),
    hint: unsafe extern "C" fn(*const c_char, *const c_char) -> c_int,
    count: unsafe extern "C" fn() -> c_int,
    is_controller: unsafe extern "C" fn(c_int) -> c_int,
    instance: unsafe extern "C" fn(c_int) -> c_int,
    guid: unsafe extern "C" fn(c_int) -> Guid,
    guid_string: unsafe extern "C" fn(Guid, *mut c_char, c_int),
    vendor: unsafe extern "C" fn(c_int) -> u16,
    product: unsafe extern "C" fn(c_int) -> u16,
    version: unsafe extern "C" fn(c_int) -> u16,
    path: unsafe extern "C" fn(c_int) -> *const c_char,
    open: unsafe extern "C" fn(c_int) -> Pad,
    close: unsafe extern "C" fn(Pad),
    name: unsafe extern "C" fn(Pad) -> *const c_char,
    serial: unsafe extern "C" fn(Pad) -> *const c_char,
    mapping: unsafe extern "C" fn(Pad) -> *mut c_char,
    free: unsafe extern "C" fn(*mut c_void),
    update: unsafe extern "C" fn(),
    button: unsafe extern "C" fn(Pad, c_int) -> u8,
    axis: unsafe extern "C" fn(Pad, c_int) -> i16,
    has_rumble: unsafe extern "C" fn(Pad) -> c_int,
    rumble: unsafe extern "C" fn(Pad, u16, u16, u32) -> c_int,
    error: unsafe extern "C" fn() -> *const c_char,
}
impl Api {
    fn load() -> Result<Self, String> {
        let library = [
            "/lib/x86_64-linux-gnu/libSDL2-2.0.so.0",
            "/usr/lib/x86_64-linux-gnu/libSDL2-2.0.so.0",
        ]
        .iter()
        .find_map(|p| {
            // SAFETY: fixed administrator-owned system library paths, never user input.
            unsafe { Library::new(*p).ok() }
        })
        .ok_or("SDL2 is missing. Install Mint's libsdl2-2.0-0 package.")?;
        // SAFETY: these signatures match SDL 2.26+'s public C API. The owned Library
        // remains alive for every function call, including all controller cleanup.
        unsafe {
            macro_rules! sym {
                ($n:literal) => {
                    *library
                        .get(concat!($n, "\0").as_bytes())
                        .map_err(|e| format!("{}: {}", $n, e))?
                };
            }
            Ok(Self {
                init: sym!("SDL_Init"),
                quit: sym!("SDL_Quit"),
                hint: sym!("SDL_SetHint"),
                count: sym!("SDL_NumJoysticks"),
                is_controller: sym!("SDL_IsGameController"),
                instance: sym!("SDL_JoystickGetDeviceInstanceID"),
                guid: sym!("SDL_JoystickGetDeviceGUID"),
                guid_string: sym!("SDL_JoystickGetGUIDString"),
                vendor: sym!("SDL_JoystickGetDeviceVendor"),
                product: sym!("SDL_JoystickGetDeviceProduct"),
                version: sym!("SDL_JoystickGetDeviceProductVersion"),
                path: sym!("SDL_JoystickPathForIndex"),
                open: sym!("SDL_GameControllerOpen"),
                close: sym!("SDL_GameControllerClose"),
                name: sym!("SDL_GameControllerName"),
                serial: sym!("SDL_GameControllerGetSerial"),
                mapping: sym!("SDL_GameControllerMapping"),
                free: sym!("SDL_free"),
                update: sym!("SDL_GameControllerUpdate"),
                button: sym!("SDL_GameControllerGetButton"),
                axis: sym!("SDL_GameControllerGetAxis"),
                has_rumble: sym!("SDL_GameControllerHasRumble"),
                rumble: sym!("SDL_GameControllerRumble"),
                error: sym!("SDL_GetError"),
                _library: library,
            })
        }
    }
}
// SAFETY: called exclusively for valid SDL-owned NUL-terminated strings. Null
// is explicitly allowed by these SDL getters. No reference escapes the copy.
unsafe fn string(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}

fn bits(path: &std::path::Path) -> Vec<u16> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut out = Vec::new();
    for (word, s) in text.split_whitespace().rev().enumerate() {
        if let Ok(n) = u64::from_str_radix(s, 16) {
            for bit in 0..64 {
                if n & (1_u64 << bit) != 0 {
                    out.push((word * 64 + bit) as u16);
                }
            }
        }
    }
    out
}
fn native(
    path: &str,
    vendor: u16,
    product: u16,
    ordinal: usize,
) -> Option<(u16, u16, Vec<u16>, Vec<u16>)> {
    let rd = |p: &std::path::Path| {
        std::fs::read_to_string(p)
            .ok()
            .and_then(|s| u16::from_str_radix(s.trim(), 16).ok())
    };
    let base = std::path::Path::new("/sys/class/input");
    let mut matches = Vec::new();
    for entry in std::fs::read_dir(base).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("event") {
            continue;
        }
        let p = entry.path().join("device");
        if rd(&p.join("id/vendor")) != Some(vendor) || rd(&p.join("id/product")) != Some(product) {
            continue;
        }
        let buttons = bits(&p.join("capabilities/key"));
        if !buttons.iter().any(|b| *b >= 0x120) {
            continue;
        }
        let n = name
            .trim_start_matches("event")
            .parse::<usize>()
            .unwrap_or(usize::MAX);
        matches.push((n, name, p, buttons));
    }
    matches.sort_by_key(|v| v.0);
    let selected = matches
        .iter()
        .find(|(_, name, _, _)| path == format!("/dev/input/{name}"))
        .or_else(|| matches.get(ordinal))?;
    Some((
        rd(&selected.2.join("id/bustype"))?,
        rd(&selected.2.join("id/version"))?,
        selected.3.clone(),
        bits(&selected.2.join("capabilities/abs")),
    ))
}

// Slots are reserved for this Input lifetime, independently of SDL's changing
// discovery ordinals. A survivor therefore never shares its preference key with
// a newly connected pad, including devices reporting duplicate serial numbers.
fn device_key(
    counts: &mut BTreeMap<String, usize>,
    guid: &str,
    vendor: u16,
    product: u16,
    serial: &str,
) -> String {
    let base = if serial.is_empty() {
        guid.to_owned()
    } else {
        format!(
            "{vendor:04x}:{product:04x}:{}",
            serial
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .take(100)
                .collect::<String>()
        )
    };
    let slot = counts.entry(base.clone()).or_default();
    let key = if serial.is_empty() {
        format!("{base}:{slot}")
    } else if *slot == 0 {
        base
    } else {
        format!("{base}:duplicate:{slot}")
    };
    *slot += 1;
    key
}

// Event numbers change across boots; phys describes the USB socket, while uniq
// identifies a paired Bluetooth device. Hex encoding is lossless (no sanitizing collisions).
fn stable_identity(guid: &str, serial: &str, uniq: &str, phys: &str) -> Option<(String, String)> {
    let (kind, value, hint) = if !serial.trim().is_empty() {
        ("serial", serial.trim(), "Saved to this controller")
    } else if !uniq.trim().is_empty() {
        ("uniq", uniq.trim(), "Saved to this paired controller")
    } else if !phys.trim().is_empty() {
        (
            "port",
            phys.trim(),
            "Saved to this USB port; use the same port",
        )
    } else {
        return None;
    };
    let hex: String = value
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Some((format!("stable:{guid}:{kind}:{hex}"), hint.into()))
}
fn linux_identity(path: &str) -> (String, String) {
    let Some(name) = std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
    else {
        return Default::default();
    };
    if !name.starts_with("event") || name.contains('/') {
        return Default::default();
    }
    let base = std::path::Path::new("/sys/class/input")
        .join(name)
        .join("device");
    (
        std::fs::read_to_string(base.join("uniq")).unwrap_or_default(),
        std::fs::read_to_string(base.join("phys")).unwrap_or_default(),
    )
}

struct OpenPad {
    pointer: Pad,
    device: Device,
}
pub struct Input {
    api: Api,
    pads: BTreeMap<i32, OpenPad>,
    pub unsupported: usize,
    key_counts: BTreeMap<String, usize>,
}
impl Input {
    pub fn new() -> Result<Self, String> {
        Self::with_native(false)
    }
    pub fn with_native(native: bool) -> Result<Self, String> {
        let api = Api::load()?;
        // SAFETY: C string literals are static and init is called exactly once here.
        unsafe {
            (api.hint)(
                c"SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS".as_ptr(),
                c"1".as_ptr(),
            );
            (api.hint)(
                c"SDL_GAMECONTROLLER_USE_BUTTON_LABELS".as_ptr(),
                c"0".as_ptr(),
            );
            if native {
                (api.hint)(c"SDL_JOYSTICK_HIDAPI".as_ptr(), c"0".as_ptr());
            } else {
                (api.hint)(c"SDL_JOYSTICK_HIDAPI_PS4".as_ptr(), c"1".as_ptr());
                (api.hint)(c"SDL_JOYSTICK_HIDAPI_PS5".as_ptr(), c"1".as_ptr());
                (api.hint)(c"SDL_JOYSTICK_HIDAPI_SWITCH".as_ptr(), c"1".as_ptr());
            }
            if (api.init)(0x2200) != 0 {
                return Err(string((api.error)()));
            }
        }
        Ok(Self {
            api,
            pads: BTreeMap::new(),
            unsupported: 0,
            key_counts: BTreeMap::new(),
        })
    }
    pub fn poll(&mut self) -> Vec<Device> {
        // SAFETY: all handles in pads originate from SDL_GameControllerOpen and are
        // used only on this thread. Removed handles are closed before being forgotten.
        unsafe {
            (self.api.update)();
            let count = (self.api.count)().clamp(0, 16);
            let live_ids: Vec<_> = (0..count).map(|i| (self.api.instance)(i)).collect();
            let mut seen = Vec::new();
            let mut guids = BTreeMap::<String, usize>::new();
            self.unsupported = 0;
            for index in 0..count {
                if (self.api.is_controller)(index) == 0 {
                    self.unsupported += 1;
                    continue;
                }
                let id = (self.api.instance)(index);
                if id < 0 {
                    continue;
                }
                seen.push(id);
                let raw = (self.api.guid)(index);
                let mut text = [0_i8; 33];
                (self.api.guid_string)(raw, text.as_mut_ptr(), 33);
                let guid = string(text.as_ptr());
                let ordinal = guids.entry(guid.clone()).or_default();
                let guid_index = *ordinal;
                *ordinal += 1;
                if !self.pads.contains_key(&id) {
                    let pointer = (self.api.open)(index);
                    if pointer.is_null() {
                        continue;
                    }
                    let name = string((self.api.name)(pointer));
                    let serial = string((self.api.serial)(pointer));
                    let vendor = (self.api.vendor)(index);
                    let product = (self.api.product)(index);
                    let mapping_ptr = (self.api.mapping)(pointer);
                    let mapping = string(mapping_ptr);
                    if !mapping_ptr.is_null() {
                        (self.api.free)(mapping_ptr.cast());
                    }
                    let family = model::family(vendor, &name);
                    let path = string((self.api.path)(index));
                    let (uniq, phys) = linux_identity(&path);
                    let stable = stable_identity(&guid, &serial, &uniq, &phys);
                    let legacy_key =
                        device_key(&mut self.key_counts, &guid, vendor, product, &serial);
                    let (key, identity_hint) = match stable {
                        Some((key, hint))
                            if !self.pads.values().any(|p| {
                                p.device.key == key && live_ids.contains(&p.device.instance)
                            }) =>
                        {
                            (key, hint)
                        }
                        _ => (
                            legacy_key.clone(),
                            "Session identity; confirm player after reconnecting".into(),
                        ),
                    };
                    let (bus, version, native_buttons, native_axes) =
                        native(&path, vendor, product, guid_index).unwrap_or((
                            u16::from_le_bytes([raw.data[0], raw.data[1]]),
                            (self.api.version)(index),
                            vec![],
                            vec![],
                        ));
                    self.pads.insert(
                        id,
                        OpenPad {
                            pointer,
                            device: Device {
                                key,
                                name,
                                identity_hint,
                                usb_port: if phys.starts_with("usb-") {
                                    phys.trim().into()
                                } else {
                                    String::new()
                                },
                                legacy_key,
                                family,
                                guid: guid.clone(),
                                guid_index,
                                sdl_index: index as usize,
                                name_index: 0,
                                vendor,
                                product,
                                version,
                                bus,
                                path,
                                instance: id,
                                sdl_mapping: mapping,
                                buttons: model::labels(family),
                                axes: vec![0.; 6],
                                values: BTreeMap::new(),
                                rumble: (self.api.has_rumble)(pointer) != 0,
                                native_buttons,
                                native_axes,
                                native_guid: String::new(),
                                native_guid_index: 0,
                            },
                        },
                    );
                }
                if let Some(pad) = self.pads.get_mut(&id) {
                    pad.device.guid_index = guid_index;
                    pad.device.sdl_index = index as usize;
                }
            }
            let removed: Vec<_> = self
                .pads
                .keys()
                .filter(|id| !seen.contains(id))
                .copied()
                .collect();
            for id in removed {
                if let Some(pad) = self.pads.remove(&id) {
                    (self.api.close)(pad.pointer);
                }
            }
            let mut name_counts = BTreeMap::<String, usize>::new();
            for pad in self.pads.values_mut() {
                let n = name_counts.entry(pad.device.name.clone()).or_default();
                pad.device.name_index = *n;
                *n += 1;
                let v = &mut pad.device.values;
                v.clear();
                for (i, key) in [
                    "south", "east", "west", "north", "back", "guide", "start", "ls", "rs", "lb",
                    "rb", "up", "down", "left", "right",
                ]
                .iter()
                .enumerate()
                {
                    v.insert(
                        (*key).into(),
                        (self.api.button)(pad.pointer, i as i32) as f32,
                    );
                }
                for axis in 0..6 {
                    let raw = (self.api.axis)(pad.pointer, axis);
                    pad.device.axes[axis as usize] = (raw as f32 / 32767.).clamp(-1., 1.);
                }
                v.insert("lt".into(), pad.device.axes[4].max(0.));
                v.insert("rt".into(), pad.device.axes[5].max(0.));
                for (i, name) in ["lx", "ly", "rx", "ry"].iter().enumerate() {
                    let n = pad.device.axes[i];
                    v.insert(format!("{name}-"), (-n).max(0.));
                    v.insert(format!("{name}+"), n.max(0.));
                }
            }
            self.pads
                .values()
                .take(4)
                .map(|p| p.device.clone())
                .collect()
        }
    }
    pub fn rumble(&self, key: &str) -> Result<(), String> {
        let pad = self
            .pads
            .values()
            .find(|p| p.device.key == key)
            .ok_or("Controller is disconnected")?;
        // SAFETY: same-thread handle is still owned by Input, fixed low intensity/time.
        unsafe {
            if (self.api.rumble)(pad.pointer, 0x2800, 0x2800, 250) != 0 {
                return Err(string((self.api.error)()));
            }
        }
        Ok(())
    }
}
impl Drop for Input {
    fn drop(&mut self) {
        // SAFETY: close each owned handle once, before SDL_Quit and library unloading.
        unsafe {
            for pad in self.pads.values() {
                (self.api.close)(pad.pointer);
            }
            (self.api.quit)();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_roles_follow_port_not_event_number_or_connection_order() {
        let a = stable_identity("guid", "", "", "usb-0000:00:14.0-3/input0").unwrap();
        let b = stable_identity("guid", "", "", "usb-0000:00:14.0-4/input0").unwrap();
        assert_ne!(a.0, b.0);
        assert_eq!(
            a,
            stable_identity("guid", "", "", "usb-0000:00:14.0-3/input0").unwrap()
        );
        assert_ne!(
            stable_identity("guid", "A-B", "", "").unwrap().0,
            stable_identity("guid", "AB", "", "").unwrap().0
        );
        assert_eq!(
            stable_identity("guid", "serial", "", "port1").unwrap(),
            stable_identity("guid", "serial", "", "port2").unwrap()
        );
        assert!(stable_identity("guid", "", "", "").is_none());
    }
    #[test]
    fn replacing_first_serialless_pad_keeps_survivor_distinct() {
        let mut counts = BTreeMap::new();
        let first = device_key(&mut counts, "guid", 1, 2, "");
        let survivor = device_key(&mut counts, "guid", 1, 2, "");
        // Removing the first pad reindexes the survivor to 0 and its replacement to 1.
        let replacement = device_key(&mut counts, "guid", 1, 2, "");
        assert_ne!(replacement, survivor);
        assert_ne!(replacement, first);
        assert_eq!(survivor, "guid:1");
    }
    #[test]
    fn duplicate_and_sanitized_serials_have_distinct_lifetime_keys() {
        let mut counts = BTreeMap::new();
        let first = device_key(&mut counts, "guid", 1, 2, "AB");
        let duplicate = device_key(&mut counts, "guid", 1, 2, "AB");
        let sanitized = device_key(&mut counts, "guid", 1, 2, "A-B");
        assert_eq!(first, "0001:0002:AB");
        assert_ne!(duplicate, first);
        assert_ne!(sanitized, first);
        assert_ne!(sanitized, duplicate);
    }
}
