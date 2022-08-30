use serialport::{SerialPort, SerialPortType};
use std::time::{self, Duration};

mod error;
use error::Error;

mod types;
use types::*;

mod credentials;
use credentials::*;

mod parse;

pub struct LoraE5<const N: usize> {
    port: Box<dyn SerialPort>,
    buf: [u8; N],
}

pub type Result<T = ()> = std::result::Result<T, error::Error>;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

impl<const N: usize> LoraE5<N> {
    pub fn open_usb(vid: u16, pid: u16) -> Result<Self> {
        let available_ports = serialport::available_ports()?;
        for port in available_ports {
            if let SerialPortType::UsbPort(usb_port) = port.port_type {
                if usb_port.vid == vid && usb_port.pid == pid {
                    let port = serialport::new(&port.port_name, 9600)
                        .timeout(Duration::from_millis(10))
                        .open()
                        .expect("Failed to open port");
                    return Ok(Self { port, buf: [0; N] });
                }
            }
        }
        Err(Error::PortNotFound { vid, pid })
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

    pub fn join(&mut self) -> Result<bool> {
        const END_LINE: &str = "+JOIN: Done";
        self.write_command("AT+JOIN=FORCE")?;
        let n = self.read_until_pattern(END_LINE, Duration::from_secs(20))?;
        let response = std::str::from_utf8(&self.buf[..n])?;
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
    use super::*;
    use std::str::FromStr;

    fn lora_test_hardware() -> LoraE5<128_usize> {
        const SILICON_LABS_VID: u16 = 0x10C4;
        const CP210X_UART_BRIDGE_PID: u16 = 0xEA60;
        LoraE5::<128>::open_usb(SILICON_LABS_VID, CP210X_UART_BRIDGE_PID).unwrap()
    }

    #[test]
    fn usb_open() {
        let _lora_e5 = lora_test_hardware();
    }

    #[test]
    fn is_ok() {
        let mut lora_e5 = lora_test_hardware();
        lora_e5.is_ok().unwrap();
    }

    #[test]
    fn get_version() {
        let mut lora_e5 = lora_test_hardware();
        let _version = lora_e5.get_version().unwrap();
    }

    #[test]
    fn set_subband() {
        let mut lora_e5 = lora_test_hardware();
        lora_e5.subband2_only().unwrap();
    }

    #[test]
    fn get_dev_eui() {
        let mut lora_e5 = lora_test_hardware();
        lora_e5.get_dev_eui().unwrap();
    }

    #[test]
    fn get_app_eui() {
        let mut lora_e5 = lora_test_hardware();
        lora_e5.get_app_eui().unwrap();
    }

    #[test]
    fn set_app_eui() {
        let app_eui = AppEui::from_str("0123456789ABCDEF").unwrap();
        let mut lora_e5 = lora_test_hardware();
        lora_e5.set_app_eui(&app_eui).unwrap();
        let fetched_app_eui = lora_e5.get_app_eui().unwrap();
        assert_eq!(app_eui, fetched_app_eui);
    }

    #[test]
    fn set_dev_eui() {
        let dev_eui = DevEui::from_str("111111111111111A").unwrap();
        let mut lora_e5 = lora_test_hardware();
        lora_e5.set_dev_eui(&dev_eui).unwrap();
        let fetched_dev_eui = lora_e5.get_dev_eui().unwrap();
        assert_eq!(dev_eui, fetched_dev_eui);
    }

    #[test]
    fn set_app_key() {
        let app_key = AppKey::from_str("111111111111111A111111111111111A").unwrap();
        let mut lora_e5 = lora_test_hardware();
        lora_e5.set_app_key(&app_key).unwrap();
    }

    #[test]
    fn set_mode_otaa() {
        let mut lora_e5 = lora_test_hardware();
        lora_e5.set_mode(Mode::Otaa).unwrap();
    }

    #[test]
    fn set_mode_abp() {
        let mut lora_e5 = lora_test_hardware();
        lora_e5.set_mode(Mode::Abp).unwrap();
    }

    #[test]
    fn set_mode_test() {
        let mut lora_e5 = lora_test_hardware();
        lora_e5.set_mode(Mode::Test).unwrap();
    }

    #[test]
    fn join() {
        let credentials = Credentials::new(
            DevEui::from_str("6081F9A775278564").unwrap(),
            AppEui::from_str("6081F9A498856DCC").unwrap(),
            AppKey::from_str("72F36B996179E634537FCA76047D0B51").unwrap(),
        );
        let mut lora_e5 = lora_test_hardware();
        lora_e5.set_mode(Mode::Otaa).unwrap();
        lora_e5.set_region(Region::Us915).unwrap();
        lora_e5.set_credentials(&credentials).unwrap();
        lora_e5.subband2_only().unwrap();
        lora_e5.join().unwrap();
        std::thread::sleep(Duration::from_millis(50));
    }
}
