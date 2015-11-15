use std::sync::mpsc::{
    self,
    SyncSender,
    TrySendError,
    RecvError
};

use serde::{Serialize, Deserialize};
use bincode;
use bincode::serde::{
    SerializeError,
    DeserializeError,
    serialize,
    deserialize,
};

mod track;

pub use self::track::{
    EnqueueTrackRequest,
    EnqueueTrackError,
    TrackSkipToEndRequest,
    TrackSkipToEndError,
};

pub const SIZE_LIMIT: bincode::SizeLimit = bincode::SizeLimit::Bounded(20 * 1 << 20);

pub enum RequestType {
    EnqueueTrack,
    TrackSkipToEnd,
}

pub struct RequestWrapper {
    // the Vec<u8>s are a bincode serialized representation
    pub response_queue: mpsc::SyncSender<Vec<u8>>,
    pub req_type: RequestType,
    pub req_buf: Vec<u8>,
}

pub type BinderResult<T, E> = Result<T, BinderError<E>>;

// wire-safe error wrapper. converted to ProxyError afterwards.

#[derive(Serialize, Deserialize)]
pub enum BinderError<T> where T: Serialize + Deserialize {
    CallError(T),
    StubImplementation,
    RemoteSerdeError,
}

impl<T> From<ProxyError<T>> for BinderError<T> where T: Serialize + Deserialize {
    fn from(e: ProxyError<T>) -> Self {
        match e {
            ProxyError::CallError(val) => BinderError::CallError(val),
            ProxyError::StubImplementation => BinderError::StubImplementation,
            ProxyError::RemoteSerdeError => BinderError::RemoteSerdeError,
            ProxyError::RpcError(_) => BinderError::RemoteSerdeError,
        }
    }
}

pub type ProxyResult<T, E> = Result<T, ProxyError<E>>;

pub enum ProxyError<T> {
    CallError(T),
    StubImplementation,
    RemoteSerdeError,
    RpcError(RpcError),
}

impl<T> From<BinderError<T>> for ProxyError<T> where T: Serialize + Deserialize {
    fn from(e: BinderError<T>) -> Self {
        match e {
            BinderError::CallError(val) => ProxyError::CallError(val),
            BinderError::StubImplementation => ProxyError::StubImplementation,
            BinderError::RemoteSerdeError => ProxyError::RemoteSerdeError,
        }
    }
}

impl<T> From<RpcError> for ProxyError<T> {
    fn from(e: RpcError) -> Self {
        ProxyError::RpcError(e)
    }
}

pub enum RpcError {
    SendError(TrySendError<()>),
    RecvError(RecvError),
    SerializeError(SerializeError),
    DeserializeError(DeserializeError),
}

impl<T> From<TrySendError<T>> for RpcError {
    fn from(e: TrySendError<T>) -> Self {
        match e {
            TrySendError::Disconnected(_) => {
                RpcError::SendError(TrySendError::Disconnected(()))
            },
            TrySendError::Full(_) => {
                RpcError::SendError(TrySendError::Full(()))
            }
        }
    }
}

impl From<RecvError> for RpcError {
    fn from(e: RecvError) -> Self {
        RpcError::RecvError(e)
    }
}

impl From<SerializeError> for RpcError {
    fn from(e: SerializeError) -> Self {
        RpcError::SerializeError(e)
    }
}

impl From<DeserializeError> for RpcError {
    fn from(e: DeserializeError) -> Self {
        RpcError::DeserializeError(e)
    }
}

pub trait Request: Serialize + Deserialize + Sized {
    type Value: Serialize + Deserialize;
    type Error: Serialize + Deserialize;

    fn req_type(&self) -> RequestType;
}

struct CoreProxy {
    sender: mpsc::SyncSender<RequestWrapper>
}

fn serialize_req<R: Request>(req: R) -> Result<Vec<u8>, RpcError> {
    Ok(try!(serialize(&req, SIZE_LIMIT)))
}

impl CoreProxy {
    pub fn execute<R: Request>(&mut self, req: R) -> ProxyResult<R::Value, R::Error> {
        let (tx, rx) = mpsc::sync_channel(1);
        let req_type = req.req_type();
        let req_buf = try!(serialize_req(req));

        let wrapper = RequestWrapper {
            response_queue: tx,
            req_type: req_type,
            req_buf: req_buf,
        };
        try!(self.sender.try_send(wrapper).map_err(RpcError::from));
        let resp_buf = try!(rx.recv().map_err(RpcError::from));
        let resp_res: Result<R::Value, R::Error> =
            try!(deserialize(&resp_buf).map_err(RpcError::from));;
        resp_res.map_err(ProxyError::CallError)
    }
}
