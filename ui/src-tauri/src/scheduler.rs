use cobweb_scheduler::{current_timestamp_ms, ScheduledTask};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;

use crate::{
    features::execute_feature_command_with_origin,
    network::server::{broadcast_tasks_sync, broadcast_web_state_sync, ACTIVE_CONNECTIONS},
    store::{TaskHistoryEntry, TaskHistoryStatus, GLOBAL_STORE},
};

lazy_static! {
    static ref CANCEL_HANDLES: Arc<Mutex<HashMap<String, oneshot::Sender<()>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

pub fn init(app: &AppHandle) {
    let tasks = {
        let store = GLOBAL_STORE.lock().unwrap();
        store.data.scheduled_tasks.clone()
    };

    for task in tasks {
        spawn_task(app.clone(), task);
    }
}

pub fn list_tasks() -> Vec<ScheduledTask> {
    let store = GLOBAL_STORE.lock().unwrap();
    let mut tasks = store.data.scheduled_tasks.clone();
    tasks.sort_by_key(|task| task.execute_at_ms);
    tasks
}

pub fn list_task_history() -> Vec<TaskHistoryEntry> {
    let store = GLOBAL_STORE.lock().unwrap();
    store.list_task_history()
}

pub fn create_task(app: &AppHandle, task: ScheduledTask) -> ScheduledTask {
    {
        let mut store = GLOBAL_STORE.lock().unwrap();
        store.upsert_scheduled_task(task.clone());
        store.append_task_history(TaskHistoryEntry::new(
            Some(task.task_id.clone()),
            task.title.clone(),
            task.origin.clone(),
            TaskHistoryStatus::Queued,
            format!("任务已创建，计划在 {} 执行", task.execute_at_ms),
        ));
    }

    spawn_task(app.clone(), task.clone());
    emit_tasks_changed(app);
    emit_task_history_changed(app);
    task
}

pub fn cancel_task(app: &AppHandle, task_id: &str) -> Option<ScheduledTask> {
    let task = {
        let mut store = GLOBAL_STORE.lock().unwrap();
        let task = store.take_scheduled_task(task_id)?;
        store.append_task_history(TaskHistoryEntry::new(
            Some(task.task_id.clone()),
            task.title.clone(),
            task.origin.clone(),
            TaskHistoryStatus::Cancelled,
            "用户已停止该定时任务".to_string(),
        ));
        task
    };

    if let Some(cancel) = CANCEL_HANDLES.lock().unwrap().remove(task_id) {
        let _ = cancel.send(());
    }

    emit_tasks_changed(app);
    emit_task_history_changed(app);
    Some(task)
}

fn spawn_task(app: AppHandle, task: ScheduledTask) {
    if let Some(previous) = CANCEL_HANDLES.lock().unwrap().remove(&task.task_id) {
        let _ = previous.send(());
    }

    let task_id = task.task_id.clone();
    let task_for_run = task.clone();
    let (cancel_tx, mut cancel_rx) = oneshot::channel();
    CANCEL_HANDLES
        .lock()
        .unwrap()
        .insert(task_id.clone(), cancel_tx);

    tauri::async_runtime::spawn(async move {
        let now = current_timestamp_ms();
        let wait_ms = task.execute_at_ms.saturating_sub(now);
        let sleep = tokio::time::sleep(std::time::Duration::from_millis(wait_ms));
        tokio::pin!(sleep);

        tokio::select! {
            _ = &mut sleep => {
                let execution_result = execute_feature_command_with_origin(
                    &app,
                    task_for_run.command.clone(),
                    task_for_run.origin.clone(),
                    Some(task_for_run.task_id.clone()),
                );

                {
                    let mut store = GLOBAL_STORE.lock().unwrap();
                    let _ = store.take_scheduled_task(&task_id);
                    match execution_result {
                        Ok(result) => store.append_task_history(TaskHistoryEntry::new(
                            Some(task_for_run.task_id.clone()),
                            task_for_run.title.clone(),
                            task_for_run.origin.clone(),
                            TaskHistoryStatus::Executed,
                            result.message,
                        )),
                        Err(error) => {
                            log::error!("Scheduled task {} failed: {}", task_id, error);
                            store.append_task_history(TaskHistoryEntry::new(
                                Some(task_for_run.task_id.clone()),
                                task_for_run.title.clone(),
                                task_for_run.origin.clone(),
                                TaskHistoryStatus::Failed,
                                error,
                            ));
                        }
                    }
                }

                CANCEL_HANDLES.lock().unwrap().remove(&task_id);
                emit_tasks_changed(&app);
                emit_task_history_changed(&app);
            }
            _ = &mut cancel_rx => {
                CANCEL_HANDLES.lock().unwrap().remove(&task_id);
            }
        }
    });
}

fn emit_tasks_changed(app: &AppHandle) {
    let _ = app.emit("scheduled_tasks_changed", serde_json::json!({}));
    broadcast_tasks_sync();
    broadcast_web_state_sync();
    notify_active_mobile_clients();
}

fn emit_task_history_changed(app: &AppHandle) {
    let _ = app.emit("task_history_changed", serde_json::json!({}));
}

fn notify_active_mobile_clients() {
    let ips = {
        let active_connections = ACTIVE_CONNECTIONS.lock().unwrap();
        let store = GLOBAL_STORE.lock().unwrap();

        active_connections
            .iter()
            .filter_map(|client_id| {
                store
                    .data
                    .paired_clients
                    .get(client_id)
                    .and_then(|client| client.last_ip.clone())
            })
            .collect::<Vec<_>>()
    };

    for ip in ips {
        tauri::async_runtime::spawn(async move {
            let client = match reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(1000))
                .build()
            {
                Ok(client) => client,
                Err(error) => {
                    log::debug!("Failed to build task notification client: {}", error);
                    return;
                }
            };

            let url = format!("http://{}:3001/tasks-changed", ip);
            if let Err(error) = client.post(url).send().await {
                log::debug!("Failed to notify mobile task change: {}", error);
            }
        });
    }
}
