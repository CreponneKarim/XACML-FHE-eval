use reqwest::Client;
use rkyv::rancor;
use tfhe::{ClientKey, CompressedPublicKey, CompressedServerKey, Config, ConfigBuilder, FheAsciiString, FheBool, FheUint, FheUint64, FheUint64Id, PublicKey, ServerKey, generate_keys, prelude::{FheDecrypt, FheEncrypt, FheEq, FheTryEncrypt}, safe_serialization::safe_deserialize, set_server_key};
use uuid::Uuid;

pub struct UserConfig {
    pub config: Config,
    pub client_key: Option<ClientKey>,
    pub public_key: PublicKey,
    pub server_key: ServerKey
}


impl UserConfig {
    pub fn init() -> UserConfig {
        let config = ConfigBuilder::default().build();
        let (cks, sks) = generate_keys(config);
        set_server_key(sks.clone());
        let public_key = PublicKey::new(&cks);

        let config = UserConfig {
            config: config,
            client_key: Some(cks),
            public_key: public_key,
            server_key: sks
        };

        config
    }

    pub fn from_fetched(
        pk: PublicKey,
        sks: ServerKey
    ) -> UserConfig {

        let config = ConfigBuilder::default().build();
        set_server_key(sks.clone());

        let config = UserConfig {
            config: config,
            client_key: None,
            public_key: pk,
            server_key: sks
        };

        config
    }
}

pub fn pairwise_reduce<T, F>(mut v: Vec<T>, mut combine: F) -> Option<T>
where
    F: FnMut(T, T) -> T,
{
    if v.is_empty() {
        return None;
    }

    while v.len() > 1 {
        let mut next = Vec::with_capacity((v.len() + 1) / 2);

        // consume v in pairs
        let mut it = v.into_iter();
        while let Some(a) = it.next() {
            if let Some(b) = it.next() {
                next.push(combine(a, b));
            } else {
                // odd element: carry
                next.push(a);
            }
        }

        v = next;
    }

    v.pop()
}