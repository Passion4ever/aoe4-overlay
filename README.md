# AoE4 Overlay

《帝国时代 4》对局信息悬浮横幅 —— 一个透明、置顶、鼠标穿透的桌面 overlay，基于 [aoe4world](https://aoe4world.com) 的实时数据，在游戏画面顶部显示双方玩家信息。

## 功能

- **对局信息横幅**：加载 aoe4world overlay，显示双方玩家、段位、地图等
- **地图名中文翻译**：自动把地图英文名补成「English / 中文」
- **对手常用文明**：拉取对手最近数据，在名字后追加其高频文明
- **自定义显示**：位置（左上 / 居中 / 右上）、透明度、缩放
- **全局快捷键**：一键显示 / 隐藏 overlay
- **托盘常驻**：右键托盘可显示/隐藏、打开设置、退出
- **更新提示**：启动检查 [Gitee Releases](https://gitee.com/Passion4ever/aoe4-overlay/releases)，有新版在设置界面顶部与托盘菜单提示（连不上则静默）

## 使用

1. 从 [Releases](https://gitee.com/Passion4ever/aoe4-overlay/releases) 下载最新的 `AoE4Overlay-x.x.x.exe`（便携版，免安装）
2. 双击运行，首次会弹出设置窗口
3. 填入你的 **aoe4world Profile ID**（即 `aoe4world.com/players/` 后面那串数字），可直接粘贴主页链接
4. 选好位置 / 透明度 / 缩放 / 快捷键，点「应用」
5. 进游戏即可看到顶部横幅

## 从源码构建

需要 [Node.js](https://nodejs.org)。

```bash
npm install        # 安装依赖
npm start          # 本地运行调试
npm run build      # 打包出 dist/AoE4Overlay-x.x.x.exe
```

## 目录结构

```
src/
  main.js        主进程（窗口/托盘/热键/更新检查）
  preload.js     渲染进程安全桥
  setup.html     设置界面
  maps.js        地图中文对照表
  civs.js        文明中文对照表
assets/
  icon.ico       图标（运行与打包共用）
  godeye-overlay.png  图标母图
```

## 技术栈

[Electron](https://www.electronjs.org) + 原生 HTML/CSS/JS，无前端框架。overlay 内容来自 aoe4world 远程页面，本地注入样式与脚本做中文翻译、文明显示等增强。

## 致谢

- 数据与 overlay 页面来自 [aoe4world](https://aoe4world.com)
- 本项目参考了开源项目 [aoe4world/overlay](https://github.com/aoe4world/overlay)

## 许可证

[MIT](./LICENSE) © 2026 Gold~ (Passion4ever)
