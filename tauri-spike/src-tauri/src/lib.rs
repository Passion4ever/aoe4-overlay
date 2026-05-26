// Tauri 可行性验证：透明 + 鼠标穿透 + 置顶 的 overlay 窗口，
// 加载 aoe4world 远程横幅页并注入透明 CSS（验证 WebView2 能否做透明 overlay）。

use tauri::{WebviewUrl, WebviewWindowBuilder};

const OVERLAY_URL: &str =
    "https://overlay.aoe4world.com/profile/16507159/bar?hideAfter=0&theme=top";

// 注入脚本：把页面背景设为透明 + 复用我们在 Electron 里的加宽/徽章修复。
const INIT_JS: &str = r#"
window.addEventListener('DOMContentLoaded', function () {
  var s = document.createElement('style');
  s.textContent =
    'html,body{background:transparent !important;overflow:hidden !important;}' +
    'body{zoom:0.75;}' +
    'p.text-sm.uppercase{margin-top:12px !important;}' +
    '[class*="w-[800px]"]{width:1000px !important;}';
  document.head.appendChild(s);
});
"#;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let win = WebviewWindowBuilder::new(
                app,
                "overlay",
                WebviewUrl::External(OVERLAY_URL.parse().unwrap()),
            )
            .title("AoE4 Overlay (Tauri spike)")
            .inner_size(1920.0, 150.0)
            .position(0.0, 0.0)
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .initialization_script(INIT_JS)
            .build()?;

            // 鼠标穿透：点击穿过 overlay 落到下面的游戏
            win.set_ignore_cursor_events(true)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
