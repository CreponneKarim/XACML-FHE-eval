/// plain function evaluation for the intermediate types here


use std::collections::HashMap;
use xsd_parser::xml::{AnyAttributes, AnyElement, Mixed};
use crate::{intermediate::IntermediateTypes, xacml::constants::{Boolean, Double, NormalInteger, SmallInteger, USmallInteger, XacmlFunction}};

// Reuse your XacmlFunction + DispatchError (same as encrypted part)
#[derive(Debug, Clone)]
pub enum DispatchError {
    UnsupportedFunction { func: XacmlFunction },
    ArityMismatch { func: XacmlFunction, expected: usize, got: usize },
    TypeMismatch { func: XacmlFunction, expected: &'static str, got: &'static str },
    NotFoundOperatorFromUrn { message: &'static str, not_found: String },
    X500NameNotAllFieldsSupplied { message: &'static str },
    AbsentInnerFunction { message: &'static str },

    Indeterminate {
        func: XacmlFunction,
        reason: &'static str,
    },
}

impl IntermediateTypes {
    pub fn ty(&self) -> &'static str {
        match self {
            IntermediateTypes::Boolean(_) => "Bool",
            IntermediateTypes::Integer(_) => "Int64/NormalInteger",
            IntermediateTypes::Double(_) => "Double",
            IntermediateTypes::Time(_) => "Time/USmallInteger",
            IntermediateTypes::Date(_) => "DateTime/NormalInteger",
            IntermediateTypes::DateTime(_) => "DateTime/NormalInteger",
            IntermediateTypes::DayTimeDuration(_) => "DayTimeDuration/NormalInteger",
            IntermediateTypes::YearMonthDuration(_) => "YearMonthDuration/NormalInteger",
            IntermediateTypes::String(_) => "String",
            IntermediateTypes::AnyUri(_) => "AnyUri(String)",
            IntermediateTypes::HexBinary(_) => "HexBinary(Vec<u8>)",
            IntermediateTypes::Base64Binary(_) => "Base64Binary(Vec<u8>)",
            IntermediateTypes::Rfc822Name(_, _) => "Rfc822Name(String,String)",
            IntermediateTypes::X500Name(_) => "X500Name(HashMap<String,String>)",
            IntermediateTypes::DnsName(_, _, _) => "DnsName(String,USmallInteger,USmallInteger)",
            IntermediateTypes::IpAddress(_) => "IpAddress(String)",
            IntermediateTypes::Bag(_) => "Bag(IntermediateTypes)",
            IntermediateTypes::Nothing => "Nothing"
        }
    }
}

/// Plain ops: same idea as TfheOp but for IntermediateTypes
pub enum PlainOp {
    Unary(fn(XacmlFunction, &IntermediateTypes) -> Result<IntermediateTypes, DispatchError>),
    Binary(fn(XacmlFunction, &IntermediateTypes, &IntermediateTypes) -> Result<IntermediateTypes, DispatchError>),
    Ternary(fn(XacmlFunction, &IntermediateTypes, &IntermediateTypes, &IntermediateTypes) -> Result<IntermediateTypes, DispatchError>),
}

pub fn eval_plain_unary(func: XacmlFunction, a: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match resolve_plain_op(&func)? {
        PlainOp::Unary(f) => f(func, a),
        _ => Err(DispatchError::ArityMismatch { func, expected: 1, got: 1 }),
    }
}

pub fn eval_plain_binary(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match resolve_plain_op(&func)? {
        PlainOp::Binary(f) => f(func, a, b),
        _ => Err(DispatchError::ArityMismatch { func, expected: 2, got: 2 }),
    }
}

pub fn eval_plain_ternary(
    func: XacmlFunction,
    a: &IntermediateTypes,
    b: &IntermediateTypes,
    c: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    match resolve_plain_op(&func)? {
        PlainOp::Ternary(f) => f(func, a, b, c),
        _ => Err(DispatchError::ArityMismatch { func, expected: 3, got: 3 }),
    }
}

/// Optional helper mirroring your encrypted evaluate_function(...)
pub fn evaluate_function_plain(
    urn: &str,
    urn2: Option<&str>,
    arguments: Vec<Option<&IntermediateTypes>>,
) -> Result<IntermediateTypes, DispatchError> {
    let func = XacmlFunction::from_urn(urn, urn2).ok_or(
        DispatchError::NotFoundOperatorFromUrn {
            message: "Did not find the operator from the URN ",
            not_found: urn.to_string(),
        }
    )?;

    // string-bag is variadic, so it must not go through Unary/Binary/Ternary.
    if matches!(func, XacmlFunction::StringBag) {
        return apply_plain_string_bag_from_borrowed_args(func, arguments);
    }

    if matches!(func, XacmlFunction::And | XacmlFunction::Or) {
        return apply_plain_bool_vec_op_from_borrowed_args(func, arguments);
    }

    match resolve_plain_op(&func)? {
        PlainOp::Unary(_) => {
            if arguments.len() != 1 {
                return Err(DispatchError::ArityMismatch {
                    func,
                    expected: 1,
                    got: arguments.len(),
                });
            }

            let a = arguments[0].ok_or(DispatchError::Indeterminate {
                func: func.clone(),
                reason: "missing unary argument",
            })?;

            eval_plain_unary(func, a)
        }

        PlainOp::Binary(_) => {
            if arguments.len() != 2 {
                return Err(DispatchError::ArityMismatch {
                    func,
                    expected: 2,
                    got: arguments.len(),
                });
            }

            let a = arguments[0].ok_or(DispatchError::Indeterminate {
                func: func.clone(),
                reason: "missing first binary argument",
            })?;

            let b = arguments[1].ok_or(DispatchError::Indeterminate {
                func: func.clone(),
                reason: "missing second binary argument",
            })?;

            eval_plain_binary(func, a, b)
        }

        PlainOp::Ternary(_) => {
            if arguments.len() != 3 {
                return Err(DispatchError::ArityMismatch {
                    func,
                    expected: 3,
                    got: arguments.len(),
                });
            }

            let a = arguments[0].ok_or(DispatchError::Indeterminate {
                func: func.clone(),
                reason: "missing first ternary argument",
            })?;

            let b = arguments[1].ok_or(DispatchError::Indeterminate {
                func: func.clone(),
                reason: "missing second ternary argument",
            })?;

            let c = arguments[2].ok_or(DispatchError::Indeterminate {
                func: func.clone(),
                reason: "missing third ternary argument",
            })?;

            eval_plain_ternary(func, a, b, c)
        }
    }
}

/// Same but consuming args (mirrors your consume variant)
pub fn evaluate_function_plain_consume(
    urn: &str,
    urn2: Option<&str>,
    arguments: Vec<Option<IntermediateTypes>>,
) -> Result<IntermediateTypes, DispatchError> {
    let func = XacmlFunction::from_urn(urn, urn2).ok_or(
        DispatchError::NotFoundOperatorFromUrn {
            message: "Did not find the operator from the URN ",
            not_found: urn.to_string(),
        }
    )?;

    // string-bag is variadic, so it must not go through Unary/Binary/Ternary.
    if matches!(func, XacmlFunction::StringBag) {
        return apply_plain_string_bag_from_owned_args(func, arguments);
    }
    if matches!(func, XacmlFunction::And | XacmlFunction::Or) {
        return apply_plain_bool_vec_op_from_owned_args(func, arguments);
    }

    match resolve_plain_op(&func)? {
        PlainOp::Unary(_) => {
            if arguments.len() != 1 {
                return Err(DispatchError::ArityMismatch {
                    func,
                    expected: 1,
                    got: arguments.len(),
                });
            }

            let a = arguments[0].as_ref().ok_or(DispatchError::Indeterminate {
                func: func.clone(),
                reason: "missing unary argument",
            })?;

            eval_plain_unary(func, a)
        }

        PlainOp::Binary(_) => {
            if arguments.len() != 2 {
                return Err(DispatchError::ArityMismatch {
                    func,
                    expected: 2,
                    got: arguments.len(),
                });
            }

            let a = arguments[0].as_ref().ok_or(DispatchError::Indeterminate {
                func: func.clone(),
                reason: "missing first binary argument",
            })?;

            let b = arguments[1].as_ref().ok_or(DispatchError::Indeterminate {
                func: func.clone(),
                reason: "missing second binary argument",
            })?;

            eval_plain_binary(func, a, b)
        }

        PlainOp::Ternary(_) => {
            if arguments.len() != 3 {
                return Err(DispatchError::ArityMismatch {
                    func,
                    expected: 3,
                    got: arguments.len(),
                });
            }

            let a = arguments[0].as_ref().ok_or(DispatchError::Indeterminate {
                func: func.clone(),
                reason: "missing first ternary argument",
            })?;

            let b = arguments[1].as_ref().ok_or(DispatchError::Indeterminate {
                func: func.clone(),
                reason: "missing second ternary argument",
            })?;

            let c = arguments[2].as_ref().ok_or(DispatchError::Indeterminate {
                func: func.clone(),
                reason: "missing third ternary argument",
            })?;

            eval_plain_ternary(func, a, b, c)
        }
    }
}

/// Map the same supported XacmlFunction variants -> plaintext implementations
pub fn resolve_plain_op(func: &XacmlFunction) -> Result<PlainOp, DispatchError> {
    use PlainOp::*;
    let cloned = func.clone();
    match cloned {
        // Integer ops
        XacmlFunction::IntegerEqual
        | XacmlFunction::DateEqual => Ok(Binary(apply_plain_integer_eq)),
        XacmlFunction::IntegerGreaterThan
        | XacmlFunction::IntegerGreaterThanOrEqual
        | XacmlFunction::IntegerLessThan
        | XacmlFunction::DateLessThanOrEqual
        | XacmlFunction::DateLessThan
        | XacmlFunction::DateLessThanOrEqual
        | XacmlFunction::DateGreaterThan
        | XacmlFunction::DateGreaterThanOrEqual => Ok(Binary(apply_plain_integer_cmp)),

        XacmlFunction::IntegerAdd
        | XacmlFunction::IntegerSubtract
        | XacmlFunction::IntegerMultiply
        | XacmlFunction::IntegerDivide
        | XacmlFunction::IntegerMod => Ok(Binary(apply_plain_integer_arith)),

        XacmlFunction::IntegerAbs => Ok(Unary(apply_plain_integer_abs)),
        
        // Boolean logic
        XacmlFunction::Not => Ok(Unary(apply_plain_not)),
        XacmlFunction::And | XacmlFunction::Or => Ok(Binary(apply_plain_bool_binop)),
        XacmlFunction::BooleanEqual => Ok(Binary(apply_plain_bool_eq)),
        XacmlFunction::IfElse => Ok(Ternary(apply_plain_bool_if_else)),

        // Double ops
        XacmlFunction::DoubleEqual => Ok(Binary(apply_plain_double_eq)),
        XacmlFunction::DoubleGreaterThan
        | XacmlFunction::DoubleGreaterThanOrEqual
        | XacmlFunction::DoubleLessThan
        | XacmlFunction::DoubleLessThanOrEqual => Ok(Binary(apply_plain_double_cmp)),
        XacmlFunction::DoubleAdd
        | XacmlFunction::DoubleSubtract
        | XacmlFunction::DoubleMultiply
        | XacmlFunction::DoubleDivide => Ok(Binary(apply_plain_double_arith)),
        XacmlFunction::DoubleAbs => Ok(Unary(apply_plain_double_abs)),

        // Time ops
        XacmlFunction::TimeEqual => Ok(Binary(apply_plain_time_eq)),
        XacmlFunction::TimeGreaterThan
        | XacmlFunction::TimeGreaterThanOrEqual
        | XacmlFunction::TimeLessThan
        | XacmlFunction::TimeLessThanOrEqual => Ok(Binary(apply_plain_time_cmp)),
        XacmlFunction::TimeInRange => Ok(Ternary(apply_plain_time_in_range)),

        // DateTime ops
        XacmlFunction::DateTimeEqual => Ok(Binary(apply_plain_datetime_eq)),
        XacmlFunction::DateTimeGreaterThan
        | XacmlFunction::DateTimeGreaterThanOrEqual
        | XacmlFunction::DateTimeLessThan
        | XacmlFunction::DateTimeLessThanOrEqual => Ok(Binary(apply_plain_datetime_cmp)),

        XacmlFunction::DateTimeAddDayTimeDuration
        | XacmlFunction::DateTimeAddYearMonthDuration
        | XacmlFunction::DateTimeSubtractDayTimeDuration
        | XacmlFunction::DateTimeSubtractYearMonthDuration => Ok(Binary(apply_plain_datetime_addsub_duration)),

        // Duration equality
        XacmlFunction::DayTimeDurationEqual => Ok(Binary(apply_plain_daytimeduration_eq)),
        XacmlFunction::YearMonthDurationEqual => Ok(Binary(apply_plain_yearmonthduration_eq)),

        // Strings / AnyUri
        XacmlFunction::AnyUriEqual => Ok(Binary(apply_plain_anyuri_eq)),
        XacmlFunction::StringEqual => Ok(Binary(apply_plain_string_eq)),
        XacmlFunction::StringEqualIgnoreCase => Ok(Binary(apply_plain_string_eq_ignore_case)),
        XacmlFunction::StringConcatenate => Ok(Binary(apply_plain_string_concat)),
        XacmlFunction::StringContains => Ok(Binary(apply_plain_string_contains)),
        XacmlFunction::StringStartsWith => Ok(Binary(apply_plain_string_starts_with)),
        XacmlFunction::StringEndsWith => Ok(Binary(apply_plain_string_ends_with)),
        XacmlFunction::AnyUriContains => Ok(Binary(apply_plain_anyuri_contains)),
        XacmlFunction::AnyUriStartsWith => Ok(Binary(apply_plain_anyuri_starts_with)),
        XacmlFunction::AnyUriEndsWith => Ok(Binary(apply_plain_anyuri_ends_with)),
        
        // ----------------- Rfc822Name (HRfc822Name`) -----------------
        XacmlFunction::Rfc822NameEqual => Ok(Binary(apply_plain_rfc822name_eq)),
        XacmlFunction::Rfc822NameFromString => Err(DispatchError::UnsupportedFunction { func: func.clone() }),
        XacmlFunction::StringFromRfc822Name => Err(DispatchError::UnsupportedFunction { func: func.clone() }),
        
        // ----------------- X500Name (HX500Name) -----------------
        XacmlFunction::X500NameEqual => Ok(Binary(apply_plain_x500name_eq)),
        XacmlFunction::X500NameFromString => Err(DispatchError::UnsupportedFunction { func: func.clone() }),
        XacmlFunction::StringFromX500Name => Err(DispatchError::UnsupportedFunction { func: func.clone() }),
        
        // ----------------- Base64Binary (CpuFheUint8Array) -----------------
        XacmlFunction::Base64BinaryEqual => Ok(Binary(apply_plain_vec_eq)),
        
        // ----------------- HexBinary (CpuFheUint8Array) -----------------
        XacmlFunction::HexBinaryEqual => Ok(Binary(apply_plain_vec_eq)),
        
        XacmlFunction::AllOf(_) => Ok(Binary(apply_plain_all_of)),
        XacmlFunction::AnyOf(_) => Ok(Binary(apply_plain_any_of)),
        XacmlFunction::AnyOfAny(_) => Ok(Binary(apply_plain_any_of_any)),
        XacmlFunction::AnyOfAll(_) => Ok(Binary(apply_plain_any_of_all)),
        XacmlFunction::AllOfAny(_) => Ok(Binary(apply_plain_all_of_any)),
        XacmlFunction::AllOfAll(_) => Ok(Binary(apply_plain_all_of_all)),
    
        // ----------------- Bagging -----------------
        XacmlFunction::StringOneAndOnly => Ok(Unary(apply_plain_string_one_and_only)),
        XacmlFunction::StringBagSize => Ok(Unary(apply_plain_string_bag_size)),
        XacmlFunction::StringIsIn => Ok(Binary(apply_plain_string_is_in)),
        XacmlFunction::StringAtLeastOneMemberOf => {
            Ok(Binary(apply_plain_string_at_least_one_member_of))
        }
        
        // StringBag is variadic and is handled directly in
        // evaluate_function_plain / evaluate_function_plain_consume.
        _ => Err(DispatchError::UnsupportedFunction { func : cloned }),
    }
}

// ==================== Plain implementations ====================

// ---- Integer ----
fn apply_plain_integer_eq(
    func: XacmlFunction,
    a: &IntermediateTypes,
    b: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::Integer(x), IntermediateTypes::Integer(y))
        | (IntermediateTypes::Date(x), IntermediateTypes::Date(y))=> {
            Ok(IntermediateTypes::Boolean(*x == *y))
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Int64/NormalInteger, Int64/NormalInteger",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_integer_cmp(
    func: XacmlFunction,
    a: &IntermediateTypes,
    b: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::Integer(x), IntermediateTypes::Integer(y))
        | (IntermediateTypes::Date(x), IntermediateTypes::Date(y)) => {
            let out = match func {
                XacmlFunction::IntegerGreaterThan => *x > *y,
                XacmlFunction::IntegerGreaterThanOrEqual => *x >= *y,
                XacmlFunction::IntegerLessThan => *x < *y,
                XacmlFunction::IntegerLessThanOrEqual => *x <= *y,
                
                XacmlFunction::DateGreaterThan => *x > *y,
                XacmlFunction::DateGreaterThanOrEqual => *x >= *y,
                XacmlFunction::DateLessThan => *x < *y,
                XacmlFunction::DateLessThanOrEqual => *x <= *y,
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            Ok(IntermediateTypes::Boolean(out))
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Int64/NormalInteger, Int64/NormalInteger",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_integer_arith(
    func: XacmlFunction,
    a: &IntermediateTypes,
    b: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::Integer(x), IntermediateTypes::Integer(y)) => {
            // Avoid UB / panics for div/mod by zero. Keep your error type unchanged.
            if matches!(func, XacmlFunction::IntegerDivide | XacmlFunction::IntegerMod) && *y == 0 {
                return Err(DispatchError::TypeMismatch {
                    func,
                    expected: "Int64/NormalInteger with non-zero divisor",
                    got: "divisor = 0",
                });
            }

            let out: NormalInteger = match func {
                XacmlFunction::IntegerAdd => *x + *y,
                XacmlFunction::IntegerSubtract => *x - *y,
                XacmlFunction::IntegerMultiply => *x * *y,
                XacmlFunction::IntegerDivide => *x / *y,
                XacmlFunction::IntegerMod => *x % *y,
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };

            Ok(IntermediateTypes::Integer(out))
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Int64/NormalInteger, Int64/NormalInteger",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_integer_abs(
    func: XacmlFunction,
    a: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    match a {
        IntermediateTypes::Integer(x) => Ok(IntermediateTypes::Integer(x.abs())),
        other => Err(DispatchError::TypeMismatch {
            func,
            expected: "Int64/NormalInteger",
            got: other.ty(),
        }),
    }
}

// ---- Boolean ----
fn apply_plain_not(func: XacmlFunction, a: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match a {
        IntermediateTypes::Boolean(x) => Ok(IntermediateTypes::Boolean(!*x)),
        other => Err(DispatchError::TypeMismatch { func, expected: "Bool", got: other.ty() }),
    }
}

fn apply_plain_bool_binop(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::Boolean(x), IntermediateTypes::Boolean(y)) => {
            let out = match func {
                XacmlFunction::And => (*x) && (*y),
                XacmlFunction::Or => (*x) || (*y),
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            Ok(IntermediateTypes::Boolean(out))
        }
        (IntermediateTypes::Nothing, IntermediateTypes::Boolean(y)) => {
            let out = match func {
                XacmlFunction::And => (*y),
                XacmlFunction::Or => (*y),
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            Ok(IntermediateTypes::Boolean(out))
        }
        (IntermediateTypes::Boolean(x), IntermediateTypes::Nothing) => {
            let out = match func {
                XacmlFunction::And => (*x),
                XacmlFunction::Or => (*x),
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            Ok(IntermediateTypes::Boolean(out))
        }
        (IntermediateTypes::Nothing, IntermediateTypes::Nothing) => {
            // let out = match func {
            //     XacmlFunction::And =>  IntermediateTypes::Nothing,
            //     XacmlFunction::Or => IntermediateTypes::Nothing,
            //     _ => return Err(DispatchError::UnsupportedFunction { func }),
            // };
            Ok(IntermediateTypes::Nothing)
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Bool, Bool",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_bool_eq(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::Boolean(x), IntermediateTypes::Boolean(y)) => Ok(IntermediateTypes::Boolean(*x == *y)),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Bool, Bool",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_bool_if_else(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes, c: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b, c) {
        (IntermediateTypes::Boolean(x), IntermediateTypes::Boolean(y), IntermediateTypes::Boolean(z)) => Ok(IntermediateTypes::Boolean(if *x {y.clone()} else {z.clone()})),
        (va, vb, vc) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Bool, Bool",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

// ---- Double ----
fn apply_plain_double_eq(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::Double(x), IntermediateTypes::Double(y)) => Ok(IntermediateTypes::Boolean(*x == *y)),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Double, Double",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_double_cmp(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::Double(x), IntermediateTypes::Double(y)) => {
            let out = match func {
                XacmlFunction::DoubleGreaterThan => *x > *y,
                XacmlFunction::DoubleGreaterThanOrEqual => *x >= *y,
                XacmlFunction::DoubleLessThan => *x < *y,
                XacmlFunction::DoubleLessThanOrEqual => *x <= *y,
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            Ok(IntermediateTypes::Boolean(out))
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Double, Double",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_double_arith(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::Double(x), IntermediateTypes::Double(y)) => {
            let out = match func {
                XacmlFunction::DoubleAdd => *x + *y,
                XacmlFunction::DoubleSubtract => *x - *y,
                XacmlFunction::DoubleMultiply => *x * *y,
                XacmlFunction::DoubleDivide => *x / *y, // fixed-point caution same as your comment
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            Ok(IntermediateTypes::Double(out))
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Double, Double",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_double_abs(func: XacmlFunction, a: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match a {
        IntermediateTypes::Double(x) => Ok(IntermediateTypes::Double(x.abs())),
        other => Err(DispatchError::TypeMismatch { func, expected: "Double", got: other.ty() }),
    }
}

// ---- Time ----
fn apply_plain_time_eq(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::Time(x), IntermediateTypes::Time(y)) => Ok(IntermediateTypes::Boolean(*x == *y)),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Time, Time",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_time_cmp(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::Time(x), IntermediateTypes::Time(y)) => {
            let out = match func {
                XacmlFunction::TimeGreaterThan => *x > *y,
                XacmlFunction::TimeGreaterThanOrEqual => *x >= *y,
                XacmlFunction::TimeLessThan => *x < *y,
                XacmlFunction::TimeLessThanOrEqual => *x <= *y,
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            Ok(IntermediateTypes::Boolean(out))
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Time, Time",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_time_in_range(
    func: XacmlFunction,
    t: &IntermediateTypes,
    start: &IntermediateTypes,
    end: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    match (t, start, end) {
        (IntermediateTypes::Time(tt), IntermediateTypes::Time(s), IntermediateTypes::Time(e)) => {
            Ok(IntermediateTypes::Boolean(*s <= *tt && *tt <= *e))
        }
        (vt, vs, ve) => Err(DispatchError::TypeMismatch {
            func,
            expected: "Time, Time, Time",
            got: if vt.ty() == vs.ty() && vs.ty() == ve.ty() { vt.ty() } else { "Mixed" },
        }),
    }
}

// ---- DateTime ----
fn apply_plain_datetime_eq(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::DateTime(x), IntermediateTypes::DateTime(y)) => Ok(IntermediateTypes::Boolean(*x == *y)),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "DateTime, DateTime",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_datetime_cmp(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::DateTime(x), IntermediateTypes::DateTime(y)) => {
            let out = match func {
                XacmlFunction::DateTimeGreaterThan => *x > *y,
                XacmlFunction::DateTimeGreaterThanOrEqual => *x >= *y,
                XacmlFunction::DateTimeLessThan => *x < *y,
                XacmlFunction::DateTimeLessThanOrEqual => *x <= *y,
                _ => return Err(DispatchError::UnsupportedFunction { func }),
            };
            Ok(IntermediateTypes::Boolean(out))
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "DateTime, DateTime",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_datetime_addsub_duration(
    func: XacmlFunction,
    dt: &IntermediateTypes,
    dur: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    match (dt, dur) {
        (IntermediateTypes::DateTime(x), IntermediateTypes::DayTimeDuration(d)) => {
            let out = match func {
                XacmlFunction::DateTimeAddDayTimeDuration => *x + *d,
                XacmlFunction::DateTimeSubtractDayTimeDuration => *x - *d,
                _ => {
                    return Err(DispatchError::TypeMismatch {
                        func,
                        expected: "DateTime +/- DayTimeDuration",
                        got: "Mismatched duration kind",
                    })
                }
            };
            Ok(IntermediateTypes::DateTime(out))
        }
        (IntermediateTypes::DateTime(x), IntermediateTypes::YearMonthDuration(d)) => {
            let out = match func {
                XacmlFunction::DateTimeAddYearMonthDuration => *x + *d,
                XacmlFunction::DateTimeSubtractYearMonthDuration => *x - *d,
                _ => {
                    return Err(DispatchError::TypeMismatch {
                        func,
                        expected: "DateTime +/- YearMonthDuration",
                        got: "Mismatched duration kind",
                    })
                }
            };
            Ok(IntermediateTypes::DateTime(out))
        }
        (vdt, vdur) => Err(DispatchError::TypeMismatch {
            func,
            expected: "DateTime, (DayTimeDuration|YearMonthDuration)",
            got: if vdt.ty() == vdur.ty() { vdt.ty() } else { "Mixed" },
        }),
    }
}

// ---- Duration equality ----
fn apply_plain_daytimeduration_eq(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::DayTimeDuration(x), IntermediateTypes::DayTimeDuration(y)) => Ok(IntermediateTypes::Boolean(*x == *y)),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "DayTimeDuration, DayTimeDuration",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_yearmonthduration_eq(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::YearMonthDuration(x), IntermediateTypes::YearMonthDuration(y)) => Ok(IntermediateTypes::Boolean(*x == *y)),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "YearMonthDuration, YearMonthDuration",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

// ---- String ----
fn apply_plain_string_eq(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::String(x), IntermediateTypes::String(y)) => Ok(IntermediateTypes::Boolean(x == y)),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "String, String",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_string_eq_ignore_case(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::String(x), IntermediateTypes::String(y)) => {
            Ok(IntermediateTypes::Boolean(x.eq_ignore_ascii_case(y)))
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "String, String",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_string_concat(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::String(x), IntermediateTypes::String(y)) => {
            let mut out = x.clone();
            out.push_str(y);
            Ok(IntermediateTypes::String(out))
        }
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "String, String",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_string_contains(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::String(x), IntermediateTypes::String(y)) => Ok(IntermediateTypes::Boolean(x.contains(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "String, String",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_string_starts_with(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::String(x), IntermediateTypes::String(y)) => Ok(IntermediateTypes::Boolean(x.starts_with(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "String, String",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_string_ends_with(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::String(x), IntermediateTypes::String(y)) => Ok(IntermediateTypes::Boolean(x.ends_with(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "String, String",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

// ---- AnyUri (String-backed) ----
fn apply_plain_anyuri_eq(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::AnyUri(x), IntermediateTypes::AnyUri(y)) => Ok(IntermediateTypes::Boolean(x == y)),
        // mirror your encrypted-side allowance: AnyUri vs String
        (IntermediateTypes::AnyUri(x), IntermediateTypes::String(y))
        | (IntermediateTypes::String(y), IntermediateTypes::AnyUri(x)) => Ok(IntermediateTypes::Boolean(x == y)),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "AnyUri(String), AnyUri(String)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_anyuri_contains(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::AnyUri(x), IntermediateTypes::AnyUri(y)) => Ok(IntermediateTypes::Boolean(x.contains(y))),
        (IntermediateTypes::AnyUri(x), IntermediateTypes::String(y)) => Ok(IntermediateTypes::Boolean(x.contains(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "AnyUri(String), (AnyUri|String)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_anyuri_starts_with(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::AnyUri(x), IntermediateTypes::AnyUri(y)) => Ok(IntermediateTypes::Boolean(x.starts_with(y))),
        (IntermediateTypes::AnyUri(x), IntermediateTypes::String(y)) => Ok(IntermediateTypes::Boolean(x.starts_with(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "AnyUri(String), (AnyUri|String)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_anyuri_ends_with(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
        (IntermediateTypes::AnyUri(x), IntermediateTypes::AnyUri(y)) => Ok(IntermediateTypes::Boolean(x.ends_with(y))),
        (IntermediateTypes::AnyUri(x), IntermediateTypes::String(y)) => Ok(IntermediateTypes::Boolean(x.ends_with(y))),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "AnyUri(String), (AnyUri|String)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

fn apply_plain_vec_eq(func: XacmlFunction, a: &IntermediateTypes, b: &IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
    match (a, b) {
    	(IntermediateTypes::Base64Binary(x), IntermediateTypes::Base64Binary(y)) => Ok(IntermediateTypes::Boolean(x==y)),
        (IntermediateTypes::HexBinary(x), IntermediateTypes::HexBinary(y)) => Ok(IntermediateTypes::Boolean(x==y)),
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "HexBinary/Base64Binary(Vec<u8>), HexBinary/Base64Binary(Vec<u8>)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

// ---- Rfc822Name (HRfc822Name) ----
fn apply_plain_rfc822name_eq(func: XacmlFunction, a: &IntermediateTypes, b: & IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
	match (a, b) {
        (IntermediateTypes::Rfc822Name(x1,x2), IntermediateTypes::Rfc822Name(y1,y2)) => {
       		let local_part_eq_res = x1.eq(y1);
        	let domain_part_eq_res = x2.to_lowercase().eq(&y2.to_lowercase());
         	let and_res = local_part_eq_res & domain_part_eq_res;
        	Ok(IntermediateTypes::Boolean(and_res))
        },
        (va, vb) => Err(DispatchError::TypeMismatch {
            func,
            expected: "AnyUri(FheAsciiString), (AnyUri|String)(FheAsciiString)",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

// ---- X500Name (HX500Name) ----
fn apply_plain_x500name_eq(func: XacmlFunction, a: &IntermediateTypes, b: & IntermediateTypes) -> Result<IntermediateTypes, DispatchError> {
	match (a, b) {
		(IntermediateTypes::X500Name(x), IntermediateTypes::X500Name(y)) => {
			// calculate all FheBools
			let kx = &mut x.keys(); 
			let ky = &mut y.keys();
			
			if (kx.len() != ky.len()) {
				return Err(DispatchError::X500NameNotAllFieldsSupplied { message: "the set of keys is not equal" }); 
			}
			
			if (!kx.all(|k| y.contains_key(k))) {
				return Err(DispatchError::X500NameNotAllFieldsSupplied { message: "one of the keys is not found in the set of other keys" }); 
			}
			let mut x_iter = x.iter();
			//	start by the first one
			let mut final_result = x_iter
				.next()
				.and_then(|(k1,v1)|
					y.get(k1).and_then(|v2|
						Some(v2.eq(v1))
					)
				).ok_or_else(|| DispatchError::X500NameNotAllFieldsSupplied { message:"fatal error: could not find key in X500Name type variable, mismatched keys"})?;
				
			for (k,v) in x_iter {
				final_result &= y.get(k).and_then(|v2|
					Some(v2.eq(v))
				).ok_or_else(|| DispatchError::X500NameNotAllFieldsSupplied { message:"fatal error: could not find key in X500Name type variable, mismatched keys"})?;
			}
			Ok(IntermediateTypes::Boolean(final_result))
		},
		(va, vb) => Err(DispatchError::TypeMismatch {
			func,
			expected: "AnyUri(FheAsciiString), (AnyUri|String)(FheAsciiString)",
			got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
		}),
	}
}


fn expect_plain_boolean(
    func: XacmlFunction,
    value: IntermediateTypes,
) -> Result<Boolean, DispatchError> {
    match value {
        IntermediateTypes::Boolean(b) => Ok(b),
        other => Err(DispatchError::TypeMismatch {
            func,
            expected: "Bool",
            got: other.ty(),
        }),
    }
}

fn eval_plain_binary_predicate_bool(
    pred: XacmlFunction,
    a: &IntermediateTypes,
    b: &IntermediateTypes,
) -> Result<Boolean, DispatchError> {
    let out = eval_plain_binary(pred.clone(), a, b)?;
    expect_plain_boolean(pred, out)
}

/// XACML any-of(predicate, value, bag)
/// OR_{e in bag} predicate(value, e)
pub fn apply_plain_any_of(
    func: XacmlFunction,
    value: &IntermediateTypes,
    bag: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    match (bag, func.clone()) {
        (IntermediateTypes::Bag(b), XacmlFunction::AnyOf(inner_pred)) => {
            let inner_pred = inner_pred.ok_or(
                DispatchError::AbsentInnerFunction {
                    message: "no inner function found in any-of function evaluation",
                }
            )?;

            let mut acc: Option<Boolean> = None;

            for elem in b {
                let r = eval_plain_binary_predicate_bool(*inner_pred.clone(), value, elem)?;

                acc = Some(match acc {
                    Some(prev) => prev || r,
                    None => r,
                });
            }

            Ok(IntermediateTypes::Boolean(acc.unwrap_or(false)))
        }

        (other, pred) => Err(DispatchError::TypeMismatch {
            func: pred,
            expected: "Bag",
            got: other.ty(),
        }),
    }
}

/// XACML all-of(predicate, value, bag)
/// AND_{e in bag} predicate(value, e)
pub fn apply_plain_all_of(
    func: XacmlFunction,
    value: &IntermediateTypes,
    bag: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    match (bag, func.clone()) {
        (IntermediateTypes::Bag(b), XacmlFunction::AllOf(inner_pred)) => {
            let inner_pred = inner_pred.ok_or(
                DispatchError::AbsentInnerFunction {
                    message: "no inner function found in all-of function evaluation",
                }
            )?;

            let mut acc: Option<Boolean> = None;

            for elem in b {
                let r = eval_plain_binary_predicate_bool(*inner_pred.clone(), value, elem)?;

                acc = Some(match acc {
                    Some(prev) => prev && r,
                    None => r,
                });
            }

            Ok(IntermediateTypes::Boolean(acc.unwrap_or(true)))
        }

        (other, pred) => Err(DispatchError::TypeMismatch {
            func: pred,
            expected: "Bag",
            got: other.ty(),
        }),
    }
}

/// XACML any-of-any(predicate, bag1, bag2)
/// OR_{x in bag1, y in bag2} predicate(x, y)
pub fn apply_plain_any_of_any(
    func: XacmlFunction,
    bag1: &IntermediateTypes,
    bag2: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    match (bag1, bag2, func.clone()) {
        (
            IntermediateTypes::Bag(b1),
            IntermediateTypes::Bag(b2),
            XacmlFunction::AnyOfAny(inner_pred),
        ) => {
            let inner_pred = inner_pred.ok_or(
                DispatchError::AbsentInnerFunction {
                    message: "no inner function found in any-of-any function evaluation",
                }
            )?;

            let mut acc: Option<Boolean> = None;

            for x in b1 {
                for y in b2 {
                    let r = eval_plain_binary_predicate_bool(*inner_pred.clone(), x, y)?;

                    acc = Some(match acc {
                        Some(prev) => prev || r,
                        None => r,
                    });
                }
            }

            Ok(IntermediateTypes::Boolean(acc.unwrap_or(false)))
        }

        (va, vb, pred) => Err(DispatchError::TypeMismatch {
            func: pred,
            expected: "Bag, Bag",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

/// XACML all-of-any(predicate, bag1, bag2)
/// AND_{x in bag1} OR_{y in bag2} predicate(x, y)
pub fn apply_plain_all_of_any(
    func: XacmlFunction,
    bag1: &IntermediateTypes,
    bag2: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    match (bag1, bag2, func.clone()) {
        (
            IntermediateTypes::Bag(b1),
            IntermediateTypes::Bag(b2),
            XacmlFunction::AllOfAny(inner_pred),
        ) => {
            let inner_pred = inner_pred.ok_or(
                DispatchError::AbsentInnerFunction {
                    message: "no inner function found in all-of-any function evaluation",
                }
            )?;

            let mut outer_acc: Option<Boolean> = None;

            for x in b1 {
                let mut inner_acc: Option<Boolean> = None;

                for y in b2 {
                    let r = eval_plain_binary_predicate_bool(*inner_pred.clone(), x, y)?;

                    inner_acc = Some(match inner_acc {
                        Some(prev) => prev || r,
                        None => r,
                    });
                }

                let inner = inner_acc.unwrap_or(false);

                outer_acc = Some(match outer_acc {
                    Some(prev) => prev && inner,
                    None => inner,
                });
            }

            Ok(IntermediateTypes::Boolean(outer_acc.unwrap_or(true)))
        }

        (va, vb, pred) => Err(DispatchError::TypeMismatch {
            func: pred,
            expected: "Bag, Bag",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

/// XACML any-of-all(predicate, bag1, bag2)
/// AND_{y in bag2} OR_{x in bag1} predicate(x, y)
pub fn apply_plain_any_of_all(
    func: XacmlFunction,
    bag1: &IntermediateTypes,
    bag2: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    match (bag1, bag2, func.clone()) {
        (
            IntermediateTypes::Bag(b1),
            IntermediateTypes::Bag(b2),
            XacmlFunction::AnyOfAll(inner_pred),
        ) => {
            let inner_pred = inner_pred.ok_or(
                DispatchError::AbsentInnerFunction {
                    message: "no inner function found in any-of-all function evaluation",
                }
            )?;

            let mut outer_acc: Option<Boolean> = None;

            for y in b2 {
                let mut inner_acc: Option<Boolean> = None;

                for x in b1 {
                    let r = eval_plain_binary_predicate_bool(*inner_pred.clone(), x, y)?;

                    inner_acc = Some(match inner_acc {
                        Some(prev) => prev || r,
                        None => r,
                    });
                }

                let inner = inner_acc.unwrap_or(false);

                outer_acc = Some(match outer_acc {
                    Some(prev) => prev && inner,
                    None => inner,
                });
            }

            Ok(IntermediateTypes::Boolean(outer_acc.unwrap_or(true)))
        }

        (va, vb, pred) => Err(DispatchError::TypeMismatch {
            func: pred,
            expected: "Bag, Bag",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

/// XACML all-of-all(predicate, bag1, bag2)
/// AND_{x in bag1, y in bag2} predicate(x, y)
pub fn apply_plain_all_of_all(
    func: XacmlFunction,
    bag1: &IntermediateTypes,
    bag2: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    match (bag1, bag2, func.clone()) {
        (
            IntermediateTypes::Bag(b1),
            IntermediateTypes::Bag(b2),
            XacmlFunction::AllOfAll(inner_pred),
        ) => {
            let inner_pred = inner_pred.ok_or(
                DispatchError::AbsentInnerFunction {
                    message: "no inner function found in all-of-all function evaluation",
                }
            )?;

            let mut acc: Option<Boolean> = None;

            for x in b1 {
                for y in b2 {
                    let r = eval_plain_binary_predicate_bool(*inner_pred.clone(), x, y)?;

                    acc = Some(match acc {
                        Some(prev) => prev && r,
                        None => r,
                    });
                }
            }

            Ok(IntermediateTypes::Boolean(acc.unwrap_or(true)))
        }

        (va, vb, pred) => Err(DispatchError::TypeMismatch {
            func: pred,
            expected: "Bag, Bag",
            got: if va.ty() == vb.ty() { va.ty() } else { "Mixed" },
        }),
    }
}

pub fn apply_plain_string_one_and_only(
    func: XacmlFunction,
    bag: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    match bag {
        IntermediateTypes::Bag(values) => {
            if values.len() != 1 {
                return Err(DispatchError::Indeterminate {
                    func,
                    reason: "string-one-and-only requires exactly one value in the bag",
                });
            }

            match &values[0] {
                IntermediateTypes::String(_) => Ok(values[0].clone()),

                other => Err(DispatchError::TypeMismatch {
                    func,
                    expected: "String",
                    got: other.ty(),
                }),
            }
        }

        other => Err(DispatchError::TypeMismatch {
            func,
            expected: "Bag",
            got: other.ty(),
        }),
    }
}

pub fn apply_plain_string_bag_size(
    func: XacmlFunction,
    bag: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    match bag {
        IntermediateTypes::Bag(values) => {
            for value in values {
                if !matches!(value, IntermediateTypes::String(_)) {
                    return Err(DispatchError::TypeMismatch {
                        func,
                        expected: "Bag(String)",
                        got: value.ty(),
                    });
                }
            }

            let size: NormalInteger = values.len().try_into().map_err(|_| {
                DispatchError::Indeterminate {
                    func: func.clone(),
                    reason: "bag size does not fit into NormalInteger",
                }
            })?;

            Ok(IntermediateTypes::Integer(size))
        }

        other => Err(DispatchError::TypeMismatch {
            func,
            expected: "Bag",
            got: other.ty(),
        }),
    }
}

pub fn apply_plain_string_is_in(
    func: XacmlFunction,
    value: &IntermediateTypes,
    bag: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    let value = match value {
        IntermediateTypes::String(s) => s,
        other => {
            return Err(DispatchError::TypeMismatch {
                func,
                expected: "String",
                got: other.ty(),
            });
        }
    };

    match bag {
        IntermediateTypes::Bag(values) => {
            let mut acc = false;

            for elem in values {
                let elem = match elem {
                    IntermediateTypes::String(s) => s,
                    other => {
                        return Err(DispatchError::TypeMismatch {
                            func,
                            expected: "Bag(String)",
                            got: other.ty(),
                        });
                    }
                };

                acc = acc || value == elem;
            }

            Ok(IntermediateTypes::Boolean(acc))
        }

        other => Err(DispatchError::TypeMismatch {
            func,
            expected: "Bag",
            got: other.ty(),
        }),
    }
}

pub fn apply_plain_string_bag_from_owned_args(
    func: XacmlFunction,
    args: Vec<Option<IntermediateTypes>>,
) -> Result<IntermediateTypes, DispatchError> {
    let mut values = Vec::with_capacity(args.len());

    for arg in args {
        let value = arg.ok_or(DispatchError::Indeterminate {
            func: func.clone(),
            reason: "missing argument in string-bag",
        })?;

        match value {
            IntermediateTypes::String(_) => {
                values.push(value);
            }

            other => {
                return Err(DispatchError::TypeMismatch {
                    func,
                    expected: "String",
                    got: other.ty(),
                });
            }
        }
    }

    Ok(IntermediateTypes::Bag(values))
}

pub fn apply_plain_string_bag_from_borrowed_args(
    func: XacmlFunction,
    args: Vec<Option<&IntermediateTypes>>,
) -> Result<IntermediateTypes, DispatchError> {
    let mut values = Vec::with_capacity(args.len());

    for arg in args {
        let value = arg.ok_or(DispatchError::Indeterminate {
            func: func.clone(),
            reason: "missing argument in string-bag",
        })?;

        match value {
            IntermediateTypes::String(_) => {
                values.push(value.clone());
            }

            other => {
                return Err(DispatchError::TypeMismatch {
                    func,
                    expected: "String",
                    got: other.ty(),
                });
            }
        }
    }

    Ok(IntermediateTypes::Bag(values))
}

pub fn apply_plain_string_at_least_one_member_of(
    func: XacmlFunction,
    left: &IntermediateTypes,
    right: &IntermediateTypes,
) -> Result<IntermediateTypes, DispatchError> {
    let left_values = expect_plain_string_values(func.clone(), left)?;
    let right_values = expect_plain_string_values(func.clone(), right)?;

    for s1 in left_values {
        for s2 in &right_values {
            if s1.as_str() == (*s2).as_str() {
                return Ok(IntermediateTypes::Boolean(true));
            }
        }
    }

    Ok(IntermediateTypes::Boolean(false))
}

fn expect_plain_string_values<'a>(
    func: XacmlFunction,
    value: &'a IntermediateTypes,
) -> Result<Vec<&'a String>, DispatchError> {
    match value {
        IntermediateTypes::String(s) => Ok(vec![s]),

        IntermediateTypes::Bag(values) => {
            let mut out = Vec::with_capacity(values.len());

            for elem in values {
                match elem {
                    IntermediateTypes::String(s) => out.push(s),

                    other => {
                        return Err(DispatchError::TypeMismatch {
                            func,
                            expected: "String or Bag(String)",
                            got: other.ty(),
                        });
                    }
                }
            }

            Ok(out)
        }

        other => Err(DispatchError::TypeMismatch {
            func,
            expected: "String or Bag(String)",
            got: other.ty(),
        }),
    }
}

pub fn apply_plain_bool_vec_op_from_borrowed_args(
    func: XacmlFunction,
    args: Vec<Option<&IntermediateTypes>>,
) -> Result<IntermediateTypes, DispatchError> {
    if args.is_empty() {
        return Err(DispatchError::ArityMismatch {
            func,
            expected: 1,
            got: 0,
        });
    }

    let mut values: Vec<Boolean> = Vec::with_capacity(args.len());

    for arg in args {
        let value = arg.ok_or(DispatchError::Indeterminate {
            func: func.clone(),
            reason: "missing argument in boolean vector operation",
        })?;

        match value {
            IntermediateTypes::Boolean(b) => {
                values.push(*b);
            }

            // Optional compatibility with your Nothing accumulator style.
            IntermediateTypes::Nothing => {}

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
        return Ok(IntermediateTypes::Nothing);
    }

    let result = match func {
        XacmlFunction::And => {
            reduce_balanced(values, |a, b| a && b)
        }

        XacmlFunction::Or => {
            reduce_balanced(values, |a, b| a || b)
        }

        _ => {
            return Err(DispatchError::UnsupportedFunction { func });
        }
    }
    .ok_or(DispatchError::Indeterminate {
        func: func.clone(),
        reason: "failed to reduce boolean vector operation",
    })?;

    Ok(IntermediateTypes::Boolean(result))
}

pub fn apply_plain_bool_vec_op_from_owned_args(
    func: XacmlFunction,
    args: Vec<Option<IntermediateTypes>>,
) -> Result<IntermediateTypes, DispatchError> {
    if args.is_empty() {
        return Err(DispatchError::ArityMismatch {
            func,
            expected: 1,
            got: 0,
        });
    }

    let mut values: Vec<Boolean> = Vec::with_capacity(args.len());

    for arg in args {
        let value = arg.ok_or(DispatchError::Indeterminate {
            func: func.clone(),
            reason: "missing argument in boolean vector operation",
        })?;

        match value {
            IntermediateTypes::Boolean(b) => {
                values.push(b);
            }

            // Optional compatibility with your Nothing accumulator style.
            IntermediateTypes::Nothing => {}

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
        return Ok(IntermediateTypes::Nothing);
    }

    let result = match func {
        XacmlFunction::And => {
            reduce_balanced(values, |a, b| a && b)
        }

        XacmlFunction::Or => {
            reduce_balanced(values, |a, b| a || b)
        }

        _ => {
            return Err(DispatchError::UnsupportedFunction { func });
        }
    }
    .ok_or(DispatchError::Indeterminate {
        func: func.clone(),
        reason: "failed to reduce boolean vector operation",
    })?;

    Ok(IntermediateTypes::Boolean(result))
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