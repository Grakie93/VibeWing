const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('vibeWingDesktop', {
  copyText: text => ipcRenderer.invoke('vibewing:copy-text', String(text || '')),
  setThemeIcon: dark => ipcRenderer.invoke('vibewing:set-theme-icon', Boolean(dark)),
});
