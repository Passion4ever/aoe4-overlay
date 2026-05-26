const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("electronAPI", {
  launchOverlay: (cfg) => ipcRenderer.invoke("launch-overlay", cfg),
  getConfig: () => ipcRenderer.invoke("get-config"),
  openExternal: (url) => ipcRenderer.invoke("open-external", url),
});
