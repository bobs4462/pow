use serde::{Deserialize, Serialize};

pub struct Message<T> {
    header: Header,
    payload: T,
}

pub enum DeserializedMessage {
    Request(Request),
    Challenge(Challenge),
    Solution(Solution),
    Response(Response),
}

#[repr(C)]
pub struct Header {
    pub length: u32,
    pub kind: MessageKind,
}

#[repr(C)]
pub enum MessageKind {
    Request = 0,
    Challenge,
    Solution,
    Response,
}

/// Client initiated request
#[derive(Serialize, Deserialize)]
pub struct Request {
    /// quote number modulo total count of quotes
    pub number: Option<usize>,
}

/// Server generated challenge
#[derive(Serialize, Deserialize)]
pub struct Challenge {
    /// session number assigned by server
    pub session: u32,
    /// random string for PoW
    pub string: Vec<u8>,
    /// target nonce
    pub target: u128,
}

/// Client generated solution for PoW puzzle
#[derive(Serialize, Deserialize)]
pub struct Solution {
    /// session number to identify particule puzzle solving flow
    pub session: u32,
    /// client found number, which solved the puzzle
    pub nonce: u128,
}

/// Server response, containing requested quote, after puzzle has been solved
#[derive(Serialize, Deserialize)]
pub struct Response {
    pub session: u32,
    pub result: Result<String, String>,
}

impl MessageKind {
    pub fn parse_from_bytes(&self, bytes: &[u8]) -> bincode::Result<DeserializedMessage> {
        let msg = match self {
            MessageKind::Request => bincode::deserialize::<Request>(bytes)?.into_message(),
            MessageKind::Challenge => bincode::deserialize::<Challenge>(bytes)?.into_message(),
            MessageKind::Solution => bincode::deserialize::<Solution>(bytes)?.into_message(),
            MessageKind::Response => bincode::deserialize::<Response>(bytes)?.into_message(),
        };
        Ok(msg)
    }
}

pub trait Serializeable {
    fn kind(&self) -> MessageKind;
    fn into_message(self) -> DeserializedMessage;
}

macro_rules! serializeable {
    ($type: ident) => {
        impl Serializeable for $type {
            fn kind(&self) -> MessageKind {
                MessageKind::$type
            }

            fn into_message(self) -> DeserializedMessage {
                DeserializedMessage::$type(self)
            }
        }
    };
}

serializeable!(Request);
serializeable!(Challenge);
serializeable!(Solution);
serializeable!(Response);

impl<'de, T: Serialize + Deserialize<'de> + Serializeable> Message<T> {
    #[inline]
    pub fn new(payload: T) -> Self {
        let length = bincode::serialized_size(&payload).unwrap() as u32;
        let kind = payload.kind();
        let header = Header { length, kind };
        Self { header, payload }
    }
}

impl<T> Message<T> {
    #[inline]
    pub fn header(&self) -> &Header {
        &self.header
    }

    #[inline]
    pub fn payload(&self) -> &T {
        &self.payload
    }
}
