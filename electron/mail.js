import { ImapFlow } from 'imapflow';
import nodemailer from 'nodemailer';
import { simpleParser } from 'mailparser';

export class MailService {
    constructor(config, onError = null, onNewMail = null) {
        this.config = config;
        this.onError = onError;
        this.onNewMail = onNewMail;
        this.client = null;
        this.idleLock = null;
    }

    async connect() {
        this.client = new ImapFlow({
            host: this.config.host,
            port: this.config.port,
            secure: this.config.secure !== false,
            auth: {
                user: this.config.user,
                pass: this.config.pass,
            },
            tls: {
                rejectUnauthorized: false
            },
            logger: false,
        });

        this.client.on('error', (err) => {
            console.error(`IMAP Client Error (${this.config.user}):`, err.message || err);
            if (this.onError) {
                this.onError(err);
            }
            this.client = null;
        });

        // 监听邮箱变动事件
        this.client.on('exists', (data) => {
            console.log(`[MailService] Mailbox update for ${this.config.user}:`, data);
            if (this.onNewMail) this.onNewMail(data);
        });

        await this.client.connect();
    }

    async startIdle(folder = 'INBOX') {
        if (!this.client) return;
        console.log(`[MailService] Starting IDLE for ${this.config.user} on ${folder}`);
        
        // 持续保持一个邮箱处于选中并 IDLE 状态
        while (this.client) {
            try {
                this.idleLock = await this.client.getMailboxLock(folder);
                // imapflow 在锁定期且无操作时会自动 IDLE
                // 如果锁释放，则说明活跃操作需要使用连接，或者断开了
                await this.idleLock.release(); 
                // 注意：在 imapflow 中，只要不显式退出，它就会尽量保持连接
                // 对于 IDLE，我们其实只需要确保定期有活动或者保持选中状态
                await new Promise(resolve => setTimeout(resolve, 60000)); // 每分钟检查一次状态
            } catch (e) {
                console.warn(`[MailService] IDLE loop error for ${this.config.user}:`, e.message);
                break;
            }
        }
    }

    isConnected() {
        return this.client && (this.client.state === 'authenticated' || this.client.state === 'connected');
    }

    async disconnect() {
        if (this.client) {
            await this.client.logout();
            this.client = null;
        }
    }

    async getFolders() {
        if (!this.client) throw new Error('Not connected');
        const tree = await this.client.listTree();

        const flattenFolders = (folders, parentPath = '') => {
            const result = [];
            for (const folder of folders) {
                result.push({
                    name: folder.name,
                    path: folder.path,
                    specialUse: folder.specialUse || null,
                    delimiter: folder.delimiter,
                });
                if (folder.folders && folder.folders.length > 0) {
                    result.push(...flattenFolders(folder.folders, folder.path));
                }
            }
            return result;
        };

        return flattenFolders(tree.folders || []);
    }

    async getMessages(folder = 'INBOX', page = 1, pageSize = 50) {
        if (!this.client) throw new Error('Not connected');

        const lock = await this.client.getMailboxLock(folder);
        try {
            // 关键：强制获取最新的邮箱状态（exists, unseen 等）
            // status 命令比 noop 更能确保获得最新的数值信息
            await this.client.status(folder, { messages: true });
            const mailbox = this.client.mailbox;
            const total = mailbox.exists;

            if (total === 0) {
                return { messages: [], total, page, pageSize };
            }

            const start = Math.max(1, total - (page * pageSize) + 1);
            const end = Math.max(1, total - ((page - 1) * pageSize));
            const range = `${start}:${end}`;

            const messages = [];

            for await (let msg of this.client.fetch(range, {
                envelope: true,
                flags: true,
                uid: true,
                bodyStructure: true,
                size: true,
            })) {
                const env = msg.envelope;
                messages.push({
                    uid: msg.uid,
                    seq: msg.seq,
                    flags: Array.from(msg.flags || []),
                    size: msg.size,
                    subject: env.subject || '(无主题)',
                    from: env.from?.[0] ? {
                        name: env.from[0].name || env.from[0].address,
                        address: env.from[0].address,
                    } : { name: '未知', address: '' },
                    to: (env.to || []).map(t => ({
                        name: t.name || t.address,
                        address: t.address,
                    })),
                    date: env.date ? new Date(env.date).toISOString() : null,
                    messageId: env.messageId,
                    seen: msg.flags?.has('\\Seen') || false,
                    flagged: msg.flags?.has('\\Flagged') || false,
                });
            }

            messages.sort((a, b) => new Date(b.date) - new Date(a.date));

            return { messages, total, page, pageSize };
        } finally {
            lock.release();
        }
    }

    async getMessage(folder, uid) {
        if (!this.client) throw new Error('Not connected');

        const lock = await this.client.getMailboxLock(folder);
        try {
            const source = await this.client.download(uid.toString(), undefined, { uid: true });
            const parsed = await simpleParser(source.content);

            // Mark as seen
            await this.client.messageFlagsAdd(uid.toString(), ['\\Seen'], { uid: true });

            return {
                uid,
                subject: parsed.subject || '(无主题)',
                from: parsed.from?.value?.[0] ? {
                    name: parsed.from.value[0].name || parsed.from.value[0].address,
                    address: parsed.from.value[0].address,
                } : { name: '未知', address: '' },
                to: (parsed.to?.value || []).map(t => ({
                    name: t.name || t.address,
                    address: t.address,
                })),
                cc: (parsed.cc?.value || []).map(t => ({
                    name: t.name || t.address,
                    address: t.address,
                })),
                date: parsed.date ? parsed.date.toISOString() : null,
                html: parsed.html || null,
                text: parsed.text || null,
                attachments: (parsed.attachments || []).map(att => ({
                    filename: att.filename || '未命名附件',
                    contentType: att.contentType,
                    size: att.size,
                    content: att.content?.toString('base64') || null,
                })),
            };
        } finally {
            lock.release();
        }
    }

    async deleteMessage(folder, uid) {
        if (!this.client) throw new Error('Not connected');
        const lock = await this.client.getMailboxLock(folder);
        try {
            await this.client.messageFlagsAdd(uid.toString(), ['\\Deleted'], { uid: true });
            await this.client.messageDelete(uid.toString(), { uid: true });
        } finally {
            lock.release();
        }
    }

    async moveMessage(folder, uid, destination) {
        if (!this.client) throw new Error('Not connected');
        const lock = await this.client.getMailboxLock(folder);
        try {
            await this.client.messageMove(uid.toString(), destination, { uid: true });
        } finally {
            lock.release();
        }
    }

    async markSeen(folder, uid) {
        if (!this.client) throw new Error('Not connected');
        const lock = await this.client.getMailboxLock(folder);
        try {
            await this.client.messageFlagsAdd(uid.toString(), ['\\Seen'], { uid: true });
        } finally {
            lock.release();
        }
    }

    async markUnseen(folder, uid) {
        if (!this.client) throw new Error('Not connected');
        const lock = await this.client.getMailboxLock(folder);
        try {
            await this.client.messageFlagsRemove(uid.toString(), ['\\Seen'], { uid: true });
        } finally {
            lock.release();
        }
    }

    async appendMessage(folder, content, flags = []) {
        if (!this.client) throw new Error('Not connected');
        try {
            await this.client.append(folder, content, flags);
        } catch (error) {
            console.error('Append error:', error);
            throw error;
        }
    }

    static async sendMail(smtpConfig, mailOptions) {
        const transporter = nodemailer.createTransport({
            host: smtpConfig.host,
            port: smtpConfig.port,
            secure: smtpConfig.secure !== false,
            auth: {
                user: smtpConfig.user,
                pass: smtpConfig.pass,
            },
            tls: {
                rejectUnauthorized: false
            }
        });

        console.log(`Attempting to send mail from ${smtpConfig.user} via ${smtpConfig.host}:${smtpConfig.port}`);

        await transporter.sendMail({
            from: mailOptions.from,
            to: mailOptions.to,
            cc: mailOptions.cc,
            subject: mailOptions.subject,
            text: mailOptions.text,
            html: mailOptions.html,
            attachments: mailOptions.attachments || [],
        });
    }
}
