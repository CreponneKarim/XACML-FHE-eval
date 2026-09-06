use std::collections::HashMap;

use tfhe::{FheBool, ServerKey};
use uuid::Uuid;

use crate::{homomorphic::{policy::EncryptedTypes, types_impls::HBoolean}, mixed::MixedTypes, xacml::types::Request};

pub struct Context {
    pub request: Request,
    pub req_enc_attributes: HashMap<Uuid,Vec<MixedTypes>>,
    pub pol_enc_attributes: HashMap<Uuid,MixedTypes>,
    pub encrypted_effects: HashMap<Uuid, HBoolean>,
    pub enc_true: FheBool,
    pub enc_false: FheBool,
    pub server_key : ServerKey
}

impl Context {
    pub fn get_req_attr<'a>(&'a self, att_id: &str) -> Option<&'a Vec<MixedTypes>> {
        //  find first attribute value and return the uuid that is found in it !
        let result = self.request.attributes.iter().find_map(|v| {
            v.attribute.iter().find_map( |v2| {
                if v2.attribute_id.as_str() == att_id {
                    v2.attribute_value.iter().find_map(|v3| {
                        v3.text.clone().and_then(|attribute_uuid|{
                            v3.text.clone()
                        })
                    })
                }else {
                    None
                }
            })
        }).and_then(|uuid_text| {
            self.req_enc_attributes.iter().find_map(|(k,v)| {
                if k.to_string() == uuid_text.0 {
                    Some(v)
                }else{
                    None
                }
            })
        });

        //  search in the hashmap
        result
    }

    pub fn get_policy_attr_from_uuid<'a>(&'a self, uuid: &str) -> Option<&'a MixedTypes> {
        self.pol_enc_attributes.iter().find_map(|(k,v)| {
            if k.to_string().as_str() == uuid {
                Some(v)
            }else {
                None
            }
        })
    }

    pub fn get_effect<'a>(&'a self, uuid: &Uuid) -> Option<&'a HBoolean> {
        self.encrypted_effects.iter().find_map(|(k,v)|
            if k.eq(uuid) {
                Some(v)
            }else {
                None
            }
        )
    }
}

// pub struct EncryptedRequest {
//     req: Request,
// }