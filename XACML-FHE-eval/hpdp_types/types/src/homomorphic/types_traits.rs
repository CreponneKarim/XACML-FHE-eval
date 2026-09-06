/// This file describes all the fundamental and non fundamental homomorphic types

pub mod HFundamentalTypes {
	pub trait HFundamentalType<T> {
		fn inner(&self) -> &T;
	}

	pub trait HIntegerType<T>:HFundamentalType<T> {
	}
	pub trait HUnsignedIntegerType<T>:HFundamentalType<T> {
	}
	pub trait HDoubleType<T>:HFundamentalType<T> {
	}
	pub trait HStringType<T>:HFundamentalType<T> {
	}
	pub trait HBooleanType<T>:HFundamentalType<T> {
	}
}

pub mod HXacmlSpecializedTypes {
    use crate::homomorphic::types_traits::HFundamentalTypes::{HIntegerType, HStringType, HUnsignedIntegerType};

	// pub trait  HTimeGroupType<S,T:HIntegerType<S>> {
	// }
	pub trait HTimeType<S,T:HUnsignedIntegerType<S>> {

	}
	pub trait HDateTimeType<S,T:HIntegerType<S>> {
		
	}
	pub trait HDayTimeDurationType<S,T:HIntegerType<S>> {
		
	}
	pub trait HYearMonthDurationType<S,T:HIntegerType<S>> {
		
	}
	pub trait HAnyUriType<S,T:HStringType<S>>{
		
	}
	pub trait HHexBinaryType<S,T:HStringType<S>> {
		
	}
	pub trait HBase64BinaryType<S,T:HStringType<S>> {
		
	}
	pub trait HRfc822NameType<S,T:HStringType<S>> {
		
	}
	pub trait HX500NameType<S,T:HStringType<S>> {
		
	}
	pub trait HIpAddressType<S,T:HStringType<S>> {
		
	}
	pub trait HDnsNameType<S,T:HStringType<S>> {
		
	}
	
}