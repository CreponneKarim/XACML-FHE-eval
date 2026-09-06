use serde::{Deserialize, Serialize};

/// defining mixed type here, where the type in question could be encrypted or plain

use crate::{homomorphic::policy::{EncryptedTypes, MixedAttributeSerStruct}, intermediate::IntermediateTypes};

#[derive(Clone,Serialize,Deserialize)]
pub enum MixedTypes {
    EncryptedType(EncryptedTypes),
    IntermediateType(IntermediateTypes)
}

pub mod funcs;