use std::fmt;
use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};
use tracing::error;

use hungry::tl;

pub trait Function: tl::ser::SerializeUnchecked + fmt::Debug + Send + Sync {}
impl<X: tl::ser::SerializeUnchecked + fmt::Debug + Send + Sync> Function for X {}

#[derive(Debug)]
pub enum RequestError {
    RpcError(tl::mtproto::types::RpcError),
    BadServerSalt,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use RequestError::*;

        f.write_str("request error: ")?;

        match self {
            RpcError(tl::mtproto::types::RpcError {
                error_code: code,
                error_message: message,
            }) => write!(
                f,
                "rpc_error#2144ca19 {{ code: {code}, message: {message} }}"
            ),
            BadServerSalt => write!(f, "bad_server_salt#edab447b {{ .. }}"),
        }
    }
}

impl std::error::Error for RequestError {}

pub struct Request {
    pub tx: oneshot::Sender<Result<tl::Object, RequestError>>,
    pub len: usize,
    pub func: Arc<dyn Function>,
}

#[derive(Clone)]
pub struct Client {
    pub tx: mpsc::UnboundedSender<Request>,
}

impl Client {
    pub async fn invoke(&self, func: Arc<dyn Function>) -> anyhow::Result<tl::Object> {
        let len = func.serialized_len();

        loop {
            let (tx, rx) = oneshot::channel();

            // The `Sender` does not store requests, just `Msg` and `Extra`.
            // They are only serialized. When sent, a buffer is cleared.
            // Since RPC queries are not expected to fail often, this is
            // an efficient way to avoid copying buffer intermediately.
            self.tx.send(Request {
                tx,
                len,
                func: Arc::clone(&func),
            })?;

            let err = match rx.await? {
                Ok(obj) => return Ok(obj),
                Err(err) => err,
            };

            error!(%err);

            let secs = match err {
                RequestError::RpcError(error) if error.error_code == 420 => {
                    u64::from(
                        error
                            .error_message
                            .split(|c: char| !c.is_ascii_digit())
                            .flat_map(|value| value.parse::<u32>())
                            .next()
                            .unwrap_or(1),
                    ) + 1
                }
                RequestError::RpcError(error) => return Err(RequestError::RpcError(error).into()),
                RequestError::BadServerSalt => 2,
            };

            tokio::time::sleep(tokio::time::Duration::from_secs(secs)).await;
        }
    }
}
