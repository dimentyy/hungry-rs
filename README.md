# Telegram MTProto API client in Rust with the main focus on <ins>reliability</ins>.

###### Binaries
* [hungry-test](/bin/hungry-test) — binary for testing the libraries

###### Libraries
* [hungry](/lib/hungry) — connection and protocol logic, client
* [hungry-tl](/lib/hungry-tl) — generated TL schema with their traits
* [hungry-tl-gen](/lib/hungry-tl-gen) — building library for `hungry-tl`

###### To be separated?
* [unbite](/lib/unbite) — efficient operations on contiguous slices of memory allocations (replacement of `bytes` for `hungry`)  

## TODO:

- [ ] Generate an enum with all bare types (determined by `CONSTRUCTOR_ID`) wrapped in a `Box` to catch deserialization failures early.
- [ ] Wrapper for storing precalculated serialized length.
- [ ] `unbite::DynRaw` container to easily unsplit buffers after they are received?
- [ ] Write safety comments.
- [ ] Remove `todo!()` panics.
- [ ] Add other points here.
- [ ] Scrape documentation for types and functions?
- [ ] Support Gzip containers.
- [ ] Logging. `tracing` / `log`?
- [ ] Lower amount of unsafe code in `hungry-tl` (current: ~5170).
- [ ] Actual documentation.
+ [ ] Plain sender.
- [ ] **Safe** serialization buffer. (`&mut [MaybeUninit<u8>]` => `&mut [u8]`)
  - [x] Base structure. See `hungry_tl::ser::buf`.
+ [ ] Stabilize `unbite` crate.
  - [ ] Guarantee memory-safety.
  - [ ] Reduce split/unsplit mess.
- [ ] Follow all [security guidelines](https://core.telegram.org/mtproto/security_guidelines).
  - [x] "Checking SHA256 hash value of msg_key".
  - [ ] "Checking message length".
  - [ ] "Checking msg_id".
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
  - [x] Full. (quick ACK is unavailable?)
  - [x] Obfuscation.
    - [ ] Test.
  - [ ] Support [quick ACKs](https://core.telegram.org/mtproto/mtproto-transports#quick-ack) higher than transport functions.
+ [ ] Nice way to `TransportInit::init`. Future constructed with a buffer that returns an owned `Writer` / `QueuedWriter` with a buffer queued (without futures to await)?
  - [x] `OwnedWrite`?

> The repository will be recreated once this project is complete and refactored.

## License

This project is licensed under the MIT license.
