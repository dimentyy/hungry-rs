mod req_dh_params;
mod req_pq_multi;
mod res_pq;

pub mod error;

pub use req_dh_params::ReqDhParams;
pub use req_pq_multi::ReqPqMulti;
pub use res_pq::ResPq;

#[inline]
pub fn start(nonce: crate::tl::Int128) -> ReqPqMulti {
    ReqPqMulti::new(nonce)
}
