mod mail;

use mail::{MailConnection, ConnectConfig, SmtpConfig, MailOptions, ImapConfig};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use tauri::{
    AppHandle, Emitter, Manager, State,
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_notification::NotificationExt;

// 嵌入静态资源，确保无论路径如何都能正确加载
const MESSAGE_SOUND: &[u8] = include_bytes!("../../public/sounds/message-sound.mp3");
const NOTIF_ICON: &[u8] = include_bytes!("../icons/128x128.png");

// ========== 应用状态 ==========

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct SavedState {
    account_configs: HashMap<String, ImapConfig>,
    smtp_configs: HashMap<String, SmtpConfig>,
    last_max_uids: HashMap<String, u32>,
    polling_interval: u64,
}

pub struct AppState {
    mail_services: Mutex<HashMap<String, Arc<Mutex<MailConnection>>>>,
    account_configs: Mutex<HashMap<String, ImapConfig>>,
    smtp_configs: Mutex<HashMap<String, SmtpConfig>>,
    last_max_uids: Mutex<HashMap<String, u32>>,
    polling_interval: Mutex<u64>,
}

impl AppState {
    fn load_state(app: &AppHandle) -> SavedState {
        let path = app.path().app_data_dir().unwrap_or_default().join("mail-state-tauri.json");
        if let Ok(content) = std::fs::read_to_string(path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            SavedState::default()
        }
    }

    fn save_state(&self, app: &AppHandle) {
        let path = app.path().app_data_dir().unwrap_or_default().join("mail-state-tauri.json");
        let saved = SavedState {
            account_configs: self.account_configs.lock().unwrap().clone(),
            smtp_configs: self.smtp_configs.lock().unwrap().clone(),
            last_max_uids: self.last_max_uids.lock().unwrap().clone(),
            polling_interval: *self.polling_interval.lock().unwrap(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&saved) {
            let _ = std::fs::create_dir_all(path.parent().unwrap());
            let _ = std::fs::write(path, json);
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mail_services: Mutex::new(HashMap::new()),
            account_configs: Mutex::new(HashMap::new()),
            smtp_configs: Mutex::new(HashMap::new()),
            last_max_uids: Mutex::new(HashMap::new()),
            polling_interval: Mutex::new(5),
        }
    }
}

// ========== Tauri 命令 ==========

#[tauri::command]
async fn mail_connect(state: State<'_, AppState>, config: ConnectConfig) -> Result<Value, String> {
    let account_id = config.imap.user.clone();
    match MailConnection::connect(&config.imap) {
        Ok(conn) => {
            let mut services = state.mail_services.lock().unwrap();
            services.insert(account_id.clone(), Arc::new(Mutex::new(conn)));
            let mut configs = state.account_configs.lock().unwrap();
            configs.insert(account_id.clone(), config.imap);
            let mut smtp = state.smtp_configs.lock().unwrap();
            smtp.insert(account_id, config.smtp);
            Ok(json!({ "success": true }))
        }
        Err(e) => Ok(json!({ "success": false, "error": e })),
    }
}

#[tauri::command]
async fn mail_disconnect(state: State<'_, AppState>, account_id: String) -> Result<Value, String> {
    let mut services = state.mail_services.lock().unwrap();
    if let Some(conn_mutex) = services.remove(&account_id) {
        let mut conn = conn_mutex.lock().unwrap();
        let _ = conn.disconnect();
    }
    state.account_configs.lock().unwrap().remove(&account_id);
    state.smtp_configs.lock().unwrap().remove(&account_id);
    state.last_max_uids.lock().unwrap().remove(&account_id);
    Ok(json!({ "success": true }))
}

#[tauri::command]
async fn mail_get_folders(state: State<'_, AppState>, account_id: String) -> Result<Value, String> {
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&account_id).cloned()
    };

    match conn_arc {
        Some(conn_mutex) => {
            let mut conn = conn_mutex.lock().unwrap();
            match conn.get_folders() {
                Ok(folders) => Ok(json!({ "success": true, "data": folders })),
                Err(e) => Ok(json!({ "success": false, "error": e })),
            }
        },
        None => Ok(json!({ "success": false, "error": "账号未连接" })),
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetMessagesParams {
    account_id: String,
    folder: String,
    page: u32,
    page_size: u32,
}

#[tauri::command]
async fn mail_get_messages(state: State<'_, AppState>, params: GetMessagesParams) -> Result<Value, String> {
    log::info!("前端请求邮件列表: {} -> {}", params.account_id, params.folder);
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&params.account_id).cloned()
    };

    match conn_arc {
        Some(conn_mutex) => {
            log::debug!("获取到账号连接锁，正在尝试进入读取...");
            let mut conn = conn_mutex.lock().unwrap();
            log::debug!("已获得 IMAP 会话锁，正在执行 get_messages...");
            match conn.get_messages(&params.folder, params.page, params.page_size) {
                Ok(result) => {
                    log::info!("获取邮件列表成功 (count: {})", result.messages.len());
                    Ok(json!({ "success": true, "data": result }))
                },
                Err(e) => {
                    log::error!("获取邮件列表失败: {}", e);
                    Ok(json!({ "success": false, "error": e }))
                },
            }
        },
        None => {
            log::warn!("账号 {} 未连接，无法获取邮件", params.account_id);
            Ok(json!({ "success": false, "error": "账号未连接" }))
        },
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetMessageParams {
    account_id: String,
    folder: String,
    uid: u32,
}

#[tauri::command]
async fn mail_get_message(state: State<'_, AppState>, params: GetMessageParams) -> Result<Value, String> {
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&params.account_id).cloned()
    };

    match conn_arc {
        Some(conn_mutex) => {
            let mut conn = conn_mutex.lock().unwrap();
            match conn.get_message(&params.folder, params.uid) {
                Ok(detail) => Ok(json!({ "success": true, "data": detail })),
                Err(e) => Ok(json!({ "success": false, "error": e })),
            }
        },
        None => Ok(json!({ "success": false, "error": "账号未连接" })),
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendParams {
    smtp_config: SmtpConfig,
    mail_options: MailOptions,
}

#[tauri::command]
async fn mail_send(params: SendParams) -> Result<Value, String> {
    match mail::send_mail(&params.smtp_config, &params.mail_options) {
        Ok(()) => Ok(json!({ "success": true })),
        Err(e) => Ok(json!({ "success": false, "error": e })),
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteParams {
    account_id: String,
    folder: String,
    uid: u32,
}

#[tauri::command]
async fn mail_delete(state: State<'_, AppState>, params: DeleteParams) -> Result<Value, String> {
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&params.account_id).cloned()
    };
    match conn_arc {
        Some(conn_mutex) => {
            let mut conn = conn_mutex.lock().unwrap();
            match conn.delete_message(&params.folder, params.uid) {
                Ok(()) => Ok(json!({ "success": true })),
                Err(e) => Ok(json!({ "success": false, "error": e })),
            }
        },
        None => Ok(json!({ "success": false, "error": "账号未连接" })),
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveParams {
    account_id: String,
    folder: String,
    uid: u32,
    destination: String,
}

#[tauri::command]
async fn mail_move(state: State<'_, AppState>, params: MoveParams) -> Result<Value, String> {
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&params.account_id).cloned()
    };
    match conn_arc {
        Some(conn_mutex) => {
            let mut conn = conn_mutex.lock().unwrap();
            match conn.move_message(&params.folder, params.uid, &params.destination) {
                Ok(()) => Ok(json!({ "success": true })),
                Err(e) => Ok(json!({ "success": false, "error": e })),
            }
        },
        None => Ok(json!({ "success": false, "error": "账号未连接" })),
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FlagParams {
    account_id: String,
    folder: String,
    uid: u32,
}

#[tauri::command]
async fn mail_mark_seen(state: State<'_, AppState>, params: FlagParams) -> Result<Value, String> {
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&params.account_id).cloned()
    };
    match conn_arc {
        Some(conn_mutex) => {
            let mut conn = conn_mutex.lock().unwrap();
            match conn.mark_seen(&params.folder, params.uid) {
                Ok(()) => Ok(json!({ "success": true })),
                Err(e) => Ok(json!({ "success": false, "error": e })),
            }
        },
        None => Ok(json!({ "success": false, "error": "账号未连接" })),
    }
}

#[tauri::command]
async fn mail_mark_unseen(state: State<'_, AppState>, params: FlagParams) -> Result<Value, String> {
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&params.account_id).cloned()
    };
    match conn_arc {
        Some(conn_mutex) => {
            let mut conn = conn_mutex.lock().unwrap();
            match conn.mark_unseen(&params.folder, params.uid) {
                Ok(()) => Ok(json!({ "success": true })),
                Err(e) => Ok(json!({ "success": false, "error": e })),
            }
        },
        None => Ok(json!({ "success": false, "error": "账号未连接" })),
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveDraftParams {
    account_id: String,
    content: String,
}

#[tauri::command]
async fn mail_save_draft(state: State<'_, AppState>, params: SaveDraftParams) -> Result<Value, String> {
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&params.account_id).cloned()
    };
    match conn_arc {
        Some(conn_mutex) => {
            let mut conn = conn_mutex.lock().unwrap();
            // 先获取文件夹列表找到草稿箱
            let draft_folder = match conn.get_folders() {
                Ok(folders) => {
                    folders.iter()
                        .find(|f| f.special_use.as_deref() == Some("\\Drafts"))
                        .or_else(|| folders.iter().find(|f| f.path.to_lowercase().contains("draft")))
                        .map(|f| f.path.clone())
                        .unwrap_or_else(|| "INBOX".to_string())
                }
                Err(_) => "INBOX".to_string(),
            };
            match conn.append_message(&draft_folder, &params.content, &["\\Draft"]) {
                Ok(()) => Ok(json!({ "success": true })),
                Err(e) => Ok(json!({ "success": false, "error": e })),
            }
        }
        None => Ok(json!({ "success": false, "error": "账号未连接" })),
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadAttachmentParams {
    account_id: String,
    folder: String,
    uid: u32,
    index: usize,
    filename: String,
}

#[tauri::command]
async fn mail_download_attachment(app: AppHandle, state: State<'_, AppState>, params: DownloadAttachmentParams) -> Result<Value, String> {
    use tauri_plugin_dialog::DialogExt;
    
    // 1. 呼起保存对话框
    let file_path = app.dialog().file().set_file_name(&params.filename).blocking_save_file();
    
    let path = match file_path {
        Some(p) => p.into_path().map_err(|e| format!("路径无效: {}", e))?,
        None => return Ok(json!({ "success": true, "canceled": true })),
    };

    // 2. 获取字节流
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&params.account_id).cloned()
    };
    let bytes = match conn_arc {
        Some(conn_mutex) => {
            let mut conn = conn_mutex.lock().unwrap();
            conn.get_attachment_bytes(&params.folder, params.uid, params.index)?
        },
        None => return Err("账号未连接".to_string()),
    };

    // 3. 写入文件
    std::fs::write(&path, bytes).map_err(|e| format!("保存失败: {}", e))?;

    // 4. 通知前端已完成
    let _ = app.emit("mail-notif-message", json!({
        "message": format!("附件已保存至: {}", path.file_name().unwrap_or_default().to_string_lossy()),
        "type": "success"
    }));

    Ok(json!({ "success": true, "canceled": false, "path": path.to_string_lossy() }))
}

#[tauri::command]
async fn mail_update_polling(state: State<'_, AppState>, interval: u64) -> Result<Value, String> {
    let mut pi = state.polling_interval.lock().unwrap();
    *pi = interval;
    Ok(json!({ "success": true }))
}

#[tauri::command]
fn mail_get_sound(_app: AppHandle) -> Option<String> {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(MESSAGE_SOUND);
    Some(format!("data:audio/mpeg;base64,{}", b64))
}

// ========== 测试工具箱 ==========

#[tauri::command]
async fn test_notification(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;
    
    // 将嵌入的图标写入临时文件，以便通知插件读取（针对某些 Linux 桌面环境的要求）
    let temp_icon = app.path().app_cache_dir().unwrap_or_default().join("notif_icon_temp.png");
    let _ = std::fs::create_dir_all(temp_icon.parent().unwrap());
    let _ = std::fs::write(&temp_icon, NOTIF_ICON);

    let mut builder = app.notification().builder();
    builder = builder.title("4yMail 测试通知")
        .body("这是一条来自 Tauri 构建版本的测试消息。");
    
    // 优先使用绝对路径图标
    builder = builder.icon(temp_icon.to_string_lossy().into_owned());
    
    let _ = builder.show();
    let _ = app.emit("mail-play-sound", ());
    Ok(())
}

#[tauri::command]
async fn test_connection_error(app: AppHandle) -> Result<(), String> {
    let _ = app.emit("mail-connection-error", json!({
        "accountId": "test@example.com",
        "error": "这是一条由 Tauri 后端触发的模拟错误"
    }));
    Ok(())
}

fn ensure_notification_icon(app: &AppHandle) -> String {
    let app_cache = app.path().app_cache_dir().unwrap_or_default();
    let temp_icon = app_cache.join("notif_icon_temp.png");
    if !temp_icon.exists() {
        let _ = std::fs::create_dir_all(&app_cache);
        let _ = std::fs::write(&temp_icon, NOTIF_ICON);
    }
    temp_icon.to_string_lossy().into_owned()
}

#[tauri::command]
async fn test_clear_cache(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    state.last_max_uids.lock().unwrap().clear();
    state.save_state(&app);
    let _ = app.emit("mail-notif-message", json!({
        "message": "Tauri 轮询缓存已清空",
        "type": "info"
    }));
    Ok(())
}

#[tauri::command]
async fn test_open_logs(app: AppHandle) -> Result<(), String> {
    let path = app.path().app_data_dir().unwrap_or_default();
    let _ = tauri_plugin_shell::ShellExt::shell(&app)
        .open(path.to_string_lossy(), None);
    Ok(())
}

fn sanitize_notif_text(text: &str, max_len: usize) -> String {
    let s = text.replace('\n', " ").replace('\r', " ").trim().to_string();
    if s.chars().count() > max_len {
        s.chars().take(max_len).collect::<String>() + "..."
    } else {
        s
    }
}

// ========== 新邮件轮询 ==========

fn check_new_emails(app: &AppHandle, state: &AppState, account_id: &str) {
    log::debug!("开始检查账号新邮件: {}", account_id);
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(account_id).cloned()
    };

    let conn_mutex = match conn_arc {
        Some(c) => c,
        None => return,
    };
    let mut conn = conn_mutex.lock().unwrap();
    let messages = match conn.get_messages("INBOX", 1, 5) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("后台轮询连接失效 {}: {}", account_id, e);
            // 核心修复：清理失效连接，强制下次轮询触发重连
            state.mail_services.lock().unwrap().remove(account_id);
            return;
        }
    };

    // 核心优化：获取完数据立即释放锁，防止通知弹窗阻塞前端请求
    drop(conn);
    log::debug!("账号 {} 后台数据读取完毕，已提前释放连接锁", account_id);

    if messages.messages.is_empty() {
        return;
    }

    let current_max_uid = messages.messages.iter().map(|m| m.uid).max().unwrap_or(0);
    let mut last_uids = state.last_max_uids.lock().unwrap();
    let last_uid = *last_uids.get(account_id).unwrap_or(&0);

    if last_uid == 0 {
        last_uids.insert(account_id.to_string(), current_max_uid);
        return;
    }

    if current_max_uid > last_uid {
        let new_count = messages.messages.iter().filter(|m| m.uid > last_uid).count();
        if new_count > 0 {
            let latest = &messages.messages[0];
            let clean_title = sanitize_notif_text(&format!("4yMail - {} ({} 封新邮件)", account_id, new_count), 60);
            let clean_body = sanitize_notif_text(&format!("{}: {}", latest.from.name, latest.subject), 150);

            log::info!("正在请求主线程发送通知: [{}]", clean_title);

            let icon_path = ensure_notification_icon(app);
            
            // 核心修复：转发到主线程执行弹窗，解决 Linux 后台线程通知不出的问题
            let app_clone = app.clone();
            let _ = app.run_on_main_thread(move || {
                let _ = app_clone.notification()
                    .builder()
                    .title(clean_title)
                    .body(clean_body)
                    .icon(icon_path)
                    .show();
            });

            let _ = app.emit("mail-play-sound", ());
            let _ = app.emit("mail-new-arrived", json!({ "accountId": account_id }));
        }
        last_uids.insert(account_id.to_string(), current_max_uid);
        state.save_state(app);
    }
}

/// 核心：确保账号连接在线，如果离线则重连
fn ensure_connection(app: &AppHandle, account_id: &str) -> bool {
    let state = app.state::<AppState>();
    
    // 1. 检查是否已经有活跃连接
    {
        let services = state.mail_services.lock().unwrap();
        if services.contains_key(account_id) {
            return true;
        }
    }

    // 2. 如果没有连接，尝试使用保存的配置重连
    let config = {
        let configs = state.account_configs.lock().unwrap();
        configs.get(account_id).cloned()
    };

    if let Some(imap_config) = config {
        log::info!("正在尝试自动重连账号: {}", account_id);
        match MailConnection::connect(&imap_config) {
            Ok(conn) => {
                log::info!("账号 {} 自动重连成功", account_id);
                state.mail_services.lock().unwrap().insert(account_id.to_string(), Arc::new(Mutex::new(conn)));
                let _ = app.emit("mail-connection-success", json!({ "accountId": account_id }));
                true
            }
            Err(e) => {
                log::error!("自动重连失败 {}: {}", account_id, e);
                // 向前端发送连接错误提醒
                let _ = app.emit("mail-connection-error", json!({
                    "accountId": account_id,
                    "error": format!("自动重连失败: {}", e)
                }));
                false
            }
        }
    } else {
        false
    }
}

// ========== 系统托盘 ==========

fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::PredefinedMenuItem;
    
    let show = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>)?;
    let check = MenuItem::with_id(app, "check", "立即检查邮件", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show, &check, &separator, &quit])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("4yMail")
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "check" => {
                    let _ = app.emit("mail-refresh-all", ());
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

// ========== 启动轮询 ==========

fn start_polling(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // 首次检查延迟，等待应用初始化完成
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        loop {
            let (account_ids, interval) = {
                let state = app.state::<AppState>();
                let configs = state.account_configs.lock().unwrap();
                let ids: Vec<String> = configs.keys().cloned().collect();
                let intv = *state.polling_interval.lock().unwrap();
                (ids, intv)
            };
            
            for id in account_ids {
                let app_task = app.clone();
                tokio::spawn(async move {
                    let id_task = id;
                    // 确保连接在线后再检查
                    if ensure_connection(&app_task, &id_task) {
                        let state = app_task.state::<AppState>();
                        check_new_emails(&app_task, &state, &id_task);
                    }
                });
            }

            tokio::time::sleep(std::time::Duration::from_secs(interval * 60)).await;
        }
    });
}

// ========== 应用入口 ==========

#[tauri::command]
fn start_drag(window: tauri::Window) {
    let _ = window.start_dragging();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(tauri_plugin_log::Builder::default().build())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            mail_connect,
            mail_disconnect,
            mail_get_folders,
            mail_get_messages,
            mail_get_message,
            mail_send,
            mail_delete,
            mail_move,
            mail_mark_seen,
            mail_mark_unseen,
            mail_save_draft,
            mail_update_polling,
            mail_get_sound,
            mail_download_attachment,
            test_notification,
            test_connection_error,
            test_clear_cache,
            test_open_logs,
            start_drag
        ])
        .setup(|app| {
            // 加载持久化状态并恢复
            {
                let saved = AppState::load_state(app.handle());
                let state = app.state::<AppState>();
                *state.account_configs.lock().unwrap() = saved.account_configs;
                *state.smtp_configs.lock().unwrap() = saved.smtp_configs;
                *state.last_max_uids.lock().unwrap() = saved.last_max_uids;
                *state.polling_interval.lock().unwrap() = if saved.polling_interval > 0 { saved.polling_interval } else { 5 };
            }

            setup_tray(app.handle())?;

            let window = app.get_webview_window("main").unwrap();

            // 检测启动参数，如果是开机启动 (--hidden 或 --minimized)，则初始不显示窗口
            let args: Vec<String> = std::env::args().collect();
            let is_hidden = args.iter().any(|arg| arg == "--hidden" || arg == "--minimized");
            if is_hidden {
                let _ = window.hide();
            }

            // 窗口关闭时隐藏而不是退出
            let app_handle_close = app.handle().clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Some(w) = app_handle_close.get_webview_window("main") {
                        let _ = w.hide();
                    }
                }
            });

            // 启动轮询
            start_polling(app.handle().clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
