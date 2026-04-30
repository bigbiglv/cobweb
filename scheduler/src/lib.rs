use cobweb_service::FeatureCommand;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskOriginKind {
    Pc,
    Mobile,
    Web,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskOrigin {
    pub kind: TaskOriginKind,
    pub client_id: Option<String>,
    pub client_name: String,
}

impl TaskOrigin {
    pub fn pc() -> Self {
        Self {
            kind: TaskOriginKind::Pc,
            client_id: None,
            client_name: "PC".into(),
        }
    }

    pub fn mobile(client_id: impl Into<String>, client_name: impl Into<String>) -> Self {
        Self {
            kind: TaskOriginKind::Mobile,
            client_id: Some(client_id.into()),
            client_name: client_name.into(),
        }
    }

    pub fn web(client_name: impl Into<String>) -> Self {
        Self {
            kind: TaskOriginKind::Web,
            client_id: None,
            client_name: client_name.into(),
        }
    }

    pub fn web_client(client_id: impl Into<String>, client_name: impl Into<String>) -> Self {
        Self {
            kind: TaskOriginKind::Web,
            client_id: Some(client_id.into()),
            client_name: client_name.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTask {
    pub task_id: String,
    pub title: String,
    pub created_at_ms: u64,
    pub execute_at_ms: u64,
    pub origin: TaskOrigin,
    #[serde(flatten)]
    pub command: FeatureCommand,
}

impl ScheduledTask {
    pub fn new(command: FeatureCommand, execute_at_ms: u64, origin: TaskOrigin) -> Self {
        Self {
            task_id: Uuid::new_v4().to_string(),
            title: describe_command(&command),
            created_at_ms: current_timestamp_ms(),
            execute_at_ms,
            origin,
            command,
        }
    }
}

pub fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

pub fn describe_command(command: &FeatureCommand) -> String {
    match command {
        FeatureCommand::Shutdown => "定时关机".into(),
        FeatureCommand::Restart => "定时重启".into(),
        FeatureCommand::TestNotification => "定时测试提示".into(),
        FeatureCommand::ErrorTest => "定时错误测试提示".into(),
        FeatureCommand::Volume { level } => format!("定时音量调整到 {level}%"),
        FeatureCommand::AppleMusicOpen => "定时打开 Apple Music".into(),
        FeatureCommand::AppleMusicPrevious => "定时 Apple Music 上一曲".into(),
        FeatureCommand::AppleMusicPlayPause => "定时 Apple Music 播放状态切换".into(),
        FeatureCommand::AppleMusicNext => "定时 Apple Music 下一曲".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{ScheduledTask, TaskOrigin};
    use cobweb_service::FeatureCommand;

    #[test]
    fn create_task_should_fill_default_fields() {
        let task = ScheduledTask::new(
            FeatureCommand::Shutdown,
            1_700_000_000_000,
            TaskOrigin::pc(),
        );

        assert!(!task.task_id.is_empty());
        assert_eq!(task.title, "定时关机");
        assert_eq!(task.execute_at_ms, 1_700_000_000_000);
        assert_eq!(task.origin.client_name, "PC");
    }
}
