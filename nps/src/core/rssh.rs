use super::AppState;

use anyhow::Result;
use russh::client;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use std::net::SocketAddr;
use std::sync::Arc;

pub struct RsshClient {}

impl russh::client::Handler for RsshClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        log::debug!("Server public key algorithm: {}", server_public_key.algorithm());
        Ok(true)
    }

}


pub struct RsshSession {
    app: AppState
}

impl RsshSession {
    pub fn new(app: AppState) -> RsshSession {
        RsshSession { 
            app
        }
    }

    pub async fn channel(&self, id: &str) -> Result<russh::Channel<russh::client::Msg>> {
        let mut yamux_channel = self.app.get_agent2(id).await?;
        let yamux_stream = yamux_channel.open_stream().await?;
        let ssh = RsshClient{};
        let config = russh::client::Config{
            channel_buffer_size: u32::MAX as usize,
            .. Default::default()
        };
        let mut session = russh::client::connect_stream(Arc::new(config),yamux_stream,ssh).await?;
        let _ = session.authenticate_password("Hacker","manjusaka").await?;
        let channel = session.channel_open_session().await?;
        Ok(channel)
    }

    pub async fn sftp(&self, id: &str) -> Result<russh_sftp::client::SftpSession> {
        let mut channel = self.channel(id).await?;
        channel.request_subsystem(true, "sftp").await?;
        let sftp_session = russh_sftp::client::SftpSession::new(channel.into_stream()).await?;
        Ok(sftp_session)
    }

    pub async fn ssh(&self, id: &str) -> Result<russh::Channel<russh::client::Msg>> {
        let mut channel = self.channel(id).await?;
        let _ = channel.request_pty(false,"",400,400,0,0,&[]).await?;
        let _ = channel.request_shell(true).await?;
        Ok(channel)
    }

    pub async fn vnc(&self, id: &str) -> Result<russh::Channel<russh::client::Msg>> {
        let mut channel = self.channel(id).await?;
        let _ = channel.request_subsystem(true, "vnc").await?;
        Ok(channel)
    }

    pub async fn rdp(&self, id: &str) -> Result<russh::Channel<russh::client::Msg>> {
        let mut channel = self.channel(id).await?;
        let _ = channel.request_subsystem(true, "rdp").await?;
        Ok(channel)
    }

    pub async fn agent(&self, id: &str) -> Result<russh::Channel<russh::client::Msg>> {
        let mut channel = self.channel(id).await?;
        let _ = channel.request_subsystem(true, "agent").await?;
        Ok(channel)
    }

    pub async fn proxy(
        &self, 
        id: &str, 
        port: u32, 
        remote_addr: Option<SocketAddr>, 
        username: String, 
        password: String
    ) -> Result<oneshot::Sender<()>> {

        let mut yamux_channel = self.app.get_agent2(id).await?;
        let listener = TcpListener::bind(format!("0.0.0.0:{}",port)).await?;
        let (tx, mut rx) = oneshot::channel();

        tokio::spawn(async move{
            loop {
                tokio::select! { 
                    result = listener.accept() => {
                        if let Ok((mut inbound, client_addr)) = result {
                            if let Ok(yamux_stream) = yamux_channel.open_stream().await{
                                let ssh = RsshClient{};
                                let config = russh::client::Config{
                                    channel_buffer_size: u32::MAX as usize,
                                    .. Default::default()
                                };
                                if let Ok(mut session) = russh::client::connect_stream(
                                    Arc::new(config),
                                    yamux_stream,
                                    ssh
                                ).await {
                                    if let Err(e) = session.authenticate_password(&username, &password).await{
                                        log::error!("ssh authenticate_password err {}",e);
                                        continue;
                                    }
                                    let channel = if let Some(addr) = remote_addr {
                                        let channel = session.channel_open_direct_tcpip(
                                                addr.ip().to_string(),
                                                addr.port() as u32,
                                                client_addr.ip().to_string(),
                                                client_addr.port() as u32
                                        ).await.unwrap();
                                        channel
                                    }else{
                                        let channel = session.channel_open_session().await.unwrap();
                                        let _ = channel.request_subsystem(true, "socks5").await.unwrap();
                                        channel
                                    };
                                    let mut outbound = channel.into_stream();
                                    tokio::spawn(async move{
                                        let _ = tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await;
                                    });
                                }
                            }
                        }
                    },
                    _ = &mut rx => {
                        break
                    },
                }
            }
        });
        Ok(tx)
    }

}
        