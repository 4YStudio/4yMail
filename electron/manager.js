import fs from 'fs';
import path from 'path';
import { app } from 'electron';
import { MailService } from './mail.js';

export class MailManager {
    constructor(mainWindow) {
        this.mainWindow = mainWindow;
        this.services = new Map(); // accountId -> MailService
        this.configs = new Map();  // accountId -> config
        this.statePath = path.join(app.getPath('userData'), 'mail-state.json');
        this.states = this.loadState();
        this.reconnectTimers = new Map();
    }

    loadState() {
        try {
            if (fs.existsSync(this.statePath)) {
                return JSON.parse(fs.readFileSync(this.statePath, 'utf8'));
            }
        } catch (e) {
            console.error('[MailManager] Load state error:', e.message);
        }
        return {};
    }

    saveState() {
        try {
            fs.writeFileSync(this.statePath, JSON.stringify(this.states, null, 2));
        } catch (e) {
            console.error('[MailManager] Save state error:', e.message);
        }
    }

    async connectAccount(accountId, config) {
        console.log(`[MailManager] Connecting account: ${accountId}`);
        this.configs.set(accountId, config);
        
        // 清除现有的自动重连定时器
        if (this.reconnectTimers.has(accountId)) {
            clearTimeout(this.reconnectTimers.get(accountId));
            this.reconnectTimers.delete(accountId);
        }

        let service = this.services.get(accountId);
        if (service) {
            await service.disconnect();
        }

        const handleError = (err) => {
            console.error(`[MailManager] Error for ${accountId}:`, err.message);
            this.mainWindow?.webContents.send('mail-connection-error', { accountId, error: err.message });
            this.scheduleReconnect(accountId);
        };

        const handleNewMail = (data) => {
            this.notifyNewMail(accountId, data);
        };

        service = new MailService(config, handleError, handleNewMail);
        this.services.set(accountId, service);

        try {
            await service.connect();
            console.log(`[MailManager] ${accountId} connected successfully.`);
            this.mainWindow?.webContents.send('mail-connection-success', { accountId });
            
            // 初始化基准 UID
            if (!this.states[accountId]) {
                const res = await service.getMessages('INBOX', 1, 1);
                if (res.messages.length > 0) {
                    this.states[accountId] = { lastMaxUid: res.messages[0].uid };
                    this.saveState();
                }
            }
            
            // 启动 IDLE 监听
            service.startIdle('INBOX').catch(err => {
                console.error(`[MailManager] IDLE failed for ${accountId}:`, err.message);
                // 如果 IDLE 失败，可以在这里回退到轮询，或者重试
            });

            return { success: true };
        } catch (err) {
            console.error(`[MailManager] Connect failed for ${accountId}:`, err.message);
            this.scheduleReconnect(accountId);
            return { success: false, error: err.message };
        }
    }

    async disconnectAccount(accountId) {
        console.log(`[MailManager] Disconnecting account: ${accountId}`);
        const service = this.services.get(accountId);
        if (service) {
            await service.disconnect();
            this.services.delete(accountId);
        }
        this.configs.delete(accountId);
        if (this.reconnectTimers.has(accountId)) {
            clearTimeout(this.reconnectTimers.get(accountId));
            this.reconnectTimers.delete(accountId);
        }
    }

    scheduleReconnect(accountId) {
        if (!this.configs.has(accountId)) return;
        if (this.reconnectTimers.has(accountId)) return;

        const delay = 30000; // 30秒后重连
        console.log(`[MailManager] Scheduling reconnect for ${accountId} in ${delay}ms`);
        
        const timer = setTimeout(() => {
            this.reconnectTimers.delete(accountId);
            const config = this.configs.get(accountId);
            if (config) {
                this.connectAccount(accountId, config).catch(() => {});
            }
        }, delay);
        
        this.reconnectTimers.set(accountId, timer);
    }

    async notifyNewMail(accountId, data) {
        const service = this.services.get(accountId);
        if (!service) return;

        try {
            const res = await service.getMessages('INBOX', 1, 5);
            if (res.messages.length > 0) {
                const currentMaxUid = Math.max(...res.messages.map(m => m.uid));
                const lastMaxUid = this.states[accountId]?.lastMaxUid || 0;

                if (currentMaxUid > lastMaxUid) {
                    const newMails = res.messages.filter(m => m.uid > lastMaxUid);
                    if (newMails.length > 0) {
                        const latest = newMails[0];
                        // 直接触发逻辑，不再绕道渲染进程
                        this.mainWindow?.webContents.send('play-sound');
                        this.mainWindow?.webContents.send('mail-new-arrived', { accountId });
                        
                        // 发送系统通知（通过 IPC 内部触发或直接调用）
                        // 由于 MailManager 在主进程，可以直接调用 sendSystemNotification
                        // 但为了保持 main.js 的封装，我们可以通过 emit 一个事件
                        import('electron').then(({ ipcMain }) => {
                            ipcMain.emit('request-system-notification', null, {
                                title: `4yMail - ${accountId} (${newMails.length} 封新邮件)`,
                                body: `${latest.from.name}: ${latest.subject}`
                            });
                        });
                    }
                    this.states[accountId] = { lastMaxUid: currentMaxUid };
                    this.saveState();
                }
            }
        } catch (e) {
            console.error(`[MailManager] Fetch new mail error for ${accountId}:`, e.message);
        }
    }

    // 暴露服务方法以便 IPC 调用
    getService(accountId) {
        return this.services.get(accountId);
    }
}
