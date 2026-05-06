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

// Note 1: 这个结构体是后端发给前端的提示消息。Rust 后端不会直接操作 Vue 页面，
// 而是通过 Tauri 事件把 title/message/tone 传出去，前端收到事件后再展示提示。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureNoticePayload {
    pub title: String,
    pub message: String,
    pub tone: String,
}

#[tauri::command]
pub fn get_feature_groups() -> Result<Vec<FeatureGroup>, String> {
    // Note 2: #[tauri::command] 会把这个 Rust 函数暴露给前端 invoke 调用。
    // 这里不直接拼 UI，而是返回一份“功能清单”，前端按清单渲染按钮、滑块和播放器控件。
    Ok(load_feature_groups())
}

#[tauri::command]
pub fn get_feature_snapshot() -> Result<FeatureSnapshot, String> {
    // Note 3: snapshot 表示当前系统状态，比如音量、Apple Music 是否运行。
    // map_err 把业务错误转成 String，因为 Tauri 命令返回给前端时需要可序列化的错误。
    load_feature_snapshot().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn execute_feature_command(
    app: AppHandle,
    command: FeatureCommand,
) -> Result<FeatureExecutionResult, String> {
    // Note 4: PC 桌面端按钮触发的命令会走这里。TaskOrigin::pc() 用来记录历史来源，
    // 这样任务历史里能区分是 PC、Web 还是移动端发起的功能执行。
    execute_feature_command_with_origin(&app, command, TaskOrigin::pc(), None)
}

pub fn execute_feature_command_with_origin(
    app: &AppHandle,
    command: FeatureCommand,
    origin: TaskOrigin,
    task_id: Option<String>,
) -> Result<FeatureExecutionResult, String> {
    // Note 5: 这个函数是所有功能执行的统一入口，手动点击、Web 控制和定时任务都会复用它。
    // task_id 为 Some 时表示定时任务执行，历史记录由 scheduler 写入，避免重复记一条。
    if task_id.is_none() {
        if let Some(payload) = build_web_started_notice(&command, &origin) {
            let _ = app.emit("feature_notice", payload);
        }
    }

    // Note 6: dispatch_feature_command 才是真正的业务分发函数，位于 service crate。
    // 这里负责外围工作：失败记录、成功提示、历史记录和通知 Web 端刷新状态。
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

    // Note 7: 某些功能执行后会改变系统状态，例如音量或 Apple Music 播放状态。
    // 广播同步可以让 Web 控制台、移动端和桌面端尽快看到最新状态。
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
    // Note 8: GLOBAL_STORE 是一个全局持久化状态容器。这里先 lock 拿到可写权限，
    // 写完后 drop(store) 主动释放锁，再发事件，避免事件处理过程中继续占着锁。
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
    // Note 9: FeatureCommand 是后端真正认识的功能枚举。这里把枚举转成人能读懂的中文标题，
    // 用在通知和历史记录里；新增功能时通常也要在这里补一条标题。
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
    // Note 10: 不是每个功能都需要弹出桌面提示。当前只给“测试提示”返回 Some，
    // 其他功能返回 None，表示执行成功但不额外弹提示。
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
    // Note 11: Web 控制台可以远程触发 PC 功能，所以这里专门给 Web 来源加一个警示提示。
    // matches! 是 Rust 的模式匹配宏，用来判断 origin.kind 是否为 Web。
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
