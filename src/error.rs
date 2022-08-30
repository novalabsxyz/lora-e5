use crate::ParseError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("serialport error: {0}")]
    SerialPort(#[from] serialport::Error),
    #[error("utf8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unexpected at response: {0}")]
    UnexpectedResponse(String),
    #[error("partial response after timeout: {0}")]
    PartialResponse(String),
    #[error("enabled to find port with pid = {vid} abd vid = {pid}")]
    PortNotFound { vid: u16, pid: u16 },
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),
    #[error("wrote incorrect amount of bytes: {0} instead of {1}")]
    IncorrectWrite(usize, usize),
}
