use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{sleep, Duration};

// ========== 消息结构（与 Channel trait 兼容） ==========
#[derive(Debug, Clone)]
pub struct ChannelMessage {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub channel: String,  // 格式 "user:user_id"
    pub timestamp: u64,
}

// ========== 微信协议数据结构 ==========

// 获取二维码响应
#[derive(Debug, Deserialize)]
struct QrCodeResponse {
    qrcode: String,
    qrcode_img_content: String,
}

// 二维码状态响应
#[derive(Debug, Deserialize)]
struct QrCodeStatusResponse {
    status: String,
    bot_token: Option<String>,
    ilink_bot_id: Option<String>,
    ilink_user_id: Option<String>,
    baseurl: Option<String>,
}

// 消息列表响应（getupdates）
#[derive(Debug, Deserialize)]
struct GetUpdatesResponse {
    msgs: Vec<ReceivedMessage>,
    get_updates_buf: String,
    longpolling_timeout_ms: u64,
}

// 接收到的单条消息
#[derive(Debug, Deserialize)]
struct ReceivedMessage {
    from_user_id: String,
    message_id: String,
    seq: u64,
    create_time_ms: u64,
    item_list: Vec<MessageItem>,
    context_token: String,
}

// 消息项（文本/语音等）
#[derive(Debug, Deserialize)]
struct MessageItem {
    #[serde(rename = "type")]
    typ: u32,
    text_item: Option<TextItem>,
    voice_item: Option<VoiceItem>,
}

#[derive(Debug, Deserialize)]
struct TextItem {
    text: String,
}

#[derive(Debug, Deserialize)]
struct VoiceItem {
    text: Option<String>, // 语音转文本
}

// 发送消息请求体
#[derive(Debug, Serialize)]
struct SendMessageRequest {
    msg: SendMessageBody,
    base_info: BaseInfo,
}

#[derive(Debug, Serialize)]
struct SendMessageBody {
    from_user_id: String,
    to_user_id: String,
    client_id: String,
    message_type: u32,
    message_state: u32,
    item_list: Vec<SendMessageItem>,
    context_token: String,
}

#[derive(Debug, Serialize)]
struct SendMessageItem {
    #[serde(rename = "type")]
    typ: u32,
    text_item: Option<SendTextItem>,
}

#[derive(Debug, Serialize)]
struct SendTextItem {
    text: String,
}

#[derive(Debug, Serialize)]
struct BaseInfo {
    channel_version: String,
}

// 发送消息响应
#[derive(Debug, Deserialize)]
struct SendMessageResponse {
    ret: i32,
    errcode: i32,
    errmsg: Option<String>,
}

// ========== 微信机器人实现 ==========
pub struct WeixinBot {
    bot_token: String,
    ilink_bot_id: String,
    ilink_user_id: String,
    baseurl: String,
    http_client: reqwest::Client,
    updates_buf: Arc<Mutex<String>>, // 游标，用于长轮询
}

impl WeixinBot {
    /// 创建并完成登录绑定
    pub async fn new(bot_type: u32) -> anyhow::Result<Self> {
        let client = reqwest::Client::new();
        let default_baseurl = "https://ilinkai.weixin.qq.com".to_string();

        // 1. 获取二维码
        let qr_url = format!("{}/ilink/bot/get_bot_qrcode?bot_type={}", default_baseurl, bot_type);
        let qr_resp = client.get(&qr_url).send().await?;
        if !qr_resp.status().is_success() {
            anyhow::bail!("获取二维码失败: {}", qr_resp.status());
        }
        let qr: QrCodeResponse = qr_resp.json().await?;

        println!("请扫描二维码: {}", qr.qrcode_img_content);
        // 2. 轮询二维码状态
        let status_url = format!("{}/ilink/bot/get_qrcode_status?qrcode={}", default_baseurl, qr.qrcode);
        let mut interval = Duration::from_secs(2);
        let mut attempts = 0;
        let max_attempts = 300; // 最多等10分钟
        let (bot_token, ilink_bot_id, ilink_user_id, baseurl) = loop {
            if attempts >= max_attempts {
                anyhow::bail!("二维码超时");
            }
            let resp = client.get(&status_url).send().await?;
            let status: QrCodeStatusResponse = resp.json().await?;
            match status.status.as_str() {
                "confirmed" => {
                    let token = status.bot_token.ok_or_else(|| anyhow::anyhow!("缺少 bot_token"))?;
                    let bot_id = status.ilink_bot_id.ok_or_else(|| anyhow::anyhow!("缺少 ilink_bot_id"))?;
                    let user_id = status.ilink_user_id.ok_or_else(|| anyhow::anyhow!("缺少 ilink_user_id"))?;
                    let base = status.baseurl.unwrap_or(default_baseurl.clone());
                    break (token, bot_id, user_id, base);
                }
                "expired" => anyhow::bail!("二维码已过期"),
                "wait" | "scaned" => {
                    attempts += 1;
                    sleep(interval).await;
                    // 动态增加等待时间，但不超过5秒
                    interval = (interval * 2).min(Duration::from_secs(5));
                }
                _ => anyhow::bail!("未知状态: {}", status.status),
            }
        };

        println!("登录成功！bot_token: {}, baseurl: {}", bot_token, baseurl);

        Ok(Self {
            bot_token,
            ilink_bot_id,
            ilink_user_id,
            baseurl,
            http_client: client,
            updates_buf: Arc::new(Mutex::new(String::new())),
        })
    }

    /// 生成 X-WECHAT-UIN 头
    fn generate_uin_header(&self) -> String {
        let rnd: u32 = rand::thread_rng().gen();
        let s = rnd.to_string();
        BASE64.encode(s.as_bytes())
    }

    /// 生成唯一的 client_id
    fn generate_client_id(&self) -> String {
        let ts = Utc::now().timestamp_millis();
        let rnd: u64 = rand::thread_rng().gen();
        format!("openclaw-weixin:{}-{}", ts, rnd)
    }

    /// 从收到的消息中提取文本内容
    fn extract_text_from_message(msg: &ReceivedMessage) -> String {
        for item in &msg.item_list {
            match item.typ {
                1 => {
                    if let Some(text_item) = &item.text_item {
                        return text_item.text.clone();
                    }
                }
                3 => {
                    if let Some(voice_item) = &item.voice_item {
                        if let Some(text) = &voice_item.text {
                            return text.clone();
                        }
                    }
                }
                _ => {}
            }
        }
        String::new()
    }

    /// 发送消息（实际执行）
    async fn send_message_impl(&self, to_user_id: &str, text: &str, context_token: &str) -> anyhow::Result<()> {
        let url = format!("{}/ilink/bot/sendmessage", self.baseurl);
        let client_id = self.generate_client_id();
        let body = SendMessageRequest {
            msg: SendMessageBody {
                from_user_id: self.ilink_bot_id.clone(),
                to_user_id: to_user_id.to_string(),
                client_id,
                message_type: 2,
                message_state: 2,
                item_list: vec![SendMessageItem {
                    typ: 1,
                    text_item: Some(SendTextItem {
                        text: text.to_string(),
                    }),
                }],
                context_token: context_token.to_string(),
            },
            base_info: BaseInfo {
                channel_version: "1.0.0".to_string(),
            },
        };

        let uin_header = self.generate_uin_header();
        let resp = self.http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("AuthorizationType", "ilink_bot_token")
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .header("X-WECHAT-UIN", uin_header)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("HTTP 错误: {}", resp.status());
        }

        let send_resp: SendMessageResponse = resp.json().await?;
        if send_resp.ret != 0 || send_resp.errcode != 0 {
            anyhow::bail!("发送消息失败: ret={}, errcode={}, msg={:?}", send_resp.ret, send_resp.errcode, send_resp.errmsg);
        }
        Ok(())
    }

    /// 长轮询拉取消息并处理（通过 tx 发送）
    async fn poll_updates(&self, tx: &mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        let url = format!("{}/ilink/bot/getupdates", self.baseurl);
        let mut buf = self.updates_buf.lock().await.clone();

        loop {
            let uin_header = self.generate_uin_header();
            let body = serde_json::json!({
                "get_updates_buf": buf,
                "base_info": {
                    "channel_version": "1.0.0"
                }
            });

            let resp = self.http_client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("AuthorizationType", "ilink_bot_token")
                .header("Authorization", format!("Bearer {}", self.bot_token))
                .header("X-WECHAT-UIN", uin_header)
                .json(&body)
                .send()
                .await?;

            if !resp.status().is_success() {
                eprintln!("拉取消息失败: {}", resp.status());
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            let updates: GetUpdatesResponse = match resp.json().await {
                Ok(up) => up,
                Err(e) => {
                    eprintln!("解析响应失败: {}", e);
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            // 更新游标
            buf = updates.get_updates_buf;
            *self.updates_buf.lock().await = buf.clone();

            // 处理每条消息
            for msg in updates.msgs {
                let content = Self::extract_text_from_message(&msg);
                if content.is_empty() {
                    continue;
                }
                let channel_msg = ChannelMessage {
                    id: msg.message_id,
                    sender: msg.from_user_id.clone(),
                    content,
                    channel: format!("user:{}", msg.from_user_id),
                    timestamp: msg.create_time_ms / 1000, // 转换为秒
                };
                if let Err(e) = tx.send(channel_msg).await {
                    eprintln!("发送消息到通道失败: {}", e);
                    break;
                }
            }

            // 等待服务端建议的时间后再轮询
            let wait_ms = updates.longpolling_timeout_ms.max(1000); // 至少1秒
            sleep(Duration::from_millis(wait_ms)).await;
        }
    }
}

#[async_trait]
impl Channel for WeixinBot {
    fn name(&self) -> &str {
        "weixin"
    }

    async fn send(&self, message: &str, recipient: &str) -> anyhow::Result<()> {
        // recipient 格式: "user:用户ID"
        let parts: Vec<&str> = recipient.splitn(2, ':').collect();
        if parts.len() != 2 {
            anyhow::bail!("recipient 格式错误，应为 'user:用户ID'");
        }
        if parts[0] != "user" {
            anyhow::bail!("不支持的 recipient 类型: {}", parts[0]);
        }
        let to_user_id = parts[1];

        // 注意：发送消息需要 context_token，但这里没有。我们需要一个存储 context_token 的机制。
        // 简单起见，我们可以在消息接收时记录每个用户的最后一个 context_token，但这里无法获取。
        // 为了能够回复，我们在接收到消息时会将 context_token 附加到 ChannelMessage 中，或者通过其他方式。
        // 但 Channel trait 没有提供 context_token，所以这里我们无法直接发送。
        // 解决方案：修改 ChannelMessage 包含 context_token，或者让 WeixinBot 内部维护一个映射。
        // 为了不破坏接口，我们暂时返回错误，实际中需要在 listen 中处理回复。
        anyhow::bail!("微信发送需要 context_token，请使用 listen 中的消息对象直接回复")
    }

    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        // 启动长轮询任务
        self.poll_updates(&tx).await?;
        Ok(())
    }
}