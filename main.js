const {
  app,
  BrowserWindow,
  ipcMain,
  Tray,
  Menu,
  nativeImage,
  globalShortcut,
  screen,
} = require("electron");
const path = require("path");
const fs = require("fs");
const MAP_NAMES = require("./maps");
const CIV_NAMES = require("./civs");

// Avoid GPU cache errors when previous instance left locks
app.commandLine.appendSwitch("disable-gpu-shader-disk-cache");

// ── Single instance lock ──
const gotLock = app.requestSingleInstanceLock();
if (!gotLock) {
  app.quit();
}

const CONFIG_FILE = path.join(app.getPath("userData"), "config.json");
const OVERLAY_URL =
  "https://overlay.aoe4world.com/profile/{id}/bar?hideAfter=0&theme=top";
const OVERLAY_H = 500;
const APP_NAME = "AoE4-Overlay";
const DEFAULTS = { opacity: 75, zoom: 75, position: "top-center", hotkey: "" };

let tray = null;
let overlayWin = null;
let overlayVisible = true;

function loadConfig() {
  try {
    return { ...DEFAULTS, ...JSON.parse(fs.readFileSync(CONFIG_FILE, "utf-8")) };
  } catch {
    return { ...DEFAULTS };
  }
}

function saveConfig(cfg) {
  fs.writeFileSync(CONFIG_FILE, JSON.stringify(cfg, null, 2));
}

function getContentAlign(pos) {
  if (pos === "top-left") return "flex-start";
  if (pos === "top-right") return "flex-end";
  return "center";
}

const ICON_PATH = path.join(__dirname, "icon.ico");

function createTrayIcon() {
  return nativeImage.createFromPath(ICON_PATH);
}

function registerHotkey(cfg) {
  globalShortcut.unregisterAll();
  if (!cfg.hotkey) return;
  try {
    globalShortcut.register(cfg.hotkey, () => {
      if (!overlayWin) return;
      overlayVisible = !overlayVisible;
      overlayWin.setOpacity(overlayVisible ? 1 : 0);
      updateTrayMenu();
    });
  } catch (e) {
    console.error("Hotkey failed:", e);
  }
}

function createTray() {
  tray = new Tray(createTrayIcon());
  tray.setToolTip(`${APP_NAME} v${app.getVersion()} · by Gold~`);
  updateTrayMenu();
}

function updateTrayMenu() {
  const cfg = loadConfig();
  const visible = overlayWin && overlayVisible;
  const hk = cfg.hotkey ? `  [${cfg.hotkey}]` : "";
  tray.setContextMenu(
    Menu.buildFromTemplate([
      {
        label: (visible ? "隐藏" : "显示") + hk,
        click: () => {
          if (!overlayWin) return;
          overlayVisible = !overlayVisible;
          overlayWin.setOpacity(overlayVisible ? 1 : 0);
          updateTrayMenu();
        },
        enabled: !!overlayWin,
      },
      { label: "设置", click: () => createSetupWindow() },
      { label: "退出", click: () => app.quit() },
    ])
  );
}

function createSetupWindow() {
  BrowserWindow.getAllWindows().forEach((w) => {
    if (w !== overlayWin) w.close();
  });
  const win = new BrowserWindow({
    width: 400,
    height: 530,
    useContentSize: true,
    resizable: false,
    autoHideMenuBar: true,
    backgroundColor: "#0d0d10",
    icon: ICON_PATH,
    webPreferences: { preload: path.join(__dirname, "preload.js") },
  });
  win.loadFile("setup.html");
  win.webContents.on("did-finish-load", () => {
    win.webContents.executeJavaScript(
      `window._loadConfig(${JSON.stringify(loadConfig())});` +
        `window._setVersion(${JSON.stringify(app.getVersion())})`
    );
  });
  return win;
}

function createOverlayWindow(cfg) {
  const url = OVERLAY_URL.replace("{id}", cfg.profile_id);
  const align = getContentAlign(cfg.position);
  const opacity = (cfg.opacity || DEFAULTS.opacity) / 100;
  const zoom = (cfg.zoom || DEFAULTS.zoom) / 100;
  const { width: screenW } = screen.getPrimaryDisplay().workAreaSize;

  if (overlayWin) { overlayWin.destroy(); overlayWin = null; }

  overlayWin = new BrowserWindow({
    title: APP_NAME,
    width: screenW, height: OVERLAY_H, x: 0, y: 0,
    transparent: true, frame: false, hasShadow: false,
    resizable: false, skipTaskbar: true, focusable: false,
    icon: ICON_PATH,
    webPreferences: { preload: path.join(__dirname, "preload.js") },
  });

  overlayWin.setAlwaysOnTop(true, "screen-saver");
  overlayWin.setIgnoreMouseEvents(true);
  overlayWin.loadURL(url);

  overlayWin.webContents.on("did-finish-load", () => {
    overlayWin.webContents.executeJavaScript(`
      document.body.style.background = 'transparent';
      document.documentElement.style.background = 'transparent';
      document.body.style.opacity = '${opacity}';
      document.body.style.zoom = '${zoom}';
      document.body.style.display = 'flex';
      document.body.style.justifyContent = '${align}';
      document.body.style.overflow = 'hidden';
      document.documentElement.style.overflow = 'hidden';

      // Layout tweaks injected as a <style> so they survive React re-renders:
      //  1. Mode badge ("RM 2V2") has a built-in mt-[27px] that pushes it below
      //     the dark center panel (~51px), clipping its lower half — pull it up.
      //  2. Widen the fixed-width banner card (default w-[800px]) so the two
      //     teams spread out and player names get more room (height unchanged).
      const fixStyle = document.createElement('style');
      fixStyle.textContent =
        'p.text-sm.uppercase{margin-top:12px !important;}' +
        '[class*="w-[800px]"]{width:1000px !important;}';
      document.head.appendChild(fixStyle);

      // Map name CN translation
      const MAP_CN = ${JSON.stringify(MAP_NAMES)};
      const observer = new MutationObserver(() => {
        // Target: only the map name element (font-bold, not uppercase)
        document.querySelectorAll('p.text-sm.font-bold:not(.uppercase)').forEach(el => {
          const text = el.textContent.trim();
          const en = text.includes(' / ') ? text.split(' / ')[0] : text;
          const cn = MAP_CN[en];
          if (cn) {
            const expected = en + ' / ' + cn;
            if (text !== expected) {
              el.textContent = expected;
            }
          }
        });
      });
      observer.observe(document.body, { childList: true, subtree: true, characterData: true });

      // Fetch opponent's most played civs and append to their name
      const CIV_CN = ${JSON.stringify(CIV_NAMES)};
      const profileId = '${cfg.profile_id}';
      let lastGameId = null;

      async function fetchAndShowCivs() {
        try {
          const resp = await fetch('https://aoe4world.com/api/v0/players/' + profileId + '/games/last');
          if (!resp.ok) return;
          const game = await resp.json();
          if (!game || !game.teams || game.game_id === lastGameId) return;
          lastGameId = game.game_id;

          // Find opponent profile IDs
          const myTeam = game.teams.find(t => t.some(p => game.filters.profile_ids.includes(p.profile_id))) || [];
          const myIds = myTeam.map(p => p.profile_id);
          const opponents = game.teams.flat().filter(p => !myIds.includes(p.profile_id));

          // Fetch each opponent's profile for civ stats
          for (const opp of opponents) {
            try {
              const pResp = await fetch('https://aoe4world.com/api/v0/players/' + opp.profile_id);
              if (!pResp.ok) continue;
              const pData = await pResp.json();

              // Find civ stats for the current leaderboard mode
              const mode = game.leaderboard || 'rm_solo';
              const civs = pData.modes?.[mode]?.civilizations;
              if (!civs || civs.length === 0) continue;

              // Sort by pick rate, take top 2
              const top = civs
                .sort((a, b) => (b.pick_rate || 0) - (a.pick_rate || 0))
                .slice(0, 2);

              const civText = top
                .map(c => {
                  const cn = CIV_CN[c.civilization] || c.civilization;
                  return cn;
                })
                .join('·');

              // Find the player's name element in DOM and append
              const nameEls = document.querySelectorAll('h1.font-bold.text-md.truncate');
              for (const el of nameEls) {
                const name = el.textContent.trim();
                if (name === opp.name || name.startsWith(opp.name + ' (')) {
                  if (!name.includes('(')) {
                    el.textContent = opp.name + ' (' + civText + ')';
                  }
                }
              }
            } catch(e) {}
          }
        } catch(e) {}
      }

      // Run on load and periodically
      setTimeout(fetchAndShowCivs, 3000);
      setInterval(fetchAndShowCivs, 20000);
    `);
  });

  overlayWin.on("closed", () => { overlayWin = null; updateTrayMenu(); });

  // Auto-adapt when display changes (plugging in a monitor)
  screen.on("display-metrics-changed", () => {
    if (!overlayWin) return;
    const { width: sw } = screen.getPrimaryDisplay().workAreaSize;
    overlayWin.setBounds({ x: 0, y: 0, width: sw, height: OVERLAY_H });
  });

  registerHotkey(cfg);
  updateTrayMenu();
  return overlayWin;
}

// Only the primary instance initializes. A second launch fails the lock
// above (app.quit) and must NOT register whenReady, otherwise it briefly
// builds a duplicate tray icon before quitting. The running instance gets
// the "second-instance" event instead and surfaces its settings window.
if (gotLock) {
  app.on("second-instance", () => createSetupWindow());

  app.whenReady().then(() => {
    createTray();
    const cfg = loadConfig();
    cfg.profile_id ? createOverlayWindow(cfg) : createSetupWindow();
  });
}

ipcMain.handle("launch-overlay", (_, cfg) => {
  saveConfig(cfg);
  BrowserWindow.getAllWindows().forEach((w) => { if (w !== overlayWin) w.close(); });
  createOverlayWindow(cfg);
});

ipcMain.handle("get-config", () => loadConfig());

app.on("window-all-closed", () => {});
app.on("will-quit", () => globalShortcut.unregisterAll());
