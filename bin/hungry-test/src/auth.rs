use std::sync::Arc;

use anyhow::bail;

use hungry::tl;

use tl::api::{enums, funcs, types};

use crate::Client;

async fn import_bot_authorization(
    client: Client,
    api_id: i32,
    api_hash: String,
    bot_auth_token: String,
) -> anyhow::Result<Box<types::auth::Authorization>> {
    let func = Arc::new(funcs::auth::ImportBotAuthorization {
        flags: 0,
        api_id,
        api_hash,
        bot_auth_token,
    });

    let obj = client.invoke(func).await?;

    let tl::Object::api_auth_Authorization(enums::auth::Authorization::Authorization(
        authorization,
    )) = obj
    else {
        bail!("invalid type");
    };

    Ok(authorization)
}

pub async fn auth(
    client: Client,
    api_id: i32,
    api_hash: String,
    bot_auth_token: String,
) -> anyhow::Result<()> {
    let func = Arc::new(funcs::InvokeWithLayer {
        layer: 214,
        query: funcs::InitConnection {
            api_id,
            device_model: "device_model".to_string(),
            system_version: "system_version".to_string(),
            app_version: "0.0.1".to_string(),
            system_lang_code: "en".to_string(),
            lang_pack: "".to_string(),
            lang_code: "en".to_string(),
            proxy: None,
            params: None,
            query: funcs::updates::GetState {},
        },
    });

    let obj = client.invoke(func).await?;

    match obj {
        // Not logged it.
        tl::Object::mtproto_RpcError(_) => {
            import_bot_authorization(client, api_id, api_hash, bot_auth_token).await?;
        }
        _ => {}
    }

    Ok::<(), anyhow::Error>(())
}
