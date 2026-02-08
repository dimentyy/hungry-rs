# `HUNGRY-RS`

Telegram [MTProto] API client in Rust, with **RELIABILITY** as the top priority.

Ideas for this project were massively inspired by ★ [gramme.rs] libraries.

![LINES OF CODE](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fapi.codetabs.com%2Fv1%2Floc%2F%3Fgithub%3Ddimentyy%2Fhungry-rs&query=%24%5B0%5D.linesOfCode&style=for-the-badge&label=LINES%20OF%20CODE&color=%23ff6600)

## Overview

This project is **NOT** a framework — it's a library, meaning there is **NO**
session management, input peer cache, or any high-level convenient API methods.
The user shall adapt the library for their own specific use-case, rather than
their projects, with `HUNGRY-RS` providing a stable base for Telegram clients.

## <ins>Working</ins> example

See the actually working [`hungry-test`](/bin/hungry-test) binary right now.

It contains just a few files with comments to get an
understanding of how the library is actually working!

This is just a simple echo-bot with a `/stats` command.

## Unbite

See the [crate](/lib/unbite), currently in this repository. This is a
replacement of `bytes`, designed for efficient, compile-time checked
operations on contiguous memory, with support for constant-sized buffers.

## Current status

Work in progress.

#### Milestones:

- [x] TL generation
- [x] MTProto Transports
- [x] Reader, Writer
- [ ] Sender
  - [x] Container
  - [x] Deserialization
  - [ ] Gzip compression
  - [ ] Salts
  - [ ] Acknowledgment of Receipt
  - [ ] RPC results
  - [ ] Error recovery
- [ ] Client

+ [ ] Stabilization
+ [ ] Active testing

## Libraries
* [`hungry`](/lib/hungry) — client, connection and protocol logic
* [`hungry-tl`](/lib/hungry-tl) — generated TL-schema, their traits
* [`hungry-tl-gen`](/lib/hungry-tl-gen) — TL-generator for `hungry-tl`

## Todo

- [x] **Safe** serialization buffer. (`&mut [MaybeUninit<u8>]` => `&mut [u8]`)
- [ ] Generate an enum with all bare types (determined by `CONSTRUCTOR_ID`) to catch deserialization failures early.
  - [x] Box large variants.
  - [ ] Optimize this monster.
- [ ] Wrapper for storing precalculated serialized length.
- [ ] `unbite::DynRaw` container to easily unsplit buffers after they are received?
- [ ] Write safety comments.
- [ ] Scrape documentation for types and functions?
- [ ] Support Gzip containers.
- [ ] Logging. `tracing` / `log`?
- [ ] Lower amount of unsafe code in `hungry-tl` (current: ~5170).
- [ ] Actual documentation.
+ [ ] Remove TODOs.
  - `todo!()` & `.unwrap()` panics.
  - `// TODO:` comments.
- [ ] Plain sender.
  + [x] Base. (`hungry::plain`)
+ [ ] Stabilize `unbite` crate.
  - [ ] Guarantee memory-safety.
  - [ ] Reduce split/unsplit mess.
- [ ] Follow all [security guidelines](https://core.telegram.org/mtproto/security_guidelines).
  - [ ] Diffie-Hellman key exchange
  - [ ] MTProto Encrypted Messages
    - [x] Checking SHA256 hash value of msg_key.
    - [ ] Checking message length.
    - [x] Checking session_id.
    - [x] Checking msg_id.
  + [x] Seq no checking.
+ [ ] Authorization / Sign in.
  - [ ] Full error handling.
  - [ ] Auth key generation.
  - [ ] Bot sign in via token.
  - [ ] Connection to different DC?
- [ ] Transport.
  - [ ] Abridged.
  - [x] Intermediate.
    - [x] Quick ACK.
    - [ ] Test.
  - [ ] Padded intermediate.
  - [x] Full.
  - [x] Obfuscation.
    - [ ] Test.
  - [ ] Support [quick ACKs](https://core.telegram.org/mtproto/mtproto-transports#quick-ack) higher than transport functions.
+ [x] Nice way to initialize transport.
  - [x] `OwnedWrite`?
- [ ] Encrypted sender.
  - [x] Message container.
  - [ ] Gzipping.
+ [ ] "Zero-Copy" file uploading?
- [ ] Handle `msgs_ack`.
+ [ ] Salt management.
- [ ] `Send`able and `Clone`able `Client` to use the `Sender`.
+ [ ] Use `#[forbid(clippy::todo)]` and `#[forbid(unsafe_code)]` as much as possible.

> The repository will be recreated once this project is complete and refactored.

## License

This project is licensed under the MIT license.

[MTProto]: https://core.telegram.org/mtproto
[gramme.rs]: https://github.com/lonami/grammers
