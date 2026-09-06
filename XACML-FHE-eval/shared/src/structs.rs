use anyhow::anyhow;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use rkyv::collections::swiss_table::ArchivedHashMap;
use serde::{Deserialize,Serialize};
use types::{
    homomorphic::policy::{EncryptedAttributeSerStruct, EncryptedTypes},
    xacml::types::{AttributeDesignatorType, CategorizedAttributeDesignators},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct EncryptorResponse {
    pub names: Vec<String>,
    pub types: Vec<String>,
    pub values: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StoreResponse {
    pub key: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GlobalConfig {
    pub encryptorurl: String,
    pub decryptorurl: String,
    pub userurl: String,
    pub pdpurl: String,
    pub pdpdburl: String,
    pub papurl: String,
    pub pepurl: String,
    pub keycloakurl: String,
    pub redisaastoreurl: String,
    pub keystore_url: String,
}

// #[derive(Deserialize, Serialize, rkyv::Archive, PartialEq, rkyv::Deserialize, rkyv::Serialize)]
// pub struct PDPToPEPResponse {
//     pub id_do: String,
//     pub applicability: Vec<u8>,
//     pub effect: Vec<u8>,
// }

// #[derive(rkyv::Archive, PartialEq, rkyv::Deserialize, rkyv::Serialize)]
// pub struct PEPToPDPEvaluationRequest {
//     pub request: Vec<u8>,
//     pub attribute_list: HashMap<u128, MixedAttributeSerStruct>,
//     pub id_do: String,
// }

// #[derive(rkyv::Archive, PartialEq, rkyv::Deserialize, rkyv::Serialize)]
// pub struct PDPToPEPRequest {
//     pub attribute_designators: CategorizedAttributeDesignators,
//     pub public_key_digest: Vec<u8>,
// }

// #[derive(Deserialize, Serialize, rkyv::Archive, PartialEq, rkyv::Deserialize, rkyv::Serialize)]
// pub struct PEPToPDPRequest {
//     pub public_key_digest: Vec<u8>,
// }

// #[derive(Deserialize, Serialize, rkyv::Archive, PartialEq, rkyv::Deserialize, rkyv::Serialize)]
// pub struct DUToPEPEvaluationRequest {
//     pub token: TokenResponse,
//     pub id_do: String,
//     pub request: Vec<u8>
//     // pub public_key_digest_alg: DigestAlg,
// }

// #[derive(Clone, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
// pub struct PAPToPDPRequest {
//     pub do_to_pap_request: DOToPAPRequest,
//     pub attribute_designators: CategorizedAttributeDesignators,
// }

// #[derive(Deserialize, Serialize, rkyv::Archive, PartialEq, rkyv::Deserialize, rkyv::Serialize)]
// pub struct AttributeList {
//     pub encrypted_attributes: EncryptedAttributeList,
//     pub plain_attributes: HashMap<u128, IntermediateTypes>,
// }

// /// uuids can be use to recover the type of the attribute from
// /// the request that has the uuids in the designators
// #[derive(Deserialize, Serialize, rkyv::Archive, PartialEq, rkyv::Deserialize, rkyv::Serialize)]
// pub struct EncryptedAttributeList {
//     pub uuid: Vec<u128>,
//     pub types: Vec<String>,
//     pub ciphers: Vec<u8>, //	deser using the safe deserialize
// }

// // /// having the request
// // #[derive(Debug, Deserialize, Serialize, rkyv::Archive,PartialEq,rkyv::Deserialize, rkyv::Serialize)]
// // pub struct PlainAttributeList {
// // 	// pub uuid: Vec<>,
// // 	pub values: HashMap<u128,IntermediateTypes>
// // }

// #[derive(
//     Debug, Deserialize, Serialize, rkyv::Archive, PartialEq, rkyv::Deserialize, rkyv::Serialize,
// )]
// pub struct TokenResponse {
//     pub access_token: Option<String>,
//     pub expires_in: Option<u64>,
//     pub refresh_expires_in: Option<u64>,
//     pub refresh_token: Option<String>,
//     pub token_type: Option<String>,
//     pub id_token: Option<String>,
//     pub not_before_policy: Option<u64>,
//     pub session_state: Option<String>,
//     pub scope: Option<String>,

//     // useful if Keycloak returns an error
//     pub error: Option<String>,
//     pub error_description: Option<String>,
// }

// fn deserialize_base64_opt<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
// where
//     D: Deserializer<'de>,
// {
//     let opt = Option::<String>::deserialize(deserializer)?;

//     opt.map(|s| URL_SAFE_NO_PAD.decode(s).map_err(serde::de::Error::custom))
//         .transpose()
// }
// #[derive(Debug, Deserialize)]
// struct OidcDiscoveryDocument {
//     issuer: String,
//     jwks_uri: String,
// }

// pub async fn verify_keycloak_access_token(
//     access_token: &str,
//     discovery_url: &str,
//     expected_issuer: &str,
//     expected_audience_or_client_id: &str,
// ) -> Result<AccessTokenClaims, Box<dyn std::error::Error + Send + Sync>> {
//     // 1) Parse JOSE header
//     let header = decode_header(access_token)?;
//     let kid = header.kid.ok_or("missing kid in JWT header")?;
//     let alg = header.alg;

//     // Refuse "none"
//     if alg == Algorithm::HS256 || alg == Algorithm::HS384 || alg == Algorithm::HS512 {
//         return Err("unexpected HMAC algorithm for Keycloak public-key verification".into());
//     }

//     // 2) Fetch OIDC discovery
//     let client = reqwest::Client::builder()
//         .timeout(Duration::from_secs(12000))
//         .connect_timeout(Duration::from_secs(10))
//         .pool_idle_timeout(Duration::from_secs(25))
//         .connection_verbose(true) // helps debug
//         .tcp_keepalive(Duration::from_secs(15000))
//         .build()
//         .unwrap();
//     let discovery: OidcDiscoveryDocument = client
//         .get(discovery_url)
//         .send()
//         .await?
//         .error_for_status()?
//         .json()
//         .await?;

//     if discovery.issuer != expected_issuer {
//         return Err(format!(
//             "issuer mismatch: discovery says {}, expected {}",
//             discovery.issuer, expected_issuer
//         )
//         .into());
//     }

//     // 3) Fetch JWKS
//     let jwks: JwkSet = client
//         .get(&discovery.jwks_uri)
//         .send()
//         .await?
//         .error_for_status()?
//         .json()
//         .await?;

//     // 4) Find matching key by kid
//     let jwk = jwks
//         .keys
//         .iter()
//         .find(|k| k.common.key_id.as_deref() == Some(&kid))
//         .ok_or("no matching JWK found for kid")?;

//     // 5) Build decoding key from JWK
//     let decoding_key = DecodingKey::from_jwk(jwk)?;

//     // 6) Validate signature + standard claims
//     let mut validation = Validation::new(alg);
//     validation.set_issuer(&[expected_issuer]);

//     // For access tokens, audience handling varies by setup.
//     // If your access token uses aud, validate it here:
//     validation.set_audience(&[expected_audience_or_client_id]);

//     let token_data = decode::<AccessTokenClaims>(access_token, &decoding_key, &validation)?;

//     // 7) Optional extra check for azp
//     if let Some(azp) = &token_data.claims.azp {
//         if azp != expected_audience_or_client_id {
//             return Err(format!(
//                 "azp mismatch: got {}, expected {}",
//                 azp, expected_audience_or_client_id
//             )
//             .into());
//         }
//     }

//     Ok(token_data.claims)
// }

// pub struct BundleAccessToken {
//     pub claims: AccessTokenClaims,
//     pub signature: String,
//     pub header: String,
// }

// pub fn decode_access_token(
//     access_token: &str,
// ) -> Result<BundleAccessToken, Box<dyn std::error::Error + Send + Sync>> {
//     let mut parts = access_token.split('.');

//     let header = parts.next().ok_or("missing jwt header")?;
//     let payload = parts.next().ok_or("missing jwt payload")?;
//     let signature = parts.next().ok_or("missing jwt signature")?;

//     let decoded_payload = URL_SAFE_NO_PAD
//         .decode(payload)
//         .map_err(|e| format!("error is -> {}", e))?;
//     let claims: AccessTokenClaims = serde_json::from_slice(&decoded_payload)
//         .map_err(|e| format!("claim access error -> {}", e))?;

//     Ok(BundleAccessToken {
//         claims: claims,
//         signature: signature.to_string(),
//         header: header.to_string(),
//     })
// }

// #[derive(Debug, Deserialize)]
// pub struct AccessTokenClaims {
//     pub exp: Option<u64>,
//     pub iat: Option<u64>,
//     pub iss: Option<String>,
//     pub aud: Option<serde_json::Value>,
//     pub sub: Option<String>,
//     pub azp: Option<String>,
//     pub scope: Option<String>,

//     // your custom mapper claim
//     #[serde(default, deserialize_with = "deserialize_base64_opt")]
//     pub encrypted_blob: Option<Vec<u8>>,

//     // optional extra fields you may have added
//     #[serde(flatten)]
//     pub extra: std::collections::HashMap<String, serde_json::Value>,
// }

// #[derive(Deserialize, Serialize, rkyv::Archive, PartialEq, rkyv::Deserialize, rkyv::Serialize)]
// pub struct PublicKeyDigestResponse {
//     pub bytes: Vec<u8>,
//     pub digest: Vec<u8>,
//     pub alg: DigestAlg,
// }

// #[derive(Deserialize, Serialize, rkyv::Archive, PartialEq, rkyv::Deserialize, rkyv::Serialize)]
// pub struct ServerKeyDigestResponse {
//     pub bytes: Vec<u8>,
//     pub digest: Vec<u8>,
//     pub alg: DigestAlg,
// }

// #[derive(Deserialize, Serialize, rkyv::Archive, PartialEq, rkyv::Deserialize, rkyv::Serialize)]
// pub struct BothKeysResponse {
//     pub pk_comp: Vec<u8>,
//     pub sks_comp: Vec<u8>,
// }

// #[derive(
//     Debug,
//     Deserialize,
//     Serialize,
//     rkyv::Archive,
//     PartialEq,
//     rkyv::Deserialize,
//     rkyv::Serialize,
//     Clone,
// )]
// pub enum DigestAlg {
//     Sha256,
// }
// #[derive(Debug, Serialize, rkyv::Archive, PartialEq, rkyv::Deserialize, rkyv::Serialize, Clone)]
// pub struct StoreKeysRequest {
//     pub user_id: String,
//     pub compressed_pk: Vec<u8>,
//     pub compressed_sks: Vec<u8>,
//     pub compressed_pk_digest: String,
//     pub compressed_sks_digest: String,
//     pub alg: DigestAlg,
// }
// impl StoreKeysRequest {
//     pub fn new(user_id: String, compressed_pk: Vec<u8>, compressed_sks: Vec<u8>) -> Self {
//         assert!(!user_id.is_empty());
//         assert!(!compressed_pk.is_empty());
//         assert!(!compressed_sks.is_empty());

//         let compressed_pk_digest = sha256::digest(&compressed_pk);
//         let compressed_sks_digest = sha256::digest(&compressed_sks);

//         Self {
//             user_id: user_id,
//             compressed_pk: compressed_pk,
//             compressed_sks: compressed_sks,
//             compressed_pk_digest: compressed_pk_digest,
//             compressed_sks_digest: compressed_sks_digest,
//             alg: DigestAlg::Sha256,
//         }
//     }
// }
// #[derive(Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
// pub struct DecryptApplicabilityEffectRequest {
//     pub user_id: String,
//     pub ciphertext_list: Vec<u8>,
// }

// #[derive(Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
// pub struct PostKskRequest {
//     pub user_id: String,
//     pub ksk_sered: Vec<u8>,
// }

// #[derive(Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
// pub struct PartialDecryptRequest {
//     // pub session_id: String,
//     pub user_id: String,
//     pub session_id: String,
//     /// two booleans that are encrypted
//     pub ciphertext_list: Vec<u8>,
// }

// #[derive(Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
// pub struct PartialDecryptRequestUncompressed {
//     // pub session_id: String,
//     pub user_id: String,
//     pub session_id: String,
//     /// two booleans that are encrypted
//     pub a: Vec<u8>,
//     pub e: Vec<u8>,
// }

// #[derive(Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
// pub struct PartialDecryptResponse<PartiallyDecryptedTypes> {
//     pub user_id: String,
//     pub session_id: String,
//     /// safe_serialize(CompressedServerKey) — the party's shard of the server key
//     pub a_part: PartiallyDecryptedTypes,
//     pub e_part: PartiallyDecryptedTypes,
// }

// // #[derive(Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
// // pub struct PartialDecryptShare {
// //     pub session_id: String,
// //     pub party_id: usize,
// //     /// The masked partial decryption value (plaintext u64, mod q)
// //     /// In the real MPC lib this would be a LWE sample or polynomial share.
// //     /// Here it is the noise-flooded inner product of ct body · sk_share.
// //     pub share_value: Vec<u8>,  // serialized share
// // }

// #[derive(Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
// pub struct RegisterPartyRequest<KeyShareType> {
//     pub user_id: String,
//     /// safe_serialize(CompressedServerKey) — the party's shard of the server key
//     pub key_share_bytes: KeyShareType,
// }

// #[derive(Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
// pub struct DUToPEPRequest {
//     pub resource_id: String,
//     // pub request: Vec<u8>,
// }
// #[derive(Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
// pub struct PEPToDUResponse {
// 	pub attributes_to_ask_for:Vec<String>,
// 	pub id_do: String
// }

// #[derive(Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
// pub struct PEPToDUEvaluationResponse {
// 	pub a:bool,
// 	pub e:bool,
// 	pub id_do: String
// }
