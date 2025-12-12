# Telegram MTProto API client in Rust with the main focus on <ins>reliability</ins>

###### Binaries
* [hungry-test](/bin/hungry-test) — binary for testing the libraries

###### Libraries
* [hungry](/lib/hungry) — connection and protocol logic, client
* [hungry-tl](/lib/hungry-tl) — generated TL schema with their traits
* [hungry-tl-gen](/lib/hungry-tl-gen) — building library for `hungry-tl`

Run test program yourself! *(auth key creation, `get_future_salts#b921bd04`)*

**Check out the [main.rs](/bin/hungry-test/src/main.rs)**

```shell
cargo run --bin hungry-test
```

The repository will be recreated once this project is complete and refactored.
