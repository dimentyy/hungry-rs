fn main() {
    let mtproto_schema = std::fs::read_to_string("schema/mtproto.tl").unwrap();

    let mtproto_config = hungry_tl_gen::Config {
        derive_clone: false,
        derive_debug: true,
        schema_title: "mtproto".to_owned(),
    };

    hungry_tl_gen::generate(mtproto_config, &mtproto_schema);

    // let api_schema = std::fs::read_to_string("schema/api.tl").unwrap();
    //
    // let api_config = hungry_tl_gen::Config {
    //     derive_clone: true,
    //     derive_debug: true,
    //     schema_title: "api".to_owned(),
    // };
    //
    // hungry_tl_gen::generate(api_config, &api_schema);
}
