mod features;
#[cfg(target_os = "windows")]
mod flydigi_battery;
#[cfg(target_os = "windows")]
mod logitech_hidpp;
pub mod network;
mod peripheral_battery;
mod peripherals;
#[cfg(target_os = "windows")]
mod royal_kludge_battery;
mod scheduler;
pub mod store;
mod web_console;

use crate::network::server::{
    cleanup_stale_presence, disconnect_client_session, get_clients_with_status,
    notify_mobile_disconnect, ping_mobile_device, SessionEvent,
};
use mdns_sd::ServiceDaemon;
use serde::Serialize;
use std::env;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, LogicalSize, Manager, RunEvent, Size, State, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_updater::UpdaterExt;
use tokio::time::MissedTickBehavior;

const TRAY_MENU_SHOW: &str = "show";
const TRAY_MENU_EXIT: &str = "exit";
const STARTUP_ARG_HIDDEN: &str = "--cobweb-startup-hidden";
const STARTUP_REGISTRY_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const STARTUP_REGISTRY_VALUE: &str = "Cobweb";

#[derive(Default)]
struct MdnsRuntime {
    daemon: Mutex<Option<ServiceDaemon>>,
}

#[derive(Default)]
struct ExitState {
    allowed: AtomicBool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MdnsStatus {
    enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CloseBehavior {
    close_to_tray_on_close: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StartupBehavior {
    launch_on_startup: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateBehavior {
    auto_update_enabled: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AppNotice {
    title: String,
    message: String,
    tone: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WebConsoleStatus {
    running: bool,
    port: Option<u16>,
    urls: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StartupAutoUpdateResult {
    Finished,
    RetryableFailure,
}

fn current_mdns_enabled() -> bool {
    let store = store::GLOBAL_STORE.lock().unwrap();
    store.data.mdns_enabled
}

fn apply_mdns_enabled(
    runtime: &MdnsRuntime,
    device_id: &str,
    device_name: &str,
    port: u16,
    enabled: bool,
) -> Result<(), String> {
    if enabled {
        let mut daemon = runtime.daemon.lock().unwrap();
        if daemon.is_some() {
            return Ok(());
        }

        let next = network::mdns::start_mdns(device_id, device_name, port)
            .map_err(|error| error.to_string())?;
        *daemon = Some(next);
        return Ok(());
    }

    let existing = {
        let mut daemon = runtime.daemon.lock().unwrap();
        daemon.take()
    };

    if let Some(existing) = existing {
        network::mdns::stop_mdns(existing).map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn web_console_status_value() -> WebConsoleStatus {
    let port = network::server::current_web_port();
    let urls = port
        .map(|port| {
            let mut addresses = network::mdns::local_lan_ip_addresses()
                .into_iter()
                .map(|ip| format!("{}:{}/web", ip, port))
                .collect::<Vec<_>>();

            if addresses.is_empty() {
                addresses.push(format!("127.0.0.1:{}/web", port));
            }

            addresses
        })
        .unwrap_or_default();

    WebConsoleStatus {
        running: port.is_some(),
        port,
        urls,
    }
}

#[tauri::command]
async fn resolve_pair_request(client_id: String, allowed: bool) -> Result<(), String> {
    let mut map = network::server::PENDING_REQUESTS.lock().unwrap();
    if let Some(tx) = map.remove(&client_id) {
        let _ = tx.send(allowed);
        Ok(())
    } else {
        Err("No pending request found".into())
    }
}

#[tauri::command]
fn get_pending_tasks() -> Vec<cobweb_scheduler::ScheduledTask> {
    scheduler::list_tasks()
}

#[tauri::command]
fn get_task_history_entries() -> Vec<store::TaskHistoryEntry> {
    scheduler::list_task_history()
}

#[tauri::command]
fn get_mdns_status() -> MdnsStatus {
    MdnsStatus {
        enabled: current_mdns_enabled(),
    }
}

#[tauri::command]
fn get_web_console_status() -> WebConsoleStatus {
    web_console_status_value()
}

#[tauri::command]
fn get_close_behavior() -> CloseBehavior {
    let store = store::GLOBAL_STORE.lock().unwrap();
    CloseBehavior {
        close_to_tray_on_close: store.data.close_to_tray_on_close,
    }
}

#[tauri::command]
fn set_close_to_tray_on_close(enabled: bool) -> CloseBehavior {
    let mut store = store::GLOBAL_STORE.lock().unwrap();
    store.set_close_to_tray_on_close(enabled);
    CloseBehavior {
        close_to_tray_on_close: enabled,
    }
}

fn is_startup_hidden_launch() -> bool {
    env::args_os().any(|arg| arg == STARTUP_ARG_HIDDEN)
}

fn apply_configured_main_window_layout(app: &AppHandle, window: &WebviewWindow) {
    if let Some(config) = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
    {
        let _ = window.set_size(Size::Logical(LogicalSize {
            width: config.width,
            height: config.height,
        }));

        if config.center {
            let _ = window.center();
        }
    }
}

#[cfg(target_os = "windows")]
fn startup_command() -> Result<String, String> {
    let exe_path = env::current_exe().map_err(|error| format!("读取程序路径失败: {error}"))?;
    Ok(format!("\"{}\" {}", exe_path.display(), STARTUP_ARG_HIDDEN))
}

#[cfg(target_os = "windows")]
fn is_launch_on_startup_enabled() -> Result<bool, String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey(STARTUP_REGISTRY_KEY) {
        Ok(key) => key,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("读取自启设置失败: {error}")),
    };
    Ok(key.get_value::<String, _>(STARTUP_REGISTRY_VALUE).is_ok())
}

#[cfg(not(target_os = "windows"))]
fn is_launch_on_startup_enabled() -> Result<bool, String> {
    Ok(false)
}

#[cfg(target_os = "windows")]
fn set_launch_on_startup_enabled(enabled: bool) -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(STARTUP_REGISTRY_KEY)
        .map_err(|error| format!("打开自启设置失败: {error}"))?;

    if enabled {
        key.set_value(STARTUP_REGISTRY_VALUE, &startup_command()?)
            .map_err(|error| format!("写入自启设置失败: {error}"))?;
    } else {
        match key.delete_value(STARTUP_REGISTRY_VALUE) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("关闭自启失败: {error}")),
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn set_launch_on_startup_enabled(_enabled: bool) -> Result<(), String> {
    Err("开机自启当前仅支持 Windows 桌面端".into())
}

#[tauri::command]
fn get_startup_behavior() -> Result<StartupBehavior, String> {
    Ok(StartupBehavior {
        launch_on_startup: is_launch_on_startup_enabled()?,
    })
}

#[tauri::command]
fn set_launch_on_startup(enabled: bool) -> Result<StartupBehavior, String> {
    set_launch_on_startup_enabled(enabled)?;
    Ok(StartupBehavior {
        launch_on_startup: is_launch_on_startup_enabled()?,
    })
}

#[tauri::command]
fn get_update_behavior() -> UpdateBehavior {
    let store = store::GLOBAL_STORE.lock().unwrap();
    UpdateBehavior {
        auto_update_enabled: store.data.auto_update_enabled,
    }
}

#[tauri::command]
fn set_auto_update_enabled(enabled: bool) -> UpdateBehavior {
    let mut store = store::GLOBAL_STORE.lock().unwrap();
    store.set_auto_update_enabled(enabled);
    UpdateBehavior {
        auto_update_enabled: enabled,
    }
}

fn emit_app_notice(
    app: &AppHandle,
    title: impl Into<String>,
    message: impl Into<String>,
    tone: impl Into<String>,
) {
    let _ = app.emit(
        "app_notice",
        AppNotice {
            title: title.into(),
            message: message.into(),
            tone: tone.into(),
        },
    );
}

async fn run_startup_auto_update(app: AppHandle, silent: bool) {
    let max_attempts = if silent { 3 } else { 1 };

    for attempt in 1..=max_attempts {
        let result = run_startup_auto_update_once(app.clone(), silent).await;
        if result == StartupAutoUpdateResult::Finished || attempt == max_attempts {
            return;
        }

        // 开机后台启动时网络服务可能还没准备好，静默重试可以提高首轮自动更新成功率。
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

async fn run_startup_auto_update_once(app: AppHandle, silent: bool) -> StartupAutoUpdateResult {
    if cfg!(debug_assertions) {
        return StartupAutoUpdateResult::Finished;
    }

    let auto_update_enabled = {
        let store = store::GLOBAL_STORE.lock().unwrap();
        store.data.auto_update_enabled
    };

    if !auto_update_enabled {
        return StartupAutoUpdateResult::Finished;
    }

    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(error) => {
            log::error!("初始化自动更新失败: {}", error);
            if !silent {
                emit_app_notice(&app, "自动更新失败", error.to_string(), "warning");
            }
            return StartupAutoUpdateResult::RetryableFailure;
        }
    };

    let update = match updater.check().await {
        Ok(update) => update,
        Err(error) => {
            log::error!("检查自动更新失败: {}", error);
            if !silent {
                emit_app_notice(&app, "自动更新失败", error.to_string(), "warning");
            }
            return StartupAutoUpdateResult::RetryableFailure;
        }
    };

    let Some(update) = update else {
        return StartupAutoUpdateResult::Finished;
    };

    let version = update.version.clone();
    if !silent {
        emit_app_notice(
            &app,
            "发现新版本",
            format!("正在后台更新到 {version}"),
            "success",
        );
    }

    let install_notice_app = app.clone();
    let result = update
        .download_and_install(
            |_chunk_length, _content_length| {},
            move || {
                if !silent {
                    emit_app_notice(
                        &install_notice_app,
                        "正在安装更新",
                        "下载完成，应用即将重启",
                        "success",
                    );
                }
            },
        )
        .await;

    match result {
        Ok(()) => {
            log::info!("自动更新已安装，准备重启应用");
            app.state::<ExitState>()
                .allowed
                .store(true, Ordering::SeqCst);
            app.request_restart();
            StartupAutoUpdateResult::Finished
        }
        Err(error) => {
            log::error!("自动安装更新失败: {}", error);
            if !silent {
                emit_app_notice(
                    &app,
                    "自动更新失败",
                    format!("{error}，可在设置中手动重试"),
                    "warning",
                );
            }
            StartupAutoUpdateResult::RetryableFailure
        }
    }
}

fn show_existing_main_window(app: &AppHandle) -> bool {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        apply_configured_main_window_layout(app, &window);
        let _ = window.set_focus();
        return true;
    }

    false
}

fn create_main_window(app: &AppHandle) -> tauri::Result<()> {
    if show_existing_main_window(app) {
        return Ok(());
    }

    // 托盘状态下销毁 WebView，重新打开时按需创建，避免隐藏页面继续运行前端脚本。
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .expect("missing main window config");
    let window = WebviewWindowBuilder::from_config(app, config)?.build()?;
    apply_configured_main_window_layout(app, &window);
    let _ = window.set_focus();
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if show_existing_main_window(app) {
        return;
    }

    let app = app.clone();
    std::thread::spawn(move || {
        let _ = create_main_window(&app);
    });
}

fn setup_system_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, TRAY_MENU_SHOW, "打开 Cobweb", true, None::<&str>)?;
    let exit = MenuItem::with_id(app, TRAY_MENU_EXIT, "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &exit])?;

    let mut tray = TrayIconBuilder::new()
        .tooltip("Cobweb")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_MENU_SHOW => show_main_window(app),
            TRAY_MENU_EXIT => {
                app.state::<ExitState>()
                    .allowed
                    .store(true, Ordering::SeqCst);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
            | TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => show_main_window(tray.app_handle()),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}

#[tauri::command]
fn set_mdns_enabled(
    app: tauri::AppHandle,
    runtime: State<MdnsRuntime>,
    enabled: bool,
) -> Result<MdnsStatus, String> {
    let (device_id, device_name) = {
        let mut store = store::GLOBAL_STORE.lock().unwrap();
        store.set_mdns_enabled(enabled);
        (store.data.device_id.clone(), store.data.device_name.clone())
    };

    let port = network::server::current_web_port().unwrap_or(3000);
    apply_mdns_enabled(runtime.inner(), &device_id, &device_name, port, enabled)?;
    let status = MdnsStatus { enabled };
    let _ = app.emit("mdns_status_changed", &status);
    Ok(status)
}

#[tauri::command]
fn remove_paired_client(app: tauri::AppHandle, client_id: String) -> Result<(), String> {
    let disconnected_event = {
        let store = store::GLOBAL_STORE.lock().unwrap();
        store
            .data
            .paired_clients
            .get(&client_id)
            .map(|client| SessionEvent {
                client_id: client.client_id.clone(),
                client_name: client.client_name.clone(),
            })
    };

    disconnect_client_session(&client_id, "PC 已移除设备授权");

    let mut store = store::GLOBAL_STORE.lock().unwrap();
    store.remove_paired_client(&client_id);
    drop(store);

    network::server::ACTIVE_CONNECTIONS
        .lock()
        .unwrap()
        .remove(&client_id);
    network::server::ONLINE_CLIENTS
        .lock()
        .unwrap()
        .remove(&client_id);
    network::server::LAST_HEARTBEATS
        .lock()
        .unwrap()
        .remove(&client_id);

    if let Some(event) = disconnected_event {
        let _ = app.emit("device_disconnected", event);
    }
    let _ = app.emit("paired_clients_changed", serde_json::json!({}));
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(peripherals::init_state())
        .manage(MdnsRuntime::default())
        .manage(ExitState::default())
        .invoke_handler(tauri::generate_handler![
            peripherals::get_peripheral_devices,
            peripherals::start_device_watch,
            peripherals::stop_device_watch,
            features::get_feature_groups,
            features::get_feature_snapshot,
            features::execute_feature_command,
            resolve_pair_request,
            get_clients_with_status,
            get_pending_tasks,
            get_task_history_entries,
            get_web_console_status,
            get_mdns_status,
            set_mdns_enabled,
            get_close_behavior,
            set_close_to_tray_on_close,
            get_startup_behavior,
            set_launch_on_startup,
            get_update_behavior,
            set_auto_update_enabled,
            remove_paired_client,
            ping_mobile_device,
            notify_mobile_disconnect
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            if let Ok(app_dir) = app.path().app_data_dir() {
                store::init_store(app_dir);
            }

            setup_system_tray(app)?;

            scheduler::init(&app.handle().clone());

            let startup_hidden_launch = is_startup_hidden_launch();

            if !startup_hidden_launch {
                create_main_window(app.handle())?;
            }

            let updater_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // 普通启动时等前端通知组件挂载，避免自动更新提示被过早丢弃。
                if !startup_hidden_launch {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                run_startup_auto_update(updater_app, startup_hidden_launch).await;
            });

            let tauri_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match network::server::start_server(3000, tauri_app.clone()).await {
                    Ok(port) => {
                        let (device_id, device_name, mdns_enabled) = {
                            let store = store::GLOBAL_STORE.lock().unwrap();
                            (
                                store.data.device_id.clone(),
                                store.data.device_name.clone(),
                                store.data.mdns_enabled,
                            )
                        };

                        if mdns_enabled {
                            let runtime = tauri_app.state::<MdnsRuntime>();
                            if let Err(error) = apply_mdns_enabled(
                                runtime.inner(),
                                &device_id,
                                &device_name,
                                port,
                                true,
                            ) {
                                log::error!("Failed to start mDNS: {}", error);
                            }
                        }

                        let _ = tauri_app
                            .emit("web_console_status_changed", web_console_status_value());
                    }
                    Err(error) => {
                        log::error!("Failed to start Axum server: {}", error);
                    }
                }
            });

            let cleanup_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(5));
                interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

                loop {
                    interval.tick().await;
                    cleanup_stale_presence(&cleanup_app, 20_000);
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let close_to_tray_on_close = {
                    let store = store::GLOBAL_STORE.lock().unwrap();
                    store.data.close_to_tray_on_close
                };

                if close_to_tray_on_close {
                    api.prevent_close();
                    peripherals::stop_watcher(
                        window
                            .app_handle()
                            .state::<peripherals::WatcherState>()
                            .inner(),
                    );
                    let _ = window.destroy();
                } else {
                    window
                        .app_handle()
                        .state::<ExitState>()
                        .allowed
                        .store(true, Ordering::SeqCst);
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::ExitRequested { api, code, .. } = event {
                if code == Some(tauri::RESTART_EXIT_CODE) {
                    app.state::<ExitState>()
                        .allowed
                        .store(true, Ordering::SeqCst);
                    return;
                }

                let allowed = app.state::<ExitState>().allowed.load(Ordering::SeqCst);
                if !allowed {
                    api.prevent_exit();
                }
            }
        });
}
