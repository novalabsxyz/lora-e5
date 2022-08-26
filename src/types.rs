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

            fn from_str(s: &str) -> Result<Self, Self::Err> {
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

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("hex error: {0}")]
    FromHex(#[from] hex::FromHexError),
    #[error("Vec is unexpected of len {0}")]
    VecWrongSize(usize),
}
