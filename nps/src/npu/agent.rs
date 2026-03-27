use crate::models;
use crate::protos;
use crate::core;
use crate::aibot;
use crate::npu::user::types::UserInfo;
use super::{ApiResponse,JsonResponse};
use crate::unwrap_err;

use poem::web::websocket::{WebSocket,Message};
use poem::web::{Path,Query,Data,Multipart};
use poem::{
    Result,
    Error,
    handler,
    FromRequest,
    OnUpgrade,
    Response,
    Request,
    IntoResponse,
    http::StatusCode,
    session::Session,
};
use poem_openapi::{
    OpenApi, 
    Multipart as ApiMultipart,
    payload::{Json,Attachment,AttachmentType}
};
use prost::Message as PbMessage;
use sea_orm::{EntityTrait,ActiveModelTrait,Set};
use futures::{SinkExt, StreamExt, Stream};
use tokio::io::{AsyncReadExt,AsyncWriteExt};

use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};
use std::io::Write;

pub mod types {
    use serde::{Deserialize, Serialize};
    use poem_openapi::Object;

    #[derive(Debug, Object, Deserialize, Default, Serialize)]
    pub struct AgentAction {
        pub id: String,
    }

    #[derive(Debug, Object, Deserialize, Default, Serialize)]
    pub struct AgentData {
        pub id: String,
        pub data: String,
    }

    #[derive(Debug, Object, Deserialize, Default, Serialize)]
    pub struct ChatData {
        pub taskid: String,
        pub id: String,
        pub data: String,
    }

    #[derive(Debug, Object, Deserialize, Default, Serialize)]
    pub struct FAction {
        pub id: String,
        pub path: String,
    }

    #[derive(Debug, Object, Serialize, Default, Deserialize)]
    pub struct DirEntry {
        pub file_name: String,
        pub file_type: String,
        pub permissions: String,
        pub size: u64,
        pub user: String,
        pub group: String,
        pub modified: String,
    }

    #[derive(Debug, Object, Serialize, Default, Deserialize)]
    pub struct DirDetail {
        pub current_dir: String,
        pub entrys: Vec<DirEntry>,
    }
}



pub struct AgentApi;

#[OpenApi]
impl AgentApi {

    #[oai(path = "/agentget", method = "get")]
    pub async fn agentget(
        &self,
        app: Data<&core::AppState>,
    ) -> JsonResponse<Vec<protos::nps::Agent>> {

        let mut response = ApiResponse::default();
        response.with_data(app.get_agents().await);
        Json(response)
    }

    #[oai(path = "/agentdel", method = "post")]
    pub async fn agentdel(
        &self,
        app: Data<&core::AppState>,
        Json(req): Json<types::AgentAction>,
    ) -> JsonResponse<Vec<protos::nps::Agent>> {

        let mut response = ApiResponse::default();
        app.remove_agent(&req.id).await;
        response.with_data(app.get_agents().await);
        Json(response)
    }

    #[oai(path = "/npc2stop", method = "post")]
    pub async fn npc2stop(
        &self,
        app: Data<&core::AppState>,
        Json(req): Json<types::AgentAction>,
    ) -> JsonResponse<Vec<protos::nps::Agent>> {

        let mut response = ApiResponse::default();
        app.remove_agent2(&req.id).await;
        Json(response)
    }

    #[oai(path = "/agentnote", method = "post")]
    pub async fn agentnote(
        &self,
        sess: &Session,
        app: Data<&core::AppState>,
        Json(req): Json<types::AgentData>,
    ) -> JsonResponse<String> {

        let mut response = ApiResponse::default();
        let _ = app.agent_note(&req.id, req.data).await;
        response.with_msg("ok");
        Json(response)
    }

    #[oai(path = "/addevent", method = "post")]
    pub async fn addevent(
        &self,
        app: Data<&core::AppState>,
        Json(req): Json<types::AgentData>,
    ) -> Result<JsonResponse<String>> {
        let mut response = ApiResponse::default();
        let data = unwrap_err!(utils::b64decode(&req.data));
        let mut cursor = std::io::Cursor::new(data);
        let mut event = unwrap_err!(protos::nps::AgentEvent::decode(&mut cursor));
        app.add_event(event).await;
        Ok(Json(response))
    }

    #[oai(path = "/agentaichat", method = "post")]
    pub async fn agentaichat(
        &self,
        app: Data<&core::AppState>,
        Json(req): Json<types::AgentData>,
    ) -> Result<JsonResponse<String>> {
        let mut response = ApiResponse::default();

        let data = aibot::run_app(&app, &req.id, &req.data).await?;
        response.with_data(data);

        Ok(Json(response))
    }

    #[oai(path = "/agentinfo", method = "post")] 
    pub async fn agentinfo(
        &self,
        app: Data<&core::AppState>,
        Json(req): Json<types::AgentAction>,
    ) -> Result<JsonResponse<serde_json::Value>> {

        let mut response = ApiResponse::default();
        if let Some(ret) = app.get_agent(&req.id).await {
            let mut info = serde_json::json!({
                "id"       : ret.npc1.id,
                "target"   : ret.npc1.target,
                "intranet" : ret.npc1.intranet,
                "username" : ret.npc1.username,
                "hostname" : ret.npc1.hostname,
                "platform" : ret.npc1.platform,
                "arch"     : ret.npc1.arch,
                "process"  : ret.npc1.process,
                "pid"      : ret.npc1.pid,
                "internet" : ret.npc1.internet,
                "note"     : ret.npc1.note,
                "firsteat" : ret.npc1.updateat,
                "updateat" : utils::timestamp(),
                "ntype"    : ret.ntype,
                "npc2"     : ret.npc2.is_some(),
            });
            response.with_data(info);
        }
        Ok(Json(response))
    }
 
    #[oai(path = "/pluglist", method = "post")]
    pub async fn pluglist(
        &self,
        app: Data<&core::AppState>,
    ) -> Result<JsonResponse<serde_json::Value>> {
        let mut response = ApiResponse::default();
        let mut items = Vec::new();

        if let Ok(mut entries) = tokio::fs::read_dir("./plugins").await {
            while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
                if let Ok(file_type) = entry.file_type().await {
                    if file_type.is_file() {
                        if let Some(name) = entry.file_name().to_str() {
                            items.push(serde_json::json!({
                                "name":     name.to_string(),
                                "entry":    "".to_string(),
                                "args":     "".to_string(),
                                "arch":     "".to_string(),
                            }));
                        }
                    }
                }
            }
        }

        response.with_data(serde_json::json!({
            "items": items
        }));
        Ok(Json(response))
    }

    #[oai(path = "/sftp/upload/:aid", method = "post")]
    pub async fn sftp_upload(
        &self, 
        app: Data<&core::AppState>,
        Path(aid): Path<String>,
        mut multipart: Multipart
    ) -> Result<JsonResponse<String>> {
        log::info!("upload aid {}", aid);

        let rssh = core::RsshSession::new(app.clone());
        let sftp = unwrap_err!(rssh.sftp(&aid).await);
        
        while let Some(field) = multipart.next_field().await? {
            let path = field.file_name().map(|s| s.to_string()).unwrap_or_default();
            let data = field.bytes().await?;

            let dir = std::path::Path::new(&path)
                .parent()
                .map(|v| v.to_str())
                .flatten();

            if let Some(dir) = dir {
                if let Ok(is_exists) = sftp.try_exists(dir).await{
                    if !is_exists {
                        unwrap_err!(sftp.create_dir(dir).await);
                    }
                }
            }

            let mut file = unwrap_err!(sftp.create(path).await);
            unwrap_err!(file.write_all(&data).await);
        }

        let mut response = ApiResponse::default();
        Ok(Json(response))
    }

    #[oai(path = "/sftp/read_file/:aid", method = "post")]
    pub async fn sftp_read_file(
        &self,
        app: Data<&core::AppState>,
        Path(aid): Path<String>,
        Json(req): Json<types::FAction>,
    ) -> Result<JsonResponse<String>> {

        let rssh = core::RsshSession::new(app.clone());
        let sftp = unwrap_err!(rssh.sftp(&aid).await);

        let data = unwrap_err!(sftp.read(&req.path).await);
        
        let name = std::path::Path::new(&req.path)
            .file_name()
            .map(|v| v.to_str())
            .flatten()
            .map_or("download.tmp".to_string(), |v| v.to_owned());

        let fid = utils::uuid();
        let file_path = std::path::Path::new("./data").join(&fid);
        if let Ok(mut file) = std::fs::File::create(file_path){
            if let Ok(_) = file.write_all(&data){
                let mut active_model = models::fileresult::ActiveModel{
                    id :        Set(fid),
                    fileauth:   Set(utils::uuid()),
                    filepath:   Set(req.path),
                    filename:   Set(name.clone()),
                    filesize:   Set(data.len() as i64),
                    .. Default::default()
                };
                if let Some(agent) = app.get_agent(&aid).await{
                    active_model.agent = Set(format!("{}@{}",agent.npc1.username,agent.npc1.intranet));
                }
                let _ = active_model.insert(&app.conn).await;
            }
        }
        let mut response = ApiResponse::default();
        response.with_msg("ok");
        Ok(Json(response))
    }


    #[oai(path = "/sftp/download", method = "get")]
    pub async fn sftp_download(
        &self,
        app: Data<&core::AppState>,
        Query(req): Query<types::FAction>, 
    ) -> Result<Attachment<Vec<u8>>> {

        let rssh = core::RsshSession::new(app.clone());
        let sftp = rssh.sftp(&req.id).await.map_err(|e| {
            Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
        })?;

        let data = sftp.read(&req.path).await.map_err(|e| {
            Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
        })?;
        
        let name = std::path::Path::new(&req.path)
            .file_name()
            .map(|v| v.to_str())
            .flatten()
            .map_or("download.tmp".to_string(), |v| v.to_owned());

        let fid = utils::uuid();
        let file_path = std::path::Path::new("./data").join(&fid);
        if let Ok(mut file) = std::fs::File::create(file_path){
            if let Ok(_) = file.write_all(&data){
 
                let mut active_model = models::fileresult::ActiveModel{
                    id :        Set(fid),
                    fileauth:   Set(utils::uuid()),
                    filepath:   Set(req.path),
                    filename:   Set(name.clone()),
                    filesize:   Set(data.len() as i64),
                    .. Default::default()
                };
                if let Some(agent) = app.get_agent(&req.id).await{
                    active_model.agent = Set(format!("{}@{}",agent.npc1.username,agent.npc1.intranet));
                }

                let _ = active_model.insert(&app.conn).await;
            }
        }

        let mut attachment = Attachment::<Vec<u8>>::new(data).attachment_type(AttachmentType::Attachment);
        attachment = attachment.filename(name);
        Ok(attachment)
    }

    #[oai(path = "/sftp/read_dir/:aid", method = "post")]
    pub async fn sftp_read_dir(
        &self,
        app: Data<&core::AppState>,
        Path(aid): Path<String>,
        Json(req): Json<types::FAction>,
    ) -> Result<JsonResponse<types::DirDetail>> {

        let rssh = core::RsshSession::new(app.clone());
        let sftp = unwrap_err!(rssh.sftp(&aid).await);

        let current_dir = unwrap_err!(sftp.canonicalize(&req.path).await);
        let dir = unwrap_err!(sftp.read_dir(&req.path).await);

        let mut entrys = Vec::new();
        for entry in dir {
            let meta = entry.metadata();
            let permissions = format!("{}", meta.permissions());
            let file_type = format!("{:?}", entry.file_type()).to_string();
            let file_name = entry.file_name();
            let modified = DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_secs(meta.mtime.unwrap_or(0) as u64))
                .with_timezone(&chrono::Local)
                .naive_local()
                .to_string();
            let user = if let Some(user) = &meta.user {
                user.to_string()
            } else {
                meta.uid.unwrap_or(0).to_string()
            };

            let group = if let Some(user) = &meta.group {
                user.to_string()
            } else {
                meta.gid.unwrap_or(0).to_string()
            };
            let size = meta.size.unwrap_or(0);

            entrys.push(types::DirEntry {
                file_name,
                file_type,
                permissions,
                modified,
                size,
                user,
                group,
            })
        }

        let mut result = types::DirDetail {
            current_dir,
            entrys,
        };

        let mut response = ApiResponse::default();
        response.with_data(result);
        Ok(Json(response))
    }

    #[oai(path = "/sftp/remove/:aid", method = "post")]
    pub async fn sftp_remove(
        &self,
        app: Data<&core::AppState>,
        Path(aid): Path<String>,
        Json(req): Json<types::FAction>,
    ) -> Result<JsonResponse<String>> {
        
        let rssh = core::RsshSession::new(app.clone());
        let sftp = unwrap_err!(rssh.sftp(&aid).await);

        unwrap_err!(if req.id == "Dir" {
            sftp.remove_dir(req.path).await
        } else {
            sftp.remove_file(req.path).await
        });

        let mut response = ApiResponse::default();
        response.with_msg("ok");
        Ok(Json(response))
    }

    #[oai(path = "/sftp/rename/:aid", method = "post")]
    pub async fn sftp_rename(
        &self,
        app: Data<&core::AppState>,
        Path(aid): Path<String>,
        Json(req): Json<types::FAction>,
    ) -> Result<JsonResponse<String>> {

        let rssh = core::RsshSession::new(app.clone());
        let sftp = unwrap_err!(rssh.sftp(&aid).await);
        unwrap_err!(sftp.rename(req.id,req.path).await);

        let mut response = ApiResponse::default();
        response.with_msg("ok");
        Ok(Json(response))
    }
        
}
