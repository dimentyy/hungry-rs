## A set of Rust libraries for using Telegram API via MTProto with the main focus on reliability.

###### Binaries
* [hungry-test](/bin/hungry-test) — binary for testing the libraries

###### Libraries
* [hungry](/lib/hungry) — connection and protocol logic, client
* [hungry-tl](/lib/hungry-tl) — generated TL schema with their traits
* [hungry-tl-gen](/lib/hungry-tl-gen) — building library for `hungry-tl`

Run test program yourself! (send `req_pq_multi#be7e8ef1` and recv `ResPQ`)

**Check out the [main.rs](/bin/hungry-test/src/main.rs)**

```shell
cargo run --bin hungry-test
```

The repository will be recreated once this project is complete and refactored. ⚠️
