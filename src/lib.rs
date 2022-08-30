use serialport::{SerialPort, SerialPortType};
use std::time;
use std::{str::FromStr, time::Duration};
mod error;
use error::Error;

mod types;
use types::*;

pub struct LoraE5 {
    port: Box<dyn SerialPort>,
}

pub type Result<T = ()> = std::result::Result<T, error::Error>;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

impl LoraE5 {
    pub fn open_usb(vid: u16, pid: u16) -> Result<LoraE5> {
        let available_ports = serialport::available_ports()?;
        for port in available_ports {
            if let SerialPortType::UsbPort(usb_port) = port.port_type {
                if usb_port.vid == vid && usb_port.pid == pid {
                    let port = serialport::new(&port.port_name, 9600)
                        .timeout(Duration::from_millis(10))
                        .open()
                        .expect("Failed to open port");
                    return Ok(LoraE5 { port });
                }
            }
        }
        Err(Error::PortNotFound { vid, pid })
    }

    pub fn is_ok(&mut self) -> Result<bool> {
        self.write_command("AT")?;
        let mut buf: Vec<u8> = vec![0; 32];
        let n = self.read_until_break(&mut buf, Duration::from_millis(50))?;
        let response = std::str::from_utf8(&buf[..n])?;
        if response.trim_end() == "+AT: OK" {
            Ok(true)
        } else {
            Err(Error::UnexpectedResponse(response.to_string()))
        }
    }

    pub fn get_version(&mut self) -> Result<String> {
        const EXPECTED_PRELUDE: &str = "+VER: ";
        self.write_command("AT+VER")?;
        let mut buf: Vec<u8> = vec![0; 64];
        let n = self.read_until_break(&mut buf, DEFAULT_TIMEOUT)?;
        let response = std::str::from_utf8(&buf[..n])?;
        let (prelude, version) = response.split_at(EXPECTED_PRELUDE.len());
        if prelude == EXPECTED_PRELUDE {
            return Ok(version.trim_end().to_string());
        } else {
            Err(Error::UnexpectedResponse(response.to_string()))
        }
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

    pub fn set_channel(&mut self, ch: u8, enable: bool) -> Result {
        let cmd = format!("AT+CH={ch},{}", if enable { "on" } else { "off" });
        self.write_command(&cmd)?;
        let mut buf: Vec<u8> = vec![0; 64];
        let n = self.read_until_break(&mut buf, DEFAULT_TIMEOUT)?;
        let response = std::str::from_utf8(&buf[..n])?;
        if response.starts_with(&format!("+CH: CH{ch} off")) {
            Ok(())
        } else {
            Err(Error::UnexpectedResponse(response.to_string()))
        }
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

    fn read_until_break(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize> {
        let mut start = 0;
        let mut bytes_read = 0;
        let mut time = time::Instant::now();
        loop {
            if let Ok(n) = self.port.read(&mut buf[bytes_read..]) {
                if n != 0 {
                    start = bytes_read;
                    bytes_read += n;
                    time = time::Instant::now();
                } else {
                    return Ok(bytes_read);
                }
            }
            for byte in &buf[start..bytes_read] {
                if *byte == b'\n' {
                    return Ok(bytes_read);
                }
            }
            if time.elapsed() > timeout {
                let partial_response = std::str::from_utf8(&buf[..bytes_read])?;
                return Err(Error::PartialResponse(partial_response.to_string()));
            }
        }
    }

    fn read_until_close(
        &mut self,
        buf: &mut [u8],
        timeout: Duration,
    ) -> Result<usize> {
        let mut bytes_read = 0;
        let mut time = time::Instant::now();
        loop {
            if let Ok(n) = self.port.read(&mut buf[bytes_read..]) {
                if n != 0 {
                    bytes_read += n;
                    time = time::Instant::now();
                } else {
                    println!("{}", std::str::from_utf8(&buf[..bytes_read]).unwrap());
                    return Ok(bytes_read);
                }
            }
            if time.elapsed() > timeout {
                let partial_response = std::str::from_utf8(&buf[..bytes_read])?;
                return Err(Error::PartialResponse(partial_response.to_string()));
            }
        }
    }

    pub fn get_dev_eui(&mut self) -> Result<DevEui> {
        const EXPECTED_PRELUDE: &str = "+ID: DevEui, ";
        self.write_command("AT+ID=DevEui")?;
        let mut buf: Vec<u8> = vec![0; 64];
        let n = self.read_until_break(&mut buf, DEFAULT_TIMEOUT)?;
        let response = std::str::from_utf8(&buf[..n])?;
        let (prelude, dev_eui) = response.split_at(EXPECTED_PRELUDE.len());
        if prelude == EXPECTED_PRELUDE {
            Ok(DevEui::from_str(dev_eui.trim_end())?)
        } else {
            Err(Error::UnexpectedResponse(response.to_string()))
        }
    }

    pub fn get_app_eui(&mut self) -> Result<AppEui> {
        const EXPECTED_PRELUDE: &str = "+ID: AppEui, ";
        self.write_command("AT+ID=AppEui")?;
        let mut buf: Vec<u8> = vec![0; 64];
        let n = self.read_until_break(&mut buf, DEFAULT_TIMEOUT)?;
        let response = std::str::from_utf8(&buf[..n])?;
        let (prelude, app_eui) = response.split_at(EXPECTED_PRELUDE.len());
        if prelude == EXPECTED_PRELUDE {
            Ok(AppEui::from_str(app_eui.trim_end())?)
        } else {
            Err(Error::UnexpectedResponse(response.to_string()))
        }
    }

    pub fn set_app_eui(&mut self, app_eui: &AppEui) -> Result {
        const EXPECTED_PRELUDE: &str = "+ID: AppEui, ";
        let cmd = format!("AT+ID=AppEui, {app_eui}");
        self.write_command(&cmd)?;
        let mut buf: Vec<u8> = vec![0; 64];
        let n = self.read_until_break(&mut buf, DEFAULT_TIMEOUT)?;
        let response = std::str::from_utf8(&buf[..n])?;
        let (prelude, app_eui_str) = response.split_at(EXPECTED_PRELUDE.len());
        if prelude == EXPECTED_PRELUDE {
            let app_eui_response = AppEui::from_str(app_eui_str.trim_end())?;
            if &app_eui_response == app_eui {
                Ok(())
            } else {
                Err(Error::UnexpectedResponse(app_eui_str.to_string()))
            }
        } else {
            Err(Error::UnexpectedResponse(response.to_string()))
        }
    }

    pub fn set_dev_eui(&mut self, dev_eui: &DevEui) -> Result {
        const EXPECTED_PRELUDE: &str = "+ID: DevEui, ";
        let cmd = format!("AT+ID=DevEui, {dev_eui}");
        self.write_command(&cmd)?;
        let mut buf: Vec<u8> = vec![0; 64];
        let n = self.read_until_break(&mut buf, DEFAULT_TIMEOUT)?;
        let response = std::str::from_utf8(&buf[..n])?;
        let (prelude, dev_eui_str) = response.split_at(EXPECTED_PRELUDE.len());
        if prelude == EXPECTED_PRELUDE {
            let dev_eui_response = DevEui::from_str(dev_eui_str.trim_end())?;
            if &dev_eui_response == dev_eui {
                Ok(())
            } else {
                Err(Error::UnexpectedResponse(dev_eui_str.to_string()))
            }
        } else {
            Err(Error::UnexpectedResponse(response.to_string()))
        }
    }

    pub fn set_app_key(&mut self, app_key: &AppKey) -> Result {
        const EXPECTED_PRELUDE: &str = "+KEY: APPKEY ";
        let cmd = format!("AT+KEY=APPKEY, {app_key}");
        self.write_command(&cmd)?;
        let mut buf: Vec<u8> = vec![0; 64];
        let n = self.read_until_break(&mut buf, DEFAULT_TIMEOUT)?;
        let response = std::str::from_utf8(&buf[..n])?;
        let (prelude, app_key_str) = response.split_at(EXPECTED_PRELUDE.len());
        if prelude == EXPECTED_PRELUDE {
            let app_key_response = AppKey::from_str(app_key_str.trim_end())?;
            if &app_key_response == app_key {
                Ok(())
            } else {
                Err(Error::UnexpectedResponse(app_key_str.to_string()))
            }
        } else {
            Err(Error::UnexpectedResponse(response.to_string()))
        }
    }

    pub fn set_credentials(&mut self, credentials: &Credentials) -> Result {
        self.set_dev_eui(&credentials.dev_eui)?;
        self.set_app_eui(&credentials.app_eui)?;
        self.set_app_key(&credentials.app_key)
    }

    pub fn set_region(&mut self, region: Region) -> Result {
        const EXPECTED_PRELUDE: &str = "+DR: ";
        let cmd = format!("AT+DR={}", region.as_str());
        self.write_command(&cmd)?;
        let mut buf: Vec<u8> = vec![0; 64];
        let n = self.read_until_break(&mut buf, DEFAULT_TIMEOUT)?;
        let response = std::str::from_utf8(&buf[..n])?;
        let (prelude, region_response) = response.split_at(EXPECTED_PRELUDE.len());
        if prelude == EXPECTED_PRELUDE {
            if region_response.trim_end() == region.as_str() {
                Ok(())
            } else {
                Err(Error::UnexpectedResponse(region.as_str().to_string()))
            }
        } else {
            Err(Error::UnexpectedResponse(response.to_string()))
        }
    }

    pub fn set_mode(&mut self, mode: Mode) -> Result {
        const EXPECTED_PRELUDE: &str = "+MODE: ";
        let cmd = format!("AT+MODE={}", mode.as_str());
        self.write_command(&cmd)?;
        let mut buf: Vec<u8> = vec![0; 64];
        let n = self.read_until_break(&mut buf, DEFAULT_TIMEOUT)?;
        let response = std::str::from_utf8(&buf[..n])?;
        let (prelude, mode_response) = response.split_at(EXPECTED_PRELUDE.len());
        if prelude == EXPECTED_PRELUDE {
            if mode_response.trim_end() == mode.as_str() {
                Ok(())
            } else {
                Err(Error::UnexpectedResponse(mode.as_str().to_string()))
            }
        } else {
            Err(Error::UnexpectedResponse(response.to_string()))
        }
    }

    pub fn join(&mut self) -> Result<bool> {
        self.write_command("AT+JOIN=FORCE")?;
        let mut buf: Vec<u8> = vec![0; 64];
        let n = self.read_until_close(&mut buf, Duration::from_secs(10))?;
        let response = std::str::from_utf8(&buf[..n])?;
        if response.contains("Network Joined") {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// These tests all require a physical LoRa E5.
/// Run them one at a time to avoid port collisions:
///   ie: cargo test --  --nocapture --test-threads 1
#[cfg(test)]
mod tests {
    const VID: u16 = 4292;
    const PID: u16 = 60000;

    use super::*;

    #[test]
    fn usb_open() {
        let _lora_e5 = LoraE5::open_usb(VID, PID).unwrap();
    }

    #[test]
    fn is_ok() {
        let mut lora_e5 = LoraE5::open_usb(VID, PID).unwrap();
        lora_e5.is_ok().unwrap();
    }

    #[test]
    fn get_version() {
        let mut lora_e5 = LoraE5::open_usb(VID, PID).unwrap();
        let _version = lora_e5.get_version().unwrap();
    }

    #[test]
    fn set_subband() {
        let mut lora_e5 = LoraE5::open_usb(VID, PID).unwrap();
        lora_e5.subband2_only().unwrap();
    }

    #[test]
    fn get_dev_eui() {
        let mut lora_e5 = LoraE5::open_usb(VID, PID).unwrap();
        lora_e5.get_dev_eui().unwrap();
    }

    #[test]
    fn get_app_eui() {
        let mut lora_e5 = LoraE5::open_usb(VID, PID).unwrap();
        lora_e5.get_app_eui().unwrap();
    }

    #[test]
    fn set_app_eui() {
        let app_eui = AppEui::from_str("0123456789ABCDEF").unwrap();
        let mut lora_e5 = LoraE5::open_usb(VID, PID).unwrap();
        lora_e5.set_app_eui(&app_eui).unwrap();
        let fetched_app_eui = lora_e5.get_app_eui().unwrap();
        assert_eq!(app_eui, fetched_app_eui);
    }

    #[test]
    fn set_dev_eui() {
        let dev_eui = DevEui::from_str("111111111111111A").unwrap();
        let mut lora_e5 = LoraE5::open_usb(VID, PID).unwrap();
        lora_e5.set_dev_eui(&dev_eui).unwrap();
        let fetched_dev_eui = lora_e5.get_dev_eui().unwrap();
        assert_eq!(dev_eui, fetched_dev_eui);
    }

    #[test]
    fn set_app_key() {
        let app_key = AppKey::from_str("111111111111111A111111111111111A").unwrap();
        let mut lora_e5 = LoraE5::open_usb(VID, PID).unwrap();
        lora_e5.set_app_key(&app_key).unwrap();
    }

    #[test]
    fn set_mode_otaa() {
        let mut lora_e5 = LoraE5::open_usb(VID, PID).unwrap();
        lora_e5.set_mode(Mode::Otaa).unwrap();
    }

    #[test]
    fn set_mode_abp() {
        let mut lora_e5 = LoraE5::open_usb(VID, PID).unwrap();
        lora_e5.set_mode(Mode::Abp).unwrap();
    }

    #[test]
    fn set_mode_test() {
        let mut lora_e5 = LoraE5::open_usb(VID, PID).unwrap();
        lora_e5.set_mode(Mode::Test).unwrap();
    }

    #[test]
    fn join() {
        let credentials = Credentials::new(
            DevEui::from_str("6081F9A775278564").unwrap(),
            AppEui::from_str("6081F9A498856DCC").unwrap(),
            AppKey::from_str("72F36B996179E634537FCA76047D0B51").unwrap(),
        );
        let mut lora_e5 = LoraE5::open_usb(VID, PID).unwrap();
        lora_e5.set_mode(Mode::Otaa).unwrap();
        lora_e5.set_region(Region::Us915).unwrap();
        lora_e5.set_credentials(&credentials).unwrap();
        lora_e5.subband2_only().unwrap();
        lora_e5.join().unwrap();
        std::thread::sleep(Duration::from_millis(50));
    }
}
