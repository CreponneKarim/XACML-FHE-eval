use core::panic;
use std::{collections::HashMap, str::FromStr};
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;
use xsd_parser::{AsAny, xml::{ Text}};

use crate::xacml::types::ExtractAttributeDesignators;
use crate::xacml::types::{AllOfType, AllOfTypeContent, AnyOfType, AnyOfTypeContent, ApplyType, AttributeDesignator, AttributeSelector, AttributeValue, AttributeValueType, AttributeValueWithId, ConditionType, Drill, Expression, ExpressionTrait, Function, MatchContent47Type, MatchType, Policy, PolicyContent41Type, PolicySetContent31Type, PolicySetType, PolicyType, RuleType, TargetType, TargetTypeContent, VariableReference};

#[derive(Debug)]
pub enum Loadout {
	Policy(PolicyType),
	PolicySet(PolicySetType)
}

impl ExtractAttributeDesignators for Loadout {
	fn collect_attribute_designators(
    &self,
    output: &mut super::types::CategorizedAttributeDesignators,
	)
	{
		match self {
			Loadout::Policy(p) 		=> p.collect_attribute_designators(output),
			Loadout::PolicySet(ps) 	=> ps.collect_attribute_designators(output),
		}
	}
}
// #[derive(Debug, PartialEq,Deserialize, Serialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
// pub enum LoadoutToSend {
// 	Policy(String),
// 	PolicySet(String)
// }

pub fn nb_rules_ps(p: &PolicySetType)->usize {
	let mut counter = 0;

	for c in &p.content_31 {
		match c {
			PolicySetContent31Type::Policy(p) => {
				counter+=nb_rules_p(p);
			}
			PolicySetContent31Type::PolicySet(p) => {
				counter+=nb_rules_ps(p);
			}
			_=>{}
		}
	}
	counter
}

pub fn nb_rules_p(p: &Policy)->usize {
	let mut counter = 0;

	for c in &p.content_41 {
		match c {
			PolicyContent41Type::Rule(_) => {
				counter+=1;
			}
			_=>{}
		}
	}
	counter
}

impl Loadout {

	pub fn get_attributes_ids_of_n_first_rules(&self,n:usize) -> Vec<String> {
		let mut att_ids = Vec::new();
		let mut processed_rules = 0;
		match self {
			Loadout::Policy(p) =>  {
				p.content_41.iter().for_each(|p| {
					match p {
						PolicyContent41Type::Rule(r) => {
							r.target.as_ref().inspect(|t| {
								t.content.iter().for_each(|inner_t| {
									inner_t.any_of.content.iter().for_each(|inner_any_of| {
										inner_any_of.all_of.content.iter().for_each(|inner_all_of| {
											match &inner_all_of.match_.content_47{
												MatchContent47Type::AttributeDesignator(att_desig) => {
													// inner_all_of.match_.content_47.as_ref().inspect(|inner_target_value| {
													if processed_rules >=n {
														return;
													}else {
														processed_rules+=1;
														att_ids.push(att_desig.attribute_id.clone());
													}
													// });
												},
												MatchContent47Type::AttributeSelector(_) => {
													panic!("");
												},
											} 
											
										});
									});
								});
							});
						},
						_=>{}
					}
				});
			},
			Loadout::PolicySet(ps) => {
				panic!()
			}
		}	
		att_ids	
	}


	pub fn nb_rules(&self) ->usize {
		let mut counter = 0;
		match self {
			Loadout::Policy(p) => {
				counter+=nb_rules_p(p);
			}

			Loadout::PolicySet(p) => {
				for c in &p.content_31 {
					match c {
						PolicySetContent31Type::Policy(p) => {
							counter+=nb_rules_p(p);
						}
						_=>{}
					}
				}
			}
		}
		counter
	}
}

impl Drill<AttributeValueWithId> for Loadout{
	fn drill_for_attributes_values_and_transform_values(&mut self) -> std::collections::HashMap<Uuid,AttributeValueWithId> {
		match self {
			Loadout::Policy(p) => p.drill_for_attributes_values_and_transform_values(),
			Loadout::PolicySet(p) => p.drill_for_attributes_values_and_transform_values(),
		}
	}
}



impl Drill<AttributeValueWithId> for PolicySetType {
	fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {

		let mut result = HashMap::new();
		for content in &mut self.content_31 {
			let atts = content.drill_for_attributes_values_and_transform_values();
			result.extend(atts.into_iter());
		}
		result
	}
}

impl Drill<AttributeValueWithId> for PolicySetContent31Type {
	fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
		let mut result = HashMap::new();
		
		// match self {
		// 	PolicySetTypeContent::Target(e)  => {
		// 		let atts = e.drill_for_attributes_values_and_transform_values();
		// 		result.extend(atts);
		// 	},
		// 	PolicySetTypeContent::Policy(e) => {
		// 		let atts = e.drill_for_attributes_values_and_transform_values();
		// 		result.extend(atts);
		// 	}
		// 	_=> {}
		// }
		match self {
			PolicySetContent31Type::Policy(p) => {
				let r = p.drill_for_attributes_values_and_transform_values();
				result.extend(r);
			},
			PolicySetContent31Type::PolicySet(p) => {
				let r = p.drill_for_attributes_values_and_transform_values();
				result.extend(r);
			},
			_=>{}
		}

		result
	}
}

impl Drill<AttributeValueWithId> for PolicyType {
	fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
		let mut result = HashMap::new();
		for content in &mut self.content_41 {
			let atts = content.drill_for_attributes_values_and_transform_values();
			result.extend(atts);
		}
		result
	}
}

impl Drill<AttributeValueWithId> for PolicyContent41Type {
	fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
		// let mut result = HashMap::new();
		
		match self {
			PolicyContent41Type::Rule(r) => {
				r.drill_for_attributes_values_and_transform_values()
			}
			_=> {
				HashMap::new()
			}
		}
	}
}

impl Drill<AttributeValueWithId> for RuleType {
	fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
		let mut result = self.target.as_mut().map_or_else(
			|| HashMap::new(),
			|v| v.drill_for_attributes_values_and_transform_values());
		let res2 = self.condition.as_mut().map_or_else(
			|| HashMap::new(),
			|v| v.drill_for_attributes_values_and_transform_values());
		result.extend(res2);
		//	explore result and replace the rule id
		let r = result.into_iter().map(|(k,v)|{
				let mut c  = v.clone();
				c.rule_id =  self.rule_id.clone().trim().to_string();
				(k.clone(),c)
			}
		).collect();
		r
	}
}
impl Drill<AttributeValueWithId> for ConditionType{
	fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
		self.expression.0.drill_for_attributes_values_and_transform_values()
	}
}

// impl Drill<AttributeValueWithId> for Expression {
// 	fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
// 		let mut result = HashMap::<Uuid, AttributeValueType>::new();
// 		// match self {
// 		// 	Expression::AttributeValue(e) => {
// 		// 		let id = Uuid::new_v4();
// 		// 		result.insert(id, e.get_copy());
// 		// 		e.text_before = Some(Text::new(id.to_string()));
// 		// 		e.any = Vec::new();
// 		// 	},
// 		// 	Expression::Apply(e) => {
// 		// 		let r = e.drill_for_attributes_values_and_transform_values();
// 		// 		result.extend(r);
// 		// 	},
// 		// 	_ =>{}
// 		// }

// 		let inner = self.0;


// 		result
// 	}
// }

impl Drill<AttributeValueWithId> for ApplyType {
	fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
		let mut result = HashMap::<Uuid, AttributeValueWithId>::new();
		
		let mut id = None;
		let mut att_val_option: Option<HashMap<Uuid,AttributeValueWithId>> = None;
		for expression in &mut self.expression {

			// if          let Some(apply)     = expression.as_any_mut().downcast_mut::<ApplyType>() {
            // 	apply.evaluation_expression(context)
			// }else if    let Some(function)  = expression.as_any_mut().downcast_mut::<Function>() {
			// 	function.evaluation_expression(context)
			// }else if    let Some(var_ref)   = expression.as_any_mut().downcast_mut::<VariableReference>() {
			// 	var_ref.evaluation_expression(context)
			// }else 
			if    let Some(var_ref)   = (*expression.0).as_any_mut().downcast_mut::<AttributeDesignator>() {
				id = Some(var_ref.attribute_id.clone());
				//	check if an attribute has already been added to the set
				match &mut att_val_option {
					Some(inner_value) => {
						inner_value
							.iter_mut()
							.for_each(|(k,v)|{v.id = var_ref.attribute_id.clone()});
						
					},
					None => {
						// panic!("something went wrong while drilling for attributes (Expression error)");
						// println!("do nothing");
					},
				}
				//	explore previous results and give them this id if they have no id 
				for (u,av) in &mut result {
					match & id {
						Some(inner_value) => {
							if av.id.trim() == "" {
								av.id = inner_value.clone();
							}
						},
						None => {
							// println!("do nothing");
						},
					};
				}
			} else if let Some(var_ref)   = (*expression.0).as_any_mut().downcast_mut::<AttributeValue>() {
				let mut att_value_with_id = var_ref.drill_for_attributes_values_and_transform_values();
				
				match & id {
					Some(inner_value) => {
						att_value_with_id
							.iter_mut()
							.for_each(|(k,v)|{v.id = inner_value.clone()});	
					},
					None => {
						// println!("do nothing");
					},
				};
				att_val_option = Some(att_value_with_id);

			}else {
				// panic!("No downcast found for {:#?}",self)
				//	just call the drill and get attributes
				let r = expression.0.drill_for_attributes_values_and_transform_values();
				result.extend(r);
			}
		}
		match att_val_option {
			Some(att_val) => {result.extend(att_val);},
			None => {},
		};

		result
	}
}

impl Drill<AttributeValueWithId> for TargetType {
	fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
		let mut result = HashMap::new();
		for content in &mut self.content {
			let atts = content.drill_for_attributes_values_and_transform_values();
			result.extend(atts);
		}
		result
	}
}

impl Drill<AttributeValueWithId> for TargetTypeContent {
	fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
		self.any_of.drill_for_attributes_values_and_transform_values()
	}
}

impl Drill<AttributeValueWithId> for AnyOfType {
	fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
		let mut result = HashMap::new();
		for content in &mut self.content {
			let atts = content.drill_for_attributes_values_and_transform_values();
			result.extend(atts);
		}
		result	
	}
}

impl Drill<AttributeValueWithId> for AnyOfTypeContent {
	fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
		self.all_of.drill_for_attributes_values_and_transform_values()
	}
}

impl Drill<AttributeValueWithId> for AllOfType {
	fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
		let mut result = HashMap::new();
		for content in &mut self.content {
			let atts = content.drill_for_attributes_values_and_transform_values();
			result.extend(atts);
		}
		result	
	}
}

impl Drill<AttributeValueWithId> for AllOfTypeContent {
	fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
		self.match_.drill_for_attributes_values_and_transform_values()
	}
}

impl Drill<AttributeValueWithId> for MatchType {
	fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
		let mut result = HashMap::new();

		let id = Uuid::new_v4();
		let mut att_copy = self.attribute_value.get_copy();
		let s = att_copy.text.and_then(|inner|Some(xsd_parser::xml::Text::new(inner.0.trim().to_string())));

		att_copy.text = s;
		let tmp = AttributeValueWithId {
			rule_id	: "".to_string(),
			att_val: att_copy,
			id: match &self.content_47 {
				MatchContent47Type::AttributeDesignator(d) => d.attribute_id.clone(),
				MatchContent47Type::AttributeSelector(d) => d.context_selector_id.as_ref().unwrap().clone(),
			}
		};
		result.insert(id,tmp);
		self.attribute_value.text = Some(Text::new(id.to_string()));

		result
	}
}