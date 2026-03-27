
use super::{RPC_TIMEOUT_SECS, ToolAct, rpc_exec};
use super::{Tool, ToolResult};

use crate::core::AppState;

pub struct GetListTool {
    app: AppState,
}

impl GetListTool {
    pub fn new(app: AppState) -> Self {
        Self {app}
    }
}

#[async_trait::async_trait]
impl Tool for GetListTool {
    fn name(&self) -> &str {
        "get_list"
    }

    fn description(&self) -> &str {
        "获取主机列表，以 序号，id、用户名、主机名、主机ip、构架、进程 为列做成表格形式，不要添加其它无关内容"
    }

    /// 获取工具参数模式
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }


    async fn execute(&self, _aid: &str, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        
        let data: Vec<serde_json::Value> = self.app.get_agents().await.iter().map(|a|{
            serde_json::json!({
            "id": a.id,
            "username": a.username,
            "hostname": a.hostname,
            "ip": format!("内网IP-{} 外网IP-{}", a.intranet,a.internet),
            "构架": format!("{}.{}",a.platform,a.arch),
            "进程": format!("PID:[{}]-{}",a.pid,a.process),
            "是否在线": if (utils::timestamp()-a.updateat) > 100 {"不在线"}else{"在线"},
            "npc2": a.npc2,
        })
        }).collect();

        Ok(ToolResult {
            success: true,
            output: format!("{:?}",data),
            error: None,
        })
    }
}

