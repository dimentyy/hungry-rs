# Telegram MTProto API client in Rust with the main focus on <ins>reliability</ins>.

###### Binaries
* [hungry-test](/bin/hungry-test) — binary for testing the libraries

###### Libraries
* [hungry](/lib/hungry) — connection and protocol logic, client
* [hungry-tl](/lib/hungry-tl) — generated TL schema with their traits
* [hungry-tl-gen](/lib/hungry-tl-gen) — building library for `hungry-tl`

* [unbite](/lib/unbite) — efficient operations on contiguous slices of memory allocations (replacement for `bytes`)  

## TODO:

- [ ] Wrapper for storing precalculated serialized length.
- [ ] Safe serialization buffer. (`&mut [MaybeUninit<u8>]` => `&mut [u8]`)


- [ ] Write safety comments.
- [ ] Add other points here.

> The repository will be recreated once this project is complete and refactored.

## License

This project is licensed under the MIT license.
