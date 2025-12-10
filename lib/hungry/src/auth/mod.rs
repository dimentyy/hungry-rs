//! TODO: security checks, retries
#![allow(unused)]

mod error;
mod req_dh_params;
mod req_pq_multi;
mod res_pq;
mod server_dh_params;
mod set_client_dh_params;

use crate::tl;

use tl::mtproto::funcs;
use tl::Int128;

pub use error::NonceMismatch;
pub use req_dh_params::ReqDhParams;
pub use req_pq_multi::ReqPqMulti;
pub use res_pq::ResPq;
pub use server_dh_params::ServerDhParams;
pub use set_client_dh_params::SetClientDhParams;

#[inline]
#[must_use]
pub fn start(nonce: Int128) -> ReqPqMulti {
    let func = funcs::ReqPqMulti { nonce };

    ReqPqMulti { func }
}
