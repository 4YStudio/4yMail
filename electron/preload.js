const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
    // Window controls
    minimize: () => ipcRenderer.send('window-minimize'),
    maximize: () => ipcRenderer.send('window-maximize'),
    close: () => ipcRenderer.send('window-close'),
    isMaximized: () => ipcRenderer.invoke('window-is-maximized'),

    // Mail operations
    connect: (config) => ipcRenderer.invoke('mail-connect', config),
    disconnect: (accountId) => ipcRenderer.invoke('mail-disconnect', accountId),
    getFolders: (accountId) => ipcRenderer.invoke('mail-get-folders', accountId),
    getMessages: (params) => ipcRenderer.invoke('mail-get-messages', params),
    getMessage: (params) => ipcRenderer.invoke('mail-get-message', params),
    sendMail: (params) => ipcRenderer.invoke('mail-send', params),
    deleteMail: (params) => ipcRenderer.invoke('mail-delete', params),
    moveMail: (params) => ipcRenderer.invoke('mail-move', params),
    markSeen: (params) => ipcRenderer.invoke('mail-mark-seen', params),
    markUnseen: (params) => ipcRenderer.invoke('mail-mark-unseen', params),
    saveDraft: (params) => ipcRenderer.invoke('mail-save-draft', params),
    updatePollingInterval: (interval) => ipcRenderer.invoke('mail-update-polling', interval),

    // Utils
    openExternal: (url) => ipcRenderer.invoke('open-external', url),
    openFileDialog: () => ipcRenderer.invoke('dialog-open-files'),
    onConnectionError: (callback) => ipcRenderer.on('mail-connection-error', (_event, value) => callback(value)),
    onNewMailArrived: (callback) => ipcRenderer.on('mail-new-arrived', (_event, value) => callback(value)),
});
