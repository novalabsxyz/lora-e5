use crate::{Credentials, Mode, Region};
use crate::{Downlink, Error as LoraE5Error, JoinResponse, LoraE5};
use tokio::{
    sync::{mpsc, oneshot},
    task,
    time::Duration,
};

pub type Result<T = ()> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Request {
    At(String, Duration, oneshot::Sender<Result<String>>),
    Join(bool, oneshot::Sender<Result<JoinResponse>>),
    Configure(Credentials, oneshot::Sender<Result>),
    Shutdown,
    SendData(Vec<u8>, u8, bool, oneshot::Sender<Result<Option<Downlink>>>),
    SendAscii(String, u8, bool, oneshot::Sender<Result<Option<Downlink>>>),
}

pub struct Client {
    sender: mpsc::Sender<Request>,
}

impl Client {
    pub async fn at_command(&self, cmd: &str, timeout: Duration) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        let mut cmd = cmd.to_string();
        cmd.push('\n');
        self.sender.send(Request::At(cmd, timeout, tx)).await?;
        rx.await?
    }

    pub async fn join(&self, force: bool) -> Result<JoinResponse> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(Request::Join(force, tx)).await?;
        rx.await?
    }

    pub async fn configure(&self, credentials: Credentials) -> Result {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(Request::Configure(credentials, tx))
            .await?;
        rx.await?
    }

    pub async fn send(&self, data: Vec<u8>, port: u8, confirmed: bool) -> Result<Option<Downlink>> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(Request::SendData(data, port, confirmed, tx))
            .await?;
        rx.await?
    }
    pub async fn send_ascii(
        &self,
        data: String,
        port: u8,
        confirmed: bool,
    ) -> Result<Option<Downlink>> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(Request::SendAscii(data, port, confirmed, tx))
            .await?;
        rx.await?
    }

    pub async fn send_shutdown(&self) -> Result {
        Ok(self.sender.send(Request::Shutdown).await?)
    }
}

pub struct Setup {
    sender: mpsc::Sender<Request>,
    receiver: mpsc::Receiver<Request>,
}

impl Default for Setup {
    fn default() -> Self {
        Self::new::<32>()
    }
}

impl Setup {
    pub fn new<const C: usize>() -> Self {
        let (sender, receiver) = mpsc::channel(C);
        Self { sender, receiver }
    }

    pub fn get_client(&self) -> Client {
        Client {
            sender: self.sender.clone(),
        }
    }

    pub fn complete(self) -> Runtime {
        Runtime {
            receiver: self.receiver,
        }
    }
}

pub struct Runtime {
    receiver: mpsc::Receiver<Request>,
}

fn respond<T>(response_sender: oneshot::Sender<Result<T>>, response: Result<T>) -> Result {
    response_sender
        .send(response)
        .map_err(|_| Error::ResponseSendError)
}

impl Runtime {
    pub async fn run<const N: usize>(mut self, mut lora_e5: LoraE5<N>) -> Result {
        task::spawn_blocking(|| async move {
            while let Some(request) = self.receiver.recv().await {
                match request {
                    Request::At(cmd, timeout, sender) => {
                        if let Err(e) = lora_e5.write_command(&cmd) {
                            respond(sender, Err(e.into()))?;
                        } else {
                            match lora_e5.read_until_break(timeout) {
                                Ok(n) => {
                                    let response =
                                        std::str::from_utf8(&lora_e5.buf[..n])?.to_string();
                                    respond(sender, Ok(response))?;
                                }
                                Err(e) => {
                                    respond(sender, Err(e.into()))?;
                                }
                            };
                        }
                    }
                    Request::Configure(credentials, response_sender) => {
                        //todo: send back errors
                        lora_e5.set_mode(Mode::Otaa)?;
                        lora_e5.set_region(Region::Us915)?;
                        lora_e5.set_credentials(&credentials)?;
                        lora_e5.subband2_only()?;
                        response_sender
                            .send(Ok(()))
                            .map_err(|_| Error::ResponseSendError)?;
                    }
                    Request::Join(force, sender) => {
                        println!("Sending Join");
                        let result = if force {
                            lora_e5.force_join()
                        } else {
                            lora_e5.join()
                        };
                        respond(sender, result.map_err(|e| e.into()))?;
                    }

                    Request::SendData(data, port, confirmed, sender) => {
                        let result = lora_e5.send(&data, port, confirmed);
                        respond(sender, result.map_err(|e| e.into()))?;
                    }
                    Request::SendAscii(data, port, confirmed, sender) => {
                        let result = lora_e5.send_ascii(&data, port, confirmed);
                        respond(sender, result.map_err(|e| e.into()))?;
                    }
                    Request::Shutdown => {
                        return Ok(());
                    }
                }
            }
            Ok(())
        })
        .await?
        .await
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("lora e5: {0}")]
    LoraE5(#[from] LoraE5Error),
    #[error("join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("utf8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("request send error: {0}")]
    RequestSendError(#[from] mpsc::error::SendError<Request>),
    #[error("response receive error: {0}")]
    ResponseReceiveError(#[from] oneshot::error::RecvError),
    #[error("response send error")]
    ResponseSendError,
}
