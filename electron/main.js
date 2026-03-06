import { app, BrowserWindow, ipcMain, shell, nativeImage, Tray, Menu, Notification, dialog } from 'electron';
import path from 'path';
import { fileURLToPath } from 'url';
import { exec } from 'child_process';
import { MailService } from './mail.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

let mainWindow;
let tray;
// 改为 Map 管理多个账号的服务实例: Map<accountId, MailService>
const mailServices = new Map();
// 记录每个账号最后一次检查到的最大 UID: Map<accountId, number>
const lastMaxUids = new Map();
let pollingInterval = 5; // 默认 5 分钟
let pollingTimer = null;
let isQuitting = false;

const isDev = process.env.NODE_ENV === 'development';

function createWindow() {
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
  createWindow();
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

// App settings
ipcMain.handle('app-set-autostart', (event, openAtLogin) => {
  app.setLoginItemSettings({
    openAtLogin: openAtLogin,
    path: app.getPath('exe'),
  });
  return { success: true };
});

ipcMain.handle('app-get-autostart', () => {
  const settings = app.getLoginItemSettings();
  return { success: true, enabled: settings.openAtLogin };
});
