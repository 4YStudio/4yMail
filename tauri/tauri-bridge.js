/**
 * Tauri ↔ Electron API 兼容层
 * 将 window.electronAPI 的所有调用映射到 Tauri invoke
 * 使现有 Vue 前端代码无需修改即可在 Tauri 中运行
 */

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open as shellOpen } from '@tauri-apps/plugin-shell';
import { open as dialogOpen, save as dialogSave } from '@tauri-apps/plugin-dialog';
import { isEnabled, enable, disable } from '@tauri-apps/plugin-autostart';
import { onAction } from '@tauri-apps/plugin-notification';

const appWindow = getCurrentWindow();
window.__IS_TAURI__ = true;

// 全局通知点击监听：点击任何通知都唤回并聚焦主窗口
onAction(() => {
  appWindow.show();
  appWindow.focus();
});

window.electronAPI = {
  // ========== 窗口控制 ==========
  minimize: () => appWindow.minimize(),

  maximize: async () => {
    if (await appWindow.isMaximized()) {
      await appWindow.unmaximize();
    } else {
      await appWindow.maximize();
    }
  },

  close: () => appWindow.close(),

  isMaximized: () => appWindow.isMaximized(),

  // ========== 邮件操作 ==========
  connect: (config) => invoke('mail_connect', { config }),

  disconnect: (accountId) => invoke('mail_disconnect', { accountId }),

  getFolders: (accountId) => invoke('mail_get_folders', { accountId }),

  getMessages: (params) => invoke('mail_get_messages', { params }),

  getMessage: (params) => invoke('mail_get_message', { params }),

  sendMail: (params) => invoke('mail_send', { params }),

  deleteMail: (params) => invoke('mail_delete', { params }),

  moveMail: (params) => invoke('mail_move', { params }),

  markSeen: (params) => invoke('mail_mark_seen', { params }),

  markUnseen: (params) => invoke('mail_mark_unseen', { params }),

  saveDraft: (params) => invoke('mail_save_draft', { params }),
 
  updatePollingInterval: (interval) => invoke('mail_update_polling', { interval }),
 
  getSoundData: () => invoke('mail_get_sound'),
 
  downloadAttachment: (params) => invoke('mail_download_attachment', { params }),

  // ========== 测试工具箱 ==========
  testNotification: () => invoke('test_notification'),
  testConnectionError: () => invoke('test_connection_error'),
  testClearCache: () => invoke('test_clear_cache'),
  testOpenLogs: () => invoke('test_open_logs'),
 
  showNotification: (title, body) => invoke('test_notification', { title, body }), // 兼容以前的调用
 
  // ========== 工具 ==========
  openExternal: (url) => shellOpen(url),

  openFileDialog: async () => {
    try {
      const result = await dialogOpen({ multiple: true });
      if (result === null) {
        return { canceled: true, filePaths: [] };
      }
      const filePaths = Array.isArray(result) ? result : [result];
      return { canceled: false, filePaths };
    } catch (e) {
      console.error('File dialog error:', e);
      return { canceled: true, filePaths: [] };
    }
  },

  saveFileDialog: async (options) => {
    try {
      const result = await dialogSave({
        title: options?.title || '保存文件',
        defaultPath: options?.defaultPath,
        filters: options?.filters
      });
      if (result === null) {
        return { canceled: true, filePath: '' };
      }
      return { canceled: false, filePath: result };
    } catch (e) {
      console.error('Save dialog error:', e);
      return { canceled: true, filePath: '' };
    }
  },

  onConnectionError: (callback) => {
    listen('mail-connection-error', (event) => callback(event.payload));
  },

  onNewMailArrived: (callback) => {
    listen('mail-new-arrived', (event) => callback(event.payload));
  },
 
  onRefreshAll: (callback) => {
    listen('mail-refresh-all', () => callback());
  },
 
  onPlaySound: (callback) => {
    listen('mail-play-sound', () => callback());
  },
 
  onNotifMessage: (callback) => {
    listen('mail-notif-message', (event) => callback(event.payload));
  },
 
  onConnectionSuccess: (callback) => {
    listen('mail-connection-success', (event) => callback(event.payload));
  },

  // ========== 应用设置 ==========
  setZoomFactor: (factor) => {
    // Tauri 环境下的全局缩放已在 App.vue 通过计算属性 (transform: scale) 统一管理
    // 此处设为 no-op，避免与新的缩放方案冲突
    console.log('[Tauri Bridge] setZoomFactor called with:', factor);
  },

  getAutostart: async () => {
    try {
      const enabled = await isEnabled();
      return { success: true, enabled };
    } catch (err) {
      return { success: false, error: String(err), enabled: false };
    }
  },

  setAutostart: async (openAtLogin) => {
    try {
      if (openAtLogin) {
        await enable(["--hidden"]);
      } else {
        await disable();
      }
      return { success: true };
    } catch (err) {
      return { success: false, error: String(err) };
    }
  },
};
