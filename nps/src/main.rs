#![allow(unused_imports, unused_mut, unused_variables, dead_code)]

mod protos;
mod models;
mod npu;
mod npc;
mod core;
mod macros;
mod aibot;
mod bots;

#[tokio::main]
async fn main() {

    dir_init();

    tracing_subscriber::fmt::init();

    let state = core::AppState::connect("sqlite:nps.db?mode=rwc").await.unwrap();

    models::init_database(&state.conn).await.unwrap();

    state.init().await.unwrap();

    // 消息推送
    core::start(state.clone()).await; 

    npc::npc2::start(state.clone()).await.unwrap();

    let npcaddr = models::settings::get(&state.conn,"npc.listen","0.0.0.0:31000").await;
    npc::npc1::start(&npcaddr,state.clone()).await;

    let npsaddr = models::settings::get(&state.conn,"nps.listen","0.0.0.0:33000").await;
    npu::start(&npsaddr,state).await.unwrap();
    
}
 


fn dir_init() {
    let _ = std::fs::create_dir_all("./data");
    let _ = std::fs::create_dir_all("./payloads");
    let _ = std::fs::create_dir_all("./plugins");
}

