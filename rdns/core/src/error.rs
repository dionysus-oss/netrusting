use std::{io, string};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RDNSError {
    #[error("name exceeds the 255 byte limit")]
    NameTooLong(usize),

    #[error("name label exceeds the 63 byte limit")]
    NameLabelTooLong(u8),

    #[error("name label is invalid at position")]
    NameLabelInvalid(u8),

    #[error("the name is invalid")]
    NameInvalid(),

    #[error("the resource record is invalid")]
    ResourceRecordInvalid(),

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
