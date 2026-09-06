use std::collections::HashMap;


use uuid::Uuid;
use xsd_parser::xml::Text;

use crate::xacml::types::{AttributeType, AttributeValueType, AttributeValueWithId, AttributesType, Drill, Request};

impl Drill<Vec<AttributeValueWithId>> for Request {
    fn drill_for_attributes_values_and_transform_values(&mut self) -> std::collections::HashMap<uuid::Uuid,Vec<AttributeValueWithId>> {
        let mut result = HashMap::<uuid::Uuid,Vec<AttributeValueWithId>>::new();

        for c in &mut self.attributes {
            let r = c.drill_for_attributes_values_and_transform_values();
            result.extend(r);
        }

        result
    }
}

impl Request {
    fn from_name(&self, attribute_id: &str) -> Option<String>{
        self.attributes.iter().find_map(
            |atts_type| atts_type.attribute.iter().find_map(
                |att_type| if att_type.attribute_id.as_str() == attribute_id {
                    Some(att_type.attribute_id.clone())
                }else {
                    None
                }
            )
        )
    }
}

impl Drill<Vec<AttributeValueWithId>> for AttributesType {
    fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<uuid::Uuid,Vec<AttributeValueWithId>> {
        let mut result = HashMap::<uuid::Uuid,Vec<AttributeValueWithId>>::new();
        for a in &mut self.attribute {
            let r = a.drill_for_attributes_values_and_transform_values();
            result.extend(r);
        }
        result
    }
}

impl Drill<Vec<AttributeValueWithId>> for AttributeType {
    fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<uuid::Uuid,Vec<AttributeValueWithId>> {
        let mut result = HashMap::<uuid::Uuid,Vec<AttributeValueWithId>>::new();
        let new_key = Uuid::new_v4();
        for a in &mut self.attribute_value {
            let r = a.drill_for_attributes_values_and_transform_values();
            for (_k,mut v) in r {
                //  change the ID 
                //  this bad practice but we gotta do with it now
                v.id = self.attribute_id.clone();

                if let Some(existing_value) = result.get_mut(&new_key) {
                    existing_value.push(v);
                } else {
                    //  insert the value
                    result.insert(new_key, vec![v]);
                }

            }
            //  write out that newly generated uuid to the policy attribute text
            a.text =  Some(Text::new(new_key.to_string()));

        }
        result
    }
}
