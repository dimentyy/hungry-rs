use hungry::tl;

use tokio::sync::oneshot;

use tracing::warn;

use crate::RequestError;

pub struct Handle {}

impl hungry::sender::Handle for Handle {
    /// Additional one-shot result sender, stored with requests in `Sender`.
    type Extra = oneshot::Sender<Result<tl::Object, RequestError>>;

    fn rpc_result(&mut self, extra: Self::Extra, typ: u32, buf: &mut tl::de::Buf<'_>) {
        let obj = match tl::Object::deserialize(typ, buf) {
            Ok(x) => x,
            Err(err) => todo!("{err}"),
        };

        if extra.send(Ok(obj)).is_err() {
            warn!("request receiver is closed");
        }
    }

    fn rpc_result_error(&mut self, extra: Self::Extra, error: tl::mtproto::types::RpcError) {
        if extra.send(Err(RequestError::RpcError(error))).is_err() {
            warn!("request receiver is closed");
        }
    }

    fn bad_server_salt(&mut self, extra: Self::Extra) {
        if extra.send(Err(RequestError::BadServerSalt)).is_err() {
            warn!("request receiver is closed");
        }
    }
}
