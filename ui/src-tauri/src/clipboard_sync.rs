use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::{fs as async_fs, io::AsyncWriteExt};
use uuid::Uuid;

const SYNC_DIR: &str = "clipboard-sync";
const MESSAGES_DIR: &str = "messages";
const MESSAGE_FILE: &str = "message.json";
const HISTORY_LIMIT: usize = 500;

lazy_static! {
    static ref SYNC_ROOT: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardSyncSourceKind {
    Pc,
    Web,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardSyncSource {
    pub kind: ClipboardSyncSourceKind,
    pub client_id: Option<String>,
    pub device_name: Option<String>,
    pub device_model: Option<String>,
    pub platform: Option<String>,
    pub browser: Option<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardSyncAttachment {
    pub attachment_id: String,
    pub file_name: String,
    pub stored_name: String,
    pub mime_type: Option<String>,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardSyncMessage {
    pub message_id: String,
    pub created_at_ms: u64,
    pub source: ClipboardSyncSource,
    pub text: Option<String>,
    pub attachments: Vec<ClipboardSyncAttachment>,
}

#[derive(Debug)]
pub struct IncomingAttachment {
    pub file_name: String,
    pub mime_type: Option<String>,
    pub bytes: Vec<u8>,
}

pub fn init(app_data_dir: PathBuf) {
    let root = app_data_dir.join(SYNC_DIR);
    let mut sync_root = SYNC_ROOT.lock().unwrap();
    *sync_root = Some(root);
}

pub fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn root_dir() -> Result<PathBuf, String> {
    SYNC_ROOT
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "剪切板同步存储尚未初始化".to_string())
}

fn messages_dir() -> Result<PathBuf, String> {
    Ok(root_dir()?.join(MESSAGES_DIR))
}

fn message_dir(message_id: &str) -> Result<PathBuf, String> {
    if !is_safe_id(message_id) {
        return Err("消息 ID 不合法".into());
    }
    Ok(messages_dir()?.join(message_id))
}

fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

fn clean_text(value: Option<String>) -> Option<String> {
    let value = value?.trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn sanitize_file_name(file_name: &str) -> String {
    let cleaned = file_name
        .trim()
        .chars()
        .filter(|ch| {
            !ch.is_control() && !matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
        })
        .take(120)
        .collect::<String>();

    if cleaned.is_empty() {
        "file".into()
    } else {
        cleaned
    }
}

fn save_message_metadata(message: &ClipboardSyncMessage, dir: &Path) -> Result<(), String> {
    let content = serde_json::to_string_pretty(message).map_err(|error| error.to_string())?;
    fs::write(dir.join(MESSAGE_FILE), content).map_err(|error| error.to_string())
}

fn read_message(path: &Path) -> Option<ClipboardSyncMessage> {
    let content = fs::read_to_string(path.join(MESSAGE_FILE)).ok()?;
    serde_json::from_str::<ClipboardSyncMessage>(&content).ok()
}

fn enforce_history_limit() -> Result<(), String> {
    let dir = messages_dir()?;
    let mut messages = list_messages()?;
    if messages.len() <= HISTORY_LIMIT {
        return Ok(());
    }

    messages.sort_by_key(|message| std::cmp::Reverse(message.created_at_ms));
    for message in messages.into_iter().skip(HISTORY_LIMIT) {
        let _ = fs::remove_dir_all(dir.join(message.message_id));
    }

    Ok(())
}

pub async fn create_message(
    text: Option<String>,
    attachments: Vec<IncomingAttachment>,
    source: ClipboardSyncSource,
    write_clipboard: bool,
) -> Result<ClipboardSyncMessage, String> {
    let text = clean_text(text);
    if text.is_none() && attachments.is_empty() {
        return Err("不能发送空消息".into());
    }

    let created_at_ms = current_timestamp_ms();
    let uuid = Uuid::new_v4().to_string();
    let message_id = format!("{}-{}", created_at_ms, uuid);
    let dir = message_dir(&message_id)?;
    async_fs::create_dir_all(&dir)
        .await
        .map_err(|error| error.to_string())?;

    let mut saved_attachments = Vec::new();
    for attachment in attachments {
        let attachment_id = Uuid::new_v4().to_string();
        let file_name = sanitize_file_name(&attachment.file_name);
        let stored_name = format!("{}_{}", attachment_id, file_name);
        let target = dir.join(&stored_name);
        let mut file = async_fs::File::create(&target)
            .await
            .map_err(|error| error.to_string())?;
        file.write_all(&attachment.bytes)
            .await
            .map_err(|error| error.to_string())?;

        saved_attachments.push(ClipboardSyncAttachment {
            attachment_id,
            file_name,
            stored_name,
            mime_type: attachment.mime_type,
            size_bytes: attachment.bytes.len() as u64,
        });
    }

    let message = ClipboardSyncMessage {
        message_id,
        created_at_ms,
        source,
        text,
        attachments: saved_attachments,
    };

    save_message_metadata(&message, &dir)?;
    enforce_history_limit()?;

    if write_clipboard {
        write_message_to_clipboard(&message)?;
    }

    Ok(message)
}

pub fn list_messages() -> Result<Vec<ClipboardSyncMessage>, String> {
    let dir = messages_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut messages = fs::read_dir(dir)
        .map_err(|error| error.to_string())?
        .filter_map(|entry| read_message(&entry.ok()?.path()))
        .collect::<Vec<_>>();
    messages.sort_by_key(|message| std::cmp::Reverse(message.created_at_ms));
    Ok(messages)
}

pub fn delete_message(message_id: &str) -> Result<(), String> {
    let dir = message_dir(message_id)?;
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn clear_messages() -> Result<(), String> {
    let dir = messages_dir()?;
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(dir).map_err(|error| error.to_string())
}

pub fn attachment_path(
    message_id: &str,
    attachment_id: &str,
) -> Result<(PathBuf, ClipboardSyncAttachment), String> {
    if !is_safe_id(attachment_id) {
        return Err("附件 ID 不合法".into());
    }

    let dir = message_dir(message_id)?;
    let message = read_message(&dir).ok_or_else(|| "消息不存在".to_string())?;
    let attachment = message
        .attachments
        .into_iter()
        .find(|item| item.attachment_id == attachment_id)
        .ok_or_else(|| "附件不存在".to_string())?;
    Ok((dir.join(&attachment.stored_name), attachment))
}

fn unique_target_path(directory: &Path, file_name: &str) -> PathBuf {
    let target = directory.join(file_name);
    if !target.exists() {
        return target;
    }

    let source = Path::new(file_name);
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("file");
    let extension = source.extension().and_then(|value| value.to_str());

    // 避免覆盖用户目录中已有文件，重复文件名按系统常见格式追加序号。
    for index in 1.. {
        let next_name = match extension {
            Some(extension) if !extension.is_empty() => format!("{stem} ({index}).{extension}"),
            _ => format!("{stem} ({index})"),
        };
        let next_target = directory.join(next_name);
        if !next_target.exists() {
            return next_target;
        }
    }

    target
}

pub fn save_attachments_to_directory(
    message_id: &str,
    attachment_ids: &[String],
    directory: &Path,
) -> Result<usize, String> {
    if attachment_ids.is_empty() {
        return Err("请选择要下载的附件".into());
    }
    if !directory.is_dir() {
        return Err("保存目录不存在".into());
    }

    let mut saved_count = 0;
    for attachment_id in attachment_ids {
        let (source, attachment) = attachment_path(message_id, attachment_id)?;
        let target = unique_target_path(directory, &attachment.file_name);
        fs::copy(&source, target).map_err(|error| error.to_string())?;
        saved_count += 1;
    }

    Ok(saved_count)
}

pub fn write_message_to_clipboard(message: &ClipboardSyncMessage) -> Result<(), String> {
    let paths = message
        .attachments
        .iter()
        .map(|attachment| {
            message_dir(&message.message_id).map(|dir| dir.join(&attachment.stored_name))
        })
        .collect::<Result<Vec<_>, _>>()?;

    write_clipboard(message.text.as_deref(), &paths)
}

pub fn write_text_to_clipboard(text: &str) -> Result<(), String> {
    write_clipboard(Some(text), &[])
}

pub fn write_attachments_to_clipboard(
    message: &ClipboardSyncMessage,
    attachment_ids: &[String],
) -> Result<(), String> {
    if attachment_ids.is_empty() {
        return Err("请选择要复制的图片".into());
    }

    let selected_ids = attachment_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let dir = message_dir(&message.message_id)?;
    let paths = message
        .attachments
        .iter()
        .filter(|attachment| selected_ids.contains(attachment.attachment_id.as_str()))
        .map(|attachment| dir.join(&attachment.stored_name))
        .collect::<Vec<_>>();

    if paths.len() != selected_ids.len() {
        return Err("部分附件不存在".into());
    }

    write_clipboard(None, &paths)
}

#[cfg(target_os = "windows")]
fn write_clipboard(text: Option<&str>, files: &[PathBuf]) -> Result<(), String> {
    use std::{mem::size_of, os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::{
        Foundation::{GlobalFree, POINT},
        System::{
            DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData},
            Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
            Ole::{CF_HDROP, CF_UNICODETEXT},
        },
        UI::Shell::DROPFILES,
    };

    unsafe fn set_clipboard_text(text: &str) -> Result<(), String> {
        let mut wide = text.encode_utf16().collect::<Vec<_>>();
        wide.push(0);
        let size = wide.len() * size_of::<u16>();
        let handle = GlobalAlloc(GMEM_MOVEABLE, size);
        if handle.is_null() {
            return Err("分配剪切板文本内存失败".into());
        }

        let lock = GlobalLock(handle) as *mut u16;
        if lock.is_null() {
            GlobalFree(handle);
            return Err("锁定剪切板文本内存失败".into());
        }

        ptr::copy_nonoverlapping(wide.as_ptr(), lock, wide.len());
        GlobalUnlock(handle);

        if SetClipboardData(CF_UNICODETEXT.into(), handle).is_null() {
            GlobalFree(handle);
            return Err("写入剪切板文本失败".into());
        }
        Ok(())
    }

    unsafe fn set_clipboard_files(files: &[PathBuf]) -> Result<(), String> {
        let mut file_list = Vec::<u16>::new();
        for path in files {
            file_list.extend(path.as_os_str().encode_wide());
            file_list.push(0);
        }
        file_list.push(0);

        let header_size = size_of::<DROPFILES>();
        let files_size = file_list.len() * size_of::<u16>();
        let handle = GlobalAlloc(GMEM_MOVEABLE, header_size + files_size);
        if handle.is_null() {
            return Err("分配剪切板文件内存失败".into());
        }

        let lock = GlobalLock(handle) as *mut u8;
        if lock.is_null() {
            GlobalFree(handle);
            return Err("锁定剪切板文件内存失败".into());
        }

        let drop_files = DROPFILES {
            pFiles: header_size as u32,
            pt: POINT { x: 0, y: 0 },
            fNC: 0,
            fWide: 1,
        };
        ptr::copy_nonoverlapping(
            &drop_files as *const DROPFILES as *const u8,
            lock,
            header_size,
        );
        ptr::copy_nonoverlapping(
            file_list.as_ptr() as *const u8,
            lock.add(header_size),
            files_size,
        );
        GlobalUnlock(handle);

        if SetClipboardData(CF_HDROP.into(), handle).is_null() {
            GlobalFree(handle);
            return Err("写入剪切板文件失败".into());
        }
        Ok(())
    }

    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return Err("打开剪切板失败".into());
        }

        let mut result = Ok(());
        if EmptyClipboard() == 0 {
            result = Err("清空剪切板失败".into());
        }

        if result.is_ok() {
            if let Some(text) = text {
                result = set_clipboard_text(text);
            }
        }

        if result.is_ok() && !files.is_empty() {
            result = set_clipboard_files(files);
        }

        CloseClipboard();
        result
    }
}

#[cfg(not(target_os = "windows"))]
fn write_clipboard(text: Option<&str>, files: &[PathBuf]) -> Result<(), String> {
    let _ = (text, files);
    Ok(())
}
