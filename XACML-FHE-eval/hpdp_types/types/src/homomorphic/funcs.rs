// Assumptions you requested (encoding layer):
// - encrypted double            => HLongINteger
// - encrypted Time              => HUnsignedSmallInteger
// - encrypted DateTime          => HNormalInteger
// - encrypted DayTimeDuration   => HNormalInteger
// - encrypted YearMonthDuration => HNormalInteger
// - encrypted AnyUri            => encrypted string (FheAsciiString)
// - encrypted IpAddress         => encrypted string (FheAsciiString)
// - encrypted Base64Binary      => CpuFheUint8Array   (same for Base64Binary)

// Cargo.toml (example):
// tfhe = { version = "~1.4.3", features = ["integer", "strings"] }  // strings needed for AnyUri/IpAddress ops

use std::panic;
use std::time::Instant;
use std::todo;

use rkyv::rancor::OptionExt;
use tfhe::array::fhe_base_radix_ciphertext_array_eq;
use tfhe::array::fhe_uint_array_eq;
use tfhe::prelude::*;

//#[cfg(feature = "strings")]
use tfhe::FheAsciiString;

// NOTE: Depending on your tfhe-rs version/module layout, CpuFheUint8Array
// may live in a different path. Adjust the import if needed.
use tfhe::CpuFheUint8Array;

use crate::homomorphic::policy::EncryptedTypes;
use crate::homomorphic::types_impls::HBase64Binary;
use crate::{homomorphic::types_impls::{HBoolean, HLongInteger, HNormalInteger, HUnsignedSmallInteger}, xacml::constants::XacmlFunction};

#[derive(Debug, Clone)]
pub enum DispatchError {
    UnsupportedFunction { func: XacmlFunction },
    ArityMismatch { func: XacmlFunction, expected: usize, got: usize },
    TypeMismatch { func: XacmlFunction, expected: &'static str, got: &'static str },
    NotFoundOperatorFromUrn{message: &'static str, not_found: String},
    X500NameNotAllFieldsSupplied{message: &'static str},
    AbsentInnerFunction { message: &'static str},
    
    Indeterminate {
        func: XacmlFunction,
        reason: &'static str,
    },
}

impl EncryptedTypes {
    pub fn ty(&self) -> &'static str {
        match self {
            EncryptedTypes::Boolean(_) => "Bool",
            EncryptedTypes::Integer(_) => "Int64",
            EncryptedTypes::Double(_) => "Double(HLongINteger)",
            EncryptedTypes::Time(_) => "Time(HUnsignedSmallInteger)",
            EncryptedTypes::Date(_) => "Date(HNormalInteger)",
            EncryptedTypes::DateTime(_) => "DateTime(HNormalInteger)",
            EncryptedTypes::DayTimeDuration(_) => "DayTimeDuration(HNormalInteger)",
            EncryptedTypes::YearMonthDuration(_) => "YearMonthDuration(HNormalInteger)",
            //#[cfg(feature = "strings")]
            EncryptedTypes::String(_) => "String(FheAsciiString)",
            //#[cfg(feature = "strings")]
            EncryptedTypes::AnyUri(_) => "AnyUri(FheAsciiString)",
            EncryptedTypes::HexBinary(_) => "HexBinary(CpuFheUint8Array)",
            EncryptedTypes::Base64Binary(_) => "Base64Binary(CpuFheUint8Array)",
            EncryptedTypes::Rfc822Name(_) => "Rfc822Name(HRfc822Name)",
            EncryptedTypes::X500Name(_) => "X500Name(HX500Name)",
            EncryptedTypes::DnsName(_) => "DnsName(HDnsName)",
            //#[cfg(feature = "strings")]
            EncryptedTypes::IpAddress(_) => "IpAddress(FheAsciiString)",
            EncryptedTypes::Bag(_) => "Bag(EncryptedTypes)",
            EncryptedTypes::Nothing => "Nothing"
        }
    }
}

pub enum TfheOp {
    Unary(fn(XacmlFunction, &EncryptedTypes) -> Result<EncryptedTypes, DispatchError>),
    Binary(fn(XacmlFunction, &EncryptedTypes, &EncryptedTypes) -> Result<EncryptedTypes, DispatchError>),
    Ternary(fn(XacmlFunction, &EncryptedTypes, &EncryptedTypes, &EncryptedTypes) -> Result<EncryptedTypes, DispatchError>),
    // Complex(fn(XacmlFunction, XacmlFunction, &EncryptedTypes, &EncryptedTypes) -> Result<EncryptedTypes, DispatchError>)
}

pub fn eval_unary(func: XacmlFunction, a: &EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match resolve_tfhe_op(&func)? {
        TfheOp::Unary(f) => f(func, a),
        _ => Err(DispatchError::ArityMismatch { func, expected: 1, got: 1 }),
    }
}

pub fn eval_binary(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match resolve_tfhe_op(&func)? {
        TfheOp::Binary(f) => f(func, a, b),
        _ => Err(DispatchError::ArityMismatch { func, expected: 2, got: 2 }),
    }
}

pub fn eval_ternary(
    func: XacmlFunction,
    a: &EncryptedTypes,
    b: &EncryptedTypes,
    c: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {
    match resolve_tfhe_op(&func)? {
        TfheOp::Ternary(f) => f(func, a, b, c),
        _ => Err(DispatchError::ArityMismatch { func, expected: 3, got: 3 }),
    }
}

// pub fn eval_binary_bag_bag(
//     func: XacmlFunction,
//     a: &[EncryptedTypes],
//     b: &[EncryptedTypes],
// ) -> Result<EncryptedTypes, DispatchError> {
//     match resolve_tfhe_op(func)? {
//         TfheOp::BinaryBagBag(f) => f(func, a, b),
//         _ => Err(DispatchError::ArityMismatch { func, expected: 2, got: 0 }),
//     }
// }

// pub fn eval_binary_val_bag(
//     func: XacmlFunction,
//     a: &EncryptedTypes,
//     b: &[EncryptedTypes],
// ) -> Result<EncryptedTypes, DispatchError> {
//     match resolve_tfhe_op(func)? {
//         TfheOp::BinaryValBag(f) => f(func, a, b),
//         _ => Err(DispatchError::ArityMismatch { func, expected: 2, got: 0 }),
//     }
// }

pub fn evaluate_function(
    urn: &str,
    urn_2: Option<&str>,
    arguments: Vec<Option<&EncryptedTypes>>,
) -> Result<EncryptedTypes, DispatchError> {
    let xacml_function_enum = XacmlFunction::from_urn(&urn, urn_2)
        .ok_or(DispatchError::NotFoundOperatorFromUrn {
            message: "Did not find the operator from the URN ",
            not_found: urn.to_string(),
        })?;

    
    if matches!(xacml_function_enum, XacmlFunction::And | XacmlFunction::Or) {
        return apply_bool_vec_op_from_borrowed_args(xacml_function_enum, arguments);
    }
    
    if matches!(xacml_function_enum, XacmlFunction::StringBag) {
        return apply_string_bag_from_borrowed_args(xacml_function_enum, arguments);
    }

    let result = resolve_tfhe_op(&xacml_function_enum)
        .and_then(|inner| match inner {
            TfheOp::Unary(_) => {
                assert!(arguments.len() == 1);
                assert!(arguments.get(0).unwrap().is_some());

                let param1 = arguments.get(0).unwrap().as_ref().unwrap();

                eval_unary(xacml_function_enum, param1)
            }

            TfheOp::Binary(_) => {
                assert!(arguments.len() == 2);
                assert!(arguments.get(0).unwrap().is_some());
                assert!(arguments.get(1).unwrap().is_some());

                let param1 = arguments.get(0).unwrap().as_ref().unwrap();
                let param2 = arguments.get(1).unwrap().as_ref().unwrap();

                eval_binary(xacml_function_enum, param1, param2)
            }

            TfheOp::Ternary(_) => {
                panic!("Unsupported ternary operator detected - Target")
            }
        });

    result
}


pub fn evaluate_function_consume(
    urn: &str,
    urn_2: Option<&str>,
    arguments: Vec<Option<EncryptedTypes>>,
) -> Result<EncryptedTypes, DispatchError> {
    let xacml_function_enum = XacmlFunction::from_urn(urn, urn_2)
        .ok_or(DispatchError::NotFoundOperatorFromUrn {
            message: "Did not find the operator from the URN ",
            not_found: urn.to_string(),
        })?;

    if matches!(xacml_function_enum, XacmlFunction::StringBag) {
        return apply_string_bag_from_owned_args(xacml_function_enum, arguments);
    }

    if matches!(xacml_function_enum, XacmlFunction::And | XacmlFunction::Or) {
        return apply_bool_vec_op_from_owned_args(xacml_function_enum, arguments);
    }
    
    let result = resolve_tfhe_op(&xacml_function_enum)
        .and_then(|inner| match inner {
            TfheOp::Unary(_) => {
                assert!(arguments.len() == 1);
                assert!(arguments.get(0).unwrap().is_some());

                let param1 = arguments.get(0).unwrap().as_ref().unwrap();

                eval_unary(xacml_function_enum, param1)
            }

            TfheOp::Binary(_) => {
                assert!(arguments.len() == 2);
                assert!(arguments.get(0).unwrap().is_some());
                assert!(arguments.get(1).unwrap().is_some());

                let param1 = arguments.get(0).unwrap().as_ref().unwrap();
                let param2 = arguments.get(1).unwrap().as_ref().unwrap();

                eval_binary(xacml_function_enum, param1, param2)
            }

            TfheOp::Ternary(_) => {
                assert!(arguments.len() == 3);
                assert!(arguments.get(0).unwrap().is_some());
                assert!(arguments.get(1).unwrap().is_some());
                assert!(arguments.get(2).unwrap().is_some());

                let param1 = arguments.get(0).unwrap().as_ref().unwrap();
                let param2 = arguments.get(1).unwrap().as_ref().unwrap();
                let param3 = arguments.get(2).unwrap().as_ref().unwrap();

                eval_ternary(xacml_function_enum, param1, param2, param3)
            }
        });

    result
}

/// Maps your XACML function enum -> a TFHE-backed callable op when supported.
pub fn resolve_tfhe_op(func: &XacmlFunction) -> Result<TfheOp, DispatchError> {
    use TfheOp::*;

    
    match func {
        // ----------------- Integer (Int64-ish) -----------------
        XacmlFunction::IntegerEqual
       |XacmlFunction::DateEqual => Ok(Binary(apply_integer_eq)),

        XacmlFunction::IntegerGreaterThan
        | XacmlFunction::IntegerGreaterThanOrEqual
        | XacmlFunction::IntegerLessThan
        | XacmlFunction::IntegerLessThanOrEqual
        | XacmlFunction::DateLessThan
        | XacmlFunction::DateLessThanOrEqual
        | XacmlFunction::DateGreaterThan
        | XacmlFunction::DateGreaterThanOrEqual=> Ok(Binary(apply_integer_cmp)),

        XacmlFunction::IntegerAdd
        | XacmlFunction::IntegerSubtract
        | XacmlFunction::IntegerMultiply
        | XacmlFunction::IntegerDivide => Ok(Binary(apply_integer_arith)),

        XacmlFunction::IntegerAbs => Ok(Unary(apply_integer_abs)),

        // ----------------- Boolean logic -----------------
        XacmlFunction::Not => Ok(Unary(apply_not)),
        XacmlFunction::And | XacmlFunction::Or => Ok(Binary(apply_bool_binop)),
        XacmlFunction::BooleanEqual => Ok(Binary(apply_bool_eq)),
        XacmlFunction::IfElse => Ok(Ternary(apply_bool_if_else)),

        // ----------------- "double-*" encoded as HLongINteger -----------------
        XacmlFunction::DoubleEqual => Ok(Binary(apply_double_eq)),
        XacmlFunction::DoubleGreaterThan
        | XacmlFunction::DoubleGreaterThanOrEqual
        | XacmlFunction::DoubleLessThan
        | XacmlFunction::DoubleLessThanOrEqual => Ok(Binary(apply_double_cmp)),
        XacmlFunction::DoubleAdd
        | XacmlFunction::DoubleSubtract
        | XacmlFunction::DoubleMultiply
        | XacmlFunction::DoubleDivide => Ok(Binary(apply_double_arith)),
        XacmlFunction::DoubleAbs => Ok(Unary(apply_double_abs)),

        // round/floor don't exist “as double” in your encoding (still Unsupported by default)
        XacmlFunction::Round | XacmlFunction::Floor => {
            Err(DispatchError::UnsupportedFunction { func: func.clone() })
        }

        // ----------------- Time (HUnsignedSmallInteger) -----------------
        XacmlFunction::TimeEqual => Ok(Binary(apply_time_eq)),
        XacmlFunction::TimeGreaterThan
        | XacmlFunction::TimeGreaterThanOrEqual
        | XacmlFunction::TimeLessThan
        | XacmlFunction::TimeLessThanOrEqual => Ok(Binary(apply_time_cmp)),
        XacmlFunction::TimeInRange => Ok(Ternary(apply_time_in_range)),

        // ----------------- DateTime (HNormalInteger) -----------------
        XacmlFunction::DateTimeEqual => Ok(Binary(apply_datetime_eq)),
        XacmlFunction::DateTimeGreaterThan
        | XacmlFunction::DateTimeGreaterThanOrEqual
        | XacmlFunction::DateTimeLessThan
        | XacmlFunction::DateTimeLessThanOrEqual => Ok(Binary(apply_datetime_cmp)),

        // DateTime +/- durations (all HNormalInteger in your model)
        XacmlFunction::DateTimeAddDayTimeDuration
        | XacmlFunction::DateTimeAddYearMonthDuration
        | XacmlFunction::DateTimeSubtractDayTimeDuration
        | XacmlFunction::DateTimeSubtractYearMonthDuration => Ok(Binary(apply_datetime_addsub_duration)),

        // ----------------- Durations (HNormalInteger) -----------------
        XacmlFunction::DayTimeDurationEqual => Ok(Binary(apply_daytimeduration_eq)),
        XacmlFunction::YearMonthDurationEqual => Ok(Binary(apply_yearmonthduration_eq)),

        // ----------------- AnyUri and IpAddress (encrypted strings) -----------------
        //#[cfg(feature = "strings")]
        XacmlFunction::AnyUriEqual => Ok(Binary(apply_anyuri_eq)),

        //#[cfg(feature = "strings")]
        XacmlFunction::StringEqual => Ok(Binary(apply_string_eq)),
        //#[cfg(feature = "strings")]
        XacmlFunction::StringEqualIgnoreCase => Ok(Binary(apply_string_eq_ignore_case)),
        //#[cfg(feature = "strings")]
        XacmlFunction::StringConcatenate => Ok(Binary(apply_string_concat)),
        
        //#[cfg(feature = "strings")]
        XacmlFunction::AnyUriContains => Ok(Binary(apply_anyuri_contains)),
        //#[cfg(feature = "strings")]
        XacmlFunction::AnyUriStartsWith => Ok(Binary(apply_anyuri_starts_with)),
        //#[cfg(feature = "strings")]
        XacmlFunction::AnyUriEndsWith => Ok(Binary(apply_anyuri_ends_with)),
        
        //#[cfg(feature = "strings")]
        XacmlFunction::StringContains => Ok(Binary(apply_string_contains)),
        //#[cfg(feature = "strings")]
        XacmlFunction::StringStartsWith => Ok(Binary(apply_string_starts_with)),
        //#[cfg(feature = "strings")]
        XacmlFunction::StringEndsWith => Ok(Binary(apply_string_ends_with)),
        
        // ipAddress-from-string is a parse/validation step (usually plaintext), left unsupported.
        // But ipAddress-equality can be handled as string equality if you add it to your enum list.

        // ----------------- Rfc822Name (HRfc822Name`) -----------------
        XacmlFunction::Rfc822NameEqual => Ok(Binary(apply_rfc822name_eq)),
        XacmlFunction::Rfc822NameFromString => Err(DispatchError::UnsupportedFunction { func: func.clone() }),
        XacmlFunction::StringFromRfc822Name => Err(DispatchError::UnsupportedFunction { func: func.clone() }),
        
        // ----------------- X500Name (HX500Name) -----------------
        XacmlFunction::X500NameEqual => Ok(Binary(apply_x500name_eq)),
        XacmlFunction::X500NameFromString => Err(DispatchError::UnsupportedFunction { func: func.clone() }),
        XacmlFunction::StringFromX500Name => Err(DispatchError::UnsupportedFunction { func: func.clone() }),
        
        // ----------------- Base64Binary (CpuFheUint8Array) -----------------
        XacmlFunction::Base64BinaryEqual => Ok(Binary(apply_cpu_uint_array_eq)),
        
        // ----------------- HexBinary (CpuFheUint8Array) -----------------
        XacmlFunction::HexBinaryEqual => Ok(Binary(apply_cpu_uint_array_eq)),
        
        // ----------------- Anything else -----------------
        // Date-* comparisons (not mapped; you didn't specify date encoding here)
        // HexBinaryEqual (not mapped; you didn't specify encoding; could reuse CpuFheUint8Array similarly)
        // Integer-* (if you still want them, add cases mapping to Int64 or separate Int type)
        // any-of/all-of/n-of (policy-level combinators), substring (not sure TFHE provides), casts from/to string, etc.
        XacmlFunction::AllOf(_) => Ok(Binary(apply_all_of)),
        XacmlFunction::AnyOf(_) => Ok(Binary(apply_any_of)),
        XacmlFunction::AnyOfAny(_) => Ok(Binary(apply_any_of_any)),
        XacmlFunction::AnyOfAll(_) => Ok(Binary(apply_any_of_all)),
        XacmlFunction::AllOfAny(_) => Ok(Binary(apply_all_of_any)),
        XacmlFunction::AllOfAll(_) => Ok(Binary(apply_all_of_all)),
     
        // ----------------- Bagging -----------------
        XacmlFunction::StringOneAndOnly => Ok(Unary(apply_string_one_and_only)),
        XacmlFunction::StringBagSize => Ok(Unary(apply_string_bag_size)),
        XacmlFunction::StringIsIn => Ok(Binary(apply_string_is_in)),

        XacmlFunction::StringAtLeastOneMemberOf => {
            Ok(Binary(apply_string_at_least_one_member_of))
        }
        
        _ => Err(DispatchError::UnsupportedFunction { func : func.clone() }),
    }
}

// ==================== Concrete TFHE mappings ====================

// ---- Integer (Int64-ish) ----

fn apply_integer_eq(
    func: XacmlFunction,
    a: &EncryptedTypes,
    b: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::Integer(x), EncryptedTypes::Integer(y))
        | (EncryptedTypes::Date(x), EncryptedTypes::Date(y)) => {
            // let start = Instant::now();
            let r = EncryptedTypes::Boolean(x.eq(y));
            // println!("elapsed time in binop integer is {}",start.elapsed().as_millis());
            Ok(r)
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Int64, Int64",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_integer_cmp(
    func: XacmlFunction,
    a: &EncryptedTypes,
    b: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::Integer(x), EncryptedTypes::Integer(y))
       | (EncryptedTypes::Date(x), EncryptedTypes::Date(y))=> {
            let out = match func {
                XacmlFunction::IntegerGreaterThan => x.gt(y),
                XacmlFunction::IntegerGreaterThanOrEqual => x.ge(y),
                XacmlFunction::IntegerLessThan => x.lt(y),
                XacmlFunction::IntegerLessThanOrEqual => x.le(y),

                
                XacmlFunction::DateGreaterThan => x.gt(y),
                XacmlFunction::DateGreaterThanOrEqual => x.ge(y),
                XacmlFunction::DateLessThan => x.lt(y),
                XacmlFunction::DateLessThanOrEqual => x.le(y),
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            Ok(EncryptedTypes::Boolean(out))
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Int64, Int64",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_integer_arith(
    func: XacmlFunction,
    a: &EncryptedTypes,
    b: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::Integer(x), EncryptedTypes::Integer(y)) => {
            let out = match func {
                XacmlFunction::IntegerAdd => x + y,
                XacmlFunction::IntegerSubtract => x - y,
                XacmlFunction::IntegerMultiply => x * y,
                XacmlFunction::IntegerDivide => x / y, // integer division semantics
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            Ok(EncryptedTypes::Integer(out))
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Int64, Int64",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_integer_abs(
    func: XacmlFunction,
    a: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {
    match a {
        EncryptedTypes::Integer(x) => Ok(EncryptedTypes::Integer(x.abs())),
        other => Err(DispatchError::TypeMismatch {
            func,
            expected: "Int64",
            got: other.ty(),
        }),
    }
}

// Optional if you have IntegerMod:
// fn apply_integer_mod(func: XacmlFunction, a: &EncryptedTypes, b: &EncryptedTypes)
//     -> Result<EncryptedTypes, DispatchError>
// {
//     match (a, b) {
//         (EncryptedTypes::Integer(x), EncryptedTypes::Integer(y)) => Ok(EncryptedTypes::Integer(x % y)),
//         (va, vb) => Err(DispatchError::TypeMismatch {
//             func, expected: "Int64, Int64",
//             got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
//         }),
//     }
// }


// ---- Boolean ----
fn apply_not(func: XacmlFunction, a: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match a {
        EncryptedTypes::Boolean(x) => Ok(EncryptedTypes::Boolean(!x)),
        other => Err(DispatchError::TypeMismatch { func, expected: "Bool", got: other.ty() }),
    }
}

fn apply_bool_binop(func: XacmlFunction, a: & EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::Boolean(x), EncryptedTypes::Boolean(y)) => {
            let start = Instant::now();
            let out = match func {
                XacmlFunction::And => x & y,
                XacmlFunction::Or => x | y,
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            // println!("elapsed time in binop and is {}",start.elapsed().as_millis());
            Ok(EncryptedTypes::Boolean(out))
        }
        (EncryptedTypes::Nothing, EncryptedTypes::Boolean(y)) => {
            let out = match func {
                XacmlFunction::And =>  y.clone(),
                XacmlFunction::Or => y.clone(),
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            Ok(EncryptedTypes::Boolean(out))
        }
        (EncryptedTypes::Boolean(x), EncryptedTypes::Nothing) => {
            let out = match func {
                XacmlFunction::And =>  x.clone(),
                XacmlFunction::Or => x.clone(),
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            Ok(EncryptedTypes::Boolean(out))
        }
        (EncryptedTypes::Nothing, EncryptedTypes::Nothing) => {
            // let out = match func {
            //     XacmlFunction::And =>  EncryptedTypes::Nothing,
            //     XacmlFunction::Or => EncryptedTypes::Nothing,
            //     _ => return Err(DispatchError::UnsupportedFunction { func }),
            // };
            Ok(EncryptedTypes::Nothing)
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Bool, Bool",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_bool_eq(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::Boolean(x), EncryptedTypes::Boolean(y)) => Ok(EncryptedTypes::Boolean(x.eq(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Bool, Bool",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_bool_if_else(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes, c: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b, c ) {
        (EncryptedTypes::Boolean(x), EncryptedTypes::Boolean(y), EncryptedTypes::Boolean(z)) => Ok(EncryptedTypes::Boolean(x.select(y,z))),
        (va, vb, vc) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Bool, Bool, Bool",
            got: if va.ty() == vb.ty() && vc.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

// ---- Double (HLongINteger) ----
fn apply_double_eq(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::Double(x), EncryptedTypes::Double(y)) => Ok(EncryptedTypes::Boolean(x.eq(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Double(HLongINteger), Double(HLongINteger)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_double_cmp(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::Double(x), EncryptedTypes::Double(y)) => {
            let out = match func {
                XacmlFunction::DoubleGreaterThan => x.gt(y),
                XacmlFunction::DoubleGreaterThanOrEqual => x.ge(y),
                XacmlFunction::DoubleLessThan => x.lt(y),
                XacmlFunction::DoubleLessThanOrEqual => x.le(y),
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            Ok(EncryptedTypes::Boolean(out))
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Double(HLongINteger), Double(HLongINteger)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

/// NOTE: DoubleDivide here means integer division on the encoded fixed-point representation.
/// If you're using fixed-point scaling, division semantics must be handled carefully.
fn apply_double_arith(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::Double(x), EncryptedTypes::Double(y)) => {
            let out = match func {
                XacmlFunction::DoubleAdd => x + y,
                XacmlFunction::DoubleSubtract => x - y,
                XacmlFunction::DoubleMultiply => x * y,
                XacmlFunction::DoubleDivide => x / y,
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            Ok(EncryptedTypes::Double(out))
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Double(HLongINteger), Double(HLongINteger)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_double_abs(func: XacmlFunction, a: &EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match a {
        EncryptedTypes::Double(x) => Ok(EncryptedTypes::Double(x.abs())),
        other => Err(DispatchError::TypeMismatch {
            func,
            expected: "Double(HLongINteger)",
            got: other.ty(),
        }),
    }
}

// ---- Time (HUnsignedSmallInteger) ----
fn apply_time_eq(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::Time(x), EncryptedTypes::Time(y)) => Ok(EncryptedTypes::Boolean(x.eq(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Time(HUnsignedSmallInteger), Time(HUnsignedSmallInteger)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_time_cmp(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::Time(x), EncryptedTypes::Time(y)) => {
            let out = match func {
                XacmlFunction::TimeGreaterThan => x.gt(y),
                XacmlFunction::TimeGreaterThanOrEqual => x.ge(y),
                XacmlFunction::TimeLessThan => x.lt(y),
                XacmlFunction::TimeLessThanOrEqual => x.le(y),
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            Ok(EncryptedTypes::Boolean(out))
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Time(HUnsignedSmallInteger), Time(HUnsignedSmallInteger)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

/// XACML time-in-range(time, start, end) => (start <= time) AND (time <= end)
fn apply_time_in_range(
    func: XacmlFunction,
    t: &EncryptedTypes,
    start: &EncryptedTypes,
    end: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {
    match (t, start, end) {
        (EncryptedTypes::Time(tt), EncryptedTypes::Time(s), EncryptedTypes::Time(e)) => {
            let ge_start = tt.ge(s);
            let le_end = tt.le(e);
            Ok(EncryptedTypes::Boolean(ge_start & le_end))
        }
        (vt, vs, ve) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Time(HUnsignedSmallInteger), Time(HUnsignedSmallInteger), Time(HUnsignedSmallInteger)",
            got: if vt.ty() == vs.ty() && vs.ty() == ve.ty() { vt.ty() } else { "Mixed" },
        }),
    }
}

// ---- DateTime (HNormalInteger) ----
fn apply_datetime_eq(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::DateTime(x), EncryptedTypes::DateTime(y)) => Ok(EncryptedTypes::Boolean(x.eq(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "DateTime(HNormalInteger), DateTime(HNormalInteger)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_datetime_cmp(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::DateTime(x), EncryptedTypes::DateTime(y)) => {
            let out = match func {
                XacmlFunction::DateTimeGreaterThan => x.gt(y),
                XacmlFunction::DateTimeGreaterThanOrEqual => x.ge(y),
                XacmlFunction::DateTimeLessThan => x.lt(y),
                XacmlFunction::DateTimeLessThanOrEqual => x.le(y),
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            Ok(EncryptedTypes::Boolean(out))
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "DateTime(HNormalInteger), DateTime(HNormalInteger)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

/// dateTime +/- duration (both are HNormalInteger in your encoding)
/// We accept either DayTimeDuration or YearMonthDuration as RHS depending on func.
fn apply_datetime_addsub_duration(
    func: XacmlFunction,
    dt: &EncryptedTypes,
    dur: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {
    match (dt, dur) {
        (EncryptedTypes::DateTime(x), EncryptedTypes::DayTimeDuration(d)) => {
            let out = match func {
                XacmlFunction::DateTimeAddDayTimeDuration => x + d,
                XacmlFunction::DateTimeSubtractDayTimeDuration => x - d,
                _ => return Err(DispatchError::TypeMismatch {
                    func,
                    expected: "DateTime +/− DayTimeDuration",
                    got: "Mismatched duration kind",
                }),
            };
            Ok(EncryptedTypes::DateTime(out))
        }
        (EncryptedTypes::DateTime(x), EncryptedTypes::YearMonthDuration(d)) => {
            let out = match func {
                XacmlFunction::DateTimeAddYearMonthDuration => x + d,
                XacmlFunction::DateTimeSubtractYearMonthDuration => x - d,
                _ => return Err(DispatchError::TypeMismatch {
                    func,
                    expected: "DateTime +/− YearMonthDuration",
                    got: "Mismatched duration kind",
                }),
            };
            Ok(EncryptedTypes::DateTime(out))
        }
        (vdt, vdur) => Err(DispatchError::TypeMismatch {
            func,
            expected: "DateTime(HNormalInteger), (DayTimeDuration|YearMonthDuration)(HNormalInteger)",
            got: if vdt.ty() == vdur.ty() { vdt.ty() } else { "Mixed" },
        }),
    }
}

// ---- Durations (HNormalInteger) ----
fn apply_daytimeduration_eq(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::DayTimeDuration(x), EncryptedTypes::DayTimeDuration(y)) => Ok(EncryptedTypes::Boolean(x.eq(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "DayTimeDuration(HNormalInteger), DayTimeDuration(HNormalInteger)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_yearmonthduration_eq(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::YearMonthDuration(x), EncryptedTypes::YearMonthDuration(y)) => Ok(EncryptedTypes::Boolean(x.eq(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "YearMonthDuration(HNormalInteger), YearMonthDuration(HNormalInteger)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

// ---- Strings / AnyUri / IpAddress (FheAsciiString) ----
//#[cfg(feature = "strings")]
fn apply_string_eq(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::String(x), EncryptedTypes::String(y)) => {
            let start = Instant::now();
            let eval = EncryptedTypes::Boolean(x.eq(y));
            // println!("evaluation time inside the binop function {}",start.elapsed().as_millis());

            Ok(eval)},
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "String(FheAsciiString), String(FheAsciiString)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

//#[cfg(feature = "strings")]
fn apply_string_eq_ignore_case(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::String(x), EncryptedTypes::String(y)) => Ok(EncryptedTypes::Boolean(x.eq_ignore_case(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "String(FheAsciiString), String(FheAsciiString)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

//#[cfg(feature = "strings")]
fn apply_string_concat(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::String(x), EncryptedTypes::String(y)) => Ok(EncryptedTypes::String(x.concat(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "String(FheAsciiString), String(FheAsciiString)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

//#[cfg(feature = "strings")]
fn apply_string_contains(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::String(x), EncryptedTypes::String(y)) => Ok(EncryptedTypes::Boolean(x.contains(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "String(FheAsciiString), String(FheAsciiString)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

//#[cfg(feature = "strings")]
fn apply_string_starts_with(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::String(x), EncryptedTypes::String(y)) => Ok(EncryptedTypes::Boolean(x.starts_with(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "String(FheAsciiString), String(FheAsciiString)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

//#[cfg(feature = "strings")]
fn apply_string_ends_with(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::String(x), EncryptedTypes::String(y)) => Ok(EncryptedTypes::Boolean(x.ends_with(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "String(FheAsciiString), String(FheAsciiString)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

//#[cfg(feature = "strings")]
fn apply_anyuri_eq(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::AnyUri(x), EncryptedTypes::AnyUri(y)) => Ok(EncryptedTypes::Boolean(x.eq(y))),
        // allow AnyUri to compare with plain String too, if you want:
        (EncryptedTypes::AnyUri(x), EncryptedTypes::String(y)) | (EncryptedTypes::String(y), EncryptedTypes::AnyUri(x)) => {
            Ok(EncryptedTypes::Boolean(x.eq(y)))
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "AnyUri(FheAsciiString), AnyUri(FheAsciiString)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

//#[cfg(feature = "strings")]
fn apply_anyuri_contains(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::AnyUri(x), EncryptedTypes::AnyUri(y)) => Ok(EncryptedTypes::Boolean(x.contains(y))),
        (EncryptedTypes::AnyUri(x), EncryptedTypes::String(y)) => Ok(EncryptedTypes::Boolean(x.contains(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "AnyUri(FheAsciiString), (AnyUri|String)(FheAsciiString)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

//#[cfg(feature = "strings")]
fn apply_anyuri_starts_with(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::AnyUri(x), EncryptedTypes::AnyUri(y)) => Ok(EncryptedTypes::Boolean(x.starts_with(y))),
        (EncryptedTypes::AnyUri(x), EncryptedTypes::String(y)) => Ok(EncryptedTypes::Boolean(x.starts_with(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "AnyUri(FheAsciiString), (AnyUri|String)(FheAsciiString)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

//#[cfg(feature = "strings")]
fn apply_anyuri_ends_with(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::AnyUri(x), EncryptedTypes::AnyUri(y)) => Ok(EncryptedTypes::Boolean(x.ends_with(y))),
        (EncryptedTypes::AnyUri(x), EncryptedTypes::String(y)) => Ok(EncryptedTypes::Boolean(x.ends_with(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "AnyUri(FheAsciiString), (AnyUri|String)(FheAsciiString)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

// ---- Rfc822Name (HRfc822Name) ----
fn apply_rfc822name_eq(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
	match (a, b) {
        (EncryptedTypes::Rfc822Name(x), EncryptedTypes::Rfc822Name(y)) => {
       		let local_part_eq_res = &x.localpart.eq(&y.localpart);
        	let domain_part_eq_res = &x.domainpart.eq(&y.domainpart);
         	let and_res = local_part_eq_res & domain_part_eq_res;
        	Ok(EncryptedTypes::Boolean(and_res))
        },
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "AnyUri(FheAsciiString), (AnyUri|String)(FheAsciiString)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

// ---- X500Name (HX500Name) ----
fn apply_x500name_eq(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
	match (a, b) {
		(EncryptedTypes::X500Name(x), EncryptedTypes::X500Name(y)) => {
			// calculate all FheBools
			let kx = &mut x.val.keys(); 
			let ky = &mut y.val.keys();
			if (kx.len() != ky.len()) {
				return Err(DispatchError::X500NameNotAllFieldsSupplied { message: "the set of keys is not equal" }); 
			}
			
			if ( !kx.all(|k| y.val.contains_key(k))) {
				return Err(DispatchError::X500NameNotAllFieldsSupplied { message: "one of the keys is not found in the set of other keys" }); 
			}
			let mut x_iter = x.val.iter();
			//	start by the first one
			let mut final_result = x_iter
				.next()
				.and_then(|(k1,v1)|
					y.val.get(k1).and_then(|v2|
						Some(v2.eq(v1))
					)
				).ok_or_else(|| DispatchError::X500NameNotAllFieldsSupplied { message:"fatal error: could not find key in X500Name type variable, mismatched keys"})?;
				
			for (k,v) in x_iter {
				final_result &= y.val.get(k).and_then(|v2|
					Some(v2.eq(v))
				).ok_or_else(|| DispatchError::X500NameNotAllFieldsSupplied { message:"fatal error: could not find key in X500Name type variable, mismatched keys"})?;
			}
			Ok(EncryptedTypes::Boolean(final_result))
		},
		(va, vb) => Err(DispatchError::TypeMismatch {
			func,
			expected: "AnyUri(FheAsciiString), (AnyUri|String)(FheAsciiString)",
			got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
		}),
	}
}

// ---- Base64Binary (CpuFheUint8Array) / HexBinary (CpuFheUint8Array) ----.
fn apply_cpu_uint_array_eq(func: XacmlFunction, a: &EncryptedTypes, b: & EncryptedTypes) -> Result<EncryptedTypes, DispatchError> {
    match (a, b) {
        (EncryptedTypes::Base64Binary(x), EncryptedTypes::Base64Binary(y)) =>{ 
            let result = fhe_base_radix_ciphertext_array_eq(x.container(), y.container());
            Ok(EncryptedTypes::Boolean(result))
        },
        (EncryptedTypes::HexBinary(x), EncryptedTypes::HexBinary(y)) =>{ 
            let result = fhe_base_radix_ciphertext_array_eq(x.container(), y.container());
            Ok(EncryptedTypes::Boolean(result))
        },
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Base64Binary(CpuFheUint8Array), Base64Binary(CpuFheUint8Array)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

/// Replace this with however your project creates an encrypted/trivial boolean.
/// For tfhe-rs projects this is often some form of trivial encryption or a project-local helper.
fn encrypt_bool_constant(_value: bool) -> HBoolean {
    todo!("wire this to your HBoolean/FheBool trivial-encryption helper")
}

fn expect_boolean(func: XacmlFunction, value: EncryptedTypes) -> Result<HBoolean, DispatchError> {
    match value {
        EncryptedTypes::Boolean(b) => Ok(b),
        other => Err(DispatchError::TypeMismatch {
            func,
            expected: "Bool",
            got: other.ty(),
        }),
    }
}

fn eval_binary_predicate_bool(
    pred: XacmlFunction,
    a: &EncryptedTypes,
    b: &EncryptedTypes,
) -> Result<HBoolean, DispatchError> {
    let out = eval_binary(pred.clone(), a, b)?;
    expect_boolean(pred, out)
}

/// XACML any-of(predicate, value, bag)
/// Common binary-predicate form:
///   OR_{e in bag} predicate(value, e)
pub fn apply_any_of(
    func: XacmlFunction,
    value: &EncryptedTypes,
    bag: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {
	match (bag, func.clone()) {
		(EncryptedTypes::Bag(b), XacmlFunction::AnyOf(inner_pred)) => {
			match inner_pred {
				Some(inner_inner_pred)=> {
					let mut acc: Option<HBoolean> = None;

				    for elem in b {
				        let r = eval_binary_predicate_bool(*inner_inner_pred.clone(), value, elem)?;
				        acc = Some(match acc {
				            Some(prev) => prev | r,
				            None => r,
				        });
				    }
				
				    Ok(EncryptedTypes::Boolean(
				        acc.unwrap_or_else(|| encrypt_bool_constant(false))
				    ))
				}
				None => {
					Err(
						DispatchError::AbsentInnerFunction { message: "no inner function found in all-of function evaluation" }
					)
				}
			}
		}
		(va, pred) => Err(DispatchError::TypeMismatch {
	            func : pred,
	            expected: "Bag",
	            got: va.ty(),
	        }
		)
    }
    
}

/// XACML all-of(predicate, value, bag)
/// Common binary-predicate form:
///   AND_{e in bag} predicate(value, e)
pub fn apply_all_of(
    func: XacmlFunction,
    value: &EncryptedTypes,
    bag: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {
	match (bag, func.clone()) {
		(EncryptedTypes::Bag(b), XacmlFunction::AllOf(inner_pred)) => {
			match inner_pred {
				Some(inner_inner_pred)=> {
					let mut acc: Option<HBoolean> = None;

				    for elem in b {
				        let r = eval_binary_predicate_bool(*inner_inner_pred.clone(), value, elem)?;
				        acc = Some(match acc {
				            Some(prev) => prev & r,
				            None => r,
				        });
				    }
				
				    Ok(EncryptedTypes::Boolean(
				        acc.unwrap_or_else(|| encrypt_bool_constant(true))
				    ))
				}
				None => {
					Err(
						DispatchError::AbsentInnerFunction { message: "no inner function found in all-of function evaluation" }
					)
				}
			}
		}
		(va, pred) => Err(DispatchError::TypeMismatch {
	            func : pred,
	            expected: "Bag",
	            got: va.ty(),
	        }
		)
    }
	
}

/// XACML any-of-any(predicate, bag1, bag2)
///   OR_{x in bag1, y in bag2} predicate(x, y)
pub fn apply_any_of_any(
    func: XacmlFunction,
    bag1: &EncryptedTypes,
    bag2: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {

	match (bag1, bag2, func.clone()) {
		(EncryptedTypes::Bag(b1), EncryptedTypes::Bag(b2), XacmlFunction::AnyOfAny(inner_pred)) => {
			match inner_pred {
				Some(inner_inner_pred)=> {
					let mut acc: Option<HBoolean> = None;

	    			for x in b1 {
				        for y in b2 {
				            let r = eval_binary_predicate_bool(*inner_inner_pred.clone(), x, y)?;
				            acc = Some(match acc {
				                Some(prev) => prev | r,
				                None => r,
				            });
				        }
				    }
				
				    Ok(EncryptedTypes::Boolean(
				        acc.unwrap_or_else(|| encrypt_bool_constant(false))
				    ))
				}
				None => {
					Err(
						DispatchError::AbsentInnerFunction { message: "no inner function found in all-of-all function evaluation" }
					)
				}
			}
		}
		(va, vb, pred) => Err(DispatchError::TypeMismatch {
	            func : pred,
	            expected: "Bag, Bag",
	            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
	        }
		)
    }
}

/// XACML all-of-any(predicate, bag1, bag2)
///   AND_{x in bag1} OR_{y in bag2} predicate(x, y)
pub fn apply_all_of_any(
    func: XacmlFunction,
    bag1: &EncryptedTypes,
    bag2: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {
	match (bag1, bag2, func.clone()) {
		(EncryptedTypes::Bag(b1), EncryptedTypes::Bag(b2), XacmlFunction::AllOfAny(inner_pred)) => {
			match inner_pred {
				Some(inner_inner_pred)=> {
					let mut outer_acc: Option<HBoolean> = None;

				    for x in b1 {
				        let mut inner_acc: Option<HBoolean> = None;
				
				        for y in b2 {
				            let r = eval_binary_predicate_bool(*inner_inner_pred.clone(), x, y)?;
				            inner_acc = Some(match inner_acc {
				                Some(prev) => prev | r,
				                None => r,
				            });
				        }
				
				        let inner = inner_acc.unwrap_or_else(|| encrypt_bool_constant(false));
				
				        outer_acc = Some(match outer_acc {
				            Some(prev) => prev & inner,
				            None => inner,
				        });
				    }
				
				    Ok(EncryptedTypes::Boolean(
				        outer_acc.unwrap_or_else(|| encrypt_bool_constant(true))
				    ))
				}
				None => {
					Err(
						DispatchError::AbsentInnerFunction { message: "no inner function found in all-of-any function evaluation" }
					)
				}
			}
		}
		(va, vb, pred) => Err(DispatchError::TypeMismatch {
	            func : pred,
	            expected: "Bag, Bag",
	            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
	        }
		)
    }
}

/// XACML any-of-all(predicate, bag1, bag2)
/// Per spec:
///   AND_{y in bag2} OR_{x in bag1} predicate(x, y)
pub fn apply_any_of_all(
    func: XacmlFunction,
    bag1: &EncryptedTypes,
    bag2: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {
	match (bag1, bag2, func.clone()) {
		(EncryptedTypes::Bag(b1), EncryptedTypes::Bag(b2), XacmlFunction::AnyOfAll(inner_pred)) => {
			match inner_pred {
				Some(inner_inner_pred)=> {
					let mut outer_acc: Option<HBoolean> = None;
	
				    for y in b2 {
				        let mut inner_acc: Option<HBoolean> = None;
				
				        for x in b1 {
				            let r = eval_binary_predicate_bool(*inner_inner_pred.clone(), x, y)?;
				            inner_acc = Some(match inner_acc {
				                Some(prev) => prev | r,
				                None => r,
				            });
				        }
				
				        let inner = inner_acc.unwrap_or_else(|| encrypt_bool_constant(false));
				
				        outer_acc = Some(match outer_acc {
				            Some(prev) => prev & inner,
				            None => inner,
				        });
				    }
				
				    Ok(EncryptedTypes::Boolean(
				        outer_acc.unwrap_or_else(|| encrypt_bool_constant(true))
				    ))
				}
				None => {
					Err(
						DispatchError::AbsentInnerFunction { message: "no inner function found in any-of-all function evaluation" }
					)
				}
			}
		}
		(va, vb, pred) => Err(DispatchError::TypeMismatch {
	            func : pred,
	            expected: "Bag, Bag",
	            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
	        }
		)
    }
}

/// XACML all-of-all(predicate, bag1, bag2)
///   AND_{x in bag1, y in bag2} predicate(x, y)
pub fn apply_all_of_all(
    func: XacmlFunction,
    bag1: &EncryptedTypes,
    bag2: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {
    match (bag1, bag2, func.clone()) {
		(EncryptedTypes::Bag(b1), EncryptedTypes::Bag(b2), XacmlFunction::AllOfAll(inner_pred)) => {
			match inner_pred {
				Some(inner_inner_pred)=> {
					let mut acc: Option<HBoolean> = None;
					for a in b1 {
						for b in b2 {
							let r = eval_binary_predicate_bool(*inner_inner_pred.clone(), a, b)?;
							acc = Some(match acc {
								Some(prev) => prev & r,
								None => r,
							});
						}
					}
					Ok(EncryptedTypes::Boolean(
						acc.unwrap_or_else(|| encrypt_bool_constant(true))
					))
				}
				None => {
					Err(
						DispatchError::AbsentInnerFunction { message: "no inner function found in all-of-all function evaluation" }
					)
				}
			}
		}
		(va, vb, pred) => Err(DispatchError::TypeMismatch {
            func : pred,
            expected: "Bag, Bag",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        })
    }
}

pub fn apply_string_one_and_only(
    func: XacmlFunction,
    bag: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {
    match bag {
    
	    EncryptedTypes::String(value) => {
	    	Ok(EncryptedTypes::String(value.clone()))
	    },
        EncryptedTypes::Bag(values) => {
            if values.len() != 1 {
                return Err(DispatchError::Indeterminate {
                    func,
                    reason: "string-one-and-only requires exactly one value in the bag",
                });
            }

            match &values[0] {
                EncryptedTypes::String(_) => Ok(values[0].clone()),

                other => Err(DispatchError::TypeMismatch {
                    func,
                    expected: "String(FheAsciiString)",
                    got: other.ty(),
                }),
            }
        },
        other => {
        	println!("so the problem is here");
        	Err(DispatchError::TypeMismatch {
	            func,
	            expected: "Bag",
	            got: other.ty(),
	        })
        },
    }
}

pub fn apply_string_bag_size(
    func: XacmlFunction,
    bag: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {
    match bag {
        EncryptedTypes::Bag(values) => {
            for value in values {
                if !matches!(value, EncryptedTypes::String(_)) {
                    return Err(DispatchError::TypeMismatch {
                        func,
                        expected: "Bag(String)",
                        got: value.ty(),
                    });
                }
            }

            let size = i64::try_from(values.len()).map_err(|_| {
                DispatchError::Indeterminate {
                    func: func.clone(),
                    reason: "bag size does not fit into i64",
                }
            })?;

           	//TODO FIX
            Ok(EncryptedTypes::Integer(HNormalInteger::encrypt_trivial(size)))
        }

        other => Err(DispatchError::TypeMismatch {
            func,
            expected: "Bag",
            got: other.ty(),
        }),
    }
}

pub fn apply_string_is_in(
    func: XacmlFunction,
    value: &EncryptedTypes,
    bag: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {
    let value = match value {
        EncryptedTypes::String(s) => s,
        other => {
            return Err(DispatchError::TypeMismatch {
                func,
                expected: "String(FheAsciiString)",
                got: other.ty(),
            });
        }
    };

    match bag {
        EncryptedTypes::Bag(values) => {
            let mut acc: Option<HBoolean> = None;

            for elem in values {
                let elem = match elem {
                    EncryptedTypes::String(s) => s,
                    other => {
                        return Err(DispatchError::TypeMismatch {
                            func,
                            expected: "Bag(String)",
                            got: other.ty(),
                        });
                    }
                };

                let eq = value.eq(elem);

                acc = Some(match acc {
                    Some(prev) => prev | eq,
                    None => eq,
                });
            }

            Ok(EncryptedTypes::Boolean(
                acc.unwrap_or_else(|| encrypt_bool_constant(false)),
            ))
        }

        other => Err(DispatchError::TypeMismatch {
            func,
            expected: "Bag",
            got: other.ty(),
        }),
    }
}

pub fn apply_string_bag_from_owned_args(
    func: XacmlFunction,
    args: Vec<Option<EncryptedTypes>>,
) -> Result<EncryptedTypes, DispatchError> {
    let mut values = Vec::with_capacity(args.len());

    for arg in args {
        let value = arg.ok_or(DispatchError::Indeterminate {
            func: func.clone(),
            reason: "missing argument in string-bag",
        })?;

        match value {
            EncryptedTypes::String(_) => values.push(value),

            other => {
                return Err(DispatchError::TypeMismatch {
                    func,
                    expected: "String(FheAsciiString)",
                    got: other.ty(),
                });
            }
        }
    }

    Ok(EncryptedTypes::Bag(values))
}

pub fn apply_string_bag_from_borrowed_args(
    func: XacmlFunction,
    args: Vec<Option<&EncryptedTypes>>,
) -> Result<EncryptedTypes, DispatchError> {
    let mut values = Vec::with_capacity(args.len());

    for arg in args {
        let value = arg.ok_or(DispatchError::Indeterminate {
            func: func.clone(),
            reason: "missing argument in string-bag",
        })?;

        match value {
            EncryptedTypes::String(_) => values.push(value.clone()),

            other => {
                return Err(DispatchError::TypeMismatch {
                    func,
                    expected: "String(FheAsciiString)",
                    got: other.ty(),
                });
            }
        }
    }

    Ok(EncryptedTypes::Bag(values))
}

pub fn apply_string_at_least_one_member_of(
    func: XacmlFunction,
    left: &EncryptedTypes,
    right: &EncryptedTypes,
) -> Result<EncryptedTypes, DispatchError> {
    let left_values = expect_encrypted_string_values(func.clone(), left)?;
    let right_values = expect_encrypted_string_values(func.clone(), right)?;

    let mut acc: Option<HBoolean> = None;

    for s1 in left_values {
        for s2 in &right_values {
            let eq = s1.eq(*s2);

            acc = Some(match acc {
                Some(prev) => prev | eq,
                None => eq,
            });
        }
    }

    Ok(EncryptedTypes::Boolean(
        acc.unwrap_or_else(|| encrypt_bool_constant(false)),
    ))
}

fn expect_encrypted_string_values<'a>(
    func: XacmlFunction,
    value: &'a EncryptedTypes,
) -> Result<Vec<&'a FheAsciiString>, DispatchError> {
    match value {
        EncryptedTypes::String(s) => Ok(vec![s]),

        EncryptedTypes::Bag(values) => {
            let mut out = Vec::with_capacity(values.len());

            for elem in values {
                match elem {
                    EncryptedTypes::String(s) => out.push(s),

                    other => {
                        return Err(DispatchError::TypeMismatch {
                            func,
                            expected: "String(FheAsciiString) or Bag(String)",
                            got: other.ty(),
                        });
                    }
                }
            }

            Ok(out)
        }

        other => Err(DispatchError::TypeMismatch {
            func,
            expected: "String(FheAsciiString) or Bag(String)",
            got: other.ty(),
        }),
    }
}

fn reduce_balanced<T, F>(
    mut values: Vec<T>,
    mut op: F,
) -> Option<T>
where
    F: FnMut(T, T) -> T,
{
    if values.is_empty() {
        return None;
    }

    while values.len() > 1 {
        let mut next_level = Vec::with_capacity((values.len() + 1) / 2);
        let mut iter = values.into_iter();

        while let Some(left) = iter.next() {
            match iter.next() {
                Some(right) => {
                    next_level.push(op(left, right));
                }
                None => {
                    // Odd number of nodes: promote the last one unchanged.
                    next_level.push(left);
                }
            }
        }

        values = next_level;
    }

    values.pop()
}

pub fn apply_bool_vec_op_from_borrowed_args(
    func: XacmlFunction,
    args: Vec<Option<&EncryptedTypes>>,
) -> Result<EncryptedTypes, DispatchError> {
    if args.is_empty() {
        return Err(DispatchError::ArityMismatch {
            func,
            expected: 1,
            got: 0,
        });
    }

    let mut values: Vec<HBoolean> = Vec::with_capacity(args.len());

    for arg in args {
        let value = arg.ok_or(DispatchError::Indeterminate {
            func: func.clone(),
            reason: "missing argument in boolean vector operation",
        })?;

        match value {
            EncryptedTypes::Boolean(b) => {
                values.push(b.clone());
            }

            // Optional compatibility with your Nothing accumulator style.
            // If you do not want this behavior, remove this arm.
            EncryptedTypes::Nothing => {}

            other => {
                return Err(DispatchError::TypeMismatch {
                    func,
                    expected: "Bool",
                    got: other.ty(),
                });
            }
        }
    }

    if values.is_empty() {
        return Ok(EncryptedTypes::Nothing);
    }

    let result = match func {
        XacmlFunction::And => {
            reduce_balanced(values, |a, b| a & b)
        }

        XacmlFunction::Or => {
            reduce_balanced(values, |a, b| a | b)
        }

        _ => {
            return Err(DispatchError::UnsupportedFunction { func });
        }
    }
    .ok_or(DispatchError::Indeterminate {
        func: func.clone(),
        reason: "failed to reduce boolean vector operation",
    })?;

    Ok(EncryptedTypes::Boolean(result))
}

pub fn apply_bool_vec_op_from_owned_args(
    func: XacmlFunction,
    args: Vec<Option<EncryptedTypes>>,
) -> Result<EncryptedTypes, DispatchError> {
    if args.is_empty() {
        return Err(DispatchError::ArityMismatch {
            func,
            expected: 1,
            got: 0,
        });
    }

    let mut values: Vec<HBoolean> = Vec::with_capacity(args.len());

    for arg in args {
        let value = arg.ok_or(DispatchError::Indeterminate {
            func: func.clone(),
            reason: "missing argument in boolean vector operation",
        })?;

        match value {
            EncryptedTypes::Boolean(b) => {
                values.push(b);
            }

            // Optional compatibility with your Nothing accumulator style.
            EncryptedTypes::Nothing => {}

            other => {
                return Err(DispatchError::TypeMismatch {
                    func,
                    expected: "Bool",
                    got: other.ty(),
                });
            }
        }
    }

    if values.is_empty() {
        return Ok(EncryptedTypes::Nothing);
    }

    let result = match func {
        XacmlFunction::And => {
            reduce_balanced(values, |a, b| a & b)
        }

        XacmlFunction::Or => {
            reduce_balanced(values, |a, b| a | b)
        }

        _ => {
            return Err(DispatchError::UnsupportedFunction { func });
        }
    }
    .ok_or(DispatchError::Indeterminate {
        func: func.clone(),
        reason: "failed to reduce boolean vector operation",
    })?;

    Ok(EncryptedTypes::Boolean(result))
}