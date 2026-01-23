use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlateStatus {
    #[default]
    Starting,
    Running,
    Idle,
    AwaitingInput,
    AwaitingApproval,
    Error,
    Closed,
}

impl PlateStatus {
    pub fn from_tool(tool_name: &str) -> Self {
        match tool_name {
            "AskUserQuestion" => Self::AwaitingInput,
            "ExitPlanMode" => Self::AwaitingApproval,
            _ => Self::Running,
        }
    }

    pub fn needs_attention(&self) -> bool {
        matches!(
            self,
            Self::AwaitingInput | Self::AwaitingApproval | Self::Idle | Self::Error | Self::Closed
        )
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Idle => "idle",
            Self::AwaitingInput => "awaiting_input",
            Self::AwaitingApproval => "awaiting_approval",
            Self::Error => "error",
            Self::Closed => "closed",
        }
    }

    pub fn icon(&self) -> char {
        match self {
            Self::Starting => '.',
            Self::Running => '>',
            Self::Idle => '-',
            Self::AwaitingInput => '?',
            Self::AwaitingApproval => '!',
            Self::Error => 'X',
            Self::Closed => 'x',
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Starting => "start",
            Self::Running => "running",
            Self::Idle => "idle",
            Self::AwaitingInput => "input",
            Self::AwaitingApproval => "approve",
            Self::Error => "error",
            Self::Closed => "closed",
        }
    }
}

impl std::str::FromStr for PlateStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "starting" => Ok(Self::Starting),
            "running" => Ok(Self::Running),
            "idle" => Ok(Self::Idle),
            "awaiting_input" => Ok(Self::AwaitingInput),
            "awaiting_approval" => Ok(Self::AwaitingApproval),
            "error" => Ok(Self::Error),
            "closed" => Ok(Self::Closed),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub session_id: String,
    pub project_path: String,
    pub event_type: String,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_params: Option<serde_json::Value>,
    #[serde(default)]
    pub transcript_path: Option<String>,
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub tmux_target: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plate {
    pub session_id: String,
    pub project_path: String,
    #[serde(default)]
    pub transcript_path: Option<String>,
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub tmux_target: Option<String>,
    pub status: PlateStatus,
    #[serde(default)]
    pub last_event_type: Option<String>,
    #[serde(default)]
    pub last_tool: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub todo_progress: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Plate {
    pub fn project_name(&self) -> &str {
        self.project_path
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(&self.project_path)
    }
}
