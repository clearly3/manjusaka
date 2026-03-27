
use super::{RPC_TIMEOUT_SECS, ToolAct, rpc_exec_timeout};
use super::{Tool, ToolResult};

use crate::core::{AppState,RsshSession};


pub struct HttpGetTool {
    app: AppState,
}

impl HttpGetTool {
    pub fn new(app: AppState) -> Self {
        Self {app}
    }
}

#[async_trait::async_trait]
impl Tool for HttpGetTool {
    fn name(&self) -> &str {
        "http_get"
    }

    fn description(&self) -> &str {
        "访问获取对应url并返回里面的内容"
    }

    /// 获取工具参数模式
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "需要访问的 url 地址"
                }
            },
            "required": ["url"]
        })
    }

    /// 执行工具
    async fn execute(&self, aid: &str, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' parameter"))?;

        let mut req_act = ToolAct::default();
        req_act.act = "http_get".to_string();
        req_act.args = args.to_string();

        let rssh = RsshSession::new(self.app.clone());
        let channel = rssh.agent(aid).await?;
        let mut stream = channel.into_stream();

        match rpc_exec_timeout(&mut stream, req_act).await {
            Ok(res_act) => Ok(ToolResult {
                success: res_act.act.as_str() == "true",
                output: String::from_utf8_lossy(&res_act.data).to_string(),
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

