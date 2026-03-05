import { app, BrowserWindow, ipcMain, shell, nativeImage } from 'electron';
import path from 'path';
import { fileURLToPath } from 'url';
import { MailService } from './mail.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

let mainWindow;
let mailService = null;

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

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

app.whenReady().then(createWindow);

app.on('window-all-closed', () => {
  if (mailService) {
    mailService.disconnect();
  }
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
  try {
    mailService = new MailService(config);
    await mailService.connect();
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-disconnect', async () => {
  try {
    if (mailService) {
      await mailService.disconnect();
      mailService = null;
    }
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-get-folders', async () => {
  try {
    if (!mailService) throw new Error('未连接到邮箱');
    const folders = await mailService.getFolders();
    return { success: true, data: folders };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-get-messages', async (event, { folder, page, pageSize }) => {
  try {
    if (!mailService) throw new Error('未连接到邮箱');
    const messages = await mailService.getMessages(folder, page, pageSize);
    return { success: true, data: messages };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-get-message', async (event, { folder, uid }) => {
  try {
    if (!mailService) throw new Error('未连接到邮箱');
    const message = await mailService.getMessage(folder, uid);
    return { success: true, data: message };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-send', async (event, { smtpConfig, mailOptions }) => {
  try {
    await MailService.sendMail(smtpConfig, mailOptions);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-delete', async (event, { folder, uid }) => {
  try {
    if (!mailService) throw new Error('未连接到邮箱');
    await mailService.deleteMessage(folder, uid);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('mail-move', async (event, { folder, uid, destination }) => {
  try {
    if (!mailService) throw new Error('未连接到邮箱');
    await mailService.moveMessage(folder, uid, destination);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
});

ipcMain.handle('open-external', async (event, url) => {
  shell.openExternal(url);
});
