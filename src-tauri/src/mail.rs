use imap;
use native_tls::{TlsConnector, TlsStream};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{SmtpTransport, Transport, Message as LettreMessage};
use lettre::message::{header::ContentType, MultiPart, SinglePart, Attachment, Mailbox};
use mail_parser::MimeHeaders;

/// 建立 TCP 连接（含 TLS 前握手）的超时
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// 会话期读写超时
const IO_TIMEOUT: Duration = Duration::from_secs(30);

// ========== 数据类型 ==========

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProxyConfig {
    /// "socks5" | "http"
    #[serde(default)]
    pub scheme: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub pass: String,
}

impl ProxyConfig {
    fn usable(&self) -> bool {
        !self.host.trim().is_empty() && self.port != 0
    }

    fn is_socks(&self) -> bool {
        let s = self.scheme.to_ascii_lowercase();
        !s.contains("http")
    }

    fn label(&self) -> String {
        format!("{}:{}（{}）", self.host, self.port, self.scheme)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImapConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    #[serde(default = "default_true")]
    pub secure: bool,
    /// 可选的网络代理（SOCKS5 / HTTP CONNECT）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<ProxyConfig>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<ProxyConfig>,
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
    #[serde(rename = "unreadCount")]
    pub unread_count: u32,
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
    pub seen: bool,
    pub flagged: bool,
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

// ========== 网络层：直连 / SOCKS5 / HTTP CONNECT ==========

/// 连接被重置时的说明。
///
/// 993 / 995 / 465 这类「隐式 TLS」邮件端口在校园网、公司网、部分运营商上会被
/// 中间设备直接 RST（表现：TCP 能连上，一发数据就被重置）。这与账号、密码、
/// 证书都无关，在应用层无法绕过，只能换网络或走代理。
fn reset_hint(host: &str, port: u16) -> String {
    format!(
        "连接被网络重置（{host}:{port}）。\n\
         这类加密邮件端口（993/995/465）常被校园网、公司网或运营商拦截：TCP 能建立，\
         但一到 TLS 握手就被掐断，与账号密码无关。可以尝试：\n\
         ① 换网络（手机热点）验证；\n\
         ② 在代理软件里开启 TUN / 全局模式；\n\
         ③ 在账户设置里填写 SOCKS5 或 HTTP 代理（推荐）。",
        host = host,
        port = port
    )
}

fn is_reset_error(err: &std::io::Error) -> bool {
    err.kind() == std::io::ErrorKind::ConnectionReset
        || err.kind() == std::io::ErrorKind::ConnectionAborted
        || err.raw_os_error() == Some(104)
}

fn describe_io_error(host: &str, port: u16, err: &std::io::Error, during: &str) -> String {
    if is_reset_error(err) {
        reset_hint(host, port)
    } else {
        format!("{}失败（{}:{}）: {}", during, host, port, err)
    }
}

/// 把 IMAP 登录错误翻译成用户能看懂的提示
fn login_error(err: &imap::error::Error) -> String {
    let text = err.to_string();
    let lower = text.to_ascii_lowercase();
    if lower.contains("authenticationfailed") || lower.contains("login failed") {
        return format!(
            "IMAP 登录被拒绝：{}。\n\
             QQ / 163 / 126 等国内邮箱不能用登录密码，需要在网页端开启 IMAP 服务并生成\
             「授权码」（QQ 邮箱：设置 → 账号 → POP3/IMAP/SMTP 服务），把授权码当密码填进来。",
            text
        );
    }
    if lower.contains("disable") || lower.contains("not enabled") {
        return format!(
            "IMAP 登录被拒绝：{}。\n该邮箱可能未开启 IMAP 服务，请先到网页版设置里开启（QQ 邮箱：设置 → 账号 → POP3/IMAP/SMTP 服务）。",
            text
        );
    }
    format!("IMAP 登录失败: {}", text)
}

fn connect_direct(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, String> {
    let addrs = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("解析服务器地址 {} 失败: {}", host, e))?;
    let mut last_err: Option<std::io::Error> = None;
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(stream) => return Ok(stream),
            Err(e) => last_err = Some(e),
        }
    }
    match last_err {
        Some(e) => Err(describe_io_error(host, port, &e, "TCP 连接")),
        None => Err(format!("服务器 {}:{} 没有可用地址", host, port)),
    }
}

fn socks5_handshake(
    stream: &mut TcpStream,
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
) -> Result<(), String> {
    let proxy_label = "代理";
    let use_auth = !user.is_empty();
    let methods: Vec<u8> = if use_auth { vec![0x00, 0x02] } else { vec![0x00] };

    let mut greeting = vec![0x05u8, methods.len() as u8];
    greeting.extend_from_slice(&methods);
    stream
        .write_all(&greeting)
        .map_err(|e| describe_io_error(proxy_label, 0, &e, "SOCKS5 问候"))?;

    let mut resp = [0u8; 2];
    stream
        .read_exact(&mut resp)
        .map_err(|e| describe_io_error(proxy_label, 0, &e, "SOCKS5 问候应答"))?;
    if resp[0] != 0x05 {
        return Err("代理没有返回 SOCKS5 协议，请检查代理类型与端口是否正确".to_string());
    }

    match resp[1] {
        0x00 => {}
        0x02 => {
            let ub = user.as_bytes();
            let pb = pass.as_bytes();
            if ub.len() > 255 || pb.len() > 255 {
                return Err("代理用户名或密码过长".to_string());
            }
            let mut auth = vec![0x01u8, ub.len() as u8];
            auth.extend_from_slice(ub);
            auth.push(pb.len() as u8);
            auth.extend_from_slice(pb);
            stream
                .write_all(&auth)
                .map_err(|e| describe_io_error(proxy_label, 0, &e, "SOCKS5 认证"))?;
            let mut ar = [0u8; 2];
            stream
                .read_exact(&mut ar)
                .map_err(|e| describe_io_error(proxy_label, 0, &e, "SOCKS5 认证应答"))?;
            if ar[1] != 0x00 {
                return Err("SOCKS5 认证失败（用户名或密码错误）".to_string());
            }
        }
        0xff => return Err("代理拒绝了所有认证方式".to_string()),
        m => return Err(format!("代理要求不支持的认证方式: 0x{:02x}", m)),
    }

    let host_bytes = host.as_bytes();
    if host_bytes.len() > 255 {
        return Err("服务器域名过长".to_string());
    }
    let mut request = vec![0x05u8, 0x01, 0x00, 0x03, host_bytes.len() as u8];
    request.extend_from_slice(host_bytes);
    request.extend_from_slice(&port.to_be_bytes());
    stream
        .write_all(&request)
        .map_err(|e| describe_io_error(proxy_label, 0, &e, "SOCKS5 CONNECT"))?;

    let mut head = [0u8; 4];
    stream
        .read_exact(&mut head)
        .map_err(|e| describe_io_error(proxy_label, 0, &e, "SOCKS5 CONNECT 应答"))?;
    if head[1] != 0x00 {
        let reason = match head[1] {
            0x01 => "代理内部错误",
            0x02 => "规则不允许该连接",
            0x03 => "网络不可达",
            0x04 => "主机不可达",
            0x05 => "目标拒绝连接",
            0x06 => "TTL 过期",
            0x07 => "不支持的命令",
            0x08 => "不支持的地址类型",
            _ => "未知错误",
        };
        return Err(format!(
            "代理拒绝连接 {}:{}（{}，code {}）",
            host, port, reason, head[1]
        ));
    }

    // 跳过代理返回的绑定地址
    let skip = match head[3] {
        0x01 => 4,
        0x04 => 16,
        0x03 => {
            let mut len = [0u8; 1];
            stream
                .read_exact(&mut len)
                .map_err(|e| describe_io_error(proxy_label, 0, &e, "SOCKS5 应答"))?;
            len[0] as usize
        }
        _ => 0,
    };
    let mut drain = vec![0u8; skip + 2];
    stream
        .read_exact(&mut drain)
        .map_err(|e| describe_io_error(proxy_label, 0, &e, "SOCKS5 应答"))?;
    Ok(())
}

fn http_connect_handshake(
    stream: &mut TcpStream,
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
) -> Result<(), String> {
    let proxy_label = "代理";
    let mut request = format!(
        "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\nProxy-Connection: keep-alive\r\n",
        host, port, host, port
    );
    if !user.is_empty() {
        use base64::Engine;
        let token = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", user, pass));
        request.push_str(&format!("Proxy-Authorization: Basic {}\r\n", token));
    }
    request.push_str("\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|e| describe_io_error(proxy_label, 0, &e, "HTTP CONNECT"))?;

    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match stream.read(&mut byte) {
            Ok(0) => return Err("代理在 HTTP CONNECT 阶段关闭了连接".to_string()),
            Ok(_) => {
                buf.push(byte[0]);
                if buf.len() >= 4 && &buf[buf.len() - 4..] == b"\r\n\r\n" {
                    break;
                }
                if buf.len() > 16 * 1024 {
                    return Err("代理返回的响应头过长".to_string());
                }
            }
            Err(e) => return Err(describe_io_error(proxy_label, 0, &e, "HTTP CONNECT 应答")),
        }
    }

    let head = String::from_utf8_lossy(&buf).to_string();
    let status_line = head.lines().next().unwrap_or_default().to_string();
    let code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse::<u16>().ok());
    match code {
        Some(200) => Ok(()),
        Some(407) => Err("代理需要认证（407），请填写用户名和密码".to_string()),
        Some(c) => Err(format!("代理拒绝连接 {}:{}（HTTP {}）", host, port, c)),
        None => Err(format!("代理返回了无法解析的响应: {}", status_line)),
    }
}

/// 建立到 `host:port` 的 TCP 连接；配置了代理则先穿过代理。
fn open_tcp(
    proxy: &Option<ProxyConfig>,
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<TcpStream, String> {
    let proxy = match proxy.as_ref().filter(|p| p.usable()) {
        Some(p) => p,
        None => return connect_direct(host, port, timeout),
    };

    let mut stream = connect_direct(&proxy.host, proxy.port, timeout)
        .map_err(|e| format!("连接代理 {} 失败: {}", proxy.label(), e))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| format!("设置代理超时失败: {}", e))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|e| format!("设置代理超时失败: {}", e))?;

    if proxy.is_socks() {
        socks5_handshake(&mut stream, host, port, &proxy.user, &proxy.pass)?;
    } else {
        http_connect_handshake(&mut stream, host, port, &proxy.user, &proxy.pass)?;
    }
    Ok(stream)
}

/// 给 lettre 用的本地隧道：lettre 只能自己建连，所以我们在本地开一个
/// 127.0.0.1:随机端口，把连接经代理转发到真实的 SMTP 服务器。
fn spawn_tunnel(proxy: ProxyConfig, host: String, port: u16) -> Result<u16, String> {
    let listener =
        TcpListener::bind("127.0.0.1:0").map_err(|e| format!("启动本地代理隧道失败: {}", e))?;
    let local_port = listener
        .local_addr()
        .map_err(|e| format!("读取本地隧道端口失败: {}", e))?
        .port();

    std::thread::spawn(move || {
        for incoming in listener.incoming() {
            let Ok(client) = incoming else { continue };
            let host = host.clone();
            let proxy = proxy.clone();
            std::thread::spawn(move || {
                let upstream = match open_tcp(&Some(proxy), &host, port, CONNECT_TIMEOUT) {
                    Ok(s) => s,
                    Err(_) => return,
                };
                let (Ok(mut up_read), Ok(mut cl_read)) =
                    (upstream.try_clone(), client.try_clone())
                else {
                    return;
                };
                let mut up_write = upstream;
                let mut cl_write = client;
                let pump = std::thread::spawn(move || {
                    let _ = std::io::copy(&mut up_read, &mut cl_write);
                });
                let _ = std::io::copy(&mut cl_read, &mut up_write);
                let _ = pump.join();
            });
        }
    });

    Ok(local_port)
}

// ========== 邮件连接 ==========

#[derive(Debug)]
pub struct MailConnection {
    session: imap::Session<TlsStream<TcpStream>>,
    #[allow(dead_code)]
    pub config: ImapConfig,
}

#[derive(Debug, PartialEq)]
enum ConnectStage {
    Tcp,
    Tls,
    Login,
}

struct ConnectAttempt {
    port: u16,
    implicit_tls: bool,
}

impl MailConnection {
    pub fn connect(config: &ImapConfig) -> Result<Self, String> {
        let tls = TlsConnector::builder()
            .build()
            .map_err(|e| format!("TLS 初始化失败: {}", e))?;

        let mut attempts = vec![ConnectAttempt {
            port: config.port,
            implicit_tls: config.secure,
        }];
        // 993 常被网络拦截，而 143 通常放行：失败时回退到 143 + STARTTLS
        // （QQ 不做 STARTTLS，但 163、Gmail、自建服务器等大多支持）。
        if config.secure && config.port == 993 {
            attempts.push(ConnectAttempt {
                port: 143,
                implicit_tls: false,
            });
        }

        let mut errors: Vec<String> = Vec::new();
        for attempt in &attempts {
            match Self::try_connect(config, &tls, attempt) {
                Ok(conn) => return Ok(conn),
                Err((stage, message)) => {
                    let multi = attempts.len() > 1;
                    errors.push(if multi {
                        format!("端口 {}：{}", attempt.port, message)
                    } else {
                        message
                    });
                    // 账号密码错误没必要换端口重试
                    if stage == ConnectStage::Login {
                        break;
                    }
                }
            }
        }

        Err(errors.join("\n"))
    }

    fn try_connect(
        config: &ImapConfig,
        tls: &TlsConnector,
        attempt: &ConnectAttempt,
    ) -> Result<Self, (ConnectStage, String)> {
        let stream = open_tcp(&config.proxy, &config.host, attempt.port, CONNECT_TIMEOUT)
            .map_err(|e| (ConnectStage::Tcp, e))?;
        stream
            .set_read_timeout(Some(IO_TIMEOUT))
            .map_err(|e| (ConnectStage::Tcp, format!("设置读取超时失败: {}", e)))?;
        stream
            .set_write_timeout(Some(IO_TIMEOUT))
            .map_err(|e| (ConnectStage::Tcp, format!("设置写入超时失败: {}", e)))?;

        let session = if attempt.implicit_tls {
            let tls_stream = tls.connect(&config.host, stream).map_err(|e| {
                let text = e.to_string();
                let wrapped = if text.to_ascii_lowercase().contains("reset") {
                    reset_hint(&config.host, attempt.port)
                } else {
                    format!("TLS 握手失败（{}:{}）: {}", config.host, attempt.port, text)
                };
                (ConnectStage::Tls, wrapped)
            })?;
            let client = imap::Client::new(tls_stream);
            client
                .login(&config.user, &config.pass)
                .map_err(|(e, _)| (ConnectStage::Login, login_error(&e)))?
        } else {
            let client = imap::Client::new(stream);
            let tls_client = client.secure(&config.host, tls).map_err(|e| {
                (
                    ConnectStage::Tls,
                    format!(
                        "STARTTLS 升级失败（{}:{}）: {}（该服务器可能不支持 143 端口加密）",
                        config.host, attempt.port, e
                    ),
                )
            })?;
            tls_client
                .login(&config.user, &config.pass)
                .map_err(|(e, _)| (ConnectStage::Login, login_error(&e)))?
        };

        Ok(MailConnection {
            session,
            config: config.clone(),
        })
    }

    pub fn is_connected(&mut self) -> bool {
        // 给 NOOP 设置更短的超时，避免长时间阻塞
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

    pub fn get_messages(
        &mut self,
        folder: &str,
        page: u32,
        page_size: u32,
        query: Option<&str>,
        unread_only: bool,
        flagged_only: bool,
    ) -> Result<MailListResult, String> {
        let mailbox = self.session.select(folder)
            .map_err(|e| format!("选择文件夹失败: {}", e))?;

        let _folder_total = mailbox.exists;
        let unread_count = self.session.search("UNSEEN")
            .map_err(|e| format!("获取未读数量失败: {}", e))?
            .len() as u32;
        let page = page.max(1);
        let page_size = page_size.clamp(1, 200);
        let query = query.map(str::trim).filter(|query| !query.is_empty());
        let mut criteria = Vec::new();
        let mut search_charset = None;
        if unread_only {
            criteria.push("UNSEEN".to_string());
        }
        if flagged_only {
            criteria.push("FLAGGED".to_string());
        }
        if let Some(query) = query {
            let query: String = query.chars().filter(|ch| !ch.is_control()).take(200).collect();
            let escaped = query.replace('\\', "\\\\").replace('"', "\\\"");
            if !escaped.trim().is_empty() {
                if !query.is_ascii() {
                    search_charset = Some("CHARSET UTF-8 ");
                }
                criteria.push(format!("OR SUBJECT \"{}\" OR FROM \"{}\" TO \"{}\"", escaped, escaped, escaped));
            }
        }
        let criteria = if criteria.is_empty() {
            "ALL".to_string()
        } else {
            format!("{}{}", search_charset.unwrap_or(""), criteria.join(" "))
        };
        let mut uids: Vec<u32> = self.session.uid_search(&criteria)
            .map_err(|e| format!("搜索邮件失败: {}", e))?
            .into_iter()
            .collect();
        uids.sort_unstable_by(|a, b| b.cmp(a));
        let matching_total = uids.len() as u32;
        let offset = u64::from(page - 1) * u64::from(page_size);
        if offset >= u64::from(matching_total) {
            return Ok(MailListResult { messages: vec![], total: matching_total, unread_count, page, page_size });
        }
        let page_uids = uids.iter()
            .skip(offset as usize)
            .take(page_size as usize)
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");

        let fetches = self.session.uid_fetch(&page_uids, "(FLAGS ENVELOPE UID RFC822.SIZE)")
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

            let subject = match decode_header(envelope.subject.as_deref()) {
                value if value.trim().is_empty() => "(无主题)".to_string(),
                value => value,
            };

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

        // IMAP sequence ranges are fetched newest-first by position; sort by UID to
        // keep ordering stable even when server date headers use differing formats.
        messages.sort_by(|a, b| b.uid.cmp(&a.uid));

        Ok(MailListResult { messages, total: matching_total, unread_count, page, page_size })
    }

    pub fn get_message(&mut self, folder: &str, uid: u32) -> Result<MailDetail, String> {
        self.session.select(folder)
            .map_err(|e| format!("选择文件夹失败: {}", e))?;

        let fetches = self.session.uid_fetch(uid.to_string(), "(FLAGS RFC822)")
            .map_err(|e| format!("获取邮件详情失败: {}", e))?;

        let fetch = fetches.iter().next()
            .ok_or_else(|| "邮件未找到".to_string())?;

        let flagged = fetch.flags().iter().any(|f| matches!(f, imap::types::Flag::Flagged));

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

        Ok(MailDetail { uid, subject, from, to, cc, date, html, text, attachments, seen: true, flagged })
    }

    pub fn delete_messages(&mut self, folder: &str, uids: &[u32]) -> Result<(), String> {
        if uids.is_empty() { return Ok(()); }
        self.session.select(folder).map_err(|e| format!("选择文件夹失败: {}", e))?;
        let uids_str = uids.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");
        self.session.uid_store(&uids_str, "+FLAGS (\\Deleted)")
            .map_err(|e| format!("标记删除失败: {}", e))?;
        self.session.expunge().map_err(|e| format!("清除失败: {}", e))?;
        Ok(())
    }

    pub fn delete_message(&mut self, folder: &str, uid: u32) -> Result<(), String> {
        self.delete_messages(folder, &[uid])
    }

    pub fn move_messages(&mut self, folder: &str, uids: &[u32], destination: &str) -> Result<(), String> {
        if uids.is_empty() { return Ok(()); }
        self.session.select(folder).map_err(|e| format!("选择文件夹失败: {}", e))?;
        let uids_str = uids.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");
        match self.session.uid_mv(&uids_str, destination) {
            Ok(_) => Ok(()),
            Err(_) => {
                self.session.uid_copy(&uids_str, destination)
                    .map_err(|e| format!("复制邮件失败: {}", e))?;
                self.session.uid_store(&uids_str, "+FLAGS (\\Deleted)")
                    .map_err(|e| format!("标记删除失败: {}", e))?;
                self.session.expunge().map_err(|e| format!("清除失败: {}", e))?;
                Ok(())
            }
        }
    }

    pub fn move_message(&mut self, folder: &str, uid: u32, destination: &str) -> Result<(), String> {
        self.move_messages(folder, &[uid], destination)
    }

    pub fn mark_messages_seen(&mut self, folder: &str, uids: &[u32]) -> Result<(), String> {
        if uids.is_empty() { return Ok(()); }
        self.session.select(folder).map_err(|e| format!("选择文件夹失败: {}", e))?;
        let uids_str = uids.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");
        self.session.uid_store(&uids_str, "+FLAGS (\\Seen)")
            .map_err(|e| format!("标记已读失败: {}", e))?;
        Ok(())
    }

    pub fn mark_seen(&mut self, folder: &str, uid: u32) -> Result<(), String> {
        self.mark_messages_seen(folder, &[uid])
    }

    pub fn mark_messages_unseen(&mut self, folder: &str, uids: &[u32]) -> Result<(), String> {
        if uids.is_empty() { return Ok(()); }
        self.session.select(folder).map_err(|e| format!("选择文件夹失败: {}", e))?;
        let uids_str = uids.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");
        self.session.uid_store(&uids_str, "-FLAGS (\\Seen)")
            .map_err(|e| format!("标记未读失败: {}", e))?;
        Ok(())
    }

    pub fn mark_unseen(&mut self, folder: &str, uid: u32) -> Result<(), String> {
        self.mark_messages_unseen(folder, &[uid])
    }

    pub fn mark_messages_flagged(&mut self, folder: &str, uids: &[u32]) -> Result<(), String> {
        if uids.is_empty() { return Ok(()); }
        self.session.select(folder).map_err(|e| format!("选择文件夹失败: {}", e))?;
        let uids_str = uids.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");
        self.session.uid_store(&uids_str, "+FLAGS (\\Flagged)")
            .map_err(|e| format!("标记星标失败: {}", e))?;
        Ok(())
    }

    pub fn mark_messages_unflagged(&mut self, folder: &str, uids: &[u32]) -> Result<(), String> {
        if uids.is_empty() { return Ok(()); }
        self.session.select(folder).map_err(|e| format!("选择文件夹失败: {}", e))?;
        let uids_str = uids.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");
        self.session.uid_store(&uids_str, "-FLAGS (\\Flagged)")
            .map_err(|e| format!("取消星标失败: {}", e))?;
        Ok(())
    }

    pub fn append_message(&mut self, folder: &str, content: &str, flags: &[&str]) -> Result<(), String> {
        let parsed_flags: Vec<imap::types::Flag<'_>> = flags.iter().map(|flag| match *flag {
            "\\Draft" => imap::types::Flag::Draft,
            "\\Seen" => imap::types::Flag::Seen,
            "\\Flagged" => imap::types::Flag::Flagged,
            _ => imap::types::Flag::Custom(std::borrow::Cow::Borrowed(*flag)),
        }).collect();
        self.session.append_with_flags(folder, content.as_bytes(), &parsed_flags)
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

    // 主端口优先，失败后在 465（隐式 TLS）与 587（STARTTLS）之间回退。
    // 465 在很多网络里被直接 RST，而 587 通常放行。
    let primary_implicit = smtp_config.secure && smtp_config.port == 465;
    let mut attempts: Vec<(u16, bool)> = vec![(smtp_config.port, primary_implicit)];
    let fallback_port = if smtp_config.port == 465 { 587 } else { 465 };
    attempts.push((fallback_port, fallback_port == 465));

    let mut errors: Vec<String> = Vec::new();
    for (port, implicit_tls) in attempts {
        match build_smtp_transport(smtp_config, &creds, port, implicit_tls) {
            Ok(mailer) => match mailer.send(&email) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    let text = e.to_string();
                    let hint = if text.to_ascii_lowercase().contains("reset") {
                        reset_hint(&smtp_config.host, port)
                    } else {
                        format!("发送失败（{}:{}）: {}", smtp_config.host, port, text)
                    };
                    errors.push(format!("端口 {}：{}", port, hint));
                    // 认证错误、收件人被拒等业务错误不必换端口重试
                    if !is_transport_error(&text) {
                        break;
                    }
                }
            },
            Err(e) => {
                errors.push(format!("端口 {}：{}", port, e));
            }
        }
    }

    Err(errors.join("\n"))
}

/// 粗粒度判断是不是传输层（网络/TLS）错误，决定要不要换端口重试
fn is_transport_error(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "reset",
        "broken pipe",
        "timed out",
        "timeout",
        "connection refused",
        "network is unreachable",
        "tls",
        "handshake",
        "unexpected eof",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

/// 构建 SMTP 传输：可选走代理（本地隧道）+ 显式指定 TLS 模式
fn build_smtp_transport(
    smtp_config: &SmtpConfig,
    creds: &Credentials,
    port: u16,
    implicit_tls: bool,
) -> Result<SmtpTransport, String> {
    let tls_params = TlsParameters::builder(smtp_config.host.clone())
        .build()
        .map_err(|e| format!("TLS 初始化失败: {}", e))?;

    let (server, connect_port) = match smtp_config.proxy.as_ref().filter(|p| p.usable()) {
        Some(proxy) => {
            let local_port = spawn_tunnel(proxy.clone(), smtp_config.host.clone(), port)?;
            ("127.0.0.1".to_string(), local_port)
        }
        None => (smtp_config.host.clone(), port),
    };

    let builder = SmtpTransport::builder_dangerous(server)
        .port(connect_port)
        .credentials(creds.clone())
        .timeout(Some(Duration::from_secs(30)));

    let builder = if implicit_tls {
        builder.tls(Tls::Wrapper(tls_params))
    } else {
        builder.tls(Tls::Required(tls_params))
    };

    Ok(builder.build())
}

#[cfg(test)]
mod proxy_tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;

    /// 一个最小 SOCKS5 服务器：不做认证，接受 CONNECT 后回 "PONG"
    fn spawn_mock_socks5(user: Option<(&'static str, &'static str)>) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut head = [0u8; 2];
            stream.read_exact(&mut head).unwrap();
            let mut methods = vec![0u8; head[1] as usize];
            stream.read_exact(&mut methods).unwrap();
            match user {
                None => stream.write_all(&[0x05, 0x00]).unwrap(),
                Some(_) => {
                    stream.write_all(&[0x05, 0x02]).unwrap();
                    let mut auth_head = [0u8; 2];
                    stream.read_exact(&mut auth_head).unwrap();
                    let mut u = vec![0u8; auth_head[1] as usize];
                    stream.read_exact(&mut u).unwrap();
                    let mut plen = [0u8; 1];
                    stream.read_exact(&mut plen).unwrap();
                    let mut p = vec![0u8; plen[0] as usize];
                    stream.read_exact(&mut p).unwrap();
                    stream.write_all(&[0x05, 0x00]).unwrap();
                }
            }
            // CONNECT 请求
            let mut req_head = [0u8; 4];
            stream.read_exact(&mut req_head).unwrap();
            assert_eq!(req_head[3], 0x03, "只测域名形式的 CONNECT");
            let mut host_len = [0u8; 1];
            stream.read_exact(&mut host_len).unwrap();
            let mut host = vec![0u8; host_len[0] as usize];
            stream.read_exact(&mut host).unwrap();
            let mut tail = [0u8; 2];
            stream.read_exact(&mut tail).unwrap();
            // 成功应答：BND.ADDR = 127.0.0.1:0
            stream
                .write_all(&[0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 0])
                .unwrap();
            stream.write_all(b"PONG").unwrap();
        });
        port
    }

    #[test]
    fn socks5_no_auth() {
        let port = spawn_mock_socks5(None);
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        socks5_handshake(&mut stream, "imap.qq.com", 993, "", "").unwrap();
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"PONG");
    }

    #[test]
    fn socks5_with_password() {
        let port = spawn_mock_socks5(Some(("user", "pass")));
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        socks5_handshake(&mut stream, "imap.qq.com", 993, "user", "pass").unwrap();
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"PONG");
    }

    fn spawn_mock_http_proxy() -> (u16, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
                request.push_str(&line);
            }
            stream
                .write_all(b"HTTP/1.1 200 Connection established\r\n\r\nPONG")
                .unwrap();
            request
        });
        (port, handle)
    }

    #[test]
    fn http_connect_with_basic_auth() {
        let (port, handle) = spawn_mock_http_proxy();
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        http_connect_handshake(&mut stream, "imap.qq.com", 993, "user", "pass").unwrap();
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"PONG");
        let request = handle.join().unwrap();
        assert!(request.starts_with("CONNECT imap.qq.com:993 HTTP/1.1"));
        assert!(request.contains("Proxy-Authorization: Basic dXNlcjpwYXNz"));
    }

    #[test]
    fn http_connect_rejected() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            let _ = stream.write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n");
        });
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let err = http_connect_handshake(&mut stream, "imap.qq.com", 993, "", "").unwrap_err();
        assert!(err.contains("407"), "unexpected: {}", err);
    }

    #[test]
    fn reset_error_says_network_blocked() {
        let err = std::io::Error::from_raw_os_error(104);
        let msg = describe_io_error("imap.qq.com", 993, &err, "TCP 连接");
        assert!(msg.contains("连接被网络重置"));
        assert!(msg.contains("993"));
    }
}
