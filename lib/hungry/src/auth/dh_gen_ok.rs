use crate::mtproto;

#[must_use]
#[derive(Debug, Eq, PartialEq)]
pub struct DhGenOk {
    pub auth_key: mtproto::AuthKey,
    pub server_salt: mtproto::Salt,
}
