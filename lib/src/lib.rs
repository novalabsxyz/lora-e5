use serialport::{SerialPort, SerialPortType};
use std::time::{self, Duration};

mod error;
pub use error::Error;

mod types;
use types::*;

mod credentials;
pub use credentials::*;

mod parse;

#[cfg(test)]
mod tests;

pub const SILICON_LABS_VID: u16 = 0x10C4;
pub const CP210X_UART_BRIDGE_PID: u16 = 0xEA60;

#[cfg(feature = "runtime")]
pub mod process;

pub struct LoraE5<const N: usize> {
    port: Box<dyn SerialPort>,
    buf: [u8; N],
}

pub type Result<T = ()> = std::result::Result<T, error::Error>;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct Downlink {
    pub rssi: isize,
    pub snr: f32,
}
#[derive(Debug, PartialEq, Eq)]
pub enum JoinResponse {
    JoinComplete,
    JoinFailed,
    AlreadyJoined,
}

impl<const N: usize> LoraE5<N> {
    pub fn open_usb(vid: u16, pid: u16) -> Result<Self> {
        let available_ports = serialport::available_ports()?;
        for port in available_ports {
            if let SerialPortType::UsbPort(usb_port) = port.port_type {
                if usb_port.vid == vid && usb_port.pid == pid {
                    let port = serialport::new(&port.port_name, 9600)
                        .timeout(Duration::from_millis(10))
                        .open()?;
                    return Ok(Self { port, buf: [0; N] });
                }
            }
        }
        Err(Error::PortNotFound { vid, pid })
    }

    pub fn open_path<'a>(path: impl Into<std::borrow::Cow<'a, str>>) -> Result<Self> {
        let port = serialport::new(path, 9600)
            .timeout(Duration::from_millis(10))
            .open()?;
        Ok(Self { port, buf: [0; N] })
    }

    fn write_command(&mut self, cmd: &str) -> Result {
        let n = self.port.write(cmd.as_bytes())?;
        if n != cmd.len() {
            return Err(Error::IncorrectWrite(n, cmd.len()));
        }
        let n = self.port.write("\n".as_bytes())?;
        if n != 1 {
            return Err(Error::IncorrectWrite(n, 1));
        }
        Ok(())
    }

    pub fn is_ok(&mut self) -> Result<bool> {
        self.write_command("AT")?;
        let n = self.read_until_break(Duration::from_millis(50))?;
        Ok(self.check_framed_response(n, "+AT: ", "OK").is_ok())
    }

    pub fn get_version(&mut self) -> Result<String> {
        const EXPECTED_PRELUDE: &str = "+VER: ";
        self.write_command("AT+VER")?;
        let n = self.read_until_break(DEFAULT_TIMEOUT)?;
        let version = self.framed_response(n, EXPECTED_PRELUDE)?;
        Ok(version.trim_end().to_string())
    }

    pub fn set_channel(&mut self, ch: u8, enable: bool) -> Result {
        let cmd = format!("AT+CH={ch},{}", if enable { "on" } else { "off" });
        self.write_command(&cmd)?;
        let n = self.read_until_break(DEFAULT_TIMEOUT)?;
        self.check_framed_response(n, "+CH: CH", &format!("{ch} off"))
    }

    pub fn subband2_only(&mut self) -> Result {
        for n in 0..8 {
            self.set_channel(n, false)?;
        }
        for n in 16..72 {
            self.set_channel(n, false)?;
        }
        Ok(())
    }

    pub fn set_region(&mut self, region: Region) -> Result {
        const EXPECTED_PRELUDE: &str = "+DR: ";
        let cmd = format!("AT+DR={}", region.as_str());
        self.write_command(&cmd)?;
        let n = self.read_until_break(DEFAULT_TIMEOUT)?;
        self.check_framed_response(n, EXPECTED_PRELUDE, region.as_str())
    }

    pub fn set_mode(&mut self, mode: Mode) -> Result {
        const EXPECTED_PRELUDE: &str = "+MODE: ";
        let cmd = format!("AT+MODE={}", mode.as_str());
        self.write_command(&cmd)?;
        let n = self.read_until_break(DEFAULT_TIMEOUT)?;
        self.check_framed_response(n, EXPECTED_PRELUDE, mode.as_str())
    }

    pub fn join(&mut self) -> Result<JoinResponse> {
        const JOIN_DONE: &str = "+JOIN: Done\r\n";
        const ALREADY_JOINED: &str = "+JOIN: Joined already\r\n";

        self.write_command("AT+JOIN")?;
        let n = self.read_until_pattern(&[JOIN_DONE, ALREADY_JOINED], Duration::from_secs(20))?;
        let response = std::str::from_utf8(&self.buf[..n])?;
        Ok(if response.contains(&ALREADY_JOINED) {
            JoinResponse::AlreadyJoined
        } else if response.contains("Network joined") {
            JoinResponse::JoinComplete
        } else {
            JoinResponse::JoinFailed
        })
    }

    pub fn force_join(&mut self) -> Result<JoinResponse> {
        const JOIN_DONE: &str = "+JOIN: Done\r\n";
        self.write_command("AT+JOIN=FORCE")?;
        let n = self.read_until_pattern(&[JOIN_DONE], Duration::from_secs(20))?;
        let response = std::str::from_utf8(&self.buf[..n])?;
        Ok(if response.contains("Network joined") {
            JoinResponse::JoinComplete
        } else {
            JoinResponse::JoinFailed
        })
    }

    pub fn set_port(&mut self, port: u8) -> Result {
        const EXPECTED_PRELUDE: &str = "+PORT: ";
        let cmd = format!("AT+PORT={port}");
        self.write_command(&cmd)?;
        let n = self.read_until_break(DEFAULT_TIMEOUT)?;
        self.check_framed_response(n, EXPECTED_PRELUDE, &port.to_string())
    }

    pub fn send(&mut self, data: &[u8], port: u8, confirmed: bool) -> Result<Option<Downlink>> {
        self.set_port(port)?;
        let end_line = if confirmed {
            "+CMSGHEX: Done\r\n"
        } else {
            "+MSGHEX: Done\r\n"
        };

        let hex = hex::encode(&data);
        let cmd = format!(
            "AT+{}=\"{hex}\"",
            if confirmed { "CMSGHEX" } else { "MSGHEX" }
        );
        self.write_command(&cmd)?;
        let n = self.read_until_pattern(&[end_line], Duration::from_secs(3))?;
        let response = std::str::from_utf8(&self.buf[..n])?;

        if let Some(m) = response.find("RXWIN1") {
            let (rssi, snr) = parse_rssi_snr(response, m)?;
            Ok(Some(Downlink { rssi, snr }))
        } else if let Some(m) = response.find("RXWIN2") {
            let (rssi, snr) = parse_rssi_snr(response, m)?;
            Ok(Some(Downlink { rssi, snr }))
        } else if confirmed {
            // we expect a downlink when sending confirmed uplinks
            // todo: check for ACK in response
            Err(Error::Nack)
        } else {
            Ok(None)
        }
    }

    pub fn send_ascii(
        &mut self,
        data: &str,
        port: u8,
        confirmed: bool,
    ) -> Result<Option<Downlink>> {
        self.set_port(port)?;
        let end_line = if confirmed {
            "+CMSG: Done\r\n"
        } else {
            "+MSG: Done\r\n"
        };
        let hex = hex::encode(&data);
        let cmd = format!("AT+{}=\"{hex}\"", if confirmed { "CMSG" } else { "MSG" });
        self.write_command(&cmd)?;
        let n = self.read_until_pattern(&[end_line], Duration::from_secs(3))?;
        let response = std::str::from_utf8(&self.buf[..n])?;

        if let Some(m) = response.find("RXWIN1") {
            let (rssi, snr) = parse_rssi_snr(response, m)?;
            Ok(Some(Downlink { rssi, snr }))
        } else if let Some(m) = response.find("RXWIN2") {
            let (rssi, snr) = parse_rssi_snr(response, m)?;
            Ok(Some(Downlink { rssi, snr }))
        } else if confirmed {
            // we expect a downlink when sending confirmed uplinks
            // todo: check for ACK in response
            Err(Error::Nack)
        } else {
            Ok(None)
        }
    }
}

pub(crate) fn parse_rssi_snr(response: &str, m: usize) -> Result<(isize, f32)> {
    let (_, remaining_str) = response.split_at(m);
    if let Some(n) = remaining_str.find("\r\n") {
        let (line, _) = remaining_str.split_at(n);
        let (_, signal) = line.split_at(", RSSI ".len());
        if let Some(n) = signal.find(", ") {
            let (rssi_remainder, snr_remainder) = signal.split_at(n);
            let (_, rssi) = rssi_remainder.split_at(" RSSI ".len());
            let (_, snr) = snr_remainder.split_at(", SNR ".len());
            return Ok((
                rssi.parse().map_err(Error::FailedToParseRssiInt)?,
                snr.parse().map_err(Error::FailedToParseSnrF32)?,
            ));
        }
    }
    Err(Error::FailedToParseRssiSnr(response.to_string()))
}
