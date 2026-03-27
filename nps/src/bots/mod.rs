pub mod qq;

pub use qq::QQBot;

#[async_trait::async_trait]
pub trait Bot: Send + Sync {
    fn name(&self) -> &str;
    async fn send_msg(&self, msg: &str, tid: &str) -> anyhow::Result<()>;
}