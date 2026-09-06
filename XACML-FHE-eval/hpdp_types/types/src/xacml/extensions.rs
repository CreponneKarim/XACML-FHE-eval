use std::hash::Hash;

use crate::xacml::types::{AttributeDesignatorType, AttributeValueType};

impl PartialEq for AttributeDesignatorType {
    fn eq(&self, other: &Self) -> bool {
        self.must_be_present == other.must_be_present &&
        self.attribute_id == self.attribute_id &&
        self.category == self.category &&
        self.data_type == self.data_type &&
        self.issuer == self.issuer
    }
    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

impl Eq for AttributeDesignatorType {

}

impl Hash for AttributeDesignatorType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut result = format!("{:?}",self);
        result.hash(state);
    }
}

impl AttributeDesignatorType {
    pub fn get_copy(&self) -> AttributeDesignatorType {
        AttributeDesignatorType { category: self.category.clone(), attribute_id: self.attribute_id.clone(), data_type: self.data_type.clone(), issuer: self.issuer.clone(), must_be_present: self.must_be_present.clone() }
    }
}

impl AttributeValueType {
    pub fn get_copy(&self) -> AttributeValueType {
        AttributeValueType { data_type: self.data_type.clone(),text: self.text.clone()}
    }
}