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
