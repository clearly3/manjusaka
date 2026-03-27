
mod state;
mod rssh;
mod srdi;

use crate::protos;
use crate::bots;
use crate::models;

pub use state::AppState;
pub use rssh::{RsshClient,RsshSession};
use tokio::time::{interval, sleep, Duration};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn start(app: AppState) {
	let app1 = app.clone();

	let qq_app_id = models::settings::get(&app.conn,"qq.app_id","").await;
	let qq_app_secret = models::settings::get(&app.conn,"qq.app_secret","").await;
	let weixin_app_id = models::settings::get(&app.conn,"weixin.app_id","").await;
	let weixin_app_secret = models::settings::get(&app.conn,"weixin.app_secret","").await;

	tokio::spawn(async move{
		let token = botrs::Token::new(qq_app_id, qq_app_secret);
	    let intents = botrs::Intents::default()
	        .with_public_guild_messages()  // 接收 @ 提及
	        .with_public_messages();
	    let qq = bots::QQBot::new(app1);   
	    if let Ok(mut client) = botrs::Client::new(token, intents, qq, true){
	    	if let Err(e) = client.start().await{
	    		log::error!("MyQQBot start err {}",e);
	    	}
	    }
	});

	tokio::spawn(async move{
		let wait: i64 = 50;
		let mut monitor = interval(Duration::from_secs(wait as u64));
		loop {
			monitor.tick().await;
			let mut agents = Vec::new();
			if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH){
                for agent in app.get_agents().await {
					if now.as_secs() as i64 - agent.updateat < wait {
						agents.push(agent);
					}
				}
            }
			
			if !agents.is_empty() {
				let mut agentlist = protos::nps::AgentList::default();
	            agentlist.status = "update".to_string();
	            agentlist.agents = agents;
	            let mut agentevent = protos::nps::AgentEvent::default();
	            agentevent.enumof =  Some(protos::nps::agent_event::Enumof::List(agentlist));
				let _ = app.broadcast(agentevent).await;
			}
		}
	});

}