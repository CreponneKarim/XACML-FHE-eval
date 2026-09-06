use std::{collections::HashMap, panic};

use tfhe::{CpuFheUint8Array, CpuFheUintArray, FheAsciiString, FheBool, FheInt64, FheInt128, FheUint32, PublicKey, ServerKey, prelude::{FheEncrypt, FheTryEncrypt}, safe_serialization::safe_serialize, strings::ciphertext::{FheAsciiChar, FheString}};
use crate::{ homomorphic::{policy::EncryptedTypes, types_impls::{HBoolean, HDnsName, HRfc822Name, HString, HX500Name}}, intermediate::{IntermediateAttributeValueType, IntermediateTypes}, xacml::{constants::{LongInteger, NormalInteger, XacmlDataTypesStrings}, policy::Loadout, types::{AttributeValue, EffectType, Policy, PolicyContent41Type, PolicySet, PolicySetContent31Type, Rule, RuleType}}};
use uuid::{self, Uuid};
use xsd_parser::xml::Text;

/// encrypt the text before only


pub trait EncryptType<T> {
    fn encrypt(t:& T, pk: &PublicKey) -> EncryptedTypes;
}

impl EncryptType<IntermediateTypes> for EncryptedTypes {
    fn encrypt(t:&IntermediateTypes, pk: &PublicKey) -> EncryptedTypes {
        match t {
            IntermediateTypes::String(s) => {
                println!("|{}|the len is |{}|",s.trim(),s.trim().len());
                EncryptedTypes::String(
                    FheAsciiString::try_encrypt(s.trim(), pk).unwrap()
                )
            },
            IntermediateTypes::Integer(i) => {
                EncryptedTypes::Integer(
                    FheInt64::try_encrypt(i.clone(), pk).unwrap()
                )
            },
            IntermediateTypes::Double(i) => {
                EncryptedTypes::Double(
                    FheInt128::try_encrypt(i.clone(), pk).unwrap()
                )
            },
            IntermediateTypes::Boolean(i) => {
                EncryptedTypes::Boolean(
                    FheBool::try_encrypt(i.clone(), pk).unwrap()
                )
            },
            IntermediateTypes::Time(i) => {
                EncryptedTypes::Time(
                    FheUint32::try_encrypt(i.clone(), pk).unwrap()
                )
            },
            IntermediateTypes::Date(i) => {
                EncryptedTypes::Date(
                    FheInt64::try_encrypt(i.clone(), pk).unwrap()
                )
            },
            IntermediateTypes::DateTime(i) => {
                EncryptedTypes::DateTime(
                    FheInt64::try_encrypt(i.clone(), pk).unwrap()
                )
            },
            IntermediateTypes::DayTimeDuration(i) => {
                EncryptedTypes::DayTimeDuration(
                    FheInt64::try_encrypt(i.clone(), pk).unwrap()
                )
            },
            IntermediateTypes::YearMonthDuration(i) => {
                EncryptedTypes::YearMonthDuration(
                    FheInt64::try_encrypt(i.clone(), pk).unwrap()
                )
            },
            IntermediateTypes::AnyUri(s) => {
                EncryptedTypes::AnyUri(
                    FheAsciiString::try_encrypt(s.trim(), pk).unwrap()
                )
            },
            IntermediateTypes::HexBinary(a) => {
                EncryptedTypes::HexBinary(
                    CpuFheUint8Array::try_encrypt(a.as_slice(), pk).unwrap()
                )
            },
            IntermediateTypes::Base64Binary(a) => {
                EncryptedTypes::Base64Binary(
                    CpuFheUint8Array::try_encrypt(a.as_slice(), pk).unwrap()
                )
            },
            IntermediateTypes::Rfc822Name(local_part, domain_part) => {
                let rfc822name_encrypted = HRfc822Name::new(
                    FheAsciiString::try_encrypt(local_part.trim(), pk).unwrap(),
                    FheAsciiString::try_encrypt(domain_part.trim(), pk).unwrap()
                );
                EncryptedTypes::Rfc822Name(
                    rfc822name_encrypted
                )
            },
            IntermediateTypes::X500Name(map) => {
                let mut encrypted_hashmap = HashMap::new();
                for (key,value) in map.into_iter() {
                    
                    encrypted_hashmap.insert(
                        key.clone(),
                        FheAsciiString::try_encrypt(value.trim(), pk).unwrap()
                    );
                }

                EncryptedTypes::X500Name(
                    HX500Name::new(encrypted_hashmap)
                )
            },
            IntermediateTypes::IpAddress(ip) => {
                EncryptedTypes::IpAddress(
                    FheAsciiString::try_encrypt(ip.trim(), pk).unwrap()
                )
            },
            IntermediateTypes::DnsName(addr, port_start, port_end) => {
                EncryptedTypes::DnsName(
                    HDnsName::new(
                        FheAsciiString::try_encrypt(addr.trim(), pk).unwrap(),
                        FheUint32::try_encrypt(port_start.clone(), pk).unwrap(), 
                        FheUint32::try_encrypt(port_end.clone(), pk).unwrap(), 
                    )
                )
            },
            IntermediateTypes::Bag(inner) => {
	            EncryptedTypes::Bag(
	                inner.iter().map(|v|EncryptedTypes::encrypt(v, pk)).collect()
	            )
            }
            
            IntermediateTypes::Nothing => EncryptedTypes::Nothing
        }
    }
}

pub trait EncryptEffect {
    fn encrypt_effect(&mut self,pk: &PublicKey) -> HashMap<Uuid, HBoolean>;
}

impl EncryptEffect for Loadout {
    fn encrypt_effect(&mut self,pk: &PublicKey) -> HashMap<Uuid, HBoolean> {
        match self {
            Loadout::PolicySet(p)   => p.encrypt_effect(pk),
            Loadout::Policy(p)         => p.encrypt_effect(pk)
        }
    }
}
impl EncryptEffect for Policy {
    fn encrypt_effect(&mut self,pk: &PublicKey) -> HashMap<Uuid, HBoolean> {
        let mut final_map:HashMap<Uuid, HBoolean> = HashMap::new();
        for c in &mut self.content_41 {
            match c {
                PolicyContent41Type::Rule(r) => {
                    final_map.extend(r.encrypt_effect(pk));
                }
                _=>{}
            };
        }
        final_map
    }
}
impl EncryptEffect for PolicySet {
    fn encrypt_effect(&mut self,pk: &PublicKey) -> HashMap<Uuid, HBoolean> {
        let mut final_map:HashMap<Uuid, HBoolean> = HashMap::new();
        for c in &mut self.content_31 {
            match c {
                PolicySetContent31Type::Policy(p) => {
                    final_map.extend(
                        p.encrypt_effect(pk)
                    );
                }
                PolicySetContent31Type::PolicySet(p) => {
                    final_map.extend(
                        p.encrypt_effect(pk)
                    );
                }
                _ =>{}
            }
        }
        
        final_map
    }
}
impl EncryptEffect for RuleType {
    fn encrypt_effect(&mut self,pk: &PublicKey) -> HashMap<Uuid, HBoolean> {
        let encrypted_effect = match self.effect {
            EffectType::Permit => {
                FheBool::encrypt(true, pk)
            }
            EffectType::Deny => {
                FheBool::encrypt(false, pk)
            }
            _=>{panic!("trying to encrypt an already encrypted effect")}
        };
        let id = Uuid::new_v4(); 
        self.effect= EffectType::EncryptedDecision(id.clone());

        let mut result = HashMap::new();
        result.insert(id, encrypted_effect);
        result
    }
}