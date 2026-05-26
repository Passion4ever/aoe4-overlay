// AoE4 Overlay —— Tauri 版主进程
// 对应 Electron 的 main.js：overlay 窗口 / 设置窗口 / 托盘 / 热键 / 单实例 / 更新检查。

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, Wry, WebviewUrl,
    WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_opener::OpenerExt;

const UPDATE_API: &str =
    "https://gitee.com/api/v5/repos/Passion4ever/aoe4-overlay/releases/latest";
const RELEASES_PAGE: &str = "https://gitee.com/Passion4ever/aoe4-overlay/releases";

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
    #[serde(default = "def_hotkey")]
    hotkey: String,
}
fn def_pos() -> String {
    "top-center".into()
}
fn def_hotkey() -> String {
    "Ctrl+`".into()
}
fn def_op() -> u32 {
    60
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
            hotkey: def_hotkey(),
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

// ── overlay 窗口（加载本地复刻横幅）──
fn create_overlay(app: &AppHandle, cfg: &Config) {
    if let Some(w) = app.get_webview_window("overlay") {
        w.close().ok();
    }
    if cfg.profile_id.is_empty() {
        return;
    }
    let align = match cfg.position.as_str() {
        "top-left" => "left",
        "top-right" => "right",
        _ => "center",
    };
    // 配置注入给前端（窗口创建即生效；改设置时窗口会重建）
    let init = format!(
        "window.__PROFILE__={:?};window.__OPACITY__={};window.__ZOOM__={};window.__ALIGN__={:?};",
        cfg.profile_id,
        cfg.opacity as f64 / 100.0,
        cfg.zoom as f64 / 100.0,
        align
    );

    if let Ok(win) = WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("index.html".into()))
        .title("AoE4 Overlay")
        .inner_size(600.0, 140.0) // 初始占位；前端量好内容后 fit_overlay 贴合
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
        w.unminimize().ok(); // 最小化的窗口要先恢复，否则 show/focus 唤不起（托盘点击 bug）
        w.show().ok();
        w.set_focus().ok();
        return;
    }
    WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("setup.html".into()))
        .title("AoE4 Overlay 设置")
        .inner_size(500.0, 680.0) // 初始接近最终；前端 fit_settings 再精确贴合
        .resizable(false)
        .maximizable(false) // 固定表单，不需要最大化
        .center()
        .visible(false) // 先隐藏，内容就绪后由 fit_settings 显示，避免白屏闪烁
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
    register_hotkey(&app, &config.hotkey);
    let align = match config.position.as_str() {
        "top-left" => "left",
        "top-right" => "right",
        _ => "center",
    };
    if let Some(w) = app.get_webview_window("overlay") {
        // 横幅已存在：推送新配置让前端实时更新，窗口不销毁、横幅不消失
        w.show().ok();
        app.emit(
            "config-changed",
            serde_json::json!({
                "profileId": config.profile_id,
                "opacity": config.opacity as f64 / 100.0,
                "zoom": config.zoom as f64 / 100.0,
                "align": align,
            }),
        )
        .ok();
    } else {
        // 首次（之前没 profile）：创建横幅窗口
        create_overlay(&app, &config);
    }
    if let Some(w) = app.get_webview_window("settings") {
        w.close().ok();
    }
}

#[tauri::command]
fn open_external(app: AppHandle, url: String) {
    app.opener().open_url(url, None::<&str>).ok();
}

// 设置窗口自适应到内容真实尺寸（同 fit_overlay 的 DPI 自校正），保证任意缩放比下
// CSS 视口正好 = Electron 原版设计尺寸(宽 400)，底部按钮/页脚不被裁。居中显示。
#[tauri::command]
fn fit_settings(app: AppHandle, width: f64, height: f64, vw: f64, vh: f64) {
    if vw < 1.0 || vh < 1.0 {
        return;
    }
    if let Some(w) = app.get_webview_window("settings") {
        let sf = w.scale_factor().unwrap_or(1.0);
        let (cur_w, cur_h) = w
            .inner_size()
            .map(|s| (s.width as f64 / sf, s.height as f64 / sf))
            .unwrap_or((400.0, 560.0));
        let new_w = (cur_w * width / vw).clamp(200.0, 1400.0);
        let new_h = (cur_h * height / vh).clamp(200.0, 1800.0);
        w.set_size(LogicalSize::new(new_w, new_h)).ok();
        w.center().ok();
        // 内容已就绪+贴合，此刻才显示（窗口创建时是隐藏的）→ 不白屏闪烁
        if !w.is_visible().unwrap_or(true) {
            w.show().ok();
            w.set_focus().ok();
        }
    }
}

// 前端量好横幅 CSS 尺寸(width/height) + 当前视口 CSS 尺寸(vw/vh)后调用。
// 本机 WebView2 的 devicePixelRatio ≠ Tauri 窗口缩放因子，所以不能直接把 CSS 尺寸当窗口逻辑尺寸；
// 用「当前窗口逻辑尺寸 × 横幅CSS ÷ 当前视口CSS」自校正，一步把视口逼到正好等于横幅。
// 定位用 Tauri 逻辑像素(屏宽与窗宽同一坐标系)，左/中/右贴顶。
#[tauri::command]
fn fit_overlay(app: AppHandle, width: f64, height: f64, align: String, vw: f64, vh: f64, dpr: f64) {
    if vw < 1.0 || vh < 1.0 || dpr < 0.1 {
        return;
    }
    let _ = dpr;
    if let Some(w) = app.get_webview_window("overlay") {
        let sf = w.scale_factor().unwrap_or(1.0);
        let isz = w.inner_size().ok();
        let cur_phys_w = isz.map(|s| s.width as f64).unwrap_or(900.0); // 当前实际物理宽
        let cur_phys_h = isz.map(|s| s.height as f64).unwrap_or(210.0);
        let new_w = (cur_phys_w / sf * width / vw).clamp(150.0, 4000.0);
        let new_h = (cur_phys_h / sf * height / vh).clamp(40.0, 1200.0);
        w.set_size(LogicalSize::new(new_w, new_h)).ok();
        // outer_size 含 Windows 不可见边框；可见横幅(客户区)比外框窄 border。
        // 反向补偿 border，让"可见横幅"真正贴边（左右对称）。居中不受影响(边框对称)。
        let outer_w = w
            .outer_size()
            .map(|s| s.width as f64)
            .unwrap_or(cur_phys_w);
        let border = ((outer_w - cur_phys_w) / 2.0).max(0.0); // 单侧不可见边框
        let screen_phys = app
            .primary_monitor()
            .ok()
            .flatten()
            .map(|m| m.size().width as f64)
            .unwrap_or(1920.0);
        let m = 2.0; // 期望可见边距(物理px)，贴边
        let x = match align.as_str() {
            "left" => m - border,
            "right" => screen_phys - outer_w + border - m,
            _ => (screen_phys - outer_w) / 2.0,
        };
        w.set_position(PhysicalPosition::new(x, 0.0)).ok();
    }
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
            open_external,
            fit_overlay,
            fit_settings
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
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            // 关闭所有窗口不退出（常驻托盘）；只有托盘“退出”(app.exit) 才真退。
            if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
