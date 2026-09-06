///	The policy is explored searching for variables. For each found variable that exists inside of the policy
/// a place holder is 

use std::{collections::HashMap, fmt::Debug, panic};
use anyhow::{Error, anyhow};
use quick_xml::events::{BytesStart, Event, attributes::Attribute};
use rkyv::rancor;
use tfhe::{CompressedCiphertextList, CpuFheUint8Array, FheAsciiString, FheBool, FheInt64, FheInt128, FheUint, FheUint8Id, Versionize, array::{FheArrayBase, stride::DynDimensions}, boolean::prelude::CompressedCiphertext, integer::{RadixCiphertext, ciphertext::BaseRadixCiphertext}, prelude::CiphertextList, safe_serialization::{safe_deserialize, safe_serialize}};
use uuid::Uuid;
use xsd_parser::{models::schema::xs::quick_xml_deserialize, xml::{AttributeValue, Text}};
use crate::{homomorphic::types_impls::{HAnyUri, HBase64Binary, HBoolean, HChar, HDate, HDateTime, HDayTimeDuration, HDnsName, HDnsNameSerDeser, HDouble, HHexBinary, HIpAddress, HNormalInteger, HRfc822Name, HRfc822NameSerDeser, HString, HTime, HUnsignedSmallInteger, HX500Name, HX500NameSerDeser, HYearMonthDuration}, intermediate::IntermediateTypes, mixed::MixedTypes, xacml::constants::{STR_ANY_URI, STR_BOOLEAN, STR_DATE_TIME, STR_DAY_TIME_DURATION, STR_DNS_NAME, STR_DOUBLE, STR_INTEGER, STR_IP_ADDRESS, STR_RFC_822_NAME, STR_STRING, STR_TIME, STR_X_500_NAME, STR_YEAR_MONTH_DURATION, XacmlDataTypesStrings}};



// #[derive(Clone, Debug, serde::Serialize, serde::Deserialize,rkyv::Archive,rkyv::Deserialize, rkyv::Serialize)]
// #[rkyv(remote = tfhe::array::stride::DynDimensions)]
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
// pub struct DynDimensionsInternal {
// 	#[rkyv(
// 	    omit_bounds,
// 	)]
// 	#[rkyv(getter = tfhe::array::stride::DynDimensions::shape)]
//     pub(crate) shape: Vec<usize>,
//     #[rkyv(
// 		    omit_bounds,
// 	)]
//     #[rkyv(getter = tfhe::array::stride::DynDimensions::strides)]
//     pub(crate) strides: Vec<usize>,
// }

// impl From<DynDimensionsInternal> for DynDimensions {
// 	fn from(value: DynDimensionsInternal) -> Self {
// 		Self{
// 			shape: value.shape,
// 			strides: value.strides
// 		}
// 	}
// }


// impl <C> From<FheArrayBaseInternal<C>> for  FheArrayBase<C, FheUint8Id> {
// 	fn from(value: FheArrayBaseInternal<C>) -> Self {
// 		FheArrayBase::new(value.elems, DynDimensions::from(value.dims))
// 	}
// }
// // impl From<FheArrayBaseInternal<C>> for  FheArrayBase<Vec<BaseRadixCiphertext<tfhe::shortint::Ciphertext>>, FheUint8Id> {
// // 	fn from(value: FheArrayBaseInternal<Vec<BaseRadixCiphertext<tfhe::shortint::Ciphertext>>>) -> Self {
// // 		FheArrayBase::new(value.elems, DynDimensions::from(value.dims))
// // 	}
// // }

// #[derive(Clone, serde::Serialize, serde::Deserialize, rkyv::Archive,rkyv::Deserialize, rkyv::Serialize)]
// #[rkyv(remote = FheArrayBase<C,FheUint8Id>)]
// pub struct FheArrayBaseInternal<C>
// {
// 	#[rkyv(getter = FheArrayBase::container)]
// 	elems: C,
// 	// #[rkyv(getter = FheArrayBase::sha)]
//     dims: DynDimensionsInternal,
// }


#[derive(serde::Serialize, serde::Deserialize,Clone
	// ,rkyv::Archive,rkyv::Deserialize, rkyv::Serialize
)]
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
pub enum EncryptedTypes {
	Integer				(HNormalInteger),
	Double				(HDouble),
	String				(HString),
	Boolean				(HBoolean),
	Time				(HTime),
	Date				(HDate),
	DateTime			(HDateTime),
	DayTimeDuration		(HDayTimeDuration),
	YearMonthDuration	(HYearMonthDuration),
	AnyUri				(HAnyUri),
	HexBinary			(HHexBinary),
	Base64Binary		(HBase64Binary),
	Rfc822Name			(HRfc822Name),
	X500Name			(HX500Name),
	DnsName				(HDnsName),
	IpAddress			(HIpAddress),
	Bag					(Vec<EncryptedTypes>),
	Nothing
}

impl EncryptedTypes {
	pub fn from_ciphertexts_list(
		c_list : &mut CompressedCiphertextList,
		index: usize,
		xacml_t : XacmlDataTypesStrings
	) -> Self{
		match xacml_t {
			XacmlDataTypesStrings::String=> {
				Self::String(c_list.get(index).unwrap().unwrap())
			}
			XacmlDataTypesStrings::Integer=> {
				Self::Integer(c_list.get(index).unwrap().unwrap())
			}
			XacmlDataTypesStrings::Double=> {
				Self::Double(c_list.get(index).unwrap().unwrap())
			}
			XacmlDataTypesStrings::Boolean=> {
				Self::Boolean(c_list.get(index).unwrap().unwrap())
			}
			XacmlDataTypesStrings::Time=> {
				Self::Time(c_list.get(index).unwrap().unwrap())
			}
			XacmlDataTypesStrings::DateTime=> {
				Self::DateTime(c_list.get(index).unwrap().unwrap())
			}
			XacmlDataTypesStrings::DayTimeDuration=> {
				Self::DayTimeDuration(c_list.get(index).unwrap().unwrap())
			}
			XacmlDataTypesStrings::YearMonthDuration=> {
				Self::YearMonthDuration(c_list.get(index).unwrap().unwrap())
			}
			XacmlDataTypesStrings::AnyUri=> {
				Self::AnyUri(c_list.get(index).unwrap().unwrap())
			}
			
			XacmlDataTypesStrings::IpAddress=> {
				Self::IpAddress(c_list.get(index).unwrap().unwrap())
			}
			_=>{
				panic!("type <{}> not yet supported",xacml_t.to_string())
			}
			// for now ignore
			// XacmlDataTypesStrings::HexBinary=> {
			// 	Self::HexBinary(c_list.get(index).unwrap().unwrap())
			// }
			// XacmlDataTypesStrings::Base64Binary=> {
			// 	Self::Base64Binary(c_list.get(index).unwrap().unwrap())
			// }
			// XacmlDataTypesStrings::Rfc822Name=> {
			// 	Self::Rfc822Name(c_list.get(index).unwrap().unwrap())
			// }
			// XacmlDataTypesStrings::X500Name=> {
			// 	Self::X500Name(c_list.get(index).unwrap().unwrap())
			// }
			// XacmlDataTypesStrings::DnsName=> {
			// 	Self::DnsName(c_list.get(index).unwrap().unwrap())
			// }
		}
	}
	
	pub fn safe_ser(&self) -> Vec<u8> {
		let mut buffer =vec![];
		let serialized_size_limit = 1<<33;
		let res =match self {
			Self::Integer(val) => {
				safe_serialize(val, &mut buffer, serialized_size_limit)
			}
			Self::Double(val) => {
				safe_serialize(val, &mut buffer, serialized_size_limit)
			}
			Self::String(val) => {
				safe_serialize(val, &mut buffer, serialized_size_limit)
			}
			Self::Boolean(val) => {
				safe_serialize(val, &mut buffer, serialized_size_limit)
			}
			Self::Time(val) => {
				safe_serialize(val, &mut buffer, serialized_size_limit)
			}
			Self::DateTime(val) => {
				safe_serialize(val, &mut buffer, serialized_size_limit)
			}
			Self::DayTimeDuration(val) => {
				safe_serialize(val, &mut buffer, serialized_size_limit)
			}
			Self::YearMonthDuration(val) => {
				safe_serialize(val, &mut buffer, serialized_size_limit)
			}
			Self::AnyUri(val) => {
				safe_serialize(val, &mut buffer, serialized_size_limit)
			}
			// Self::HexBinary(val) => {
				
			// 	safe_serialize(val, &mut buffer, serialized_size_limit)
			// }
			// Self::Base64Binary(val) => {
			// 	safe_serialize(val, &mut buffer, serialized_size_limit)
			// }
			Self::Rfc822Name(val) => {
				let mut inter_buffer1 = vec![];
				let mut inter_buffer2 = vec![];
				safe_serialize(&val.domainpart, &mut inter_buffer1, serialized_size_limit);
				safe_serialize(&val.localpart, &mut inter_buffer2, serialized_size_limit);
				let ser_deser = crate::homomorphic::types_impls::HRfc822NameSerDeser{ domainpart: inter_buffer1,localpart:inter_buffer2};
				buffer = rkyv::to_bytes::<rkyv::rancor::Error>(&ser_deser).unwrap().to_vec();
				Ok(())
			}
			Self::X500Name(val) => {
				let mut map = HashMap::new();
				for (k,v) in val.val.iter() {
					let mut inter_buffer = vec![];
					safe_serialize(v, &mut inter_buffer, serialized_size_limit).unwrap();
					map.insert(k.clone(), inter_buffer);
				}
				let ser_deser = crate::homomorphic::types_impls::HX500NameSerDeser{ val: map};
				buffer = rkyv::to_bytes::<rkyv::rancor::Error>(&ser_deser).unwrap().to_vec();
				Ok(())
				
			}
			Self::DnsName(val) => {
				let mut inter_buffer1 = vec![];
				let mut inter_buffer2 = vec![];
				let mut inter_buffer3 = vec![];

				safe_serialize(&val.adr, &mut inter_buffer1, serialized_size_limit).unwrap();
				safe_serialize(&val.start_port, &mut inter_buffer2, serialized_size_limit).unwrap();
				safe_serialize(&val.end_port, &mut inter_buffer3, serialized_size_limit).unwrap();

				let ser_deser = crate::homomorphic::types_impls::HDnsNameSerDeser{
					adr: inter_buffer1, 
					start_port: inter_buffer2, 
					end_port: inter_buffer3, 
				};
				buffer = rkyv::to_bytes::<rkyv::rancor::Error>(&ser_deser).unwrap().to_vec();
				
				Ok(())
			}
			Self::IpAddress(val) => {
				safe_serialize(val, &mut buffer, serialized_size_limit)
			}
			_=>{
				panic!("type not yet supported for serialization",)
			}
		}.unwrap();
		buffer
	}

	pub fn get_type(&self) -> String {
		let res = match self {
			Self::Integer(val) => {
				STR_INTEGER
			}
			Self::Double(val) => {
				STR_DOUBLE
			}
			Self::String(val) => {
				STR_STRING
			}
			Self::Boolean(val) => {
				STR_BOOLEAN
			}
			Self::Time(val) => {
				STR_TIME
			}
			Self::DateTime(val) => {
				STR_DATE_TIME
			}
			Self::DayTimeDuration(val) => {
				STR_DAY_TIME_DURATION
			}
			Self::YearMonthDuration(val) => {
				STR_YEAR_MONTH_DURATION
			}
			Self::AnyUri(val) => {
				STR_ANY_URI
			}
			Self::Rfc822Name(val) => {
				STR_RFC_822_NAME
			}
			Self::X500Name(val) => {
				STR_X_500_NAME			
			}
			Self::DnsName(val) => {
				STR_DNS_NAME
			}
			Self::IpAddress(val) => {
				STR_IP_ADDRESS
			}
			_=>{
				panic!("type not yet supported for serialization",)
			}
		};
		res.to_string()
	}

	pub fn from_type(type_: String, v: &[u8]) -> Result<Self,String>{
		let serialized_size_limit = 1<<33;
		let r = match type_.as_str() {
			STR_INTEGER => {
				EncryptedTypes::Integer(safe_deserialize(v, serialized_size_limit)?)
			}
			STR_DOUBLE => {
				EncryptedTypes::Double(safe_deserialize(v, serialized_size_limit)?)
			}
			STR_STRING => {
				EncryptedTypes::String(safe_deserialize(v, serialized_size_limit)?)
			}
			STR_BOOLEAN => {
				EncryptedTypes::Boolean(safe_deserialize(v, serialized_size_limit)?)
			}
			STR_TIME => {
				EncryptedTypes::Time(safe_deserialize(v, serialized_size_limit)?)
			}
			STR_DATE_TIME => {
				EncryptedTypes::DateTime(safe_deserialize(v, serialized_size_limit)?)
			}
			STR_DAY_TIME_DURATION => {
				EncryptedTypes::DayTimeDuration(safe_deserialize(v, serialized_size_limit)?)
			}
			STR_YEAR_MONTH_DURATION => {
				EncryptedTypes::YearMonthDuration(safe_deserialize(v, serialized_size_limit)?)
			}
			STR_ANY_URI => {
				EncryptedTypes::AnyUri(safe_deserialize(v, serialized_size_limit)?)
			}
			STR_RFC_822_NAME => {
				let desered: HRfc822NameSerDeser = rkyv::from_bytes::<HRfc822NameSerDeser,rancor::Error>(v).map_err(|e|e.to_string())?;
				let localpart: FheAsciiString=  safe_deserialize(&desered.localpart[..], serialized_size_limit)?;
				let domainpart: FheAsciiString=  safe_deserialize(&desered.domainpart[..], serialized_size_limit)?;
				EncryptedTypes::Rfc822Name(HRfc822Name { localpart, domainpart })
			}
			STR_X_500_NAME => {
				let desered: HX500NameSerDeser = rkyv::from_bytes::<HX500NameSerDeser,rancor::Error>(v).map_err(|e|e.to_string())?;
				
				let mut map = HashMap::new();
				for (k,value) in desered.val {
					let desered_elem: FheAsciiString = safe_deserialize(&value[..], serialized_size_limit)?;
					map.insert(k.clone(), desered_elem);
				}

				
				EncryptedTypes::X500Name(HX500Name { val: map })
			}				
			STR_DNS_NAME => {
				let desered: HDnsNameSerDeser = rkyv::from_bytes::<HDnsNameSerDeser,rancor::Error>(v).map_err(|e|e.to_string())?;
				
				let adr: FheAsciiString = safe_deserialize(&desered.adr[..], serialized_size_limit)?;
				let start_port: HUnsignedSmallInteger = safe_deserialize(&desered.start_port[..], serialized_size_limit)?;
				let end_port: HUnsignedSmallInteger = safe_deserialize(&desered.end_port[..], serialized_size_limit)?;
				EncryptedTypes::DnsName(HDnsName { adr, start_port, end_port })
			}
			STR_IP_ADDRESS => {
				EncryptedTypes::IpAddress(safe_deserialize(v, serialized_size_limit)?)
			}
			_=> {
				panic!("unsupported string type : {}", type_)
			}
		};
		Ok(r)
	}
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,PartialEq)]
pub struct EncryptedAttributeSerStruct {
	pub type_: String,
	pub sered_value: Vec<u8>
}

impl From<EncryptedTypes> for EncryptedAttributeSerStruct {
	fn from(value: EncryptedTypes) -> Self {
		//	ser the data 
     	let sered_data = value.safe_ser();
      	let type_ = value.get_type();
      	EncryptedAttributeSerStruct { type_: type_, sered_value: sered_data }
	}
}


impl From<&EncryptedTypes> for EncryptedAttributeSerStruct {
	fn from(value: &EncryptedTypes) -> Self {
		//	ser the data 
     	let sered_data = value.safe_ser();
      	let type_ = value.get_type();
      	EncryptedAttributeSerStruct { type_: type_, sered_value: sered_data }
	}
}

impl From<EncryptedAttributeSerStruct> for EncryptedTypes {
	fn from(value: EncryptedAttributeSerStruct) -> Self {
      	EncryptedTypes::from_type(value.type_, &value.sered_value[..]).unwrap()
	}
}

impl From<&EncryptedAttributeSerStruct> for EncryptedTypes {
	fn from(value: &EncryptedAttributeSerStruct) -> Self {
      	EncryptedTypes::from_type(value.type_.clone(), &value.sered_value[..]).unwrap()
	}
}

// impl serde::Serialize for EncryptedTypes {
// 	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
// 		where
// 			S: serde::Serializer {
// 		let buffer = vec![];
// 		let serialized_size_limit = 1 << 30;
// 		match self {
// 			Self::Integer(val) => {
// 				safe_serialize(val, &mut buffer, serialized_size_limit)
// 			}
// 			Self::Double(val) => {
// 				safe_serialize(val, &mut buffer, serialized_size_limit)
// 			}
// 			Self::String(val) => {
// 				safe_serialize(val, &mut buffer, serialized_size_limit)
// 			}
// 			Self::Boolean(val) => {
// 				safe_serialize(val, &mut buffer, serialized_size_limit)
// 			}
// 			Self::Time(val) => {
// 				safe_serialize(val, &mut buffer, serialized_size_limit)
// 			}
// 			Self::DateTime(val) => {
// 				safe_serialize(val, &mut buffer, serialized_size_limit)
// 			}
// 			Self::DayTimeDuration(val) => {
// 				safe_serialize(val, &mut buffer, serialized_size_limit)
// 			}
// 			Self::YearMonthDuration(val) => {
// 				safe_serialize(val, &mut buffer, serialized_size_limit)
// 			}
// 			Self::AnyUri(val) => {
// 				safe_serialize(val, &mut buffer, serialized_size_limit)
// 			}
// 			Self::HexBinary(val) => {
// 				safe_serialize(val, &mut buffer, serialized_size_limit)
// 			}
// 			Self::Base64Binary(val) => {
// 				safe_serialize(val, &mut buffer, serialized_size_limit)
// 			}
// 			Self::Rfc822Name(val) => {
// 				safe_serialize(val, &mut buffer, serialized_size_limit)
// 			}
// 			Self::X500Name(val) => {
// 				safe_serialize(val, &mut buffer, serialized_size_limit)
// 			}
// 			Self::DnsName(val) => {
// 				safe_serialize(val, &mut buffer, serialized_size_limit)
// 			}
// 			Self::IpAddress(val) => {
// 				safe_serialize(val, &mut buffer, serialized_size_limit)
// 			}
// 		};
// 		safe_serialize(, writer, serialized_size_limit)
// 	}
// }


impl From<FheAsciiString> for EncryptedTypes {
	fn from(value: FheAsciiString) -> Self {
		Self::String(value.clone())
	}
}
impl From<FheInt64> for EncryptedTypes {
	fn from(value: FheInt64) -> Self {
		Self::Integer(value.clone())
	}
}
impl From<FheInt128> for EncryptedTypes {
	fn from(value: FheInt128) -> Self {
		Self::Double(value.clone())
	}
}
impl From<FheBool> for EncryptedTypes {
	fn from(value: FheBool) -> Self {
		Self::Boolean(value.clone())
	}
}

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,PartialEq)]
pub enum MixedAttributeSerStruct {
	EncryptedType(EncryptedAttributeSerStruct),
    IntermediateType(IntermediateTypes)
}

impl From<MixedTypes> for MixedAttributeSerStruct {
	fn from(value: MixedTypes) -> Self {
    	match value {
			MixedTypes::IntermediateType(t) => {
				MixedAttributeSerStruct::IntermediateType(t.clone())
			}
			MixedTypes::EncryptedType(t) => {
				MixedAttributeSerStruct::EncryptedType(t.into())
			}
     	}
	}
}


impl From<MixedAttributeSerStruct> for MixedTypes {
	fn from(value: MixedAttributeSerStruct) -> Self {
    	match value {
			MixedAttributeSerStruct::IntermediateType(t) => {
				MixedTypes::IntermediateType(t.clone())
			}
			MixedAttributeSerStruct::EncryptedType(t) => {
				MixedTypes::EncryptedType(t.into())
			}
     	}
	}
}

impl From<&MixedAttributeSerStruct> for MixedTypes {
	fn from(value: &MixedAttributeSerStruct) -> Self {
    	match value {
			MixedAttributeSerStruct::IntermediateType(t) => {
				MixedTypes::IntermediateType(t.clone())
			}
			MixedAttributeSerStruct::EncryptedType(t) => {
				MixedTypes::EncryptedType(t.into())
			}
     	}
	}
}

