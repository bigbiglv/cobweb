use cobweb_scheduler::{TaskOrigin, TaskOriginKind};
use cobweb_service::{
    execute_feature_command as dispatch_feature_command, get_feature_groups as load_feature_groups,
    get_feature_snapshot as load_feature_snapshot, FeatureCommand, FeatureExecutionResult,
    FeatureGroup, FeatureSnapshot,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::network::server::broadcast_web_state_sync;
use crate::store::{TaskHistoryEntry, TaskHistoryStatus, GLOBAL_STORE};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureNoticePayload {
    pub title: String,
    pub message: String,
    pub tone: String,
}

#[tauri::command]
pub fn get_feature_groups() -> Result<Vec<FeatureGroup>, String> {
    Ok(load_feature_groups())
}

#[tauri::command]
pub fn get_feature_snapshot() -> Result<FeatureSnapshot, String> {
    load_feature_snapshot().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn execute_feature_command(
    app: AppHandle,
    command: FeatureCommand,
) -> Result<FeatureExecutionResult, String> {
    execute_feature_command_with_origin(&app, command, TaskOrigin::pc(), None)
}

pub fn execute_feature_command_with_origin(
    app: &AppHandle,
    command: FeatureCommand,
    origin: TaskOrigin,
    task_id: Option<String>,
) -> Result<FeatureExecutionResult, String> {
    if task_id.is_none() {
        if let Some(payload) = build_web_started_notice(&command, &origin) {
            let _ = app.emit("feature_notice", payload);
        }
    }

    let result = dispatch_feature_command(command.clone()).map_err(|error| {
        if task_id.is_none() {
            record_feature_history(
                None,
                &command,
                origin.clone(),
                TaskHistoryStatus::ManualFailed,
                error.to_string(),
                app,
            );
        }
        error.to_string()
    })?;

    if let Some(payload) = build_notice_payload(&command, &result) {
        let _ = app.emit("feature_notice", payload);
    }

    if task_id.is_none() {
        record_feature_history(
            None,
            &command,
            origin,
            TaskHistoryStatus::ManualExecuted,
            result.message.clone(),
            app,
        );
    }

    broadcast_web_state_sync();

    Ok(result)
}

fn record_feature_history(
    task_id: Option<String>,
    command: &FeatureCommand,
    origin: TaskOrigin,
    status: TaskHistoryStatus,
    detail: String,
    app: &AppHandle,
) {
    let mut store = GLOBAL_STORE.lock().unwrap();
    store.append_task_history(TaskHistoryEntry::new(
        task_id,
        describe_command_title(command),
        origin,
        status,
        detail,
    ));
    drop(store);
    let _ = app.emit("task_history_changed", serde_json::json!({}));
}

fn describe_command_title(command: &FeatureCommand) -> String {
    match command {
        FeatureCommand::Shutdown => "关机".into(),
        FeatureCommand::Restart => "重启".into(),
        FeatureCommand::TestNotification => "测试提示".into(),
        FeatureCommand::ErrorTest => "错误测试提示".into(),
        FeatureCommand::Volume { level } => format!("设置音量 {level}%"),
        FeatureCommand::AppleMusicOpen => "打开 Apple Music".into(),
        FeatureCommand::AppleMusicPrevious => "Apple Music 上一曲".into(),
        FeatureCommand::AppleMusicPlayPause => "Apple Music 播放状态切换".into(),
        FeatureCommand::AppleMusicNext => "Apple Music 下一曲".into(),
    }
}

fn build_notice_payload(
    command: &FeatureCommand,
    result: &FeatureExecutionResult,
) -> Option<FeatureNoticePayload> {
    match command {
        FeatureCommand::TestNotification => Some(FeatureNoticePayload {
            title: "测试提示".into(),
            message: result.message.clone(),
            tone: "success".into(),
        }),
        _ => None,
    }
}

fn build_web_started_notice(
    command: &FeatureCommand,
    origin: &TaskOrigin,
) -> Option<FeatureNoticePayload> {
    if !matches!(origin.kind, TaskOriginKind::Web) {
        return None;
    }

    Some(FeatureNoticePayload {
        title: "Web 指令".into(),
        message: format!(
            "{} 发起了{}",
            origin.client_name,
            describe_command_title(command)
        ),
        tone: "warning".into(),
    })
}
