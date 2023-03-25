use lora_e5::{
    process, AppEui, AppKey, Credentials, DevEui, LoraE5, CP210X_UART_BRIDGE_PID, DR,
    SILICON_LABS_VID,
};
use std::str::FromStr;
use thiserror::Error;
use tokio::time::Duration;

#[derive(Debug, clap::Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"))]
#[clap(name = "lora-e5-cli", about = "CLI for interacting with LoRa E5")]
enum Cmd {
    /// Send AT command to modem. Returns single-line response (does not work well with multi-line
    /// responses, such as Join)
    At(At),
    /// Join. Use --force flag to force a join, otherwise active session will be maintained.
    Join(Join),
    /// Configure with credentials
    Configure(Configure),
    /// Read out AppEui
    GetAppEui,
    /// Read out DevEui
    GetDevEui,
    /// Set data rate
    Datarate(Datarate),
    /// Send data. Input must be in hex format.
    Send(SendHex),
    /// Send ASCII
    SendAscii(SendAscii),
}

#[derive(Debug, Clone, clap::Args)]
struct Configure {
    /// DevEui as hex string
    pub dev_eui: DevEui,
    /// AppEui as hex string
    pub app_eui: AppEui,
    /// AppKey as hex string
    pub app_key: AppKey,
}

#[derive(Debug, Clone, clap::Args)]
struct SendHex {
    /// Data in hexadecimal format
    pub data: HexData,
    /// Port
    #[arg(default_value = "1")]
    pub port: u8,
    /// Require ACK
    #[arg(long, short)]
    pub confirmed: bool,
}

#[derive(Debug, Clone, clap::Args)]
struct Datarate {
    pub dr: DR,
}

#[derive(Debug, Clone, clap::Args)]
struct HexData {
    data: Vec<u8>,
}

impl FromStr for HexData {
    type Err = hex::FromHexError;

    fn from_str(str: &str) -> std::result::Result<HexData, Self::Err> {
        let data = hex::decode(str)?;
        Ok(HexData { data })
    }
}

#[derive(Debug, clap::Args)]
struct SendAscii {
    /// ASCII string
    pub data: String,
    #[arg(default_value = "1")]
    pub port: u8,
    /// Require ACK
    #[arg(long, short)]
    pub confirmed: bool,
}

#[derive(Debug, clap::Args)]
struct At {
    /// AT Command
    cmd: String,
    /// Timeout in millis,
    #[arg(default_value = "250")]
    timeout: u64,
}

#[derive(Debug, clap::Args)]
struct Join {
    /// Force a join request. Otherwise, if device is already joined, no join occurs.
    #[arg(long, short)]
    force: bool,
}

pub type Result<T = ()> = std::result::Result<T, Error>;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result {
    use clap::Parser;
    let cmd = Cmd::parse();

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
            println!("Credentials configured");
        }
        Cmd::GetAppEui => {
            let app_eui = client.get_app_eui().await?.to_string();
            println!("{app_eui}");
        }
        Cmd::GetDevEui => {
            let dev_eui = client.get_dev_eui().await?.to_string();
            println!("{dev_eui}");
        }
        Cmd::Datarate(Datarate { dr }) => {
            client.data_rate(dr).await?;
            println!("DR{} set", dr.as_str());
        }
        Cmd::Send(SendHex {
            data,
            port,
            confirmed,
        }) => {
            let response = client.send(data.data, port, confirmed).await?;
            println!("{response:?}");
        }
        Cmd::SendAscii(SendAscii {
            data,
            port,
            confirmed,
        }) => {
            let response = client.send_ascii(data, port, confirmed).await?;
            println!("{response:?}");
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
