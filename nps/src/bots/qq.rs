use super::Bot;

use crate::core;
use crate::aibot;

pub struct QQBot{
	pub app: core::AppState,
}

impl QQBot {
    pub fn new(app: core::AppState) -> Self {
        Self {app}
    }
}

#[async_trait::async_trait]
impl Bot for QQBot {
    fn name(&self) -> &str {
        "qqbot"
    }

    async fn send_msg(&self, msg:&str, tid:&str)-> anyhow::Result<()>{
        //...
        Ok(())
    }
}

#[async_trait::async_trait]
impl botrs::EventHandler for QQBot {
    async fn ready(&self, _ctx: botrs::Context, ready: botrs::Ready) {
        log::info!("QQ机器人已就绪！登录为：{}", ready.user.username);
    }

    async fn message_create(&self, ctx: botrs::Context, message: botrs::Message) {
    	log::info!("message_create {:?}",message.content);
    }

    async fn c2c_message_create(&self, ctx: botrs::Context, message: botrs::C2CMessage) {
    	log::info!("c2c_message_create {:?}",message.content);

    	if let Some(ref text) = message.content {
            if let Ok(res) = aibot::run_app(&self.app,"npc2id",&text).await{

                log::info!("c2c_message_reply {}",res);
        		if let Err(e) = message.reply(&ctx.api, &ctx.token, &res).await{
                    log::error!("c2c_message_reply {}",e);
                }
            }
    	}
    }

    async fn group_message_create(&self, ctx: botrs::Context, message: botrs::GroupMessage) {
        log::info!("group_message_create {:?}",message.content);
    }

}

