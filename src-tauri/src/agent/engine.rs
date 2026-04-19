use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::process::Command;

use crate::observability::{EventBus, JarvisEvent};

const CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";
const MODEL: &str = "claude-sonnet-4-6";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
    system: String,
    tools: Vec<ToolDefinition>,
}

#[derive(Debug, Serialize, Clone)]
struct ToolDefinition {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ResponseContent>,
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ResponseContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

pub struct AgentEngine {
    api_key: String,
    history: Vec<ClaudeMessage>,
}

impl AgentEngine {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            history: Vec::new(),
        }
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn history(&self) -> &[ClaudeMessage] {
        &self.history
    }

    pub fn set_history(&mut self, history: Vec<ClaudeMessage>) {
        self.history = history;
    }

    pub async fn send_with(
        api_key: &str,
        history: &[ClaudeMessage],
        message: &str,
        event_bus: &EventBus,
    ) -> Result<(String, Vec<ClaudeMessage>), String> {
        if api_key.is_empty() {
            return Err("No API key configured. Please set ANTHROPIC_API_KEY in your .env file.".to_string());
        }

        let mut messages: Vec<ClaudeMessage> = history.to_vec();
        messages.push(ClaudeMessage {
            role: "user".to_string(),
            content: serde_json::Value::String(message.to_string()),
        });

        event_bus.emit(JarvisEvent::StateChanged {
            from: "idle".to_string(),
            to: "thinking".to_string(),
        });

        let tools = get_tool_definitions();

        let mut iteration = 0;
        let max_iterations = 10;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                event_bus.emit(JarvisEvent::StateChanged {
                    from: "acting".to_string(),
                    to: "idle".to_string(),
                });
                return Ok(("I reached my maximum number of reasoning steps. Please try rephrasing your request.".to_string(), messages));
            }

            let request = ClaudeRequest {
                model: MODEL.to_string(),
                max_tokens: 4096,
                messages: messages.clone(),
                system: build_system_prompt(),
                tools: tools.clone(),
            };

            let client = Client::new();
            let response = client
                .post(CLAUDE_API_URL)
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&request)
                .send()
                .await
                .map_err(|e| {
                    event_bus.emit(JarvisEvent::Error {
                        source: "agent".to_string(),
                        message: format!("Network error: {}", e),
                    });
                    format!("Network error: {}", e)
                })?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                let error_msg = format!("API error {}: {}", status, body);
                event_bus.emit(JarvisEvent::Error {
                    source: "agent".to_string(),
                    message: error_msg.clone(),
                });
                event_bus.emit(JarvisEvent::StateChanged {
                    from: "thinking".to_string(),
                    to: "idle".to_string(),
                });
                return Err(error_msg);
            }

            let claude_response: ClaudeResponse = response
                .json()
                .await
                .map_err(|e| {
                    format!("Parse error: {}", e)
                })?;

            let stop_reason = claude_response.stop_reason.as_deref().unwrap_or("");

            // Build assistant message from response
            let assistant_content: Vec<serde_json::Value> = claude_response
                .content
                .iter()
                .map(|c| match c {
                    ResponseContent::Text { text } => serde_json::json!({
                        "type": "text",
                        "text": text
                    }),
                    ResponseContent::ToolUse { id, name, input } => serde_json::json!({
                        "type": "tool_use",
                        "id": id,
                        "name": name,
                        "input": input
                    }),
                })
                .collect();

            messages.push(ClaudeMessage {
                role: "assistant".to_string(),
                content: serde_json::Value::Array(assistant_content.clone()),
            });

            if stop_reason == "end_turn" {
                let text_parts: Vec<String> = claude_response
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        ResponseContent::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect();

                event_bus.emit(JarvisEvent::StateChanged {
                    from: "acting".to_string(),
                    to: "idle".to_string(),
                });

                return Ok((text_parts.join(""), messages));
            }

            if stop_reason == "tool_use" {
                event_bus.emit(JarvisEvent::StateChanged {
                    from: "thinking".to_string(),
                    to: "acting".to_string(),
                });

                let mut tool_results: Vec<serde_json::Value> = Vec::new();

                for content in &claude_response.content {
                    if let ResponseContent::ToolUse { id, name, input } = content {
                        event_bus.emit(JarvisEvent::ToolInvoked {
                            tool: name.clone(),
                            params: input.clone(),
                        });

                        let result = execute_tool(name, &input);

                        event_bus.emit(JarvisEvent::ToolCompleted {
                            tool: name.clone(),
                            result: serde_json::Value::String(result.clone()),
                        });

                        tool_results.push(serde_json::json!({
                            "type": "tool_result",
                            "tool_use_id": id,
                            "content": result
                        }));
                    }
                }

                messages.push(ClaudeMessage {
                    role: "user".to_string(),
                    content: serde_json::Value::Array(tool_results),
                });

                continue;
            }

            // Unknown stop reason
            let text_parts: Vec<String> = claude_response
                .content
                .iter()
                .filter_map(|c| match c {
                    ResponseContent::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect();

            event_bus.emit(JarvisEvent::StateChanged {
                from: "acting".to_string(),
                to: "idle".to_string(),
            });

            return Ok((
                if text_parts.is_empty() {
                    "I completed the task.".to_string()
                } else {
                    text_parts.join("")
                },
                messages,
            ));
        }
    }
}

fn execute_tool(name: &str, input: &serde_json::Value) -> String {
    match name {
        "launch_app" => {
            let app_name = input.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
            let output = Command::new("osascript")
                .arg("-e")
                .arg(format!("tell application \"{}\" to activate", app_name))
                .output();

            match output {
                Ok(o) if o.status.success() => format!("Launched {}", app_name),
                Ok(o) => format!("Failed to launch {}: {}", app_name, String::from_utf8_lossy(&o.stderr)),
                Err(e) => format!("Error launching {}: {}", app_name, e),
            }
        }
        "open_url" => {
            let url = input.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let output = Command::new("open").arg(url).output();
            match output {
                Ok(o) if o.status.success() => format!("Opened {}", url),
                Ok(o) => format!("Failed to open URL: {}", String::from_utf8_lossy(&o.stderr)),
                Err(e) => format!("Error opening URL: {}", e),
            }
        }
        "open_file" => {
            let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let output = Command::new("open").arg(path).output();
            match output {
                Ok(o) if o.status.success() => format!("Opened {}", path),
                Ok(o) => format!("Failed to open file: {}", String::from_utf8_lossy(&o.stderr)),
                Err(e) => format!("Error opening file: {}", e),
            }
        }
        "list_running_apps" => {
            let output = Command::new("osascript")
                .arg("-e")
                .arg("tell application \"System Events\" to get name of every process whose background only is false")
                .output();

            match output {
                Ok(o) if o.status.success() => {
                    let result = String::from_utf8_lossy(&o.stdout);
                    let apps: Vec<&str> = result.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                    format!("Running apps: {}", apps.join(", "))
                }
                Ok(o) => format!("Failed to list apps: {}", String::from_utf8_lossy(&o.stderr)),
                Err(e) => format!("Error listing apps: {}", e),
            }
        }
        "save_memory" => {
            let content = input.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let category = input.get("category").and_then(|v| v.as_str()).unwrap_or("notes");
            format!("Memory saved: [{}] {}", category, content)
        }
        "draft_job_description" => {
            let role = input.get("role").and_then(|v| v.as_str()).unwrap_or("unknown role");
            let department = input.get("department").and_then(|v| v.as_str()).unwrap_or("");
            let level = input.get("level").and_then(|v| v.as_str()).unwrap_or("mid");
            format!("Job description drafted for {} {} in {} department. The AI will generate this content in its response.", level, role, department)
        }
        "draft_outreach" => {
            let candidate = input.get("candidate_name").and_then(|v| v.as_str()).unwrap_or("candidate");
            let role = input.get("role").and_then(|v| v.as_str()).unwrap_or("position");
            format!("Outreach message drafted for {} regarding {} role.", candidate, role)
        }
        "draft_interview_questions" => {
            let role = input.get("role").and_then(|v| v.as_str()).unwrap_or("position");
            let focus = input.get("focus_area").and_then(|v| v.as_str()).unwrap_or("general");
            format!("Interview question pack drafted for {} role, focus: {}.", role, focus)
        }
        _ => format!("Unknown tool: {}", name),
    }
}

fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "launch_app".to_string(),
            description: "Launch or activate an application on macOS by name.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Application name (e.g. 'Safari', 'Spotify', 'VS Code')" }
                },
                "required": ["name"]
            }),
        },
        ToolDefinition {
            name: "open_url".to_string(),
            description: "Open a URL in the default browser.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "The URL to open" }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "open_file".to_string(),
            description: "Open a file or folder with the default application.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File or folder path to open" }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "list_running_apps".to_string(),
            description: "List all currently running macOS applications.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDefinition {
            name: "save_memory".to_string(),
            description: "Save important information to long-term memory for later recall.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "content": { "type": "string", "description": "The information to remember" },
                    "category": { "type": "string", "description": "Category: profile, preferences, facts, notes, recruiting", "enum": ["profile", "preferences", "facts", "notes", "recruiting", "workflows"] }
                },
                "required": ["content"]
            }),
        },
        ToolDefinition {
            name: "draft_job_description".to_string(),
            description: "Signal that you are drafting a job description. Use this when the user asks you to write a JD, then generate the content in your response.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "role": { "type": "string", "description": "Job title/role" },
                    "department": { "type": "string", "description": "Department or team" },
                    "level": { "type": "string", "description": "Seniority level (junior, mid, senior, lead, principal)", "enum": ["junior", "mid", "senior", "lead", "principal"] }
                },
                "required": ["role"]
            }),
        },
        ToolDefinition {
            name: "draft_outreach".to_string(),
            description: "Signal that you are drafting an outreach message to a candidate. Use this when asked to write a recruiting email or message.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "candidate_name": { "type": "string", "description": "Name of the candidate" },
                    "role": { "type": "string", "description": "Position being recruited for" }
                },
                "required": ["candidate_name", "role"]
            }),
        },
        ToolDefinition {
            name: "draft_interview_questions".to_string(),
            description: "Signal that you are drafting interview questions. Use this when asked to prepare interview questions for a role.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "role": { "type": "string", "description": "The role to interview for" },
                    "focus_area": { "type": "string", "description": "Area of focus (technical, behavioral, leadership, culture-fit)", "enum": ["technical", "behavioral", "leadership", "culture-fit", "general"] }
                },
                "required": ["role"]
            }),
        },
    ]
}

fn build_system_prompt() -> String {
    r#"You are JARVIS, an autonomous desktop assistant. You are helpful, intelligent, and proactive.

You have access to tools that let you take real actions on the user's Mac. Use them when the user asks you to do something that requires system interaction. When you use tools, the results will be shown to you so you can respond intelligently.

Guidelines:
- When asked to open or launch an app, use the launch_app tool
- When asked to open a URL, use the open_url tool
- When asked to open a file, use the open_file tool
- When asked about running apps, use the list_running_apps tool
- When the user says "remember this" or "save this", use the save_memory tool
- When asked to draft a job description, use the draft_job_description tool and then write the JD in your response
- When asked to draft outreach to a candidate, use the draft_outreach tool and then write the message
- When asked to draft interview questions, use the draft_interview_questions tool and then write the questions
- For questions and conversations, respond directly without tools
- Be concise and efficient — think like an executive assistant
- If a tool fails, explain what happened and suggest alternatives

Recruiting capabilities:
- Draft job descriptions for any role and level
- Write candidate outreach messages (emails, LinkedIn)
- Generate interview question packs (technical, behavioral, leadership)
- Help compare candidates and assess fit
- All recruiting context is saved in memory for future reference"#.to_string()
}