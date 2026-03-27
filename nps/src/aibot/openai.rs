
use super::tools;

use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::error::Error;

pub trait Provider {
    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        temperature: f64,
    ) -> anyhow::Result<String>;

    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        temperature: f64,
    ) -> anyhow::Result<ToolChatResponse>;
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum ChatMessage {
    System {
        content: String,
    },
    User {
        content: String,
    },
    Assistant {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<ToolCall>>,
    },
    Tool {
        tool_call_id: String,
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub kind: String,
    pub function: FunctionDef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum ToolChatResponse {
    Text(String),
    ToolUse {
        tool_calls: Vec<ToolCall>,
        text: Option<String>,
    },
}

pub fn tool_spec_to_definition(spec: &tools::ToolSpec) -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: FunctionDef {
            name: spec.name.clone(),
            description: spec.description.clone(),
            parameters: spec.parameters.clone(),
        },
    }
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f64,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<WireToolCall>>,
}

#[derive(Debug, Serialize)]
struct ToolChatRequest {
    model: String,
    messages: Vec<WireMessage>,
    temperature: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDefinition>>,
}

#[derive(Debug, Serialize)]
struct WireMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<WireToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireToolCall {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    function: WireFunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireFunctionCall {
    name: String,
    #[serde(deserialize_with = "deserialize_arguments")]
    arguments: String,
}


fn deserialize_arguments<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: serde_json::Value = Deserialize::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) => Ok(s),
        other => Ok(other.to_string()),
    }
}

impl From<&ChatMessage> for WireMessage {
    fn from(msg: &ChatMessage) -> Self {
        match msg {
            ChatMessage::System { content } => WireMessage {
                role: "system".into(),
                content: Some(content.clone()),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage::User { content } => WireMessage {
                role: "user".into(),
                content: Some(content.clone()),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage::Assistant {
                content,
                tool_calls,
            } => WireMessage {
                role: "assistant".into(),
                content: content.clone(),
                tool_calls: tool_calls.as_ref().map(|tcs| {
                    tcs.iter()
                        .map(|tc| WireToolCall {
                            id: tc.id.clone(),
                            kind: "function".into(),
                            function: WireFunctionCall {
                                name: tc.function.name.clone(),
                                arguments: tc.function.arguments.clone(),
                            },
                        })
                        .collect()
                }),
                tool_call_id: None,
            },
            ChatMessage::Tool {
                tool_call_id,
                content,
            } => WireMessage {
                role: "tool".into(),
                content: Some(content.clone()),
                tool_calls: None,
                tool_call_id: Some(tool_call_id.clone()),
            },
        }
    }
}

pub struct OpenAiProvider {
    base_url: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(
        base_url: &str, 
        api_key: &str,
        model: &str,
    ) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5分钟超时
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            client,
        }
    }
}


impl Provider for OpenAiProvider {
    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(Message {
                role: "system".to_string(),
                content: sys.to_string(),
            });
        }
        messages.push(Message {
            role: "user".to_string(),
            content: message.to_string(),
        });
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature,
        };
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let mut builder = self.client.post(&url);
        builder = builder.header("Authorization", format!("Bearer {}", self.api_key));
        builder = builder.header("Content-Type", "application/json");
        let response = builder.json(&request).send().await?;
        if !response.status().is_success() {
            let error = response.text().await?;
            anyhow::bail!("ai api {error}");
        }
        let chat_response: ChatResponse = response.json().await?;
        chat_response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| anyhow::anyhow!("No response"))
    }


    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        temperature: f64,
    ) -> anyhow::Result<ToolChatResponse> {
        let wire_messages: Vec<WireMessage> = messages.iter().map(WireMessage::from).collect();
        let tools_field = if tools.is_empty() {
            None
        } else {
            Some(tools.to_vec())
        };
        let request = ToolChatRequest {
            model: self.model.clone(),
            messages: wire_messages,
            temperature,
            tools: tools_field,
        };

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let mut builder = self.client.post(&url);
        builder = builder.header("Authorization", format!("Bearer {}", self.api_key));
        builder = builder.header("Content-Type", "application/json");
        let response = builder.json(&request).send().await?;
        if !response.status().is_success() {
            let error = response.text().await?;
            anyhow::bail!("ai api {error}");
        }
        let chat_response: ChatResponse = response.json().await?;
        let choice = chat_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No response"))?;
        let msg = &choice.message;
        if let Some(wire_tool_calls) = &msg.tool_calls {
            if !wire_tool_calls.is_empty() {
                let tool_calls: Vec<ToolCall> = wire_tool_calls
                    .into_iter()
                    .map(|wtc| ToolCall {
                        id: wtc.id.clone(),
                        function: FunctionCall {
                            name: wtc.function.name.clone(),
                            arguments: wtc.function.arguments.clone(),
                        },
                    })
                    .collect();
                return Ok(ToolChatResponse::ToolUse {
                    tool_calls,
                    text: msg.content.clone(),
                });
            }
        }

        let text = msg
            .content
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No content in response from"))?;
        Ok(ToolChatResponse::Text(text))
    }
}

