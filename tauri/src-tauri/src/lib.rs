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
#[cfg(not(target_os = "linux"))]
use tauri_plugin_notification::NotificationExt;

// 嵌入静态资源，确保无论路径如何都能正确加载
const MESSAGE_SOUND: &[u8] = include_bytes!("../../public/sounds/message-sound.mp3");
#[cfg(not(target_os = "linux"))]
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
async fn mail_connect(app: AppHandle, state: State<'_, AppState>, config: ConnectConfig) -> Result<Value, String> {
    let account_id = config.imap.user.clone();
    
    log::info!("开始连接账号: {}", account_id);

    // 彻底清理旧连接（如果存在）
    {
        let mut services = state.mail_services.lock().unwrap();
        if let Some(old_conn) = services.remove(&account_id) {
            // 尝试安全断开旧连接，捕获所有可能的 panic
            let _ = std::panic::catch_unwind(|| {
                // 尝试获取锁并断开
                if let Ok(mut conn) = old_conn.lock() {
                    let _ = conn.disconnect();
                }
            });
            log::info!("已清理账号 {} 的旧连接", account_id);
        }
    }
    
    // 发送正在连接的通知
    let _ = app.emit("mail-connecting", json!({ "accountId": account_id.clone() }));

    // 使用 spawn_blocking 避免阻塞 Tauri 异步运行时
    let imap_config = config.imap.clone();
    let connect_result = tauri::async_runtime::spawn_blocking(move || {
        MailConnection::connect(&imap_config)
    }).await;

    match connect_result {
        Ok(Ok(conn)) => {
            log::info!("账号 {} 连接成功", account_id);
            // 保存连接和配置
            {
                let mut services = state.mail_services.lock().unwrap();
                services.insert(account_id.clone(), Arc::new(Mutex::new(conn)));
            }
            {
                let mut configs = state.account_configs.lock().unwrap();
                configs.insert(account_id.clone(), config.imap);
            }
            {
                let mut smtp = state.smtp_configs.lock().unwrap();
                smtp.insert(account_id.clone(), config.smtp);
            }
            
            let _ = app.emit("mail-connection-success", json!({ "accountId": account_id.clone() }));
            Ok(json!({ "success": true }))
        }
        Ok(Err(e)) => {
            log::error!("账号 {} 连接失败: {}", account_id, e);
            let _ = app.emit("mail-connection-error", json!({ 
                "accountId": account_id.clone(), 
                "error": e.clone() 
            }));
            Ok(json!({ "success": false, "error": e }))
        }
        Err(e) => {
            log::error!("账号 {} 连接任务执行失败: {}", account_id, e);
            let error_msg = format!("连接任务执行失败: {}", e);
            let _ = app.emit("mail-connection-error", json!({ 
                "accountId": account_id.clone(), 
                "error": error_msg.clone() 
            }));
            Ok(json!({ "success": false, "error": error_msg }))
        }
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
async fn mail_get_folders(app: AppHandle, state: State<'_, AppState>, account_id: String) -> Result<Value, String> {
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&account_id).cloned()
    };

    match conn_arc {
        Some(conn_mutex) => {
            let mut conn = conn_mutex.lock().unwrap();
            match conn.get_folders() {
                Ok(folders) => Ok(json!({ "success": true, "data": folders })),
                Err(e) => {
                    log::error!("获取文件夹失败 [{}]: {}", account_id, e);
                    drop(conn);
                    state.mail_services.lock().unwrap().remove(&account_id);
                    let _ = app.emit("mail-connection-error", json!({
                        "accountId": account_id,
                        "error": format!("连接已断开: {}", e)
                    }));
                    Ok(json!({ "success": false, "error": format!("连接已断开: {}", e) }))
                },
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
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    unread_only: bool,
    #[serde(default)]
    flagged_only: bool,
}

#[tauri::command]
async fn mail_get_messages(app: AppHandle, state: State<'_, AppState>, params: GetMessagesParams) -> Result<Value, String> {
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
            match conn.get_messages(
                &params.folder,
                params.page,
                params.page_size,
                params.query.as_deref(),
                params.unread_only,
                params.flagged_only,
            ) {
                Ok(result) => {
                    log::info!("获取邮件列表成功 (count: {})", result.messages.len());
                    Ok(json!({ "success": true, "data": result }))
                },
                Err(e) => {
                    log::error!("获取邮件列表失败，正在清理坏连接 ({}): {}", params.account_id, e);
                    drop(conn);
                    state.mail_services.lock().unwrap().remove(&params.account_id);
                    let _ = app.emit("mail-connection-error", json!({
                        "accountId": params.account_id,
                        "error": format!("连接已断开: {}", e)
                    }));
                    Ok(json!({ "success": false, "error": format!("连接已断开: {}", e) }))
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
async fn mail_get_message(app: AppHandle, state: State<'_, AppState>, params: GetMessageParams) -> Result<Value, String> {
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&params.account_id).cloned()
    };

    match conn_arc {
        Some(conn_mutex) => {
            let mut conn = conn_mutex.lock().unwrap();
            match conn.get_message(&params.folder, params.uid) {
                Ok(detail) => Ok(json!({ "success": true, "data": detail })),
                Err(e) => {
                    log::error!("获取邮件详情失败 [{}]: {}", params.account_id, e);
                    drop(conn);
                    state.mail_services.lock().unwrap().remove(&params.account_id);
                    let _ = app.emit("mail-connection-error", json!({
                        "accountId": params.account_id,
                        "error": format!("连接已断开: {}", e)
                    }));
                    Ok(json!({ "success": false, "error": format!("连接已断开: {}", e) }))
                },
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
struct BatchActionParams {
    account_id: String,
    folder: String,
    uids: Vec<u32>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchMoveParams {
    account_id: String,
    folder: String,
    uids: Vec<u32>,
    destination: String,
}

#[tauri::command]
async fn mail_batch_mark_seen(state: State<'_, AppState>, params: BatchActionParams) -> Result<Value, String> {
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&params.account_id).cloned()
    };
    match conn_arc {
        Some(conn_mutex) => {
            let mut conn = conn_mutex.lock().unwrap();
            match conn.mark_messages_seen(&params.folder, &params.uids) {
                Ok(()) => Ok(json!({ "success": true })),
                Err(e) => Ok(json!({ "success": false, "error": e })),
            }
        },
        None => Ok(json!({ "success": false, "error": "账号未连接" })),
    }
}

#[tauri::command]
async fn mail_batch_mark_unseen(state: State<'_, AppState>, params: BatchActionParams) -> Result<Value, String> {
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&params.account_id).cloned()
    };
    match conn_arc {
        Some(conn_mutex) => {
            let mut conn = conn_mutex.lock().unwrap();
            match conn.mark_messages_unseen(&params.folder, &params.uids) {
                Ok(()) => Ok(json!({ "success": true })),
                Err(e) => Ok(json!({ "success": false, "error": e })),
            }
        },
        None => Ok(json!({ "success": false, "error": "账号未连接" })),
    }
}

#[tauri::command]
async fn mail_batch_delete(state: State<'_, AppState>, params: BatchActionParams) -> Result<Value, String> {
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&params.account_id).cloned()
    };
    match conn_arc {
        Some(conn_mutex) => {
            let mut conn = conn_mutex.lock().unwrap();
            match conn.delete_messages(&params.folder, &params.uids) {
                Ok(()) => Ok(json!({ "success": true })),
                Err(e) => Ok(json!({ "success": false, "error": e })),
            }
        },
        None => Ok(json!({ "success": false, "error": "账号未连接" })),
    }
}

#[tauri::command]
async fn mail_batch_move(state: State<'_, AppState>, params: BatchMoveParams) -> Result<Value, String> {
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&params.account_id).cloned()
    };
    match conn_arc {
        Some(conn_mutex) => {
            let mut conn = conn_mutex.lock().unwrap();
            match conn.move_messages(&params.folder, &params.uids, &params.destination) {
                Ok(()) => Ok(json!({ "success": true })),
                Err(e) => Ok(json!({ "success": false, "error": e })),
            }
        },
        None => Ok(json!({ "success": false, "error": "账号未连接" })),
    }
}

#[tauri::command]
async fn mail_mark_flagged(state: State<'_, AppState>, params: BatchActionParams) -> Result<Value, String> {
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&params.account_id).cloned()
    };
    match conn_arc {
        Some(conn_mutex) => {
            let mut conn = conn_mutex.lock().unwrap();
            match conn.mark_messages_flagged(&params.folder, &params.uids) {
                Ok(()) => Ok(json!({ "success": true })),
                Err(e) => Ok(json!({ "success": false, "error": e })),
            }
        },
        None => Ok(json!({ "success": false, "error": "账号未连接" })),
    }
}

#[tauri::command]
async fn mail_mark_unflagged(state: State<'_, AppState>, params: BatchActionParams) -> Result<Value, String> {
    let conn_arc = {
        let services = state.mail_services.lock().unwrap();
        services.get(&params.account_id).cloned()
    };
    match conn_arc {
        Some(conn_mutex) => {
            let mut conn = conn_mutex.lock().unwrap();
            match conn.mark_messages_unflagged(&params.folder, &params.uids) {
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
                        .ok_or_else(|| "未找到草稿文件夹，请检查邮箱文件夹配置".to_string())?
                }
                Err(e) => return Ok(json!({ "success": false, "error": e })),
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

// 播放新邮件提示音（优先尝试系统原生播放器，同时向前端发射事件）
fn play_desktop_sound(app: &AppHandle) {
    #[cfg(target_os = "linux")]
    {
        let cache_dir = app.path().app_cache_dir().unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
        let sound_path = cache_dir.join("message-sound.mp3");
        if !sound_path.exists() {
            let _ = std::fs::create_dir_all(&cache_dir);
            let _ = std::fs::write(&sound_path, MESSAGE_SOUND);
        }
        let sound_str = sound_path.to_string_lossy().to_string();
        std::thread::spawn(move || {
            let played = std::process::Command::new("pw-play")
                .arg(&sound_str)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);

            if !played {
                let _ = std::process::Command::new("paplay")
                    .arg(&sound_str)
                    .status();
            }
        });
    }

    let _ = app.emit("mail-play-sound", ());
}

// ========== 测试工具箱 ==========

#[tauri::command]
async fn test_notification(app: AppHandle) -> Result<(), String> {
    play_desktop_sound(&app);
    show_system_notification(
        &app,
        "4YMail 测试通知".to_string(),
        "这是一条桌面测试通知，证明系统通知服务正常工作，点击可直接激活并唤醒主窗口。".to_string(),
        None,
        None,
    )
    .await?;
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

#[cfg(not(target_os = "linux"))]
fn ensure_notification_icon(app: &AppHandle) -> String {
    let app_cache = app.path().app_cache_dir().unwrap_or_default();
    let temp_icon = app_cache.join("notif_icon_temp.png");
    if !temp_icon.exists() {
        let _ = std::fs::create_dir_all(&app_cache);
        let _ = std::fs::write(&temp_icon, NOTIF_ICON);
    }
    temp_icon.to_string_lossy().into_owned()
}

async fn show_system_notification(
    app: &AppHandle,
    title: String,
    body: String,
    account_id: Option<String>,
    uid: Option<u32>,
) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let app_clone = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let mut notification = notify_rust::Notification::new();
            notification
                .appname("4YMail")
                .summary(&title)
                .body(&body)
                .icon("forymail")
                .action("default", "打开邮件")
                .hint(notify_rust::Hint::Category("email".to_string()))
                .hint(notify_rust::Hint::DesktopEntry("4YMail.desktop".to_string()));

            match notification.show() {
                Ok(handle) => {
                    // 关键：在独立的后台线程中等待用户动作，绝不阻塞当前通知的显示和立即返回
                    std::thread::spawn(move || {
                        handle.wait_for_action(move |action| {
                            if action == "default" {
                                log::info!("用户点击了桌面通知，唤醒主窗口并发送跳转事件");
                                if let Some(window) = app_clone.get_webview_window("main") {
                                    let _ = window.unminimize();
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                                let _ = app_clone.emit("mail-notification-clicked", json!({
                                    "accountId": account_id,
                                    "uid": uid,
                                    "folder": "INBOX"
                                }));
                            }
                        });
                    });
                    Ok(())
                }
                Err(error) => Err(error.to_string()),
            }
        })
        .await
        .map_err(|error| format!("通知任务失败: {error}"))?
    }

    #[cfg(not(target_os = "linux"))]
    {
        app.notification()
            .builder()
            .title(title)
            .body(body)
            .icon(ensure_notification_icon(app))
            .show()
            .map_err(|error| error.to_string())
    }
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
#[allow(deprecated)]
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
    
    match conn_arc {
        Some(conn_mutex) => {
            log::debug!("获取连接锁: {}", account_id);
            
            // 获取连接并尝试获取消息
            let mut conn = match conn_mutex.lock() {
                Ok(c) => c,
                Err(_) => {
                    log::warn!("账号 {} 连接锁获取失败", account_id);
                    return;
                }
            };
            
            let messages_result = conn.get_messages("INBOX", 1, 5, None, false, false);
            drop(conn);
            
            match messages_result {
                Ok(messages) => {
                    // 核心优化：获取完数据立即释放锁，防止通知弹窗阻塞前端请求

                    if messages.messages.is_empty() {
                        let _ = app.emit("mail-unread-count", json!({
                            "accountId": account_id,
                            "count": messages.unread_count
                        }));
                        return;
                    }

                    let _ = app.emit("mail-unread-count", json!({
                        "accountId": account_id,
                        "count": messages.unread_count
                    }));
                    
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
                            let clean_title = sanitize_notif_text(&format!("4YMail - {} ({} 封新邮件)", account_id, new_count), 60);
                            let clean_body = sanitize_notif_text(&format!("{}: {}", latest.from.name, latest.subject), 150);

                            let latest_uid = latest.uid;
                            let acc_id = account_id.to_string();
                            let app_clone = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(error) = show_system_notification(
                                    &app_clone,
                                    clean_title.clone(),
                                    clean_body,
                                    Some(acc_id),
                                    Some(latest_uid),
                                )
                                .await
                                {
                                    log::error!("发送新邮件系统通知失败: {error}");
                                } else {
                                    log::info!("新邮件系统通知已发送: {clean_title}");
                                }
                            });
                            
                            play_desktop_sound(app);
                            let _ = app.emit("mail-new-arrived", json!({ "accountId": account_id }));
                        }
                        last_uids.insert(account_id.to_string(), current_max_uid);
                        drop(last_uids);
                        state.save_state(app);
                    }
                }
                Err(e) => {
                    log::warn!("后台轮询连接失效 {}: {}", account_id, e);
                    // 安全清理坏连接
                    {
                        let mut services = state.mail_services.lock().unwrap();
                        services.remove(account_id);
                    }
                    let _ = app.emit("mail-connection-error", json!({
                        "accountId": account_id,
                        "error": format!("后台轮询检测到连接断开")
                    }));
                }
            }
        }
        None => {
            log::warn!("账号 {} 未连接，跳过轮询", account_id);
        }
    }
}

/// 核心：确保账号连接在线，如果离线则重连
fn ensure_connection(app: &AppHandle, account_id: &str) -> bool {
    let state = app.state::<AppState>();
    
    // 1. 检查是否已经有活跃连接，并进行实时心跳校验 (NOOP)
    let (has_connection, is_alive) = {
        let services = state.mail_services.lock().unwrap();
        if let Some(conn_arc) = services.get(account_id) {
            match conn_arc.lock() {
                Ok(mut conn) => (true, conn.is_connected()),
                Err(_) => (true, false),
            }
        } else {
            (false, false)
        }
    };
    
    if has_connection && is_alive {
        log::debug!("账号 {} 连接正常", account_id);
        return true;
    }
    
    // 2. 如果连接失效或没有连接，先安全清理旧连接
    if has_connection {
        log::warn!("账号 {} 连接已失效，准备重连", account_id);
        let mut services = state.mail_services.lock().unwrap();
        if let Some(old_conn) = services.remove(account_id) {
            if let Ok(mut conn) = old_conn.lock() {
                let _ = conn.disconnect();
            }
        }
        
        // 3. 发送连接错误通知
        let _ = app.emit("mail-connection-error", json!({
            "accountId": account_id,
            "error": "连接已断开，正在尝试重连"
        }));
    }

    // 4. 尝试使用保存的配置重连
    let config = {
        let configs = state.account_configs.lock().unwrap();
        configs.get(account_id).cloned()
    };

    if let Some(imap_config) = config {
        log::info!("正在尝试自动重连账号: {}", account_id);
        
        // 直接连接，避免复杂的 catch_unwind 处理
        let connect_result = MailConnection::connect(&imap_config);
        
        match connect_result {
            Ok(conn) => {
                log::info!("账号 {} 自动重连成功", account_id);
                {
                    let mut services = state.mail_services.lock().unwrap();
                    services.insert(account_id.to_string(), Arc::new(Mutex::new(conn)));
                }
                let _ = app.emit("mail-connection-success", json!({ "accountId": account_id }));
                true
            }
            Err(e) => {
                log::error!("自动重连失败 {}: {}", account_id, e);
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

#[tauri::command]
fn open_devtools(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        window.open_devtools();
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
        .tooltip("4YMail")
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.unminimize();
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
        .register_asynchronous_uri_scheme_protocol("mail-img", |_app, request, responder| {
            // 解析请求 URL，格式为 mail-img://proxy/<encoded_url>
            let uri = request.uri().to_string();
            let encoded_url = uri.trim_start_matches("mail-img://proxy/");
            
            // 解码 URL
            let target_url = match urlencoding::decode(encoded_url) {
                Ok(u) => u.into_owned(),
                Err(_) => {
                    responder.respond(tauri::http::Response::builder()
                        .status(400)
                        .body(Vec::new())
                        .unwrap());
                    return;
                }
            };

            // 发起请求抓取图片
            tauri::async_runtime::spawn(async move {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .build();
                
                if let Ok(client) = client {
                    match client.get(&target_url).send().await {
                        Ok(resp) => {
                            let content_type = resp.headers()
                                .get(reqwest::header::CONTENT_TYPE)
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("image/png")
                                .to_string();
                            
                            if let Ok(bytes) = resp.bytes().await {
                                responder.respond(tauri::http::Response::builder()
                                    .header("Content-Type", content_type)
                                    .header("Access-Control-Allow-Origin", "*")
                                    .body(bytes.to_vec())
                                    .unwrap());
                                return;
                            }
                        }
                        Err(e) => {
                            log::error!("Proxy image fetch error [{}]: {}", target_url, e);
                        }
                    }
                }
                
                // 默认失败响应
                responder.respond(tauri::http::Response::builder()
                    .status(404)
                    .body(Vec::new())
                    .unwrap());
            });
        })
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
            mail_batch_mark_seen,
            mail_batch_mark_unseen,
            mail_batch_delete,
            mail_batch_move,
            mail_mark_flagged,
            mail_mark_unflagged,
            mail_save_draft,
            mail_update_polling,
            mail_get_sound,
            mail_download_attachment,
            test_notification,
            test_connection_error,
            test_clear_cache,
            test_open_logs,
            open_devtools,
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
