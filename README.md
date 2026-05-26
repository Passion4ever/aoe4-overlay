# AoE4 Overlay · 对局信息悬浮横幅

《帝国时代 4》桌面悬浮横幅：开局自动在屏幕顶部显示**双方玩家**的国旗、文明、段位、评分、战绩、常用文明，以及地图与模式。透明、置顶、点击穿透，不挡游戏操作。

## 功能

- **对局横幅**：双方队伍并排显示 国旗 / 文明 / 段位徽章 / 评分 / 排名 / 战绩(W-L) / 本赛季常用文明；地图与模式名中文优先，1v1~4v4 自适应。
- **不挡操作**：横幅鼠标穿透到游戏，永远置顶；可左 / 中 / 右对齐。
- **可调**：整体缩放、不透明度、对齐位置、显隐快捷键。
- **省心**：托盘常驻、全局热键、单实例、自动检查更新。

## 使用

1. 从 [Releases](https://gitee.com/Passion4ever/aoe4-overlay/releases) 下载 `AoE4Overlay-x.x.x.exe`，**免安装，双击即用**。
2. 托盘图标 → 设置 → 填入你的 **Profile ID / Steam ID**（也可直接粘贴 aoe4world 或 Steam 主页链接）→ 应用。
3. 默认快捷键 **Ctrl+`** 切换显示 / 隐藏。

> 绝大多数 Win10 / Win11 都能直接打开。万一双击没反应，装一下微软官方的 [WebView2 运行时](https://developer.microsoft.com/microsoft-edge/webview2/)（很小，选「Evergreen 引导程序」一键装完即可，之后无需再装）。

## 开发

```bash
npm install
npm run tauri dev      # 调试
npm run tauri build    # 打包
```

旧版本保留在 [`legacy`](../../tree/legacy) 分支。

## 致谢

- 对局数据来自 **[aoe4world.com](https://aoe4world.com)**。
- 「显示玩家所属国家国旗」的想法参考了 **[gearlam/AoE4_Overlay_CS](https://github.com/gearlam/AoE4_Overlay_CS)**，感谢。
- 国家旗素材来自 [flag-icons](https://github.com/lipis/flag-icons)（MIT）。

## License

[MIT](LICENSE) · by Gold~
