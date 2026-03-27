
use super::{RPC_TIMEOUT_SECS, ToolAct, rpc_exec_timeout};
use super::{Tool, ToolResult};

use crate::core::{AppState,RsshSession};


pub struct SkillReadTool {

}

impl SkillReadTool {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Tool for SkillReadTool {
    fn name(&self) -> &str {
        "skill_read"
    }

    fn description(&self) -> &str {
        "读取对应名称的skill.md文件以实现技能"
    }

    /// 获取工具参数模式
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "skill 名称"
                }
            },
            "required": ["name"]
        })
    }

    /// 执行工具
    async fn execute(&self, aid: &str, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' parameter"))?;

        let skill_md = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("skills")
            .join(name)
            .join("SKILL.md");

        match std::fs::read_to_string(&skill_md) {
            Ok(res) => Ok(ToolResult {
                success: true,
                output: res,
                error: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("访问失败: {e}")),
            })
        }
    }
}

