//! ESP3 packet encoding and decoding


use std::{str::{Utf8Error, FromStr}, fmt::Display};

use num_enum::{TryFromPrimitive, IntoPrimitive};
use thiserror::Error;

use crate::{frame::{ESP3Frame, ESP3FrameRef}, enocean::Rorg};

pub type ResponseCode = crate::enocean::ReturnCode;

#[derive(Debug,Clone,Copy,Eq,PartialEq,Hash)]
pub struct Address([u8; 4]);

pub const BROADCAST: Address = Address([0xff,0xff,0xff,0xff]);

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02x}{:02x}{:02x}{:02x}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

impl FromStr for Address {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut address = [0; 4];
        hex::decode_to_slice(s, &mut address)?;
        Ok(Self(address))
    }
}


pub struct EEPProfileCode([u8; 3]);

#[derive(Debug,Error)]
pub enum ParseError {
    #[error("Unsupported packet type")] UnsupportedPacketType,
    #[error("Packet too short")]        PacketTooShort,
    #[error("UTF8 decoding Error")]     UTF8(#[from] Utf8Error),
    #[error("Invalid result code")]     InvalidResultCode(u8),
    #[error("Invalid primitive")]       InvalidPrimitive,
}

#[derive(Debug,Clone,Copy,PartialEq,Eq,TryFromPrimitive,IntoPrimitive)]
#[repr(u8)]
pub enum SubtelNum {
    Send = 3, 
    Receive = 0,
}

#[derive(Debug,Clone,Copy,PartialEq,Eq,TryFromPrimitive,IntoPrimitive)]
#[repr(u8)]
pub enum Security {
    None = 0,
    Obsolete = 1,
    Decrypted = 2,
    Authenticated = 3,
    AuthAndDecrypted = 4,
}

#[derive(Debug,Clone,Copy)]
pub struct RadioErp1<'a> {
    pub choice: Rorg,
    pub user_data: &'a [u8],
    pub sender_id: Address,
    pub status: u8,
    pub subtel_num: Option<SubtelNum>,
    pub destination: Option<Address>,
    pub rssi: Option<u8>,
    pub security: Option<Security>
}

#[derive(Debug,Clone,Copy)]
// TODO parse details
pub enum Event<'a> {
    SAReclaimUnsuccessful,
    SAConfirmLearn       { data: &'a [u8; 17] }, 
    SALearnAck           { data: &'a [u8; 3]},
    COReady              { wakeup: u8, mode: Option<u8> },
    COEventSecureDevices { cause: u8, device: Address },
    CODutyCycleLimit     { cause: u8},
    COTXFailed           { cause: u8},
    COTXDone,
    COLrnModeDisabled,
}

#[derive(Debug,Clone)]
pub struct Response {
    pub code: ResponseCode,
    pub data: Vec<u8>,
}

#[derive(Debug,Clone,Copy)]
pub struct Version {
    pub main: u8,
    pub beta: u8,
    pub alpha: u8,
    pub build: u8,
}

#[derive(Debug,Clone)]
pub struct VersionResponse {
    pub app: Version,
    pub api: Version,
    pub chip_id: Address,
    pub chip_version: [u8; 4],
    pub description: String,
}

#[derive(Debug,Clone,Copy)]
pub enum CommonCommand<'a> {
    //Reset,
    ReadVersion,
    //ReadSystemLog,

    Unknown { code: u8, data: &'a [u8], optional: &'a [u8] }
}

#[derive(Debug,Clone)]
pub enum Packet<'a> {
    RadioErp1(RadioErp1<'a>),
    Response(Response),
    //Event(Event<'a>),
    CommonCommand(CommonCommand<'a>),
    //SmartAck,
    //RemoteMan,
    //RadioMessage,
    //RadioErp2,
    //CommandAccepted,
    //RadioLRWPAN,
    //Command24GHz,

    Unknown { packet_type: u8, data: &'a [u8], optional: &'a [u8] }
    //RadioSubTel(RadioSubTel),
}

impl VersionResponse {
    pub fn encode(&self) -> Response {
        todo!();
    }

    pub fn decode(response: &Response) -> Result<Self, ParseError> {

        fn fromcstr(s: &[u8]) -> Result<String, Utf8Error> {
            let mut idx = 0;
            while idx < s.len() && s[idx] == 0 { idx += 1 };
            Ok(std::str::from_utf8(&s[..idx])?.to_owned())
        }

        let d = &response.data;
        if d.len() != 32 {
            return Err(ParseError::PacketTooShort)
        }

        Ok(Self {
            app: Version { main: d[0], beta: d[1], alpha: d[2], build: d[3] },
            api: Version { main: d[4], beta: d[5], alpha: d[6], build: d[7] },
            chip_id: Address(d[8..12].try_into().unwrap()),
            chip_version: d[12..16].try_into().unwrap(),
            description: fromcstr(&d[16..32])?,
        })

    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}.{}", self.main, self.beta, self.alpha, self.build)
    }
}

impl Display for VersionResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (APP:{}, API:{}, Chip address:{}, version {:?}", self.description, self.app, self.api, self.chip_id, self.chip_version)
    }
}

impl<'a> RadioErp1<'a> {
    pub fn encode(&self) -> ESP3Frame {
        todo!()

    }

    pub fn decode(frame: ESP3FrameRef<'a>) -> Result<Self, ParseError> {
        let payload_len = frame.data.len() - 6;
        let opt_len = frame.optional_data.len();
        Ok(Self { choice: Rorg::try_from_primitive(frame.data[0]).map_err(|_| ParseError::UnsupportedPacketType)?,
                  user_data: &frame.data[1..][..payload_len],
                  sender_id: Address(frame.data[1+payload_len..][..4].try_into().unwrap()),
                  status: frame.data[5+payload_len],
                  subtel_num: if opt_len >= 1 { Some(SubtelNum::try_from_primitive(frame.optional_data[0]).map_err(|_| ParseError::InvalidPrimitive)?) } 
                              else { None },
                  destination: if opt_len >= 5 { Some(Address(frame.optional_data[1..5].try_into().unwrap())) } else { None },
                  rssi: if opt_len >= 6 { Some(frame.optional_data[5]) } else { None },
                  security: if opt_len >= 7 { Some(Security::try_from_primitive(frame.optional_data[6]).map_err(|_| ParseError::InvalidPrimitive)?) } else { None }
        })
    }
}

impl Response {

    pub fn encode(&self) -> ESP3Frame {
        todo!()
    }

    pub fn decode(frame: ESP3FrameRef) -> Result<Self, ParseError> {
        let code = ResponseCode::try_from_primitive(frame.data[0])
            .map_err(|_| ParseError::InvalidResultCode(frame.data[0]))?;
        let data = frame.data[1..].into();
        Ok( Self { code, data })
    }

}

impl<'a> CommonCommand<'a> {

    fn assemble(code: u8, data: &[u8], optional: &[u8]) -> ESP3Frame {
        let packet_type = 0x05;
        let mut frame_data = vec![code];
        frame_data.extend_from_slice(data);
        ESP3Frame::assemble(packet_type, &frame_data, optional)
    }

    fn encode(&self) -> ESP3Frame {
        match self {
            &Self::Unknown { code, data, optional } => CommonCommand::assemble(code, data, optional),
            &Self::ReadVersion => CommonCommand::assemble(0x03, &[], &[]),
        }
    }
}

impl<'a> Packet<'a> {
    pub fn encode(&self) -> ESP3Frame {

        use Packet::*;
        match &self {
            &RadioErp1(erp) => erp.encode(),
            &CommonCommand(cmd) => cmd.encode(),
            &Response(resp) => resp.encode(),
            &Unknown { packet_type, data, optional } => ESP3Frame::assemble(*packet_type, data, optional),
        }       
    }

    pub fn decode(frame: ESP3FrameRef<'a>) -> Result<Self, ParseError> {
        match frame.packet_type {
            0x01 => Ok(Self::RadioErp1(RadioErp1::decode(frame)?)),
            0x02 => Ok(Self::Response(Response::decode(frame)?)),
            _    => Err(ParseError::UnsupportedPacketType),
        }
    }

}

