use std::collections::HashMap;
use tfhe::FheBool;
use uuid::Uuid;
use crate::{mixed::MixedTypes, xacml::policy::Loadout};

pub struct EncryptedPolicyWrapper {
	pub loadout:Loadout,
	pub encrypted_atts:HashMap<Uuid,MixedTypes>,
	pub encrypted_effects:HashMap<Uuid, FheBool>,
	pub all_atts_encrypted_times_policy:HashMap<Uuid,u128>,
	pub nb_enc_atts: usize,
	pub nb_total_atts:usize,
	pub att_uuid_to_rule_id: HashMap<Uuid, String>
}