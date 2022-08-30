use std::{fmt, str::FromStr};

pub struct Credentials {
    pub app_eui: AppEui,
    pub app_key: AppKey,
    pub dev_eui: DevEui,
}

impl Credentials {
    pub fn new(dev_eui: DevEui, app_eui: AppEui, app_key: AppKey) -> Self {
        Self {
            dev_eui,
            app_eui,
            app_key,
        }
    }
}

macro_rules! derive_from_str {
    ($name:ident, $size:expr) => {
        #[derive(Debug, PartialEq, Eq)]
        pub struct $name([u8; $size]);

        impl FromStr for $name {
            type Err = ParseError;

            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                let mut s = s.to_string();
                s.retain(|c| c != ':');
                let byte_vec = hex::decode(&s)?;
                let len = byte_vec.len();
                let byte_arr: [u8; $size] = byte_vec
                    .try_into()
                    .map_err(|_| ParseError::VecWrongSize(len))?;
                Ok(Self(byte_arr))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let str = hex::encode(&self.0).to_uppercase();
                write!(f, "{str}")
            }
        }

        impl From<[u8; $size]> for $name {
            fn from(arr: [u8; $size]) -> Self {
                Self(arr)
            }
        }
    };
}

derive_from_str!(AppEui, 8);
derive_from_str!(DevEui, 8);
derive_from_str!(AppKey, 16);

use super::*;

impl<const N: usize> LoraE5<N> {
    pub fn get_dev_eui(&mut self) -> Result<DevEui> {
        const EXPECTED_PRELUDE: &str = "+ID: DevEui, ";
        self.write_command("AT+ID=DevEui")?;
        let n = self.read_until_break(DEFAULT_TIMEOUT)?;
        let response = self.framed_response(n, EXPECTED_PRELUDE)?;
        Ok(DevEui::from_str(response.trim_end())?)
    }

    pub fn get_app_eui(&mut self) -> Result<AppEui> {
        const EXPECTED_PRELUDE: &str = "+ID: AppEui, ";
        self.write_command("AT+ID=AppEui")?;
        let n = self.read_until_break(DEFAULT_TIMEOUT)?;
        let response = self.framed_response(n, EXPECTED_PRELUDE)?;
        Ok(AppEui::from_str(response.trim_end())?)
    }

    pub fn set_app_eui(&mut self, app_eui: &AppEui) -> Result {
        const EXPECTED_PRELUDE: &str = "+ID: AppEui, ";
        let cmd = format!("AT+ID=AppEui, {app_eui}");
        self.write_command(&cmd)?;
        let n = self.read_until_break(DEFAULT_TIMEOUT)?;
        let response = self.framed_response(n, EXPECTED_PRELUDE)?;
        let app_eui_response = AppEui::from_str(response.trim_end())?;
        if &app_eui_response == app_eui {
            Ok(())
        } else {
            Err(Error::UnexpectedResponse(app_eui_response.to_string()))
        }
    }

    pub fn set_dev_eui(&mut self, dev_eui: &DevEui) -> Result {
        const EXPECTED_PRELUDE: &str = "+ID: DevEui, ";
        let cmd = format!("AT+ID=DevEui, {dev_eui}");
        self.write_command(&cmd)?;
        let n = self.read_until_break(DEFAULT_TIMEOUT)?;
        let response = self.framed_response(n, EXPECTED_PRELUDE)?;
        let dev_eui_response = DevEui::from_str(response.trim_end())?;
        if &dev_eui_response == dev_eui {
            Ok(())
        } else {
            Err(Error::UnexpectedResponse(dev_eui_response.to_string()))
        }
    }

    pub fn set_app_key(&mut self, app_key: &AppKey) -> Result {
        const EXPECTED_PRELUDE: &str = "+KEY: APPKEY ";
        let cmd = format!("AT+KEY=APPKEY, {app_key}");
        self.write_command(&cmd)?;
        let n = self.read_until_break(DEFAULT_TIMEOUT)?;
        let response = self.framed_response(n, EXPECTED_PRELUDE)?;
        let app_key_response = AppKey::from_str(response.trim_end())?;
        if &app_key_response == app_key {
            Ok(())
        } else {
            Err(Error::UnexpectedResponse(response.to_string()))
        }
    }

    pub fn set_credentials(&mut self, credentials: &Credentials) -> Result {
        self.set_dev_eui(&credentials.dev_eui)?;
        self.set_app_eui(&credentials.app_eui)?;
        self.set_app_key(&credentials.app_key)
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("hex error: {0}")]
    FromHex(#[from] hex::FromHexError),
    #[error("Vec is unexpected of len {0}")]
    VecWrongSize(usize),
}
