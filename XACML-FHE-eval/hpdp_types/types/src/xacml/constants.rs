use anyhow::anyhow;

pub const STR_STRING : &str = "http://www.w3.org/2001/XMLSchema#string";
pub const STR_INTEGER : &str = "http://www.w3.org/2001/XMLSchema#integer";
pub const STR_DOUBLE : &str = "http://www.w3.org/2001/XMLSchema#double";
pub const STR_BOOLEAN : &str = "http://www.w3.org/2001/XMLSchema#boolean";
pub const STR_TIME : &str = "http://www.w3.org/2001/XMLSchema#time";
pub const STR_DATE : &str = "http://www.w3.org/2001/XMLSchema#date";
pub const STR_DATE_TIME : &str = "http://www.w3.org/2001/XMLSchema#dateTime";
pub const STR_DAY_TIME_DURATION : &str = "http://www.w3.org/2001/XMLSchema#dayTimeDuration";
pub const STR_YEAR_MONTH_DURATION : &str = "http://www.w3.org/2001/XMLSchema#yearMonthDuration";
pub const STR_ANY_URI : &str = "http://www.w3.org/2001/XMLSchema#anyUri";
pub const STR_HEX_BINARY : &str = "http://www.w3.org/2001/XMLSchema#hexBinary";
pub const STR_BASE_64_BINARY : &str = "http://www.w3.org/2001/XMLSchema#base64Binary";
pub const STR_RFC_822_NAME : &str = "urn:oasis:names:tc:xacml:1.0:data-type:rfc822Name"; 
pub const STR_X_500_NAME : &str = "urn:oasis:names:tc:xacml:1.0:data-type:x500Name"; 
pub const STR_DNS_NAME : &str = "urn:oasis:names:tc:xacml:2.0:data-type:dnsName"; 
pub const STR_IP_ADDRESS : &str = "urn:oasis:names:tc:xacml:2.0:data-type:ipAddress"; 

pub const STR_CAT_RESOURCE:&str = "urn:oasis:names:tc:xacml:3.0:attribute-category:resource";
pub const STR_CAT_ACTION:&str = "urn:oasis:names:tc:xacml:3.0:attribute-category:action";
pub const STR_CAT_ACCESS_SUBJECT:&str = "urn:oasis:names:tc:xacml:3.0:attribute-category:environment";


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum XacmlFunction {
    // --- Equality ---
    StringEqual,
    BooleanEqual,
    IntegerEqual,
    DoubleEqual,
    DateEqual,
    TimeEqual,
    DateTimeEqual,
    DayTimeDurationEqual,
    YearMonthDurationEqual,
    StringEqualIgnoreCase,
    AnyUriEqual,
    X500NameEqual,
    Rfc822NameEqual,
    HexBinaryEqual,
    Base64BinaryEqual,

    // --- Arithmetic & numeric ---
    IntegerAdd,
    DoubleAdd,
    IntegerSubtract,
    DoubleSubtract,
    IntegerMultiply,
    DoubleMultiply,
    IntegerDivide,
    DoubleDivide,
    IntegerMod,
    IntegerAbs,
    DoubleAbs,
    Round,
    Floor,

    // --- String normalization & casting ---
    StringNormalizeSpace,
    StringNormalizeToLowerCase,
    DoubleToInteger,
    IntegerToDouble,

    // --- Logical ---
    Or,
    And,
    NOf,
    Not,
    IfElse,

    // --- Numeric comparisons ---
    IntegerGreaterThan,
    IntegerGreaterThanOrEqual,
    IntegerLessThan,
    IntegerLessThanOrEqual,
    DoubleGreaterThan,
    DoubleGreaterThanOrEqual,
    DoubleLessThan,
    DoubleLessThanOrEqual,

    // --- Date/Time arithmetic (XACML 3.0) ---
    DateTimeAddDayTimeDuration,
    DateTimeAddYearMonthDuration,
    DateTimeSubtractDayTimeDuration,
    DateTimeSubtractYearMonthDuration,
    DateAddYearMonthDuration,
    DateSubtractYearMonthDuration,

    // --- String comparisons ---
    StringGreaterThan,
    StringGreaterThanOrEqual,
    StringLessThan,
    StringLessThanOrEqual,

    // --- Time/Date/DateTime comparisons + range ---
    TimeGreaterThan,
    TimeGreaterThanOrEqual,
    TimeLessThan,
    TimeLessThanOrEqual,
    TimeInRange,

    DateTimeGreaterThan,
    DateTimeGreaterThanOrEqual,
    DateTimeLessThan,
    DateTimeLessThanOrEqual,

    DateGreaterThan,
    DateGreaterThanOrEqual,
    DateLessThan,
    DateLessThanOrEqual,

    // --- Concatenation ---
    StringConcatenate,

    // --- Type conversions (string <-> type) ---
    BooleanFromString,
    StringFromBoolean,
    IntegerFromString,
    StringFromInteger,
    DoubleFromString,
    StringFromDouble,
    TimeFromString,
    StringFromTime,
    DateFromString,
    StringFromDate,
    DateTimeFromString,
    StringFromDateTime,
    AnyUriFromString,
    StringFromAnyUri,
    DayTimeDurationFromString,
    StringFromDayTimeDuration,
    YearMonthDurationFromString,
    StringFromYearMonthDuration,
    X500NameFromString,
    StringFromX500Name,
    Rfc822NameFromString,
    StringFromRfc822Name,
    IpAddressFromString,
    StringFromIpAddress,
    DnsNameFromString,
    StringFromDnsName,

    // --- String/URI pattern functions ---
    StringStartsWith,
    AnyUriStartsWith,
    StringEndsWith,
    AnyUriEndsWith,
    StringContains,
    AnyUriContains,
    StringSubstring,
    AnyUriSubstring,

    // --- Quantifier / set semantics ---
    AnyOf(Option<Box<XacmlFunction>>),
    AllOf(Option<Box<XacmlFunction>>),
    AnyOfAny(Option<Box<XacmlFunction>>),
    AllOfAny(Option<Box<XacmlFunction>>),
    AnyOfAll(Option<Box<XacmlFunction>>),
    AllOfAll(Option<Box<XacmlFunction>>),
    
    // --- Bagging functions ---
    StringOneAndOnly,
    StringBagSize,
    StringIsIn,
    StringBag,
    StringAtLeastOneMemberOf
}

impl XacmlFunction {
    /// URN for the function (exactly as in your tables).
    pub const fn urn(&self) -> &'static str {
        match self {
            // Equality
            Self::StringEqual => "urn:oasis:names:tc:xacml:1.0:function:string-equal",
            Self::BooleanEqual => "urn:oasis:names:tc:xacml:1.0:function:boolean-equal",
            Self::IntegerEqual => "urn:oasis:names:tc:xacml:1.0:function:integer-equal",
            Self::DoubleEqual => "urn:oasis:names:tc:xacml:1.0:function:double-equal",
            Self::DateEqual => "urn:oasis:names:tc:xacml:1.0:function:date-equal",
            Self::TimeEqual => "urn:oasis:names:tc:xacml:1.0:function:time-equal",
            Self::DateTimeEqual => "urn:oasis:names:tc:xacml:1.0:function:dateTime-equal",
            Self::DayTimeDurationEqual => "urn:oasis:names:tc:xacml:3.0:function:dayTimeDuration-equal",
            Self::YearMonthDurationEqual => "urn:oasis:names:tc:xacml:3.0:function:yearMonthDuration-equal",
            Self::StringEqualIgnoreCase => "urn:oasis:names:tc:xacml:1.0:function:string-equal-ignore-case",
            Self::AnyUriEqual => "urn:oasis:names:tc:xacml:1.0:function:anyURI-equal",
            Self::X500NameEqual => "urn:oasis:names:tc:xacml:1.0:function:x500Name-equal",
            Self::Rfc822NameEqual => "urn:oasis:names:tc:xacml:1.0:function:rfc822Name-equal",
            Self::HexBinaryEqual => "urn:oasis:names:tc:xacml:1.0:function:hexBinary-equal",
            Self::Base64BinaryEqual => "urn:oasis:names:tc:xacml:1.0:function:base64Binary-equal",

            // Arithmetic & numeric
            Self::IntegerAdd => "urn:oasis:names:tc:xacml:1.0:function:integer-add",
            Self::DoubleAdd => "urn:oasis:names:tc:xacml:1.0:function:double-add",
            Self::IntegerSubtract => "urn:oasis:names:tc:xacml:1.0:function:integer-subtract",
            Self::DoubleSubtract => "urn:oasis:names:tc:xacml:1.0:function:double-subtract",
            Self::IntegerMultiply => "urn:oasis:names:tc:xacml:1.0:function:integer-multiply",
            Self::DoubleMultiply => "urn:oasis:names:tc:xacml:1.0:function:double-multiply",
            Self::IntegerDivide => "urn:oasis:names:tc:xacml:1.0:function:integer-divide",
            Self::DoubleDivide => "urn:oasis:names:tc:xacml:1.0:function:double-divide",
            Self::IntegerMod => "urn:oasis:names:tc:xacml:1.0:function:integer-mod",
            Self::IntegerAbs => "urn:oasis:names:tc:xacml:1.0:function:integer-abs",
            Self::DoubleAbs => "urn:oasis:names:tc:xacml:1.0:function:double-abs",
            Self::Round => "urn:oasis:names:tc:xacml:1.0:function:round",
            Self::Floor => "urn:oasis:names:tc:xacml:1.0:function:floor",

            // String normalization & casting
            Self::StringNormalizeSpace => "urn:oasis:names:tc:xacml:1.0:function:string-normalize-space",
            Self::StringNormalizeToLowerCase => "urn:oasis:names:tc:xacml:1.0:function:string-normalize-to-lower-case",
            Self::DoubleToInteger => "urn:oasis:names:tc:xacml:1.0:function:double-to-integer",
            Self::IntegerToDouble => "urn:oasis:names:tc:xacml:1.0:function:integer-to-double",

            // Logical
            Self::Or => "urn:oasis:names:tc:xacml:1.0:function:or",
            Self::And => "urn:oasis:names:tc:xacml:1.0:function:and",
            Self::NOf => "urn:oasis:names:tc:xacml:1.0:function:n-of",
            Self::Not => "urn:oasis:names:tc:xacml:1.0:function:not",
            Self::IfElse => "urn:oasis:names:tc:xacml:1.0:function:ifelse",

            // Numeric comparisons
            Self::IntegerGreaterThan => "urn:oasis:names:tc:xacml:1.0:function:integer-greater-than",
            Self::IntegerGreaterThanOrEqual => "urn:oasis:names:tc:xacml:1.0:function:integer-greater-than-or-equal",
            Self::IntegerLessThan => "urn:oasis:names:tc:xacml:1.0:function:integer-less-than",
            Self::IntegerLessThanOrEqual => "urn:oasis:names:tc:xacml:1.0:function:integer-less-than-or-equal",
            Self::DoubleGreaterThan => "urn:oasis:names:tc:xacml:1.0:function:double-greater-than",
            Self::DoubleGreaterThanOrEqual => "urn:oasis:names:tc:xacml:1.0:function:double-greater-than-or-equal",
            Self::DoubleLessThan => "urn:oasis:names:tc:xacml:1.0:function:double-less-than",
            Self::DoubleLessThanOrEqual => "urn:oasis:names:tc:xacml:1.0:function:double-less-than-or-equal",
            Self::DateGreaterThan => "urn:oasis:names:tc:xacml:1.0:function:date-greater-than",
            Self::DateGreaterThanOrEqual => "urn:oasis:names:tc:xacml:1.0:function:date-greater-than-or-equal",
            Self::DateLessThan => "urn:oasis:names:tc:xacml:1.0:function:date-less-than",
            Self::DateLessThanOrEqual => "urn:oasis:names:tc:xacml:1.0:function:date-less-than-or-equal",

            
            // Date/Time arithmetic
            Self::DateTimeAddDayTimeDuration => "urn:oasis:names:tc:xacml:3.0:function:dateTime-add-dayTimeDuration",
            Self::DateTimeAddYearMonthDuration => "urn:oasis:names:tc:xacml:3.0:function:dateTime-add-yearMonthDuration",
            Self::DateTimeSubtractDayTimeDuration => "urn:oasis:names:tc:xacml:3.0:function:dateTime-subtract-dayTimeDuration",
            Self::DateTimeSubtractYearMonthDuration => "urn:oasis:names:tc:xacml:3.0:function:dateTime-subtract-yearMonthDuration",
            Self::DateAddYearMonthDuration => "urn:oasis:names:tc:xacml:3.0:function:date-add-yearMonthDuration",
            Self::DateSubtractYearMonthDuration => "urn:oasis:names:tc:xacml:3.0:function:date-subtract-yearMonthDuration",

            // String comparisons
            Self::StringGreaterThan => "urn:oasis:names:tc:xacml:1.0:function:string-greater-than",
            Self::StringGreaterThanOrEqual => "urn:oasis:names:tc:xacml:1.0:function:string-greater-than-or-equal",
            Self::StringLessThan => "urn:oasis:names:tc:xacml:1.0:function:string-less-than",
            Self::StringLessThanOrEqual => "urn:oasis:names:tc:xacml:1.0:function:string-less-than-or-equal",

            // Time / Date / DateTime comparisons + range
            Self::TimeGreaterThan => "urn:oasis:names:tc:xacml:1.0:function:time-greater-than",
            Self::TimeGreaterThanOrEqual => "urn:oasis:names:tc:xacml:1.0:function:time-greater-than-or-equal",
            Self::TimeLessThan => "urn:oasis:names:tc:xacml:1.0:function:time-less-than",
            Self::TimeLessThanOrEqual => "urn:oasis:names:tc:xacml:1.0:function:time-less-than-or-equal",
            Self::TimeInRange => "urn:oasis:names:tc:xacml:2.0:function:time-in-range",

            Self::DateTimeGreaterThan => "urn:oasis:names:tc:xacml:1.0:function:dateTime-greater-than",
            Self::DateTimeGreaterThanOrEqual => "urn:oasis:names:tc:xacml:1.0:function:dateTime-greater-than-or-equal",
            Self::DateTimeLessThan => "urn:oasis:names:tc:xacml:1.0:function:dateTime-less-than",
            Self::DateTimeLessThanOrEqual => "urn:oasis:names:tc:xacml:1.0:function:dateTime-less-than-or-equal",

            Self::DateGreaterThan => "urn:oasis:names:tc:xacml:1.0:function:date-greater-than",
            Self::DateGreaterThanOrEqual => "urn:oasis:names:tc:xacml:1.0:function:date-greater-than-or-equal",
            Self::DateLessThan => "urn:oasis:names:tc:xacml:1.0:function:date-less-than",
            Self::DateLessThanOrEqual => "urn:oasis:names:tc:xacml:1.0:function:date-less-than-or-equal",

            // Concatenation
            Self::StringConcatenate => "urn:oasis:names:tc:xacml:2.0:function:string-concatenate",

            // Type conversion
            Self::BooleanFromString => "urn:oasis:names:tc:xacml:3.0:function:boolean-from-string",
            Self::StringFromBoolean => "urn:oasis:names:tc:xacml:3.0:function:string-from-boolean",
            Self::IntegerFromString => "urn:oasis:names:tc:xacml:3.0:function:integer-from-string",
            Self::StringFromInteger => "urn:oasis:names:tc:xacml:3.0:function:string-from-integer",
            Self::DoubleFromString => "urn:oasis:names:tc:xacml:3.0:function:double-from-string",
            Self::StringFromDouble => "urn:oasis:names:tc:xacml:3.0:function:string-from-double",
            Self::TimeFromString => "urn:oasis:names:tc:xacml:3.0:function:time-from-string",
            Self::StringFromTime => "urn:oasis:names:tc:xacml:3.0:function:string-from-time",
            Self::DateFromString => "urn:oasis:names:tc:xacml:3.0:function:date-from-string",
            Self::StringFromDate => "urn:oasis:names:tc:xacml:3.0:function:string-from-date",
            Self::DateTimeFromString => "urn:oasis:names:tc:xacml:3.0:function:dateTime-from-string",
            Self::StringFromDateTime => "urn:oasis:names:tc:xacml:3.0:function:string-from-dateTime",
            Self::AnyUriFromString => "urn:oasis:names:tc:xacml:3.0:function:anyURI-from-string",
            Self::StringFromAnyUri => "urn:oasis:names:tc:xacml:3.0:function:string-from-anyURI",
            Self::DayTimeDurationFromString => "urn:oasis:names:tc:xacml:3.0:function:dayTimeDuration-from-string",
            Self::StringFromDayTimeDuration => "urn:oasis:names:tc:xacml:3.0:function:string-from-dayTimeDuration",
            Self::YearMonthDurationFromString => "urn:oasis:names:tc:xacml:3.0:function:yearMonthDuration-from-string",
            Self::StringFromYearMonthDuration => "urn:oasis:names:tc:xacml:3.0:function:string-from-yearMonthDuration",
            Self::X500NameFromString => "urn:oasis:names:tc:xacml:3.0:function:x500Name-from-string",
            Self::StringFromX500Name => "urn:oasis:names:tc:xacml:3.0:function:string-from-x500Name",
            Self::Rfc822NameFromString => "urn:oasis:names:tc:xacml:3.0:function:rfc822Name-from-string",
            Self::StringFromRfc822Name => "urn:oasis:names:tc:xacml:3.0:function:string-from-rfc822Name",
            Self::IpAddressFromString => "urn:oasis:names:tc:xacml:3.0:function:ipAddress-from-string",
            Self::StringFromIpAddress => "urn:oasis:names:tc:xacml:3.0:function:string-from-ipAddress",
            Self::DnsNameFromString => "urn:oasis:names:tc:xacml:3.0:function:dnsName-from-string",
            Self::StringFromDnsName => "urn:oasis:names:tc:xacml:3.0:function:string-from-dnsName",

            // String / URI pattern
            Self::StringStartsWith => "urn:oasis:names:tc:xacml:3.0:function:string-starts-with",
            Self::AnyUriStartsWith => "urn:oasis:names:tc:xacml:3.0:function:anyURI-starts-with",
            Self::StringEndsWith => "urn:oasis:names:tc:xacml:3.0:function:string-ends-with",
            Self::AnyUriEndsWith => "urn:oasis:names:tc:xacml:3.0:function:anyURI-ends-with",
            Self::StringContains => "urn:oasis:names:tc:xacml:3.0:function:string-contains",
            Self::AnyUriContains => "urn:oasis:names:tc:xacml:3.0:function:anyURI-contains",
            Self::StringSubstring => "urn:oasis:names:tc:xacml:3.0:function:string-substring",
            Self::AnyUriSubstring => "urn:oasis:names:tc:xacml:3.0:function:anyURI-substring",

            // Quantifier / set semantics
            Self::AnyOf(_) => "urn:oasis:names:tc:xacml:1.0:function:any-of",
            Self::AllOf(_) => "urn:oasis:names:tc:xacml:1.0:function:all-of",
            Self::AnyOfAny(_) => "urn:oasis:names:tc:xacml:1.0:function:any-of-any",
            Self::AllOfAny(_) => "urn:oasis:names:tc:xacml:1.0:function:all-of-any",
            Self::AnyOfAll(_) => "urn:oasis:names:tc:xacml:1.0:function:any-of-all",
            Self::AllOfAll(_) => "urn:oasis:names:tc:xacml:1.0:function:all-of-all",
            
            Self::StringOneAndOnly => "urn:oasis:names:tc:xacml:1.0:function:string-one-and-only",
            Self::StringBagSize => "urn:oasis:names:tc:xacml:3.0:function:string-bag-size",
            Self::StringIsIn => "urn:oasis:names:tc:xacml:3.0:function:string-is-in",
            Self::StringBag => "urn:oasis:names:tc:xacml:1.0:function:string-bag",
            Self::StringAtLeastOneMemberOf => "urn:oasis:names:tc:xacml:1.0:function:string-at-least-one-member-of",
        }
    }

    /// Parse from a URN into the enum.
    pub fn from_urn(urn: &str, urn2: Option<&str>) -> Option<Self> {
        // O(N) is fine for ~100 items; switch to phf/perfect-hash later if you want.
        Self::ALL
        	.iter()
        	.find(|f| 
         		f.urn().eq(urn) 
      		).and_then(|c|{
        		let mut result = c.clone();
          		match result {
          			Self::AnyOf(_) => {Some(Self::AnyOf(Some(Box::new(XacmlFunction::from_urn(urn2.unwrap(), None).unwrap()))))},
           			Self::AllOf(_) => {Some(Self::AllOf(Some(Box::new(XacmlFunction::from_urn(urn2.unwrap(), None).unwrap()))))},
           			Self::AnyOfAny(_) => {Some(Self::AnyOfAny(Some(Box::new(XacmlFunction::from_urn(urn2.unwrap(), None).unwrap()))))},
           			Self::AllOfAny(_) => {Some(Self::AllOfAny(Some(Box::new(XacmlFunction::from_urn(urn2.unwrap(), None).unwrap()))))},
           			Self::AnyOfAll(_) => {Some(Self::AnyOfAll(Some(Box::new(XacmlFunction::from_urn(urn2.unwrap(), None).unwrap()))))},
           			Self::AllOfAll(_) => {Some(Self::AllOfAll(Some(Box::new(XacmlFunction::from_urn(urn2.unwrap(), None).unwrap()))))},
              		_=>{Some(result)}
          		}
        	})
    }

    /// All functions in the preserved order.
    pub const ALL: &'static [Self] = &[
        Self::StringEqual,
        Self::BooleanEqual,
        Self::IntegerEqual,
        Self::DoubleEqual,
        Self::DateEqual,
        Self::TimeEqual,
        Self::DateTimeEqual,
        Self::DayTimeDurationEqual,
        Self::YearMonthDurationEqual,
        Self::StringEqualIgnoreCase,
        Self::AnyUriEqual,
        Self::X500NameEqual,
        Self::Rfc822NameEqual,
        Self::HexBinaryEqual,
        Self::Base64BinaryEqual,
        Self::IntegerAdd,
        Self::DoubleAdd,
        Self::IntegerSubtract,
        Self::DoubleSubtract,
        Self::IntegerMultiply,
        Self::DoubleMultiply,
        Self::IntegerDivide,
        Self::DoubleDivide,
        Self::IntegerMod,
        Self::IntegerAbs,
        Self::DoubleAbs,
        Self::Round,
        Self::Floor,
        Self::StringNormalizeSpace,
        Self::StringNormalizeToLowerCase,
        Self::DoubleToInteger,
        Self::IntegerToDouble,
        Self::Or,
        Self::And,
        Self::NOf,
        Self::Not,
        Self::IfElse,
        Self::IntegerGreaterThan,
        Self::IntegerGreaterThanOrEqual,
        Self::IntegerLessThan,
        Self::IntegerLessThanOrEqual,
        Self::DoubleGreaterThan,
        Self::DoubleGreaterThanOrEqual,
        Self::DoubleLessThan,
        Self::DoubleLessThanOrEqual,
        Self::DateTimeAddDayTimeDuration,
        Self::DateTimeAddYearMonthDuration,
        Self::DateTimeSubtractDayTimeDuration,
        Self::DateTimeSubtractYearMonthDuration,
        Self::DateAddYearMonthDuration,
        Self::DateSubtractYearMonthDuration,
        Self::StringGreaterThan,
        Self::StringGreaterThanOrEqual,
        Self::StringLessThan,
        Self::StringLessThanOrEqual,
        Self::TimeGreaterThan,
        Self::TimeGreaterThanOrEqual,
        Self::TimeLessThan,
        Self::TimeLessThanOrEqual,
        Self::TimeInRange,
        Self::DateTimeGreaterThan,
        Self::DateTimeGreaterThanOrEqual,
        Self::DateTimeLessThan,
        Self::DateTimeLessThanOrEqual,
        Self::DateGreaterThan,
        Self::DateGreaterThanOrEqual,
        Self::DateLessThan,
        Self::DateLessThanOrEqual,
        Self::StringConcatenate,
        Self::BooleanFromString,
        Self::StringFromBoolean,
        Self::IntegerFromString,
        Self::StringFromInteger,
        Self::DoubleFromString,
        Self::StringFromDouble,
        Self::TimeFromString,
        Self::StringFromTime,
        Self::DateFromString,
        Self::StringFromDate,
        Self::DateTimeFromString,
        Self::StringFromDateTime,
        Self::AnyUriFromString,
        Self::StringFromAnyUri,
        Self::DayTimeDurationFromString,
        Self::StringFromDayTimeDuration,
        Self::YearMonthDurationFromString,
        Self::StringFromYearMonthDuration,
        Self::X500NameFromString,
        Self::StringFromX500Name,
        Self::Rfc822NameFromString,
        Self::StringFromRfc822Name,
        Self::IpAddressFromString,
        Self::StringFromIpAddress,
        Self::DnsNameFromString,
        Self::StringFromDnsName,
        Self::StringStartsWith,
        Self::AnyUriStartsWith,
        Self::StringEndsWith,
        Self::AnyUriEndsWith,
        Self::StringContains,
        Self::AnyUriContains,
        Self::StringSubstring,
        Self::AnyUriSubstring,
        Self::AnyOf(None),
        Self::AllOf(None),
        Self::AnyOfAny(None),
        Self::AllOfAny(None),
        Self::AnyOfAll(None),
        Self::AllOfAll(None),
        
        
        // --- Bagging functions ---
        Self::StringOneAndOnly,
        Self::StringBagSize,
        Self::StringIsIn,
        Self::StringBag,
        Self::StringAtLeastOneMemberOf,
    ];
}

//  define tpes used here
pub type USmallInteger 			= u32;

pub type SmallInteger 			= i32;
pub type NormalInteger 		    = i64;
pub type LongInteger 			= i128;
pub type UnsignedSmallInteger 	= u32;
pub type UnsignedNormalInteger  = u64;
pub type UnsignedLongInteger 	= u128;
pub type Double 				= i128;
// pub type String				    = String;
pub type Boolean				= bool;

#[derive(Clone)]
pub enum XacmlDataTypesStrings {
    String,
    Integer,
    Double,
    Boolean,
    Time,
    Date,
    DateTime,
    DayTimeDuration,
    YearMonthDuration,
    AnyUri,
    HexBinary,
    Base64Binary,
    Rfc822Name,
    X500Name,
    DnsName,
    IpAddress
}

impl ToString for XacmlDataTypesStrings {
    fn to_string(&self) -> String {
        match self {
            XacmlDataTypesStrings::String => {
                STR_STRING.to_string()
            },
            XacmlDataTypesStrings::Integer => {
                STR_INTEGER.to_string()
            },
            XacmlDataTypesStrings::Double => {
                STR_DOUBLE.to_string()
            },
            XacmlDataTypesStrings::Boolean => {
                STR_BOOLEAN.to_string()
            },
            // XacmlDataTypesStrings::Bag => {
            //     "http://www.w3.org/2001/XMLSchema#bag"
            // },
            // XacmlDataTypesStrings::Set => {
            //     "http://www.w3.org/2001/XMLSchema#Set"
            // },
            XacmlDataTypesStrings::Time => {
                STR_TIME.to_string()
            },
            XacmlDataTypesStrings::Date => {
                STR_DATE.to_string()
            },
            XacmlDataTypesStrings::DateTime => {
                STR_DATE_TIME.to_string()
            },
            XacmlDataTypesStrings::DayTimeDuration => {
                STR_DAY_TIME_DURATION.to_string()
            },
            XacmlDataTypesStrings::YearMonthDuration => {
                STR_YEAR_MONTH_DURATION.to_string()
            },
            XacmlDataTypesStrings::AnyUri => {
                STR_ANY_URI.to_string()
            },
            XacmlDataTypesStrings::HexBinary => {
                STR_HEX_BINARY.to_string()
            },
            XacmlDataTypesStrings::Base64Binary => {
                STR_BASE_64_BINARY.to_string()
            },
            XacmlDataTypesStrings::Rfc822Name => {
                STR_RFC_822_NAME.to_string()
            },
            XacmlDataTypesStrings::X500Name => {
                STR_X_500_NAME.to_string()
            },
            XacmlDataTypesStrings::DnsName => {
                STR_DNS_NAME.to_string()
            },
            XacmlDataTypesStrings::IpAddress => {
                STR_IP_ADDRESS.to_string()
            }
            
        }
    }
}

impl XacmlDataTypesStrings {
    pub fn from_str(s:&str) -> Result<Self,anyhow::Error> {
        match s {
            STR_STRING =>               Ok(Self::String),
            STR_INTEGER =>              Ok(Self::Integer),
            STR_DOUBLE =>               Ok(Self::Double),
            STR_BOOLEAN =>              Ok(Self::Boolean),
            STR_TIME =>                 Ok(Self::Time),
            STR_DATE_TIME =>            Ok(Self::DateTime),
            STR_DATE =>            		Ok(Self::Date),
            STR_DAY_TIME_DURATION =>    Ok(Self::DayTimeDuration),
            STR_YEAR_MONTH_DURATION =>  Ok(Self::YearMonthDuration),
            STR_ANY_URI =>              Ok(Self::AnyUri),
            STR_HEX_BINARY =>           Ok(Self::HexBinary),
            STR_BASE_64_BINARY =>       Ok(Self::Base64Binary),
            STR_RFC_822_NAME =>         Ok(Self::Rfc822Name),
            STR_X_500_NAME =>           Ok(Self::X500Name),
            STR_DNS_NAME =>             Ok(Self::DnsName),
            STR_IP_ADDRESS =>           Ok(Self::IpAddress),
            _=> {
                Err(anyhow!("cannot find match for type \"{}\"\n",s))
            }
        }
    }
}

/// XACML Rule Combining Algorithms (RuleCombiningAlgId)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuleCombiningAlg {
    DenyOverrides,        // XACML 3.0
    PermitOverrides,      // XACML 3.0
    FirstApplicable,      // XACML 1.0
    OrderedDenyOverrides, // XACML 3.0
    OrderedPermitOverrides,// XACML 3.0
    DenyUnlessPermit,     // XACML 3.0
    PermitUnlessDeny,     // XACML 3.0
}

impl RuleCombiningAlg {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuleCombiningAlg::DenyOverrides =>
                "urn:oasis:names:tc:xacml:3.0:rule-combining-algorithm:deny-overrides",
            RuleCombiningAlg::PermitOverrides =>
                "urn:oasis:names:tc:xacml:3.0:rule-combining-algorithm:permit-overrides",
            RuleCombiningAlg::FirstApplicable =>
                "urn:oasis:names:tc:xacml:1.0:rule-combining-algorithm:first-applicable",
            RuleCombiningAlg::OrderedDenyOverrides =>
                "urn:oasis:names:tc:xacml:3.0:rule-combining-algorithm:ordered-deny-overrides",
            RuleCombiningAlg::OrderedPermitOverrides =>
                "urn:oasis:names:tc:xacml:3.0:rule-combining-algorithm:ordered-permit-overrides",
            RuleCombiningAlg::DenyUnlessPermit =>
                "urn:oasis:names:tc:xacml:3.0:rule-combining-algorithm:deny-unless-permit",
            RuleCombiningAlg::PermitUnlessDeny =>
                "urn:oasis:names:tc:xacml:3.0:rule-combining-algorithm:permit-unless-deny",
        }
    }
}

/// XACML Policy Combining Algorithms (PolicyCombiningAlgId)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PolicyCombiningAlg {
    DenyOverrides,         // XACML 3.0
    PermitOverrides,       // XACML 3.0
    FirstApplicable,       // XACML 1.0
    OnlyOneApplicable,     // XACML 1.0
    OrderedDenyOverrides,  // XACML 3.0
    OrderedPermitOverrides,// XACML 3.0
    DenyUnlessPermit,      // XACML 3.0
    PermitUnlessDeny,      // XACML 3.0
}

impl PolicyCombiningAlg {
    pub fn as_str(&self) -> &'static str {
        match self {
            PolicyCombiningAlg::DenyOverrides =>
                "urn:oasis:names:tc:xacml:3.0:policy-combining-algorithm:deny-overrides",
            PolicyCombiningAlg::PermitOverrides =>
                "urn:oasis:names:tc:xacml:3.0:policy-combining-algorithm:permit-overrides",
            PolicyCombiningAlg::FirstApplicable =>
                "urn:oasis:names:tc:xacml:1.0:policy-combining-algorithm:first-applicable",
            PolicyCombiningAlg::OnlyOneApplicable =>
                "urn:oasis:names:tc:xacml:1.0:policy-combining-algorithm:only-one-applicable",
            PolicyCombiningAlg::OrderedDenyOverrides =>
                "urn:oasis:names:tc:xacml:3.0:policy-combining-algorithm:ordered-deny-overrides",
            PolicyCombiningAlg::OrderedPermitOverrides =>
                "urn:oasis:names:tc:xacml:3.0:policy-combining-algorithm:ordered-permit-overrides",
            PolicyCombiningAlg::DenyUnlessPermit =>
                "urn:oasis:names:tc:xacml:3.0:policy-combining-algorithm:deny-unless-permit",
            PolicyCombiningAlg::PermitUnlessDeny =>
                "urn:oasis:names:tc:xacml:3.0:policy-combining-algorithm:permit-unless-deny",
        }
    }
}
