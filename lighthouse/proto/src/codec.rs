use crate::{frame::Frame, Opcode};
use bytes::{Buf, BufMut, BytesMut};
use std::convert::TryInto;
use strum::IntoEnumIterator;
use tokio_util::codec::{Decoder, Encoder};

#[derive(Debug)]
pub enum Error {
    InvalidOpcode(u8),
    InvalidResponseCode(u8),
    IOError(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        use Error::*;
        let msg = match self {
            InvalidOpcode(opcode) => format!("Invalid opcode: `{}`", opcode),
            InvalidResponseCode(response_code) => format!("Invalid response code: `{}`", response_code),
            IOError(err) => format!("IOError: {}", err),
        };
        write!(f, "{}", msg)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(item: std::io::Error) -> Error {
        Error::IOError(item)
    }
}

pub struct FrameCodec {}

impl FrameCodec {
    pub fn new() -> Self {
        Self {}
    }
}

impl Decoder for FrameCodec {
    type Item = Frame;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        const MIN_SIZE: usize = std::mem::size_of::<u8>();

        if src.len() < MIN_SIZE {
            return Ok(None);
        }

        let opcode = src.get_u8();

        let opcode = Opcode::iter()
            .find(|v| *v as u8 == opcode)
            .ok_or(Error::InvalidOpcode(opcode))?;

        let frame = match opcode {
            Opcode::Connect => {
                let mut client_id = [0; 16];
                src.copy_to_slice(&mut client_id[..]);

                Frame::Connect {
                    client_id: client_id.into(),
                }
            }
            Opcode::ConnACK => {
                let response_code = src.get_u8();

                Frame::ConnACK {
                response_code: response_code.try_into().map_err(|_| Error::InvalidOpcode(response_code))?,
                }
            }
        };

        Ok(Some(frame))
    }
}

impl Encoder<Frame> for FrameCodec {
    type Error = Error;

    fn encode(&mut self, item: Frame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            Frame::Connect { client_id } => {
                let client_id: [u8; 16] = client_id.into();
                dst.put_u8(Opcode::Connect as u8);
                dst.put_slice(&client_id[..]);
            }
            Frame::ConnACK { response_code } => {
                dst.put_u8(Opcode::ConnACK as u8);
                dst.put_u8(response_code as u8);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::random;

    fn test_frame_codec(frame: Frame) {
        let mut codec = FrameCodec {};
        let mut bytes = BytesMut::new();
        codec.encode(frame, &mut bytes).unwrap();

        let decoded_frame = codec.decode(&mut bytes).unwrap().unwrap();

        assert_eq!(frame, decoded_frame);
    }

    #[test]
    fn test_connect_codec() {
        let frame = Frame::Connect {
            client_id: random(),
        };
        test_frame_codec(frame)
    }

    #[test]
    fn test_connack_codec() {
        let frame = Frame::ConnACK {
            response_code: random(),
        };
        test_frame_codec(frame)
    }
}
