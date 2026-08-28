const { app, BrowserWindow, dialog, shell, ipcMain, clipboard, nativeImage } = require('electron');
const { spawn, execFileSync } = require('child_process');
const path = require('path');
const fs = require('fs');
const http = require('http');
const net = require('net');
const crypto = require('crypto');

app.setName('VibeWing');
if (process.platform === 'darwin') app.setActivationPolicy('regular');

let backend;
let cleanedUp = false;
let accessToken = '';
let mainWindow;
const lightIconPath = path.join(__dirname, 'build', 'icon.png');
const darkIconPath = path.join(__dirname, 'build', 'icon-dark.png');
ipcMain.handle('vibewing:copy-text', (_event, value) => {
  clipboard.writeText(String(value || ''));
  return true;
});
ipcMain.handle('vibewing:set-theme-icon', (_event, dark) => {
  const iconPath = dark && fs.existsSync(darkIconPath) ? darkIconPath : lightIconPath;
  if (!fs.existsSync(iconPath)) return false;
  const icon = nativeImage.createFromPath(iconPath);
  if (process.platform === 'darwin') app.dock.setIcon(icon);
  if (process.platform === 'win32' && mainWindow && !mainWindow.isDestroyed()) mainWindow.setIcon(icon);
  return true;
});

function commandPath() {
  const current = process.env.PATH || '';
  const candidates = [];
  if (process.platform === 'darwin') {
    try {
      const marker = '__VIBEWING_PATH__';
      const output = execFileSync(process.env.SHELL || '/bin/zsh', ['-ilc', `printf '\n${marker}%s' "$PATH"`], {
        encoding: 'utf8',
        timeout: 5000,
        env: process.env,
        stdio: ['ignore', 'pipe', 'ignore'],
      });
      const shellPath = output.slice(output.lastIndexOf(marker) + marker.length).trim();
      if (shellPath) candidates.push(...shellPath.split(path.delimiter));
    } catch {}
    candidates.push('/opt/homebrew/bin', '/opt/homebrew/sbin', '/usr/local/bin');
  } else if (process.platform === 'win32') {
    const userProfile = process.env.USERPROFILE || '';
    const appData = process.env.APPDATA || '';
    if (appData) candidates.push(path.join(appData, 'npm'));
    if (userProfile) candidates.push(path.join(userProfile, 'AppData', 'Local', 'pnpm'));
  }
  candidates.push(...current.split(path.delimiter));
  return [...new Set(candidates.filter(Boolean))].join(path.delimiter);
}

function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.unref();
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const port = server.address().port;
      server.close(() => resolve(port));
    });
  });
}

function backendCommand() {
  if (!app.isPackaged) return { command: process.platform === 'win32' ? 'python' : 'python3', args: [path.join(__dirname, 'app.py')] };
  const executable = process.platform === 'win32' ? 'vibewing-backend.exe' : 'vibewing-backend';
  return { command: path.join(process.resourcesPath, 'backend', executable), args: [] };
}

function cleanupBackend() {
  if (cleanedUp) return;
  cleanedUp = true;
  if (backend && backend.exitCode === null) backend.kill('SIGTERM');
  backend = undefined;
}

function waitForServer(port, token, tries = 120) {
  return new Promise((resolve, reject) => {
    const check = () => {
      const request = http.get({ hostname: '127.0.0.1', port, path: '/api/settings', headers: { 'X-VibeWing-Token': token } }, response => {
        response.resume();
        if (response.statusCode === 200) resolve();
        else if (tries-- <= 0) reject(new Error(`Backend returned HTTP ${response.statusCode}`));
        else setTimeout(check, 150);
      });
      request.on('error', () => {
        if (tries-- <= 0) reject(new Error('VibeWing backend did not start'));
        else setTimeout(check, 150);
      });
    };
    check();
  });
}

async function createWindow() {
  const dataDir = app.getPath('userData');
  fs.mkdirSync(dataDir, { recursive: true });
  const port = await freePort();
  const iconPath = lightIconPath;
  if (process.platform === 'darwin' && fs.existsSync(iconPath)) app.dock.setIcon(nativeImage.createFromPath(iconPath));
  accessToken = crypto.randomBytes(32).toString('hex');
  const launch = backendCommand();
  const runtimePath = commandPath();
  backend = spawn(launch.command, launch.args, {
    cwd: app.isPackaged ? process.resourcesPath : __dirname,
    windowsHide: true,
    env: { ...process.env, PATH: runtimePath, VIBEWING_DATA_DIR: dataDir, VIBEWING_PORT: String(port), VIBEWING_ACCESS_TOKEN: accessToken },
  });
  backend.stderr.on('data', data => console.error(String(data)));
  try {
    await waitForServer(port, accessToken);
  } catch (error) {
    dialog.showErrorBox('VibeWing 启动失败', `${error.message}\n\n${launch.command}`);
    return;
  }
  const window = mainWindow = new BrowserWindow({
    width: 1180,
    height: 820,
    minWidth: 860,
    minHeight: 620,
    title: 'VibeWing',
    icon: iconPath,
    backgroundColor: '#0b1020',
    webPreferences: { contextIsolation: true, sandbox: true, preload: path.join(__dirname, 'preload.js') },
  });
  window.on('closed', () => { if (mainWindow === window) mainWindow = undefined; });
  await window.loadURL(`http://127.0.0.1:${port}/?token=${encodeURIComponent(accessToken)}`, { extraHeaders: `X-VibeWing-Token: ${accessToken}\n` });
  window.webContents.setWindowOpenHandler(({ url }) => { shell.openExternal(url); return { action: 'deny' }; });
}

app.whenReady().then(createWindow);
app.on('window-all-closed', () => app.quit());
app.once('before-quit', cleanupBackend);
app.once('will-quit', cleanupBackend);
