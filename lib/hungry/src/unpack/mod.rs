// use std::mem;
// use std::num::{NonZero, NonZeroUsize};
//
// use crate::tl;
//
// #[derive(Default)]
// pub enum MsgContainer<'a> {
//     #[default]
//     Empty,
//     Msg {
//         buf: tl::de::Buf<'a>,
//     },
//     MsgContainer {
//         buf: tl::de::Buf<'a>,
//         len: NonZeroUsize,
//     },
// }
//
// impl<'a> MsgContainer<'a> {
//     pub fn new(mut buf: tl::de::Buf<'a>) -> Self {
//
//     }
// }
//
// impl<'a> Iterator for MsgContainer<'a> {
//     type Item = tl::de::Buf<'a>;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         use self::MsgContainer::*;
//
//         let buf = match self {
//             Empty => return None,
//             Msg { .. } => match mem::take(self) {
//                 Msg { buf } => return Some(buf),
//                 _ => unreachable!(),
//             },
//             MsgContainer { buf, len } => {
//                 let Some((*len)) = NonZero::new(len.get() - 1) else
//                     match mem::take(self) {
//                         MsgContainer { buf, .. } => return Some(buf),
//                         _ => unreachable!(),
//                     }
//                 }
//
//                 buf
//             }
//         };
//     }
// }
