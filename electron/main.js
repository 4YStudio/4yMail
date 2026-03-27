import { app, BrowserWindow, ipcMain, shell, nativeImage, Tray, Menu, Notification, dialog, clipboard } from 'electron';
import path from 'path';
import { fileURLToPath } from 'url';
import { exec } from 'child_process';
import fs from 'fs';
import os from 'os';
import { MailManager } from './manager.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// 设置应用名称和识别码，这对通知显示非常重要
app.setName('4yMail');
if (process.platform === 'win32') {
  app.setAppUserModelId('com.4ymail.app');
}

let mainWindow;
let tray;
let mailManager;
let isAutoStart = false;
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
      autoplayPolicy: 'no-user-gesture-required', // 允许自动播放提示音
    },
  });

  if (isDev) {
    mainWindow.loadURL('http://localhost:5173');
  } else {
    mainWindow.loadFile(path.join(__dirname, '../dist/index.html'));
  }

  // 初始化邮件管理器
  mailManager = new MailManager(mainWindow);

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
        mainWindow?.webContents.send('mail-refresh-all');
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

// 移除旧的轮询函数，功能已整合到 MailManager

function sendSystemNotification(title, body) {
  const iconPath = isDev
    ? path.join(__dirname, '../public/logo.png')
    : path.join(__dirname, '../dist/logo.png');

  console.log(`[Notification] Sending: ${title} - ${body}`);

  // 优先尝试 Electron 原生通知
  if (Notification.isSupported()) {
    try {
      const notif = new Notification({ 
        title, 
        body, 
        icon: iconPath,
        silent: false 
      });
      notif.on('click', () => {
        mainWindow?.show();
        mainWindow?.focus();
      });
      notif.show();
      return;
    } catch (e) {
      console.warn('[Notification] Electron native notification error:', e.message);
    }
  }

  // Linux 回退方案: 使用 notify-send
  try {
    // 基础过滤，防止注入
    const cleanTitle = title.replace(/"/g, '\\"').replace(/`/g, '\\`');
    const cleanBody = body.replace(/"/g, '\\"').replace(/`/g, '\\`');
    
    // 使用 -u critical 确保通知在锁屏或后台也能显示（可选）
    // 使用 -a 指定应用名
    const command = `notify-send -a "4yMail" -i "${iconPath}" "${cleanTitle}" "${cleanBody}"`;
    exec(command, (err) => {
      if (err) console.error('[Notification] notify-send failed:', err.message);
    });
  } catch (err) {
    console.error('[Notification] Fallback error:', err.message);
  }
}

// checkNewEmails 功能已移动到 MailManager

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

app.on('window-all-closed', () => {
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
  return await mailManager.connectAccount(config.imap.user, config.imap);
});

ipcMain.handle('mail-disconnect', async (event, accountId) => {
  await mailManager.disconnectAccount(accountId);
  return { success: true };
});

ipcMain.handle('mail-get-folders', async (event, accountId) => {
  try {
    const service = mailManager.getService(accountId);
    if (!service) throw new Error('账号未连接');
    const folders = await service.getFolders();
    return { success: true, data: folders };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-get-messages', async (event, { accountId, folder, page, pageSize }) => {
  try {
    const service = mailManager.getService(accountId);
    if (!service) throw new Error('账号未连接');
    const messages = await service.getMessages(folder, page, pageSize);
    return { success: true, data: messages };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-get-message', async (event, { accountId, folder, uid }) => {
  try {
    const service = mailManager.getService(accountId);
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
    const service = mailManager.getService(accountId);
    if (!service) throw new Error('账号未连接');

    // 动态解析草稿箱路径
    const folders = await service.getFolders();
    const draftFolder = folders.find(f => f.specialUse === '\\Drafts') ||
      folders.find(f => f.path.toLowerCase().includes('draft')) ||
      { path: 'INBOX' }; // 兜底

    await service.appendMessage(draftFolder.path, content, ['\\Draft']);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-get-sound', async () => {
  try {
    const soundPath = isDev 
      ? path.join(__dirname, '../public/sounds/message-sound.mp3')
      : path.join(app.getAppPath(), 'dist/sounds/message-sound.mp3');
      
    console.log(`[Mail] Requested sound path: ${soundPath}`);
    
    if (fs.existsSync(soundPath)) {
      const data = fs.readFileSync(soundPath);
      console.log(`[Mail] Sound file found, size: ${data.length} bytes`);
      return `data:audio/mpeg;base64,${data.toString('base64')}`;
    } else {
      console.warn(`[Mail] Sound file NOT found at: ${soundPath}`);
    }
    return null;
  } catch (err) {
    console.error('[Mail] Get sound error:', err);
    return null;
  }
});

ipcMain.handle('mail-delete', async (event, { accountId, folder, uid }) => {
  try {
    const service = mailManager.getService(accountId);
    if (!service) throw new Error('账号未连接');
    await service.deleteMessage(folder, uid);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-move', async (event, { accountId, folder, uid, destination }) => {
  try {
    const service = mailManager.getService(accountId);
    if (!service) throw new Error('账号未连接');
    await service.moveMessage(folder, uid, destination);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-mark-seen', async (event, { accountId, folder, uid }) => {
  try {
    const service = mailManager.getService(accountId);
    if (!service) throw new Error('账号未连接');
    await service.markSeen(folder, uid);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-mark-unseen', async (event, { accountId, folder, uid }) => {
  try {
    const service = mailManager.getService(accountId);
    if (!service) throw new Error('账号未连接');
    await service.markUnseen(folder, uid);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

// 监听来自 Manager 的通知请求
ipcMain.on('request-system-notification', (event, { title, body }) => {
  sendSystemNotification(title, body);
});

// 测试工具箱：手动触发通知和音效
ipcMain.on('test-notification', () => {
  console.log('[Test] Triggering test notification and sound');
  sendSystemNotification('测试通知', '这是一条来自 4yMail 测试工具箱的测试消息。');
  mainWindow?.webContents.send('play-sound');
});

// 测试工具箱：模拟连接错误
ipcMain.on('test-connection-error', () => {
  mailManager.scheduleReconnect('test@example.com');
  mainWindow?.webContents.send('mail-connection-error', {
    accountId: 'test@example.com',
    error: '这是一条模拟的连接断开错误'
  });
});

// 测试工具箱：清空轮询缓存指标
ipcMain.on('test-clear-cache', () => {
  console.log('[Test] Clearing polling cache via Manager');
  mailManager.states = {};
  mailManager.saveState();
  mainWindow?.webContents.send('notif-message', { message: '轮询缓存（持久化）已清空', type: 'info' });
});

// 测试工具箱：打开日志/主目录
ipcMain.on('test-open-logs', () => {
  const logPath = app.getPath('userData');
  console.log(`[Test] Opening app directory: ${logPath}`);
  shell.openPath(logPath);
});
