use std::fmt;
use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};
use tracing::error;

use hungry::tl;

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
                error_code,
                error_message,
            }) => write!(
                f,
                "rpc_error#2144ca19 {{ error_code: {error_code}, error_message: {error_message} }}"
            ),
            BadServerSalt => write!(f, "bad_server_salt#edab447b {{ .. }}"),
        }
    }
}

impl std::error::Error for RequestError {}

pub struct Request {
    pub tx: oneshot::Sender<Result<tl::Object, RequestError>>,
    pub len: usize,
    pub func: Arc<dyn tl::ser::SerializeUnchecked + Send + Sync>,
}

#[derive(Clone)]
pub struct Client {
    pub tx: mpsc::UnboundedSender<Request>,
}

impl Client {
    pub async fn invoke(
        &self,
        func: Arc<dyn tl::ser::SerializeUnchecked + Send + Sync>,
    ) -> anyhow::Result<tl::Object> {
        let obj = loop {
            let (tx, rx) = oneshot::channel();

            self.tx.send(Request {
                tx,
                len: func.serialized_len(),
                func: Arc::clone(&func) as _,
            })?;

            match rx.await? {
                Ok(obj) => break obj,
                Err(err) => {
                    error!(%err);

                    let secs = match err {
                        RequestError::RpcError(error) => {
                            if error.error_code == 420 {
                                u64::from(
                                    error
                                        .error_message
                                        .split(|c: char| !c.is_ascii_digit())
                                        .flat_map(|value| value.parse::<u32>())
                                        .next()
                                        .unwrap_or(0),
                                ) + 1
                            } else {
                                1
                            }
                        }
                        RequestError::BadServerSalt => 1,
                    };

                    tokio::time::sleep(tokio::time::Duration::from_secs(secs)).await;
                }
            }
        };

        Ok(obj)
    }
}
