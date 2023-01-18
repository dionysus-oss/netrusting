use std::fmt::{Display, Formatter};
use std::{io, string};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RDNSError {
    #[error("name exceeds the 255 byte limit")]
    NameTooLong(usize),

    #[error("name label exceeds the 63 byte limit")]
    NameLabelTooLong(u8),

    #[error("name label is invalid at position {0}")]
    NameLabelInvalid(u8),

    #[error("the name is invalid")]
    NameInvalid(),

    #[error("the resource record is invalid")]
    ResourceRecordInvalid(),

    // TODO capture line and char position
    #[error("the format of the master file is invalid at position {1} - {0}")]
    MasterFileFormatError(String, LineCharPos),

    #[error("i/o error")]
    IoError {
        #[from]
        source: io::Error,
    },

    #[error("invalid encoding")]
    EncodingError {
        #[from]
        source: string::FromUtf8Error,
    },
}

#[derive(Debug)]
pub struct LineCharPos {
    pub line: u32,
    pub char: u32,
}

impl Display for LineCharPos {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.char)
    }
}
