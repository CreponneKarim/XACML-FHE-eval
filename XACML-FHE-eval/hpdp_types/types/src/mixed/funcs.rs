use std::time::Instant;

use crate::{homomorphic::{funcs::{evaluate_function, evaluate_function_consume}, policy::{EncryptedTypes, MixedAttributeSerStruct}}, intermediate::{IntermediateTypes, funcs::{evaluate_function_plain, evaluate_function_plain_consume}}, mixed::MixedTypes };

#[derive(Debug,Clone)]
pub enum DispatchError {
    EncryptedDispatchError(crate::homomorphic::funcs::DispatchError),
    PlainDispatchError(crate::intermediate::funcs::DispatchError),
}

pub fn evaluate_function_mixed_consume(
    urn: &str,
    urn2: Option<&str>,
    arguments: Vec<Option<MixedTypes>>,
) -> Result<MixedTypes, DispatchError> {
    // if one is encrypted then evaluate through encrypted types
    let mut plain_array: Vec<Option<&IntermediateTypes>> = Vec::new();
    for arg in & arguments {
        let is_it_encrypted = arg.as_ref().and_then(|e| 
            match e {
                MixedTypes::EncryptedType(_) => {Some(true)}
                _ => {Some(false)}
            }
        ).unwrap_or(false);
        
        if is_it_encrypted {
            //  turn all plains into trivial encrypted values and call an evaluation function
            let encrypted_array = arguments.iter().map(
                |v| v.as_ref().and_then(
                    |inner| match inner {
                        MixedTypes::EncryptedType(enc) => {Some(enc.clone())},
                        MixedTypes::IntermediateType(plain) => {println!("trivially encrypting");Some(plain.trivial_enc())}
                    }
                )
            ).collect::<Vec<Option<EncryptedTypes>>>();

            return evaluate_function_consume(urn, urn2, encrypted_array)
                .and_then(|v|Ok(MixedTypes::EncryptedType(v)))
                .map_err(|e|DispatchError::EncryptedDispatchError(e));

        }else {
            match arg.as_ref() {
                Some(inter) => {
                    match inter {
                        MixedTypes::IntermediateType(inner) => {plain_array.push(Some(inner));}
                        MixedTypes::EncryptedType(_) => {
                            panic!("expect IntermediateTypes found EncryptedType - Unexpected behaviour")
                        }
                    }
                },
                None => {
                    plain_array.push(None);
                },
            } 
                
            
        }
    }

    //  evaluate as plains
    evaluate_function_plain(urn, urn2, plain_array)
        .map_err(|e|DispatchError::PlainDispatchError(e))
        .and_then(|r|Ok(MixedTypes::IntermediateType(r)))
}

pub fn evaluate_function_mixed_consume_no_option(
    urn: &str,
    urn2: Option<&str>,
    arguments: Vec<MixedTypes>,
) -> Result<MixedTypes, DispatchError> {
    // if one is encrypted then evaluate through encrypted types
    let mut plain_array: Vec<Option<&IntermediateTypes>> = Vec::new();
    for arg in & arguments {
        let is_it_encrypted = match arg {
            MixedTypes::EncryptedType(_) => {true}
            _ => {false}
        };
        
        if is_it_encrypted {
            //  turn all plains into trivial encrypted values and call an evaluation function
            let encrypted_array = arguments.iter().map(
                |v| match v {
                    MixedTypes::EncryptedType(enc) => {Some(enc.clone())},
                    MixedTypes::IntermediateType(plain) => {println!("trivially encrypting");Some(plain.trivial_enc())}
                }
            ).collect::<Vec<Option<EncryptedTypes>>>();

            return evaluate_function_consume(urn, urn2, encrypted_array)
                .and_then(|v|Ok(MixedTypes::EncryptedType(v)))
                .map_err(|e|DispatchError::EncryptedDispatchError(e));

        }else {
            match arg {
                MixedTypes::IntermediateType(inner) => {plain_array.push(Some(inner));}
                MixedTypes::EncryptedType(_) => {
                    panic!("expect IntermediateTypes found EncryptedType - Unexpected behaviour")
                }
            }
        }
    }

    //  evaluate as plains
    evaluate_function_plain(urn, urn2, plain_array)
        .map_err(|e|DispatchError::PlainDispatchError(e))
        .and_then(|r|Ok(MixedTypes::IntermediateType(r)))
}


pub fn evaluate_function_mixed(
	urn: &str,
    urn2: Option<&str>,
    arguments: Vec<Option<&MixedTypes>>,
) -> Result<MixedTypes, DispatchError> {
    // if one is encrypted then evaluate through encrypted types
    let mut plain_array: Vec<Option<&IntermediateTypes>> = Vec::new();
    for arg in &arguments {
        let is_it_encrypted = arg.as_ref().and_then(|e| 
            match e {
                MixedTypes::EncryptedType(_) => {Some(true)}
                _ => {Some(false)}
            }
        ).unwrap_or(false);
        
        if is_it_encrypted {
            //  turn all plains into trivial encrypted values and call an evaluation function
            let encrypted_array = arguments.iter().map(
                |v| v.as_ref().and_then(
                    |inner| match inner {
                        MixedTypes::EncryptedType(enc) => {Some(enc.clone())},
                        MixedTypes::IntermediateType(plain) => {{println!("trivially encrypting");Some(plain.trivial_enc())}}
                    }
                )
            ).collect::<Vec<Option<EncryptedTypes>>>();
 
            let evaluation_result = evaluate_function_consume(urn, urn2, encrypted_array)
                .and_then(|v|Ok(MixedTypes::EncryptedType(v)))
                .map_err(|e|DispatchError::EncryptedDispatchError(e));

            return evaluation_result;
        } else {
            match arg.as_ref() {
                Some(inter) => {
                    match inter {
                        MixedTypes::IntermediateType(inner) => {plain_array.push(Some(inner));}
                        MixedTypes::EncryptedType(_) => {
                            panic!("expect IntermediateTypes found EncryptedType - Unexpected behaviour")
                        }
                    }
                },
                None => {
                    plain_array.push(None);
                },
            } 
        }
    }

    // println!("the passed array is {:#?} and the function is {}",plain_array, urn);

    //  evaluate as plains
    evaluate_function_plain(urn, urn2, plain_array)
        .map_err(|e|DispatchError::PlainDispatchError(e))
        .and_then(|r|Ok(MixedTypes::IntermediateType(r)))
}


pub fn evaluate_function_mixed_no_option(
    urn: &str,
    urn2: Option<&str>,
    arguments: Vec<&MixedTypes>,
) -> Result<MixedTypes, DispatchError> {
    // if one is encrypted then evaluate through encrypted types
    let mut plain_array: Vec<Option<&IntermediateTypes>> = Vec::new();
    for arg in &arguments {
        let is_it_encrypted = match arg{
            MixedTypes::EncryptedType(_) => {true}
            _ => {false}
        };
        
        if is_it_encrypted {
            //  turn all plains into trivial encrypted values and call an evaluation function
            let encrypted_array = arguments.iter().map(
                |v| match v {
                    MixedTypes::EncryptedType(enc) => {Some(enc.clone())},
                    MixedTypes::IntermediateType(plain) => {{println!("trivially encrypting");Some(plain.trivial_enc())}}
                }
            ).collect::<Vec<Option<EncryptedTypes>>>();
 
            let evaluation_result = evaluate_function_consume(urn, urn2, encrypted_array)
                .and_then(|v|Ok(MixedTypes::EncryptedType(v)))
                .map_err(|e|DispatchError::EncryptedDispatchError(e));

            return evaluation_result;
        }else {
            match arg {
                MixedTypes::IntermediateType(inner) => {plain_array.push(Some(inner));}
                MixedTypes::EncryptedType(_) => {
                    panic!("expect IntermediateTypes found EncryptedType - Unexpected behaviour")
                }
            } 
        }
    }

    //  evaluate as plains
    evaluate_function_plain(urn, urn2, plain_array)
        .map_err(|e|DispatchError::PlainDispatchError(e))
        .and_then(|r|Ok(MixedTypes::IntermediateType(r)))
}
