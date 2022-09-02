use lora_e5::{
    process, AppEui, AppKey, Credentials, DevEui, LoraE5, CP210X_UART_BRIDGE_PID, SILICON_LABS_VID,
};
use std::str::FromStr;
use structopt::StructOpt;
use thiserror::Error;
use tokio::time::Duration;

#[derive(Debug, StructOpt)]
#[structopt(name = "lora-e5-cli", about = "CLI for interacting with LoRa E5")]
enum Cmd {
    /// Send AT command to modem. Returns single-line response (does not work well with multi-line
    /// responses, such as Join)
    At(At),
    /// Join
    Join(Join),
    /// Configure
    Configure(Configure),
    /// Send data. Input must be in hex format.
    Send(SendHex),
    /// Send ASCII
    SendAscii(SendAscii),
}

#[derive(Debug, StructOpt)]
struct Configure {
    /// DevEui as hex string
    pub dev_eui: DevEui,
    /// AppEui as hex string
    pub app_eui: AppEui,
    /// AppKey as hex string
    pub app_key: AppKey,
}

#[derive(Debug, StructOpt)]
struct SendHex {
    /// Data in hexadecimal format
    pub data: HexData,
    /// Port
    #[structopt(default_value = "1")]
    pub port: u8,
    /// Require ACK
    #[structopt(long, short)]
    pub confirmed: bool,
}

#[derive(Debug)]
struct HexData {
    data: Vec<u8>,
}

impl FromStr for HexData {
    type Err = hex::FromHexError;

    fn from_str(str: &str) -> std::result::Result<HexData, Self::Err> {
        let data = hex::decode(&str)?;
        Ok(HexData { data })
    }
}

#[derive(Debug, StructOpt)]
struct SendAscii {
    /// ASCII string
    pub data: String,
    #[structopt(default_value = "1")]
    pub port: u8,
    /// Require ACK
    #[structopt(long, short)]
    pub confirmed: bool,
}

#[derive(Debug, StructOpt)]
struct At {
    /// AT Command
    cmd: String,
    /// Timeout in millis,
    #[structopt(default_value = "250")]
    timeout: u64,
}

#[derive(Debug, StructOpt)]
struct Join {
    /// Force a join request. Otherwise, if device is already joined, no join occurs.
    #[structopt(long, short)]
    force: bool,
}

pub type Result<T = ()> = std::result::Result<T, Error>;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result {
    let cmd = Cmd::from_args();

    let process = process::Setup::default();
    let client = process.get_client();
    let runtime = process.complete();
    let lora_e5 = LoraE5::<128>::open_usb(SILICON_LABS_VID, CP210X_UART_BRIDGE_PID)?;

    let runtime_handle = tokio::spawn(runtime.run(lora_e5));

    match cmd {
        Cmd::At(At { cmd, timeout }) => {
            let response = client
                .at_command(&cmd, Duration::from_millis(timeout))
                .await?;
            println!("{response}");
        }
        Cmd::Join(Join { force }) => {
            let join_response = client.join(force).await?;
            println!("{join_response:?}");
        }
        Cmd::Configure(Configure {
            dev_eui,
            app_eui,
            app_key,
        }) => {
            client
                .configure(Credentials {
                    dev_eui,
                    app_eui,
                    app_key,
                })
                .await?;
        }
        Cmd::Send(SendHex {
            data,
            port,
            confirmed,
        }) => {
            let join_response = client.send(data.data, port, confirmed).await?;
            println!("{join_response:?}");
        }
        Cmd::SendAscii(SendAscii {
            data,
            port,
            confirmed,
        }) => {
            let join_response = client.send_ascii(data, port, confirmed).await?;
            println!("{join_response:?}");
        }
    }

    client.send_shutdown().await?;
    runtime_handle.await??;

    Ok(())
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("lora e5 error: {0}")]
    LoraE5(#[from] lora_e5::Error),
    #[error("lora e5 process error: {0}")]
    LoraE5Process(#[from] lora_e5::process::Error),
    #[error("join handle error: {0}")]
    JoinHandle(#[from] tokio::task::JoinError),
}
