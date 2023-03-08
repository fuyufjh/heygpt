use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeltaMessage {
    pub role: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Request {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResponseMessage {
    pub choices: Vec<ResponseChoice>,
    pub created: u64,
    pub id: String,
    pub model: String,
    pub object: String,
    pub usage: ResponseUsage,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResponseChoice {
    pub message: Message,
    pub index: usize,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResponseUsage {
    pub completion_tokens: isize,
    pub prompt_tokens: isize,
    pub total_tokens: isize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResponseStreamMessage {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ResponseDeltaChoice>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResponseDeltaChoice {
    pub delta: DeltaMessage,
    pub index: usize,
    pub finish_reason: Option<String>,
}

/// OpenAI API returns error object on failure
#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub message: String,
    pub r#type: String,
    pub param: Option<serde_json::Value>,
    pub code: Option<serde_json::Value>,
}

/// Wrapper to deserialize the error object nested in "error" JSON key
#[derive(Debug, Deserialize)]
pub struct WrappedApiError {
    pub error: ApiError,
}
