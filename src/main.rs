use controller_studio::{
    model::{self, Bindings, Mode, Settings},
    profiles,
    sdl::Input,
    storage,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs,
    io::Read,
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
const VERSION: &str = env!("CARGO_PKG_VERSION");
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Update {
    key: String,
    system: String,
    mode: Mode,
    player: usize,
    bindings: Bindings,
    auto_apply: bool,
    #[serde(default)]
    game: Option<String>,
    #[serde(default)]
    inherit: bool,
    #[serde(default)]
    display: Option<String>,
}
fn header(request: &Request, name: &'static str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|h| h.field.equiv(name))
        .map(|h| h.value.as_str().to_owned())
}
fn reply(request: Request, status: u16, content: Vec<u8>, kind: &str, cookie: Option<&str>) {
    let mut response = Response::from_data(content).with_status_code(StatusCode(status));
    for (k, v) in [
        ("Content-Type", kind),
        ("Cache-Control", "no-store"),
        ("X-Content-Type-Options", "nosniff"),
        ("Referrer-Policy", "no-referrer"),
        (
            "Content-Security-Policy",
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
    ] {
        if let Ok(h) = Header::from_bytes(k, v) {
            response.add_header(h);
        }
    }
    if let Some(cookie) = cookie
        && let Ok(h) = Header::from_bytes("Set-Cookie", cookie)
    {
        response.add_header(h);
    }
    let _ = request.respond(response);
}
fn respond(request: Request, status: u16, value: Value) {
    reply(
        request,
        status,
        serde_json::to_vec(&value).unwrap_or_else(|_| b"{}".to_vec()),
        "application/json; charset=utf-8",
        None,
    )
}
fn read_json<T: serde::de::DeserializeOwned>(request: &mut Request) -> Result<T, String> {
    if header(request, "Content-Type").as_deref() != Some("application/json") {
        return Err("Expected application/json".into());
    }
    if request.body_length().is_some_and(|n| n > 128_000) {
        return Err("Request is too large".into());
    }
    let mut body = Vec::new();
    let started = Instant::now();
    let mut reader = request.as_reader().take(128_001);
    loop {
        if started.elapsed() > Duration::from_secs(2) {
            return Err("Request body timed out".into());
        }
        let mut chunk = [0u8; 4096];
        let n = reader.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    if body.len() > 128_000 {
        return Err("Request is too large".into());
    }
    serde_json::from_slice(&body).map_err(|e| e.to_string())
}
fn nonce() -> Result<String, String> {
    let mut bytes = [0_u8; 24];
    fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|e| e.to_string())?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn fingerprint(devices: &[model::Device], settings: &Settings) -> String {
    serde_json::to_string(&(
        devices
            .iter()
            .map(|d| (&d.key, d.instance))
            .collect::<Vec<_>>(),
        settings,
    ))
    .unwrap_or_default()
}

fn native_snapshot(devices: &mut [model::Device]) {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Ok(output) = Command::new(exe).arg("--inspect-native").output() else {
        return;
    };
    if !output.status.success() || output.stdout.len() > 500_000 {
        return;
    }
    let Ok(native) = serde_json::from_slice::<Vec<model::Device>>(&output.stdout) else {
        return;
    };
    for d in devices {
        if let Some(n) = native
            .iter()
            .filter(|n| n.vendor == d.vendor && n.product == d.product && n.version == d.version)
            .nth(d.guid_index)
        {
            d.native_guid = n.guid.clone();
            d.native_guid_index = n.guid_index;
        }
    }
}
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "Controller Studio {VERSION}\n\nOpen from Mint's Games menu, plug in a controller and choose your layout.\n\nOptions: --background  --inspect  --version\nDeveloper checks: --home PATH  --port NUMBER  --export-plan  --apply-once"
        );
        return Ok(());
    }
    if args.iter().any(|a| a == "--version") {
        println!("{VERSION}");
        return Ok(());
    }
    let home = args
        .iter()
        .position(|a| a == "--home")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .ok_or("Home directory missing")?;
    if !home.is_dir() {
        return Err("Home directory does not exist".into());
    }
    let effective = fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find(|s| s.starts_with("Uid:"))
        .and_then(|s| s.split_whitespace().nth(2))
        .map(str::to_owned);
    if effective.as_deref() == Some("0") {
        return Err("Run Controller Studio as your normal desktop user, without sudo.".into());
    }
    if !args.iter().any(|a| {
        [
            "--background",
            "--inspect",
            "--inspect-native",
            "--export-plan",
            "--apply-once",
        ]
        .contains(&a.as_str())
    }) {
        use std::os::unix::process::CommandExt;
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let window = executable.with_file_name("controller-studio-native.py");
        if !window.is_file() {
            return Err("The native window is missing. Install controller-studio-native.py beside the engine; see README.md.".into());
        }
        let error = Command::new("/usr/bin/python3")
            .arg(window)
            .arg("--backend")
            .arg(executable)
            .args(&args[1..])
            .exec();
        return Err(format!("Could not open the native window: {error}"));
    }
    let is_native = args.contains(&"--inspect-native".into());
    let mut input = match Input::with_native(is_native) {
        Ok(i) => Some(i),
        Err(e) => {
            eprintln!("Controller input: {e}");
            None
        }
    };
    let mut initial = input.as_mut().map(|i| i.poll()).unwrap_or_default();
    if is_native {
        println!(
            "{}",
            serde_json::to_string(&initial).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    native_snapshot(&mut initial);
    if !args
        .iter()
        .any(|a| ["--inspect", "--inspect-native", "--export-plan"].contains(&a.as_str()))
    {
        storage::recover_interrupted(&home)?;
    }
    let mut library = controller_studio::library::Library::load(&home)?;
    // The first scan is bounded and runs once, never on controller polls.
    if !args
        .iter()
        .any(|a| ["--inspect", "--inspect-native", "--export-plan"].contains(&a.as_str()))
    {
        library.refresh(&home)?;
    }
    let mut settings = storage::load_settings(&home)?;
    if !args
        .iter()
        .any(|a| ["--inspect", "--inspect-native", "--export-plan"].contains(&a.as_str()))
        && library.migrate_settings(&mut settings)
    {
        storage::save_settings(&home, &settings)?;
    }
    settings.assign(&initial);
    if args.contains(&"--inspect".into()) {
        println!(
            "{}",
            serde_json::to_string_pretty(&initial).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if args.contains(&"--export-plan".into()) {
        let plan = profiles::build(&home, &initial, &settings)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if args.contains(&"--apply-once".into()) {
        let plan = profiles::build(&home, &initial, &settings)?;
        let receipt = storage::apply(&home, &plan.changes)?;
        storage::save_settings(&home, &settings)?;
        println!("{}", json!({"receipt":receipt,"warnings":plan.warnings}));
        return Ok(());
    }
    let port = args
        .iter()
        .position(|a| a == "--port")
        .and_then(|i| args.get(i + 1))
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(17645);
    let origin = format!("http://127.0.0.1:{port}");
    let server = match Server::http(format!("127.0.0.1:{port}")) {
        Ok(s) => s,
        Err(e) => {
            return Err(format!(
                "The helper port is already in use; an existing studio may already be open. {e}"
            ));
        }
    };
    let token = nonce()?;
    let cookie_name = format!("controller_studio_{port}");
    let cookie_value = format!("{cookie_name}={token}");
    let cookie = format!("{cookie_value}; Path=/; HttpOnly; SameSite=Strict");
    println!("Controller Studio {VERSION} — {origin}");
    let mut native_devices = initial.clone();
    let mut devices = initial;
    let mut last = fingerprint(&devices, &settings);
    let mut pending = true;
    let mut pending_manual = settings.pending_apply;
    let mut apply_error = false;
    let (runtime_tx, runtime_rx) = std::sync::mpsc::channel();
    let runtime_home = home.clone();
    std::thread::spawn(move || {
        loop {
            if runtime_tx
                .send(controller_studio::runtime::detect_with_library(
                    &controller_studio::library::Library::load(&runtime_home).unwrap_or_default(),
                ))
                .is_err()
            {
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    });
    let mut runtime = controller_studio::runtime::Runtime::default();
    let mut changed = Instant::now();
    let mut last_check = Instant::now();
    let mut saved_message = "Ready to set up your controllers".to_owned();
    let mut warnings = Vec::<String>::new();
    let mut last_receipt = None;
    let mut game_child: Option<std::process::Child> = None;
    let mut launch_status = String::new();
    loop {
        for sample in runtime_rx.try_iter() {
            if sample.game.is_some() && sample.game != settings.last_game {
                settings.last_game = sample.game.clone();
                let _ = storage::save_settings(&home, &settings);
            }
            runtime = sample;
        }
        if let Some(child) = game_child.as_mut()
            && let Some(status) = child.try_wait().map_err(|e| e.to_string())?
        {
            launch_status = if status.success() {
                "Game session closed.".into()
            } else {
                format!(
                    "Emulator exited with {status}. Details: ~/.local/state/controller-studio/last-launch.log"
                )
            };
            game_child = None;
        }
        if last_check.elapsed() > Duration::from_millis(35) {
            devices = input.as_mut().map(|i| i.poll()).unwrap_or_default();
            if devices.iter().map(|d| d.instance).collect::<Vec<_>>()
                != native_devices
                    .iter()
                    .map(|d| d.instance)
                    .collect::<Vec<_>>()
            {
                native_snapshot(&mut devices);
                native_devices = devices.clone();
            } else {
                for d in &mut devices {
                    if let Some(n) = native_devices.iter().find(|n| n.instance == d.instance) {
                        d.native_guid = n.native_guid.clone();
                        d.native_guid_index = n.native_guid_index;
                    }
                }
            }
            settings.assign(&devices);
            let next = fingerprint(&devices, &settings);
            if next != last {
                last = next;
                pending = true;
                changed = Instant::now();
            }
            last_check = Instant::now();
        }
        if pending
            && game_child.is_none()
            && (settings.auto_apply || pending_manual)
            && !apply_error
            && storage::running_emulators().is_empty()
            && !devices.is_empty()
            && changed.elapsed() > Duration::from_secs(2)
        {
            match profiles::build(&home, &devices, &settings).and_then(|p| {
                warnings = p.warnings;
                let r = storage::apply(&home, &p.changes)?;
                Ok(r)
            }) {
                Ok(r) => {
                    saved_message = if r.changed == 0 {
                        "Your controls are already up to date".into()
                    } else {
                        format!(
                            "Controls saved for {} connected controller{}",
                            devices.len(),
                            if devices.len() == 1 { "" } else { "s" }
                        )
                    };
                    last_receipt = Some(r);
                    pending = false;
                    pending_manual = false;
                    settings.pending_apply = false;
                    settings.applied_revision = settings.desired_revision;
                    last = fingerprint(&devices, &settings);
                    let _ = storage::save_settings(&home, &settings);
                }
                Err(e) => {
                    saved_message = e;
                    apply_error = true;
                    changed = Instant::now();
                }
            }
        }
        let Some(mut request) = server
            .recv_timeout(Duration::from_millis(15))
            .map_err(|e| e.to_string())?
        else {
            continue;
        };
        if header(&request, "Host").as_deref() != Some(&format!("127.0.0.1:{port}")) {
            respond(request, 403, json!({"error":"Invalid host"}));
            continue;
        }
        let url = request.url().split('?').next().unwrap_or("").to_owned();
        let method = request.method().clone();
        if method == Method::Get && url == "/health" {
            respond(
                request,
                200,
                json!({"app":"controller-studio","version":VERSION,"home":home.canonicalize().unwrap_or_else(|_| home.clone())}),
            );
            continue;
        }
        if method == Method::Get && !url.starts_with("/api/") {
            let asset = match url.as_str() {
                "/" => Some((
                    include_bytes!("../web/index.html").as_slice(),
                    "text/html; charset=utf-8",
                )),
                "/app.js" => Some((
                    include_bytes!("../web/app.js").as_slice(),
                    "text/javascript; charset=utf-8",
                )),
                "/diagrams.mjs" => Some((
                    include_bytes!("../web/diagrams.mjs").as_slice(),
                    "text/javascript; charset=utf-8",
                )),
                "/ports.mjs" => Some((
                    include_bytes!("../web/ports.mjs").as_slice(),
                    "text/javascript; charset=utf-8",
                )),
                "/input.mjs" => Some((
                    include_bytes!("../web/input.mjs").as_slice(),
                    "text/javascript; charset=utf-8",
                )),
                "/model.js" => Some((
                    include_bytes!("../web/model.js").as_slice(),
                    "text/javascript; charset=utf-8",
                )),
                "/style.css" => Some((
                    include_bytes!("../web/style.css").as_slice(),
                    "text/css; charset=utf-8",
                )),
                "/three.module.js" => Some((
                    include_bytes!("../vendor/three.module.js").as_slice(),
                    "text/javascript; charset=utf-8",
                )),
                "/three.core.js" => Some((
                    include_bytes!("../vendor/three.core.js").as_slice(),
                    "text/javascript; charset=utf-8",
                )),
                _ => None,
            };
            if let Some((data, kind)) = asset {
                reply(
                    request,
                    200,
                    data.to_vec(),
                    kind,
                    if url == "/" { Some(&cookie) } else { None },
                );
            } else {
                respond(request, 404, json!({"error":"Not found"}));
            }
            continue;
        }
        let authenticated = header(&request, "Cookie")
            .is_some_and(|h| h.split(';').any(|s| s.trim() == cookie_value));
        let origin_ok = header(&request, "Origin").is_none_or(|o| o == origin);
        let site_ok =
            header(&request, "Sec-Fetch-Site").is_none_or(|s| s == "same-origin" || s == "none");
        if !authenticated
            || !origin_ok
            || !site_ok
            || (method == Method::Post && header(&request, "Origin").as_deref() != Some(&origin))
        {
            respond(
                request,
                403,
                json!({"error":"Open Controller Studio before continuing"}),
            );
            continue;
        }
        match (method, url.as_str()) {
            (Method::Get, "/api/state") => {
                let prepared:Vec<Value>=devices.iter().map(|d|{let maps:std::collections::BTreeMap<_,_>=model::SYSTEMS.iter().map(|sys|((*sys).to_owned(),settings.bindings(d,sys))).collect();json!({"game_layouts":library.catalog().iter().map(|g|{let own=settings.devices.get(&d.key).and_then(|p|p.games.get(&g.id));(g.id.clone(),json!({"bindings":settings.game_bindings_for(d,g),"mode":own.map(|p|p.mode).unwrap_or_else(||settings.devices.get(&d.key).map(|p|p.mode).unwrap_or(Mode::Labels)),"custom":own.is_some()}))}).collect::<std::collections::BTreeMap<_,_>>(),"device":d,"preference":settings.devices.get(&d.key),"mappings":maps,"defaults":model::SYSTEMS.iter().map(|sys|((*sys).to_owned(),model::defaults(d.family,settings.devices.get(&d.key).map(|p|p.mode).unwrap_or(Mode::Labels),sys))).collect::<std::collections::BTreeMap<_,_>>()})}).collect();
                respond(
                    request,
                    200,
                    json!({"version":VERSION,"preview":std::env::var_os("HOME").map(PathBuf::from).is_none_or(|h|h!=home),"games":library.catalog().iter().map(|g|json!({"id":g.id,"title":g.title,"system":g.system,"available":library.available(&home,&g.id)})).collect::<Vec<_>>(),"library_issues":library.issues,"launch_status":launch_status,"runtime":runtime,"last_game":settings.last_game,"ds_display":settings.ds_display,"desired_revision":settings.desired_revision,"applied_revision":settings.applied_revision,"apply_phase":controller_studio::runtime::application_phase(settings.pending_apply,game_child.is_some() || !storage::running_emulators().is_empty() || devices.is_empty(),apply_error),"controllers":prepared,"usb_labels":settings.usb_labels,"saved_roles":settings.devices.iter().map(|(key,p)|json!({"key":key,"player":p.player})).collect::<Vec<_>>(),"auto_apply":settings.auto_apply,"wii_style":settings.wii_style,"status":saved_message,"warnings":warnings,"pending":pending,"input_available":input.is_some(),"unsupported":input.as_ref().map(|i|i.unsupported).unwrap_or(0),"receipt":last_receipt,"running":storage::running_emulators(),"systems":model::SYSTEMS.iter().map(|s|(*s,model::actions(s))).collect::<std::collections::BTreeMap<_,_>>()}),
                );
            }

            (Method::Post, "/api/library/refresh") => match library.refresh(&home) {
                Ok(()) => {
                    if library.migrate_settings(&mut settings) {
                        storage::save_settings(&home, &settings)?;
                    }
                    respond(request, 200, json!({"ok":true,"issues":library.issues}));
                }
                Err(e) => respond(request, 400, json!({"error":e})),
            },
            (Method::Post, "/api/launch") => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Launch {
                    game: String,
                }
                let result: Result<Value, String> = (|| {
                    let args = read_json::<Launch>(&mut request)?;
                    let game = library.find(&args.game)?;
                    if std::env::var_os("HOME")
                        .map(PathBuf::from)
                        .is_none_or(|h| h != home)
                    {
                        return Err("Game launch is disabled in preview settings".into());
                    }
                    if game_child.is_some() || !storage::running_emulators().is_empty() {
                        return Err(
                            "Close all emulator windows before launching another game".into()
                        );
                    }
                    let path = library.resolve(&home, &game.id)?;
                    devices = input.as_mut().map(|i| i.poll()).unwrap_or_default();
                    native_snapshot(&mut devices);
                    settings.assign(&devices);
                    if game.system == "ds" && controller_studio::library::valid_id(&game.id) {
                        settings
                            .ds_display
                            .entry(game.id.clone())
                            .or_insert("dual".into());
                    }
                    let plan = profiles::build(&home, &devices, &settings)?;
                    storage::apply(&home, &plan.changes)?;
                    storage::save_settings(&home, &settings)?;
                    let mut command = Command::new("flatpak");

                    if game.system == "ds" {
                        controller_studio::games::check_ds_launch(&home, &game.id)?;
                        let config = controller_studio::games::ds_config(&game.id)?;
                        if devices.is_empty() || storage::read(&home, &config)?.is_none() {
                            let base = storage::read(
                                &home,
                                ".var/app/net.kuribo64.melonDS/config/melonDS/melonDS.toml",
                            )?
                            .ok_or("Open melonDS once before launching")?;
                            let base = profiles::ds_keyboard_profile(&base)?;
                            let base = profiles::ds_display_profile(
                                &base,
                                settings
                                    .ds_display
                                    .get(&game.id)
                                    .map(String::as_str)
                                    .unwrap_or(if game.id == "ASME" { "top" } else { "dual" }),
                            )?;
                            storage::apply(
                                &home,
                                &[storage::Change {
                                    path: config.clone(),
                                    content: base,
                                    note: "Keyboard-only DS game configuration".into(),
                                }],
                            )?;
                        }
                        controller_studio::games::copy_wfc_once(&home, &game.id)?;
                        let config_root = home
                            .join(config)
                            .parent()
                            .and_then(|p| p.parent())
                            .ok_or("Invalid game config directory")?
                            .to_path_buf();
                        command.args(controller_studio::games::ds_launch_args_with_program(
                            &config_root,
                            &path,
                            &controller_studio::games::installed_ds_program(&home),
                        ));
                    } else {
                        command
                            .arg("run")
                            .arg("org.DolphinEmu.dolphin-emu")
                            .arg("--exec")
                            .arg(&path);
                    }
                    use std::os::unix::fs::OpenOptionsExt;
                    let log_path = storage::safe_path(
                        &home,
                        ".local/state/controller-studio/last-launch.log",
                    )?;
                    let log = fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .mode(0o600)
                        .custom_flags(0x20000)
                        .open(log_path)
                        .map_err(|e| e.to_string())?;
                    let log_copy = log.try_clone().map_err(|e| e.to_string())?;
                    command
                        .current_dir(&home)
                        .stdin(std::process::Stdio::null())
                        .stdout(log)
                        .stderr(log_copy);
                    game_child = Some(
                        command
                            .spawn()
                            .map_err(|e| format!("Could not launch emulator: {e}"))?,
                    );
                    launch_status = format!(
                        "Starting {}. Close its emulator before changing controls.",
                        game.title
                    );
                    Ok(json!({"ok":true,"message":launch_status}))
                })();
                match result {
                    Ok(v) => respond(request, 200, v),
                    Err(e) => respond(request, 400, json!({"error":e})),
                }
            }
            (Method::Post, "/api/defaults") => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Preset {
                    key: String,
                    system: String,
                    mode: Mode,
                }
                let result = read_json::<Preset>(&mut request).and_then(|v| {
                    let d = devices
                        .iter()
                        .find(|d| d.key == v.key)
                        .ok_or("Controller is disconnected")?;
                    if !model::SYSTEMS.contains(&v.system.as_str()) {
                        return Err("Unknown console".into());
                    }
                    Ok(model::defaults(d.family, v.mode, &v.system))
                });
                match result {
                    Ok(b) => respond(request, 200, json!({"bindings":b})),
                    Err(e) => respond(request, 400, json!({"error":e})),
                }
            }
            (Method::Post, "/api/save") => {
                let result: Result<Value, String> = (|| {
                    let u: Update = read_json(&mut request)?;
                    model::validate(&u.bindings, &u.system)?;
                    if !(1..=4).contains(&u.player) {
                        return Err("Choose player 1–4".into());
                    }
                    if !devices.iter().any(|d| d.key == u.key) {
                        return Err("Controller was disconnected. Reconnect it first.".into());
                    }
                    let mut next = settings.clone();
                    if let Some(id) = u.game.as_deref() {
                        let game = library.find(id)?;
                        if game.system != u.system {
                            return Err("Game and controller style do not match".into());
                        }
                        let p = next
                            .devices
                            .get_mut(&u.key)
                            .ok_or("Controller preference missing")?;
                        if u.player != p.player {
                            return Err("Assign player roles from Shared presets".into());
                        }
                        if u.inherit {
                            p.games.remove(id);
                        } else {
                            p.games.insert(
                                id.into(),
                                model::GamePreference {
                                    mode: u.mode,
                                    bindings: u.bindings,
                                },
                            );
                        }
                    } else {
                        if u.inherit {
                            return Err("Choose a game to restore its shared layout".into());
                        }
                        let previous_player = next
                            .devices
                            .get(&u.key)
                            .ok_or("Controller preference missing")?
                            .player;
                        let other = next
                            .devices
                            .iter()
                            .find(|(key, p)| **key != u.key && p.player == u.player)
                            .map(|(key, _)| key.clone());
                        if let Some(other) = other
                            && let Some(p) = next.devices.get_mut(&other)
                        {
                            p.player = previous_player;
                        }
                        let p = next
                            .devices
                            .get_mut(&u.key)
                            .ok_or("Controller preference missing")?;
                        if p.mode != u.mode {
                            p.mappings.clear();
                        }
                        p.mode = u.mode;
                        p.player = u.player;
                        p.mappings.insert(u.system.clone(), u.bindings);
                        next.auto_apply = u.auto_apply;
                        if u.system.starts_with("wii-") {
                            next.wii_style = u.system.clone();
                        }
                    }
                    if let Some(display) = u.display {
                        if u.system != "ds" || !["dual", "focus", "top"].contains(&display.as_str())
                        {
                            return Err("Choose a supported DS screen layout".into());
                        }
                        next.ds_display
                            .insert(u.game.unwrap_or_else(|| "shared".into()), display);
                    }
                    let plan = profiles::build(&home, &devices, &next)?;
                    next.desired_revision = next
                        .desired_revision
                        .checked_add(1)
                        .filter(|n| *n <= 9_007_199_254_740_990)
                        .ok_or("Too many settings revisions")?;
                    next.pending_apply = true;
                    storage::save_settings(&home, &next)?;
                    settings = next;
                    apply_error = false;
                    last = fingerprint(&devices, &settings);
                    warnings = plan.warnings;
                    if game_child.is_some() || !storage::running_emulators().is_empty() {
                        pending = true;
                        pending_manual = true;
                        changed = Instant::now();
                        return Ok(
                            json!({"ok":true,"revision":settings.desired_revision,"queued":true,"message":"Close Dolphin or the running emulator completely to apply these controls"}),
                        );
                    }
                    match storage::apply(&home, &plan.changes) {
                        Ok(r) => {
                            saved_message = if r.changed == 0 {
                                "Your controls are up to date".into()
                            } else {
                                "Controls saved. Start your game when ready.".into()
                            };
                            last_receipt = Some(r.clone());
                            pending = false;
                            pending_manual = false;
                            settings.pending_apply = false;
                            settings.applied_revision = settings.desired_revision;
                            last = fingerprint(&devices, &settings);
                            storage::save_settings(&home, &settings)?;
                            Ok(
                                json!({"ok":true,"revision":settings.desired_revision,"receipt":r,"warnings":warnings}),
                            )
                        }
                        Err(e) => {
                            pending = true;
                            pending_manual = true;
                            changed = Instant::now();
                            saved_message = e.clone();
                            apply_error = true;
                            Err(e)
                        }
                    }
                })();
                match result {
                    Ok(v) => respond(request, 200, v),
                    Err(e) => respond(request, 400, json!({"error":e})),
                }
            }
            (Method::Post, "/api/restore") => match if game_child.is_some() {
                Err("Close the launched emulator before restoring controls".into())
            } else {
                storage::restore(&home)
            } {
                Ok(n) => {
                    settings.auto_apply = false;
                    settings.pending_apply = false;
                    apply_error = false;
                    let _ = storage::save_settings(&home, &settings);
                    pending = false;
                    pending_manual = false;
                    saved_message = "Earlier controls restored. Automatic apply is paused.".into();
                    respond(request, 200, json!({"ok":true,"files":n}));
                }
                Err(e) => respond(request, 400, json!({"error":e})),
            },
            (Method::Post, "/api/rumble") => {
                let result = read_json::<Value>(&mut request).and_then(|v| {
                    let key = v
                        .get("key")
                        .and_then(Value::as_str)
                        .ok_or("Choose a connected controller")?;
                    input.as_ref().ok_or("SDL input unavailable")?.rumble(key)
                });
                match result {
                    Ok(()) => respond(request, 200, json!({"ok":true})),
                    Err(e) => respond(request, 400, json!({"error":e})),
                }
            }
            (Method::Post, "/api/bluetooth") => match Command::new("blueman-manager").spawn() {
                Ok(_) => respond(request, 200, json!({"ok":true})),
                Err(_) => respond(
                    request,
                    400,
                    json!({"error":"Open Mint Bluetooth Manager from the system menu to pair your controller."}),
                ),
            },
            (Method::Post, "/api/quit") => {
                respond(request, 200, json!({"ok":true}));
                break;
            }
            _ => respond(request, 404, json!({"error":"Not found"})),
        }
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("Controller Studio: {e}");
        std::process::exit(1);
    }
}
