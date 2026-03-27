
mod tools;
mod memory;
mod openai;
mod skills;
mod utils;

use openai::Provider;
use memory::Memory;
use crate::core;
use crate::models;

use std::sync::Arc;
use std::fmt::Write;

async fn build_context(mem: &dyn memory::Memory, user_msg: &str) -> String {
    let mut context = String::new();
    if let Ok(entries) = mem.recall(user_msg, 5).await {
        if !entries.is_empty() {
            context.push_str("## 相关记忆\n\n");
            for entry in entries {
                let _ = writeln!(context, "- {}", entry.content);
            }
            context.push('\n');
        }
    }
    context
}


pub fn build_system_prompt(
    tools: &[(&str, &str)],
    skills: &[skills::Skill],
) -> String {
    let mut prompt = String::with_capacity(8192);
    prompt.push_str("你是牛屎虾，一个c2的AI助手。用简洁直接的语句回答我不要废话。\n\n");
    if !tools.is_empty() {
        prompt.push_str("你可以使用以下工具：\n\n");
        for (name, desc) in tools {
            let _ = writeln!(prompt, "- **{name}**: {desc}");
        }
        prompt.push_str("\n当需要使用工具时，请直接调用工具接口。不要在文本中描述工具调用，而是直接调用它们。\n\n");
    }

    if !skills.is_empty() {
        prompt.push_str("技能按需加载。使用 `skill_read` 工具读取技能路径以获取完整说明。\n\n");
        prompt.push_str("<available_skills>\n");
        for skill in skills {
            let _ = writeln!(prompt, "<skill>");
            let _ = writeln!(prompt, "<name>{}</name>", skill.name);
            let _ = writeln!(prompt, "<description>{}</description>",skill.description);
            let location = skill.location.clone().unwrap_or_else(|| {
                std::env::current_dir().unwrap_or(std::path::PathBuf::from("."))
                    .join("skills")
                    .join(&skill.name)
                    .join("SKILL.md")
            });
            let _ = writeln!(prompt, "<location>{}</location>", location.display());
            let _ = writeln!(prompt, "</skill>");
        }
        prompt.push_str("</available_skills>\n\n");
    }

    prompt
}

/// - 工具执行结果的消息列表
pub async fn execute_tool_calls(
    current_aid: &mut String,
    tool_calls: &[openai::ToolCall],
    tools: &[Box<dyn tools::Tool>],
) -> Vec<openai::ChatMessage> {
    let mut results = Vec::with_capacity(tool_calls.len());

    for tc in tool_calls {
        let tool_name = &tc.function.name;

        let tool = tools.iter().find(|t| t.name() == tool_name);
        let Some(tool) = tool else {
            results.push(openai::ChatMessage::Tool {
                tool_call_id: tc.id.clone(),
                content: format!("错误: 未知工具「{tool_name}」"),
            });
            continue;
        };

        // 解析参数
        let args: serde_json::Value = match serde_json::from_str(&tc.function.arguments) {
            Ok(v) => v,
            Err(e) => {
                results.push(openai::ChatMessage::Tool {
                    tool_call_id: tc.id.clone(),
                    content: format!("错误: 参数解析失败: {e}"),
                });
                continue;
            }
        };

        if tool_name == "set_id" { //current_aid 传递指针地址 所以需要单独处理
            let new_id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            *current_aid = new_id.to_string();
        }

        let tool_result = match tool.execute(current_aid, args.clone()).await {
            Ok(result) => {
                if result.success {
                    result.output.trim().to_string()
                } else {
                    format!("错误: {}", result.error.unwrap_or(result.output))
                }
            }
            Err(e) => {
                format!("错误: {e}")
            }
        };

        let preview = utils::truncate_with_ellipsis(&tool_result, 200);
        log::info!("ID [{current_aid}] Tool [{tool_name}] Args[{args}] Ret {preview}");

        results.push(openai::ChatMessage::Tool {
            tool_call_id: tc.id.clone(),
            content: tool_result,
        });
    }

    results
}


pub async fn run_tool_loop(
    current_aid: &mut String,
    provider: &openai::OpenAiProvider,
    history: &mut Vec<openai::ChatMessage>,
    tools: &[Box<dyn tools::Tool>],
    tool_definitions: &[openai::ToolDefinition],
    max_iterations: usize,
    temperature: f32,
) -> anyhow::Result<String> {
    for iteration in 0..max_iterations {
        let response = provider.chat_with_tools(history, tool_definitions, temperature as f64).await?;
        match response {
            openai::ToolChatResponse::Text(text) => {
                // 将助手的最终文本添加到历史记录中，以便后续调用可以看到
                history.push(openai::ChatMessage::Assistant {
                    content: Some(text.clone()),
                    tool_calls: None,
                });
                return Ok(text);
            }
            openai::ToolChatResponse::ToolUse {
                tool_calls,
                text: assistant_text,
            } => {
                // 添加助手的工具调用消息到历史记录
                history.push(openai::ChatMessage::Assistant {
                    content: assistant_text,
                    tool_calls: Some(tool_calls.clone()),
                });

                let tool_results = execute_tool_calls(current_aid, &tool_calls, tools).await;
                history.extend(tool_results);
            }
        }
    }

    history.push(openai::ChatMessage::User {
        content: "你已达到工具调用的最大迭代次数。请根据目前收集的信息，立即给出最终回答。".to_string(),
    });

    let final_response = provider.chat_with_tools(history, &[], temperature as f64).await?;
    match final_response {
        openai::ToolChatResponse::Text(text) => {
            history.push(openai::ChatMessage::Assistant {
                content: Some(text.clone()),
                tool_calls: None,
            });
            Ok(text)
        }
        openai::ToolChatResponse::ToolUse { text, .. } => {
            let final_text = text.unwrap_or_else(|| "在迭代次数限制内未能给出最终回答。".to_string());
            history.push(openai::ChatMessage::Assistant {
                content: Some(final_text.clone()),
                tool_calls: None,
            });
            Ok(final_text)
        }
    }
}

pub async fn run_app(
    app: &core::AppState,
    aid: &str,
    msg: &str
) -> anyhow::Result<String>{

    let base_url = models::settings::get(&app.conn,"ai.base_url","https://api.deepseek.com/v1/").await;
    let api_key = models::settings::get(&app.conn,"ai.api_key","sk-xxxxxxxxxxxxxxxxxxxxxxxx").await;
    let model_name = models::settings::get(&app.conn,"ai.model_name","deepseek-chat").await;

    let provider = openai::OpenAiProvider::new(&base_url,&api_key,&model_name);

    let mem = Arc::from(memory::SqliteMemory::new("ai.db")?);
    let tools = tools::all_tools(mem.clone(), app);

    let _ = mem.store("user_msg", &msg, memory::MemoryCategory::Conversation).await;
    let tool_descs: Vec<(&str, &str)> = tools
        .iter()
        .map(|t| {(t.name(), t.description())})
        .collect();
    let skills = skills::load_skills();
    let system_prompt = build_system_prompt(
        &tool_descs,
        &skills,
    );
    let tool_definitions: Vec<openai::ToolDefinition> = tools
        .iter()
        .map(|t: &Box<dyn tools::Tool>| openai::tool_spec_to_definition(&t.spec()))
        .collect();
    let context = build_context(mem.as_ref(), &msg).await;
    let enriched = if context.is_empty() {
        msg.to_string()
    } else {
        format!("{context}{msg}")
    };
    let mut history = vec![
        openai::ChatMessage::System {
            content: system_prompt.clone(),
        },
        openai::ChatMessage::User { 
            content: enriched 
        },
    ];
    let mut current_aid = aid.to_string();
    let response = run_tool_loop(
        &mut current_aid,
        &provider,
        &mut history,
        &tools,
        &tool_definitions,
        10, // max_iterations
        0.7 // temperature
    ).await?;
    log::debug!("{response}");

    let summary = utils::truncate_with_ellipsis(&response, 100);
    let _ = mem.store("assistant_resp", &summary, memory::MemoryCategory::Daily).await;
    Ok(response)
}

