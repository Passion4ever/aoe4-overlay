# AoE4 Overlay · 对局信息悬浮横幅

《帝国时代 4》桌面悬浮横幅：开局自动拉取 [aoe4world](https://aoe4world.com) 数据，在屏幕顶部显示**双方玩家**的国旗、文明、段位徽章、评分、战绩、常用文明，以及地图与模式。透明穿透、永远置顶，不挡操作。

> 基于 [Tauri](https://tauri.app) v2 + WebView2，安装包小（~10MB）。早期 Electron 版本保留在 [`legacy`](../../tree/legacy) 分支。

## 功能

- **对局横幅**：双方队伍并排，含 国旗 / 文明旗 / 段位徽章 / 评分 / 排名 / 战绩(W-L) / 本赛季常用文明；地图与模式名中文优先。1v1~4v4 自适应。
- **透明穿透置顶**：横幅不接收鼠标，点击穿透到游戏；窗口随内容自动贴合、可左/中/右对齐。
- **设置面板**：填 Profile ID 或 Steam ID（也可直接粘贴 aoe4world / Steam 主页链接）；可调**整体缩放**、**不透明度**、**对齐**、**显隐快捷键**。
- **托盘 / 全局热键 / 单实例 / 自动更新检查**（Gitee）。

## 使用

1. 从 [Releases](https://gitee.com/Passion4ever/aoe4-overlay/releases) 下载并运行。
2. 托盘图标 → 设置 → 填入你的 **Profile ID / Steam ID**（或主页链接）→ 应用。
3. 默认快捷键 **Ctrl+`** 切换显示/隐藏。

## 开发 / 构建

需要 [Rust](https://www.rust-lang.org/) 与 [Node.js](https://nodejs.org/)（含 WebView2，Win10 2004+/Win11 自带）。

```bash
npm install
npm run tauri dev      # 开发调试
npm run tauri build    # 打包（产物在 src-tauri/target/release/bundle/）
```

- 前端是纯静态页面（`src/`，无打包步骤），主进程在 `src-tauri/src/lib.rs`。
- 单点版本号：`src-tauri/tauri.conf.json` 的 `version`（与 `Cargo.toml` 保持一致）。

## 数据与致谢

- 对局数据来自 **[aoe4world.com](https://aoe4world.com)** 公开 API。
- 「显示玩家所属国家国旗」的想法参考了 **[gearlam/AoE4_Overlay_CS](https://github.com/gearlam/AoE4_Overlay_CS)**，感谢。
- 国家旗素材来自 [flag-icons](https://github.com/lipis/flag-icons)（MIT）。

## License

[MIT](LICENSE) · by Gold~
