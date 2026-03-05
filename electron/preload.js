const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
    // Window controls
    minimize: () => ipcRenderer.send('window-minimize'),
    maximize: () => ipcRenderer.send('window-maximize'),
    close: () => ipcRenderer.send('window-close'),
    isMaximized: () => ipcRenderer.invoke('window-is-maximized'),

    // Mail operations
    connect: (config) => ipcRenderer.invoke('mail-connect', config),
    disconnect: () => ipcRenderer.invoke('mail-disconnect'),
    getFolders: () => ipcRenderer.invoke('mail-get-folders'),
    getMessages: (params) => ipcRenderer.invoke('mail-get-messages', params),
    getMessage: (params) => ipcRenderer.invoke('mail-get-message', params),
    sendMail: (params) => ipcRenderer.invoke('mail-send', params),
    deleteMail: (params) => ipcRenderer.invoke('mail-delete', params),
    moveMail: (params) => ipcRenderer.invoke('mail-move', params),

    // Utils
    openExternal: (url) => ipcRenderer.invoke('open-external', url),
});
