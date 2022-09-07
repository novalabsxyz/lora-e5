pub enum Mode {
    Test,
    Otaa,
    Abp,
}

impl Mode {
    pub fn as_str(&self) -> &str {
        match self {
            Mode::Test => "TEST",
            Mode::Abp => "LWABP",
            Mode::Otaa => "LWOTAA",
        }
    }
}

pub enum Region {
    Eu868,
    Us915,
}

impl Region {
    pub fn as_str(&self) -> &str {
        match self {
            Region::Eu868 => "EU868",
            Region::Us915 => "US915",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DR {
    _0,
    _1,
    _2,
    _3,
    _4,
}

use super::Error;
use std::str::FromStr;

impl FromStr for DR {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(DR::_0),
            "1" => Ok(DR::_1),
            "2" => Ok(DR::_2),
            "3" => Ok(DR::_3),
            "4" => Ok(DR::_4),
            _ => Err(Error::InvalidDatarateStr(s.to_string())),
        }
    }
}

impl DR {
    pub fn as_str(&self) -> &str {
        match self {
            DR::_0 => "0",
            DR::_1 => "1",
            DR::_2 => "2",
            DR::_3 => "3",
            DR::_4 => "4",
        }
    }

    pub fn termination_pattern(&self) -> &str {
        match self {
            DR::_0 => "US915 DR0  SF10 BW125K \r\n",
            DR::_1 => "US915 DR1  SF9  BW125K \r\n",
            DR::_2 => "US915 DR2  SF8  BW125K \r\n",
            DR::_3 => "US915 DR3  SF7  BW125K \r\n",
            DR::_4 => "US915 DR4  SF8  BW500K \r\n",
        }
    }

    pub fn all_patterns() -> [&'static str; 5] {
        [
            DR::_0.termination_pattern(),
            DR::_1.termination_pattern(),
            DR::_2.termination_pattern(),
            DR::_3.termination_pattern(),
            DR::_4.termination_pattern(),
        ]
    }
}
