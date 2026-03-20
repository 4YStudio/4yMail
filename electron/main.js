import { app, BrowserWindow, ipcMain, shell, nativeImage, Tray, Menu, Notification, dialog, clipboard } from 'electron';
import path from 'path';
import { fileURLToPath } from 'url';
import { exec } from 'child_process';
import fs from 'fs';
import os from 'os';
import { MailService } from './mail.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

let mainWindow;
let tray;
// 记录是否是开机自启动
let isAutoStart = false;
// Map 管理多个账号的服务实例：Map<accountId, MailService>
const mailServices = new Map();
// 记录每个账号最后一次检查到的最大 UID: Map<accountId, number>
const lastMaxUids = new Map();
let pollingInterval = 5; // 默认 5 分钟
let pollingTimer = null;
let isQuitting = false;

const isDev = process.env.NODE_ENV === 'development';

function createWindow(showWindow = true) {
  const iconPath = isDev
    ? path.join(__dirname, '../public/logo.png')
    : path.join(__dirname, '../dist/logo.png');

  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    minWidth: 900,
    minHeight: 600,
    frame: false,
    transparent: false,
    backgroundColor: '#0f0f1a',
    icon: iconPath,
    show: showWindow, // 控制是否显示窗口
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  if (isDev) {
    mainWindow.loadURL('http://localhost:5173');
  } else {
    mainWindow.loadFile(path.join(__dirname, '../dist/index.html'));
  }

  mainWindow.on('close', (event) => {
    if (!isQuitting) {
      event.preventDefault();
      mainWindow.hide();
    }
  });

  mainWindow.on('closed', () => {
    mainWindow = null;
  });

  // Handle external links (target="_blank")
  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith('http://') || url.startsWith('https://') || url.startsWith('mailto:')) {
      shell.openExternal(url);
    }
    return { action: 'deny' };
  });

  // Handle same-window navigation to external links
  mainWindow.webContents.on('will-navigate', (event, url) => {
    // Only intercept if it's an external HTTP/HTTPS link, 
    // avoiding intercepting local file:// or dev server localhost
    const isLocalhost = isDev && url.startsWith('http://localhost:5173');
    if (!isLocalhost && (url.startsWith('http://') || url.startsWith('https://') || url.startsWith('mailto:'))) {
      event.preventDefault();
      shell.openExternal(url);
    }
  });
}

function createTray() {
  const iconPath = isDev
    ? path.join(__dirname, '../public/logo.png')
    : path.join(__dirname, '../dist/logo.png');

  tray = new Tray(nativeImage.createFromPath(iconPath).resize({ width: 16, height: 16 }));

  const contextMenu = Menu.buildFromTemplate([
    {
      label: '显示主界面',
      click: () => {
        mainWindow?.show();
      }
    },
    {
      label: '立即检查邮件',
      click: () => {
        checkAllAccounts();
      }
    },
    { type: 'separator' },
    {
      label: '退出',
      click: () => {
        isQuitting = true;
        app.quit();
      }
    }
  ]);

  tray.setToolTip('4yMail');
  tray.setContextMenu(contextMenu);

  tray.on('double-click', () => {
    mainWindow?.show();
  });
}

function startPolling(intervalMinutes) {
  if (intervalMinutes) pollingInterval = intervalMinutes;
  stopPolling();
  // 根据用户配置的时间间隔启动
  pollingTimer = setInterval(checkAllAccounts, pollingInterval * 60 * 1000);
  // 稍后立即检查一次
  setTimeout(checkAllAccounts, 5000);
}

function stopPolling() {
  if (pollingTimer) {
    clearInterval(pollingTimer);
    pollingTimer = null;
  }
}

async function checkAllAccounts() {
  for (const [accountId, service] of mailServices.entries()) {
    try {
      await checkNewEmails(accountId, service);
    } catch (err) {
      console.error(`Check failed for ${accountId}:`, err.message);
    }
  }
}

function sendSystemNotification(title, body) {
  const iconPath = isDev
    ? path.join(__dirname, '../public/logo.png')
    : path.join(__dirname, '../dist/logo.png');

  // 优先尝试 Electron 原生通知
  if (Notification.isSupported()) {
    try {
      const notif = new Notification({ title, body, icon: iconPath });
      notif.show();
      return;
    } catch (e) {
      console.warn('Electron Notification failed, falling back to notify-send:', e.message);
    }
  }

  // KDE6/Linux fallback: 使用 notify-send
  const escapedTitle = title.replace(/'/g, "'\\''");
  const escapedBody = body.replace(/'/g, "'\\''");
  exec(`notify-send -a '4yMail' -i '${iconPath}' '${escapedTitle}' '${escapedBody}'`, (err) => {
    if (err) console.error('notify-send failed:', err.message);
  });
}

async function checkNewEmails(accountId, service) {
  if (!service || !service.client) return;
  try {
    const res = await service.getMessages('INBOX', 1, 5);
    if (res && res.messages && res.messages.length > 0) {
      const currentMaxUid = Math.max(...res.messages.map(m => m.uid));
      const lastMaxUid = lastMaxUids.get(accountId) || 0;

      if (lastMaxUid === 0) {
        lastMaxUids.set(accountId, currentMaxUid);
        return;
      }

      if (currentMaxUid > lastMaxUid) {
        const newMails = res.messages.filter(m => m.uid > lastMaxUid);
        if (newMails.length > 0) {
          const latest = newMails[0];
          sendSystemNotification(
            `4yMail - ${accountId} (${newMails.length} 封新邮件)`,
            `${latest.from.name}: ${latest.subject}`
          );

          mainWindow?.webContents.send('mail-new-arrived', { accountId });
        }
        lastMaxUids.set(accountId, currentMaxUid);
      }
    }
  } catch (error) {
    console.error(`Polling error for ${accountId}:`, error);
  }
}

app.whenReady().then(() => {
  // 检测是否是开机自启动
  let shouldCheckAutostart = false;
  
  if (process.platform === 'win32') {
    // Windows: 通过命令行参数判断
    // 如果设置了开机自启动，Electron 会以 --hidden 或静默方式启动
    shouldCheckAutostart = true;
    isAutoStart = process.argv.some(arg => 
      arg.includes('--hidden') || 
      arg.includes('--silent') || 
      arg.includes('--minimized')
    );
  } else if (process.platform === 'linux') {
    // Linux: 通过环境变量或参数判断
    shouldCheckAutostart = true;
    isAutoStart = process.env.STARTUP_NOTIFICATION === '0' || 
                  process.argv.some(arg => arg === '--hidden' || arg === '--minimized');
  }
  
  // 如果不是开机自启动，则显示窗口
  createWindow(!isAutoStart);
  createTray();
});

app.on('window-all-closed', async () => {
  for (const [id, service] of mailServices.entries()) {
    try {
      await service.disconnect();
    } catch (e) {
      console.error(`Disconnect error for ${id}:`, e.message);
    }
  }
  mailServices.clear();
  app.quit();
});

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow();
  }
});

// Window controls
ipcMain.on('window-minimize', () => mainWindow?.minimize());
ipcMain.on('window-maximize', () => {
  if (mainWindow?.isMaximized()) {
    mainWindow.unmaximize();
  } else {
    mainWindow?.maximize();
  }
});
ipcMain.on('window-close', () => mainWindow?.close());
ipcMain.handle('window-is-maximized', () => mainWindow?.isMaximized());

// Mail operations
ipcMain.handle('mail-connect', async (event, config) => {
  const accountId = config.imap.user;
  try {
    const handleError = (err) => {
      console.error(`Connection Error (${accountId}):`, err);
      mainWindow?.webContents.send('mail-connection-error', {
        accountId,
        error: err.message || '连接异常断开'
      });
      mailServices.delete(accountId);
    };
    const service = new MailService(config.imap, handleError);
    await service.connect();
    mailServices.set(accountId, service);
    // 连接第一个账号时或配置变化时启动轮询
    startPolling();
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-disconnect', async (event, accountId) => {
  try {
    const service = mailServices.get(accountId);
    if (service) {
      await service.disconnect();
      mailServices.delete(accountId);
      lastMaxUids.delete(accountId);
    }
    if (mailServices.size === 0) stopPolling();
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-get-folders', async (event, accountId) => {
  try {
    const service = mailServices.get(accountId);
    if (!service) throw new Error('账号未连接');
    const folders = await service.getFolders();
    return { success: true, data: folders };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-get-messages', async (event, { accountId, folder, page, pageSize }) => {
  try {
    const service = mailServices.get(accountId);
    if (!service) throw new Error('账号未连接');
    const messages = await service.getMessages(folder, page, pageSize);
    return { success: true, data: messages };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-get-message', async (event, { accountId, folder, uid }) => {
  try {
    const service = mailServices.get(accountId);
    if (!service) throw new Error('账号未连接');
    const message = await service.getMessage(folder, uid);
    return { success: true, data: message };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-send', async (event, { smtpConfig, mailOptions }) => {
  try {
    if (!smtpConfig || !smtpConfig.user || !smtpConfig.pass) {
      throw new Error('SMTP 凭据缺失');
    }
    await MailService.sendMail(smtpConfig, mailOptions);
    return { success: true };
  } catch (error) {
    console.error('SMTP Send Error:', error);
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-save-draft', async (event, { accountId, content }) => {
  try {
    const service = mailServices.get(accountId);
    if (!service) throw new Error('账号未连接');

    // 动态解析草稿箱路径
    const folders = await service.client.list();
    const draftFolder = folders.find(f => f.specialUse === '\\Drafts') ||
      folders.find(f => f.path.toLowerCase().includes('draft')) ||
      { path: 'INBOX' }; // 兜底

    await service.appendMessage(draftFolder.path, content, ['\\Draft']);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-delete', async (event, { accountId, folder, uid }) => {
  try {
    const service = mailServices.get(accountId);
    if (!service) throw new Error('账号未连接');
    await service.deleteMessage(folder, uid);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-move', async (event, { accountId, folder, uid, destination }) => {
  try {
    const service = mailServices.get(accountId);
    if (!service) throw new Error('账号未连接');
    await service.moveMessage(folder, uid, destination);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-mark-seen', async (event, { accountId, folder, uid }) => {
  try {
    const service = mailServices.get(accountId);
    if (!service) throw new Error('账号未连接');
    await service.markSeen(folder, uid);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-mark-unseen', async (event, { accountId, folder, uid }) => {
  try {
    const service = mailServices.get(accountId);
    if (!service) throw new Error('账号未连接');
    await service.markUnseen(folder, uid);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-update-polling', (event, interval) => {
  startPolling(interval);
  return { success: true };
});

ipcMain.handle('open-external', async (event, url) => {
  shell.openExternal(url);
});

ipcMain.handle('dialog-open-files', async () => {
  const result = await dialog.showOpenDialog(mainWindow, {
    properties: ['openFile', 'multiSelections']
  });
  return result;
});

ipcMain.handle('app-set-autostart', (event, openAtLogin) => {
  // 获取最准确的可执行文件路径
  let exePath;
  if (process.platform === 'win32') {
    // 优先使用 PORTABLE_EXECUTABLE_FILE（如果存在），否则使用 process.execPath
    exePath = process.env.PORTABLE_EXECUTABLE_FILE || process.execPath;
  } else {
    // Linux 端优先使用 APPIMAGE 环境变量，如果不是以 AppImage 运行则使用 app.getPath('exe')
    exePath = process.env.APPIMAGE || app.getPath('exe');
  }
  
  console.log('[Autostart SET] Platform:', process.platform);
  console.log('[Autostart SET] Exe Path:', exePath);
  console.log('[Autostart SET] openAtLogin:', openAtLogin);
  
  if (process.platform === 'win32') {
    try {
      app.setLoginItemSettings({
        openAtLogin: openAtLogin,
        path: exePath,
        args: ['--hidden'],
        name: '4yMail'
      });
      return { success: true };
    } catch (err) {
      console.error('[Autostart SET] Win32 Error:', err);
      return { success: false, error: err.message };
    }
  } else if (process.platform === 'linux') {
    const autostartDir = path.join(os.homedir(), '.config', 'autostart');
    const desktopPath = path.join(autostartDir, '4ymail.desktop');
    
    if (openAtLogin) {
      try {
        if (!fs.existsSync(autostartDir)) {
          fs.mkdirSync(autostartDir, { recursive: true });
        }
        
        const desktopContent = `[Desktop Entry]
Type=Application
Name=4yMail
Comment=4yMail email client
Exec="${exePath}" --hidden
Terminal=false
Icon=4ymail
X-GNOME-Autostart-enabled=true
Hidden=false
NoDisplay=false
X-GNOME-AutoRestart=false
StartupNotify=false
`;
        fs.writeFileSync(desktopPath, desktopContent, 'utf-8');
        fs.chmodSync(desktopPath, '755');
        return { success: true };
      } catch (err) {
        console.error('[Autostart SET] Linux Error:', err);
        return { success: false, error: err.message };
      }
    } else {
      try {
        if (fs.existsSync(desktopPath)) {
          fs.unlinkSync(desktopPath);
        }
        return { success: true };
      } catch (err) {
        console.error('[Autostart SET] Linux Remove Error:', err);
        return { success: false, error: err.message };
      }
    }
  } else {
    // macOS
    try {
      app.setLoginItemSettings({
        openAtLogin: openAtLogin,
        path: exePath,
        args: []
      });
      return { success: true };
    } catch (err) {
      return { success: false, error: err.message };
    }
  }
});

ipcMain.handle('app-get-autostart', () => {
  let exePath;
  if (process.platform === 'win32') {
    exePath = process.env.PORTABLE_EXECUTABLE_FILE || process.execPath;
  } else {
    exePath = process.env.APPIMAGE || app.getPath('exe');
  }
  
  if (process.platform === 'win32') {
    try {
      const settings = app.getLoginItemSettings({
        path: exePath,
        args: ['--hidden']
      });
      return { success: true, enabled: settings.openAtLogin };
    } catch (err) {
      return { success: false, error: err.message, enabled: false };
    }
  } else if (process.platform === 'linux') {
    const autostartDir = path.join(os.homedir(), '.config', 'autostart');
    const desktopPath = path.join(autostartDir, '4ymail.desktop');
    
    try {
      if (!fs.existsSync(desktopPath)) {
        return { success: true, enabled: false };
      }
      
      const content = fs.readFileSync(desktopPath, 'utf-8');
      
      // 检查是否包含关键行
      if (content.includes('Hidden=true') || content.includes('X-GNOME-Autostart-enabled=false')) {
        return { success: true, enabled: false };
      }

      // 简单起见，只要文件存在且未显式禁用，且包含 Exec="/path/to/exe" 就算已启用
      // 我们不进行严苛的完全路径匹配，因为 AppImage 路径在运行时可能会变化
      const execLine = content.split('\n').find(line => line.startsWith('Exec='));
      if (!execLine) return { success: true, enabled: false };
      
      // 如果 Exec 行包含了当前应用的标识符（不分大小写），则认为匹配
      const appIdentifier = '4yMail';
      if (execLine.toLowerCase().includes(appIdentifier.toLowerCase())) {
        return { success: true, enabled: true };
      }
      
      return { success: true, enabled: false };
    } catch (err) {
      console.error('[Autostart GET] Linux Error:', err);
      return { success: false, error: err.message, enabled: false };
    }
  } else {
    // macOS fallback
    try {
      const settings = app.getLoginItemSettings({
        path: exePath
      });
      return { success: true, enabled: settings.openAtLogin };
    } catch (err) {
      return { success: false, error: err.message, enabled: false };
    }
  }
});
