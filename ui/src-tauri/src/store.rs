use cobweb_scheduler::{current_timestamp_ms, ScheduledTask, TaskOrigin};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const TASK_HISTORY_LIMIT: usize = 200;

fn default_mdns_enabled() -> bool {
    true
}

fn default_close_to_tray_on_close() -> bool {
    true
}

fn default_auto_update_enabled() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PairedClient {
    pub client_id: String,
    pub client_name: String,
    pub token_hash: String,
    pub last_seen_at: u64,
    #[serde(default)]
    pub last_ip: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TaskHistoryStatus {
    Queued,
    Cancelled,
    Executed,
    Failed,
    ManualExecuted,
    ManualFailed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskHistoryEntry {
    pub entry_id: String,
    pub task_id: Option<String>,
    pub title: String,
    pub origin: TaskOrigin,
    pub status: TaskHistoryStatus,
    pub recorded_at_ms: u64,
    pub detail: String,
}

impl TaskHistoryEntry {
    pub fn new(
        task_id: Option<String>,
        title: impl Into<String>,
        origin: TaskOrigin,
        status: TaskHistoryStatus,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            entry_id: Uuid::new_v4().to_string(),
            task_id,
            title: title.into(),
            origin,
            status,
            recorded_at_ms: current_timestamp_ms(),
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub device_id: String,
    pub device_name: String,
    pub paired_clients: HashMap<String, PairedClient>,
    #[serde(default)]
    pub scheduled_tasks: Vec<ScheduledTask>,
    #[serde(default)]
    pub task_history: Vec<TaskHistoryEntry>,
    #[serde(default = "default_mdns_enabled")]
    pub mdns_enabled: bool,
    #[serde(default = "default_close_to_tray_on_close")]
    pub close_to_tray_on_close: bool,
    #[serde(default = "default_auto_update_enabled")]
    pub auto_update_enabled: bool,
    #[serde(default)]
    pub audio_app_routes: HashMap<String, String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let hostname = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "Cobweb".to_string());

        Self {
            device_id: Uuid::new_v4().to_string(),
            device_name: hostname,
            paired_clients: HashMap::new(),
            scheduled_tasks: Vec::new(),
            task_history: Vec::new(),
            mdns_enabled: true,
            close_to_tray_on_close: true,
            auto_update_enabled: true,
            audio_app_routes: HashMap::new(),
        }
    }
}

lazy_static! {
    pub static ref GLOBAL_STORE: Arc<Mutex<Store>> = Arc::new(Mutex::new(Store::new(None)));
}

pub fn init_store(app_data_dir: PathBuf) {
    let config_file = app_data_dir.join("devices.json");
    let mut store = GLOBAL_STORE.lock().unwrap();
    store.set_path(config_file);
}

pub struct Store {
    pub config_path: Option<PathBuf>,
    pub data: AppConfig,
}

impl Store {
    pub fn new(path: Option<PathBuf>) -> Self {
        let mut store = Self {
            config_path: path,
            data: AppConfig::default(),
        };
        store.load();
        store
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.config_path = Some(path);
        self.load();
    }

    pub fn load(&mut self) -> bool {
        if let Some(path) = &self.config_path {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                        self.data = config;
                        return true;
                    }
                }
            }
        }
        self.save();
        false
    }

    pub fn save(&self) -> bool {
        if let Some(path) = &self.config_path {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(content) = serde_json::to_string_pretty(&self.data) {
                return fs::write(path, content).is_ok();
            }
        }
        false
    }

    pub fn is_client_paired(&self, client_id: &str, token_hash: &str) -> bool {
        if let Some(client) = self.data.paired_clients.get(client_id) {
            return client.token_hash == token_hash;
        }
        false
    }

    pub fn add_paired_client(&mut self, client: PairedClient) {
        self.data
            .paired_clients
            .insert(client.client_id.clone(), client);
        self.save();
    }

    pub fn remove_paired_client(&mut self, client_id: &str) {
        self.data.paired_clients.remove(client_id);
        self.save();
    }

    pub fn upsert_scheduled_task(&mut self, task: ScheduledTask) {
        self.data
            .scheduled_tasks
            .retain(|existing| existing.task_id != task.task_id);
        self.data.scheduled_tasks.push(task);
        self.data
            .scheduled_tasks
            .sort_by_key(|scheduled_task| scheduled_task.execute_at_ms);
        self.save();
    }

    pub fn take_scheduled_task(&mut self, task_id: &str) -> Option<ScheduledTask> {
        let index = self
            .data
            .scheduled_tasks
            .iter()
            .position(|scheduled_task| scheduled_task.task_id == task_id)?;
        let task = self.data.scheduled_tasks.remove(index);
        self.save();
        Some(task)
    }

    pub fn append_task_history(&mut self, entry: TaskHistoryEntry) {
        self.data.task_history.insert(0, entry);
        if self.data.task_history.len() > TASK_HISTORY_LIMIT {
            self.data.task_history.truncate(TASK_HISTORY_LIMIT);
        }
        self.save();
    }

    pub fn list_task_history(&self) -> Vec<TaskHistoryEntry> {
        self.data.task_history.clone()
    }

    pub fn set_mdns_enabled(&mut self, enabled: bool) {
        self.data.mdns_enabled = enabled;
        self.save();
    }

    pub fn set_close_to_tray_on_close(&mut self, enabled: bool) {
        self.data.close_to_tray_on_close = enabled;
        self.save();
    }

    pub fn set_auto_update_enabled(&mut self, enabled: bool) {
        self.data.auto_update_enabled = enabled;
        self.save();
    }

    pub fn set_audio_app_route(&mut self, app_id: String, device_id: String) {
        self.data.audio_app_routes.insert(app_id, device_id);
        self.save();
    }

    pub fn remove_audio_app_route(&mut self, app_id: &str) {
        self.data.audio_app_routes.remove(app_id);
        self.save();
    }
}
