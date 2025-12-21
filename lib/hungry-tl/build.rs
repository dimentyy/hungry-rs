fn main() {
    let mtproto = std::fs::read_to_string("schema/mtproto.tl").unwrap();
    let api = std::fs::read_to_string("schema/api.tl").unwrap();

    let config = hungry_tl_gen::Config {
        impl_debug: true,
        derive_clone: true,
        impl_into_enum: true,
    };

    hungry_tl_gen::generate(
        config,
        vec!["mtproto".to_owned(), "api".to_owned()],
        &[&mtproto, &api],
    );
}
