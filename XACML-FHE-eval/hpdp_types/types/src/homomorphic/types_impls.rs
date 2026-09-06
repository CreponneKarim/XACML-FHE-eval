use std::{collections::HashMap, fmt::Debug};

use rkyv::{Archived, Place, Resolver, rancor::Fallible, with::{ArchiveWith, DeserializeWith}};
use tfhe::{CpuFheUint8Array, FheAsciiString, FheBool, FheInt32, FheInt64, FheInt128, FheIntId, FheUint8, FheUint8Array, FheUint32, FheUint64, FheUint128, FheUintId, core_crypto::prelude::UnsignedInteger, strings::ciphertext::FheString};

use xsd_parser::{ xml::{AnyAttributes, AnyElement, Mixed}};

use crate::{homomorphic::policy::EncryptedTypes, intermediate::IntermediateTypes, xacml::{constants::{LongInteger, NormalInteger, XacmlDataTypesStrings}, types::AttributeValueType}};

///	offer an implementation for the trait types through structs and trait implementations
///	TODO Actually add some generecity and trait manipulation, but itt will do for now

use crate::homomorphic::types_traits::{HFundamentalTypes::{self, HIntegerType}, HXacmlSpecializedTypes::{self, HDateTimeType, HDayTimeDurationType, HTimeType, HYearMonthDurationType}};

// pub type HShortInt	= FheInt32;
// pub type HInt		= FheInt64;
// pub type HLongInt	= FheInt128;
// pub type HUInt = FheUint64;

pub type HChar					= FheUint8;
pub type HSmallInteger 			= FheInt32;
pub type HNormalInteger 		= FheInt64;
pub type HLongInteger 			= FheInt128;

pub type HUnsignedSmallInteger 	= FheUint32;
pub type HUnsignedNormalInteger = FheUint64;
pub type HUnsignedLongInteger 	= FheUint128;

pub type HDouble 				= FheInt128;
pub type HString				= FheAsciiString;
pub type HBoolean				= FheBool;

pub type HTime					= HUnsignedSmallInteger;
pub type HDateTime				= HNormalInteger;
pub type HDate				= HNormalInteger;
pub type HDayTimeDuration		= HNormalInteger;
pub type HYearMonthDuration		= HNormalInteger;
pub type HAnyUri				= HString;
pub type HHexBinary				= CpuFheUint8Array;
pub type HBase64Binary			= CpuFheUint8Array;
pub type HIpAddress				= HString;


// pub struct EncryptedAttributeValueType {
//     pub data_type: ::std::string::String,
//     pub text: EncryptedTypes,
// }

// impl EncryptedAttributeValueType {
//     fn from(encrypted_data: EncryptedTypes, value: &AttributeValueType) -> Self {
//         Self {
//             data_type:value.data_type.clone(),
//             text: encrypted_data,
//         }
//     }
// }

//	fundamental types
// pub struct HInteger<T:FheIntId>
// {
// 	val: T
// }

// pub struct HUnsignedInteger<T: FheUintId>
// {
// 	val: T
// }
// pub struct HBoolean {
// 	val: FheBool
// }
// pub struct HString {
// 	val: FheString
// }

// impl <T:FheIntId> HFundamentalTypes::HFundamentalType<T>	for HInteger<T> {
// 	fn inner(&self) -> &T {
// 		&self.val
// 	}
// }

// impl <T:FheUintId> HFundamentalTypes::HFundamentalType<T>	for HUnsignedInteger<T> {
// 	fn inner(&self) -> &T {
// 		&self.val
// 	}
// }


// impl HFundamentalTypes::HFundamentalType<FheBool>	for HBoolean {
// 	fn inner(&self) -> &FheBool {
// 		&self.val
// 	}
// }
// impl HFundamentalTypes::HFundamentalType<FheString> for HString {
// 	fn inner(&self) -> &FheString {
// 		&self.val
// 	}
// }

// impl <T:FheIntId>HFundamentalTypes::HIntegerType<T> 			for HInteger<T>				{}

// impl <T:FheUintId>HFundamentalTypes::HUnsignedIntegerType<T>	for HUnsignedInteger<T>		{}

// impl HFundamentalTypes::HBooleanType<FheBool>					for HBoolean				{}
// impl HFundamentalTypes::HStringType<FheString>					for HString					{}

//	specialized types
// #[derive(serde::Serialize, serde::Deserialize)]
// pub struct HTime {
// 	val:HUnsignedSmallInteger
// }
// impl HTime {
// 	pub fn new(val:HUnsignedSmallInteger) -> Self {
// 		Self {
// 			val:val
// 		}
// 	}
// }
// #[derive(serde::Serialize, serde::Deserialize)]
// pub struct HDateTime {
// 	val:HNormalInteger
// }
// impl HDateTime {
// 	pub fn new(val:HNormalInteger) -> Self {
// 		Self {
// 			val:val
// 		}
// 	}
// }
// #[derive(serde::Serialize, serde::Deserialize)]
// pub struct HDayTimeDuration {
// 	val:HNormalInteger
// }
// impl HDayTimeDuration {
// 	pub fn new(val:HNormalInteger) -> Self {
// 		Self {
// 			val:val
// 		}
// 	}
// }
// #[derive(serde::Serialize, serde::Deserialize)]
// pub struct HYearMonthDuration {
// 	val:HNormalInteger
// }
// impl HYearMonthDuration {
// 	pub fn new(val:HNormalInteger) -> Self {
// 		Self {
// 			val:val
// 		}
// 	}
// }

//	implementing the respective traits for each type to force more constraints
//	and define functionality through the traits and not the types
// impl HTimeType<FheUint32,HUnsignedSmallInteger> for HTime				{}
// impl HDateTimeType<FheInt128,HLongInteger> 		for HDateTime			{}
// impl HDayTimeDurationType<FheInt64,HNormalInteger> 	for HDayTimeDuration	{}
// impl HYearMonthDurationType<FheInt64,HNormalInteger> 	for HYearMonthDuration	{}

//	TODO:	I stopped implementing traits for the other types, fearing performances repercussions
//			in the end try to make it more abstract by using te traits defined in the type_traits 
//			file

// #[derive(serde::Serialize, serde::Deserialize)]
// pub struct HAnyUri {
// 	val:HString	
// }
// impl HAnyUri {
// 	pub fn new(val:HString) -> Self {
// 		Self {
// 			val:val
// 		}
// 	}
// }
// #[derive(serde::Serialize, serde::Deserialize)]
// pub struct HHexBinary {
// 	val:Vec<HChar>
// }
// impl HHexBinary {
// 	pub fn new(val: Vec<HChar>) -> Self {
// 		Self {
// 			val:val
// 		}
// 	}
// }
// #[derive(serde::Serialize, serde::Deserialize)]
// pub struct HBase64Binary {
// 	val:Vec<HChar>	
// }
// impl HBase64Binary {
// 	pub fn new(val: Vec<HChar>) -> Self {
// 		Self {
// 			val:val
// 		}
// 	}
// }

// struct HStringAdapter;
// impl ArchiveWith<Vec<u8>> for HStringAdapter {
//     type Archived = Archived<Vec<u8>>;
//     type Resolver = Resolver<Vec<u8>>;

//     fn resolve_with(
//         field: &Vec<u8>,
//         resolver: Self::Resolver,
//         out: Place<Self::Archived>,
//     ) {
//         // let converted = serialize(field);
//         field.resolve(resolver, out);
//     }
// }

// impl<S> SerializeWith<i32, S> for AgeAdapter
// where
//     S: Fallible + ?Sized,
//     i32: Serialize<S>,
// {
//     fn serialize_with(
//         field: &i32,
//         serializer: &mut S,
//     ) -> Result<Self::Resolver, S::Error> {
//         let converted = serialize_age(field);
//         converted.serialize(serializer)
//     }
// }

// impl<D> DeserializeWith<Archived<HString>, Vec<u8>, D> for HStringAdapter
// where
//     D: Fallible + ?Sized,
//     Archived<i32>: rkyv::Deserialize<HString, D>,
// {
//     fn deserialize_with(
//         field: &Archived<HString>,
//         deserializer: &mut D,
//     ) -> Result<Vec<u8>, D::Error> {
//         let archived_value = field.deserialize(deserializer)?;
//         Ok(deserialize_age(archived_value))
//     }
// }
#[derive(serde::Serialize, serde::Deserialize,Clone)]
pub struct HRfc822Name {
	pub localpart:HString,
	pub domainpart:HString
}

#[derive(serde::Serialize, serde::Deserialize,Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
// #[rkyv(
// 	serialize_bounds(
// 	    __S: rkyv::ser::Writer + rkyv::ser::Allocator,
// 	    <__S as rkyv::rancor::Fallible>::Error: rkyv::rancor::Source
// 	),
// 	deserialize_bounds(
// 	    <__D as rkyv::rancor::Fallible>::Error: rkyv::rancor::Source
// 	),
// 	bytecheck(bounds(
//         __C: rkyv::validation::ArchiveContext,
//         <__C as rkyv::rancor::Fallible>::Error: rkyv::rancor::Source
//     ))
// )]
pub struct HRfc822NameSerDeser {
	pub localpart:Vec<u8>,
	pub domainpart:Vec<u8>
}
impl HRfc822Name {
	pub fn new(
		localpart:HString,
		domainpart:HString
	) -> Self {
		Self {
			localpart: localpart,
			domainpart: domainpart,
		}
	}
}
#[derive(serde::Serialize, serde::Deserialize,Clone)]
pub struct HX500Name {
	pub val:HashMap<String,HString>
}

#[derive(serde::Serialize, serde::Deserialize,Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
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
pub struct HX500NameSerDeser {
	#[rkyv(
		    omit_bounds,
	)]
	pub val:HashMap<String,Vec<u8>>
}
impl HX500Name {
	pub fn new(val: HashMap<String,HString>) -> Self {
		Self {
			val: val
		}
	}
}
// #[derive(serde::Serialize, serde::Deserialize)]
// pub struct HIpAddress {
// 	val:HString
// }
// impl HIpAddress {
// 	pub fn new(val: HString) -> Self {
// 		Self {
// 			val: val
// 		}
// 	}
// }
//	TODO :	optimization possible here, for the starting and ending port an unsigned int 16 can be used no need for 32
//			maybe add a trait for that and go and reimplement it
#[derive(serde::Serialize, serde::Deserialize,Clone)]
pub struct HDnsName {
	pub adr:		HString,
	pub start_port:	HUnsignedSmallInteger,
	pub end_port:	HUnsignedSmallInteger
}

#[derive(serde::Serialize, serde::Deserialize,Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
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
pub struct HDnsNameSerDeser {
	pub adr:		Vec<u8>,
	pub start_port:	Vec<u8>,
	pub end_port:	Vec<u8>
}
impl HDnsName {
	pub fn new(
		adr:		HString,
		start_port:	HUnsignedSmallInteger,
		end_port:	HUnsignedSmallInteger
	) -> Self {
		Self {
			adr: adr,
			start_port: start_port,
			end_port: end_port,
		}
	}
}