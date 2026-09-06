use std::collections::HashMap;

use rkyv::{ArchiveUnsized, ArchivedMetadata, ptr_meta::Pointee, traits::ArchivePointee};
use serde::{Deserialize, Serialize};
use tfhe::{CpuFheUint8Array, FheAsciiString, FheBool, FheInt64, FheInt128, FheUint32, prelude::{FheTryEncrypt, FheTryTrivialEncrypt}};
use xsd_parser::xml::{AnyAttributes, AnyElement, Mixed};

use crate::{homomorphic::{policy::EncryptedTypes, types_impls::{HDnsName, HHexBinary, HRfc822Name, HX500Name}}, xacml::constants::{Boolean, Double, NormalInteger, SmallInteger, USmallInteger}};

#[derive(Debug,Serialize,Deserialize,rkyv::Archive, PartialEq,rkyv::Deserialize, rkyv::Serialize)]
pub struct IntermediateAttributeValueType {
    pub data_type: ::std::string::String,
    pub att_id: String,
    pub text: IntermediateTypes,
}


// impl ArchivePointee for IntermediateTypes {
// 	type ArchivedMetadata = <Vec<IntermediateTypes> as ArchivePointee>::ArchivedMetadata;
	
// 	fn pointer_metadata(
//         archived: &Self::ArchivedMetadata,
//     ) -> <Self as Pointee>::Metadata {
//    		()
//     }
// }

// impl ArchiveUnsized for IntermediateTypes {
// 	// We'll reuse our block type as our archived type.
//     type Archived = IntermediateTypes;

//     // Here's where we make the metadata for our archived type.
//     fn archived_metadata(&self) -> ArchivedMetadata<Self> {
//         // Because the metadata for our `ArchivedBlock` is the metadata of
//         // the trailing slice, we just need to return that archived
//         // metadata.
//         self.tail.archived_metadata()
//     }
// }

#[derive(Debug,Clone,Serialize,Deserialize, PartialEq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(
	serialize_bounds(
	    __S: rkyv::ser::Writer + rkyv::ser::Allocator,
	    <__S as rkyv::rancor::Fallible>::Error: rkyv::rancor::Source
	),
	deserialize_bounds(
	    <__D as rkyv::rancor::Fallible>::Error: rkyv::rancor::Source
	),
	bytecheck(bounds(
        __C: rkyv::validation::ArchiveContext,
        <__C as rkyv::rancor::Fallible>::Error: rkyv::rancor::Source
    ))
)]
pub enum IntermediateTypes {
    String              (String),
    Integer             (NormalInteger),
    Double              (Double),
    Boolean             (Boolean),
    Time                (USmallInteger),
    Date            (NormalInteger),
    DateTime            (NormalInteger),
    DayTimeDuration     (NormalInteger),
    YearMonthDuration   (NormalInteger),
    AnyUri              (String),
    HexBinary           (Vec<u8>),
    Base64Binary        (Vec<u8>),
    Rfc822Name          (String,String),
    X500Name            (HashMap<String,String>),
    DnsName             (String,USmallInteger,USmallInteger),
    IpAddress           (String),
    Bag					(
	#[rkyv(
	    omit_bounds,
	)]
	Vec<IntermediateTypes>
    ),
    Nothing
}

impl IntermediateTypes {
    pub fn trivial_enc(&self) -> EncryptedTypes{
        match self {
            IntermediateTypes::String (v) => {
                EncryptedTypes::String(
                    FheAsciiString::try_encrypt_trivial_with_fixed_sized(v.as_str(), v.len()).unwrap()
                )
            }
            IntermediateTypes::Integer (v) => {
                EncryptedTypes::Integer(
                    FheInt64::try_encrypt_trivial(v.clone()).unwrap()
                )
            }
            IntermediateTypes::Double (v) => {
                EncryptedTypes::Double(
                    FheInt128::try_encrypt_trivial(v.clone()).unwrap()
                )
            }
            IntermediateTypes::Boolean (v) => {
                EncryptedTypes::Boolean(
                    FheBool::try_encrypt_trivial(v.clone()).unwrap()
                )
            }
            IntermediateTypes::Time (v) => {
                EncryptedTypes::Time(
                    FheUint32::try_encrypt_trivial(v.clone()).unwrap()
                )
            }
            IntermediateTypes::DateTime (v) => {
                EncryptedTypes::DateTime(
                    FheInt64::try_encrypt_trivial(v.clone()).unwrap()
                )
            }
            IntermediateTypes::Date (v) => {
                EncryptedTypes::Date(
                    FheInt64::try_encrypt_trivial(v.clone()).unwrap()
                )
            }
            IntermediateTypes::DayTimeDuration (v) => {
                EncryptedTypes::DayTimeDuration(
                    FheInt64::try_encrypt_trivial(v.clone()).unwrap()
                )
            }
            IntermediateTypes::YearMonthDuration (v) => {
                EncryptedTypes::YearMonthDuration(
                    FheInt64::try_encrypt_trivial(v.clone()).unwrap()
                )
            }
            IntermediateTypes::AnyUri (v) => {
                EncryptedTypes::AnyUri(
                    FheAsciiString::try_encrypt_trivial_with_fixed_sized(v,v.len()).unwrap()
                )
            }
            IntermediateTypes::HexBinary (v) => {
                EncryptedTypes::HexBinary(
                    CpuFheUint8Array::try_encrypt_trivial(v.as_slice()).unwrap()
                )
            }
            IntermediateTypes::Base64Binary (v) => {
                EncryptedTypes::Base64Binary(
                    CpuFheUint8Array::try_encrypt_trivial(v.as_slice()).unwrap()
                )
            }
            IntermediateTypes::Rfc822Name (v1,v2) => {
                EncryptedTypes::Rfc822Name(
                    HRfc822Name::new(
                        FheAsciiString::try_encrypt_trivial_with_fixed_sized(v1,v1.len()).unwrap(),
                        FheAsciiString::try_encrypt_trivial_with_fixed_sized(v2,v2.len()).unwrap()
                    )
                )
            }
            IntermediateTypes::X500Name (v) => {
                EncryptedTypes::X500Name(
                    HX500Name::new(
                        v.iter().map(
                            |(k,v2)| (k.clone(),FheAsciiString::try_encrypt_trivial_with_fixed_sized(v2, v2.len()).unwrap())
                        ).collect()
                    )
                )
            }
            IntermediateTypes::DnsName (v,startport, endport) => {
                EncryptedTypes::DnsName(
                    HDnsName::new(
                        FheAsciiString::try_encrypt_trivial_with_fixed_sized(v,v.len()).unwrap(), 
                        FheUint32::try_encrypt_trivial(startport.clone()).unwrap(),
                        FheUint32::try_encrypt_trivial(endport.clone()).unwrap()
                    )
                )
            }
            IntermediateTypes::IpAddress (v) => {
                EncryptedTypes::IpAddress(
                    FheAsciiString::try_encrypt_trivial_with_fixed_sized(v,v.len()).unwrap()
                )
            },
            
            IntermediateTypes::Bag (v) => {
                EncryptedTypes::Bag(
                    // FheAsciiString::try_encrypt_trivial_with_fixed_sized(v,v.len()).unwrap()
                    v.clone().into_iter().map(|elem| elem.trivial_enc()).collect()
                )
            },
            IntermediateTypes::Nothing => EncryptedTypes::Nothing
        }
    }
}

pub mod funcs;