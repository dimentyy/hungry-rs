//! TODO: security checks, retries
#![allow(unused)]

mod req_dh_params;
mod req_pq_multi;
mod res_pq;
mod server_dh_params_ok;
mod set_client_dh_params;

pub mod error;

use crate::tl;

use tl::Int128;

pub use req_dh_params::ReqDhParams;
pub use req_pq_multi::ReqPqMulti;
pub use res_pq::ResPq;
pub use server_dh_params_ok::ServerDhParamsOk;
pub use set_client_dh_params::SetClientDhParams;

#[inline]
pub fn start(nonce: Int128) -> ReqPqMulti {
    ReqPqMulti::new(nonce)
}
