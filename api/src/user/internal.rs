use serde::{Deserialize, Serialize};

// Internal API request structures
#[derive(Debug, Serialize, Deserialize)]
pub struct InternalRequest {
    pub method: String,
    pub params: InternalParams,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InternalParams {
    pub thread: ThreadData,
    #[serde(rename = "createdOnServer")]
    pub created_on_server: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadData {
    pub v: u32,
    pub id: String,
    pub created: u64,
    pub messages: Vec<ThreadMessage>,
    pub env: ThreadEnvironment,
    pub title: String,
    #[serde(rename = "~debug", skip_serializing_if = "Option::is_none")]
    pub debug: Option<ThreadDebug>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadMessage {
    pub role: String,
    pub content: Vec<MessageContent>,
    #[serde(rename = "userState", skip_serializing_if = "Option::is_none")]
    pub user_state: Option<UserState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<MessageMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<MessageState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<MessageUsage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageContent {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(flatten)]
    pub data: MessageContentData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContentData {
    Text { text: String },
    Thinking { thinking: String, signature: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserState {
    #[serde(rename = "currentlyVisibleFiles")]
    pub currently_visible_files: Vec<String>,
    #[serde(rename = "runningTerminalCommands")]
    pub running_terminal_commands: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageMeta {
    #[serde(rename = "sentAt")]
    pub sent_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageState {
    #[serde(rename = "type")]
    pub state_type: String,
    #[serde(rename = "stopReason")]
    pub stop_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageUsage {
    #[serde(rename = "maxInputTokens")]
    pub max_input_tokens: u64,
    #[serde(rename = "inputTokens")]
    pub input_tokens: u64,
    #[serde(rename = "outputTokens")]
    pub output_tokens: u64,
    #[serde(rename = "cacheCreationInputTokens")]
    pub cache_creation_input_tokens: u64,
    #[serde(rename = "cacheReadInputTokens")]
    pub cache_read_input_tokens: u64,
    #[serde(rename = "totalInputTokens")]
    pub total_input_tokens: u64,
    #[serde(rename = "thinkingBudget")]
    pub thinking_budget: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadDebug {
    #[serde(rename = "lastInferenceUsage")]
    pub last_inference_usage: MessageUsage,
    #[serde(rename = "lastInferenceInput")]
    pub last_inference_input: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadEnvironment {
    pub initial: InitialEnvironment,
    #[serde(rename = "systemPromptData")]
    pub system_prompt_data: SystemPromptData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitialEnvironment {
    pub trees: Vec<TreeInfo>,
    pub platform: PlatformInfo,
    pub interactive: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TreeInfo {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "fsPath")]
    pub fs_path: String,
    pub uri: String,
    pub repository: RepositoryInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepositoryInfo {
    #[serde(rename = "type")]
    pub repo_type: String,
    pub url: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub sha: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub os: String,
    #[serde(rename = "osVersion")]
    pub os_version: String,
    #[serde(rename = "cpuArchitecture")]
    pub cpu_architecture: String,
    #[serde(rename = "webBrowser")]
    pub web_browser: bool,
    pub client: String,
    #[serde(rename = "clientVersion")]
    pub client_version: String,
    #[serde(rename = "clientType")]
    pub client_type: String,
    pub config: ConfigInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigInfo {
    pub settings: Vec<ConfigSetting>,
    pub environment: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigSetting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemPromptData {
    #[serde(rename = "workspacePaths")]
    pub workspace_paths: Vec<String>,
    #[serde(rename = "workingDirectory")]
    pub working_directory: String,
    #[serde(rename = "rootDirectoryListing")]
    pub root_directory_listing: String,
} 