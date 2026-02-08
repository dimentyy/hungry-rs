use std::sync::Arc;

use tracing::info;

use hungry::tl;

use tl::api::{enums, funcs, types};

use crate::{Client, RequestError};

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
        anyhow::bail!("invalid type");
    };

    info!(?authorization, "authorized");

    Ok(authorization)
}

pub async fn authorize(
    client: Client,
    api_id: i32,
    api_hash: String,
    bot_auth_token: String,
) -> anyhow::Result<()> {
    // The `InvokeWithLayer` and `InitConnection` functions
    // are crucial to call before continuing with anything else.
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

    match client.invoke(func).await {
        // Authorized, not need to call `auth::ImportBotAuthorization` function.
        Ok(_) => {
            info!("already authorized");
            return Ok(());
        }
        Err(err) => match err.downcast::<RequestError>() {
            // Unauthorized.
            Ok(RequestError::RpcError(error)) if error.error_code == 401 => {}

            // Other errors.
            Ok(err) => return Err(err.into()),
            Err(err) => return Err(err),
        },
    }

    import_bot_authorization(client, api_id, api_hash, bot_auth_token).await?;

    Ok(())
}
