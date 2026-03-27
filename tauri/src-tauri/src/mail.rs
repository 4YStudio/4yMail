use imap;
use native_tls::{TlsConnector, TlsStream};
use std::net::{TcpStream, ToSocketAddrs};
use serde::{Deserialize, Serialize};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport, Transport, Message as LettreMessage};
use lettre::message::{header::ContentType, MultiPart, SinglePart, Attachment, Mailbox};
use mail_parser::MimeHeaders;

// ========== 数据类型 ==========

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImapConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    #[serde(default = "default_true")]
    pub secure: bool,
}

fn default_true() -> bool { true }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    #[serde(default = "default_true")]
    pub secure: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectConfig {
    pub imap: ImapConfig,
    pub smtp: SmtpConfig,
}

#[derive(Debug, Serialize)]
pub struct MailFolder {
    pub name: String,
    pub path: String,
    #[serde(rename = "specialUse")]
    pub special_use: Option<String>,
    pub delimiter: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MailAddress {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Serialize)]
pub struct MailEnvelope {
    pub uid: u32,
    pub seq: u32,
    pub flags: Vec<String>,
    pub size: u32,
    pub subject: String,
    pub from: MailAddress,
    pub to: Vec<MailAddress>,
    pub date: Option<String>,
    #[serde(rename = "messageId")]
    pub message_id: Option<String>,
    pub seen: bool,
    pub flagged: bool,
}

#[derive(Debug, Serialize)]
pub struct MailListResult {
    pub messages: Vec<MailEnvelope>,
    pub total: u32,
    pub page: u32,
    #[serde(rename = "pageSize")]
    pub page_size: u32,
}

#[derive(Debug, Serialize)]
pub struct MailAttachment {
    pub filename: String,
    #[serde(rename = "contentType")]
    pub content_type: String,
    pub size: usize,
    pub content: Option<String>, // base64
}

#[derive(Debug, Serialize)]
pub struct MailDetail {
    pub uid: u32,
    pub subject: String,
    pub from: MailAddress,
    pub to: Vec<MailAddress>,
    pub cc: Vec<MailAddress>,
    pub date: Option<String>,
    pub html: Option<String>,
    pub text: Option<String>,
    pub attachments: Vec<MailAttachment>,
}

#[derive(Debug, Deserialize)]
pub struct MailOptions {
    pub from: String,
    pub to: String,
    pub cc: Option<String>,
    pub subject: String,
    pub text: Option<String>,
    pub html: Option<String>,
    pub attachments: Option<Vec<MailAttachmentInput>>,
}

#[derive(Debug, Deserialize)]
pub struct MailAttachmentInput {
    pub filename: String,
    pub path: String,
}

// ========== 辅助函数 ==========

fn flag_to_string(f: &imap::types::Flag<'_>) -> String {
    match f {
        imap::types::Flag::Seen => "\\Seen".to_string(),
        imap::types::Flag::Answered => "\\Answered".to_string(),
        imap::types::Flag::Flagged => "\\Flagged".to_string(),
        imap::types::Flag::Deleted => "\\Deleted".to_string(),
        imap::types::Flag::Draft => "\\Draft".to_string(),
        imap::types::Flag::Recent => "\\Recent".to_string(),
        imap::types::Flag::MayCreate => "\\MayCreate".to_string(),
        imap::types::Flag::Custom(c) => c.to_string(),
    }
}

/// 从 mail-parser 的 Address 枚举提取地址列表
fn extract_addresses(addr: Option<&mail_parser::Address>) -> Vec<MailAddress> {
    match addr {
        Some(mail_parser::Address::List(list)) => {
            list.iter().map(|a| MailAddress {
                name: a.name.as_ref().map(|s| s.to_string())
                    .unwrap_or_else(|| a.address.as_ref().map(|s| s.to_string()).unwrap_or_default()),
                address: a.address.as_ref().map(|s| s.to_string()).unwrap_or_default(),
            }).collect()
        }
        Some(mail_parser::Address::Group(groups)) => {
            groups.iter().flat_map(|g| {
                g.addresses.iter().map(|a| MailAddress {
                    name: a.name.as_ref().map(|s| s.to_string())
                        .unwrap_or_else(|| a.address.as_ref().map(|s| s.to_string()).unwrap_or_default()),
                    address: a.address.as_ref().map(|s| s.to_string()).unwrap_or_default(),
                })
            }).collect()
        }
        None => vec![],
    }
}

/// 解码 RFC 2047 编码的邮件头字符串（如 =?UTF-8?B?...）
fn decode_header(s: Option<&[u8]>) -> String {
    if let Some(bytes) = s {
        // 构造一个临时的邮件 Subject 邮件头进行解析，借用 mail_parser 强大的解码能力
        let mut raw = b"Subject: ".to_vec();
        raw.extend_from_slice(bytes);
        raw.extend_from_slice(b"\r\n\r\n");
        
        if let Some(m) = mail_parser::MessageParser::default().parse(&raw) {
            // mail_parser 的 subject() 方法会自动解析 RFC 2047 编码
            if let Some(val) = m.subject() {
                return val.to_string();
            }
        }
        // 如果解析失败，回退到普通 UTF-8（如果可能）
        return String::from_utf8_lossy(bytes).to_string();
    }
    String::new()
}

// ========== 邮件连接 ==========

pub struct MailConnection {
    session: imap::Session<TlsStream<TcpStream>>,
    pub config: ImapConfig,
}

impl MailConnection {
    pub fn connect(config: &ImapConfig) -> Result<Self, String> {
        let tls = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| format!("TLS 初始化失败: {}", e))?;

        let addr = (config.host.as_str(), config.port);
        // 核心修复：增加 IO 超时，防止死锁
        let timeout = std::time::Duration::from_secs(10);
        let tcp_stream = std::net::TcpStream::connect_timeout(&addr.to_socket_addrs().unwrap().next().unwrap(), timeout)
            .map_err(|e| format!("TCP 连接超时: {}", e))?;
        tcp_stream.set_read_timeout(Some(timeout)).unwrap();
        tcp_stream.set_write_timeout(Some(timeout)).unwrap();

        let session = if config.secure {
            let tls_stream = tls.connect(&config.host, tcp_stream)
                .map_err(|e| format!("TLS 握手失败: {}", e))?;
            let mut client = imap::Client::new(tls_stream);
            client.login(&config.user, &config.pass)
                .map_err(|(e, _)| format!("IMAP 登录失败: {}", e))?
        } else {
            // STARTTLS 稍微复杂一点，先用普通 client 建立连接并发送 STARTTLS
            let mut client = imap::Client::new(tcp_stream);
            let mut tls_client = client.secure(&config.host, &tls)
                .map_err(|e| format!("STARTTLS 升级失败: {}", e))?;
            tls_client.login(&config.user, &config.pass)
                .map_err(|(e, _)| format!("IMAP 登录失败: {}", e))?
        };

        Ok(MailConnection {
            session,
            config: config.clone(),
        })
    }

    pub fn is_connected(&mut self) -> bool {
        self.session.noop().is_ok()
    }

    pub fn disconnect(&mut self) -> Result<(), String> {
        self.session.logout().map_err(|e| format!("断开失败: {}", e))
    }

    pub fn get_folders(&mut self) -> Result<Vec<MailFolder>, String> {
        let folders = self.session.list(Some(""), Some("*"))
            .map_err(|e| format!("获取文件夹失败: {}", e))?;

        let mut result = Vec::new();
        for folder in folders.iter() {
            let full_path = folder.name().to_string();
            let delim = folder.delimiter().map(|c| c.to_string());

            // 根据名称推断 specialUse
            let lower = full_path.to_lowercase();
            let special_use = if full_path == "INBOX" || lower == "inbox" {
                Some("\\Inbox".to_string())
            } else if lower.contains("sent") {
                Some("\\Sent".to_string())
            } else if lower.contains("draft") {
                Some("\\Drafts".to_string())
            } else if lower.contains("trash") || lower.contains("deleted") || lower.contains("junk") {
                Some("\\Trash".to_string())
            } else {
                // 检查 IMAP 属性
                let mut su = None;
                for attr in folder.attributes() {
                    let attr_str = format!("{:?}", attr);
                    if attr_str.contains("Drafts") { su = Some("\\Drafts".to_string()); }
                    else if attr_str.contains("Sent") { su = Some("\\Sent".to_string()); }
                    else if attr_str.contains("Trash") || attr_str.contains("Junk") { su = Some("\\Trash".to_string()); }
                }
                su
            };

            // 提取 short name（最后一段）
            let short_name = match &delim {
                Some(d) => full_path.rsplit(d.as_str()).next().unwrap_or(&full_path).to_string(),
                None => full_path.clone(),
            };

            result.push(MailFolder {
                name: short_name,
                path: full_path,
                special_use,
                delimiter: delim,
            });
        }
        Ok(result)
    }

    pub fn get_messages(&mut self, folder: &str, page: u32, page_size: u32) -> Result<MailListResult, String> {
        let mailbox = self.session.select(folder)
            .map_err(|e| format!("选择文件夹失败: {}", e))?;

        let total = mailbox.exists;
        if total == 0 {
            return Ok(MailListResult {
                messages: vec![],
                total,
                page,
                page_size,
            });
        }

        let start = std::cmp::max(1, total as i64 - (page * page_size) as i64 + 1) as u32;
        let end = std::cmp::max(1, total as i64 - ((page - 1) * page_size) as i64) as u32;
        let range = format!("{}:{}", start, end);

        let fetches = self.session.fetch(&range, "(FLAGS ENVELOPE UID RFC822.SIZE)")
            .map_err(|e| format!("获取邮件列表失败: {}", e))?;

        let mut messages = Vec::new();
        for fetch in fetches.iter() {
            let envelope = match fetch.envelope() {
                Some(env) => env,
                None => continue,
            };

            let uid = fetch.uid.unwrap_or(0);
            let seq = fetch.message;
            let size = fetch.size.unwrap_or(0);

            let flags: Vec<String> = fetch.flags().iter().map(flag_to_string).collect();
            let seen = fetch.flags().iter().any(|f| matches!(f, imap::types::Flag::Seen));
            let flagged = fetch.flags().iter().any(|f| matches!(f, imap::types::Flag::Flagged));

            let subject = decode_header(envelope.subject.as_deref());
            if subject.is_empty() {
                " (无主题)".to_string();
            }

            let from = envelope.from.as_ref()
                .and_then(|addrs| addrs.first())
                .map(|a| {
                    let name_str = decode_header(a.name.as_deref());
                    let host = a.host.as_ref().and_then(|h| std::str::from_utf8(h).ok()).unwrap_or("");
                    let mbox = a.mailbox.as_ref().and_then(|m| std::str::from_utf8(m).ok()).unwrap_or("");
                    let addr = if host.is_empty() { mbox.to_string() } else { format!("{}@{}", mbox, host) };
                    let display = if name_str.is_empty() { addr.clone() } else { name_str };
                    MailAddress { name: display, address: addr }
                })
                .unwrap_or(MailAddress { name: "未知".to_string(), address: String::new() });

            let to: Vec<MailAddress> = envelope.to.as_ref()
                .map(|addrs| addrs.iter().map(|a| {
                    let name_str = a.name.as_ref().and_then(|n| std::str::from_utf8(n).ok()).unwrap_or("");
                    let host = a.host.as_ref().and_then(|h| std::str::from_utf8(h).ok()).unwrap_or("");
                    let mbox = a.mailbox.as_ref().and_then(|m| std::str::from_utf8(m).ok()).unwrap_or("");
                    let addr = if host.is_empty() { mbox.to_string() } else { format!("{}@{}", mbox, host) };
                    let display = if name_str.is_empty() { addr.clone() } else { name_str.to_string() };
                    MailAddress { name: display, address: addr }
                }).collect())
                .unwrap_or_default();

            let date = envelope.date.as_ref()
                .and_then(|d| std::str::from_utf8(d).ok())
                .map(|d| d.to_string());

            let message_id = envelope.message_id.as_ref()
                .and_then(|m| std::str::from_utf8(m).ok())
                .map(|m| m.to_string());

            messages.push(MailEnvelope {
                uid, seq, flags, size, subject, from, to, date, message_id, seen, flagged,
            });
        }

        messages.sort_by(|a, b| b.date.cmp(&a.date));

        Ok(MailListResult { messages, total, page, page_size })
    }

    pub fn get_message(&mut self, folder: &str, uid: u32) -> Result<MailDetail, String> {
        self.session.select(folder)
            .map_err(|e| format!("选择文件夹失败: {}", e))?;

        let fetches = self.session.uid_fetch(uid.to_string(), "RFC822")
            .map_err(|e| format!("获取邮件详情失败: {}", e))?;

        let fetch = fetches.iter().next()
            .ok_or_else(|| "邮件未找到".to_string())?;

        let body = fetch.body()
            .ok_or_else(|| "邮件内容为空".to_string())?;

        let parsed = mail_parser::MessageParser::default()
            .parse(body)
            .ok_or_else(|| "邮件解析失败".to_string())?;

        // 标记为已读
        let _ = self.session.uid_store(uid.to_string(), "+FLAGS (\\Seen)");

        let subject = parsed.subject().unwrap_or("(无主题)").to_string();

        let from_addrs = extract_addresses(parsed.from());
        let from = from_addrs.into_iter().next()
            .unwrap_or(MailAddress { name: "未知".to_string(), address: String::new() });

        let to = extract_addresses(parsed.to());
        let cc = extract_addresses(parsed.cc());

        let date = parsed.date().map(|d| d.to_rfc3339());

        let mut html = parsed.body_html(0).map(|s| s.to_string());
        let text = parsed.body_text(0).map(|s| s.to_string());

        // 处理 CID 嵌入图片：将 html 中的 cid: 链接替换为 base64 data uri
        if let Some(ref mut html_content) = html {
            use base64::Engine;
            for att in parsed.attachments() {
                if let Some(cid) = att.content_id() {
                    let cid_url = format!("cid:{}", cid);
                    if html_content.contains(&cid_url) {
                        let b64 = base64::engine::general_purpose::STANDARD.encode(att.contents());
                        let mime = att.content_type()
                            .map(|ct| format!("{}/{}", ct.ctype(), ct.subtype().unwrap_or("")))
                            .unwrap_or_else(|| "image/png".to_string());
                        let data_uri = format!("data:{};base64,{}", mime, b64);
                        *html_content = html_content.replace(&cid_url, &data_uri);
                    }
                }
            }
        }

        let mut attachments = Vec::new();
        for att in parsed.attachments() {
            let filename = att.attachment_name()
                .unwrap_or("未命名附件")
                .to_string();
            let content_type = att.content_type()
                .map(|ct| {
                    let t = ct.ctype();
                    let st = ct.subtype().unwrap_or("");
                    if st.is_empty() { t.to_string() } else { format!("{}/{}", t, st) }
                })
                .unwrap_or_else(|| "application/octet-stream".to_string());
            let size = att.contents().len();

            attachments.push(MailAttachment {
                filename, content_type, size, content: None, // 延迟加载：详情解析时不返回 base64 附件内容，避免 IPC 阻塞
            });
        }

        Ok(MailDetail { uid, subject, from, to, cc, date, html, text, attachments })
    }

    pub fn delete_message(&mut self, folder: &str, uid: u32) -> Result<(), String> {
        self.session.select(folder).map_err(|e| format!("选择文件夹失败: {}", e))?;
        self.session.uid_store(uid.to_string(), "+FLAGS (\\Deleted)")
            .map_err(|e| format!("标记删除失败: {}", e))?;
        self.session.expunge().map_err(|e| format!("清除失败: {}", e))?;
        Ok(())
    }

    pub fn move_message(&mut self, folder: &str, uid: u32, destination: &str) -> Result<(), String> {
        self.session.select(folder).map_err(|e| format!("选择文件夹失败: {}", e))?;
        match self.session.uid_mv(uid.to_string(), destination) {
            Ok(_) => Ok(()),
            Err(_) => {
                self.session.uid_copy(uid.to_string(), destination)
                    .map_err(|e| format!("复制邮件失败: {}", e))?;
                self.session.uid_store(uid.to_string(), "+FLAGS (\\Deleted)")
                    .map_err(|e| format!("标记删除失败: {}", e))?;
                self.session.expunge().map_err(|e| format!("清除失败: {}", e))?;
                Ok(())
            }
        }
    }

    pub fn mark_seen(&mut self, folder: &str, uid: u32) -> Result<(), String> {
        self.session.select(folder).map_err(|e| format!("选择文件夹失败: {}", e))?;
        self.session.uid_store(uid.to_string(), "+FLAGS (\\Seen)")
            .map_err(|e| format!("标记已读失败: {}", e))?;
        Ok(())
    }

    pub fn mark_unseen(&mut self, folder: &str, uid: u32) -> Result<(), String> {
        self.session.select(folder).map_err(|e| format!("选择文件夹失败: {}", e))?;
        self.session.uid_store(uid.to_string(), "-FLAGS (\\Seen)")
            .map_err(|e| format!("标记未读失败: {}", e))?;
        Ok(())
    }

    pub fn append_message(&mut self, folder: &str, content: &str, _flags: &[&str]) -> Result<(), String> {
        self.session.append(folder, content.as_bytes())
            .map_err(|e| format!("追加邮件失败: {}", e))?;
        Ok(())
    }

    pub fn get_attachment_bytes(&mut self, folder: &str, uid: u32, index: usize) -> Result<Vec<u8>, String> {
        self.session.select(folder).map_err(|e| format!("选择文件夹失败: {}", e))?;
        let fetches = self.session.uid_fetch(uid.to_string(), "RFC822")
            .map_err(|e| format!("获取邮件失败: {}", e))?;
        
        let fetch = fetches.iter().next().ok_or_else(|| "邮件未找到".to_string())?;
        let body = fetch.body().ok_or_else(|| "内容为空".to_string())?;
        
        let parsed = mail_parser::MessageParser::default().parse(body)
            .ok_or_else(|| "解析失败".to_string())?;
        
        let att = parsed.attachments().nth(index)
            .ok_or_else(|| "附件索引无效".to_string())?;
        
        Ok(att.contents().to_vec())
    }
}

// ========== SMTP 发送 ==========

pub fn send_mail(smtp_config: &SmtpConfig, mail_options: &MailOptions) -> Result<(), String> {
    let from_mailbox: Mailbox = mail_options.from.parse()
        .map_err(|e| format!("发件人地址无效: {}", e))?;

    let mut builder = LettreMessage::builder().from(from_mailbox);

    for addr in mail_options.to.split(',') {
        let addr = addr.trim();
        if !addr.is_empty() {
            let mbox: Mailbox = addr.parse()
                .map_err(|e| format!("收件人地址无效 {}: {}", addr, e))?;
            builder = builder.to(mbox);
        }
    }

    if let Some(ref cc) = mail_options.cc {
        for addr in cc.split(',') {
            let addr = addr.trim();
            if !addr.is_empty() {
                let mbox: Mailbox = addr.parse()
                    .map_err(|e| format!("抄送地址无效 {}: {}", addr, e))?;
                builder = builder.cc(mbox);
            }
        }
    }

    builder = builder.subject(&mail_options.subject);

    let html_body = mail_options.html.as_deref().unwrap_or("");
    let text_body = mail_options.text.as_deref().unwrap_or("");

    let has_attachments = mail_options.attachments.as_ref()
        .map(|a| !a.is_empty()).unwrap_or(false);

    let email = if has_attachments {
        let atts = mail_options.attachments.as_ref().unwrap();
        let body_part = SinglePart::builder()
            .header(ContentType::TEXT_HTML)
            .body(if html_body.is_empty() { text_body.to_string() } else { html_body.to_string() });

        let mut multi = MultiPart::mixed().singlepart(body_part);

        for att in atts {
            let file_content = std::fs::read(&att.path)
                .map_err(|e| format!("读取附件 {} 失败: {}", att.path, e))?;
            let ct_str = mime_guess::from_path(&att.filename)
                .first_or_octet_stream()
                .to_string();
            let content_type = ContentType::parse(&ct_str)
                .unwrap_or(ContentType::parse("application/octet-stream").unwrap());
            let attachment = Attachment::new(att.filename.clone())
                .body(file_content, content_type);
            multi = multi.singlepart(attachment);
        }

        builder.multipart(multi).map_err(|e| format!("构建邮件失败: {}", e))?
    } else if !html_body.is_empty() {
        builder.header(ContentType::TEXT_HTML)
            .body(html_body.to_string())
            .map_err(|e| format!("构建邮件失败: {}", e))?
    } else {
        builder.body(text_body.to_string())
            .map_err(|e| format!("构建邮件失败: {}", e))?
    };

    let creds = Credentials::new(smtp_config.user.clone(), smtp_config.pass.clone());

    let mailer = if smtp_config.secure && smtp_config.port == 465 {
        SmtpTransport::relay(&smtp_config.host)
            .map_err(|e| format!("SMTP 连接失败: {}", e))?
            .port(smtp_config.port)
            .credentials(creds)
            .build()
    } else {
        SmtpTransport::starttls_relay(&smtp_config.host)
            .map_err(|e| format!("SMTP 连接失败: {}", e))?
            .port(smtp_config.port)
            .credentials(creds)
            .build()
    };

    mailer.send(&email).map_err(|e| format!("发送失败: {}", e))?;
    Ok(())
}
