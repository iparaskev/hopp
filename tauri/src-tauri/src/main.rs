// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use hopp::sounds::{self, SoundConfig};
use log::LevelFilter;
use socket_lib::{CaptureContent, Content, Extent, Message, ScreenShareMessage};
use tauri::Manager;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    path::BaseDirectory,
    Emitter,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use tauri_plugin_log::{Target, TargetKind};

use hopp::{
    app_state::AppState, create_core_process, get_log_level, get_log_path, get_sentry_dsn,
    get_token_filename, permissions, ping_frontend, setup_start_on_launch, setup_tray_icon,
    AppData,
};
use std::sync::Mutex;
use std::{env, sync::Arc};

#[cfg(target_os = "macos")]
use std::time::Duration;

#[cfg(any(target_os = "windows", target_os = "linux"))]
use tauri::PhysicalPosition;

//te
#[tauri::command]
async fn screenshare(
    app: tauri::AppHandle,
    content: Content,
    token: String,
    resolution: Extent,
) -> bool {
    log::info!("screenshare: content: {content:?}, token: {token}, resolution: {resolution:?}");

    let data = app.state::<Mutex<AppData>>();
    let mut data = data.lock().unwrap();
    let res = data
        .socket
        .send_message(Message::StartScreenShare(ScreenShareMessage {
            content,
            token: token.clone(),
            resolution,
        }));
    if let Err(e) = res {
        log::error!("screenshare: failed to send message: {e:?}");
        return false;
    }

    // TODO: Add a timeout
    let res = data.socket.receive_message();
    if let Err(e) = res {
        log::error!("screenshare: failed to receive message: {e:?}");
        return false;
    }
    match res.unwrap() {
        Message::StartScreenShareResult(result) => {
            if !result {
                log::error!("screenshare: failed to start screenshare");
                return false;
            }
        }
        _ => {
            log::error!("screenshare: unexpected message");
        }
    }

    true
}

#[tauri::command]
async fn stop_sharing(app: tauri::AppHandle) {
    log::info!("stop_sharing");
    let data = app.state::<Mutex<AppData>>();
    let mut data = data.lock().unwrap();
    let res = data.socket.send_message(Message::StopScreenshare);
    if let Err(e) = res {
        log::error!("screenshare: failed to send message: {e:?}");
    }
}

#[tauri::command]
async fn get_available_content(app: tauri::AppHandle) -> Vec<CaptureContent> {
    log::info!("get_available_content");
    let data = app.state::<Mutex<AppData>>();
    let mut data = data.lock().unwrap();
    let res = data.socket.send_message(Message::GetAvailableContent);
    if let Err(e) = res {
        log::error!("get_available_content: failed to send message: {e:?}");
        return vec![];
    }
    let res = data.socket.receive_message();
    if let Err(e) = res {
        log::error!("get_available_content: failed to receive message: {e:?}");
        return vec![];
    }
    match res.unwrap() {
        Message::AvailableContent(content) => {
            for c in &content.content {
                log::info!(
                    "get_available_content: possible content {}, content {:?}",
                    c.title,
                    c.content
                );
            }
            content.content
        }
        _ => vec![],
    }
}

#[tauri::command]
fn play_sound(app: tauri::AppHandle, sound_name: String) {
    log::info!("play_sound: {sound_name}");
    /*
     * Check if the sound is already playing, if it has finished we
     * remove the entry from the sound_entries vector.
     */
    {
        let data = app.state::<Mutex<AppData>>();
        let mut data = data.lock().unwrap();
        let mut i = 0;
        while i < data.sound_entries.len() {
            if data.sound_entries[i].name == sound_name {
                /* Send a message to see if the sound is still playing */
                let res = data.sound_entries[i].tx.send(sounds::SoundCommand::Ping);
                if res.is_err() {
                    log::debug!("play_sound: found closed channel for {sound_name}");
                    data.sound_entries.remove(i);
                    break;
                }
                log::warn!("play_sound: Sound {sound_name} is already playing");
                return;
            } else {
                i += 1;
            }
        }
    }

    let sounds = hopp::sounds::get_all_sounds();
    let mut sound_path = "".to_string();
    let mut sound_config = SoundConfig::default();
    for sound in sounds {
        if sound.0.contains(&sound_name) {
            let resource_path = app.path().resolve(sound.0, BaseDirectory::Resource);
            if let Err(e) = resource_path {
                log::error!("play_sound: Failed to resolve sound path: {e:?}");
                return;
            }
            sound_path = resource_path.unwrap().to_string_lossy().to_string();
            sound_config = sound.1;
            break;
        }
    }
    if sound_path.is_empty() {
        log::error!("play_sound: Failed to find sound: {sound_name}");
        return;
    }

    let (tx, rx) = std::sync::mpsc::channel();
    tauri::async_runtime::spawn(async move {
        let res = hopp::sounds::play_sound(sound_path, sound_config, rx);
        if res.is_err() {
            log::error!("play_sound: Failed to play sound: {:?}", res.err());
        }
    });

    let data = app.state::<Mutex<AppData>>();
    let mut data = data.lock().unwrap();
    data.sound_entries.push(sounds::SoundEntry {
        name: sound_name,
        tx,
    });
}

#[tauri::command]
fn stop_sound(app: tauri::AppHandle, sound_name: String) {
    log::info!("Stopping sound: {sound_name}");
    let data = app.state::<Mutex<AppData>>();
    let mut data = data.lock().unwrap();
    let mut i = 0;
    while i < data.sound_entries.len() {
        if data.sound_entries[i].name == sound_name {
            let _ = data.sound_entries[i].tx.send(sounds::SoundCommand::Stop);
            data.sound_entries.remove(i);
            break;
        } else {
            i += 1;
        }
    }
    log::debug!("stop_sound: entries left: {}", data.sound_entries.len());
}

#[tauri::command]
fn reset_core_process(app: tauri::AppHandle) {
    let data = app.state::<Mutex<AppData>>();
    let mut data = data.lock().unwrap();
    let res = data.socket.send_message(Message::Reset);
    if let Err(e) = res {
        log::error!("reset_core_process: failed to send message: {e:?}");
    }
}

#[tauri::command]
fn store_token_cmd(app: tauri::AppHandle, token: String) {
    let app_data_dir = match app.path().app_data_dir() {
        Ok(dir) => dir,
        Err(e) => {
            log::error!("Failed to get app data dir: {e:?}");
            return;
        }
    };

    let token_file = app_data_dir.join(get_token_filename());
    log::debug!("Storing token to: {}", token_file.display());
    match std::fs::write(token_file, token.clone()) {
        Ok(_) => {
            log::info!("Stored token");
            if let Err(e) = app.emit("token_changed", token) {
                log::error!("Failed to emit token_changed event: {e:?}");
            }
        }
        Err(e) => {
            log::error!("Failed to store token: {e:?}");
        }
    }
}

#[tauri::command]
fn get_stored_token(app: tauri::AppHandle) -> Option<String> {
    let app_data_dir = match app.path().app_data_dir() {
        Ok(dir) => dir,
        Err(e) => {
            log::error!("Failed to get app data dir: {e:?}");
            return None;
        }
    };

    let token_file = app_data_dir.join(get_token_filename());
    log::debug!("Reading token from: {}", token_file.display());
    match std::fs::read_to_string(token_file) {
        Ok(token) => Some(token),
        Err(e) => {
            log::error!("Failed to read token: {e:?}");
            None
        }
    }
}

#[tauri::command]
fn delete_stored_token(app: tauri::AppHandle) {
    log::info!("Deleting stored token");
    let app_data_dir = match app.path().app_data_dir() {
        Ok(dir) => dir,
        Err(e) => {
            log::error!("Failed to get app data dir: {e:?}");
            return;
        }
    };

    let token_file = app_data_dir.join(get_token_filename());
    if let Err(e) = std::fs::remove_file(token_file) {
        log::error!("Failed to delete token file: {e:?}");
    }

    if let Err(e) = app.emit("token_changed", "".to_string()) {
        log::error!("Failed to emit token_changed event: {e:?}");
    }
}

#[tauri::command]
fn get_logs(_app: tauri::AppHandle) -> String {
    log::info!("get_logs:");
    let log_file = get_log_path();
    if let Some(path) = log_file {
        path.to_string_lossy().to_string()
    } else {
        log::error!("Failed to get log path");
        "".to_string()
    }
}

#[tauri::command]
fn set_deactivate_hiding(app: tauri::AppHandle, deactivate: bool) {
    log::debug!("set_deactivate_hiding: {deactivate}");
    let data = app.state::<Mutex<AppData>>();
    let data = data.lock().unwrap();
    let mut deactivate_hiding = data.deactivate_hiding.lock().unwrap();
    *deactivate_hiding = deactivate;
}

#[tauri::command]
fn set_controller_cursor(app: tauri::AppHandle, enabled: bool) {
    log::info!("set_controller_cursor: {enabled}");
    let data = app.state::<Mutex<AppData>>();
    let mut data = data.lock().unwrap();
    let res = data
        .socket
        .send_message(Message::ControllerCursorEnabled(enabled));
    if let Err(e) = res {
        log::error!("set_controller_cursor: failed to send message: {e:?}");
    }
}

#[tauri::command]
fn open_accessibility_settings(_app: tauri::AppHandle) {
    log::info!("open_accessibility_settings");
    let mut process = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn()
        .expect("Failed to open System Preferences for Accessibility permissions");
    let _ = process.wait();
}

#[tauri::command]
fn open_microphone_settings(_app: tauri::AppHandle) {
    log::info!("open_microphone_settings");
    let mut process = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
        .spawn()
        .expect("Failed to open System Preferences for Microphone permissions");
    let _ = process.wait();
}

#[tauri::command]
fn open_screenshare_settings(_app: tauri::AppHandle) {
    log::info!("open_screenshare_settings");
    let mut process = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
        .spawn()
        .expect("Failed to open System Preferences for Screen Capture permissions");
    let _ = process.wait();
}

#[tauri::command]
async fn trigger_screenshare_permission(app: tauri::AppHandle) -> bool {
    log::info!("trigger_screenshare_permission");
    let content = get_available_content(app.clone()).await;
    let mut has_content = false;
    for c in content {
        if !c.base64.is_empty() {
            has_content = true;
            break;
        }
    }
    has_content
}

#[tauri::command]
fn get_control_permission(_app: tauri::AppHandle) -> bool {
    let res = permissions::accessibility();
    log::info!("get_control_permission: {res}");
    res
}

#[tauri::command]
fn get_microphone_permission(_app: tauri::AppHandle) -> bool {
    let res = permissions::microphone();
    log::info!("get_microphone_permission: {res}");
    res
}

#[tauri::command]
fn get_screenshare_permission(_app: tauri::AppHandle) -> bool {
    let res = permissions::screenshare();
    log::info!("get_screenshare_permission: {res}");
    res
}

#[tauri::command]
fn skip_tray_notification_selection_window(app: tauri::AppHandle) {
    log::info!("executing skip_tray_notification_selection_window");
    let data = app.state::<Mutex<AppData>>();
    let mut data = data.lock().unwrap();
    data.app_state.set_tray_notification(false);
}

#[allow(unused_variables)]
#[tauri::command]
fn set_dock_icon_visible(app: tauri::AppHandle, visible: bool) {
    log::info!("set_dock_icon_visible: {visible}");
    #[cfg(target_os = "macos")]
    {
        if visible {
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
        } else {
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        }

        {
            let data = app.state::<Mutex<AppData>>();
            let data = data.lock().unwrap();
            *data.dock_enabled.lock().unwrap() = visible;
        }
    }
}

#[tauri::command]
fn get_last_used_mic(app: tauri::AppHandle) -> Option<String> {
    log::info!("get_last_used_mic");
    let data = app.state::<Mutex<AppData>>();
    let data = data.lock().unwrap();
    let value = data.app_state.last_used_mic();
    log::info!("get_last_used_mic: {value:?}");
    value
}

#[tauri::command]
fn set_last_used_mic(app: tauri::AppHandle, mic: String) {
    log::info!("set_last_used_mic: {mic}");
    let data = app.state::<Mutex<AppData>>();
    let mut data = data.lock().unwrap();
    data.app_state.set_last_used_mic(mic);
}

#[tauri::command]
fn minimize_main_window(app: tauri::AppHandle) {
    log::info!("minimize_main_window");
    if let Some(window) = app.get_webview_window("main") {
        if let Err(e) = window.minimize() {
            log::error!("Failed to minimize main window: {e:?}");
        }
    } else {
        log::error!("Main window not found");
    }
}

#[tauri::command]
fn set_livekit_url(app: tauri::AppHandle, url: String) {
    log::info!("set_livekit_url: {url}");
    let data = app.state::<Mutex<AppData>>();
    let mut data = data.lock().unwrap();
    if data.livekit_server_url != url {
        data.livekit_server_url = url.clone();
        let res = data.socket.send_message(Message::LivekitServerUrl(url));
        if let Err(e) = res {
            log::error!("set_livekit_url: failed to send message: {e:?}");
        }
    }
}

#[tauri::command]
fn get_livekit_url(app: tauri::AppHandle) -> String {
    log::info!("get_livekit_url");
    let data = app.state::<Mutex<AppData>>();
    let data = data.lock().unwrap();
    data.livekit_server_url.clone()
}

fn main() {
    let _guard = sentry_utils::init_sentry("Tauri backend".to_string(), Some(get_sentry_dsn()));

    /*
     * Flag for disabling hiding the window on focus lost.
     * This is used to prevent the window from hiding when the user is writing feedback.
     */
    let deactivate_hiding = Arc::new(Mutex::new(false));
    let deactivate_hiding_clone = deactivate_hiding.clone();

    /*
     * Flag for disabling hiding the window on focus lost.
     * This is used to prevent the window from hiding when the user uses Raycast/Spotlight
     * to open the app again.
     */
    let reopen_requested = Arc::new(Mutex::new(false));
    #[allow(unused_variables)]
    let reopen_requested_clone = reopen_requested.clone();

    /* This is used to guard against hiding the dock icon if the dock has been enabled by the ui. */
    let dock_enabled = Arc::new(Mutex::new(false));

    /* This is used to guard against showing the main window if the location is not set. */
    let location_set = Arc::new(Mutex::new(false));
    let location_set_clone = location_set.clone();
    let location_set_setup = location_set.clone();

    let log_level = get_log_level();
    let mut app = tauri::Builder::default().plugin(tauri_plugin_opener::init());
    if !cfg!(debug_assertions) {
        app = app.plugin(tauri_plugin_single_instance::init(
            move |app, _args, _cwd| {
                log::info!("Reopening the app, single instance handler");
                log::debug!("app {app:?}");
                #[cfg(target_os = "macos")]
                {
                    let location_set = location_set_clone.lock().unwrap();
                    if !*location_set {
                        log::info!("Location not set, don't show the main window");
                        return;
                    }

                    let main_window = app.get_webview_window("main");
                    if let Some(window) = main_window {
                        log::info!("Single instance handler: showing main window");
                        let _ = window.show();
                        let _ = window.set_focus();
                    } else {
                        log::error!("Main window not found");
                    }
                }
            },
        ));
    }
    let app = app
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([
                    Target::new(TargetKind::LogDir { file_name: None }),
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::Webview),
                ])
                .level(LevelFilter::Warn)
                .level_for("hopp", log_level)
                .build(),
        )
        .setup(move |app| {
            let quit = MenuItemBuilder::new("Quit")
                .id("quit")
                .accelerator("Cmd+Q")
                .build(app)?;
            let menu = MenuBuilder::new(app).items(&[&quit]).build()?;

            setup_tray_icon(app, &menu, location_set_setup.clone())?;

            /* Create the app_data_dir if it doesn't exist. */
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir.");
            if !app_data_dir.exists() {
                if let Err(e) = std::fs::create_dir_all(&app_data_dir) {
                    log::error!("Failed to create app data dir: {e:?}");
                }
            }

            let (_core_process, socket) =
                create_core_process(app.handle()).expect("Failed to create core process");

            let app_state = AppState::new(&app_data_dir);
            let data = Mutex::new(AppData::new(
                socket,
                deactivate_hiding_clone,
                dock_enabled,
                app_state,
            ));
            app.manage(data);

            /* Clear app logs in the beginning of a session. */
            let dir = app.path().app_log_dir();
            if dir.is_err() {
                log::warn!("Failed to get app log dir");
            } else {
                let dir = dir.unwrap();
                let log_file = dir.join("hopp.log");
                if log_file.exists() {
                    if let Err(e) = std::fs::write(&log_file, "") {
                        log::warn!("Failed to clear log file: {e:?}");
                    }
                }
            }

            /*
             * We are sending a ping event to the frontend
             * to keep it alive.
             * TODO: do graceful shutdown on exit
             */
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                ping_frontend(app_handle);
            });

            let first_run = {
                let data = app.state::<Mutex<AppData>>();
                let data = data.lock().unwrap();
                data.app_state.first_run()
            };

            setup_start_on_launch(&app.autolaunch(), first_run);

            /* Set first run to false after checking the start on launch. */
            {
                let data = app.state::<Mutex<AppData>>();
                let mut data = data.lock().unwrap();
                if first_run {
                    data.app_state.set_first_run(false);
                }
            }

            /* Main window configuration on windows */
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            {
                let handle = app.handle();
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.set_shadow(false);
                    let _ = window.set_skip_taskbar(false);
                    /* Place window on the bottom right corner of the active display. */
                    let current_monitor = window.current_monitor();
                    if let Ok(Some(monitor)) = current_monitor {
                        let monitor_size = monitor.size();
                        let monitor_pos = monitor.position();
                        let window_size = window.inner_size().unwrap();
                        let base_offset = 20 * monitor.scale_factor() as u32;
                        let offset_y = (25. * monitor.scale_factor()) as u32 + base_offset;
                        let x = monitor_pos.x
                            + (monitor_size.width - window_size.width - base_offset) as i32;
                        let y = monitor_pos.y
                            + (monitor_size.height - window_size.height - offset_y) as i32;
                        let new_position = PhysicalPosition::new(x as f64, y as f64);
                        let _ = window.set_position(new_position);
                    }
                    let _ = window.set_always_on_top(false);
                    let _ = window.show();
                }
            }

            /* macOS specific setup */
            #[cfg(target_os = "macos")]
            {
                /* Hide dock icon on macos */
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);

                /*
                 * First show the notification window which explains that hopp lives in the
                 * menubar. Then show the permissions window if needed.
                 */
                let mut show_dock = false;
                let show_tray_notification_selection = {
                    let data = app.state::<Mutex<AppData>>();
                    let data = data.lock().unwrap();
                    data.app_state.tray_notification()
                };
                if show_tray_notification_selection {
                    let height = 250.;
                    let width = 450.;

                    let notification_window = tauri::WebviewWindowBuilder::new(
                        app,
                        "trayNotification",
                        tauri::WebviewUrl::App("trayNotification.html".into()),
                    )
                    .visible(true)
                    .focused(true)
                    .resizable(false)
                    .hidden_title(true)
                    .always_on_top(true)
                    .title_bar_style(tauri::TitleBarStyle::Overlay)
                    .title("Tray Notification")
                    .inner_size(width, height)
                    .build();
                    if let Err(e) = notification_window {
                        log::error!("Failed to create notification window: {e:?}");
                    } else {
                        let notification_window = notification_window.unwrap();
                        let _ = notification_window.show();
                        let _ = notification_window.set_focus();
                        show_dock = true;
                    }
                }

                if permissions::has_ungranted_permissions() && !cfg!(debug_assertions) {
                    log::info!("Opening permissions window");
                    let permissions_window = tauri::WebviewWindowBuilder::new(
                        app,
                        "permissions",
                        tauri::WebviewUrl::App("permissions.html".into()),
                    )
                    .visible(false)
                    .focused(true)
                    .resizable(false)
                    .hidden_title(true)
                    .always_on_top(false)
                    .title_bar_style(tauri::TitleBarStyle::Overlay)
                    .title("Permissions Configuration")
                    .inner_size(900., 620.)
                    .build();
                    if let Err(e) = permissions_window {
                        log::error!("Failed to create permissions window: {e:?}");
                    } else {
                        /*
                         * Focus the window only if the notification window is not shown.
                         * When the notification window is shown we open the permissions window
                         * when it's closed.
                         */
                        if !show_dock {
                            let permissions_window = permissions_window.unwrap();
                            let _ = permissions_window.show();
                            let _ = permissions_window.set_focus();
                        }
                        show_dock = true;
                    }
                }

                if show_dock {
                    app.set_activation_policy(tauri::ActivationPolicy::Regular);
                }
            }

            Ok(())
        })
        .on_window_event(move |window, event| {
            if let tauri::WindowEvent::Focused(is_focused) = event {
                #[cfg(any(target_os = "windows", target_os = "linux"))]
                if *is_focused && window.label() == "main" {
                    /* Place window on the bottom right corner of the active display. */
                    let current_monitor = window.current_monitor();
                    if let Ok(Some(monitor)) = current_monitor {
                        let monitor_size = monitor.size();
                        let monitor_pos = monitor.position();
                        let window_size = window.inner_size().unwrap();
                        let base_offset = 20 * monitor.scale_factor() as u32;
                        let offset_y = (25. * monitor.scale_factor()) as u32 + base_offset;
                        let x = monitor_pos.x
                            + (monitor_size.width - window_size.width - base_offset) as i32;
                        let y = monitor_pos.y
                            + (monitor_size.height - window_size.height - offset_y) as i32;
                        let new_position = PhysicalPosition::new(x as f64, y as f64);
                        let _ = window.set_position(new_position);
                    }
                }

                // detect click outside of the focused window and hide the app
                let deactivate_hiding = deactivate_hiding.lock().unwrap();
                let reopen_requested = reopen_requested.lock().unwrap();
                if !is_focused
                    && window.label() == "main"
                    && !cfg!(debug_assertions)
                    && !*deactivate_hiding
                    && !*reopen_requested
                {
                    log::info!("Hiding main window on focus lost: {}", *reopen_requested);

                    #[cfg(target_os = "macos")]
                    window.hide().unwrap();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            screenshare,
            stop_sharing,
            get_available_content,
            store_token_cmd,
            get_stored_token,
            delete_stored_token,
            play_sound,
            stop_sound,
            reset_core_process,
            get_logs,
            set_deactivate_hiding,
            set_controller_cursor,
            open_accessibility_settings,
            open_microphone_settings,
            open_screenshare_settings,
            trigger_screenshare_permission,
            get_control_permission,
            get_microphone_permission,
            get_screenshare_permission,
            skip_tray_notification_selection_window,
            set_dock_icon_visible,
            set_last_used_mic,
            get_last_used_mic,
            minimize_main_window,
            set_livekit_url,
            get_livekit_url,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(move |app_handle, event| match event {
        tauri::RunEvent::ExitRequested { .. } => {
            log::info!("Exit requested");
        }
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen { .. } => {
            log::info!("reopen requested");

            let location_set = location_set.lock().unwrap();
            if !*location_set {
                log::info!("Location not set, don't show the main window");
                return;
            }

            {
                let mut reopen_requested = reopen_requested_clone.lock().unwrap();
                *reopen_requested = true;
            }
            {
                let data = app_handle.state::<Mutex<AppData>>();
                let data = data.lock().unwrap();
                if !*data.dock_enabled.lock().unwrap() {
                    log::info!("Dock icon is not enabled, setting activation policy to accessory");
                    let _ = app_handle.set_activation_policy(tauri::ActivationPolicy::Accessory);
                }
            }
            let main_window = app_handle.get_webview_window("main");
            if let Some(window) = main_window {
                let _ = window.show();
                let _ = window.set_focus();
            } else {
                log::error!("Main window not found");
            }

            let reopen_requested_thread = reopen_requested_clone.clone();
            /*
             * When reopen is requested app is losing focus as soon as the window opens.
             * The reopen_requested flag is used to disable hiding the window on focus lost.
             * We wait 500ms before allowing again hiding on focus lost.
             */
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let mut reopen_requested = reopen_requested_thread.lock().unwrap();
                *reopen_requested = false;
            });
        }
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { .. },
            ..
        } => {
            log::info!("Close requested for window: {label}");
            if label == "trayNotification" {
                /* Make the permissions window visible in this case. */
                let permissions_window = app_handle.get_webview_window("permissions");
                if let Some(window) = permissions_window {
                    log::info!("Show permissions window");
                    let _ = window.show();
                    let _ = window.set_focus();
                } else {
                    #[cfg(target_os = "macos")]
                    let _ = app_handle.set_activation_policy(tauri::ActivationPolicy::Accessory);
                }
            } else if label == "permissions" {
                /*
                 * Permissions will always be the last window so hide the dock icon.
                 */
                #[cfg(target_os = "macos")]
                let _ = app_handle.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }
        }
        _ => {}
    });
}
