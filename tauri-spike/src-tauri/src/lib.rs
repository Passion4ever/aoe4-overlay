// AoE4 Overlay —— Tauri 版主进程
// 对应 Electron 的 main.js：overlay 窗口 / 设置窗口 / 托盘 / 热键 / 单实例 / 更新检查。

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Wry, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_opener::OpenerExt;

const OVERLAY_URL_TMPL: &str =
    "https://overlay.aoe4world.com/profile/{id}/bar?hideAfter=0&theme=top";
const UPDATE_API: &str =
    "https://gitee.com/api/v5/repos/Passion4ever/aoe4-overlay/releases/latest";
const RELEASES_PAGE: &str = "https://gitee.com/Passion4ever/aoe4-overlay/releases";
const OVERLAY_H: f64 = 500.0;

const MAPS_JSON: &str = include_str!("../maps.json");
const CIVS_JSON: &str = include_str!("../civs.json");
const OVERLAY_INIT_TMPL: &str = include_str!("overlay_inject.js");

// ── 配置 ──
#[derive(Serialize, Deserialize, Clone)]
struct Config {
    #[serde(default)]
    profile_id: String,
    #[serde(default = "def_pos")]
    position: String,
    #[serde(default = "def_op")]
    opacity: u32,
    #[serde(default = "def_zoom")]
    zoom: u32,
    #[serde(default)]
    hotkey: String,
}
fn def_pos() -> String {
    "top-center".into()
}
fn def_op() -> u32 {
    75
}
fn def_zoom() -> u32 {
    75
}
impl Default for Config {
    fn default() -> Self {
        Config {
            profile_id: String::new(),
            position: def_pos(),
            opacity: def_op(),
            zoom: def_zoom(),
            hotkey: String::new(),
        }
    }
}

#[derive(Serialize, Clone, Default)]
struct UpdateInfo {
    version: String,
    url: String,
}

#[derive(Serialize, Clone)]
struct AppInfo {
    version: String,
    update: Option<UpdateInfo>,
}

#[derive(Default)]
struct AppState {
    update: Mutex<Option<UpdateInfo>>,
    update_item: Mutex<Option<MenuItem<Wry>>>,
}

fn config_path(app: &AppHandle) -> std::path::PathBuf {
    let dir = app.path().app_config_dir().unwrap();
    std::fs::create_dir_all(&dir).ok();
    dir.join("config.json")
}

fn load_config(app: &AppHandle) -> Config {
    std::fs::read_to_string(config_path(app))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(app: &AppHandle, cfg: &Config) {
    if let Ok(s) = serde_json::to_string_pretty(cfg) {
        std::fs::write(config_path(app), s).ok();
    }
}

// ── overlay 窗口 ──
fn create_overlay(app: &AppHandle, cfg: &Config) {
    if let Some(w) = app.get_webview_window("overlay") {
        w.close().ok();
    }
    if cfg.profile_id.is_empty() {
        return;
    }
    let url = OVERLAY_URL_TMPL.replace("{id}", &cfg.profile_id);
    let align = match cfg.position.as_str() {
        "top-left" => "flex-start",
        "top-right" => "flex-end",
        _ => "center",
    };
    let init = OVERLAY_INIT_TMPL
        .replace("__OPACITY__", &format!("{}", cfg.opacity as f64 / 100.0))
        .replace("__ZOOM__", &format!("{}", cfg.zoom as f64 / 100.0))
        .replace("__ALIGN__", align)
        .replace("__PROFILE__", &cfg.profile_id)
        .replace("__MAPCN__", MAPS_JSON.trim())
        .replace("__CIVCN__", CIVS_JSON.trim());

    // 屏幕逻辑宽度（满宽横幅）
    let width = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| m.size().width as f64 / m.scale_factor())
        .unwrap_or(1920.0);

    if let Ok(win) = WebviewWindowBuilder::new(
        app,
        "overlay",
        WebviewUrl::External(url.parse().unwrap()),
    )
    .title("AoE4 Overlay")
    .inner_size(width, OVERLAY_H)
    .position(0.0, 0.0)
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .initialization_script(&init)
    .build()
    {
        win.set_ignore_cursor_events(true).ok();
    }
}

// ── 设置窗口 ──
fn open_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        w.show().ok();
        w.set_focus().ok();
        return;
    }
    WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("setup.html".into()))
        .title("AoE4 Overlay 设置")
        .inner_size(400.0, 522.0)
        .resizable(false)
        .build()
        .ok();
}

// ── 托盘 ──
fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let update_item = MenuItem::with_id(app, "update", "（已是最新）", false, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "显示 / 隐藏", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = MenuBuilder::new(app)
        .item(&update_item)
        .item(&toggle)
        .item(&settings)
        .item(&quit)
        .build()?;

    app.state::<AppState>()
        .update_item
        .lock()
        .unwrap()
        .replace(update_item);

    TrayIconBuilder::with_id("tray")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("AoE4 Overlay by Gold~")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => {
                if let Some(w) = app.get_webview_window("overlay") {
                    if w.is_visible().unwrap_or(false) {
                        w.hide().ok();
                    } else {
                        w.show().ok();
                    }
                }
            }
            "settings" => open_settings(app),
            "quit" => app.exit(0),
            "update" => {
                if let Some(u) = app.state::<AppState>().update.lock().unwrap().clone() {
                    app.opener().open_url(u.url, None::<&str>).ok();
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // 单击托盘打开设置
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                open_settings(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

// ── 全局热键 ──
fn register_hotkey(app: &AppHandle, hotkey: &str) {
    let gs = app.global_shortcut();
    gs.unregister_all().ok();
    if hotkey.is_empty() {
        return;
    }
    // 反引号键转成 Tauri 能识别的 Backquote
    let mapped = hotkey.replace('`', "Backquote");
    gs.on_shortcut(mapped.as_str(), |app, _sc, event| {
        if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
            if let Some(w) = app.get_webview_window("overlay") {
                if w.is_visible().unwrap_or(false) {
                    w.hide().ok();
                } else {
                    w.show().ok();
                }
            }
        }
    })
    .ok();
}

// ── 更新检查（Gitee）──
fn ver_parts(s: &str) -> Vec<u32> {
    s.trim_start_matches(['v', 'V'])
        .split('.')
        .map(|x| x.parse().unwrap_or(0))
        .collect()
}
fn is_newer(remote: &str, local: &str) -> bool {
    let (a, b) = (ver_parts(remote), ver_parts(local));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (*a.get(i).unwrap_or(&0), *b.get(i).unwrap_or(&0));
        if x != y {
            return x > y;
        }
    }
    false
}

fn check_update(app: AppHandle) {
    std::thread::spawn(move || {
        let body = match ureq::get(UPDATE_API)
            .set("User-Agent", "AoE4Overlay")
            .call()
            .and_then(|r| r.into_string().map_err(Into::into))
        {
            Ok(b) => b,
            Err(_) => return, // 连不上：静默
        };
        let json: serde_json::Value = match serde_json::from_str(&body) {
            Ok(j) => j,
            Err(_) => return,
        };
        let tag = json["tag_name"].as_str().unwrap_or("");
        let local = app.package_info().version.to_string();
        if tag.is_empty() || !is_newer(tag, &local) {
            return;
        }
        // 下载链接：body 里的链接 → exe 附件直链 → release 页
        let url = json["body"]
            .as_str()
            .and_then(|b| {
                b.split_whitespace()
                    .find(|w| w.starts_with("http://") || w.starts_with("https://"))
                    .map(|s| s.trim_end_matches(')').to_string())
            })
            .or_else(|| {
                json["assets"].as_array().and_then(|arr| {
                    arr.iter()
                        .filter_map(|a| a["browser_download_url"].as_str())
                        .find(|u| u.to_lowercase().ends_with(".exe"))
                        .map(|s| s.to_string())
                })
            })
            .unwrap_or_else(|| RELEASES_PAGE.to_string());

        let info = UpdateInfo {
            version: tag.trim_start_matches(['v', 'V']).to_string(),
            url,
        };
        let state = app.state::<AppState>();
        *state.update.lock().unwrap() = Some(info.clone());
        // 托盘条目
        if let Some(item) = state.update_item.lock().unwrap().as_ref() {
            item.set_text(format!("↑ 新版 v{} — 点击下载", info.version)).ok();
            item.set_enabled(true).ok();
        }
        // 通知已打开的设置窗口
        app.emit("update-available", info).ok();
    });
}

// ── IPC 命令 ──
#[tauri::command]
fn get_config(app: AppHandle) -> Config {
    load_config(&app)
}

#[tauri::command]
fn get_app_info(app: AppHandle) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        update: app.state::<AppState>().update.lock().unwrap().clone(),
    }
}

#[tauri::command]
fn launch_overlay(app: AppHandle, config: Config) {
    save_config(&app, &config);
    create_overlay(&app, &config);
    register_hotkey(&app, &config.hotkey);
    if let Some(w) = app.get_webview_window("settings") {
        w.close().ok();
    }
}

#[tauri::command]
fn open_external(app: AppHandle, url: String) {
    app.opener().open_url(url, None::<&str>).ok();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            open_settings(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_app_info,
            launch_overlay,
            open_external
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            build_tray(&handle)?;
            let cfg = load_config(&handle);
            if cfg.profile_id.is_empty() {
                open_settings(&handle);
            } else {
                create_overlay(&handle, &cfg);
                register_hotkey(&handle, &cfg.hotkey);
            }
            check_update(handle);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
