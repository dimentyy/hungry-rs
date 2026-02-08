use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::info;

use hungry::{crypto_bigint, tl, unbite};

use crypto_bigint::{Odd, U2048};

use crate::Plain;

/// The E component of public RSA key.
const E: u64 = 65537;

/// The N component of public RSA key.
const N: &str = "253428894488404155649716895907134732068988477590847790525820265945460224638539\
    4058588521595116849196570822264939918060381807420062046377613542488463216251240316379308392\
    1641631564740959529419359595852941166848940585952337613333022396096584117954892216031229237\
    3029437018775884567383353986024616752250817918203931537575049526362349513232378200365435810\
    4782690612092797248736680529211579223142368426126233039432475078545094258975175539015664775\
    1460719351439969059949569615302809050721500330239005077889855323917509948255722081644689442\
    127297605422579707142646660768825302832201908302295573257427896031830742328565032949";

/// Key generation is not fun. Try exploring other things!
async fn generate_auth_key(
    plain: &mut Plain,
    buffer: &mut unbite::DynBuf,
) -> anyhow::Result<hungry::auth::DhGenOk> {
    info!("generating new `AuthKey`");

    let n = Odd::new(U2048::from_str_radix_vartime(N, 10)?).unwrap();
    let e = Odd::new(U2048::from_u64(E)).unwrap();

    let server_public_key = hungry::crypto::RsaKey::new(n, e); // fingerprint: -5595554452916591101

    let mut nonce = tl::Int128::default();

    getrandom::fill(nonce.as_mut())?;

    let req_pq_multi = hungry::auth::start(nonce);

    let tl::mtproto::enums::ResPq::ResPq(res_pq) = plain.send(buffer, req_pq_multi.func()).await?;

    let res_pq = req_pq_multi.res_pq(&res_pq)?;

    let mut random_padding_bytes = [0; 192];
    getrandom::fill(&mut random_padding_bytes)?;

    let mut new_nonce = tl::Int256::default();
    getrandom::fill(new_nonce.as_mut())?;

    let mut req_dh_params =
        res_pq.req_dh_params(random_padding_bytes, new_nonce, &server_public_key);

    let mut temp_key = [0; 32];

    let func = loop {
        getrandom::fill(&mut temp_key)?;

        if let Some(func) = req_dh_params.func(&temp_key) {
            break func;
        }
    };

    let tl::mtproto::enums::ServerDhParams::ServerDhParamsOk(server_dh_params) =
        plain.send(buffer, func).await?
    else {
        anyhow::bail!("not ServerDhParamsOk")
    };

    let server_dh_params = req_dh_params.server_dh_params_ok(&server_dh_params)?;

    let mut b = [0; 256];
    getrandom::fill(&mut b)?;

    let set_client_dh_params =
        server_dh_params.set_client_dh_params(U2048::from_be_slice(&b), 0)?;

    let tl::mtproto::enums::SetClientDhParamsAnswer::DhGenOk(dh_gen_ok) =
        plain.send(buffer, set_client_dh_params.func()).await?
    else {
        anyhow::bail!("not DhGenOk")
    };

    Ok(set_client_dh_params.dh_gen_ok(&dh_gen_ok)?)
}

/// Use AuthKey from a file, if it exists, or generate a new one.
pub async fn get_auth_key(
    plain: &mut Plain,
    buffer: &mut unbite::DynBuf,
) -> anyhow::Result<(hungry::mtproto::AuthKey, hungry::mtproto::Salt)> {
    let filename = "test.session";

    let path = std::path::Path::new(filename);

    let mut file = tokio::fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .await?;

    let mut buf = Vec::new();

    file.read_to_end(&mut buf).await?;

    if buf.len() != 256 {
        let hungry::auth::DhGenOk {
            auth_key,
            server_salt,
        } = generate_auth_key(plain, buffer).await?;

        info!(filename, "writing `AuthKey` to the file");

        file.set_len(0).await?;
        file.write_all(auth_key.data()).await?;

        return Ok((auth_key, server_salt));
    }

    info!(filename, "using the `AuthKey` from the file`");

    let Some(auth_key) = hungry::mtproto::AuthKey::new(buf.try_into().unwrap()) else {
        anyhow::bail!("auth key zero");
    };

    // We could store the future salt, but it would be too messy for an example.
    let server_salt = getrandom::u64()?.cast_signed();

    Ok((auth_key, server_salt))
}
