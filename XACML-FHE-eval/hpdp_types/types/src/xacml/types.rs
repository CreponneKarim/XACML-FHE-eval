use core::fmt::Debug;
use std::{borrow::Cow, collections::HashMap};
use rkyv::with::Skip;
use uuid::Uuid;
use xsd_parser::{
    models::schema::Namespace,
    quick_xml::{
        BoxedSerializer, DeserializeBytes, DeserializeReader, Error, ErrorKind, RawByteStr,
        SerializeBytes, WithBoxedSerializer, WithDeserializer, WithSerializer,
    },
    xml::Text,
    AsAny,
};

use std::collections::HashSet;

pub const SUBJECT_CATEGORY: &str =
    "urn:oasis:names:tc:xacml:1.0:subject-category:access-subject";

pub const RESOURCE_CATEGORY: &str =
    "urn:oasis:names:tc:xacml:3.0:attribute-category:resource";

pub const ACTION_CATEGORY: &str =
    "urn:oasis:names:tc:xacml:3.0:attribute-category:action";

pub const ENVIRONMENT_CATEGORY: &str =
    "urn:oasis:names:tc:xacml:3.0:attribute-category:environment";

#[derive(Clone, Debug, Default,rkyv::Archive,rkyv::Deserialize, rkyv::Serialize,PartialEq)]
pub struct CategorizedAttributeDesignators {
    pub subject: Vec<AttributeDesignatorType>,
    pub resource: Vec<AttributeDesignatorType>,
    pub action: Vec<AttributeDesignatorType>,
    pub environment: Vec<AttributeDesignatorType>,

    pub subject_ids: HashSet<String>,
    pub resource_ids: HashSet<String>,
    pub action_ids: HashSet<String>,
    pub environment_ids: HashSet<String>,
}

impl CategorizedAttributeDesignators {
    pub fn insert(&mut self, designator: &AttributeDesignatorType) {
        match designator.category.as_str() {
            RESOURCE_CATEGORY => {
                if self
                    .resource_ids
                    .insert(designator.attribute_id.clone())
                {
                    self.resource.push(designator.clone());
                }
            }

            ACTION_CATEGORY => {
                if self
                    .action_ids
                    .insert(designator.attribute_id.clone())
                {
                    self.action.push(designator.clone());
                }
            }

            ENVIRONMENT_CATEGORY => {
                if self
                    .environment_ids
                    .insert(designator.attribute_id.clone())
                {
                    self.environment.push(designator.clone());
                }
            }

            // This includes access-subject and the other XACML
            // subject categories.
            category if is_subject_category(category) => {
                if self
                    .subject_ids
                    .insert(designator.attribute_id.clone())
                {
                    self.subject.push(designator.clone());
                }
            }

            // Ignore custom or unsupported categories.
            _ => {}
        }
    }
}

fn is_subject_category(category: &str) -> bool {
    category == SUBJECT_CATEGORY
        || category.starts_with(
            "urn:oasis:names:tc:xacml:1.0:subject-category:",
        )
        || category.starts_with(
            "urn:oasis:names:tc:xacml:3.0:attribute-category:subject",
        )
}
pub trait ExtractAttributeDesignators {
    fn collect_attribute_designators(
        &self,
        output: &mut CategorizedAttributeDesignators,
    );

    fn extract_attribute_designators(
        &self,
    ) -> CategorizedAttributeDesignators {
        let mut output = CategorizedAttributeDesignators::default();
        self.collect_attribute_designators(&mut output);
        output
    }
}

impl ExtractAttributeDesignators for AttributeDesignatorType {
    fn collect_attribute_designators(
        &self,
        output: &mut CategorizedAttributeDesignators,
    ) {
        output.insert(self);
    }
}
impl ExtractAttributeDesignators for ApplyType {
    fn collect_attribute_designators(
        &self,
        output: &mut CategorizedAttributeDesignators,
    ) {
        for expression in &self.expression {
            expression.collect_attribute_designators(output);
        }
    }
}
impl ExtractAttributeDesignators for Expression {
    fn collect_attribute_designators(
        &self,
        output: &mut CategorizedAttributeDesignators,
    ) {
        self.0.collect_attribute_designators(output);
    }
}
impl ExtractAttributeDesignators for AttributeSelectorType {
    fn collect_attribute_designators(
        &self,
        _output: &mut CategorizedAttributeDesignators,
    ) {
    }
}

impl ExtractAttributeDesignators for AttributeValueType {
    fn collect_attribute_designators(
        &self,
        _output: &mut CategorizedAttributeDesignators,
    ) {
    }
}

impl ExtractAttributeDesignators for FunctionType {
    fn collect_attribute_designators(
        &self,
        _output: &mut CategorizedAttributeDesignators,
    ) {
    }
}

impl ExtractAttributeDesignators for VariableReferenceType {
    fn collect_attribute_designators(
        &self,
        _output: &mut CategorizedAttributeDesignators,
    ) {
    }
}

impl ExtractAttributeDesignators for MatchType {
    fn collect_attribute_designators(
        &self,
        output: &mut CategorizedAttributeDesignators,
    ) {
        if let MatchContent47Type::AttributeDesignator(designator) =
            &self.content_47
        {
            designator.collect_attribute_designators(output);
        }
    }
}

impl ExtractAttributeDesignators for AllOfType {
    fn collect_attribute_designators(
        &self,
        output: &mut CategorizedAttributeDesignators,
    ) {
        for content in &self.content {
            content
                .match_
                .collect_attribute_designators(output);
        }
    }
}

impl ExtractAttributeDesignators for AnyOfType {
    fn collect_attribute_designators(
        &self,
        output: &mut CategorizedAttributeDesignators,
    ) {
        for content in &self.content {
            content
                .all_of
                .collect_attribute_designators(output);
        }
    }
}

impl ExtractAttributeDesignators for TargetType {
    fn collect_attribute_designators(
        &self,
        output: &mut CategorizedAttributeDesignators,
    ) {
        for content in &self.content {
            content
                .any_of
                .collect_attribute_designators(output);
        }
    }
}

impl ExtractAttributeDesignators for ConditionType {
    fn collect_attribute_designators(
        &self,
        output: &mut CategorizedAttributeDesignators,
    ) {
        self.expression
            .collect_attribute_designators(output);
    }
}

impl ExtractAttributeDesignators for RuleType {
    fn collect_attribute_designators(
        &self,
        output: &mut CategorizedAttributeDesignators,
    ) {
        if let Some(target) = &self.target {
            target.collect_attribute_designators(output);
        }

        if let Some(condition) = &self.condition {
            condition.collect_attribute_designators(output);
        }
    }
}

impl ExtractAttributeDesignators for PolicyType {
    fn collect_attribute_designators(
        &self,
        output: &mut CategorizedAttributeDesignators,
    ) {
        self.target.collect_attribute_designators(output);

        for content in &self.content_41 {
            match content {
                PolicyContent41Type::Rule(rule) => {
                    rule.collect_attribute_designators(output);
                }

                PolicyContent41Type::VariableDefinition(variable) => {
                    variable
                        .expression
                        .collect_attribute_designators(output);
                }

                PolicyContent41Type::CombinerParameters(_)
                | PolicyContent41Type::RuleCombinerParameters(_) => {}
            }
        }
    }
}

impl ExtractAttributeDesignators for PolicySetType {
    fn collect_attribute_designators(
        &self,
        output: &mut CategorizedAttributeDesignators,
    ) {
        self.target.collect_attribute_designators(output);

        for content in &self.content_31 {
            match content {
                PolicySetContent31Type::Policy(policy) => {
                    policy.collect_attribute_designators(output);
                }

                PolicySetContent31Type::PolicySet(policy_set) => {
                    policy_set.collect_attribute_designators(output);
                }

                PolicySetContent31Type::PolicySetIdReference(_)
                | PolicySetContent31Type::PolicyIdReference(_)
                | PolicySetContent31Type::CombinerParameters(_)
                | PolicySetContent31Type::PolicyCombinerParameters(_)
                | PolicySetContent31Type::PolicySetCombinerParameters(_) => {}
            }
        }
    }
}

pub const NS_XS: Namespace = Namespace::new_const(b"http://www.w3.org/2001/XMLSchema");
pub const NS_XML: Namespace = Namespace::new_const(b"http://www.w3.org/XML/1998/namespace");
pub const NS_XACML: Namespace =
    Namespace::new_const(b"urn:oasis:names:tc:xacml:3.0:core:schema:wd-17");
#[derive(Debug, Default)]
pub struct EntitiesType(pub Vec<String>);
impl SerializeBytes for EntitiesType {
    fn serialize_bytes(&self) -> std::result::Result<Option<Cow<'_, str>>, Error> {
        if self.0.is_empty() {
            return Ok(None);
        }
        let mut data = String::new();
        for item in &self.0 {
            if let Some(bytes) = item.serialize_bytes()? {
                if !data.is_empty() {
                    data.push(' ');
                }
                data.push_str(&bytes);
            }
        }
        Ok(Some(Cow::Owned(data)))
    }
}
impl DeserializeBytes for EntitiesType {
    fn deserialize_bytes<R>(reader: &R, bytes: &[u8]) -> std::result::Result<Self, Error>
    where
        R: DeserializeReader,
    {
        Ok(Self(
            bytes
                .split(|b| *b == b' ' || *b == b'|' || *b == b',' || *b == b';')
                .map(|bytes| String::deserialize_bytes(reader, bytes))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}
#[derive(Debug, Default)]
pub struct EntityType(pub Vec<String>);
impl SerializeBytes for EntityType {
    fn serialize_bytes(&self) -> std::result::Result<Option<Cow<'_, str>>, Error> {
        if self.0.is_empty() {
            return Ok(None);
        }
        let mut data = String::new();
        for item in &self.0 {
            if let Some(bytes) = item.serialize_bytes()? {
                if !data.is_empty() {
                    data.push(' ');
                }
                data.push_str(&bytes);
            }
        }
        Ok(Some(Cow::Owned(data)))
    }
}
impl DeserializeBytes for EntityType {
    fn deserialize_bytes<R>(reader: &R, bytes: &[u8]) -> std::result::Result<Self, Error>
    where
        R: DeserializeReader,
    {
        Ok(Self(
            bytes
                .split(|b| *b == b' ' || *b == b'|' || *b == b',' || *b == b';')
                .map(|bytes| String::deserialize_bytes(reader, bytes))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}
pub type IdType = String;
pub type IdrefType = String;
#[derive(Debug, Default)]
pub struct IdrefsType(pub Vec<String>);
impl SerializeBytes for IdrefsType {
    fn serialize_bytes(&self) -> std::result::Result<Option<Cow<'_, str>>, Error> {
        if self.0.is_empty() {
            return Ok(None);
        }
        let mut data = String::new();
        for item in &self.0 {
            if let Some(bytes) = item.serialize_bytes()? {
                if !data.is_empty() {
                    data.push(' ');
                }
                data.push_str(&bytes);
            }
        }
        Ok(Some(Cow::Owned(data)))
    }
}
impl DeserializeBytes for IdrefsType {
    fn deserialize_bytes<R>(reader: &R, bytes: &[u8]) -> std::result::Result<Self, Error>
    where
        R: DeserializeReader,
    {
        Ok(Self(
            bytes
                .split(|b| *b == b' ' || *b == b'|' || *b == b',' || *b == b';')
                .map(|bytes| String::deserialize_bytes(reader, bytes))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}
pub type NcNameType = String;
pub type NmtokenType = String;
#[derive(Debug, Default)]
pub struct NmtokensType(pub Vec<String>);
impl SerializeBytes for NmtokensType {
    fn serialize_bytes(&self) -> std::result::Result<Option<Cow<'_, str>>, Error> {
        if self.0.is_empty() {
            return Ok(None);
        }
        let mut data = String::new();
        for item in &self.0 {
            if let Some(bytes) = item.serialize_bytes()? {
                if !data.is_empty() {
                    data.push(' ');
                }
                data.push_str(&bytes);
            }
        }
        Ok(Some(Cow::Owned(data)))
    }
}
impl DeserializeBytes for NmtokensType {
    fn deserialize_bytes<R>(reader: &R, bytes: &[u8]) -> std::result::Result<Self, Error>
    where
        R: DeserializeReader,
    {
        Ok(Self(
            bytes
                .split(|b| *b == b' ' || *b == b'|' || *b == b',' || *b == b';')
                .map(|bytes| String::deserialize_bytes(reader, bytes))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}
pub type NotationType = String;
pub type NameType = String;
pub type QNameType = String;
pub type AnySimpleType = String;
#[derive(Debug)]
pub struct AnyType;
impl WithSerializer for AnyType {
    type Serializer<'x> = quick_xml_serialize::AnyTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::AnyTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::AnyTypeSerializerState::Init__),
            name: name.unwrap_or("anyType"),
            is_root,
        })
    }
}
impl WithDeserializer for AnyType {
    type Deserializer = quick_xml_deserialize::AnyTypeDeserializer;
}
pub type AnyUriType = String;
pub type Base64BinaryType = String;
pub type BooleanType = bool;
pub type ByteType = i8;
pub type DateType = String;
pub type DateTimeType = String;
pub type DecimalType = f64;
pub type DoubleType = f64;
pub type DurationType = String;
pub type FloatType = f32;
pub type GDayType = String;
pub type GMonthType = String;
pub type GMonthDayType = String;
pub type GYearType = String;
pub type GYearMonthType = String;
pub type HexBinaryType = String;
pub type IntType = i32;
pub type IntegerType = i32;
pub type LanguageType = String;
pub type LongType = i64;
pub type NegativeIntegerType = isize;
pub type NonNegativeIntegerType = usize;
pub type NonPositiveIntegerType = isize;
pub type NormalizedStringType = String;
pub type PositiveIntegerType = usize;
pub type ShortType = i16;
pub type StringType = String;
pub type TimeType = String;
pub type TokenType = String;
pub type UnsignedByteType = u8;
pub type UnsignedIntType = u32;
pub type UnsignedLongType = u64;
pub type UnsignedShortType = u16;
pub type Base = AnyUriType;
pub type Id = IdType;
pub type Lang = LanguageType;
#[derive(Debug)]
pub enum Space {
    Default,
    Preserve,
}
impl SerializeBytes for Space {
    fn serialize_bytes(&self) -> std::result::Result<Option<Cow<'_, str>>, Error> {
        match self {
            Self::Default => Ok(Some(Cow::Borrowed("default"))),
            Self::Preserve => Ok(Some(Cow::Borrowed("preserve"))),
        }
    }
}
impl DeserializeBytes for Space {
    fn deserialize_bytes<R>(reader: &R, bytes: &[u8]) -> std::result::Result<Self, Error>
    where
        R: DeserializeReader,
    {
        match bytes {
            b"default" => Ok(Self::Default),
            b"preserve" => Ok(Self::Preserve),
            x => Err(reader.map_error(ErrorKind::UnknownOrInvalidValue(RawByteStr::from_slice(x)))),
        }
    }
}
pub type Advice = AdviceType;
pub type AdviceExpression = AdviceExpressionType;
#[derive(Debug)]
pub struct AdviceExpressionType {
    pub advice_id: AnyUriType,
    pub applies_to: EffectType,
    pub attribute_assignment_expression: Vec<AttributeAssignmentExpression>,
}
impl WithSerializer for AdviceExpressionType {
    type Serializer<'x> = quick_xml_serialize::AdviceExpressionTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::AdviceExpressionTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::AdviceExpressionTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:AdviceExpressionType"),
            is_root,
        })
    }
}
impl WithDeserializer for AdviceExpressionType {
    type Deserializer = quick_xml_deserialize::AdviceExpressionTypeDeserializer;
}
pub type AdviceExpressions = AdviceExpressionsType;
#[derive(Debug)]
pub struct AdviceExpressionsType {
    pub advice_expression: Vec<AdviceExpression>,
}
impl WithSerializer for AdviceExpressionsType {
    type Serializer<'x> = quick_xml_serialize::AdviceExpressionsTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::AdviceExpressionsTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::AdviceExpressionsTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:AdviceExpressionsType"),
            is_root,
        })
    }
}
impl WithDeserializer for AdviceExpressionsType {
    type Deserializer = quick_xml_deserialize::AdviceExpressionsTypeDeserializer;
}
#[derive(Debug)]
pub struct AdviceType {
    pub advice_id: AnyUriType,
    pub attribute_assignment: Vec<AttributeAssignment>,
}
impl WithSerializer for AdviceType {
    type Serializer<'x> = quick_xml_serialize::AdviceTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::AdviceTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::AdviceTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:AdviceType"),
            is_root,
        })
    }
}
impl WithDeserializer for AdviceType {
    type Deserializer = quick_xml_deserialize::AdviceTypeDeserializer;
}
pub type AllOf = AllOfType;
#[derive(Debug)]
pub struct AllOfType {
    pub content: Vec<AllOfTypeContent>,
}
#[derive(Debug)]
pub struct AllOfTypeContent {
    pub match_: Match,
}
impl WithSerializer for AllOfType {
    type Serializer<'x> = quick_xml_serialize::AllOfTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::AllOfTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::AllOfTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:AllOfType"),
            is_root,
        })
    }
}
impl WithSerializer for AllOfTypeContent {
    type Serializer<'x> = quick_xml_serialize::AllOfTypeContentSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        let _name = name;
        let _is_root = is_root;
        Ok(quick_xml_serialize::AllOfTypeContentSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::AllOfTypeContentSerializerState::Init__),
        })
    }
}
impl WithDeserializer for AllOfType {
    type Deserializer = quick_xml_deserialize::AllOfTypeDeserializer;
}
impl WithDeserializer for AllOfTypeContent {
    type Deserializer = quick_xml_deserialize::AllOfTypeContentDeserializer;
}
pub type AnyOf = AnyOfType;
#[derive(Debug)]
pub struct AnyOfType {
    pub content: Vec<AnyOfTypeContent>,
}
#[derive(Debug)]
pub struct AnyOfTypeContent {
    pub all_of: AllOf,
}
impl WithSerializer for AnyOfType {
    type Serializer<'x> = quick_xml_serialize::AnyOfTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::AnyOfTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::AnyOfTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:AnyOfType"),
            is_root,
        })
    }
}
impl WithSerializer for AnyOfTypeContent {
    type Serializer<'x> = quick_xml_serialize::AnyOfTypeContentSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        let _name = name;
        let _is_root = is_root;
        Ok(quick_xml_serialize::AnyOfTypeContentSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::AnyOfTypeContentSerializerState::Init__),
        })
    }
}
impl WithDeserializer for AnyOfType {
    type Deserializer = quick_xml_deserialize::AnyOfTypeDeserializer;
}
impl WithDeserializer for AnyOfTypeContent {
    type Deserializer = quick_xml_deserialize::AnyOfTypeContentDeserializer;
}
pub type Apply = ApplyDyn;
impl ExpressionTrait for Apply {}
pub type ApplyDyn = ApplyType;
#[derive(Debug)]
pub struct ApplyType {
    pub function_id: AnyUriType,
    pub description: Option<Description>,
    pub expression: Vec<Expression>,
}

impl WithSerializer for ApplyType {
    type Serializer<'x> = quick_xml_serialize::ApplyTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::ApplyTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::ApplyTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:Apply"),
            is_root,
        })
    }
}
impl WithDeserializer for ApplyType {
    type Deserializer = quick_xml_deserialize::ApplyTypeDeserializer;
}
pub type AssociatedAdvice = AssociatedAdviceType;
#[derive(Debug)]
pub struct AssociatedAdviceType {
    pub advice: Vec<Advice>,
}
impl WithSerializer for AssociatedAdviceType {
    type Serializer<'x> = quick_xml_serialize::AssociatedAdviceTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::AssociatedAdviceTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::AssociatedAdviceTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:AssociatedAdviceType"),
            is_root,
        })
    }
}
impl WithDeserializer for AssociatedAdviceType {
    type Deserializer = quick_xml_deserialize::AssociatedAdviceTypeDeserializer;
}
pub type Attribute = AttributeType;
pub type AttributeAssignment = AttributeAssignmentType;
pub type AttributeAssignmentExpression = AttributeAssignmentExpressionType;
#[derive(Debug)]
pub struct AttributeAssignmentExpressionType {
    pub attribute_id: AnyUriType,
    pub category: Option<AnyUriType>,
    pub issuer: Option<StringType>,
    pub expression: Expression,
}
impl WithSerializer for AttributeAssignmentExpressionType {
    type Serializer<'x> = quick_xml_serialize::AttributeAssignmentExpressionTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(
            quick_xml_serialize::AttributeAssignmentExpressionTypeSerializer {
                value: self,
                state: Box::new(
                    quick_xml_serialize::AttributeAssignmentExpressionTypeSerializerState::Init__,
                ),
                name: name.unwrap_or("xacml:AttributeAssignmentExpressionType"),
                is_root,
            },
        )
    }
}
impl WithDeserializer for AttributeAssignmentExpressionType {
    type Deserializer = quick_xml_deserialize::AttributeAssignmentExpressionTypeDeserializer;
}
#[derive(Debug)]
pub struct AttributeAssignmentType {
    pub data_type: AnyUriType,
    pub attribute_id: AnyUriType,
    pub category: Option<AnyUriType>,
    pub issuer: Option<StringType>,
    pub text: Option<Text>,
}
impl WithSerializer for AttributeAssignmentType {
    type Serializer<'x> = quick_xml_serialize::AttributeAssignmentTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::AttributeAssignmentTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::AttributeAssignmentTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:AttributeAssignmentType"),
            is_root,
        })
    }
}
impl WithDeserializer for AttributeAssignmentType {
    type Deserializer = quick_xml_deserialize::AttributeAssignmentTypeDeserializer;
}
pub type AttributeDesignator = AttributeDesignatorDyn;
impl ExpressionTrait for AttributeDesignator {}

impl Drill<AttributeValueWithId> for AttributeDesignator {
    fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
        HashMap::new()
    }
}
pub type AttributeDesignatorDyn = AttributeDesignatorType;
#[derive(Debug,Clone,serde::Serialize, serde::Deserialize, rkyv::Archive,rkyv::Deserialize, rkyv::Serialize)]
pub struct AttributeDesignatorType {
    pub category: AnyUriType,
    pub attribute_id: AnyUriType,
    pub data_type: AnyUriType,
    pub issuer: Option<StringType>,
    pub must_be_present: BooleanType,
}
impl WithSerializer for AttributeDesignatorType {
    type Serializer<'x> = quick_xml_serialize::AttributeDesignatorTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::AttributeDesignatorTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::AttributeDesignatorTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:AttributeDesignator"),
            is_root,
        })
    }
}
impl WithDeserializer for AttributeDesignatorType {
    type Deserializer = quick_xml_deserialize::AttributeDesignatorTypeDeserializer;
}
pub type AttributeSelector = AttributeSelectorDyn;
impl Drill<AttributeValueWithId> for AttributeSelector {
    fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
        HashMap::new()
    }
}
impl ExpressionTrait for AttributeSelector {}
pub type AttributeSelectorDyn = AttributeSelectorType;
#[derive(Debug,Clone)]
pub struct AttributeSelectorType {
    pub category: AnyUriType,
    pub context_selector_id: Option<AnyUriType>,
    pub path: StringType,
    pub data_type: AnyUriType,
    pub must_be_present: BooleanType,
}
impl WithSerializer for AttributeSelectorType {
    type Serializer<'x> = quick_xml_serialize::AttributeSelectorTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::AttributeSelectorTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::AttributeSelectorTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:AttributeSelectorType"),
            is_root,
        })
    }
}
impl WithDeserializer for AttributeSelectorType {
    type Deserializer = quick_xml_deserialize::AttributeSelectorTypeDeserializer;
}
#[derive(Debug,Clone)]
pub struct AttributeType {
    pub attribute_id: AnyUriType,
    pub issuer: Option<StringType>,
    pub include_in_result: BooleanType,
    pub attribute_value: Vec<AttributeValue>,
}
impl WithSerializer for AttributeType {
    type Serializer<'x> = quick_xml_serialize::AttributeTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::AttributeTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::AttributeTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:AttributeType"),
            is_root,
        })
    }
}
impl WithDeserializer for AttributeType {
    type Deserializer = quick_xml_deserialize::AttributeTypeDeserializer;
}
pub type AttributeValue = AttributeValueDyn;
impl Drill<AttributeValueWithId> for AttributeValue {
    fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
        let k =  Uuid::new_v4();
        let v = self.get_copy();
        let tmp =AttributeValueWithId{
            rule_id:"".to_string(),
            att_val:v,
            id: "".to_string()
        };
        let mut result = HashMap::new();
        result.insert(k.clone(), tmp);
        self.text = Some(Text::new(k.to_string()));
        result
    }
}
impl ExpressionTrait for AttributeValue {}
pub type AttributeValueDyn = AttributeValueType;
#[derive(Debug,Clone,PartialEq,Eq,Hash)]
pub struct AttributeValueType {
    pub data_type: AnyUriType,
    pub text: Option<Text>,
}
impl WithSerializer for AttributeValueType {
    type Serializer<'x> = quick_xml_serialize::AttributeValueTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::AttributeValueTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::AttributeValueTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:AttributeValue"),
            is_root,
        })
    }
}
impl WithDeserializer for AttributeValueType {
    type Deserializer = quick_xml_deserialize::AttributeValueTypeDeserializer;
}
pub type Attributes = AttributesType;
pub type AttributesReference = AttributesReferenceType;
#[derive(Debug,Clone)]
pub struct AttributesReferenceType {
    pub reference_id: IdrefType,
}
impl WithSerializer for AttributesReferenceType {
    type Serializer<'x> = quick_xml_serialize::AttributesReferenceTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::AttributesReferenceTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::AttributesReferenceTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:AttributesReferenceType"),
            is_root,
        })
    }
}
impl WithDeserializer for AttributesReferenceType {
    type Deserializer = quick_xml_deserialize::AttributesReferenceTypeDeserializer;
}
#[derive(Debug,Clone)]
pub struct AttributesType {
    pub category: AnyUriType,
    pub id: Option<Id>,
    pub content: Option<Content>,
    pub attribute: Vec<Attribute>,
}
impl WithSerializer for AttributesType {
    type Serializer<'x> = quick_xml_serialize::AttributesTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::AttributesTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::AttributesTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:AttributesType"),
            is_root,
        })
    }
}
impl WithDeserializer for AttributesType {
    type Deserializer = quick_xml_deserialize::AttributesTypeDeserializer;
}
pub type CombinerParameter = CombinerParameterType;
#[derive(Debug)]
pub struct CombinerParameterType {
    pub parameter_name: StringType,
    pub attribute_value: AttributeValue,
}
impl WithSerializer for CombinerParameterType {
    type Serializer<'x> = quick_xml_serialize::CombinerParameterTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::CombinerParameterTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::CombinerParameterTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:CombinerParameterType"),
            is_root,
        })
    }
}
impl WithDeserializer for CombinerParameterType {
    type Deserializer = quick_xml_deserialize::CombinerParameterTypeDeserializer;
}
pub type CombinerParameters = CombinerParametersType;
#[derive(Debug)]
pub struct CombinerParametersType {
    pub combiner_parameter: Vec<CombinerParameter>,
}
impl WithSerializer for CombinerParametersType {
    type Serializer<'x> = quick_xml_serialize::CombinerParametersTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::CombinerParametersTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::CombinerParametersTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:CombinerParametersType"),
            is_root,
        })
    }
}
impl WithDeserializer for CombinerParametersType {
    type Deserializer = quick_xml_deserialize::CombinerParametersTypeDeserializer;
}
pub type Condition = ConditionType;
#[derive(Debug)]
pub struct ConditionType {
    pub expression: Expression,
}
impl WithSerializer for ConditionType {
    type Serializer<'x> = quick_xml_serialize::ConditionTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::ConditionTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::ConditionTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:ConditionType"),
            is_root,
        })
    }
}
impl WithDeserializer for ConditionType {
    type Deserializer = quick_xml_deserialize::ConditionTypeDeserializer;
}
pub type Content = ContentType;
#[derive(Debug,Clone)]
pub struct ContentType {
    pub text_before: Option<Text>,
    pub text_after_any_6: Option<Text>,
}
impl WithSerializer for ContentType {
    type Serializer<'x> = quick_xml_serialize::ContentTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::ContentTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::ContentTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:ContentType"),
            is_root,
        })
    }
}
impl WithDeserializer for ContentType {
    type Deserializer = quick_xml_deserialize::ContentTypeDeserializer;
}
pub type Decision = DecisionType;
#[derive(Debug)]
pub enum DecisionType {
    Permit,
    Deny,
    Indeterminate,
    NotApplicable,
}
impl SerializeBytes for DecisionType {
    fn serialize_bytes(&self) -> std::result::Result<Option<Cow<'_, str>>, Error> {
        match self {
            Self::Permit => Ok(Some(Cow::Borrowed("Permit"))),
            Self::Deny => Ok(Some(Cow::Borrowed("Deny"))),
            Self::Indeterminate => Ok(Some(Cow::Borrowed("Indeterminate"))),
            Self::NotApplicable => Ok(Some(Cow::Borrowed("NotApplicable"))),
        }
    }
}
impl DeserializeBytes for DecisionType {
    fn deserialize_bytes<R>(reader: &R, bytes: &[u8]) -> std::result::Result<Self, Error>
    where
        R: DeserializeReader,
    {
        match bytes {
            b"Permit" => Ok(Self::Permit),
            b"Deny" => Ok(Self::Deny),
            b"Indeterminate" => Ok(Self::Indeterminate),
            b"NotApplicable" => Ok(Self::NotApplicable),
            x => Err(reader.map_error(ErrorKind::UnknownOrInvalidValue(RawByteStr::from_slice(x)))),
        }
    }
}
#[derive(Debug,rkyv::Archive, rkyv::Serialize,rkyv::Deserialize)]
pub struct DefaultsType {
    pub content_39: DefaultsContent39Type,
}
impl WithSerializer for DefaultsType {
    type Serializer<'x> = quick_xml_serialize::DefaultsTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::DefaultsTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::DefaultsTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:DefaultsType"),
            is_root,
        })
    }
}
impl WithDeserializer for DefaultsType {
    type Deserializer = quick_xml_deserialize::DefaultsTypeDeserializer;
}
pub type Description = StringType;
#[derive(Debug,PartialEq,Eq)]
pub enum EffectType {
    Permit,
    Deny,
    EncryptedDecision(Uuid)
}
impl SerializeBytes for EffectType {
    fn serialize_bytes(&self) -> std::result::Result<Option<Cow<'_, str>>, Error> {
        match self {
            Self::Permit => Ok(Some(Cow::Borrowed("Permit"))),
            Self::Deny => Ok(Some(Cow::Borrowed("Deny"))),
            Self::EncryptedDecision(d)=> Ok(Some(Cow::Owned(d.to_string())))
        }
    }
}
impl DeserializeBytes for EffectType {
    fn deserialize_bytes<R>(
        reader: &R,
        bytes: &[u8],
    ) -> Result<Self, Error>
    where
        R: DeserializeReader,
    {
        match bytes {
            b"Permit" => Ok(Self::Permit),
            b"Deny" => Ok(Self::Deny),

            bytes => {
                let text = std::str::from_utf8(bytes)
                    .map_err(|_| {
                        reader.map_error(
                            ErrorKind::UnknownOrInvalidValue(
                                RawByteStr::from_slice(bytes),
                            ),
                        )
                    })?;

                let uuid = Uuid::parse_str(text)
                    .map_err(|_| {
                        reader.map_error(
                            ErrorKind::UnknownOrInvalidValue(
                                RawByteStr::from_slice(bytes),
                            ),
                        )
                    })?;

                Ok(Self::EncryptedDecision(uuid))
            }
        }
    }
}
#[derive(Debug)]
pub struct Expression(pub Box<dyn ExpressionTrait >);

#[derive(Debug,Clone)]
pub struct AttributeValueWithId {
    pub rule_id: String,
	pub att_val: AttributeValueType,
	pub id: String
}

pub trait Drill<T>
{
    //AttributeValueWithId
	fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,T>;
}

pub trait ExpressionTrait: 
	Debug + 
	AsAny + 
	WithBoxedSerializer + 
	Drill<AttributeValueWithId> +
	ExtractAttributeDesignators +
	Send{}
impl WithSerializer for Expression {
    type Serializer<'x> = BoxedSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        let _name = name;
        self.0.serializer(None, is_root)
    }
}
impl WithDeserializer for Expression {
    type Deserializer = quick_xml_deserialize::ExpressionDeserializer;
}
#[derive(Debug)]
pub struct ExpressionType;
impl WithSerializer for ExpressionType {
    type Serializer<'x> = quick_xml_serialize::ExpressionTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::ExpressionTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::ExpressionTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:ExpressionType"),
            is_root,
        })
    }
}
impl WithDeserializer for ExpressionType {
    type Deserializer = quick_xml_deserialize::ExpressionTypeDeserializer;
}
pub type Function = FunctionDyn;
impl Drill<AttributeValueWithId> for Function {
    fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
        HashMap::new()
    }
}
impl ExpressionTrait for Function {}
pub type FunctionDyn = FunctionType;
#[derive(Debug)]
pub struct FunctionType {
    pub function_id: AnyUriType,
}
impl WithSerializer for FunctionType {
    type Serializer<'x> = quick_xml_serialize::FunctionTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::FunctionTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::FunctionTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:FunctionType"),
            is_root,
        })
    }
}
impl WithDeserializer for FunctionType {
    type Deserializer = quick_xml_deserialize::FunctionTypeDeserializer;
}
#[derive(Debug)]
pub struct IdReferenceType {
    pub version: Option<VersionMatchType>,
    pub earliest_version: Option<VersionMatchType>,
    pub latest_version: Option<VersionMatchType>,
    pub content: AnyUriType,
}
impl WithSerializer for IdReferenceType {
    type Serializer<'x> = quick_xml_serialize::IdReferenceTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::IdReferenceTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::IdReferenceTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:IdReferenceType"),
            is_root,
        })
    }
}
impl WithDeserializer for IdReferenceType {
    type Deserializer = quick_xml_deserialize::IdReferenceTypeDeserializer;
}
pub type Match = MatchType;
#[derive(Debug,Clone)]
pub struct MatchType {
    pub match_id: AnyUriType,
    pub attribute_value: AttributeValue,
    pub content_47: MatchContent47Type,
}
impl WithSerializer for MatchType {
    type Serializer<'x> = quick_xml_serialize::MatchTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::MatchTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::MatchTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:MatchType"),
            is_root,
        })
    }
}
impl WithDeserializer for MatchType {
    type Deserializer = quick_xml_deserialize::MatchTypeDeserializer;
}
pub type MissingAttributeDetail = MissingAttributeDetailType;
#[derive(Debug)]
pub struct MissingAttributeDetailType {
    pub category: AnyUriType,
    pub attribute_id: AnyUriType,
    pub data_type: AnyUriType,
    pub issuer: Option<StringType>,
    pub attribute_value: Vec<AttributeValue>,
}
impl WithSerializer for MissingAttributeDetailType {
    type Serializer<'x> = quick_xml_serialize::MissingAttributeDetailTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::MissingAttributeDetailTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::MissingAttributeDetailTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:MissingAttributeDetailType"),
            is_root,
        })
    }
}
impl WithDeserializer for MissingAttributeDetailType {
    type Deserializer = quick_xml_deserialize::MissingAttributeDetailTypeDeserializer;
}
pub type MultiRequests = MultiRequestsType;
#[derive(Debug,Clone)]
pub struct MultiRequestsType {
    pub request_reference: Vec<RequestReference>,
}
impl WithSerializer for MultiRequestsType {
    type Serializer<'x> = quick_xml_serialize::MultiRequestsTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::MultiRequestsTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::MultiRequestsTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:MultiRequestsType"),
            is_root,
        })
    }
}
impl WithDeserializer for MultiRequestsType {
    type Deserializer = quick_xml_deserialize::MultiRequestsTypeDeserializer;
}
pub type Obligation = ObligationType;
pub type ObligationExpression = ObligationExpressionType;
#[derive(Debug)]
pub struct ObligationExpressionType {
    pub obligation_id: AnyUriType,
    pub fulfill_on: EffectType,
    pub attribute_assignment_expression: Vec<AttributeAssignmentExpression>,
}
impl WithSerializer for ObligationExpressionType {
    type Serializer<'x> = quick_xml_serialize::ObligationExpressionTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::ObligationExpressionTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::ObligationExpressionTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:ObligationExpressionType"),
            is_root,
        })
    }
}
impl WithDeserializer for ObligationExpressionType {
    type Deserializer = quick_xml_deserialize::ObligationExpressionTypeDeserializer;
}
pub type ObligationExpressions = ObligationExpressionsType;
#[derive(Debug)]
pub struct ObligationExpressionsType {
    pub obligation_expression: Vec<ObligationExpression>,
}
impl WithSerializer for ObligationExpressionsType {
    type Serializer<'x> = quick_xml_serialize::ObligationExpressionsTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::ObligationExpressionsTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::ObligationExpressionsTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:ObligationExpressionsType"),
            is_root,
        })
    }
}
impl WithDeserializer for ObligationExpressionsType {
    type Deserializer = quick_xml_deserialize::ObligationExpressionsTypeDeserializer;
}
#[derive(Debug)]
pub struct ObligationType {
    pub obligation_id: AnyUriType,
    pub attribute_assignment: Vec<AttributeAssignment>,
}
impl WithSerializer for ObligationType {
    type Serializer<'x> = quick_xml_serialize::ObligationTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::ObligationTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::ObligationTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:ObligationType"),
            is_root,
        })
    }
}
impl WithDeserializer for ObligationType {
    type Deserializer = quick_xml_deserialize::ObligationTypeDeserializer;
}
pub type Obligations = ObligationsType;
#[derive(Debug)]
pub struct ObligationsType {
    pub obligation: Vec<Obligation>,
}
impl WithSerializer for ObligationsType {
    type Serializer<'x> = quick_xml_serialize::ObligationsTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::ObligationsTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::ObligationsTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:ObligationsType"),
            is_root,
        })
    }
}
impl WithDeserializer for ObligationsType {
    type Deserializer = quick_xml_deserialize::ObligationsTypeDeserializer;
}
pub type Policy = PolicyType;
pub type PolicyCombinerParameters = PolicyCombinerParametersType;
#[derive(Debug)]
pub struct PolicyCombinerParametersType {
    pub policy_id_ref: AnyUriType,
    pub combiner_parameter: Vec<CombinerParameter>,
}
impl WithSerializer for PolicyCombinerParametersType {
    type Serializer<'x> = quick_xml_serialize::PolicyCombinerParametersTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(
            quick_xml_serialize::PolicyCombinerParametersTypeSerializer {
                value: self,
                state: Box::new(
                    quick_xml_serialize::PolicyCombinerParametersTypeSerializerState::Init__,
                ),
                name: name.unwrap_or("xacml:PolicyCombinerParametersType"),
                is_root,
            },
        )
    }
}
impl WithDeserializer for PolicyCombinerParametersType {
    type Deserializer = quick_xml_deserialize::PolicyCombinerParametersTypeDeserializer;
}
pub type PolicyDefaults = DefaultsType;
pub type PolicyIdReference = IdReferenceType;
pub type PolicyIdentifierList = PolicyIdentifierListType;
#[derive(Debug)]
pub struct PolicyIdentifierListType {
    pub content: Vec<PolicyIdentifierListTypeContent>,
}
#[derive(Debug)]
pub enum PolicyIdentifierListTypeContent {
    PolicyIdReference(PolicyIdReference),
    PolicySetIdReference(PolicySetIdReference),
}
impl WithSerializer for PolicyIdentifierListType {
    type Serializer<'x> = quick_xml_serialize::PolicyIdentifierListTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::PolicyIdentifierListTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::PolicyIdentifierListTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:PolicyIdentifierListType"),
            is_root,
        })
    }
}
impl WithSerializer for PolicyIdentifierListTypeContent {
    type Serializer<'x> = quick_xml_serialize::PolicyIdentifierListTypeContentSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        let _name = name;
        let _is_root = is_root;
        Ok(
            quick_xml_serialize::PolicyIdentifierListTypeContentSerializer {
                value: self,
                state: Box::new(
                    quick_xml_serialize::PolicyIdentifierListTypeContentSerializerState::Init__,
                ),
            },
        )
    }
}
impl WithDeserializer for PolicyIdentifierListType {
    type Deserializer = quick_xml_deserialize::PolicyIdentifierListTypeDeserializer;
}
impl WithDeserializer for PolicyIdentifierListTypeContent {
    type Deserializer = quick_xml_deserialize::PolicyIdentifierListTypeContentDeserializer;
}
pub type PolicyIssuer = PolicyIssuerType;
#[derive(Debug)]
pub struct PolicyIssuerType {
    pub content: Option<Content>,
    pub attribute: Vec<Attribute>,
}
impl WithSerializer for PolicyIssuerType {
    type Serializer<'x> = quick_xml_serialize::PolicyIssuerTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::PolicyIssuerTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::PolicyIssuerTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:PolicyIssuerType"),
            is_root,
        })
    }
}
impl WithDeserializer for PolicyIssuerType {
    type Deserializer = quick_xml_deserialize::PolicyIssuerTypeDeserializer;
}
pub type PolicySet = PolicySetType;
pub type PolicySetCombinerParameters = PolicySetCombinerParametersType;
#[derive(Debug)]
pub struct PolicySetCombinerParametersType {
    pub policy_set_id_ref: AnyUriType,
    pub combiner_parameter: Vec<CombinerParameter>,
}
impl WithSerializer for PolicySetCombinerParametersType {
    type Serializer<'x> = quick_xml_serialize::PolicySetCombinerParametersTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(
            quick_xml_serialize::PolicySetCombinerParametersTypeSerializer {
                value: self,
                state: Box::new(
                    quick_xml_serialize::PolicySetCombinerParametersTypeSerializerState::Init__,
                ),
                name: name.unwrap_or("xacml:PolicySetCombinerParametersType"),
                is_root,
            },
        )
    }
}
impl WithDeserializer for PolicySetCombinerParametersType {
    type Deserializer = quick_xml_deserialize::PolicySetCombinerParametersTypeDeserializer;
}
pub type PolicySetDefaults = DefaultsType;
pub type PolicySetIdReference = IdReferenceType;
#[derive(Debug)]
pub struct PolicySetType {
    pub policy_set_id: AnyUriType,
    pub version: VersionType,
    pub policy_combining_alg_id: AnyUriType,
    pub max_delegation_depth: Option<IntegerType>,
    pub description: Option<Description>,
    pub policy_issuer: Option<PolicyIssuer>,
    pub policy_set_defaults: Option<PolicySetDefaults>,
    pub target: Target,
    pub content_31: Vec<PolicySetContent31Type>,
    pub obligation_expressions: Option<ObligationExpressions>,
    pub advice_expressions: Option<AdviceExpressions>,
}
impl WithSerializer for PolicySetType {
    type Serializer<'x> = quick_xml_serialize::PolicySetTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::PolicySetTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::PolicySetTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:PolicySetType"),
            is_root,
        })
    }
}
impl WithDeserializer for PolicySetType {
    type Deserializer = quick_xml_deserialize::PolicySetTypeDeserializer;
}
#[derive(Debug)]
pub struct PolicyType {
    pub policy_id: AnyUriType,
    pub version: VersionType,
    pub rule_combining_alg_id: AnyUriType,
    pub max_delegation_depth: Option<IntegerType>,
    pub description: Option<Description>,
    pub policy_issuer: Option<PolicyIssuer>,
    pub policy_defaults: Option<PolicyDefaults>,
    pub target: Target,
    pub content_41: Vec<PolicyContent41Type>,
    pub obligation_expressions: Option<ObligationExpressions>,
    pub advice_expressions: Option<AdviceExpressions>,
}
impl WithSerializer for PolicyType {
    type Serializer<'x> = quick_xml_serialize::PolicyTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::PolicyTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::PolicyTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:PolicyType"),
            is_root,
        })
    }
}
impl WithDeserializer for PolicyType {
    type Deserializer = quick_xml_deserialize::PolicyTypeDeserializer;
}
pub type Request = RequestType;
pub type RequestDefaults = RequestDefaultsType;
#[derive(Debug,Clone)]
pub struct RequestDefaultsType {
    pub content_3: RequestDefaultsContent3Type,
}
impl WithSerializer for RequestDefaultsType {
    type Serializer<'x> = quick_xml_serialize::RequestDefaultsTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::RequestDefaultsTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::RequestDefaultsTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:RequestDefaultsType"),
            is_root,
        })
    }
}
impl WithDeserializer for RequestDefaultsType {
    type Deserializer = quick_xml_deserialize::RequestDefaultsTypeDeserializer;
}
pub type RequestReference = RequestReferenceType;
#[derive(Debug,Clone)]
pub struct RequestReferenceType {
    pub attributes_reference: Vec<AttributesReference>,
}
impl WithSerializer for RequestReferenceType {
    type Serializer<'x> = quick_xml_serialize::RequestReferenceTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::RequestReferenceTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::RequestReferenceTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:RequestReferenceType"),
            is_root,
        })
    }
}
impl WithDeserializer for RequestReferenceType {
    type Deserializer = quick_xml_deserialize::RequestReferenceTypeDeserializer;
}
#[derive(Debug,Clone)]
pub struct RequestType {
    pub return_policy_id_list: BooleanType,
    pub combined_decision: BooleanType,
    pub request_defaults: Option<RequestDefaults>,
    pub attributes: Vec<Attributes>,
    pub multi_requests: Option<MultiRequests>,
}
impl WithSerializer for RequestType {
    type Serializer<'x> = quick_xml_serialize::RequestTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::RequestTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::RequestTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:RequestType"),
            is_root,
        })
    }
}
impl WithDeserializer for RequestType {
    type Deserializer = quick_xml_deserialize::RequestTypeDeserializer;
}
pub type Response = ResponseType;
#[derive(Debug)]
pub struct ResponseType {
    pub result: Vec<ResultT>,
}
impl WithSerializer for ResponseType {
    type Serializer<'x> = quick_xml_serialize::ResponseTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::ResponseTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::ResponseTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:ResponseType"),
            is_root,
        })
    }
}
impl WithDeserializer for ResponseType {
    type Deserializer = quick_xml_deserialize::ResponseTypeDeserializer;
}
pub type ResultT = ResultType;
#[derive(Debug)]
pub struct ResultType {
    pub decision: Decision,
    pub status: Option<Status>,
    pub obligations: Option<Obligations>,
    pub associated_advice: Option<AssociatedAdvice>,
    pub attributes: Vec<Attributes>,
    pub policy_identifier_list: Option<PolicyIdentifierList>,
}
impl WithSerializer for ResultType {
    type Serializer<'x> = quick_xml_serialize::ResultTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::ResultTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::ResultTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:ResultType"),
            is_root,
        })
    }
}
impl WithDeserializer for ResultType {
    type Deserializer = quick_xml_deserialize::ResultTypeDeserializer;
}
pub type Rule = RuleType;
pub type RuleCombinerParameters = RuleCombinerParametersType;
#[derive(Debug)]
pub struct RuleCombinerParametersType {
    pub rule_id_ref: StringType,
    pub combiner_parameter: Vec<CombinerParameter>,
}
impl WithSerializer for RuleCombinerParametersType {
    type Serializer<'x> = quick_xml_serialize::RuleCombinerParametersTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::RuleCombinerParametersTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::RuleCombinerParametersTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:RuleCombinerParametersType"),
            is_root,
        })
    }
}
impl WithDeserializer for RuleCombinerParametersType {
    type Deserializer = quick_xml_deserialize::RuleCombinerParametersTypeDeserializer;
}
#[derive(Debug)]
pub struct RuleType {
    pub rule_id: StringType,
    pub effect: EffectType,
    pub description: Option<Description>,
    pub target: Option<Target>,
    pub condition: Option<Condition>,
    pub obligation_expressions: Option<ObligationExpressions>,
    pub advice_expressions: Option<AdviceExpressions>,
}
impl WithSerializer for RuleType {
    type Serializer<'x> = quick_xml_serialize::RuleTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::RuleTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::RuleTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:RuleType"),
            is_root,
        })
    }
}
impl WithDeserializer for RuleType {
    type Deserializer = quick_xml_deserialize::RuleTypeDeserializer;
}
pub type Status = StatusType;
pub type StatusCode = StatusCodeType;
#[derive(Debug)]
pub struct StatusCodeType {
    pub value: AnyUriType,
    pub status_code: Option<Box<StatusCode>>,
}
impl WithSerializer for StatusCodeType {
    type Serializer<'x> = quick_xml_serialize::StatusCodeTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::StatusCodeTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::StatusCodeTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:StatusCodeType"),
            is_root,
        })
    }
}
impl WithDeserializer for StatusCodeType {
    type Deserializer = quick_xml_deserialize::StatusCodeTypeDeserializer;
}
pub type StatusDetail = StatusDetailType;
#[derive(Debug)]
pub struct StatusDetailType;
impl WithSerializer for StatusDetailType {
    type Serializer<'x> = quick_xml_serialize::StatusDetailTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::StatusDetailTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::StatusDetailTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:StatusDetailType"),
            is_root,
        })
    }
}
impl WithDeserializer for StatusDetailType {
    type Deserializer = quick_xml_deserialize::StatusDetailTypeDeserializer;
}
pub type StatusMessage = StringType;
#[derive(Debug)]
pub struct StatusType {
    pub status_code: StatusCode,
    pub status_message: Option<StatusMessage>,
    pub status_detail: Option<StatusDetail>,
}
impl WithSerializer for StatusType {
    type Serializer<'x> = quick_xml_serialize::StatusTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::StatusTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::StatusTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:StatusType"),
            is_root,
        })
    }
}
impl WithDeserializer for StatusType {
    type Deserializer = quick_xml_deserialize::StatusTypeDeserializer;
}
pub type Target = TargetType;
#[derive(Debug)]
pub struct TargetType {
    pub content: Vec<TargetTypeContent>,
}
#[derive(Debug)]
pub struct TargetTypeContent {
    pub any_of: AnyOf,
}
impl WithSerializer for TargetType {
    type Serializer<'x> = quick_xml_serialize::TargetTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::TargetTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::TargetTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:TargetType"),
            is_root,
        })
    }
}
impl WithSerializer for TargetTypeContent {
    type Serializer<'x> = quick_xml_serialize::TargetTypeContentSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        let _name = name;
        let _is_root = is_root;
        Ok(quick_xml_serialize::TargetTypeContentSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::TargetTypeContentSerializerState::Init__),
        })
    }
}
impl WithDeserializer for TargetType {
    type Deserializer = quick_xml_deserialize::TargetTypeDeserializer;
}
impl WithDeserializer for TargetTypeContent {
    type Deserializer = quick_xml_deserialize::TargetTypeContentDeserializer;
}
pub type VariableDefinition = VariableDefinitionType;
#[derive(Debug)]
pub struct VariableDefinitionType {
    pub variable_id: StringType,
    pub expression: Expression,
}
impl WithSerializer for VariableDefinitionType {
    type Serializer<'x> = quick_xml_serialize::VariableDefinitionTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::VariableDefinitionTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::VariableDefinitionTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:VariableDefinitionType"),
            is_root,
        })
    }
}
impl WithDeserializer for VariableDefinitionType {
    type Deserializer = quick_xml_deserialize::VariableDefinitionTypeDeserializer;
}
pub type VariableReference = VariableReferenceDyn;
impl Drill<AttributeValueWithId> for VariableReference {
    fn drill_for_attributes_values_and_transform_values(&mut self) -> HashMap<Uuid,AttributeValueWithId> {
        HashMap::new()
    }
}
impl ExpressionTrait for VariableReference {}
pub type VariableReferenceDyn = VariableReferenceType;
#[derive(Debug)]
pub struct VariableReferenceType {
    pub variable_id: StringType,
}
impl WithSerializer for VariableReferenceType {
    type Serializer<'x> = quick_xml_serialize::VariableReferenceTypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        Ok(quick_xml_serialize::VariableReferenceTypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::VariableReferenceTypeSerializerState::Init__),
            name: name.unwrap_or("xacml:VariableReferenceType"),
            is_root,
        })
    }
}
impl WithDeserializer for VariableReferenceType {
    type Deserializer = quick_xml_deserialize::VariableReferenceTypeDeserializer;
}
pub type VersionMatchType = String;
pub type VersionType = String;
pub type XPathVersion = AnyUriType;
#[derive(Debug,rkyv::Archive, rkyv::Serialize,rkyv::Deserialize)]
pub enum DefaultsContent39Type {
    XPathVersion(XPathVersion),
}
impl WithSerializer for DefaultsContent39Type {
    type Serializer<'x> = quick_xml_serialize::DefaultsContent39TypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        let _name = name;
        Ok(quick_xml_serialize::DefaultsContent39TypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::DefaultsContent39TypeSerializerState::Init__),
            is_root,
        })
    }
}
impl WithDeserializer for DefaultsContent39Type {
    type Deserializer = quick_xml_deserialize::DefaultsContent39TypeDeserializer;
}
#[derive(Debug,Clone)]
pub enum MatchContent47Type {
    AttributeDesignator(AttributeDesignator),
    AttributeSelector(AttributeSelector),
}
impl WithSerializer for MatchContent47Type {
    type Serializer<'x> = quick_xml_serialize::MatchContent47TypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        let _name = name;
        Ok(quick_xml_serialize::MatchContent47TypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::MatchContent47TypeSerializerState::Init__),
            is_root,
        })
    }
}
impl WithDeserializer for MatchContent47Type {
    type Deserializer = quick_xml_deserialize::MatchContent47TypeDeserializer;
}
#[derive(Debug)]
pub enum PolicySetContent31Type {
    PolicySet(PolicySet),
    Policy(Policy),
    PolicySetIdReference(PolicySetIdReference),
    PolicyIdReference(PolicyIdReference),
    CombinerParameters(CombinerParameters),
    PolicyCombinerParameters(PolicyCombinerParameters),
    PolicySetCombinerParameters(PolicySetCombinerParameters),
}
impl WithSerializer for PolicySetContent31Type {
    type Serializer<'x> = quick_xml_serialize::PolicySetContent31TypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        let _name = name;
        Ok(quick_xml_serialize::PolicySetContent31TypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::PolicySetContent31TypeSerializerState::Init__),
            is_root,
        })
    }
}
impl WithDeserializer for PolicySetContent31Type {
    type Deserializer = quick_xml_deserialize::PolicySetContent31TypeDeserializer;
}
#[derive(Debug)]
pub enum PolicyContent41Type {
    CombinerParameters(Option<CombinerParameters>),
    RuleCombinerParameters(Option<RuleCombinerParameters>),
    VariableDefinition(VariableDefinition),
    Rule(Rule),
}
impl WithSerializer for PolicyContent41Type {
    type Serializer<'x> = quick_xml_serialize::PolicyContent41TypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        let _name = name;
        Ok(quick_xml_serialize::PolicyContent41TypeSerializer {
            value: self,
            state: Box::new(quick_xml_serialize::PolicyContent41TypeSerializerState::Init__),
            is_root,
        })
    }
}
impl WithDeserializer for PolicyContent41Type {
    type Deserializer = quick_xml_deserialize::PolicyContent41TypeDeserializer;
}
#[derive(Debug,Clone)]
pub enum RequestDefaultsContent3Type {
    XPathVersion(XPathVersion),
}
impl WithSerializer for RequestDefaultsContent3Type {
    type Serializer<'x> = quick_xml_serialize::RequestDefaultsContent3TypeSerializer<'x>;
    fn serializer<'ser>(
        &'ser self,
        name: Option<&'ser str>,
        is_root: bool,
    ) -> std::result::Result<Self::Serializer<'ser>, Error> {
        let _name = name;
        Ok(quick_xml_serialize::RequestDefaultsContent3TypeSerializer {
            value: self,
            state: Box::new(
                quick_xml_serialize::RequestDefaultsContent3TypeSerializerState::Init__,
            ),
            is_root,
        })
    }
}
impl WithDeserializer for RequestDefaultsContent3Type {
    type Deserializer = quick_xml_deserialize::RequestDefaultsContent3TypeDeserializer;
}
pub mod quick_xml_deserialize {
    use core::mem::replace;
    use xsd_parser::{
        quick_xml::{
            filter_xmlns_attributes, BytesStart, ContentDeserializer, DeserializeReader,
            Deserializer, DeserializerArtifact, DeserializerEvent, DeserializerOutput,
            DeserializerResult, ElementHandlerOutput, Error, ErrorKind, Event, QName, RawByteStr,
            WithDeserializer,
        },
        xml::Text,
    };
    #[derive(Debug)]
    pub struct AnyTypeDeserializer {
        state__: Box<AnyTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum AnyTypeDeserializerState {
        Init__,
        Unknown__,
    }
    impl AnyTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            Ok(Self {
                state__: Box::new(AnyTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AnyTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            Ok(())
        }
    }
    impl<'de> Deserializer<'de, super::AnyType> for AnyTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::AnyType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AnyType>
        where
            R: DeserializeReader,
        {
            if let Event::End(_) = &event {
                Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::Data(self.finish(reader)?),
                    event: DeserializerEvent::None,
                    allow_any: false,
                })
            } else {
                Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::Deserializer(self),
                    event: DeserializerEvent::Break(event),
                    allow_any: true,
                })
            }
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::AnyType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(&mut *self.state__, AnyTypeDeserializerState::Unknown__);
            self.finish_state(reader, state)?;
            Ok(super::AnyType {})
        }
    }
    #[derive(Debug)]
    pub struct AdviceExpressionTypeDeserializer {
        advice_id: super::AnyUriType,
        applies_to: super::EffectType,
        attribute_assignment_expression: Vec<super::AttributeAssignmentExpression>,
        state__: Box<AdviceExpressionTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum AdviceExpressionTypeDeserializerState {
        Init__,
        AttributeAssignmentExpression(
            Option<<super::AttributeAssignmentExpression as WithDeserializer>::Deserializer>,
        ),
        Done__,
        Unknown__,
    }
    impl AdviceExpressionTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut advice_id: Option<super::AnyUriType> = None;
            let mut applies_to: Option<super::EffectType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"AdviceId")
                ) {
                    reader.read_attrib(&mut advice_id, b"AdviceId", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"AppliesTo")
                ) {
                    reader.read_attrib(&mut applies_to, b"AppliesTo", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                advice_id: advice_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("AdviceId".into()))
                })?,
                applies_to: applies_to.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("AppliesTo".into()))
                })?,
                attribute_assignment_expression: Vec::new(),
                state__: Box::new(AdviceExpressionTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AdviceExpressionTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use AdviceExpressionTypeDeserializerState as S;
            match state {
                S::AttributeAssignmentExpression(Some(deserializer)) => {
                    self.store_attribute_assignment_expression(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_attribute_assignment_expression(
            &mut self,
            value: super::AttributeAssignmentExpression,
        ) -> std::result::Result<(), Error> {
            self.attribute_assignment_expression.push(value);
            Ok(())
        }
        fn handle_attribute_assignment_expression<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AttributeAssignmentExpression>,
            fallback: &mut Option<AdviceExpressionTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(
                    AdviceExpressionTypeDeserializerState::AttributeAssignmentExpression(None),
                );
                *self.state__ = AdviceExpressionTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_attribute_assignment_expression(data)?;
                    *self.state__ =
                        AdviceExpressionTypeDeserializerState::AttributeAssignmentExpression(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback . get_or_insert (AdviceExpressionTypeDeserializerState :: AttributeAssignmentExpression (Some (deserializer))) ;
                            * self . state__ = AdviceExpressionTypeDeserializerState :: AttributeAssignmentExpression (None) ;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            * self . state__ = AdviceExpressionTypeDeserializerState :: AttributeAssignmentExpression (Some (deserializer)) ;
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::AdviceExpressionType> for AdviceExpressionTypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AdviceExpressionType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AdviceExpressionType>
        where
            R: DeserializeReader,
        {
            use AdviceExpressionTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::AttributeAssignmentExpression(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_attribute_assignment_expression(
                            reader,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ =
                            AdviceExpressionTypeDeserializerState::AttributeAssignmentExpression(
                                None,
                            );
                        event
                    }
                    (
                        S::AttributeAssignmentExpression(None),
                        event @ (Event::Start(_) | Event::Empty(_)),
                    ) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AttributeAssignmentExpression",
                            false,
                        )?;
                        match self.handle_attribute_assignment_expression(
                            reader,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::AdviceExpressionType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                AdviceExpressionTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::AdviceExpressionType {
                advice_id: self.advice_id,
                applies_to: self.applies_to,
                attribute_assignment_expression: self.attribute_assignment_expression,
            })
        }
    }
    #[derive(Debug)]
    pub struct AdviceExpressionsTypeDeserializer {
        advice_expression: Vec<super::AdviceExpression>,
        state__: Box<AdviceExpressionsTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum AdviceExpressionsTypeDeserializerState {
        Init__,
        AdviceExpression(Option<<super::AdviceExpression as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl AdviceExpressionsTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                advice_expression: Vec::new(),
                state__: Box::new(AdviceExpressionsTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AdviceExpressionsTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use AdviceExpressionsTypeDeserializerState as S;
            match state {
                S::AdviceExpression(Some(deserializer)) => {
                    self.store_advice_expression(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_advice_expression(&mut self, value: super::AdviceExpression) -> std::result::Result<(), Error> {
            self.advice_expression.push(value);
            Ok(())
        }
        fn handle_advice_expression<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AdviceExpression>,
            fallback: &mut Option<AdviceExpressionsTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.advice_expression.len() < 1usize {
                    *self.state__ = AdviceExpressionsTypeDeserializerState::AdviceExpression(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                } else {
                    fallback.get_or_insert(
                        AdviceExpressionsTypeDeserializerState::AdviceExpression(None),
                    );
                    *self.state__ = AdviceExpressionsTypeDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_advice_expression(data)?;
                    *self.state__ = AdviceExpressionsTypeDeserializerState::AdviceExpression(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                AdviceExpressionsTypeDeserializerState::AdviceExpression(Some(
                                    deserializer,
                                )),
                            );
                            if self.advice_expression.len().saturating_add(1) < 1usize {
                                *self.state__ =
                                    AdviceExpressionsTypeDeserializerState::AdviceExpression(None);
                            } else {
                                *self.state__ = AdviceExpressionsTypeDeserializerState::Done__;
                            }
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                AdviceExpressionsTypeDeserializerState::AdviceExpression(Some(
                                    deserializer,
                                ));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::AdviceExpressionsType> for AdviceExpressionsTypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AdviceExpressionsType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AdviceExpressionsType>
        where
            R: DeserializeReader,
        {
            use AdviceExpressionsTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::AdviceExpression(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_advice_expression(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ =
                            AdviceExpressionsTypeDeserializerState::AdviceExpression(None);
                        event
                    }
                    (S::AdviceExpression(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AdviceExpression",
                            false,
                        )?;
                        match self.handle_advice_expression(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::AdviceExpressionsType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                AdviceExpressionsTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::AdviceExpressionsType {
                advice_expression: self.advice_expression,
            })
        }
    }
    #[derive(Debug)]
    pub struct AdviceTypeDeserializer {
        advice_id: super::AnyUriType,
        attribute_assignment: Vec<super::AttributeAssignment>,
        state__: Box<AdviceTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum AdviceTypeDeserializerState {
        Init__,
        AttributeAssignment(Option<<super::AttributeAssignment as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl AdviceTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut advice_id: Option<super::AnyUriType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"AdviceId")
                ) {
                    reader.read_attrib(&mut advice_id, b"AdviceId", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                advice_id: advice_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("AdviceId".into()))
                })?,
                attribute_assignment: Vec::new(),
                state__: Box::new(AdviceTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AdviceTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use AdviceTypeDeserializerState as S;
            match state {
                S::AttributeAssignment(Some(deserializer)) => {
                    self.store_attribute_assignment(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_attribute_assignment(
            &mut self,
            value: super::AttributeAssignment,
        ) -> std::result::Result<(), Error> {
            self.attribute_assignment.push(value);
            Ok(())
        }
        fn handle_attribute_assignment<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AttributeAssignment>,
            fallback: &mut Option<AdviceTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(AdviceTypeDeserializerState::AttributeAssignment(None));
                *self.state__ = AdviceTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_attribute_assignment(data)?;
                    *self.state__ = AdviceTypeDeserializerState::AttributeAssignment(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                AdviceTypeDeserializerState::AttributeAssignment(Some(
                                    deserializer,
                                )),
                            );
                            *self.state__ = AdviceTypeDeserializerState::AttributeAssignment(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = AdviceTypeDeserializerState::AttributeAssignment(Some(
                                deserializer,
                            ));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::AdviceType> for AdviceTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::AdviceType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AdviceType>
        where
            R: DeserializeReader,
        {
            use AdviceTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::AttributeAssignment(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_attribute_assignment(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = AdviceTypeDeserializerState::AttributeAssignment(None);
                        event
                    }
                    (S::AttributeAssignment(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AttributeAssignment",
                            false,
                        )?;
                        match self.handle_attribute_assignment(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::AdviceType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(&mut *self.state__, AdviceTypeDeserializerState::Unknown__);
            self.finish_state(reader, state)?;
            Ok(super::AdviceType {
                advice_id: self.advice_id,
                attribute_assignment: self.attribute_assignment,
            })
        }
    }
    #[derive(Debug)]
    pub struct AllOfTypeDeserializer {
        content: Vec<super::AllOfTypeContent>,
        state__: Box<AllOfTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum AllOfTypeDeserializerState {
        Init__,
        Next__,
        Content__(<super::AllOfTypeContent as WithDeserializer>::Deserializer),
        Unknown__,
    }
    impl AllOfTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                content: Vec::new(),
                state__: Box::new(AllOfTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AllOfTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            if let AllOfTypeDeserializerState::Content__(deserializer) = state {
                self.store_content(deserializer.finish(reader)?)?;
            }
            Ok(())
        }
        fn store_content(&mut self, value: super::AllOfTypeContent) -> std::result::Result<(), Error> {
            self.content.push(value);
            Ok(())
        }
        fn handle_content<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AllOfTypeContent>,
            fallback: &mut Option<AllOfTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = fallback
                    .take()
                    .unwrap_or(AllOfTypeDeserializerState::Next__);
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_content(data)?;
                    *self.state__ = AllOfTypeDeserializerState::Next__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = AllOfTypeDeserializerState::Content__(deserializer);
                        }
                        ElementHandlerOutput::Continue { .. } => {
                            fallback
                                .get_or_insert(AllOfTypeDeserializerState::Content__(deserializer));
                            *self.state__ = AllOfTypeDeserializerState::Next__;
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::AllOfType> for AllOfTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::AllOfType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AllOfType>
        where
            R: DeserializeReader,
        {
            use AllOfTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Content__(deserializer), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_content(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (_, Event::End(_)) => {
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (state @ (S::Init__ | S::Next__), event) => {
                        fallback.get_or_insert(state);
                        let output =
                            <super::AllOfTypeContent as WithDeserializer>::Deserializer::init(
                                reader, event,
                            )?;
                        match self.handle_content(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::Unknown__, _) => unreachable!(),
                }
            };
            let artifact = DeserializerArtifact::Deserializer(self);
            Ok(DeserializerOutput {
                artifact,
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::AllOfType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(&mut *self.state__, AllOfTypeDeserializerState::Unknown__);
            self.finish_state(reader, state)?;
            Ok(super::AllOfType {
                content: self.content,
            })
        }
    }
    #[derive(Debug)]
    pub struct AllOfTypeContentDeserializer {
        match_: Option<super::Match>,
        state__: Box<AllOfTypeContentDeserializerState>,
    }
    #[derive(Debug)]
    enum AllOfTypeContentDeserializerState {
        Init__,
        Match(Option<<super::Match as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl AllOfTypeContentDeserializer {
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AllOfTypeContentDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use AllOfTypeContentDeserializerState as S;
            match state {
                S::Match(Some(deserializer)) => self.store_match_(deserializer.finish(reader)?)?,
                _ => (),
            }
            Ok(())
        }
        fn store_match_(&mut self, value: super::Match) -> std::result::Result<(), Error> {
            if self.match_.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Match",
                )))?;
            }
            self.match_ = Some(value);
            Ok(())
        }
        fn handle_match_<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Match>,
            fallback: &mut Option<AllOfTypeContentDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.match_.is_some() {
                    fallback.get_or_insert(AllOfTypeContentDeserializerState::Match(None));
                    *self.state__ = AllOfTypeContentDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                } else {
                    *self.state__ = AllOfTypeContentDeserializerState::Match(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_match_(data)?;
                    *self.state__ = AllOfTypeContentDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(AllOfTypeContentDeserializerState::Match(Some(
                                deserializer,
                            )));
                            *self.state__ = AllOfTypeContentDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                AllOfTypeContentDeserializerState::Match(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::AllOfTypeContent> for AllOfTypeContentDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AllOfTypeContent>
        where
            R: DeserializeReader,
        {
            let deserializer = Self {
                match_: None,
                state__: Box::new(AllOfTypeContentDeserializerState::Init__),
            };
            let mut output = deserializer.next(reader, event)?;
            output.artifact = match output.artifact {
                DeserializerArtifact::Deserializer(x)
                    if matches!(&*x.state__, AllOfTypeContentDeserializerState::Init__) =>
                {
                    DeserializerArtifact::None
                }
                artifact => artifact,
            };
            Ok(output)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AllOfTypeContent>
        where
            R: DeserializeReader,
        {
            use AllOfTypeContentDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Match(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_match_(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, event @ Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::Continue(event),
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = AllOfTypeContentDeserializerState::Match(None);
                        event
                    }
                    (S::Match(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Match",
                            false,
                        )?;
                        match self.handle_match_(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::AllOfTypeContent, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                AllOfTypeContentDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::AllOfTypeContent {
                match_: self
                    .match_
                    .ok_or_else(|| ErrorKind::MissingElement("Match".into()))?,
            })
        }
    }
    #[derive(Debug)]
    pub struct AnyOfTypeDeserializer {
        content: Vec<super::AnyOfTypeContent>,
        state__: Box<AnyOfTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum AnyOfTypeDeserializerState {
        Init__,
        Next__,
        Content__(<super::AnyOfTypeContent as WithDeserializer>::Deserializer),
        Unknown__,
    }
    impl AnyOfTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                content: Vec::new(),
                state__: Box::new(AnyOfTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AnyOfTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            if let AnyOfTypeDeserializerState::Content__(deserializer) = state {
                self.store_content(deserializer.finish(reader)?)?;
            }
            Ok(())
        }
        fn store_content(&mut self, value: super::AnyOfTypeContent) -> std::result::Result<(), Error> {
            self.content.push(value);
            Ok(())
        }
        fn handle_content<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AnyOfTypeContent>,
            fallback: &mut Option<AnyOfTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = fallback
                    .take()
                    .unwrap_or(AnyOfTypeDeserializerState::Next__);
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_content(data)?;
                    *self.state__ = AnyOfTypeDeserializerState::Next__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = AnyOfTypeDeserializerState::Content__(deserializer);
                        }
                        ElementHandlerOutput::Continue { .. } => {
                            fallback
                                .get_or_insert(AnyOfTypeDeserializerState::Content__(deserializer));
                            *self.state__ = AnyOfTypeDeserializerState::Next__;
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::AnyOfType> for AnyOfTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::AnyOfType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AnyOfType>
        where
            R: DeserializeReader,
        {
            use AnyOfTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Content__(deserializer), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_content(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (_, Event::End(_)) => {
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (state @ (S::Init__ | S::Next__), event) => {
                        fallback.get_or_insert(state);
                        let output =
                            <super::AnyOfTypeContent as WithDeserializer>::Deserializer::init(
                                reader, event,
                            )?;
                        match self.handle_content(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::Unknown__, _) => unreachable!(),
                }
            };
            let artifact = DeserializerArtifact::Deserializer(self);
            Ok(DeserializerOutput {
                artifact,
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::AnyOfType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(&mut *self.state__, AnyOfTypeDeserializerState::Unknown__);
            self.finish_state(reader, state)?;
            Ok(super::AnyOfType {
                content: self.content,
            })
        }
    }
    #[derive(Debug)]
    pub struct AnyOfTypeContentDeserializer {
        all_of: Option<super::AllOf>,
        state__: Box<AnyOfTypeContentDeserializerState>,
    }
    #[derive(Debug)]
    enum AnyOfTypeContentDeserializerState {
        Init__,
        AllOf(Option<<super::AllOf as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl AnyOfTypeContentDeserializer {
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AnyOfTypeContentDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use AnyOfTypeContentDeserializerState as S;
            match state {
                S::AllOf(Some(deserializer)) => self.store_all_of(deserializer.finish(reader)?)?,
                _ => (),
            }
            Ok(())
        }
        fn store_all_of(&mut self, value: super::AllOf) -> std::result::Result<(), Error> {
            if self.all_of.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"AllOf",
                )))?;
            }
            self.all_of = Some(value);
            Ok(())
        }
        fn handle_all_of<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AllOf>,
            fallback: &mut Option<AnyOfTypeContentDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.all_of.is_some() {
                    fallback.get_or_insert(AnyOfTypeContentDeserializerState::AllOf(None));
                    *self.state__ = AnyOfTypeContentDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                } else {
                    *self.state__ = AnyOfTypeContentDeserializerState::AllOf(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_all_of(data)?;
                    *self.state__ = AnyOfTypeContentDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(AnyOfTypeContentDeserializerState::AllOf(Some(
                                deserializer,
                            )));
                            *self.state__ = AnyOfTypeContentDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                AnyOfTypeContentDeserializerState::AllOf(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::AnyOfTypeContent> for AnyOfTypeContentDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AnyOfTypeContent>
        where
            R: DeserializeReader,
        {
            let deserializer = Self {
                all_of: None,
                state__: Box::new(AnyOfTypeContentDeserializerState::Init__),
            };
            let mut output = deserializer.next(reader, event)?;
            output.artifact = match output.artifact {
                DeserializerArtifact::Deserializer(x)
                    if matches!(&*x.state__, AnyOfTypeContentDeserializerState::Init__) =>
                {
                    DeserializerArtifact::None
                }
                artifact => artifact,
            };
            Ok(output)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AnyOfTypeContent>
        where
            R: DeserializeReader,
        {
            use AnyOfTypeContentDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::AllOf(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_all_of(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, event @ Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::Continue(event),
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = AnyOfTypeContentDeserializerState::AllOf(None);
                        event
                    }
                    (S::AllOf(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AllOf",
                            false,
                        )?;
                        match self.handle_all_of(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::AnyOfTypeContent, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                AnyOfTypeContentDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::AnyOfTypeContent {
                all_of: self
                    .all_of
                    .ok_or_else(|| ErrorKind::MissingElement("AllOf".into()))?,
            })
        }
    }
    #[derive(Debug)]
    pub struct ApplyTypeDeserializer {
        function_id: super::AnyUriType,
        description: Option<super::Description>,
        expression: Vec<super::Expression>,
        state__: Box<ApplyTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum ApplyTypeDeserializerState {
        Init__,
        Description(Option<<super::Description as WithDeserializer>::Deserializer>),
        Expression(Option<<super::Expression as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl ApplyTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut function_id: Option<super::AnyUriType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"FunctionId")
                ) {
                    reader.read_attrib(&mut function_id, b"FunctionId", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                function_id: function_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("FunctionId".into()))
                })?,
                description: None,
                expression: Vec::new(),
                state__: Box::new(ApplyTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: ApplyTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use ApplyTypeDeserializerState as S;
            match state {
                S::Description(Some(deserializer)) => {
                    self.store_description(deserializer.finish(reader)?)?
                }
                S::Expression(Some(deserializer)) => {
                    self.store_expression(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_description(&mut self, value: super::Description) -> std::result::Result<(), Error> {
            if self.description.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Description",
                )))?;
            }
            self.description = Some(value);
            Ok(())
        }
        fn store_expression(&mut self, value: super::Expression) -> std::result::Result<(), Error> {
            self.expression.push(value);
            Ok(())
        }
        fn handle_description<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Description>,
            fallback: &mut Option<ApplyTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(ApplyTypeDeserializerState::Description(None));
                *self.state__ = ApplyTypeDeserializerState::Expression(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_description(data)?;
                    *self.state__ = ApplyTypeDeserializerState::Expression(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(ApplyTypeDeserializerState::Description(Some(
                                deserializer,
                            )));
                            *self.state__ = ApplyTypeDeserializerState::Expression(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                ApplyTypeDeserializerState::Description(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_expression<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Expression>,
            fallback: &mut Option<ApplyTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(ApplyTypeDeserializerState::Expression(None));
                *self.state__ = ApplyTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_expression(data)?;
                    *self.state__ = ApplyTypeDeserializerState::Expression(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(ApplyTypeDeserializerState::Expression(Some(
                                deserializer,
                            )));
                            *self.state__ = ApplyTypeDeserializerState::Expression(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                ApplyTypeDeserializerState::Expression(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::ApplyType> for ApplyTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::ApplyType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::ApplyType>
        where
            R: DeserializeReader,
        {
            use ApplyTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Description(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_description(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Expression(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_expression(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = ApplyTypeDeserializerState::Description(None);
                        event
                    }
                    (S::Description(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Description",
                            false,
                        )?;
                        match self.handle_description(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Expression(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = <super::Expression as WithDeserializer>::Deserializer::init(
                            reader, event,
                        )?;
                        match self.handle_expression(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::ApplyType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(&mut *self.state__, ApplyTypeDeserializerState::Unknown__);
            self.finish_state(reader, state)?;
            Ok(super::ApplyType {
                function_id: self.function_id,
                description: self.description,
                expression: self.expression,
            })
        }
    }
    #[derive(Debug)]
    pub struct AssociatedAdviceTypeDeserializer {
        advice: Vec<super::Advice>,
        state__: Box<AssociatedAdviceTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum AssociatedAdviceTypeDeserializerState {
        Init__,
        Advice(Option<<super::Advice as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl AssociatedAdviceTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                advice: Vec::new(),
                state__: Box::new(AssociatedAdviceTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AssociatedAdviceTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use AssociatedAdviceTypeDeserializerState as S;
            match state {
                S::Advice(Some(deserializer)) => self.store_advice(deserializer.finish(reader)?)?,
                _ => (),
            }
            Ok(())
        }
        fn store_advice(&mut self, value: super::Advice) -> std::result::Result<(), Error> {
            self.advice.push(value);
            Ok(())
        }
        fn handle_advice<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Advice>,
            fallback: &mut Option<AssociatedAdviceTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.advice.len() < 1usize {
                    *self.state__ = AssociatedAdviceTypeDeserializerState::Advice(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                } else {
                    fallback.get_or_insert(AssociatedAdviceTypeDeserializerState::Advice(None));
                    *self.state__ = AssociatedAdviceTypeDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_advice(data)?;
                    *self.state__ = AssociatedAdviceTypeDeserializerState::Advice(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(AssociatedAdviceTypeDeserializerState::Advice(
                                Some(deserializer),
                            ));
                            if self.advice.len().saturating_add(1) < 1usize {
                                *self.state__ = AssociatedAdviceTypeDeserializerState::Advice(None);
                            } else {
                                *self.state__ = AssociatedAdviceTypeDeserializerState::Done__;
                            }
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                AssociatedAdviceTypeDeserializerState::Advice(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::AssociatedAdviceType> for AssociatedAdviceTypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AssociatedAdviceType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AssociatedAdviceType>
        where
            R: DeserializeReader,
        {
            use AssociatedAdviceTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Advice(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_advice(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = AssociatedAdviceTypeDeserializerState::Advice(None);
                        event
                    }
                    (S::Advice(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Advice",
                            false,
                        )?;
                        match self.handle_advice(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::AssociatedAdviceType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                AssociatedAdviceTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::AssociatedAdviceType {
                advice: self.advice,
            })
        }
    }
    #[derive(Debug)]
    pub struct AttributeAssignmentExpressionTypeDeserializer {
        attribute_id: super::AnyUriType,
        category: Option<super::AnyUriType>,
        issuer: Option<super::StringType>,
        expression: Option<super::Expression>,
        state__: Box<AttributeAssignmentExpressionTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum AttributeAssignmentExpressionTypeDeserializerState {
        Init__,
        Expression(Option<<super::Expression as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl AttributeAssignmentExpressionTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut attribute_id: Option<super::AnyUriType> = None;
            let mut category: Option<super::AnyUriType> = None;
            let mut issuer: Option<super::StringType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"AttributeId")
                ) {
                    reader.read_attrib(&mut attribute_id, b"AttributeId", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Category")
                ) {
                    reader.read_attrib(&mut category, b"Category", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Issuer")
                ) {
                    reader.read_attrib(&mut issuer, b"Issuer", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                attribute_id: attribute_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("AttributeId".into()))
                })?,
                category: category,
                issuer: issuer,
                expression: None,
                state__: Box::new(AttributeAssignmentExpressionTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AttributeAssignmentExpressionTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use AttributeAssignmentExpressionTypeDeserializerState as S;
            match state {
                S::Expression(Some(deserializer)) => {
                    self.store_expression(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_expression(&mut self, value: super::Expression) -> std::result::Result<(), Error> {
            if self.expression.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Expression",
                )))?;
            }
            self.expression = Some(value);
            Ok(())
        }
        fn handle_expression<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Expression>,
            fallback: &mut Option<AttributeAssignmentExpressionTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.expression.is_some() {
                    fallback.get_or_insert(
                        AttributeAssignmentExpressionTypeDeserializerState::Expression(None),
                    );
                    *self.state__ = AttributeAssignmentExpressionTypeDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                } else {
                    *self.state__ =
                        AttributeAssignmentExpressionTypeDeserializerState::Expression(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_expression(data)?;
                    *self.state__ = AttributeAssignmentExpressionTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                AttributeAssignmentExpressionTypeDeserializerState::Expression(
                                    Some(deserializer),
                                ),
                            );
                            *self.state__ =
                                AttributeAssignmentExpressionTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                AttributeAssignmentExpressionTypeDeserializerState::Expression(
                                    Some(deserializer),
                                );
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::AttributeAssignmentExpressionType>
        for AttributeAssignmentExpressionTypeDeserializer
    {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AttributeAssignmentExpressionType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AttributeAssignmentExpressionType>
        where
            R: DeserializeReader,
        {
            use AttributeAssignmentExpressionTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Expression(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_expression(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ =
                            AttributeAssignmentExpressionTypeDeserializerState::Expression(None);
                        event
                    }
                    (S::Expression(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = <super::Expression as WithDeserializer>::Deserializer::init(
                            reader, event,
                        )?;
                        match self.handle_expression(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(
            mut self,
            reader: &R,
        ) -> std::result::Result<super::AttributeAssignmentExpressionType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                AttributeAssignmentExpressionTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::AttributeAssignmentExpressionType {
                attribute_id: self.attribute_id,
                category: self.category,
                issuer: self.issuer,
                expression: self
                    .expression
                    .ok_or_else(|| ErrorKind::MissingElement("Expression".into()))?,
            })
        }
    }
    #[derive(Debug)]
    pub struct AttributeAssignmentTypeDeserializer {
        data_type: super::AnyUriType,
        attribute_id: super::AnyUriType,
        category: Option<super::AnyUriType>,
        issuer: Option<super::StringType>,
        text: Option<Text>,
        state__: Box<AttributeAssignmentTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum AttributeAssignmentTypeDeserializerState {
        Init__,
        Text(Option<<Text as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl AttributeAssignmentTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut data_type: Option<super::AnyUriType> = None;
            let mut attribute_id: Option<super::AnyUriType> = None;
            let mut category: Option<super::AnyUriType> = None;
            let mut issuer: Option<super::StringType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"DataType")
                ) {
                    reader.read_attrib(&mut data_type, b"DataType", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"AttributeId")
                ) {
                    reader.read_attrib(&mut attribute_id, b"AttributeId", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Category")
                ) {
                    reader.read_attrib(&mut category, b"Category", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Issuer")
                ) {
                    reader.read_attrib(&mut issuer, b"Issuer", &attrib.value)?;
                }
            }
            Ok(Self {
                data_type: data_type.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("DataType".into()))
                })?,
                attribute_id: attribute_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("AttributeId".into()))
                })?,
                category: category,
                issuer: issuer,
                text: None,
                state__: Box::new(AttributeAssignmentTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AttributeAssignmentTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use AttributeAssignmentTypeDeserializerState as S;
            match state {
                S::Text(Some(deserializer)) => self.store_text(deserializer.finish(reader)?)?,
                _ => (),
            }
            Ok(())
        }
        fn store_text(&mut self, value: Text) -> std::result::Result<(), Error> {
            if self.text.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(b"text")))?;
            }
            self.text = Some(value);
            Ok(())
        }
        fn handle_text<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, Text>,
            fallback: &mut Option<AttributeAssignmentTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(AttributeAssignmentTypeDeserializerState::Text(None));
                *self.state__ = AttributeAssignmentTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_text(data)?;
                    *self.state__ = AttributeAssignmentTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(AttributeAssignmentTypeDeserializerState::Text(
                                Some(deserializer),
                            ));
                            *self.state__ = AttributeAssignmentTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                AttributeAssignmentTypeDeserializerState::Text(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::AttributeAssignmentType>
        for AttributeAssignmentTypeDeserializer
    {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AttributeAssignmentType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AttributeAssignmentType>
        where
            R: DeserializeReader,
        {
            use AttributeAssignmentTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Text(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_text(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        allow_any_element = true;
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = AttributeAssignmentTypeDeserializerState::Text(None);
                        event
                    }
                    (S::Text(None), event) => {
                        let output = <Text as WithDeserializer>::Deserializer::init(reader, event)?;
                        match self.handle_text(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), true);
                    }
                    (S::Unknown__, _) => unreachable!(),
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::AttributeAssignmentType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                AttributeAssignmentTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::AttributeAssignmentType {
                data_type: self.data_type,
                attribute_id: self.attribute_id,
                category: self.category,
                issuer: self.issuer,
                text: self.text,
            })
        }
    }
    #[derive(Debug)]
    pub struct AttributeDesignatorTypeDeserializer {
        category: super::AnyUriType,
        attribute_id: super::AnyUriType,
        data_type: super::AnyUriType,
        issuer: Option<super::StringType>,
        must_be_present: super::BooleanType,
        state__: Box<AttributeDesignatorTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum AttributeDesignatorTypeDeserializerState {
        Init__,
        Unknown__,
    }
    impl AttributeDesignatorTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut category: Option<super::AnyUriType> = None;
            let mut attribute_id: Option<super::AnyUriType> = None;
            let mut data_type: Option<super::AnyUriType> = None;
            let mut issuer: Option<super::StringType> = None;
            let mut must_be_present: Option<super::BooleanType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Category")
                ) {
                    reader.read_attrib(&mut category, b"Category", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"AttributeId")
                ) {
                    reader.read_attrib(&mut attribute_id, b"AttributeId", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"DataType")
                ) {
                    reader.read_attrib(&mut data_type, b"DataType", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Issuer")
                ) {
                    reader.read_attrib(&mut issuer, b"Issuer", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"MustBePresent")
                ) {
                    reader.read_attrib(&mut must_be_present, b"MustBePresent", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                category: category.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("Category".into()))
                })?,
                attribute_id: attribute_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("AttributeId".into()))
                })?,
                data_type: data_type.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("DataType".into()))
                })?,
                issuer: issuer,
                must_be_present: must_be_present.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("MustBePresent".into()))
                })?,
                state__: Box::new(AttributeDesignatorTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AttributeDesignatorTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            Ok(())
        }
    }
    impl<'de> Deserializer<'de, super::AttributeDesignatorType>
        for AttributeDesignatorTypeDeserializer
    {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AttributeDesignatorType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AttributeDesignatorType>
        where
            R: DeserializeReader,
        {
            if let Event::End(_) = &event {
                Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::Data(self.finish(reader)?),
                    event: DeserializerEvent::None,
                    allow_any: false,
                })
            } else {
                Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::Deserializer(self),
                    event: DeserializerEvent::Break(event),
                    allow_any: false,
                })
            }
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::AttributeDesignatorType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                AttributeDesignatorTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::AttributeDesignatorType {
                category: self.category,
                attribute_id: self.attribute_id,
                data_type: self.data_type,
                issuer: self.issuer,
                must_be_present: self.must_be_present,
            })
        }
    }
    #[derive(Debug)]
    pub struct AttributeSelectorTypeDeserializer {
        category: super::AnyUriType,
        context_selector_id: Option<super::AnyUriType>,
        path: super::StringType,
        data_type: super::AnyUriType,
        must_be_present: super::BooleanType,
        state__: Box<AttributeSelectorTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum AttributeSelectorTypeDeserializerState {
        Init__,
        Unknown__,
    }
    impl AttributeSelectorTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut category: Option<super::AnyUriType> = None;
            let mut context_selector_id: Option<super::AnyUriType> = None;
            let mut path: Option<super::StringType> = None;
            let mut data_type: Option<super::AnyUriType> = None;
            let mut must_be_present: Option<super::BooleanType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Category")
                ) {
                    reader.read_attrib(&mut category, b"Category", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"ContextSelectorId")
                ) {
                    reader.read_attrib(
                        &mut context_selector_id,
                        b"ContextSelectorId",
                        &attrib.value,
                    )?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Path")
                ) {
                    reader.read_attrib(&mut path, b"Path", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"DataType")
                ) {
                    reader.read_attrib(&mut data_type, b"DataType", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"MustBePresent")
                ) {
                    reader.read_attrib(&mut must_be_present, b"MustBePresent", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                category: category.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("Category".into()))
                })?,
                context_selector_id: context_selector_id,
                path: path
                    .ok_or_else(|| reader.map_error(ErrorKind::MissingAttribute("Path".into())))?,
                data_type: data_type.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("DataType".into()))
                })?,
                must_be_present: must_be_present.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("MustBePresent".into()))
                })?,
                state__: Box::new(AttributeSelectorTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AttributeSelectorTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            Ok(())
        }
    }
    impl<'de> Deserializer<'de, super::AttributeSelectorType> for AttributeSelectorTypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AttributeSelectorType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AttributeSelectorType>
        where
            R: DeserializeReader,
        {
            if let Event::End(_) = &event {
                Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::Data(self.finish(reader)?),
                    event: DeserializerEvent::None,
                    allow_any: false,
                })
            } else {
                Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::Deserializer(self),
                    event: DeserializerEvent::Break(event),
                    allow_any: false,
                })
            }
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::AttributeSelectorType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                AttributeSelectorTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::AttributeSelectorType {
                category: self.category,
                context_selector_id: self.context_selector_id,
                path: self.path,
                data_type: self.data_type,
                must_be_present: self.must_be_present,
            })
        }
    }
    #[derive(Debug)]
    pub struct AttributeTypeDeserializer {
        attribute_id: super::AnyUriType,
        issuer: Option<super::StringType>,
        include_in_result: super::BooleanType,
        attribute_value: Vec<super::AttributeValue>,
        state__: Box<AttributeTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum AttributeTypeDeserializerState {
        Init__,
        AttributeValue(Option<<super::AttributeValue as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl AttributeTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut attribute_id: Option<super::AnyUriType> = None;
            let mut issuer: Option<super::StringType> = None;
            let mut include_in_result: Option<super::BooleanType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"AttributeId")
                ) {
                    reader.read_attrib(&mut attribute_id, b"AttributeId", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Issuer")
                ) {
                    reader.read_attrib(&mut issuer, b"Issuer", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"IncludeInResult")
                ) {
                    reader.read_attrib(
                        &mut include_in_result,
                        b"IncludeInResult",
                        &attrib.value,
                    )?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                attribute_id: attribute_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("AttributeId".into()))
                })?,
                issuer: issuer,
                include_in_result: include_in_result.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("IncludeInResult".into()))
                })?,
                attribute_value: Vec::new(),
                state__: Box::new(AttributeTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AttributeTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use AttributeTypeDeserializerState as S;
            match state {
                S::AttributeValue(Some(deserializer)) => {
                    self.store_attribute_value(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_attribute_value(&mut self, value: super::AttributeValue) -> std::result::Result<(), Error> {
            self.attribute_value.push(value);
            Ok(())
        }
        fn handle_attribute_value<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AttributeValue>,
            fallback: &mut Option<AttributeTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.attribute_value.len() < 1usize {
                    *self.state__ = AttributeTypeDeserializerState::AttributeValue(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                } else {
                    fallback.get_or_insert(AttributeTypeDeserializerState::AttributeValue(None));
                    *self.state__ = AttributeTypeDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_attribute_value(data)?;
                    *self.state__ = AttributeTypeDeserializerState::AttributeValue(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(AttributeTypeDeserializerState::AttributeValue(
                                Some(deserializer),
                            ));
                            if self.attribute_value.len().saturating_add(1) < 1usize {
                                *self.state__ =
                                    AttributeTypeDeserializerState::AttributeValue(None);
                            } else {
                                *self.state__ = AttributeTypeDeserializerState::Done__;
                            }
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                AttributeTypeDeserializerState::AttributeValue(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::AttributeType> for AttributeTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::AttributeType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AttributeType>
        where
            R: DeserializeReader,
        {
            use AttributeTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::AttributeValue(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_attribute_value(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = AttributeTypeDeserializerState::AttributeValue(None);
                        event
                    }
                    (S::AttributeValue(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AttributeValue",
                            false,
                        )?;
                        match self.handle_attribute_value(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::AttributeType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                AttributeTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::AttributeType {
                attribute_id: self.attribute_id,
                issuer: self.issuer,
                include_in_result: self.include_in_result,
                attribute_value: self.attribute_value,
            })
        }
    }
    #[derive(Debug)]
    pub struct AttributeValueTypeDeserializer {
        data_type: super::AnyUriType,
        text: Option<Text>,
        state__: Box<AttributeValueTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum AttributeValueTypeDeserializerState {
        Init__,
        Text(Option<<Text as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl AttributeValueTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut data_type: Option<super::AnyUriType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"DataType")
                ) {
                    reader.read_attrib(&mut data_type, b"DataType", &attrib.value)?;
                }
            }
            Ok(Self {
                data_type: data_type.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("DataType".into()))
                })?,
                text: None,
                state__: Box::new(AttributeValueTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AttributeValueTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use AttributeValueTypeDeserializerState as S;
            match state {
                S::Text(Some(deserializer)) => self.store_text(deserializer.finish(reader)?)?,
                _ => (),
            }
            Ok(())
        }
        fn store_text(&mut self, value: Text) -> std::result::Result<(), Error> {
            if self.text.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(b"text")))?;
            }
            self.text = Some(value);
            Ok(())
        }
        fn handle_text<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, Text>,
            fallback: &mut Option<AttributeValueTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(AttributeValueTypeDeserializerState::Text(None));
                *self.state__ = AttributeValueTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_text(data)?;
                    *self.state__ = AttributeValueTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(AttributeValueTypeDeserializerState::Text(
                                Some(deserializer),
                            ));
                            *self.state__ = AttributeValueTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                AttributeValueTypeDeserializerState::Text(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::AttributeValueType> for AttributeValueTypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AttributeValueType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AttributeValueType>
        where
            R: DeserializeReader,
        {
            use AttributeValueTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Text(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_text(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        allow_any_element = true;
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = AttributeValueTypeDeserializerState::Text(None);
                        event
                    }
                    (S::Text(None), event) => {
                        let output = <Text as WithDeserializer>::Deserializer::init(reader, event)?;
                        match self.handle_text(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), true);
                    }
                    (S::Unknown__, _) => unreachable!(),
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::AttributeValueType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                AttributeValueTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::AttributeValueType {
                data_type: self.data_type,
                text: self.text,
            })
        }
    }
    #[derive(Debug)]
    pub struct AttributesReferenceTypeDeserializer {
        reference_id: super::IdrefType,
        state__: Box<AttributesReferenceTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum AttributesReferenceTypeDeserializerState {
        Init__,
        Unknown__,
    }
    impl AttributesReferenceTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut reference_id: Option<super::IdrefType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"ReferenceId")
                ) {
                    reader.read_attrib(&mut reference_id, b"ReferenceId", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                reference_id: reference_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("ReferenceId".into()))
                })?,
                state__: Box::new(AttributesReferenceTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AttributesReferenceTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            Ok(())
        }
    }
    impl<'de> Deserializer<'de, super::AttributesReferenceType>
        for AttributesReferenceTypeDeserializer
    {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AttributesReferenceType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AttributesReferenceType>
        where
            R: DeserializeReader,
        {
            if let Event::End(_) = &event {
                Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::Data(self.finish(reader)?),
                    event: DeserializerEvent::None,
                    allow_any: false,
                })
            } else {
                Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::Deserializer(self),
                    event: DeserializerEvent::Break(event),
                    allow_any: false,
                })
            }
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::AttributesReferenceType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                AttributesReferenceTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::AttributesReferenceType {
                reference_id: self.reference_id,
            })
        }
    }
    #[derive(Debug)]
    pub struct AttributesTypeDeserializer {
        category: super::AnyUriType,
        id: Option<super::Id>,
        content: Option<super::Content>,
        attribute: Vec<super::Attribute>,
        state__: Box<AttributesTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum AttributesTypeDeserializerState {
        Init__,
        Content(Option<<super::Content as WithDeserializer>::Deserializer>),
        Attribute(Option<<super::Attribute as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl AttributesTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut category: Option<super::AnyUriType> = None;
            let mut id: Option<super::Id> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Category")
                ) {
                    reader.read_attrib(&mut category, b"Category", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XML),
                    Some(b"id")
                ) {
                    reader.read_attrib(&mut id, b"id", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                category: category.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("Category".into()))
                })?,
                id: id,
                content: None,
                attribute: Vec::new(),
                state__: Box::new(AttributesTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: AttributesTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use AttributesTypeDeserializerState as S;
            match state {
                S::Content(Some(deserializer)) => {
                    self.store_content(deserializer.finish(reader)?)?
                }
                S::Attribute(Some(deserializer)) => {
                    self.store_attribute(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_content(&mut self, value: super::Content) -> std::result::Result<(), Error> {
            if self.content.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Content",
                )))?;
            }
            self.content = Some(value);
            Ok(())
        }
        fn store_attribute(&mut self, value: super::Attribute) -> std::result::Result<(), Error> {
            self.attribute.push(value);
            Ok(())
        }
        fn handle_content<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Content>,
            fallback: &mut Option<AttributesTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(AttributesTypeDeserializerState::Content(None));
                *self.state__ = AttributesTypeDeserializerState::Attribute(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_content(data)?;
                    *self.state__ = AttributesTypeDeserializerState::Attribute(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(AttributesTypeDeserializerState::Content(Some(
                                deserializer,
                            )));
                            *self.state__ = AttributesTypeDeserializerState::Attribute(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                AttributesTypeDeserializerState::Content(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_attribute<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Attribute>,
            fallback: &mut Option<AttributesTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(AttributesTypeDeserializerState::Attribute(None));
                *self.state__ = AttributesTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_attribute(data)?;
                    *self.state__ = AttributesTypeDeserializerState::Attribute(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(AttributesTypeDeserializerState::Attribute(
                                Some(deserializer),
                            ));
                            *self.state__ = AttributesTypeDeserializerState::Attribute(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                AttributesTypeDeserializerState::Attribute(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::AttributesType> for AttributesTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::AttributesType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::AttributesType>
        where
            R: DeserializeReader,
        {
            use AttributesTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Content(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_content(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Attribute(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_attribute(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = AttributesTypeDeserializerState::Content(None);
                        event
                    }
                    (S::Content(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Content",
                            false,
                        )?;
                        match self.handle_content(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Attribute(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Attribute",
                            false,
                        )?;
                        match self.handle_attribute(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::AttributesType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                AttributesTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::AttributesType {
                category: self.category,
                id: self.id,
                content: self.content,
                attribute: self.attribute,
            })
        }
    }
    #[derive(Debug)]
    pub struct CombinerParameterTypeDeserializer {
        parameter_name: super::StringType,
        attribute_value: Option<super::AttributeValue>,
        state__: Box<CombinerParameterTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum CombinerParameterTypeDeserializerState {
        Init__,
        AttributeValue(Option<<super::AttributeValue as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl CombinerParameterTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut parameter_name: Option<super::StringType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"ParameterName")
                ) {
                    reader.read_attrib(&mut parameter_name, b"ParameterName", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                parameter_name: parameter_name.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("ParameterName".into()))
                })?,
                attribute_value: None,
                state__: Box::new(CombinerParameterTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: CombinerParameterTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use CombinerParameterTypeDeserializerState as S;
            match state {
                S::AttributeValue(Some(deserializer)) => {
                    self.store_attribute_value(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_attribute_value(&mut self, value: super::AttributeValue) -> std::result::Result<(), Error> {
            if self.attribute_value.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"AttributeValue",
                )))?;
            }
            self.attribute_value = Some(value);
            Ok(())
        }
        fn handle_attribute_value<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AttributeValue>,
            fallback: &mut Option<CombinerParameterTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.attribute_value.is_some() {
                    fallback.get_or_insert(CombinerParameterTypeDeserializerState::AttributeValue(
                        None,
                    ));
                    *self.state__ = CombinerParameterTypeDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                } else {
                    *self.state__ = CombinerParameterTypeDeserializerState::AttributeValue(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_attribute_value(data)?;
                    *self.state__ = CombinerParameterTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                CombinerParameterTypeDeserializerState::AttributeValue(Some(
                                    deserializer,
                                )),
                            );
                            *self.state__ = CombinerParameterTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = CombinerParameterTypeDeserializerState::AttributeValue(
                                Some(deserializer),
                            );
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::CombinerParameterType> for CombinerParameterTypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::CombinerParameterType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::CombinerParameterType>
        where
            R: DeserializeReader,
        {
            use CombinerParameterTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::AttributeValue(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_attribute_value(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ =
                            CombinerParameterTypeDeserializerState::AttributeValue(None);
                        event
                    }
                    (S::AttributeValue(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AttributeValue",
                            false,
                        )?;
                        match self.handle_attribute_value(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::CombinerParameterType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                CombinerParameterTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::CombinerParameterType {
                parameter_name: self.parameter_name,
                attribute_value: self
                    .attribute_value
                    .ok_or_else(|| ErrorKind::MissingElement("AttributeValue".into()))?,
            })
        }
    }
    #[derive(Debug)]
    pub struct CombinerParametersTypeDeserializer {
        combiner_parameter: Vec<super::CombinerParameter>,
        state__: Box<CombinerParametersTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum CombinerParametersTypeDeserializerState {
        Init__,
        CombinerParameter(Option<<super::CombinerParameter as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl CombinerParametersTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                combiner_parameter: Vec::new(),
                state__: Box::new(CombinerParametersTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: CombinerParametersTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use CombinerParametersTypeDeserializerState as S;
            match state {
                S::CombinerParameter(Some(deserializer)) => {
                    self.store_combiner_parameter(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_combiner_parameter(
            &mut self,
            value: super::CombinerParameter,
        ) -> std::result::Result<(), Error> {
            self.combiner_parameter.push(value);
            Ok(())
        }
        fn handle_combiner_parameter<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::CombinerParameter>,
            fallback: &mut Option<CombinerParametersTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(CombinerParametersTypeDeserializerState::CombinerParameter(
                    None,
                ));
                *self.state__ = CombinerParametersTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_combiner_parameter(data)?;
                    *self.state__ =
                        CombinerParametersTypeDeserializerState::CombinerParameter(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                CombinerParametersTypeDeserializerState::CombinerParameter(Some(
                                    deserializer,
                                )),
                            );
                            *self.state__ =
                                CombinerParametersTypeDeserializerState::CombinerParameter(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                CombinerParametersTypeDeserializerState::CombinerParameter(Some(
                                    deserializer,
                                ));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::CombinerParametersType> for CombinerParametersTypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::CombinerParametersType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::CombinerParametersType>
        where
            R: DeserializeReader,
        {
            use CombinerParametersTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::CombinerParameter(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_combiner_parameter(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ =
                            CombinerParametersTypeDeserializerState::CombinerParameter(None);
                        event
                    }
                    (S::CombinerParameter(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"CombinerParameter",
                            false,
                        )?;
                        match self.handle_combiner_parameter(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::CombinerParametersType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                CombinerParametersTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::CombinerParametersType {
                combiner_parameter: self.combiner_parameter,
            })
        }
    }
    #[derive(Debug)]
    pub struct ConditionTypeDeserializer {
        expression: Option<super::Expression>,
        state__: Box<ConditionTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum ConditionTypeDeserializerState {
        Init__,
        Expression(Option<<super::Expression as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl ConditionTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                expression: None,
                state__: Box::new(ConditionTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: ConditionTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use ConditionTypeDeserializerState as S;
            match state {
                S::Expression(Some(deserializer)) => {
                    self.store_expression(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_expression(&mut self, value: super::Expression) -> std::result::Result<(), Error> {
            if self.expression.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Expression",
                )))?;
            }
            self.expression = Some(value);
            Ok(())
        }
        fn handle_expression<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Expression>,
            fallback: &mut Option<ConditionTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.expression.is_some() {
                    fallback.get_or_insert(ConditionTypeDeserializerState::Expression(None));
                    *self.state__ = ConditionTypeDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                } else {
                    *self.state__ = ConditionTypeDeserializerState::Expression(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_expression(data)?;
                    *self.state__ = ConditionTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(ConditionTypeDeserializerState::Expression(
                                Some(deserializer),
                            ));
                            *self.state__ = ConditionTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                ConditionTypeDeserializerState::Expression(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::ConditionType> for ConditionTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::ConditionType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::ConditionType>
        where
            R: DeserializeReader,
        {
            use ConditionTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Expression(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_expression(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = ConditionTypeDeserializerState::Expression(None);
                        event
                    }
                    (S::Expression(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = <super::Expression as WithDeserializer>::Deserializer::init(
                            reader, event,
                        )?;
                        match self.handle_expression(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::ConditionType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                ConditionTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::ConditionType {
                expression: self
                    .expression
                    .ok_or_else(|| ErrorKind::MissingElement("Expression".into()))?,
            })
        }
    }
    #[derive(Debug)]
    pub struct ContentTypeDeserializer {
        text_before: Option<Text>,
        text_after_any_6: Option<Text>,
        state__: Box<ContentTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum ContentTypeDeserializerState {
        Init__,
        TextBefore(Option<<Text as WithDeserializer>::Deserializer>),
        TextAfterAny6(Option<<Text as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl ContentTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                text_before: None,
                text_after_any_6: None,
                state__: Box::new(ContentTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: ContentTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use ContentTypeDeserializerState as S;
            match state {
                S::TextBefore(Some(deserializer)) => {
                    self.store_text_before(deserializer.finish(reader)?)?
                }
                S::TextAfterAny6(Some(deserializer)) => {
                    self.store_text_after_any_6(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_text_before(&mut self, value: Text) -> std::result::Result<(), Error> {
            if self.text_before.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"text_before",
                )))?;
            }
            self.text_before = Some(value);
            Ok(())
        }
        fn store_text_after_any_6(&mut self, value: Text) -> std::result::Result<(), Error> {
            if self.text_after_any_6.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"text_after_any6",
                )))?;
            }
            self.text_after_any_6 = Some(value);
            Ok(())
        }
        fn handle_text_before<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, Text>,
            fallback: &mut Option<ContentTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(ContentTypeDeserializerState::TextBefore(None));
                *self.state__ = ContentTypeDeserializerState::TextAfterAny6(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_text_before(data)?;
                    *self.state__ = ContentTypeDeserializerState::TextAfterAny6(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(ContentTypeDeserializerState::TextBefore(Some(
                                deserializer,
                            )));
                            *self.state__ = ContentTypeDeserializerState::TextAfterAny6(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                ContentTypeDeserializerState::TextBefore(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_text_after_any_6<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, Text>,
            fallback: &mut Option<ContentTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(ContentTypeDeserializerState::TextAfterAny6(None));
                *self.state__ = ContentTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_text_after_any_6(data)?;
                    *self.state__ = ContentTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(ContentTypeDeserializerState::TextAfterAny6(
                                Some(deserializer),
                            ));
                            *self.state__ = ContentTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                ContentTypeDeserializerState::TextAfterAny6(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::ContentType> for ContentTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::ContentType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::ContentType>
        where
            R: DeserializeReader,
        {
            use ContentTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::TextBefore(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_text_before(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::TextAfterAny6(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_text_after_any_6(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        allow_any_element = true;
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = ContentTypeDeserializerState::TextBefore(None);
                        event
                    }
                    (S::TextBefore(None), event) => {
                        let output = <Text as WithDeserializer>::Deserializer::init(reader, event)?;
                        match self.handle_text_before(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::TextAfterAny6(None), event) => {
                        let output = <Text as WithDeserializer>::Deserializer::init(reader, event)?;
                        match self.handle_text_after_any_6(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), true);
                    }
                    (S::Unknown__, _) => unreachable!(),
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::ContentType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(&mut *self.state__, ContentTypeDeserializerState::Unknown__);
            self.finish_state(reader, state)?;
            Ok(super::ContentType {
                text_before: self.text_before,
                text_after_any_6: self.text_after_any_6,
            })
        }
    }
    #[derive(Debug)]
    pub struct DefaultsTypeDeserializer {
        content_39: Option<super::DefaultsContent39Type>,
        state__: Box<DefaultsTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum DefaultsTypeDeserializerState {
        Init__,
        Content39(Option<<super::DefaultsContent39Type as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl DefaultsTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                content_39: None,
                state__: Box::new(DefaultsTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: DefaultsTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use DefaultsTypeDeserializerState as S;
            match state {
                S::Content39(Some(deserializer)) => {
                    self.store_content_39(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_content_39(&mut self, value: super::DefaultsContent39Type) -> std::result::Result<(), Error> {
            if self.content_39.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Content39",
                )))?;
            }
            self.content_39 = Some(value);
            Ok(())
        }
        fn handle_content_39<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::DefaultsContent39Type>,
            fallback: &mut Option<DefaultsTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.content_39.is_some() {
                    fallback.get_or_insert(DefaultsTypeDeserializerState::Content39(None));
                    *self.state__ = DefaultsTypeDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                } else {
                    *self.state__ = DefaultsTypeDeserializerState::Content39(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_content_39(data)?;
                    *self.state__ = DefaultsTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(DefaultsTypeDeserializerState::Content39(Some(
                                deserializer,
                            )));
                            *self.state__ = DefaultsTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                DefaultsTypeDeserializerState::Content39(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::DefaultsType> for DefaultsTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::DefaultsType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::DefaultsType>
        where
            R: DeserializeReader,
        {
            use DefaultsTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Content39(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_content_39(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = DefaultsTypeDeserializerState::Content39(None);
                        event
                    }
                    (S::Content39(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output =
                            <super::DefaultsContent39Type as WithDeserializer>::Deserializer::init(
                                reader, event,
                            )?;
                        match self.handle_content_39(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::DefaultsType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(&mut *self.state__, DefaultsTypeDeserializerState::Unknown__);
            self.finish_state(reader, state)?;
            Ok(super::DefaultsType {
                content_39: self
                    .content_39
                    .ok_or_else(|| ErrorKind::MissingElement("Content39".into()))?,
            })
        }
    }
    #[derive(Debug)]
    pub enum ExpressionDeserializer {
        VariableReference(<super::VariableReference as WithDeserializer>::Deserializer),
        AttributeSelector(<super::AttributeSelector as WithDeserializer>::Deserializer),
        AttributeDesignator(<super::AttributeDesignator as WithDeserializer>::Deserializer),
        AttributeValue(<super::AttributeValue as WithDeserializer>::Deserializer),
        Function(<super::Function as WithDeserializer>::Deserializer),
        Apply(<super::Apply as WithDeserializer>::Deserializer),
    }
    impl<'de> Deserializer<'de, super::Expression> for ExpressionDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::Expression>
        where
            R: DeserializeReader,
        {
            let Some(type_name) = reader.get_dynamic_type_name(&event)? else {
                return Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::None,
                    event: DeserializerEvent::None,
                    allow_any: false,
                });
            };
            let type_name = type_name.into_owned();
            if matches!(
                reader.resolve_local_name(QName(&type_name), &super::NS_XACML),
                Some(b"VariableReference")
            ) {
                let DeserializerOutput {
                    artifact,
                    event,
                    allow_any,
                } = <super::VariableReference as WithDeserializer>::Deserializer::init(
                    reader, event,
                )?;
                return Ok(DeserializerOutput {
                    artifact: artifact.map(
                        |x| super::Expression(Box::new(x)),
                        |x| Self::VariableReference(x),
                    ),
                    event,
                    allow_any,
                });
            }
            if matches!(
                reader.resolve_local_name(QName(&type_name), &super::NS_XACML),
                Some(b"AttributeSelector")
            ) {
                let DeserializerOutput {
                    artifact,
                    event,
                    allow_any,
                } = <super::AttributeSelector as WithDeserializer>::Deserializer::init(
                    reader, event,
                )?;
                return Ok(DeserializerOutput {
                    artifact: artifact.map(
                        |x| super::Expression(Box::new(x)),
                        |x| Self::AttributeSelector(x),
                    ),
                    event,
                    allow_any,
                });
            }
            if matches!(
                reader.resolve_local_name(QName(&type_name), &super::NS_XACML),
                Some(b"AttributeDesignator")
            ) {
                let DeserializerOutput {
                    artifact,
                    event,
                    allow_any,
                } = <super::AttributeDesignator as WithDeserializer>::Deserializer::init(
                    reader, event,
                )?;
                return Ok(DeserializerOutput {
                    artifact: artifact.map(
                        |x| super::Expression(Box::new(x)),
                        |x| Self::AttributeDesignator(x),
                    ),
                    event,
                    allow_any,
                });
            }
            if matches!(
                reader.resolve_local_name(QName(&type_name), &super::NS_XACML),
                Some(b"AttributeValue")
            ) {
                let DeserializerOutput {
                    artifact,
                    event,
                    allow_any,
                } = <super::AttributeValue as WithDeserializer>::Deserializer::init(reader, event)?;
                return Ok(DeserializerOutput {
                    artifact: artifact.map(
                        |x| super::Expression(Box::new(x)),
                        |x| Self::AttributeValue(x),
                    ),
                    event,
                    allow_any,
                });
            }
            if matches!(
                reader.resolve_local_name(QName(&type_name), &super::NS_XACML),
                Some(b"Function")
            ) {
                let DeserializerOutput {
                    artifact,
                    event,
                    allow_any,
                } = <super::Function as WithDeserializer>::Deserializer::init(reader, event)?;
                return Ok(DeserializerOutput {
                    artifact: artifact
                        .map(|x| super::Expression(Box::new(x)), |x| Self::Function(x)),
                    event,
                    allow_any,
                });
            }
            if matches!(
                reader.resolve_local_name(QName(&type_name), &super::NS_XACML),
                Some(b"Apply")
            ) {
                let DeserializerOutput {
                    artifact,
                    event,
                    allow_any,
                } = <super::Apply as WithDeserializer>::Deserializer::init(reader, event)?;
                return Ok(DeserializerOutput {
                    artifact: artifact.map(|x| super::Expression(Box::new(x)), |x| Self::Apply(x)),
                    event,
                    allow_any,
                });
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::None,
                event: DeserializerEvent::Break(event),
                allow_any: false,
            })
        }
        fn next<R>(
            self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::Expression>
        where
            R: DeserializeReader,
        {
            match self {
                Self::VariableReference(x) => {
                    let DeserializerOutput {
                        artifact,
                        event,
                        allow_any,
                    } = x.next(reader, event)?;
                    Ok(DeserializerOutput {
                        artifact: artifact.map(
                            |x| super::Expression(Box::new(x)),
                            |x| Self::VariableReference(x),
                        ),
                        event,
                        allow_any,
                    })
                }
                Self::AttributeSelector(x) => {
                    let DeserializerOutput {
                        artifact,
                        event,
                        allow_any,
                    } = x.next(reader, event)?;
                    Ok(DeserializerOutput {
                        artifact: artifact.map(
                            |x| super::Expression(Box::new(x)),
                            |x| Self::AttributeSelector(x),
                        ),
                        event,
                        allow_any,
                    })
                }
                Self::AttributeDesignator(x) => {
                    let DeserializerOutput {
                        artifact,
                        event,
                        allow_any,
                    } = x.next(reader, event)?;
                    Ok(DeserializerOutput {
                        artifact: artifact.map(
                            |x| super::Expression(Box::new(x)),
                            |x| Self::AttributeDesignator(x),
                        ),
                        event,
                        allow_any,
                    })
                }
                Self::AttributeValue(x) => {
                    let DeserializerOutput {
                        artifact,
                        event,
                        allow_any,
                    } = x.next(reader, event)?;
                    Ok(DeserializerOutput {
                        artifact: artifact.map(
                            |x| super::Expression(Box::new(x)),
                            |x| Self::AttributeValue(x),
                        ),
                        event,
                        allow_any,
                    })
                }
                Self::Function(x) => {
                    let DeserializerOutput {
                        artifact,
                        event,
                        allow_any,
                    } = x.next(reader, event)?;
                    Ok(DeserializerOutput {
                        artifact: artifact
                            .map(|x| super::Expression(Box::new(x)), |x| Self::Function(x)),
                        event,
                        allow_any,
                    })
                }
                Self::Apply(x) => {
                    let DeserializerOutput {
                        artifact,
                        event,
                        allow_any,
                    } = x.next(reader, event)?;
                    Ok(DeserializerOutput {
                        artifact: artifact
                            .map(|x| super::Expression(Box::new(x)), |x| Self::Apply(x)),
                        event,
                        allow_any,
                    })
                }
            }
        }
        fn finish<R>(self, reader: &R) -> std::result::Result<super::Expression, Error>
        where
            R: DeserializeReader,
        {
            match self {
                Self::VariableReference(x) => Ok(super::Expression(Box::new(x.finish(reader)?))),
                Self::AttributeSelector(x) => Ok(super::Expression(Box::new(x.finish(reader)?))),
                Self::AttributeDesignator(x) => Ok(super::Expression(Box::new(x.finish(reader)?))),
                Self::AttributeValue(x) => Ok(super::Expression(Box::new(x.finish(reader)?))),
                Self::Function(x) => Ok(super::Expression(Box::new(x.finish(reader)?))),
                Self::Apply(x) => Ok(super::Expression(Box::new(x.finish(reader)?))),
            }
        }
    }
    #[derive(Debug)]
    pub struct ExpressionTypeDeserializer {
        state__: Box<ExpressionTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum ExpressionTypeDeserializerState {
        Init__,
        Unknown__,
    }
    impl ExpressionTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                state__: Box::new(ExpressionTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: ExpressionTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            Ok(())
        }
    }
    impl<'de> Deserializer<'de, super::ExpressionType> for ExpressionTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::ExpressionType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::ExpressionType>
        where
            R: DeserializeReader,
        {
            if let Event::End(_) = &event {
                Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::Data(self.finish(reader)?),
                    event: DeserializerEvent::None,
                    allow_any: false,
                })
            } else {
                Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::Deserializer(self),
                    event: DeserializerEvent::Break(event),
                    allow_any: false,
                })
            }
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::ExpressionType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                ExpressionTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::ExpressionType {})
        }
    }
    #[derive(Debug)]
    pub struct FunctionTypeDeserializer {
        function_id: super::AnyUriType,
        state__: Box<FunctionTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum FunctionTypeDeserializerState {
        Init__,
        Unknown__,
    }
    impl FunctionTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut function_id: Option<super::AnyUriType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"FunctionId")
                ) {
                    reader.read_attrib(&mut function_id, b"FunctionId", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                function_id: function_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("FunctionId".into()))
                })?,
                state__: Box::new(FunctionTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: FunctionTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            Ok(())
        }
    }
    impl<'de> Deserializer<'de, super::FunctionType> for FunctionTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::FunctionType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::FunctionType>
        where
            R: DeserializeReader,
        {
            if let Event::End(_) = &event {
                Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::Data(self.finish(reader)?),
                    event: DeserializerEvent::None,
                    allow_any: false,
                })
            } else {
                Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::Deserializer(self),
                    event: DeserializerEvent::Break(event),
                    allow_any: false,
                })
            }
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::FunctionType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(&mut *self.state__, FunctionTypeDeserializerState::Unknown__);
            self.finish_state(reader, state)?;
            Ok(super::FunctionType {
                function_id: self.function_id,
            })
        }
    }
    #[derive(Debug)]
    pub struct IdReferenceTypeDeserializer {
        version: Option<super::VersionMatchType>,
        earliest_version: Option<super::VersionMatchType>,
        latest_version: Option<super::VersionMatchType>,
        content: Option<super::AnyUriType>,
        state__: Box<IdReferenceTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum IdReferenceTypeDeserializerState {
        Init__,
        Content__(<super::AnyUriType as WithDeserializer>::Deserializer),
        Unknown__,
    }
    impl IdReferenceTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut version: Option<super::VersionMatchType> = None;
            let mut earliest_version: Option<super::VersionMatchType> = None;
            let mut latest_version: Option<super::VersionMatchType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Version")
                ) {
                    reader.read_attrib(&mut version, b"Version", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"EarliestVersion")
                ) {
                    reader.read_attrib(&mut earliest_version, b"EarliestVersion", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"LatestVersion")
                ) {
                    reader.read_attrib(&mut latest_version, b"LatestVersion", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                version: version,
                earliest_version: earliest_version,
                latest_version: latest_version,
                content: None,
                state__: Box::new(IdReferenceTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: IdReferenceTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            if let IdReferenceTypeDeserializerState::Content__(deserializer) = state {
                self.store_content(deserializer.finish(reader)?)?;
            }
            Ok(())
        }
        fn store_content(&mut self, value: super::AnyUriType) -> std::result::Result<(), Error> {
            if self.content.is_some() {
                Err(ErrorKind::DuplicateContent)?;
            }
            self.content = Some(value);
            Ok(())
        }
        fn handle_content<'de, R>(
            mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AnyUriType>,
        ) -> DeserializerResult<'de, super::IdReferenceType>
        where
            R: DeserializeReader,
        {
            use IdReferenceTypeDeserializerState as S;
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            match artifact {
                DeserializerArtifact::None => Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::None,
                    event,
                    allow_any,
                }),
                DeserializerArtifact::Data(data) => {
                    self.store_content(data)?;
                    let data = self.finish(reader)?;
                    Ok(DeserializerOutput {
                        artifact: DeserializerArtifact::Data(data),
                        event,
                        allow_any,
                    })
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ = S::Content__(deserializer);
                    Ok(DeserializerOutput {
                        artifact: DeserializerArtifact::Deserializer(self),
                        event,
                        allow_any,
                    })
                }
            }
        }
    }
    impl<'de> Deserializer<'de, super::IdReferenceType> for IdReferenceTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::IdReferenceType>
        where
            R: DeserializeReader,
        {
            let (Event::Start(x) | Event::Empty(x)) = &event else {
                return Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::None,
                    event: DeserializerEvent::Break(event),
                    allow_any: false,
                });
            };
            Self::from_bytes_start(reader, x)?.next(reader, event)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::IdReferenceType>
        where
            R: DeserializeReader,
        {
            use IdReferenceTypeDeserializerState as S;
            match replace(&mut *self.state__, S::Unknown__) {
                S::Init__ => {
                    let output = ContentDeserializer::init(reader, event)?;
                    self.handle_content(reader, output)
                }
                S::Content__(deserializer) => {
                    let output = deserializer.next(reader, event)?;
                    self.handle_content(reader, output)
                }
                S::Unknown__ => unreachable!(),
            }
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::IdReferenceType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                IdReferenceTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::IdReferenceType {
                version: self.version,
                earliest_version: self.earliest_version,
                latest_version: self.latest_version,
                content: self.content.ok_or_else(|| ErrorKind::MissingContent)?,
            })
        }
    }
    #[derive(Debug)]
    pub struct MatchTypeDeserializer {
        match_id: super::AnyUriType,
        attribute_value: Option<super::AttributeValue>,
        content_47: Option<super::MatchContent47Type>,
        state__: Box<MatchTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum MatchTypeDeserializerState {
        Init__,
        AttributeValue(Option<<super::AttributeValue as WithDeserializer>::Deserializer>),
        Content47(Option<<super::MatchContent47Type as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl MatchTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut match_id: Option<super::AnyUriType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"MatchId")
                ) {
                    reader.read_attrib(&mut match_id, b"MatchId", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                match_id: match_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("MatchId".into()))
                })?,
                attribute_value: None,
                content_47: None,
                state__: Box::new(MatchTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: MatchTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use MatchTypeDeserializerState as S;
            match state {
                S::AttributeValue(Some(deserializer)) => {
                    self.store_attribute_value(deserializer.finish(reader)?)?
                }
                S::Content47(Some(deserializer)) => {
                    self.store_content_47(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_attribute_value(&mut self, value: super::AttributeValue) -> std::result::Result<(), Error> {
            if self.attribute_value.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"AttributeValue",
                )))?;
            }
            self.attribute_value = Some(value);
            Ok(())
        }
        fn store_content_47(&mut self, value: super::MatchContent47Type) -> std::result::Result<(), Error> {
            if self.content_47.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Content47",
                )))?;
            }
            self.content_47 = Some(value);
            Ok(())
        }
        fn handle_attribute_value<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AttributeValue>,
            fallback: &mut Option<MatchTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.attribute_value.is_some() {
                    fallback.get_or_insert(MatchTypeDeserializerState::AttributeValue(None));
                    *self.state__ = MatchTypeDeserializerState::Content47(None);
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                } else {
                    *self.state__ = MatchTypeDeserializerState::AttributeValue(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_attribute_value(data)?;
                    *self.state__ = MatchTypeDeserializerState::Content47(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(MatchTypeDeserializerState::AttributeValue(
                                Some(deserializer),
                            ));
                            *self.state__ = MatchTypeDeserializerState::Content47(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                MatchTypeDeserializerState::AttributeValue(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_content_47<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::MatchContent47Type>,
            fallback: &mut Option<MatchTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.content_47.is_some() {
                    fallback.get_or_insert(MatchTypeDeserializerState::Content47(None));
                    *self.state__ = MatchTypeDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                } else {
                    *self.state__ = MatchTypeDeserializerState::Content47(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_content_47(data)?;
                    *self.state__ = MatchTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(MatchTypeDeserializerState::Content47(Some(
                                deserializer,
                            )));
                            *self.state__ = MatchTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                MatchTypeDeserializerState::Content47(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::MatchType> for MatchTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::MatchType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::MatchType>
        where
            R: DeserializeReader,
        {
            use MatchTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::AttributeValue(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_attribute_value(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Content47(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_content_47(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = MatchTypeDeserializerState::AttributeValue(None);
                        event
                    }
                    (S::AttributeValue(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AttributeValue",
                            false,
                        )?;
                        match self.handle_attribute_value(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Content47(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output =
                            <super::MatchContent47Type as WithDeserializer>::Deserializer::init(
                                reader, event,
                            )?;
                        match self.handle_content_47(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::MatchType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(&mut *self.state__, MatchTypeDeserializerState::Unknown__);
            self.finish_state(reader, state)?;
            Ok(super::MatchType {
                match_id: self.match_id,
                attribute_value: self
                    .attribute_value
                    .ok_or_else(|| ErrorKind::MissingElement("AttributeValue".into()))?,
                content_47: self
                    .content_47
                    .ok_or_else(|| ErrorKind::MissingElement("Content47".into()))?,
            })
        }
    }
    #[derive(Debug)]
    pub struct MissingAttributeDetailTypeDeserializer {
        category: super::AnyUriType,
        attribute_id: super::AnyUriType,
        data_type: super::AnyUriType,
        issuer: Option<super::StringType>,
        attribute_value: Vec<super::AttributeValue>,
        state__: Box<MissingAttributeDetailTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum MissingAttributeDetailTypeDeserializerState {
        Init__,
        AttributeValue(Option<<super::AttributeValue as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl MissingAttributeDetailTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut category: Option<super::AnyUriType> = None;
            let mut attribute_id: Option<super::AnyUriType> = None;
            let mut data_type: Option<super::AnyUriType> = None;
            let mut issuer: Option<super::StringType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Category")
                ) {
                    reader.read_attrib(&mut category, b"Category", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"AttributeId")
                ) {
                    reader.read_attrib(&mut attribute_id, b"AttributeId", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"DataType")
                ) {
                    reader.read_attrib(&mut data_type, b"DataType", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Issuer")
                ) {
                    reader.read_attrib(&mut issuer, b"Issuer", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                category: category.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("Category".into()))
                })?,
                attribute_id: attribute_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("AttributeId".into()))
                })?,
                data_type: data_type.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("DataType".into()))
                })?,
                issuer: issuer,
                attribute_value: Vec::new(),
                state__: Box::new(MissingAttributeDetailTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: MissingAttributeDetailTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use MissingAttributeDetailTypeDeserializerState as S;
            match state {
                S::AttributeValue(Some(deserializer)) => {
                    self.store_attribute_value(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_attribute_value(&mut self, value: super::AttributeValue) -> std::result::Result<(), Error> {
            self.attribute_value.push(value);
            Ok(())
        }
        fn handle_attribute_value<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AttributeValue>,
            fallback: &mut Option<MissingAttributeDetailTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(
                    MissingAttributeDetailTypeDeserializerState::AttributeValue(None),
                );
                *self.state__ = MissingAttributeDetailTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_attribute_value(data)?;
                    *self.state__ =
                        MissingAttributeDetailTypeDeserializerState::AttributeValue(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                MissingAttributeDetailTypeDeserializerState::AttributeValue(Some(
                                    deserializer,
                                )),
                            );
                            *self.state__ =
                                MissingAttributeDetailTypeDeserializerState::AttributeValue(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                MissingAttributeDetailTypeDeserializerState::AttributeValue(Some(
                                    deserializer,
                                ));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::MissingAttributeDetailType>
        for MissingAttributeDetailTypeDeserializer
    {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::MissingAttributeDetailType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::MissingAttributeDetailType>
        where
            R: DeserializeReader,
        {
            use MissingAttributeDetailTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::AttributeValue(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_attribute_value(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ =
                            MissingAttributeDetailTypeDeserializerState::AttributeValue(None);
                        event
                    }
                    (S::AttributeValue(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AttributeValue",
                            false,
                        )?;
                        match self.handle_attribute_value(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::MissingAttributeDetailType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                MissingAttributeDetailTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::MissingAttributeDetailType {
                category: self.category,
                attribute_id: self.attribute_id,
                data_type: self.data_type,
                issuer: self.issuer,
                attribute_value: self.attribute_value,
            })
        }
    }
    #[derive(Debug)]
    pub struct MultiRequestsTypeDeserializer {
        request_reference: Vec<super::RequestReference>,
        state__: Box<MultiRequestsTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum MultiRequestsTypeDeserializerState {
        Init__,
        RequestReference(Option<<super::RequestReference as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl MultiRequestsTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                request_reference: Vec::new(),
                state__: Box::new(MultiRequestsTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: MultiRequestsTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use MultiRequestsTypeDeserializerState as S;
            match state {
                S::RequestReference(Some(deserializer)) => {
                    self.store_request_reference(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_request_reference(&mut self, value: super::RequestReference) -> std::result::Result<(), Error> {
            self.request_reference.push(value);
            Ok(())
        }
        fn handle_request_reference<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::RequestReference>,
            fallback: &mut Option<MultiRequestsTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.request_reference.len() < 1usize {
                    *self.state__ = MultiRequestsTypeDeserializerState::RequestReference(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                } else {
                    fallback
                        .get_or_insert(MultiRequestsTypeDeserializerState::RequestReference(None));
                    *self.state__ = MultiRequestsTypeDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_request_reference(data)?;
                    *self.state__ = MultiRequestsTypeDeserializerState::RequestReference(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                MultiRequestsTypeDeserializerState::RequestReference(Some(
                                    deserializer,
                                )),
                            );
                            if self.request_reference.len().saturating_add(1) < 1usize {
                                *self.state__ =
                                    MultiRequestsTypeDeserializerState::RequestReference(None);
                            } else {
                                *self.state__ = MultiRequestsTypeDeserializerState::Done__;
                            }
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = MultiRequestsTypeDeserializerState::RequestReference(
                                Some(deserializer),
                            );
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::MultiRequestsType> for MultiRequestsTypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::MultiRequestsType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::MultiRequestsType>
        where
            R: DeserializeReader,
        {
            use MultiRequestsTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::RequestReference(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_request_reference(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = MultiRequestsTypeDeserializerState::RequestReference(None);
                        event
                    }
                    (S::RequestReference(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"RequestReference",
                            false,
                        )?;
                        match self.handle_request_reference(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::MultiRequestsType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                MultiRequestsTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::MultiRequestsType {
                request_reference: self.request_reference,
            })
        }
    }
    #[derive(Debug)]
    pub struct ObligationExpressionTypeDeserializer {
        obligation_id: super::AnyUriType,
        fulfill_on: super::EffectType,
        attribute_assignment_expression: Vec<super::AttributeAssignmentExpression>,
        state__: Box<ObligationExpressionTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum ObligationExpressionTypeDeserializerState {
        Init__,
        AttributeAssignmentExpression(
            Option<<super::AttributeAssignmentExpression as WithDeserializer>::Deserializer>,
        ),
        Done__,
        Unknown__,
    }
    impl ObligationExpressionTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut obligation_id: Option<super::AnyUriType> = None;
            let mut fulfill_on: Option<super::EffectType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"ObligationId")
                ) {
                    reader.read_attrib(&mut obligation_id, b"ObligationId", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"FulfillOn")
                ) {
                    reader.read_attrib(&mut fulfill_on, b"FulfillOn", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                obligation_id: obligation_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("ObligationId".into()))
                })?,
                fulfill_on: fulfill_on.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("FulfillOn".into()))
                })?,
                attribute_assignment_expression: Vec::new(),
                state__: Box::new(ObligationExpressionTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: ObligationExpressionTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use ObligationExpressionTypeDeserializerState as S;
            match state {
                S::AttributeAssignmentExpression(Some(deserializer)) => {
                    self.store_attribute_assignment_expression(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_attribute_assignment_expression(
            &mut self,
            value: super::AttributeAssignmentExpression,
        ) -> std::result::Result<(), Error> {
            self.attribute_assignment_expression.push(value);
            Ok(())
        }
        fn handle_attribute_assignment_expression<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AttributeAssignmentExpression>,
            fallback: &mut Option<ObligationExpressionTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(
                    ObligationExpressionTypeDeserializerState::AttributeAssignmentExpression(None),
                );
                *self.state__ = ObligationExpressionTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_attribute_assignment_expression(data)?;
                    *self.state__ =
                        ObligationExpressionTypeDeserializerState::AttributeAssignmentExpression(
                            None,
                        );
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback . get_or_insert (ObligationExpressionTypeDeserializerState :: AttributeAssignmentExpression (Some (deserializer))) ;
                            * self . state__ = ObligationExpressionTypeDeserializerState :: AttributeAssignmentExpression (None) ;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            * self . state__ = ObligationExpressionTypeDeserializerState :: AttributeAssignmentExpression (Some (deserializer)) ;
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::ObligationExpressionType>
        for ObligationExpressionTypeDeserializer
    {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::ObligationExpressionType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::ObligationExpressionType>
        where
            R: DeserializeReader,
        {
            use ObligationExpressionTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::AttributeAssignmentExpression(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_attribute_assignment_expression(
                            reader,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        * self . state__ = ObligationExpressionTypeDeserializerState :: AttributeAssignmentExpression (None) ;
                        event
                    }
                    (
                        S::AttributeAssignmentExpression(None),
                        event @ (Event::Start(_) | Event::Empty(_)),
                    ) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AttributeAssignmentExpression",
                            false,
                        )?;
                        match self.handle_attribute_assignment_expression(
                            reader,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::ObligationExpressionType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                ObligationExpressionTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::ObligationExpressionType {
                obligation_id: self.obligation_id,
                fulfill_on: self.fulfill_on,
                attribute_assignment_expression: self.attribute_assignment_expression,
            })
        }
    }
    #[derive(Debug)]
    pub struct ObligationExpressionsTypeDeserializer {
        obligation_expression: Vec<super::ObligationExpression>,
        state__: Box<ObligationExpressionsTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum ObligationExpressionsTypeDeserializerState {
        Init__,
        ObligationExpression(
            Option<<super::ObligationExpression as WithDeserializer>::Deserializer>,
        ),
        Done__,
        Unknown__,
    }
    impl ObligationExpressionsTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                obligation_expression: Vec::new(),
                state__: Box::new(ObligationExpressionsTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: ObligationExpressionsTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use ObligationExpressionsTypeDeserializerState as S;
            match state {
                S::ObligationExpression(Some(deserializer)) => {
                    self.store_obligation_expression(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_obligation_expression(
            &mut self,
            value: super::ObligationExpression,
        ) -> std::result::Result<(), Error> {
            self.obligation_expression.push(value);
            Ok(())
        }
        fn handle_obligation_expression<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::ObligationExpression>,
            fallback: &mut Option<ObligationExpressionsTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.obligation_expression.len() < 1usize {
                    *self.state__ =
                        ObligationExpressionsTypeDeserializerState::ObligationExpression(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                } else {
                    fallback.get_or_insert(
                        ObligationExpressionsTypeDeserializerState::ObligationExpression(None),
                    );
                    *self.state__ = ObligationExpressionsTypeDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_obligation_expression(data)?;
                    *self.state__ =
                        ObligationExpressionsTypeDeserializerState::ObligationExpression(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                ObligationExpressionsTypeDeserializerState::ObligationExpression(
                                    Some(deserializer),
                                ),
                            );
                            if self.obligation_expression.len().saturating_add(1) < 1usize {
                                * self . state__ = ObligationExpressionsTypeDeserializerState :: ObligationExpression (None) ;
                            } else {
                                *self.state__ = ObligationExpressionsTypeDeserializerState::Done__;
                            }
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                ObligationExpressionsTypeDeserializerState::ObligationExpression(
                                    Some(deserializer),
                                );
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::ObligationExpressionsType>
        for ObligationExpressionsTypeDeserializer
    {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::ObligationExpressionsType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::ObligationExpressionsType>
        where
            R: DeserializeReader,
        {
            use ObligationExpressionsTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::ObligationExpression(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_obligation_expression(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ =
                            ObligationExpressionsTypeDeserializerState::ObligationExpression(None);
                        event
                    }
                    (
                        S::ObligationExpression(None),
                        event @ (Event::Start(_) | Event::Empty(_)),
                    ) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"ObligationExpression",
                            false,
                        )?;
                        match self.handle_obligation_expression(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::ObligationExpressionsType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                ObligationExpressionsTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::ObligationExpressionsType {
                obligation_expression: self.obligation_expression,
            })
        }
    }
    #[derive(Debug)]
    pub struct ObligationTypeDeserializer {
        obligation_id: super::AnyUriType,
        attribute_assignment: Vec<super::AttributeAssignment>,
        state__: Box<ObligationTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum ObligationTypeDeserializerState {
        Init__,
        AttributeAssignment(Option<<super::AttributeAssignment as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl ObligationTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut obligation_id: Option<super::AnyUriType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"ObligationId")
                ) {
                    reader.read_attrib(&mut obligation_id, b"ObligationId", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                obligation_id: obligation_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("ObligationId".into()))
                })?,
                attribute_assignment: Vec::new(),
                state__: Box::new(ObligationTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: ObligationTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use ObligationTypeDeserializerState as S;
            match state {
                S::AttributeAssignment(Some(deserializer)) => {
                    self.store_attribute_assignment(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_attribute_assignment(
            &mut self,
            value: super::AttributeAssignment,
        ) -> std::result::Result<(), Error> {
            self.attribute_assignment.push(value);
            Ok(())
        }
        fn handle_attribute_assignment<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AttributeAssignment>,
            fallback: &mut Option<ObligationTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(ObligationTypeDeserializerState::AttributeAssignment(None));
                *self.state__ = ObligationTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_attribute_assignment(data)?;
                    *self.state__ = ObligationTypeDeserializerState::AttributeAssignment(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                ObligationTypeDeserializerState::AttributeAssignment(Some(
                                    deserializer,
                                )),
                            );
                            *self.state__ =
                                ObligationTypeDeserializerState::AttributeAssignment(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = ObligationTypeDeserializerState::AttributeAssignment(
                                Some(deserializer),
                            );
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::ObligationType> for ObligationTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::ObligationType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::ObligationType>
        where
            R: DeserializeReader,
        {
            use ObligationTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::AttributeAssignment(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_attribute_assignment(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = ObligationTypeDeserializerState::AttributeAssignment(None);
                        event
                    }
                    (S::AttributeAssignment(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AttributeAssignment",
                            false,
                        )?;
                        match self.handle_attribute_assignment(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::ObligationType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                ObligationTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::ObligationType {
                obligation_id: self.obligation_id,
                attribute_assignment: self.attribute_assignment,
            })
        }
    }
    #[derive(Debug)]
    pub struct ObligationsTypeDeserializer {
        obligation: Vec<super::Obligation>,
        state__: Box<ObligationsTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum ObligationsTypeDeserializerState {
        Init__,
        Obligation(Option<<super::Obligation as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl ObligationsTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                obligation: Vec::new(),
                state__: Box::new(ObligationsTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: ObligationsTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use ObligationsTypeDeserializerState as S;
            match state {
                S::Obligation(Some(deserializer)) => {
                    self.store_obligation(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_obligation(&mut self, value: super::Obligation) -> std::result::Result<(), Error> {
            self.obligation.push(value);
            Ok(())
        }
        fn handle_obligation<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Obligation>,
            fallback: &mut Option<ObligationsTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.obligation.len() < 1usize {
                    *self.state__ = ObligationsTypeDeserializerState::Obligation(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                } else {
                    fallback.get_or_insert(ObligationsTypeDeserializerState::Obligation(None));
                    *self.state__ = ObligationsTypeDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_obligation(data)?;
                    *self.state__ = ObligationsTypeDeserializerState::Obligation(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(ObligationsTypeDeserializerState::Obligation(
                                Some(deserializer),
                            ));
                            if self.obligation.len().saturating_add(1) < 1usize {
                                *self.state__ = ObligationsTypeDeserializerState::Obligation(None);
                            } else {
                                *self.state__ = ObligationsTypeDeserializerState::Done__;
                            }
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                ObligationsTypeDeserializerState::Obligation(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::ObligationsType> for ObligationsTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::ObligationsType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::ObligationsType>
        where
            R: DeserializeReader,
        {
            use ObligationsTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Obligation(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_obligation(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = ObligationsTypeDeserializerState::Obligation(None);
                        event
                    }
                    (S::Obligation(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Obligation",
                            false,
                        )?;
                        match self.handle_obligation(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::ObligationsType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                ObligationsTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::ObligationsType {
                obligation: self.obligation,
            })
        }
    }
    #[derive(Debug)]
    pub struct PolicyCombinerParametersTypeDeserializer {
        policy_id_ref: super::AnyUriType,
        combiner_parameter: Vec<super::CombinerParameter>,
        state__: Box<PolicyCombinerParametersTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum PolicyCombinerParametersTypeDeserializerState {
        Init__,
        CombinerParameter(Option<<super::CombinerParameter as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl PolicyCombinerParametersTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut policy_id_ref: Option<super::AnyUriType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"PolicyIdRef")
                ) {
                    reader.read_attrib(&mut policy_id_ref, b"PolicyIdRef", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                policy_id_ref: policy_id_ref.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("PolicyIdRef".into()))
                })?,
                combiner_parameter: Vec::new(),
                state__: Box::new(PolicyCombinerParametersTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: PolicyCombinerParametersTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use PolicyCombinerParametersTypeDeserializerState as S;
            match state {
                S::CombinerParameter(Some(deserializer)) => {
                    self.store_combiner_parameter(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_combiner_parameter(
            &mut self,
            value: super::CombinerParameter,
        ) -> std::result::Result<(), Error> {
            self.combiner_parameter.push(value);
            Ok(())
        }
        fn handle_combiner_parameter<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::CombinerParameter>,
            fallback: &mut Option<PolicyCombinerParametersTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(
                    PolicyCombinerParametersTypeDeserializerState::CombinerParameter(None),
                );
                *self.state__ = PolicyCombinerParametersTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_combiner_parameter(data)?;
                    *self.state__ =
                        PolicyCombinerParametersTypeDeserializerState::CombinerParameter(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                PolicyCombinerParametersTypeDeserializerState::CombinerParameter(
                                    Some(deserializer),
                                ),
                            );
                            *self.state__ =
                                PolicyCombinerParametersTypeDeserializerState::CombinerParameter(
                                    None,
                                );
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                PolicyCombinerParametersTypeDeserializerState::CombinerParameter(
                                    Some(deserializer),
                                );
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::PolicyCombinerParametersType>
        for PolicyCombinerParametersTypeDeserializer
    {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::PolicyCombinerParametersType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::PolicyCombinerParametersType>
        where
            R: DeserializeReader,
        {
            use PolicyCombinerParametersTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::CombinerParameter(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_combiner_parameter(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ =
                            PolicyCombinerParametersTypeDeserializerState::CombinerParameter(None);
                        event
                    }
                    (S::CombinerParameter(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"CombinerParameter",
                            false,
                        )?;
                        match self.handle_combiner_parameter(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::PolicyCombinerParametersType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                PolicyCombinerParametersTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::PolicyCombinerParametersType {
                policy_id_ref: self.policy_id_ref,
                combiner_parameter: self.combiner_parameter,
            })
        }
    }
    #[derive(Debug)]
    pub struct PolicyIdentifierListTypeDeserializer {
        content: Vec<super::PolicyIdentifierListTypeContent>,
        state__: Box<PolicyIdentifierListTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum PolicyIdentifierListTypeDeserializerState {
        Init__,
        Next__,
        Content__(<super::PolicyIdentifierListTypeContent as WithDeserializer>::Deserializer),
        Unknown__,
    }
    impl PolicyIdentifierListTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                content: Vec::new(),
                state__: Box::new(PolicyIdentifierListTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: PolicyIdentifierListTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            if let PolicyIdentifierListTypeDeserializerState::Content__(deserializer) = state {
                self.store_content(deserializer.finish(reader)?)?;
            }
            Ok(())
        }
        fn store_content(
            &mut self,
            value: super::PolicyIdentifierListTypeContent,
        ) -> std::result::Result<(), Error> {
            self.content.push(value);
            Ok(())
        }
        fn handle_content<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::PolicyIdentifierListTypeContent>,
            fallback: &mut Option<PolicyIdentifierListTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = fallback
                    .take()
                    .unwrap_or(PolicyIdentifierListTypeDeserializerState::Next__);
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_content(data)?;
                    *self.state__ = PolicyIdentifierListTypeDeserializerState::Next__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                PolicyIdentifierListTypeDeserializerState::Content__(deserializer);
                        }
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                PolicyIdentifierListTypeDeserializerState::Content__(deserializer),
                            );
                            *self.state__ = PolicyIdentifierListTypeDeserializerState::Next__;
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::PolicyIdentifierListType>
        for PolicyIdentifierListTypeDeserializer
    {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::PolicyIdentifierListType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::PolicyIdentifierListType>
        where
            R: DeserializeReader,
        {
            use PolicyIdentifierListTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Content__(deserializer), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_content(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (_, Event::End(_)) => {
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (state @ (S::Init__ | S::Next__), event) => {
                        fallback.get_or_insert(state);
                        let output = < super :: PolicyIdentifierListTypeContent as WithDeserializer > :: Deserializer :: init (reader , event) ? ;
                        match self.handle_content(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::Unknown__, _) => unreachable!(),
                }
            };
            let artifact = DeserializerArtifact::Deserializer(self);
            Ok(DeserializerOutput {
                artifact,
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::PolicyIdentifierListType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                PolicyIdentifierListTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::PolicyIdentifierListType {
                content: self.content,
            })
        }
    }
    #[derive(Debug)]
    pub struct PolicyIdentifierListTypeContentDeserializer {
        state__: Box<PolicyIdentifierListTypeContentDeserializerState>,
    }
    #[derive(Debug)]
    pub enum PolicyIdentifierListTypeContentDeserializerState {
        Init__,
        PolicyIdReference(
            Option<super::PolicyIdReference>,
            Option<<super::PolicyIdReference as WithDeserializer>::Deserializer>,
        ),
        PolicySetIdReference(
            Option<super::PolicySetIdReference>,
            Option<<super::PolicySetIdReference as WithDeserializer>::Deserializer>,
        ),
        Done__(super::PolicyIdentifierListTypeContent),
        Unknown__,
    }
    impl PolicyIdentifierListTypeContentDeserializer {
        fn find_suitable<'de, R>(
            &mut self,
            reader: &R,
            event: Event<'de>,
            fallback: &mut Option<PolicyIdentifierListTypeContentDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            if let Event::Start(x) | Event::Empty(x) = &event {
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"PolicyIdReference")
                ) {
                    let output =
                        <super::PolicyIdReference as WithDeserializer>::Deserializer::init(
                            reader, event,
                        )?;
                    return self.handle_policy_id_reference(
                        reader,
                        Default::default(),
                        output,
                        &mut *fallback,
                    );
                }
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"PolicySetIdReference")
                ) {
                    let output =
                        <super::PolicySetIdReference as WithDeserializer>::Deserializer::init(
                            reader, event,
                        )?;
                    return self.handle_policy_set_id_reference(
                        reader,
                        Default::default(),
                        output,
                        &mut *fallback,
                    );
                }
            }
            *self.state__ = fallback
                .take()
                .unwrap_or(PolicyIdentifierListTypeContentDeserializerState::Init__);
            Ok(ElementHandlerOutput::return_to_parent(event, false))
        }
        fn finish_state<R>(
            reader: &R,
            state: PolicyIdentifierListTypeContentDeserializerState,
        ) -> std::result::Result<super::PolicyIdentifierListTypeContent, Error>
        where
            R: DeserializeReader,
        {
            use PolicyIdentifierListTypeContentDeserializerState as S;
            match state {
                S::Init__ => Err(ErrorKind::MissingContent.into()),
                S::PolicyIdReference(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_policy_id_reference(&mut values, value)?;
                    }
                    Ok(super::PolicyIdentifierListTypeContent::PolicyIdReference(
                        values
                            .ok_or_else(|| ErrorKind::MissingElement("PolicyIdReference".into()))?,
                    ))
                }
                S::PolicySetIdReference(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_policy_set_id_reference(&mut values, value)?;
                    }
                    Ok(
                        super::PolicyIdentifierListTypeContent::PolicySetIdReference(
                            values.ok_or_else(|| {
                                ErrorKind::MissingElement("PolicySetIdReference".into())
                            })?,
                        ),
                    )
                }
                S::Done__(data) => Ok(data),
                S::Unknown__ => unreachable!(),
            }
        }
        fn store_policy_id_reference(
            values: &mut Option<super::PolicyIdReference>,
            value: super::PolicyIdReference,
        ) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"PolicyIdReference",
                )))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn store_policy_set_id_reference(
            values: &mut Option<super::PolicySetIdReference>,
            value: super::PolicySetIdReference,
        ) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"PolicySetIdReference",
                )))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn handle_policy_id_reference<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::PolicyIdReference>,
            output: DeserializerOutput<'de, super::PolicyIdReference>,
            fallback: &mut Option<PolicyIdentifierListTypeContentDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None if values.is_none() => {
                        *self.state__ = PolicyIdentifierListTypeContentDeserializerState::Init__;
                        return Ok(ElementHandlerOutput::from_event(event, allow_any));
                    }
                    None => PolicyIdentifierListTypeContentDeserializerState::PolicyIdReference(
                        values, None,
                    ),
                    Some(PolicyIdentifierListTypeContentDeserializerState::PolicyIdReference(
                        _,
                        Some(deserializer),
                    )) => PolicyIdentifierListTypeContentDeserializerState::PolicyIdReference(
                        values,
                        Some(deserializer),
                    ),
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(PolicyIdentifierListTypeContentDeserializerState::PolicyIdReference(
                    _,
                    Some(deserializer),
                )) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_policy_id_reference(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_policy_id_reference(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        PolicyIdentifierListTypeContentDeserializerState::PolicyIdReference(
                            values, None,
                        ),
                    )?;
                    *self.state__ = PolicyIdentifierListTypeContentDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ =
                        PolicyIdentifierListTypeContentDeserializerState::PolicyIdReference(
                            values,
                            Some(deserializer),
                        );
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
        fn handle_policy_set_id_reference<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::PolicySetIdReference>,
            output: DeserializerOutput<'de, super::PolicySetIdReference>,
            fallback: &mut Option<PolicyIdentifierListTypeContentDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None if values.is_none() => {
                        *self.state__ = PolicyIdentifierListTypeContentDeserializerState::Init__;
                        return Ok(ElementHandlerOutput::from_event(event, allow_any));
                    }
                    None => PolicyIdentifierListTypeContentDeserializerState::PolicySetIdReference(
                        values, None,
                    ),
                    Some(
                        PolicyIdentifierListTypeContentDeserializerState::PolicySetIdReference(
                            _,
                            Some(deserializer),
                        ),
                    ) => PolicyIdentifierListTypeContentDeserializerState::PolicySetIdReference(
                        values,
                        Some(deserializer),
                    ),
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(PolicyIdentifierListTypeContentDeserializerState::PolicySetIdReference(
                    _,
                    Some(deserializer),
                )) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_policy_set_id_reference(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_policy_set_id_reference(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        PolicyIdentifierListTypeContentDeserializerState::PolicySetIdReference(
                            values, None,
                        ),
                    )?;
                    *self.state__ = PolicyIdentifierListTypeContentDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ =
                        PolicyIdentifierListTypeContentDeserializerState::PolicySetIdReference(
                            values,
                            Some(deserializer),
                        );
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::PolicyIdentifierListTypeContent>
        for PolicyIdentifierListTypeContentDeserializer
    {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::PolicyIdentifierListTypeContent>
        where
            R: DeserializeReader,
        {
            let deserializer = Self {
                state__: Box::new(PolicyIdentifierListTypeContentDeserializerState::Init__),
            };
            let mut output = deserializer.next(reader, event)?;
            output.artifact = match output.artifact {
                DeserializerArtifact::Deserializer(x)
                    if matches!(
                        &*x.state__,
                        PolicyIdentifierListTypeContentDeserializerState::Init__
                    ) =>
                {
                    DeserializerArtifact::None
                }
                artifact => artifact,
            };
            Ok(output)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::PolicyIdentifierListTypeContent>
        where
            R: DeserializeReader,
        {
            use PolicyIdentifierListTypeContentDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::PolicyIdReference(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_policy_id_reference(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::PolicySetIdReference(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_policy_set_id_reference(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (state, event @ Event::End(_)) => {
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(Self::finish_state(
                                reader, state,
                            )?),
                            event: DeserializerEvent::Continue(event),
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => match self.find_suitable(reader, event, &mut fallback)? {
                        ElementHandlerOutput::Break { event, allow_any } => {
                            break (event, allow_any)
                        }
                        ElementHandlerOutput::Continue { event, .. } => event,
                    },
                    (S::PolicyIdReference(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"PolicyIdReference",
                            false,
                        )?;
                        match self.handle_policy_id_reference(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::PolicySetIdReference(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"PolicySetIdReference",
                            false,
                        )?;
                        match self.handle_policy_set_id_reference(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (s @ S::Done__(_), event) => {
                        *self.state__ = s;
                        break (DeserializerEvent::Continue(event), false);
                    }
                    (S::Unknown__, _) => unreachable!(),
                }
            };
            let artifact = if matches!(&*self.state__, S::Done__(_)) {
                DeserializerArtifact::Data(self.finish(reader)?)
            } else {
                DeserializerArtifact::Deserializer(self)
            };
            Ok(DeserializerOutput {
                artifact,
                event,
                allow_any,
            })
        }
        fn finish<R>(self, reader: &R) -> std::result::Result<super::PolicyIdentifierListTypeContent, Error>
        where
            R: DeserializeReader,
        {
            Self::finish_state(reader, *self.state__)
        }
    }
    #[derive(Debug)]
    pub struct PolicyIssuerTypeDeserializer {
        content: Option<super::Content>,
        attribute: Vec<super::Attribute>,
        state__: Box<PolicyIssuerTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum PolicyIssuerTypeDeserializerState {
        Init__,
        Content(Option<<super::Content as WithDeserializer>::Deserializer>),
        Attribute(Option<<super::Attribute as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl PolicyIssuerTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                content: None,
                attribute: Vec::new(),
                state__: Box::new(PolicyIssuerTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: PolicyIssuerTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use PolicyIssuerTypeDeserializerState as S;
            match state {
                S::Content(Some(deserializer)) => {
                    self.store_content(deserializer.finish(reader)?)?
                }
                S::Attribute(Some(deserializer)) => {
                    self.store_attribute(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_content(&mut self, value: super::Content) -> std::result::Result<(), Error> {
            if self.content.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Content",
                )))?;
            }
            self.content = Some(value);
            Ok(())
        }
        fn store_attribute(&mut self, value: super::Attribute) -> std::result::Result<(), Error> {
            self.attribute.push(value);
            Ok(())
        }
        fn handle_content<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Content>,
            fallback: &mut Option<PolicyIssuerTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(PolicyIssuerTypeDeserializerState::Content(None));
                *self.state__ = PolicyIssuerTypeDeserializerState::Attribute(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_content(data)?;
                    *self.state__ = PolicyIssuerTypeDeserializerState::Attribute(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(PolicyIssuerTypeDeserializerState::Content(
                                Some(deserializer),
                            ));
                            *self.state__ = PolicyIssuerTypeDeserializerState::Attribute(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                PolicyIssuerTypeDeserializerState::Content(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_attribute<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Attribute>,
            fallback: &mut Option<PolicyIssuerTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(PolicyIssuerTypeDeserializerState::Attribute(None));
                *self.state__ = PolicyIssuerTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_attribute(data)?;
                    *self.state__ = PolicyIssuerTypeDeserializerState::Attribute(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(PolicyIssuerTypeDeserializerState::Attribute(
                                Some(deserializer),
                            ));
                            *self.state__ = PolicyIssuerTypeDeserializerState::Attribute(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                PolicyIssuerTypeDeserializerState::Attribute(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::PolicyIssuerType> for PolicyIssuerTypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::PolicyIssuerType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::PolicyIssuerType>
        where
            R: DeserializeReader,
        {
            use PolicyIssuerTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Content(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_content(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Attribute(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_attribute(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = PolicyIssuerTypeDeserializerState::Content(None);
                        event
                    }
                    (S::Content(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Content",
                            false,
                        )?;
                        match self.handle_content(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Attribute(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Attribute",
                            false,
                        )?;
                        match self.handle_attribute(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::PolicyIssuerType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                PolicyIssuerTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::PolicyIssuerType {
                content: self.content,
                attribute: self.attribute,
            })
        }
    }
    #[derive(Debug)]
    pub struct PolicySetCombinerParametersTypeDeserializer {
        policy_set_id_ref: super::AnyUriType,
        combiner_parameter: Vec<super::CombinerParameter>,
        state__: Box<PolicySetCombinerParametersTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum PolicySetCombinerParametersTypeDeserializerState {
        Init__,
        CombinerParameter(Option<<super::CombinerParameter as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl PolicySetCombinerParametersTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut policy_set_id_ref: Option<super::AnyUriType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"PolicySetIdRef")
                ) {
                    reader.read_attrib(&mut policy_set_id_ref, b"PolicySetIdRef", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                policy_set_id_ref: policy_set_id_ref.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("PolicySetIdRef".into()))
                })?,
                combiner_parameter: Vec::new(),
                state__: Box::new(PolicySetCombinerParametersTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: PolicySetCombinerParametersTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use PolicySetCombinerParametersTypeDeserializerState as S;
            match state {
                S::CombinerParameter(Some(deserializer)) => {
                    self.store_combiner_parameter(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_combiner_parameter(
            &mut self,
            value: super::CombinerParameter,
        ) -> std::result::Result<(), Error> {
            self.combiner_parameter.push(value);
            Ok(())
        }
        fn handle_combiner_parameter<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::CombinerParameter>,
            fallback: &mut Option<PolicySetCombinerParametersTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(
                    PolicySetCombinerParametersTypeDeserializerState::CombinerParameter(None),
                );
                *self.state__ = PolicySetCombinerParametersTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_combiner_parameter(data)?;
                    *self.state__ =
                        PolicySetCombinerParametersTypeDeserializerState::CombinerParameter(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                PolicySetCombinerParametersTypeDeserializerState::CombinerParameter(
                                    Some(deserializer),
                                ),
                            );
                            *self.state__ =
                                PolicySetCombinerParametersTypeDeserializerState::CombinerParameter(
                                    None,
                                );
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                PolicySetCombinerParametersTypeDeserializerState::CombinerParameter(
                                    Some(deserializer),
                                );
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::PolicySetCombinerParametersType>
        for PolicySetCombinerParametersTypeDeserializer
    {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::PolicySetCombinerParametersType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::PolicySetCombinerParametersType>
        where
            R: DeserializeReader,
        {
            use PolicySetCombinerParametersTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::CombinerParameter(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_combiner_parameter(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ =
                            PolicySetCombinerParametersTypeDeserializerState::CombinerParameter(
                                None,
                            );
                        event
                    }
                    (S::CombinerParameter(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"CombinerParameter",
                            false,
                        )?;
                        match self.handle_combiner_parameter(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::PolicySetCombinerParametersType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                PolicySetCombinerParametersTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::PolicySetCombinerParametersType {
                policy_set_id_ref: self.policy_set_id_ref,
                combiner_parameter: self.combiner_parameter,
            })
        }
    }
    #[derive(Debug)]
    pub struct PolicySetTypeDeserializer {
        policy_set_id: super::AnyUriType,
        version: super::VersionType,
        policy_combining_alg_id: super::AnyUriType,
        max_delegation_depth: Option<super::IntegerType>,
        description: Option<super::Description>,
        policy_issuer: Option<super::PolicyIssuer>,
        policy_set_defaults: Option<super::PolicySetDefaults>,
        target: Option<super::Target>,
        content_31: Vec<super::PolicySetContent31Type>,
        obligation_expressions: Option<super::ObligationExpressions>,
        advice_expressions: Option<super::AdviceExpressions>,
        state__: Box<PolicySetTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum PolicySetTypeDeserializerState {
        Init__,
        Description(Option<<super::Description as WithDeserializer>::Deserializer>),
        PolicyIssuer(Option<<super::PolicyIssuer as WithDeserializer>::Deserializer>),
        PolicySetDefaults(Option<<super::PolicySetDefaults as WithDeserializer>::Deserializer>),
        Target(Option<<super::Target as WithDeserializer>::Deserializer>),
        Content31(Option<<super::PolicySetContent31Type as WithDeserializer>::Deserializer>),
        ObligationExpressions(
            Option<<super::ObligationExpressions as WithDeserializer>::Deserializer>,
        ),
        AdviceExpressions(Option<<super::AdviceExpressions as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl PolicySetTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut policy_set_id: Option<super::AnyUriType> = None;
            let mut version: Option<super::VersionType> = None;
            let mut policy_combining_alg_id: Option<super::AnyUriType> = None;
            let mut max_delegation_depth: Option<super::IntegerType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"PolicySetId")
                ) {
                    reader.read_attrib(&mut policy_set_id, b"PolicySetId", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Version")
                ) {
                    reader.read_attrib(&mut version, b"Version", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"PolicyCombiningAlgId")
                ) {
                    reader.read_attrib(
                        &mut policy_combining_alg_id,
                        b"PolicyCombiningAlgId",
                        &attrib.value,
                    )?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"MaxDelegationDepth")
                ) {
                    reader.read_attrib(
                        &mut max_delegation_depth,
                        b"MaxDelegationDepth",
                        &attrib.value,
                    )?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                policy_set_id: policy_set_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("PolicySetId".into()))
                })?,
                version: version.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("Version".into()))
                })?,
                policy_combining_alg_id: policy_combining_alg_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("PolicyCombiningAlgId".into()))
                })?,
                max_delegation_depth: max_delegation_depth,
                description: None,
                policy_issuer: None,
                policy_set_defaults: None,
                target: None,
                content_31: Vec::new(),
                obligation_expressions: None,
                advice_expressions: None,
                state__: Box::new(PolicySetTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: PolicySetTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use PolicySetTypeDeserializerState as S;
            match state {
                S::Description(Some(deserializer)) => {
                    self.store_description(deserializer.finish(reader)?)?
                }
                S::PolicyIssuer(Some(deserializer)) => {
                    self.store_policy_issuer(deserializer.finish(reader)?)?
                }
                S::PolicySetDefaults(Some(deserializer)) => {
                    self.store_policy_set_defaults(deserializer.finish(reader)?)?
                }
                S::Target(Some(deserializer)) => self.store_target(deserializer.finish(reader)?)?,
                S::Content31(Some(deserializer)) => {
                    self.store_content_31(deserializer.finish(reader)?)?
                }
                S::ObligationExpressions(Some(deserializer)) => {
                    self.store_obligation_expressions(deserializer.finish(reader)?)?
                }
                S::AdviceExpressions(Some(deserializer)) => {
                    self.store_advice_expressions(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_description(&mut self, value: super::Description) -> std::result::Result<(), Error> {
            if self.description.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Description",
                )))?;
            }
            self.description = Some(value);
            Ok(())
        }
        fn store_policy_issuer(&mut self, value: super::PolicyIssuer) -> std::result::Result<(), Error> {
            if self.policy_issuer.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"PolicyIssuer",
                )))?;
            }
            self.policy_issuer = Some(value);
            Ok(())
        }
        fn store_policy_set_defaults(
            &mut self,
            value: super::PolicySetDefaults,
        ) -> std::result::Result<(), Error> {
            if self.policy_set_defaults.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"PolicySetDefaults",
                )))?;
            }
            self.policy_set_defaults = Some(value);
            Ok(())
        }
        fn store_target(&mut self, value: super::Target) -> std::result::Result<(), Error> {
            if self.target.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Target",
                )))?;
            }
            self.target = Some(value);
            Ok(())
        }
        fn store_content_31(&mut self, value: super::PolicySetContent31Type) -> std::result::Result<(), Error> {
            self.content_31.push(value);
            Ok(())
        }
        fn store_obligation_expressions(
            &mut self,
            value: super::ObligationExpressions,
        ) -> std::result::Result<(), Error> {
            if self.obligation_expressions.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"ObligationExpressions",
                )))?;
            }
            self.obligation_expressions = Some(value);
            Ok(())
        }
        fn store_advice_expressions(
            &mut self,
            value: super::AdviceExpressions,
        ) -> std::result::Result<(), Error> {
            if self.advice_expressions.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"AdviceExpressions",
                )))?;
            }
            self.advice_expressions = Some(value);
            Ok(())
        }
        fn handle_description<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Description>,
            fallback: &mut Option<PolicySetTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(PolicySetTypeDeserializerState::Description(None));
                *self.state__ = PolicySetTypeDeserializerState::PolicyIssuer(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_description(data)?;
                    *self.state__ = PolicySetTypeDeserializerState::PolicyIssuer(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(PolicySetTypeDeserializerState::Description(
                                Some(deserializer),
                            ));
                            *self.state__ = PolicySetTypeDeserializerState::PolicyIssuer(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                PolicySetTypeDeserializerState::Description(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_policy_issuer<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::PolicyIssuer>,
            fallback: &mut Option<PolicySetTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(PolicySetTypeDeserializerState::PolicyIssuer(None));
                *self.state__ = PolicySetTypeDeserializerState::PolicySetDefaults(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_policy_issuer(data)?;
                    *self.state__ = PolicySetTypeDeserializerState::PolicySetDefaults(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(PolicySetTypeDeserializerState::PolicyIssuer(
                                Some(deserializer),
                            ));
                            *self.state__ = PolicySetTypeDeserializerState::PolicySetDefaults(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                PolicySetTypeDeserializerState::PolicyIssuer(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_policy_set_defaults<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::PolicySetDefaults>,
            fallback: &mut Option<PolicySetTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(PolicySetTypeDeserializerState::PolicySetDefaults(None));
                *self.state__ = PolicySetTypeDeserializerState::Target(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_policy_set_defaults(data)?;
                    *self.state__ = PolicySetTypeDeserializerState::Target(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                PolicySetTypeDeserializerState::PolicySetDefaults(Some(
                                    deserializer,
                                )),
                            );
                            *self.state__ = PolicySetTypeDeserializerState::Target(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = PolicySetTypeDeserializerState::PolicySetDefaults(
                                Some(deserializer),
                            );
                        }
                    }
                    ret
                }
            })
        }
        fn handle_target<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Target>,
            fallback: &mut Option<PolicySetTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.target.is_some() {
                    fallback.get_or_insert(PolicySetTypeDeserializerState::Target(None));
                    *self.state__ = PolicySetTypeDeserializerState::Content31(None);
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                } else {
                    *self.state__ = PolicySetTypeDeserializerState::Target(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_target(data)?;
                    *self.state__ = PolicySetTypeDeserializerState::Content31(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(PolicySetTypeDeserializerState::Target(Some(
                                deserializer,
                            )));
                            *self.state__ = PolicySetTypeDeserializerState::Content31(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                PolicySetTypeDeserializerState::Target(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_content_31<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::PolicySetContent31Type>,
            fallback: &mut Option<PolicySetTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(PolicySetTypeDeserializerState::Content31(None));
                *self.state__ = PolicySetTypeDeserializerState::ObligationExpressions(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_content_31(data)?;
                    *self.state__ = PolicySetTypeDeserializerState::Content31(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(PolicySetTypeDeserializerState::Content31(
                                Some(deserializer),
                            ));
                            *self.state__ = PolicySetTypeDeserializerState::Content31(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                PolicySetTypeDeserializerState::Content31(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_obligation_expressions<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::ObligationExpressions>,
            fallback: &mut Option<PolicySetTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(PolicySetTypeDeserializerState::ObligationExpressions(None));
                *self.state__ = PolicySetTypeDeserializerState::AdviceExpressions(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_obligation_expressions(data)?;
                    *self.state__ = PolicySetTypeDeserializerState::AdviceExpressions(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                PolicySetTypeDeserializerState::ObligationExpressions(Some(
                                    deserializer,
                                )),
                            );
                            *self.state__ = PolicySetTypeDeserializerState::AdviceExpressions(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = PolicySetTypeDeserializerState::ObligationExpressions(
                                Some(deserializer),
                            );
                        }
                    }
                    ret
                }
            })
        }
        fn handle_advice_expressions<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AdviceExpressions>,
            fallback: &mut Option<PolicySetTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(PolicySetTypeDeserializerState::AdviceExpressions(None));
                *self.state__ = PolicySetTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_advice_expressions(data)?;
                    *self.state__ = PolicySetTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                PolicySetTypeDeserializerState::AdviceExpressions(Some(
                                    deserializer,
                                )),
                            );
                            *self.state__ = PolicySetTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = PolicySetTypeDeserializerState::AdviceExpressions(
                                Some(deserializer),
                            );
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::PolicySetType> for PolicySetTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::PolicySetType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::PolicySetType>
        where
            R: DeserializeReader,
        {
            use PolicySetTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Description(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_description(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::PolicyIssuer(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_policy_issuer(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::PolicySetDefaults(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_policy_set_defaults(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Target(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_target(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Content31(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_content_31(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::ObligationExpressions(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_obligation_expressions(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::AdviceExpressions(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_advice_expressions(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = PolicySetTypeDeserializerState::Description(None);
                        event
                    }
                    (S::Description(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Description",
                            false,
                        )?;
                        match self.handle_description(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::PolicyIssuer(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"PolicyIssuer",
                            false,
                        )?;
                        match self.handle_policy_issuer(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::PolicySetDefaults(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"PolicySetDefaults",
                            false,
                        )?;
                        match self.handle_policy_set_defaults(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Target(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Target",
                            false,
                        )?;
                        match self.handle_target(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Content31(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = < super :: PolicySetContent31Type as WithDeserializer > :: Deserializer :: init (reader , event) ? ;
                        match self.handle_content_31(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (
                        S::ObligationExpressions(None),
                        event @ (Event::Start(_) | Event::Empty(_)),
                    ) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"ObligationExpressions",
                            false,
                        )?;
                        match self.handle_obligation_expressions(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::AdviceExpressions(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AdviceExpressions",
                            false,
                        )?;
                        match self.handle_advice_expressions(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::PolicySetType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                PolicySetTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::PolicySetType {
                policy_set_id: self.policy_set_id,
                version: self.version,
                policy_combining_alg_id: self.policy_combining_alg_id,
                max_delegation_depth: self.max_delegation_depth,
                description: self.description,
                policy_issuer: self.policy_issuer,
                policy_set_defaults: self.policy_set_defaults,
                target: self
                    .target
                    .ok_or_else(|| ErrorKind::MissingElement("Target".into()))?,
                content_31: self.content_31,
                obligation_expressions: self.obligation_expressions,
                advice_expressions: self.advice_expressions,
            })
        }
    }
    #[derive(Debug)]
    pub struct PolicyTypeDeserializer {
        policy_id: super::AnyUriType,
        version: super::VersionType,
        rule_combining_alg_id: super::AnyUriType,
        max_delegation_depth: Option<super::IntegerType>,
        description: Option<super::Description>,
        policy_issuer: Option<super::PolicyIssuer>,
        policy_defaults: Option<super::PolicyDefaults>,
        target: Option<super::Target>,
        content_41: Vec<super::PolicyContent41Type>,
        obligation_expressions: Option<super::ObligationExpressions>,
        advice_expressions: Option<super::AdviceExpressions>,
        state__: Box<PolicyTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum PolicyTypeDeserializerState {
        Init__,
        Description(Option<<super::Description as WithDeserializer>::Deserializer>),
        PolicyIssuer(Option<<super::PolicyIssuer as WithDeserializer>::Deserializer>),
        PolicyDefaults(Option<<super::PolicyDefaults as WithDeserializer>::Deserializer>),
        Target(Option<<super::Target as WithDeserializer>::Deserializer>),
        Content41(Option<<super::PolicyContent41Type as WithDeserializer>::Deserializer>),
        ObligationExpressions(
            Option<<super::ObligationExpressions as WithDeserializer>::Deserializer>,
        ),
        AdviceExpressions(Option<<super::AdviceExpressions as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl PolicyTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut policy_id: Option<super::AnyUriType> = None;
            let mut version: Option<super::VersionType> = None;
            let mut rule_combining_alg_id: Option<super::AnyUriType> = None;
            let mut max_delegation_depth: Option<super::IntegerType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"PolicyId")
                ) {
                    reader.read_attrib(&mut policy_id, b"PolicyId", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Version")
                ) {
                    reader.read_attrib(&mut version, b"Version", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"RuleCombiningAlgId")
                ) {
                    reader.read_attrib(
                        &mut rule_combining_alg_id,
                        b"RuleCombiningAlgId",
                        &attrib.value,
                    )?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"MaxDelegationDepth")
                ) {
                    reader.read_attrib(
                        &mut max_delegation_depth,
                        b"MaxDelegationDepth",
                        &attrib.value,
                    )?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                version: version.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("Version".into()))
                })?,
                policy_id: policy_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("PolicyId".into()))
                })?,
                rule_combining_alg_id: rule_combining_alg_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("RuleCombiningAlgId".into()))
                })?,
                max_delegation_depth: max_delegation_depth,
                description: None,
                policy_issuer: None,
                policy_defaults: None,
                target: None,
                content_41: Vec::new(),
                obligation_expressions: None,
                advice_expressions: None,
                state__: Box::new(PolicyTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: PolicyTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use PolicyTypeDeserializerState as S;
            match state {
                S::Description(Some(deserializer)) => {
                    self.store_description(deserializer.finish(reader)?)?
                }
                S::PolicyIssuer(Some(deserializer)) => {
                    self.store_policy_issuer(deserializer.finish(reader)?)?
                }
                S::PolicyDefaults(Some(deserializer)) => {
                    self.store_policy_defaults(deserializer.finish(reader)?)?
                }
                S::Target(Some(deserializer)) => self.store_target(deserializer.finish(reader)?)?,
                S::Content41(Some(deserializer)) => {
                    self.store_content_41(deserializer.finish(reader)?)?
                }
                S::ObligationExpressions(Some(deserializer)) => {
                    self.store_obligation_expressions(deserializer.finish(reader)?)?
                }
                S::AdviceExpressions(Some(deserializer)) => {
                    self.store_advice_expressions(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_description(&mut self, value: super::Description) -> std::result::Result<(), Error> {
            if self.description.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Description",
                )))?;
            }
            self.description = Some(value);
            Ok(())
        }
        fn store_policy_issuer(&mut self, value: super::PolicyIssuer) -> std::result::Result<(), Error> {
            if self.policy_issuer.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"PolicyIssuer",
                )))?;
            }
            self.policy_issuer = Some(value);
            Ok(())
        }
        fn store_policy_defaults(&mut self, value: super::PolicyDefaults) -> std::result::Result<(), Error> {
            if self.policy_defaults.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"PolicyDefaults",
                )))?;
            }
            self.policy_defaults = Some(value);
            Ok(())
        }
        fn store_target(&mut self, value: super::Target) -> std::result::Result<(), Error> {
            if self.target.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Target",
                )))?;
            }
            self.target = Some(value);
            Ok(())
        }
        fn store_content_41(&mut self, value: super::PolicyContent41Type) -> std::result::Result<(), Error> {
            self.content_41.push(value);
            Ok(())
        }
        fn store_obligation_expressions(
            &mut self,
            value: super::ObligationExpressions,
        ) -> std::result::Result<(), Error> {
            if self.obligation_expressions.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"ObligationExpressions",
                )))?;
            }
            self.obligation_expressions = Some(value);
            Ok(())
        }
        fn store_advice_expressions(
            &mut self,
            value: super::AdviceExpressions,
        ) -> std::result::Result<(), Error> {
            if self.advice_expressions.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"AdviceExpressions",
                )))?;
            }
            self.advice_expressions = Some(value);
            Ok(())
        }
        fn handle_description<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Description>,
            fallback: &mut Option<PolicyTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(PolicyTypeDeserializerState::Description(None));
                *self.state__ = PolicyTypeDeserializerState::PolicyIssuer(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_description(data)?;
                    *self.state__ = PolicyTypeDeserializerState::PolicyIssuer(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(PolicyTypeDeserializerState::Description(Some(
                                deserializer,
                            )));
                            *self.state__ = PolicyTypeDeserializerState::PolicyIssuer(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                PolicyTypeDeserializerState::Description(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_policy_issuer<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::PolicyIssuer>,
            fallback: &mut Option<PolicyTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(PolicyTypeDeserializerState::PolicyIssuer(None));
                *self.state__ = PolicyTypeDeserializerState::PolicyDefaults(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_policy_issuer(data)?;
                    *self.state__ = PolicyTypeDeserializerState::PolicyDefaults(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(PolicyTypeDeserializerState::PolicyIssuer(
                                Some(deserializer),
                            ));
                            *self.state__ = PolicyTypeDeserializerState::PolicyDefaults(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                PolicyTypeDeserializerState::PolicyIssuer(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_policy_defaults<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::PolicyDefaults>,
            fallback: &mut Option<PolicyTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(PolicyTypeDeserializerState::PolicyDefaults(None));
                *self.state__ = PolicyTypeDeserializerState::Target(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_policy_defaults(data)?;
                    *self.state__ = PolicyTypeDeserializerState::Target(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(PolicyTypeDeserializerState::PolicyDefaults(
                                Some(deserializer),
                            ));
                            *self.state__ = PolicyTypeDeserializerState::Target(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                PolicyTypeDeserializerState::PolicyDefaults(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_target<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Target>,
            fallback: &mut Option<PolicyTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.target.is_some() {
                    fallback.get_or_insert(PolicyTypeDeserializerState::Target(None));
                    *self.state__ = PolicyTypeDeserializerState::Content41(None);
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                } else {
                    *self.state__ = PolicyTypeDeserializerState::Target(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_target(data)?;
                    *self.state__ = PolicyTypeDeserializerState::Content41(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(PolicyTypeDeserializerState::Target(Some(
                                deserializer,
                            )));
                            *self.state__ = PolicyTypeDeserializerState::Content41(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = PolicyTypeDeserializerState::Target(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_content_41<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::PolicyContent41Type>,
            fallback: &mut Option<PolicyTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.content_41.len() < 1usize {
                    *self.state__ = PolicyTypeDeserializerState::Content41(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                } else {
                    fallback.get_or_insert(PolicyTypeDeserializerState::Content41(None));
                    *self.state__ = PolicyTypeDeserializerState::ObligationExpressions(None);
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_content_41(data)?;
                    *self.state__ = PolicyTypeDeserializerState::Content41(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(PolicyTypeDeserializerState::Content41(Some(
                                deserializer,
                            )));
                            if self.content_41.len().saturating_add(1) < 1usize {
                                *self.state__ = PolicyTypeDeserializerState::Content41(None);
                            } else {
                                *self.state__ =
                                    PolicyTypeDeserializerState::ObligationExpressions(None);
                            }
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                PolicyTypeDeserializerState::Content41(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_obligation_expressions<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::ObligationExpressions>,
            fallback: &mut Option<PolicyTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(PolicyTypeDeserializerState::ObligationExpressions(None));
                *self.state__ = PolicyTypeDeserializerState::AdviceExpressions(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_obligation_expressions(data)?;
                    *self.state__ = PolicyTypeDeserializerState::AdviceExpressions(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                PolicyTypeDeserializerState::ObligationExpressions(Some(
                                    deserializer,
                                )),
                            );
                            *self.state__ = PolicyTypeDeserializerState::AdviceExpressions(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = PolicyTypeDeserializerState::ObligationExpressions(
                                Some(deserializer),
                            );
                        }
                    }
                    ret
                }
            })
        }
        fn handle_advice_expressions<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AdviceExpressions>,
            fallback: &mut Option<PolicyTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(PolicyTypeDeserializerState::AdviceExpressions(None));
                *self.state__ = PolicyTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_advice_expressions(data)?;
                    *self.state__ = PolicyTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(PolicyTypeDeserializerState::AdviceExpressions(
                                Some(deserializer),
                            ));
                            *self.state__ = PolicyTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                PolicyTypeDeserializerState::AdviceExpressions(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::PolicyType> for PolicyTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::PolicyType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::PolicyType>
        where
            R: DeserializeReader,
        {
            use PolicyTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Description(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_description(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::PolicyIssuer(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_policy_issuer(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::PolicyDefaults(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_policy_defaults(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Target(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_target(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Content41(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_content_41(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::ObligationExpressions(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_obligation_expressions(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::AdviceExpressions(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_advice_expressions(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = PolicyTypeDeserializerState::Description(None);
                        event
                    }
                    (S::Description(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Description",
                            false,
                        )?;
                        match self.handle_description(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::PolicyIssuer(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"PolicyIssuer",
                            false,
                        )?;
                        match self.handle_policy_issuer(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::PolicyDefaults(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"PolicyDefaults",
                            false,
                        )?;
                        match self.handle_policy_defaults(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Target(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Target",
                            false,
                        )?;
                        match self.handle_target(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Content41(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output =
                            <super::PolicyContent41Type as WithDeserializer>::Deserializer::init(
                                reader, event,
                            )?;
                        match self.handle_content_41(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (
                        S::ObligationExpressions(None),
                        event @ (Event::Start(_) | Event::Empty(_)),
                    ) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"ObligationExpressions",
                            false,
                        )?;
                        match self.handle_obligation_expressions(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::AdviceExpressions(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AdviceExpressions",
                            false,
                        )?;
                        match self.handle_advice_expressions(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::PolicyType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(&mut *self.state__, PolicyTypeDeserializerState::Unknown__);
            self.finish_state(reader, state)?;
            Ok(super::PolicyType {
                policy_id: self.policy_id,
                version: self.version,
                rule_combining_alg_id: self.rule_combining_alg_id,
                max_delegation_depth: self.max_delegation_depth,
                description: self.description,
                policy_issuer: self.policy_issuer,
                policy_defaults: self.policy_defaults,
                target: self
                    .target
                    .ok_or_else(|| ErrorKind::MissingElement("Target".into()))?,
                content_41: self.content_41,
                obligation_expressions: self.obligation_expressions,
                advice_expressions: self.advice_expressions,
            })
        }
    }
    #[derive(Debug)]
    pub struct RequestDefaultsTypeDeserializer {
        content_3: Option<super::RequestDefaultsContent3Type>,
        state__: Box<RequestDefaultsTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum RequestDefaultsTypeDeserializerState {
        Init__,
        Content3(Option<<super::RequestDefaultsContent3Type as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl RequestDefaultsTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                content_3: None,
                state__: Box::new(RequestDefaultsTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: RequestDefaultsTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use RequestDefaultsTypeDeserializerState as S;
            match state {
                S::Content3(Some(deserializer)) => {
                    self.store_content_3(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_content_3(
            &mut self,
            value: super::RequestDefaultsContent3Type,
        ) -> std::result::Result<(), Error> {
            if self.content_3.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Content3",
                )))?;
            }
            self.content_3 = Some(value);
            Ok(())
        }
        fn handle_content_3<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::RequestDefaultsContent3Type>,
            fallback: &mut Option<RequestDefaultsTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.content_3.is_some() {
                    fallback.get_or_insert(RequestDefaultsTypeDeserializerState::Content3(None));
                    *self.state__ = RequestDefaultsTypeDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                } else {
                    *self.state__ = RequestDefaultsTypeDeserializerState::Content3(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_content_3(data)?;
                    *self.state__ = RequestDefaultsTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(RequestDefaultsTypeDeserializerState::Content3(
                                Some(deserializer),
                            ));
                            *self.state__ = RequestDefaultsTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                RequestDefaultsTypeDeserializerState::Content3(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::RequestDefaultsType> for RequestDefaultsTypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::RequestDefaultsType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::RequestDefaultsType>
        where
            R: DeserializeReader,
        {
            use RequestDefaultsTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Content3(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_content_3(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = RequestDefaultsTypeDeserializerState::Content3(None);
                        event
                    }
                    (S::Content3(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = < super :: RequestDefaultsContent3Type as WithDeserializer > :: Deserializer :: init (reader , event) ? ;
                        match self.handle_content_3(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::RequestDefaultsType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                RequestDefaultsTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::RequestDefaultsType {
                content_3: self
                    .content_3
                    .ok_or_else(|| ErrorKind::MissingElement("Content3".into()))?,
            })
        }
    }
    #[derive(Debug)]
    pub struct RequestReferenceTypeDeserializer {
        attributes_reference: Vec<super::AttributesReference>,
        state__: Box<RequestReferenceTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum RequestReferenceTypeDeserializerState {
        Init__,
        AttributesReference(Option<<super::AttributesReference as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl RequestReferenceTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                attributes_reference: Vec::new(),
                state__: Box::new(RequestReferenceTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: RequestReferenceTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use RequestReferenceTypeDeserializerState as S;
            match state {
                S::AttributesReference(Some(deserializer)) => {
                    self.store_attributes_reference(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_attributes_reference(
            &mut self,
            value: super::AttributesReference,
        ) -> std::result::Result<(), Error> {
            self.attributes_reference.push(value);
            Ok(())
        }
        fn handle_attributes_reference<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AttributesReference>,
            fallback: &mut Option<RequestReferenceTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.attributes_reference.len() < 1usize {
                    *self.state__ =
                        RequestReferenceTypeDeserializerState::AttributesReference(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                } else {
                    fallback.get_or_insert(
                        RequestReferenceTypeDeserializerState::AttributesReference(None),
                    );
                    *self.state__ = RequestReferenceTypeDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_attributes_reference(data)?;
                    *self.state__ =
                        RequestReferenceTypeDeserializerState::AttributesReference(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                RequestReferenceTypeDeserializerState::AttributesReference(Some(
                                    deserializer,
                                )),
                            );
                            if self.attributes_reference.len().saturating_add(1) < 1usize {
                                *self.state__ =
                                    RequestReferenceTypeDeserializerState::AttributesReference(
                                        None,
                                    );
                            } else {
                                *self.state__ = RequestReferenceTypeDeserializerState::Done__;
                            }
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                RequestReferenceTypeDeserializerState::AttributesReference(Some(
                                    deserializer,
                                ));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::RequestReferenceType> for RequestReferenceTypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::RequestReferenceType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::RequestReferenceType>
        where
            R: DeserializeReader,
        {
            use RequestReferenceTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::AttributesReference(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_attributes_reference(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ =
                            RequestReferenceTypeDeserializerState::AttributesReference(None);
                        event
                    }
                    (S::AttributesReference(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AttributesReference",
                            false,
                        )?;
                        match self.handle_attributes_reference(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::RequestReferenceType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                RequestReferenceTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::RequestReferenceType {
                attributes_reference: self.attributes_reference,
            })
        }
    }
    #[derive(Debug)]
    pub struct RequestTypeDeserializer {
        return_policy_id_list: super::BooleanType,
        combined_decision: super::BooleanType,
        request_defaults: Option<super::RequestDefaults>,
        attributes: Vec<super::Attributes>,
        multi_requests: Option<super::MultiRequests>,
        state__: Box<RequestTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum RequestTypeDeserializerState {
        Init__,
        RequestDefaults(Option<<super::RequestDefaults as WithDeserializer>::Deserializer>),
        Attributes(Option<<super::Attributes as WithDeserializer>::Deserializer>),
        MultiRequests(Option<<super::MultiRequests as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl RequestTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut return_policy_id_list: Option<super::BooleanType> = None;
            let mut combined_decision: Option<super::BooleanType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"ReturnPolicyIdList")
                ) {
                    reader.read_attrib(
                        &mut return_policy_id_list,
                        b"ReturnPolicyIdList",
                        &attrib.value,
                    )?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"CombinedDecision")
                ) {
                    reader.read_attrib(
                        &mut combined_decision,
                        b"CombinedDecision",
                        &attrib.value,
                    )?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                return_policy_id_list: return_policy_id_list.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("ReturnPolicyIdList".into()))
                })?,
                combined_decision: combined_decision.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("CombinedDecision".into()))
                })?,
                request_defaults: None,
                attributes: Vec::new(),
                multi_requests: None,
                state__: Box::new(RequestTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: RequestTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use RequestTypeDeserializerState as S;
            match state {
                S::RequestDefaults(Some(deserializer)) => {
                    self.store_request_defaults(deserializer.finish(reader)?)?
                }
                S::Attributes(Some(deserializer)) => {
                    self.store_attributes(deserializer.finish(reader)?)?
                }
                S::MultiRequests(Some(deserializer)) => {
                    self.store_multi_requests(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_request_defaults(&mut self, value: super::RequestDefaults) -> std::result::Result<(), Error> {
            if self.request_defaults.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"RequestDefaults",
                )))?;
            }
            self.request_defaults = Some(value);
            Ok(())
        }
        fn store_attributes(&mut self, value: super::Attributes) -> std::result::Result<(), Error> {
            self.attributes.push(value);
            Ok(())
        }
        fn store_multi_requests(&mut self, value: super::MultiRequests) -> std::result::Result<(), Error> {
            if self.multi_requests.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"MultiRequests",
                )))?;
            }
            self.multi_requests = Some(value);
            Ok(())
        }
        fn handle_request_defaults<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::RequestDefaults>,
            fallback: &mut Option<RequestTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(RequestTypeDeserializerState::RequestDefaults(None));
                *self.state__ = RequestTypeDeserializerState::Attributes(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_request_defaults(data)?;
                    *self.state__ = RequestTypeDeserializerState::Attributes(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(RequestTypeDeserializerState::RequestDefaults(
                                Some(deserializer),
                            ));
                            *self.state__ = RequestTypeDeserializerState::Attributes(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                RequestTypeDeserializerState::RequestDefaults(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_attributes<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Attributes>,
            fallback: &mut Option<RequestTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.attributes.len() < 1usize {
                    *self.state__ = RequestTypeDeserializerState::Attributes(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                } else {
                    fallback.get_or_insert(RequestTypeDeserializerState::Attributes(None));
                    *self.state__ = RequestTypeDeserializerState::MultiRequests(None);
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_attributes(data)?;
                    *self.state__ = RequestTypeDeserializerState::Attributes(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(RequestTypeDeserializerState::Attributes(Some(
                                deserializer,
                            )));
                            if self.attributes.len().saturating_add(1) < 1usize {
                                *self.state__ = RequestTypeDeserializerState::Attributes(None);
                            } else {
                                *self.state__ = RequestTypeDeserializerState::MultiRequests(None);
                            }
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                RequestTypeDeserializerState::Attributes(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_multi_requests<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::MultiRequests>,
            fallback: &mut Option<RequestTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(RequestTypeDeserializerState::MultiRequests(None));
                *self.state__ = RequestTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_multi_requests(data)?;
                    *self.state__ = RequestTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(RequestTypeDeserializerState::MultiRequests(
                                Some(deserializer),
                            ));
                            *self.state__ = RequestTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                RequestTypeDeserializerState::MultiRequests(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::RequestType> for RequestTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::RequestType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::RequestType>
        where
            R: DeserializeReader,
        {
            use RequestTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::RequestDefaults(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_request_defaults(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Attributes(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_attributes(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::MultiRequests(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_multi_requests(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = RequestTypeDeserializerState::RequestDefaults(None);
                        event
                    }
                    (S::RequestDefaults(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"RequestDefaults",
                            false,
                        )?;
                        match self.handle_request_defaults(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Attributes(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Attributes",
                            false,
                        )?;
                        match self.handle_attributes(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::MultiRequests(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"MultiRequests",
                            false,
                        )?;
                        match self.handle_multi_requests(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::RequestType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(&mut *self.state__, RequestTypeDeserializerState::Unknown__);
            self.finish_state(reader, state)?;
            Ok(super::RequestType {
                return_policy_id_list: self.return_policy_id_list,
                combined_decision: self.combined_decision,
                request_defaults: self.request_defaults,
                attributes: self.attributes,
                multi_requests: self.multi_requests,
            })
        }
    }
    #[derive(Debug)]
    pub struct ResponseTypeDeserializer {
        result: Vec<super::ResultT>,
        state__: Box<ResponseTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum ResponseTypeDeserializerState {
        Init__,
        Result(Option<<super::ResultT as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl ResponseTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                result: Vec::new(),
                state__: Box::new(ResponseTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: ResponseTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use ResponseTypeDeserializerState as S;
            match state {
                S::Result(Some(deserializer)) => self.store_result(deserializer.finish(reader)?)?,
                _ => (),
            }
            Ok(())
        }
        fn store_result(&mut self, value: super::ResultT) -> std::result::Result<(), Error> {
            self.result.push(value);
            Ok(())
        }
        fn handle_result<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::ResultT>,
            fallback: &mut Option<ResponseTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.result.len() < 1usize {
                    *self.state__ = ResponseTypeDeserializerState::Result(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                } else {
                    fallback.get_or_insert(ResponseTypeDeserializerState::Result(None));
                    *self.state__ = ResponseTypeDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_result(data)?;
                    *self.state__ = ResponseTypeDeserializerState::Result(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(ResponseTypeDeserializerState::Result(Some(
                                deserializer,
                            )));
                            if self.result.len().saturating_add(1) < 1usize {
                                *self.state__ = ResponseTypeDeserializerState::Result(None);
                            } else {
                                *self.state__ = ResponseTypeDeserializerState::Done__;
                            }
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                ResponseTypeDeserializerState::Result(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::ResponseType> for ResponseTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::ResponseType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::ResponseType>
        where
            R: DeserializeReader,
        {
            use ResponseTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Result(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_result(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = ResponseTypeDeserializerState::Result(None);
                        event
                    }
                    (S::Result(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Result",
                            false,
                        )?;
                        match self.handle_result(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::ResponseType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(&mut *self.state__, ResponseTypeDeserializerState::Unknown__);
            self.finish_state(reader, state)?;
            Ok(super::ResponseType {
                result: self.result,
            })
        }
    }
    #[derive(Debug)]
    pub struct ResultTypeDeserializer {
        decision: Option<super::Decision>,
        status: Option<super::Status>,
        obligations: Option<super::Obligations>,
        associated_advice: Option<super::AssociatedAdvice>,
        attributes: Vec<super::Attributes>,
        policy_identifier_list: Option<super::PolicyIdentifierList>,
        state__: Box<ResultTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum ResultTypeDeserializerState {
        Init__,
        Decision(Option<<super::Decision as WithDeserializer>::Deserializer>),
        Status(Option<<super::Status as WithDeserializer>::Deserializer>),
        Obligations(Option<<super::Obligations as WithDeserializer>::Deserializer>),
        AssociatedAdvice(Option<<super::AssociatedAdvice as WithDeserializer>::Deserializer>),
        Attributes(Option<<super::Attributes as WithDeserializer>::Deserializer>),
        PolicyIdentifierList(
            Option<<super::PolicyIdentifierList as WithDeserializer>::Deserializer>,
        ),
        Done__,
        Unknown__,
    }
    impl ResultTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                decision: None,
                status: None,
                obligations: None,
                associated_advice: None,
                attributes: Vec::new(),
                policy_identifier_list: None,
                state__: Box::new(ResultTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: ResultTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use ResultTypeDeserializerState as S;
            match state {
                S::Decision(Some(deserializer)) => {
                    self.store_decision(deserializer.finish(reader)?)?
                }
                S::Status(Some(deserializer)) => self.store_status(deserializer.finish(reader)?)?,
                S::Obligations(Some(deserializer)) => {
                    self.store_obligations(deserializer.finish(reader)?)?
                }
                S::AssociatedAdvice(Some(deserializer)) => {
                    self.store_associated_advice(deserializer.finish(reader)?)?
                }
                S::Attributes(Some(deserializer)) => {
                    self.store_attributes(deserializer.finish(reader)?)?
                }
                S::PolicyIdentifierList(Some(deserializer)) => {
                    self.store_policy_identifier_list(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_decision(&mut self, value: super::Decision) -> std::result::Result<(), Error> {
            if self.decision.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Decision",
                )))?;
            }
            self.decision = Some(value);
            Ok(())
        }
        fn store_status(&mut self, value: super::Status) -> std::result::Result<(), Error> {
            if self.status.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Status",
                )))?;
            }
            self.status = Some(value);
            Ok(())
        }
        fn store_obligations(&mut self, value: super::Obligations) -> std::result::Result<(), Error> {
            if self.obligations.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Obligations",
                )))?;
            }
            self.obligations = Some(value);
            Ok(())
        }
        fn store_associated_advice(&mut self, value: super::AssociatedAdvice) -> std::result::Result<(), Error> {
            if self.associated_advice.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"AssociatedAdvice",
                )))?;
            }
            self.associated_advice = Some(value);
            Ok(())
        }
        fn store_attributes(&mut self, value: super::Attributes) -> std::result::Result<(), Error> {
            self.attributes.push(value);
            Ok(())
        }
        fn store_policy_identifier_list(
            &mut self,
            value: super::PolicyIdentifierList,
        ) -> std::result::Result<(), Error> {
            if self.policy_identifier_list.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"PolicyIdentifierList",
                )))?;
            }
            self.policy_identifier_list = Some(value);
            Ok(())
        }
        fn handle_decision<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Decision>,
            fallback: &mut Option<ResultTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.decision.is_some() {
                    fallback.get_or_insert(ResultTypeDeserializerState::Decision(None));
                    *self.state__ = ResultTypeDeserializerState::Status(None);
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                } else {
                    *self.state__ = ResultTypeDeserializerState::Decision(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_decision(data)?;
                    *self.state__ = ResultTypeDeserializerState::Status(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(ResultTypeDeserializerState::Decision(Some(
                                deserializer,
                            )));
                            *self.state__ = ResultTypeDeserializerState::Status(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                ResultTypeDeserializerState::Decision(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_status<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Status>,
            fallback: &mut Option<ResultTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(ResultTypeDeserializerState::Status(None));
                *self.state__ = ResultTypeDeserializerState::Obligations(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_status(data)?;
                    *self.state__ = ResultTypeDeserializerState::Obligations(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(ResultTypeDeserializerState::Status(Some(
                                deserializer,
                            )));
                            *self.state__ = ResultTypeDeserializerState::Obligations(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = ResultTypeDeserializerState::Status(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_obligations<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Obligations>,
            fallback: &mut Option<ResultTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(ResultTypeDeserializerState::Obligations(None));
                *self.state__ = ResultTypeDeserializerState::AssociatedAdvice(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_obligations(data)?;
                    *self.state__ = ResultTypeDeserializerState::AssociatedAdvice(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(ResultTypeDeserializerState::Obligations(Some(
                                deserializer,
                            )));
                            *self.state__ = ResultTypeDeserializerState::AssociatedAdvice(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                ResultTypeDeserializerState::Obligations(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_associated_advice<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AssociatedAdvice>,
            fallback: &mut Option<ResultTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(ResultTypeDeserializerState::AssociatedAdvice(None));
                *self.state__ = ResultTypeDeserializerState::Attributes(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_associated_advice(data)?;
                    *self.state__ = ResultTypeDeserializerState::Attributes(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(ResultTypeDeserializerState::AssociatedAdvice(
                                Some(deserializer),
                            ));
                            *self.state__ = ResultTypeDeserializerState::Attributes(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                ResultTypeDeserializerState::AssociatedAdvice(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_attributes<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Attributes>,
            fallback: &mut Option<ResultTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(ResultTypeDeserializerState::Attributes(None));
                *self.state__ = ResultTypeDeserializerState::PolicyIdentifierList(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_attributes(data)?;
                    *self.state__ = ResultTypeDeserializerState::Attributes(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(ResultTypeDeserializerState::Attributes(Some(
                                deserializer,
                            )));
                            *self.state__ = ResultTypeDeserializerState::Attributes(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                ResultTypeDeserializerState::Attributes(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_policy_identifier_list<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::PolicyIdentifierList>,
            fallback: &mut Option<ResultTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(ResultTypeDeserializerState::PolicyIdentifierList(None));
                *self.state__ = ResultTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_policy_identifier_list(data)?;
                    *self.state__ = ResultTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                ResultTypeDeserializerState::PolicyIdentifierList(Some(
                                    deserializer,
                                )),
                            );
                            *self.state__ = ResultTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = ResultTypeDeserializerState::PolicyIdentifierList(
                                Some(deserializer),
                            );
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::ResultType> for ResultTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::ResultType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::ResultType>
        where
            R: DeserializeReader,
        {
            use ResultTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Decision(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_decision(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Status(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_status(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Obligations(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_obligations(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::AssociatedAdvice(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_associated_advice(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Attributes(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_attributes(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::PolicyIdentifierList(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_policy_identifier_list(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = ResultTypeDeserializerState::Decision(None);
                        event
                    }
                    (S::Decision(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Decision",
                            false,
                        )?;
                        match self.handle_decision(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Status(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Status",
                            false,
                        )?;
                        match self.handle_status(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Obligations(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Obligations",
                            false,
                        )?;
                        match self.handle_obligations(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::AssociatedAdvice(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AssociatedAdvice",
                            false,
                        )?;
                        match self.handle_associated_advice(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Attributes(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Attributes",
                            false,
                        )?;
                        match self.handle_attributes(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (
                        S::PolicyIdentifierList(None),
                        event @ (Event::Start(_) | Event::Empty(_)),
                    ) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"PolicyIdentifierList",
                            false,
                        )?;
                        match self.handle_policy_identifier_list(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::ResultType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(&mut *self.state__, ResultTypeDeserializerState::Unknown__);
            self.finish_state(reader, state)?;
            Ok(super::ResultType {
                decision: self
                    .decision
                    .ok_or_else(|| ErrorKind::MissingElement("Decision".into()))?,
                status: self.status,
                obligations: self.obligations,
                associated_advice: self.associated_advice,
                attributes: self.attributes,
                policy_identifier_list: self.policy_identifier_list,
            })
        }
    }
    #[derive(Debug)]
    pub struct RuleCombinerParametersTypeDeserializer {
        rule_id_ref: super::StringType,
        combiner_parameter: Vec<super::CombinerParameter>,
        state__: Box<RuleCombinerParametersTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum RuleCombinerParametersTypeDeserializerState {
        Init__,
        CombinerParameter(Option<<super::CombinerParameter as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl RuleCombinerParametersTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut rule_id_ref: Option<super::StringType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"RuleIdRef")
                ) {
                    reader.read_attrib(&mut rule_id_ref, b"RuleIdRef", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                rule_id_ref: rule_id_ref.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("RuleIdRef".into()))
                })?,
                combiner_parameter: Vec::new(),
                state__: Box::new(RuleCombinerParametersTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: RuleCombinerParametersTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use RuleCombinerParametersTypeDeserializerState as S;
            match state {
                S::CombinerParameter(Some(deserializer)) => {
                    self.store_combiner_parameter(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_combiner_parameter(
            &mut self,
            value: super::CombinerParameter,
        ) -> std::result::Result<(), Error> {
            self.combiner_parameter.push(value);
            Ok(())
        }
        fn handle_combiner_parameter<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::CombinerParameter>,
            fallback: &mut Option<RuleCombinerParametersTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(
                    RuleCombinerParametersTypeDeserializerState::CombinerParameter(None),
                );
                *self.state__ = RuleCombinerParametersTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_combiner_parameter(data)?;
                    *self.state__ =
                        RuleCombinerParametersTypeDeserializerState::CombinerParameter(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                RuleCombinerParametersTypeDeserializerState::CombinerParameter(
                                    Some(deserializer),
                                ),
                            );
                            *self.state__ =
                                RuleCombinerParametersTypeDeserializerState::CombinerParameter(
                                    None,
                                );
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                RuleCombinerParametersTypeDeserializerState::CombinerParameter(
                                    Some(deserializer),
                                );
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::RuleCombinerParametersType>
        for RuleCombinerParametersTypeDeserializer
    {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::RuleCombinerParametersType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::RuleCombinerParametersType>
        where
            R: DeserializeReader,
        {
            use RuleCombinerParametersTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::CombinerParameter(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_combiner_parameter(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ =
                            RuleCombinerParametersTypeDeserializerState::CombinerParameter(None);
                        event
                    }
                    (S::CombinerParameter(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"CombinerParameter",
                            false,
                        )?;
                        match self.handle_combiner_parameter(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::RuleCombinerParametersType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                RuleCombinerParametersTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::RuleCombinerParametersType {
                rule_id_ref: self.rule_id_ref,
                combiner_parameter: self.combiner_parameter,
            })
        }
    }
    #[derive(Debug)]
    pub struct RuleTypeDeserializer {
        rule_id: super::StringType,
        effect: super::EffectType,
        description: Option<super::Description>,
        target: Option<super::Target>,
        condition: Option<super::Condition>,
        obligation_expressions: Option<super::ObligationExpressions>,
        advice_expressions: Option<super::AdviceExpressions>,
        state__: Box<RuleTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum RuleTypeDeserializerState {
        Init__,
        Description(Option<<super::Description as WithDeserializer>::Deserializer>),
        Target(Option<<super::Target as WithDeserializer>::Deserializer>),
        Condition(Option<<super::Condition as WithDeserializer>::Deserializer>),
        ObligationExpressions(
            Option<<super::ObligationExpressions as WithDeserializer>::Deserializer>,
        ),
        AdviceExpressions(Option<<super::AdviceExpressions as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl RuleTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut rule_id: Option<super::StringType> = None;
            let mut effect: Option<super::EffectType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"RuleId")
                ) {
                    reader.read_attrib(&mut rule_id, b"RuleId", &attrib.value)?;
                } else if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Effect")
                ) {
                    reader.read_attrib(&mut effect, b"Effect", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                rule_id: rule_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("RuleId".into()))
                })?,
                effect: effect.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("Effect".into()))
                })?,
                description: None,
                target: None,
                condition: None,
                obligation_expressions: None,
                advice_expressions: None,
                state__: Box::new(RuleTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: RuleTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use RuleTypeDeserializerState as S;
            match state {
                S::Description(Some(deserializer)) => {
                    self.store_description(deserializer.finish(reader)?)?
                }
                S::Target(Some(deserializer)) => self.store_target(deserializer.finish(reader)?)?,
                S::Condition(Some(deserializer)) => {
                    self.store_condition(deserializer.finish(reader)?)?
                }
                S::ObligationExpressions(Some(deserializer)) => {
                    self.store_obligation_expressions(deserializer.finish(reader)?)?
                }
                S::AdviceExpressions(Some(deserializer)) => {
                    self.store_advice_expressions(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_description(&mut self, value: super::Description) -> std::result::Result<(), Error> {
            if self.description.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Description",
                )))?;
            }
            self.description = Some(value);
            Ok(())
        }
        fn store_target(&mut self, value: super::Target) -> std::result::Result<(), Error> {
            if self.target.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Target",
                )))?;
            }
            self.target = Some(value);
            Ok(())
        }
        fn store_condition(&mut self, value: super::Condition) -> std::result::Result<(), Error> {
            if self.condition.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Condition",
                )))?;
            }
            self.condition = Some(value);
            Ok(())
        }
        fn store_obligation_expressions(
            &mut self,
            value: super::ObligationExpressions,
        ) -> std::result::Result<(), Error> {
            if self.obligation_expressions.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"ObligationExpressions",
                )))?;
            }
            self.obligation_expressions = Some(value);
            Ok(())
        }
        fn store_advice_expressions(
            &mut self,
            value: super::AdviceExpressions,
        ) -> std::result::Result<(), Error> {
            if self.advice_expressions.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"AdviceExpressions",
                )))?;
            }
            self.advice_expressions = Some(value);
            Ok(())
        }
        fn handle_description<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Description>,
            fallback: &mut Option<RuleTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(RuleTypeDeserializerState::Description(None));
                *self.state__ = RuleTypeDeserializerState::Target(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_description(data)?;
                    *self.state__ = RuleTypeDeserializerState::Target(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(RuleTypeDeserializerState::Description(Some(
                                deserializer,
                            )));
                            *self.state__ = RuleTypeDeserializerState::Target(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                RuleTypeDeserializerState::Description(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_target<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Target>,
            fallback: &mut Option<RuleTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(RuleTypeDeserializerState::Target(None));
                *self.state__ = RuleTypeDeserializerState::Condition(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_target(data)?;
                    *self.state__ = RuleTypeDeserializerState::Condition(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(RuleTypeDeserializerState::Target(Some(
                                deserializer,
                            )));
                            *self.state__ = RuleTypeDeserializerState::Condition(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = RuleTypeDeserializerState::Target(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_condition<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Condition>,
            fallback: &mut Option<RuleTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(RuleTypeDeserializerState::Condition(None));
                *self.state__ = RuleTypeDeserializerState::ObligationExpressions(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_condition(data)?;
                    *self.state__ = RuleTypeDeserializerState::ObligationExpressions(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(RuleTypeDeserializerState::Condition(Some(
                                deserializer,
                            )));
                            *self.state__ = RuleTypeDeserializerState::ObligationExpressions(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                RuleTypeDeserializerState::Condition(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_obligation_expressions<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::ObligationExpressions>,
            fallback: &mut Option<RuleTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(RuleTypeDeserializerState::ObligationExpressions(None));
                *self.state__ = RuleTypeDeserializerState::AdviceExpressions(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_obligation_expressions(data)?;
                    *self.state__ = RuleTypeDeserializerState::AdviceExpressions(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                RuleTypeDeserializerState::ObligationExpressions(Some(
                                    deserializer,
                                )),
                            );
                            *self.state__ = RuleTypeDeserializerState::AdviceExpressions(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = RuleTypeDeserializerState::ObligationExpressions(Some(
                                deserializer,
                            ));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_advice_expressions<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AdviceExpressions>,
            fallback: &mut Option<RuleTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(RuleTypeDeserializerState::AdviceExpressions(None));
                *self.state__ = RuleTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_advice_expressions(data)?;
                    *self.state__ = RuleTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(RuleTypeDeserializerState::AdviceExpressions(
                                Some(deserializer),
                            ));
                            *self.state__ = RuleTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                RuleTypeDeserializerState::AdviceExpressions(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::RuleType> for RuleTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::RuleType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::RuleType>
        where
            R: DeserializeReader,
        {
            use RuleTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Description(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_description(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Target(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_target(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Condition(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_condition(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::ObligationExpressions(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_obligation_expressions(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::AdviceExpressions(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_advice_expressions(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = RuleTypeDeserializerState::Description(None);
                        event
                    }
                    (S::Description(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Description",
                            false,
                        )?;
                        match self.handle_description(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Target(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Target",
                            false,
                        )?;
                        match self.handle_target(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Condition(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Condition",
                            false,
                        )?;
                        match self.handle_condition(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (
                        S::ObligationExpressions(None),
                        event @ (Event::Start(_) | Event::Empty(_)),
                    ) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"ObligationExpressions",
                            false,
                        )?;
                        match self.handle_obligation_expressions(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::AdviceExpressions(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AdviceExpressions",
                            false,
                        )?;
                        match self.handle_advice_expressions(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::RuleType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(&mut *self.state__, RuleTypeDeserializerState::Unknown__);
            self.finish_state(reader, state)?;
            Ok(super::RuleType {
                rule_id: self.rule_id,
                effect: self.effect,
                description: self.description,
                target: self.target,
                condition: self.condition,
                obligation_expressions: self.obligation_expressions,
                advice_expressions: self.advice_expressions,
            })
        }
    }
    #[derive(Debug)]
    pub struct StatusCodeTypeDeserializer {
        value: super::AnyUriType,
        status_code: Option<super::StatusCode>,
        state__: Box<StatusCodeTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum StatusCodeTypeDeserializerState {
        Init__,
        StatusCode(Option<<super::StatusCode as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl StatusCodeTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut value: Option<super::AnyUriType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"Value")
                ) {
                    reader.read_attrib(&mut value, b"Value", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                value: value
                    .ok_or_else(|| reader.map_error(ErrorKind::MissingAttribute("Value".into())))?,
                status_code: None,
                state__: Box::new(StatusCodeTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: StatusCodeTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use StatusCodeTypeDeserializerState as S;
            match state {
                S::StatusCode(Some(deserializer)) => {
                    self.store_status_code(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_status_code(&mut self, value: super::StatusCode) -> std::result::Result<(), Error> {
            if self.status_code.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"StatusCode",
                )))?;
            }
            self.status_code = Some(value);
            Ok(())
        }
        fn handle_status_code<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::StatusCode>,
            fallback: &mut Option<StatusCodeTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(StatusCodeTypeDeserializerState::StatusCode(None));
                *self.state__ = StatusCodeTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_status_code(data)?;
                    *self.state__ = StatusCodeTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(StatusCodeTypeDeserializerState::StatusCode(
                                Some(deserializer),
                            ));
                            *self.state__ = StatusCodeTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                StatusCodeTypeDeserializerState::StatusCode(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::StatusCodeType> for StatusCodeTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::StatusCodeType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::StatusCodeType>
        where
            R: DeserializeReader,
        {
            use StatusCodeTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::StatusCode(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_status_code(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = StatusCodeTypeDeserializerState::StatusCode(None);
                        event
                    }
                    (S::StatusCode(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"StatusCode",
                            false,
                        )?;
                        match self.handle_status_code(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::StatusCodeType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                StatusCodeTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::StatusCodeType {
                value: self.value,
                status_code: self.status_code.map(Box::new),
            })
        }
    }
    #[derive(Debug)]
    pub struct StatusDetailTypeDeserializer {
        state__: Box<StatusDetailTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum StatusDetailTypeDeserializerState {
        Init__,
        Unknown__,
    }
    impl StatusDetailTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                state__: Box::new(StatusDetailTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: StatusDetailTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            Ok(())
        }
    }
    impl<'de> Deserializer<'de, super::StatusDetailType> for StatusDetailTypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::StatusDetailType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::StatusDetailType>
        where
            R: DeserializeReader,
        {
            if let Event::End(_) = &event {
                Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::Data(self.finish(reader)?),
                    event: DeserializerEvent::None,
                    allow_any: false,
                })
            } else {
                Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::Deserializer(self),
                    event: DeserializerEvent::Break(event),
                    allow_any: true,
                })
            }
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::StatusDetailType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                StatusDetailTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::StatusDetailType {})
        }
    }
    #[derive(Debug)]
    pub struct StatusTypeDeserializer {
        status_code: Option<super::StatusCode>,
        status_message: Option<super::StatusMessage>,
        status_detail: Option<super::StatusDetail>,
        state__: Box<StatusTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum StatusTypeDeserializerState {
        Init__,
        StatusCode(Option<<super::StatusCode as WithDeserializer>::Deserializer>),
        StatusMessage(Option<<super::StatusMessage as WithDeserializer>::Deserializer>),
        StatusDetail(Option<<super::StatusDetail as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl StatusTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                status_code: None,
                status_message: None,
                status_detail: None,
                state__: Box::new(StatusTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: StatusTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use StatusTypeDeserializerState as S;
            match state {
                S::StatusCode(Some(deserializer)) => {
                    self.store_status_code(deserializer.finish(reader)?)?
                }
                S::StatusMessage(Some(deserializer)) => {
                    self.store_status_message(deserializer.finish(reader)?)?
                }
                S::StatusDetail(Some(deserializer)) => {
                    self.store_status_detail(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_status_code(&mut self, value: super::StatusCode) -> std::result::Result<(), Error> {
            if self.status_code.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"StatusCode",
                )))?;
            }
            self.status_code = Some(value);
            Ok(())
        }
        fn store_status_message(&mut self, value: super::StatusMessage) -> std::result::Result<(), Error> {
            if self.status_message.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"StatusMessage",
                )))?;
            }
            self.status_message = Some(value);
            Ok(())
        }
        fn store_status_detail(&mut self, value: super::StatusDetail) -> std::result::Result<(), Error> {
            if self.status_detail.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"StatusDetail",
                )))?;
            }
            self.status_detail = Some(value);
            Ok(())
        }
        fn handle_status_code<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::StatusCode>,
            fallback: &mut Option<StatusTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.status_code.is_some() {
                    fallback.get_or_insert(StatusTypeDeserializerState::StatusCode(None));
                    *self.state__ = StatusTypeDeserializerState::StatusMessage(None);
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                } else {
                    *self.state__ = StatusTypeDeserializerState::StatusCode(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_status_code(data)?;
                    *self.state__ = StatusTypeDeserializerState::StatusMessage(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(StatusTypeDeserializerState::StatusCode(Some(
                                deserializer,
                            )));
                            *self.state__ = StatusTypeDeserializerState::StatusMessage(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                StatusTypeDeserializerState::StatusCode(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_status_message<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::StatusMessage>,
            fallback: &mut Option<StatusTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(StatusTypeDeserializerState::StatusMessage(None));
                *self.state__ = StatusTypeDeserializerState::StatusDetail(None);
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_status_message(data)?;
                    *self.state__ = StatusTypeDeserializerState::StatusDetail(None);
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(StatusTypeDeserializerState::StatusMessage(
                                Some(deserializer),
                            ));
                            *self.state__ = StatusTypeDeserializerState::StatusDetail(None);
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                StatusTypeDeserializerState::StatusMessage(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
        fn handle_status_detail<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::StatusDetail>,
            fallback: &mut Option<StatusTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                fallback.get_or_insert(StatusTypeDeserializerState::StatusDetail(None));
                *self.state__ = StatusTypeDeserializerState::Done__;
                return Ok(ElementHandlerOutput::from_event(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_status_detail(data)?;
                    *self.state__ = StatusTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(StatusTypeDeserializerState::StatusDetail(
                                Some(deserializer),
                            ));
                            *self.state__ = StatusTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                StatusTypeDeserializerState::StatusDetail(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::StatusType> for StatusTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::StatusType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::StatusType>
        where
            R: DeserializeReader,
        {
            use StatusTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::StatusCode(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_status_code(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::StatusMessage(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_status_message(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::StatusDetail(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_status_detail(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = StatusTypeDeserializerState::StatusCode(None);
                        event
                    }
                    (S::StatusCode(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"StatusCode",
                            false,
                        )?;
                        match self.handle_status_code(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::StatusMessage(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"StatusMessage",
                            false,
                        )?;
                        match self.handle_status_message(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::StatusDetail(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"StatusDetail",
                            false,
                        )?;
                        match self.handle_status_detail(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::StatusType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(&mut *self.state__, StatusTypeDeserializerState::Unknown__);
            self.finish_state(reader, state)?;
            Ok(super::StatusType {
                status_code: self
                    .status_code
                    .ok_or_else(|| ErrorKind::MissingElement("StatusCode".into()))?,
                status_message: self.status_message,
                status_detail: self.status_detail,
            })
        }
    }
    #[derive(Debug)]
    pub struct TargetTypeDeserializer {
        content: Vec<super::TargetTypeContent>,
        state__: Box<TargetTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum TargetTypeDeserializerState {
        Init__,
        Next__,
        Content__(<super::TargetTypeContent as WithDeserializer>::Deserializer),
        Unknown__,
    }
    impl TargetTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                reader.raise_unexpected_attrib_checked(attrib)?;
            }
            Ok(Self {
                content: Vec::new(),
                state__: Box::new(TargetTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: TargetTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            if let TargetTypeDeserializerState::Content__(deserializer) = state {
                self.store_content(deserializer.finish(reader)?)?;
            }
            Ok(())
        }
        fn store_content(&mut self, value: super::TargetTypeContent) -> std::result::Result<(), Error> {
            self.content.push(value);
            Ok(())
        }
        fn handle_content<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::TargetTypeContent>,
            fallback: &mut Option<TargetTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = fallback
                    .take()
                    .unwrap_or(TargetTypeDeserializerState::Next__);
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_content(data)?;
                    *self.state__ = TargetTypeDeserializerState::Next__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = TargetTypeDeserializerState::Content__(deserializer);
                        }
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(TargetTypeDeserializerState::Content__(
                                deserializer,
                            ));
                            *self.state__ = TargetTypeDeserializerState::Next__;
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::TargetType> for TargetTypeDeserializer {
        fn init<R>(reader: &R, event: Event<'de>) -> DeserializerResult<'de, super::TargetType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::TargetType>
        where
            R: DeserializeReader,
        {
            use TargetTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Content__(deserializer), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_content(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (_, Event::End(_)) => {
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (state @ (S::Init__ | S::Next__), event) => {
                        fallback.get_or_insert(state);
                        let output =
                            <super::TargetTypeContent as WithDeserializer>::Deserializer::init(
                                reader, event,
                            )?;
                        match self.handle_content(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::Unknown__, _) => unreachable!(),
                }
            };
            let artifact = DeserializerArtifact::Deserializer(self);
            Ok(DeserializerOutput {
                artifact,
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::TargetType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(&mut *self.state__, TargetTypeDeserializerState::Unknown__);
            self.finish_state(reader, state)?;
            Ok(super::TargetType {
                content: self.content,
            })
        }
    }
    #[derive(Debug)]
    pub struct TargetTypeContentDeserializer {
        any_of: Option<super::AnyOf>,
        state__: Box<TargetTypeContentDeserializerState>,
    }
    #[derive(Debug)]
    enum TargetTypeContentDeserializerState {
        Init__,
        AnyOf(Option<<super::AnyOf as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl TargetTypeContentDeserializer {
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: TargetTypeContentDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use TargetTypeContentDeserializerState as S;
            match state {
                S::AnyOf(Some(deserializer)) => self.store_any_of(deserializer.finish(reader)?)?,
                _ => (),
            }
            Ok(())
        }
        fn store_any_of(&mut self, value: super::AnyOf) -> std::result::Result<(), Error> {
            if self.any_of.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"AnyOf",
                )))?;
            }
            self.any_of = Some(value);
            Ok(())
        }
        fn handle_any_of<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::AnyOf>,
            fallback: &mut Option<TargetTypeContentDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.any_of.is_some() {
                    fallback.get_or_insert(TargetTypeContentDeserializerState::AnyOf(None));
                    *self.state__ = TargetTypeContentDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                } else {
                    *self.state__ = TargetTypeContentDeserializerState::AnyOf(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_any_of(data)?;
                    *self.state__ = TargetTypeContentDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(TargetTypeContentDeserializerState::AnyOf(
                                Some(deserializer),
                            ));
                            *self.state__ = TargetTypeContentDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ =
                                TargetTypeContentDeserializerState::AnyOf(Some(deserializer));
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::TargetTypeContent> for TargetTypeContentDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::TargetTypeContent>
        where
            R: DeserializeReader,
        {
            let deserializer = Self {
                any_of: None,
                state__: Box::new(TargetTypeContentDeserializerState::Init__),
            };
            let mut output = deserializer.next(reader, event)?;
            output.artifact = match output.artifact {
                DeserializerArtifact::Deserializer(x)
                    if matches!(&*x.state__, TargetTypeContentDeserializerState::Init__) =>
                {
                    DeserializerArtifact::None
                }
                artifact => artifact,
            };
            Ok(output)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::TargetTypeContent>
        where
            R: DeserializeReader,
        {
            use TargetTypeContentDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::AnyOf(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_any_of(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, event @ Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::Continue(event),
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = TargetTypeContentDeserializerState::AnyOf(None);
                        event
                    }
                    (S::AnyOf(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AnyOf",
                            false,
                        )?;
                        match self.handle_any_of(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::TargetTypeContent, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                TargetTypeContentDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::TargetTypeContent {
                any_of: self
                    .any_of
                    .ok_or_else(|| ErrorKind::MissingElement("AnyOf".into()))?,
            })
        }
    }
    #[derive(Debug)]
    pub struct VariableDefinitionTypeDeserializer {
        variable_id: super::StringType,
        expression: Option<super::Expression>,
        state__: Box<VariableDefinitionTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum VariableDefinitionTypeDeserializerState {
        Init__,
        Expression(Option<<super::Expression as WithDeserializer>::Deserializer>),
        Done__,
        Unknown__,
    }
    impl VariableDefinitionTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut variable_id: Option<super::StringType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"VariableId")
                ) {
                    reader.read_attrib(&mut variable_id, b"VariableId", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                variable_id: variable_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("VariableId".into()))
                })?,
                expression: None,
                state__: Box::new(VariableDefinitionTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: VariableDefinitionTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            use VariableDefinitionTypeDeserializerState as S;
            match state {
                S::Expression(Some(deserializer)) => {
                    self.store_expression(deserializer.finish(reader)?)?
                }
                _ => (),
            }
            Ok(())
        }
        fn store_expression(&mut self, value: super::Expression) -> std::result::Result<(), Error> {
            if self.expression.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Expression",
                )))?;
            }
            self.expression = Some(value);
            Ok(())
        }
        fn handle_expression<'de, R>(
            &mut self,
            reader: &R,
            output: DeserializerOutput<'de, super::Expression>,
            fallback: &mut Option<VariableDefinitionTypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                if self.expression.is_some() {
                    fallback
                        .get_or_insert(VariableDefinitionTypeDeserializerState::Expression(None));
                    *self.state__ = VariableDefinitionTypeDeserializerState::Done__;
                    return Ok(ElementHandlerOutput::from_event(event, allow_any));
                } else {
                    *self.state__ = VariableDefinitionTypeDeserializerState::Expression(None);
                    return Ok(ElementHandlerOutput::break_(event, allow_any));
                }
            }
            if let Some(fallback) = fallback.take() {
                self.finish_state(reader, fallback)?;
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    self.store_expression(data)?;
                    *self.state__ = VariableDefinitionTypeDeserializerState::Done__;
                    ElementHandlerOutput::from_event(event, allow_any)
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    let ret = ElementHandlerOutput::from_event(event, allow_any);
                    match &ret {
                        ElementHandlerOutput::Continue { .. } => {
                            fallback.get_or_insert(
                                VariableDefinitionTypeDeserializerState::Expression(Some(
                                    deserializer,
                                )),
                            );
                            *self.state__ = VariableDefinitionTypeDeserializerState::Done__;
                        }
                        ElementHandlerOutput::Break { .. } => {
                            *self.state__ = VariableDefinitionTypeDeserializerState::Expression(
                                Some(deserializer),
                            );
                        }
                    }
                    ret
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::VariableDefinitionType> for VariableDefinitionTypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::VariableDefinitionType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::VariableDefinitionType>
        where
            R: DeserializeReader,
        {
            use VariableDefinitionTypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let mut allow_any_element = false;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::Expression(Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_expression(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (_, Event::End(_)) => {
                        if let Some(fallback) = fallback.take() {
                            self.finish_state(reader, fallback)?;
                        }
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(self.finish(reader)?),
                            event: DeserializerEvent::None,
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => {
                        fallback.get_or_insert(S::Init__);
                        *self.state__ = VariableDefinitionTypeDeserializerState::Expression(None);
                        event
                    }
                    (S::Expression(None), event @ (Event::Start(_) | Event::Empty(_))) => {
                        let output = <super::Expression as WithDeserializer>::Deserializer::init(
                            reader, event,
                        )?;
                        match self.handle_expression(reader, output, &mut fallback)? {
                            ElementHandlerOutput::Continue { event, allow_any } => {
                                allow_any_element = allow_any_element || allow_any;
                                event
                            }
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                        }
                    }
                    (S::Done__, event) => {
                        fallback.get_or_insert(S::Done__);
                        break (DeserializerEvent::Continue(event), allow_any_element);
                    }
                    (S::Unknown__, _) => unreachable!(),
                    (state, event) => {
                        *self.state__ = state;
                        break (DeserializerEvent::Break(event), false);
                    }
                }
            };
            if let Some(fallback) = fallback {
                *self.state__ = fallback;
            }
            Ok(DeserializerOutput {
                artifact: DeserializerArtifact::Deserializer(self),
                event,
                allow_any,
            })
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::VariableDefinitionType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                VariableDefinitionTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::VariableDefinitionType {
                variable_id: self.variable_id,
                expression: self
                    .expression
                    .ok_or_else(|| ErrorKind::MissingElement("Expression".into()))?,
            })
        }
    }
    #[derive(Debug)]
    pub struct VariableReferenceTypeDeserializer {
        variable_id: super::StringType,
        state__: Box<VariableReferenceTypeDeserializerState>,
    }
    #[derive(Debug)]
    enum VariableReferenceTypeDeserializerState {
        Init__,
        Unknown__,
    }
    impl VariableReferenceTypeDeserializer {
        fn from_bytes_start<R>(reader: &R, bytes_start: &BytesStart<'_>) -> std::result::Result<Self, Error>
        where
            R: DeserializeReader,
        {
            let mut variable_id: Option<super::StringType> = None;
            for attrib in filter_xmlns_attributes(bytes_start) {
                let attrib = attrib?;
                if matches!(
                    reader.resolve_local_name(attrib.key, &super::NS_XACML),
                    Some(b"VariableId")
                ) {
                    reader.read_attrib(&mut variable_id, b"VariableId", &attrib.value)?;
                } else {
                    reader.raise_unexpected_attrib_checked(attrib)?;
                }
            }
            Ok(Self {
                variable_id: variable_id.ok_or_else(|| {
                    reader.map_error(ErrorKind::MissingAttribute("VariableId".into()))
                })?,
                state__: Box::new(VariableReferenceTypeDeserializerState::Init__),
            })
        }
        fn finish_state<R>(
            &mut self,
            reader: &R,
            state: VariableReferenceTypeDeserializerState,
        ) -> std::result::Result<(), Error>
        where
            R: DeserializeReader,
        {
            Ok(())
        }
    }
    impl<'de> Deserializer<'de, super::VariableReferenceType> for VariableReferenceTypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::VariableReferenceType>
        where
            R: DeserializeReader,
        {
            reader.init_deserializer_from_start_event(event, Self::from_bytes_start)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::VariableReferenceType>
        where
            R: DeserializeReader,
        {
            if let Event::End(_) = &event {
                Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::Data(self.finish(reader)?),
                    event: DeserializerEvent::None,
                    allow_any: false,
                })
            } else {
                Ok(DeserializerOutput {
                    artifact: DeserializerArtifact::Deserializer(self),
                    event: DeserializerEvent::Break(event),
                    allow_any: false,
                })
            }
        }
        fn finish<R>(mut self, reader: &R) -> std::result::Result<super::VariableReferenceType, Error>
        where
            R: DeserializeReader,
        {
            let state = replace(
                &mut *self.state__,
                VariableReferenceTypeDeserializerState::Unknown__,
            );
            self.finish_state(reader, state)?;
            Ok(super::VariableReferenceType {
                variable_id: self.variable_id,
            })
        }
    }
    #[derive(Debug)]
    pub struct DefaultsContent39TypeDeserializer {
        state__: Box<DefaultsContent39TypeDeserializerState>,
    }
    #[derive(Debug)]
    pub enum DefaultsContent39TypeDeserializerState {
        Init__,
        XPathVersion(
            Option<super::XPathVersion>,
            Option<<super::XPathVersion as WithDeserializer>::Deserializer>,
        ),
        Done__(super::DefaultsContent39Type),
        Unknown__,
    }
    impl DefaultsContent39TypeDeserializer {
        fn find_suitable<'de, R>(
            &mut self,
            reader: &R,
            event: Event<'de>,
            fallback: &mut Option<DefaultsContent39TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            if let Event::Start(x) | Event::Empty(x) = &event {
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"XPathVersion")
                ) {
                    let output = <super::XPathVersion as WithDeserializer>::Deserializer::init(
                        reader, event,
                    )?;
                    return self.handle_x_path_version(
                        reader,
                        Default::default(),
                        output,
                        &mut *fallback,
                    );
                }
            }
            *self.state__ = fallback
                .take()
                .unwrap_or(DefaultsContent39TypeDeserializerState::Init__);
            Ok(ElementHandlerOutput::return_to_parent(event, false))
        }
        fn finish_state<R>(
            reader: &R,
            state: DefaultsContent39TypeDeserializerState,
        ) -> std::result::Result<super::DefaultsContent39Type, Error>
        where
            R: DeserializeReader,
        {
            use DefaultsContent39TypeDeserializerState as S;
            match state {
                S::Init__ => Err(ErrorKind::MissingContent.into()),
                S::XPathVersion(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_x_path_version(&mut values, value)?;
                    }
                    Ok(super::DefaultsContent39Type::XPathVersion(
                        values.ok_or_else(|| ErrorKind::MissingElement("XPathVersion".into()))?,
                    ))
                }
                S::Done__(data) => Ok(data),
                S::Unknown__ => unreachable!(),
            }
        }
        fn store_x_path_version(
            values: &mut Option<super::XPathVersion>,
            value: super::XPathVersion,
        ) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"XPathVersion",
                )))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn handle_x_path_version<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::XPathVersion>,
            output: DeserializerOutput<'de, super::XPathVersion>,
            fallback: &mut Option<DefaultsContent39TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None if values.is_none() => {
                        *self.state__ = DefaultsContent39TypeDeserializerState::Init__;
                        return Ok(ElementHandlerOutput::from_event(event, allow_any));
                    }
                    None => DefaultsContent39TypeDeserializerState::XPathVersion(values, None),
                    Some(DefaultsContent39TypeDeserializerState::XPathVersion(
                        _,
                        Some(deserializer),
                    )) => DefaultsContent39TypeDeserializerState::XPathVersion(
                        values,
                        Some(deserializer),
                    ),
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(DefaultsContent39TypeDeserializerState::XPathVersion(
                    _,
                    Some(deserializer),
                )) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_x_path_version(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_x_path_version(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        DefaultsContent39TypeDeserializerState::XPathVersion(values, None),
                    )?;
                    *self.state__ = DefaultsContent39TypeDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ = DefaultsContent39TypeDeserializerState::XPathVersion(
                        values,
                        Some(deserializer),
                    );
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::DefaultsContent39Type> for DefaultsContent39TypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::DefaultsContent39Type>
        where
            R: DeserializeReader,
        {
            let deserializer = Self {
                state__: Box::new(DefaultsContent39TypeDeserializerState::Init__),
            };
            let mut output = deserializer.next(reader, event)?;
            output.artifact = match output.artifact {
                DeserializerArtifact::Deserializer(x)
                    if matches!(&*x.state__, DefaultsContent39TypeDeserializerState::Init__) =>
                {
                    DeserializerArtifact::None
                }
                artifact => artifact,
            };
            Ok(output)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::DefaultsContent39Type>
        where
            R: DeserializeReader,
        {
            use DefaultsContent39TypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::XPathVersion(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_x_path_version(reader, values, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (state, event @ Event::End(_)) => {
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(Self::finish_state(
                                reader, state,
                            )?),
                            event: DeserializerEvent::Continue(event),
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => match self.find_suitable(reader, event, &mut fallback)? {
                        ElementHandlerOutput::Break { event, allow_any } => {
                            break (event, allow_any)
                        }
                        ElementHandlerOutput::Continue { event, .. } => event,
                    },
                    (S::XPathVersion(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"XPathVersion",
                            false,
                        )?;
                        match self.handle_x_path_version(reader, values, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (s @ S::Done__(_), event) => {
                        *self.state__ = s;
                        break (DeserializerEvent::Continue(event), false);
                    }
                    (S::Unknown__, _) => unreachable!(),
                }
            };
            let artifact = if matches!(&*self.state__, S::Done__(_)) {
                DeserializerArtifact::Data(self.finish(reader)?)
            } else {
                DeserializerArtifact::Deserializer(self)
            };
            Ok(DeserializerOutput {
                artifact,
                event,
                allow_any,
            })
        }
        fn finish<R>(self, reader: &R) -> std::result::Result<super::DefaultsContent39Type, Error>
        where
            R: DeserializeReader,
        {
            Self::finish_state(reader, *self.state__)
        }
    }
    #[derive(Debug)]
    pub struct MatchContent47TypeDeserializer {
        state__: Box<MatchContent47TypeDeserializerState>,
    }
    #[derive(Debug)]
    pub enum MatchContent47TypeDeserializerState {
        Init__,
        AttributeDesignator(
            Option<super::AttributeDesignator>,
            Option<<super::AttributeDesignator as WithDeserializer>::Deserializer>,
        ),
        AttributeSelector(
            Option<super::AttributeSelector>,
            Option<<super::AttributeSelector as WithDeserializer>::Deserializer>,
        ),
        Done__(super::MatchContent47Type),
        Unknown__,
    }
    impl MatchContent47TypeDeserializer {
        fn find_suitable<'de, R>(
            &mut self,
            reader: &R,
            event: Event<'de>,
            fallback: &mut Option<MatchContent47TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            if let Event::Start(x) | Event::Empty(x) = &event {
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"AttributeDesignator")
                ) {
                    let output =
                        <super::AttributeDesignator as WithDeserializer>::Deserializer::init(
                            reader, event,
                        )?;
                    return self.handle_attribute_designator(
                        reader,
                        Default::default(),
                        output,
                        &mut *fallback,
                    );
                }
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"AttributeSelector")
                ) {
                    let output =
                        <super::AttributeSelector as WithDeserializer>::Deserializer::init(
                            reader, event,
                        )?;
                    return self.handle_attribute_selector(
                        reader,
                        Default::default(),
                        output,
                        &mut *fallback,
                    );
                }
            }
            *self.state__ = fallback
                .take()
                .unwrap_or(MatchContent47TypeDeserializerState::Init__);
            Ok(ElementHandlerOutput::return_to_parent(event, false))
        }
        fn finish_state<R>(
            reader: &R,
            state: MatchContent47TypeDeserializerState,
        ) -> std::result::Result<super::MatchContent47Type, Error>
        where
            R: DeserializeReader,
        {
            use MatchContent47TypeDeserializerState as S;
            match state {
                S::Init__ => Err(ErrorKind::MissingContent.into()),
                S::AttributeDesignator(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_attribute_designator(&mut values, value)?;
                    }
                    Ok(super::MatchContent47Type::AttributeDesignator(
                        values.ok_or_else(|| {
                            ErrorKind::MissingElement("AttributeDesignator".into())
                        })?,
                    ))
                }
                S::AttributeSelector(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_attribute_selector(&mut values, value)?;
                    }
                    Ok(super::MatchContent47Type::AttributeSelector(
                        values
                            .ok_or_else(|| ErrorKind::MissingElement("AttributeSelector".into()))?,
                    ))
                }
                S::Done__(data) => Ok(data),
                S::Unknown__ => unreachable!(),
            }
        }
        fn store_attribute_designator(
            values: &mut Option<super::AttributeDesignator>,
            value: super::AttributeDesignator,
        ) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"AttributeDesignator",
                )))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn store_attribute_selector(
            values: &mut Option<super::AttributeSelector>,
            value: super::AttributeSelector,
        ) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"AttributeSelector",
                )))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn handle_attribute_designator<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::AttributeDesignator>,
            output: DeserializerOutput<'de, super::AttributeDesignator>,
            fallback: &mut Option<MatchContent47TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None if values.is_none() => {
                        *self.state__ = MatchContent47TypeDeserializerState::Init__;
                        return Ok(ElementHandlerOutput::from_event(event, allow_any));
                    }
                    None => MatchContent47TypeDeserializerState::AttributeDesignator(values, None),
                    Some(MatchContent47TypeDeserializerState::AttributeDesignator(
                        _,
                        Some(deserializer),
                    )) => MatchContent47TypeDeserializerState::AttributeDesignator(
                        values,
                        Some(deserializer),
                    ),
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(MatchContent47TypeDeserializerState::AttributeDesignator(
                    _,
                    Some(deserializer),
                )) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_attribute_designator(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_attribute_designator(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        MatchContent47TypeDeserializerState::AttributeDesignator(values, None),
                    )?;
                    *self.state__ = MatchContent47TypeDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ = MatchContent47TypeDeserializerState::AttributeDesignator(
                        values,
                        Some(deserializer),
                    );
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
        fn handle_attribute_selector<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::AttributeSelector>,
            output: DeserializerOutput<'de, super::AttributeSelector>,
            fallback: &mut Option<MatchContent47TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None if values.is_none() => {
                        *self.state__ = MatchContent47TypeDeserializerState::Init__;
                        return Ok(ElementHandlerOutput::from_event(event, allow_any));
                    }
                    None => MatchContent47TypeDeserializerState::AttributeSelector(values, None),
                    Some(MatchContent47TypeDeserializerState::AttributeSelector(
                        _,
                        Some(deserializer),
                    )) => MatchContent47TypeDeserializerState::AttributeSelector(
                        values,
                        Some(deserializer),
                    ),
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(MatchContent47TypeDeserializerState::AttributeSelector(
                    _,
                    Some(deserializer),
                )) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_attribute_selector(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_attribute_selector(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        MatchContent47TypeDeserializerState::AttributeSelector(values, None),
                    )?;
                    *self.state__ = MatchContent47TypeDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ = MatchContent47TypeDeserializerState::AttributeSelector(
                        values,
                        Some(deserializer),
                    );
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::MatchContent47Type> for MatchContent47TypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::MatchContent47Type>
        where
            R: DeserializeReader,
        {
            let deserializer = Self {
                state__: Box::new(MatchContent47TypeDeserializerState::Init__),
            };
            let mut output = deserializer.next(reader, event)?;
            output.artifact = match output.artifact {
                DeserializerArtifact::Deserializer(x)
                    if matches!(&*x.state__, MatchContent47TypeDeserializerState::Init__) =>
                {
                    DeserializerArtifact::None
                }
                artifact => artifact,
            };
            Ok(output)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::MatchContent47Type>
        where
            R: DeserializeReader,
        {
            use MatchContent47TypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::AttributeDesignator(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_attribute_designator(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::AttributeSelector(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_attribute_selector(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (state, event @ Event::End(_)) => {
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(Self::finish_state(
                                reader, state,
                            )?),
                            event: DeserializerEvent::Continue(event),
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => match self.find_suitable(reader, event, &mut fallback)? {
                        ElementHandlerOutput::Break { event, allow_any } => {
                            break (event, allow_any)
                        }
                        ElementHandlerOutput::Continue { event, .. } => event,
                    },
                    (S::AttributeDesignator(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AttributeDesignator",
                            false,
                        )?;
                        match self.handle_attribute_designator(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::AttributeSelector(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"AttributeSelector",
                            false,
                        )?;
                        match self.handle_attribute_selector(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (s @ S::Done__(_), event) => {
                        *self.state__ = s;
                        break (DeserializerEvent::Continue(event), false);
                    }
                    (S::Unknown__, _) => unreachable!(),
                }
            };
            let artifact = if matches!(&*self.state__, S::Done__(_)) {
                DeserializerArtifact::Data(self.finish(reader)?)
            } else {
                DeserializerArtifact::Deserializer(self)
            };
            Ok(DeserializerOutput {
                artifact,
                event,
                allow_any,
            })
        }
        fn finish<R>(self, reader: &R) -> std::result::Result<super::MatchContent47Type, Error>
        where
            R: DeserializeReader,
        {
            Self::finish_state(reader, *self.state__)
        }
    }
    #[derive(Debug)]
    pub struct PolicySetContent31TypeDeserializer {
        state__: Box<PolicySetContent31TypeDeserializerState>,
    }
    #[derive(Debug)]
    pub enum PolicySetContent31TypeDeserializerState {
        Init__,
        PolicySet(
            Option<super::PolicySet>,
            Option<<super::PolicySet as WithDeserializer>::Deserializer>,
        ),
        Policy(
            Option<super::Policy>,
            Option<<super::Policy as WithDeserializer>::Deserializer>,
        ),
        PolicySetIdReference(
            Option<super::PolicySetIdReference>,
            Option<<super::PolicySetIdReference as WithDeserializer>::Deserializer>,
        ),
        PolicyIdReference(
            Option<super::PolicyIdReference>,
            Option<<super::PolicyIdReference as WithDeserializer>::Deserializer>,
        ),
        CombinerParameters(
            Option<super::CombinerParameters>,
            Option<<super::CombinerParameters as WithDeserializer>::Deserializer>,
        ),
        PolicyCombinerParameters(
            Option<super::PolicyCombinerParameters>,
            Option<<super::PolicyCombinerParameters as WithDeserializer>::Deserializer>,
        ),
        PolicySetCombinerParameters(
            Option<super::PolicySetCombinerParameters>,
            Option<<super::PolicySetCombinerParameters as WithDeserializer>::Deserializer>,
        ),
        Done__(super::PolicySetContent31Type),
        Unknown__,
    }
    impl PolicySetContent31TypeDeserializer {
        fn find_suitable<'de, R>(
            &mut self,
            reader: &R,
            event: Event<'de>,
            fallback: &mut Option<PolicySetContent31TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            if let Event::Start(x) | Event::Empty(x) = &event {
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"PolicySet")
                ) {
                    let output =
                        <super::PolicySet as WithDeserializer>::Deserializer::init(reader, event)?;
                    return self.handle_policy_set(
                        reader,
                        Default::default(),
                        output,
                        &mut *fallback,
                    );
                }
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"Policy")
                ) {
                    let output =
                        <super::Policy as WithDeserializer>::Deserializer::init(reader, event)?;
                    return self.handle_policy(reader, Default::default(), output, &mut *fallback);
                }
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"PolicySetIdReference")
                ) {
                    let output =
                        <super::PolicySetIdReference as WithDeserializer>::Deserializer::init(
                            reader, event,
                        )?;
                    return self.handle_policy_set_id_reference(
                        reader,
                        Default::default(),
                        output,
                        &mut *fallback,
                    );
                }
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"PolicyIdReference")
                ) {
                    let output =
                        <super::PolicyIdReference as WithDeserializer>::Deserializer::init(
                            reader, event,
                        )?;
                    return self.handle_policy_id_reference(
                        reader,
                        Default::default(),
                        output,
                        &mut *fallback,
                    );
                }
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"CombinerParameters")
                ) {
                    let output =
                        <super::CombinerParameters as WithDeserializer>::Deserializer::init(
                            reader, event,
                        )?;
                    return self.handle_combiner_parameters(
                        reader,
                        Default::default(),
                        output,
                        &mut *fallback,
                    );
                }
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"PolicyCombinerParameters")
                ) {
                    let output =
                        <super::PolicyCombinerParameters as WithDeserializer>::Deserializer::init(
                            reader, event,
                        )?;
                    return self.handle_policy_combiner_parameters(
                        reader,
                        Default::default(),
                        output,
                        &mut *fallback,
                    );
                }
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"PolicySetCombinerParameters")
                ) {
                    let output = < super :: PolicySetCombinerParameters as WithDeserializer > :: Deserializer :: init (reader , event) ? ;
                    return self.handle_policy_set_combiner_parameters(
                        reader,
                        Default::default(),
                        output,
                        &mut *fallback,
                    );
                }
            }
            *self.state__ = fallback
                .take()
                .unwrap_or(PolicySetContent31TypeDeserializerState::Init__);
            Ok(ElementHandlerOutput::return_to_parent(event, false))
        }
        fn finish_state<R>(
            reader: &R,
            state: PolicySetContent31TypeDeserializerState,
        ) -> std::result::Result<super::PolicySetContent31Type, Error>
        where
            R: DeserializeReader,
        {
            use PolicySetContent31TypeDeserializerState as S;
            match state {
                S::Init__ => Err(ErrorKind::MissingContent.into()),
                S::PolicySet(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_policy_set(&mut values, value)?;
                    }
                    Ok(super::PolicySetContent31Type::PolicySet(
                        values.ok_or_else(|| ErrorKind::MissingElement("PolicySet".into()))?,
                    ))
                }
                S::Policy(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_policy(&mut values, value)?;
                    }
                    Ok(super::PolicySetContent31Type::Policy(values.ok_or_else(
                        || ErrorKind::MissingElement("Policy".into()),
                    )?))
                }
                S::PolicySetIdReference(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_policy_set_id_reference(&mut values, value)?;
                    }
                    Ok(super::PolicySetContent31Type::PolicySetIdReference(
                        values.ok_or_else(|| {
                            ErrorKind::MissingElement("PolicySetIdReference".into())
                        })?,
                    ))
                }
                S::PolicyIdReference(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_policy_id_reference(&mut values, value)?;
                    }
                    Ok(super::PolicySetContent31Type::PolicyIdReference(
                        values
                            .ok_or_else(|| ErrorKind::MissingElement("PolicyIdReference".into()))?,
                    ))
                }
                S::CombinerParameters(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_combiner_parameters(&mut values, value)?;
                    }
                    Ok(super::PolicySetContent31Type::CombinerParameters(
                        values.ok_or_else(|| {
                            ErrorKind::MissingElement("CombinerParameters".into())
                        })?,
                    ))
                }
                S::PolicyCombinerParameters(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_policy_combiner_parameters(&mut values, value)?;
                    }
                    Ok(super::PolicySetContent31Type::PolicyCombinerParameters(
                        values.ok_or_else(|| {
                            ErrorKind::MissingElement("PolicyCombinerParameters".into())
                        })?,
                    ))
                }
                S::PolicySetCombinerParameters(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_policy_set_combiner_parameters(&mut values, value)?;
                    }
                    Ok(super::PolicySetContent31Type::PolicySetCombinerParameters(
                        values.ok_or_else(|| {
                            ErrorKind::MissingElement("PolicySetCombinerParameters".into())
                        })?,
                    ))
                }
                S::Done__(data) => Ok(data),
                S::Unknown__ => unreachable!(),
            }
        }
        fn store_policy_set(
            values: &mut Option<super::PolicySet>,
            value: super::PolicySet,
        ) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"PolicySet",
                )))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn store_policy(
            values: &mut Option<super::Policy>,
            value: super::Policy,
        ) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"Policy",
                )))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn store_policy_set_id_reference(
            values: &mut Option<super::PolicySetIdReference>,
            value: super::PolicySetIdReference,
        ) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"PolicySetIdReference",
                )))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn store_policy_id_reference(
            values: &mut Option<super::PolicyIdReference>,
            value: super::PolicyIdReference,
        ) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"PolicyIdReference",
                )))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn store_combiner_parameters(
            values: &mut Option<super::CombinerParameters>,
            value: super::CombinerParameters,
        ) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"CombinerParameters",
                )))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn store_policy_combiner_parameters(
            values: &mut Option<super::PolicyCombinerParameters>,
            value: super::PolicyCombinerParameters,
        ) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"PolicyCombinerParameters",
                )))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn store_policy_set_combiner_parameters(
            values: &mut Option<super::PolicySetCombinerParameters>,
            value: super::PolicySetCombinerParameters,
        ) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"PolicySetCombinerParameters",
                )))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn handle_policy_set<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::PolicySet>,
            output: DeserializerOutput<'de, super::PolicySet>,
            fallback: &mut Option<PolicySetContent31TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None if values.is_none() => {
                        *self.state__ = PolicySetContent31TypeDeserializerState::Init__;
                        return Ok(ElementHandlerOutput::from_event(event, allow_any));
                    }
                    None => PolicySetContent31TypeDeserializerState::PolicySet(values, None),
                    Some(PolicySetContent31TypeDeserializerState::PolicySet(
                        _,
                        Some(deserializer),
                    )) => PolicySetContent31TypeDeserializerState::PolicySet(
                        values,
                        Some(deserializer),
                    ),
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(PolicySetContent31TypeDeserializerState::PolicySet(_, Some(deserializer))) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_policy_set(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_policy_set(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        PolicySetContent31TypeDeserializerState::PolicySet(values, None),
                    )?;
                    *self.state__ = PolicySetContent31TypeDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ = PolicySetContent31TypeDeserializerState::PolicySet(
                        values,
                        Some(deserializer),
                    );
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
        fn handle_policy<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::Policy>,
            output: DeserializerOutput<'de, super::Policy>,
            fallback: &mut Option<PolicySetContent31TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None if values.is_none() => {
                        *self.state__ = PolicySetContent31TypeDeserializerState::Init__;
                        return Ok(ElementHandlerOutput::from_event(event, allow_any));
                    }
                    None => PolicySetContent31TypeDeserializerState::Policy(values, None),
                    Some(PolicySetContent31TypeDeserializerState::Policy(
                        _,
                        Some(deserializer),
                    )) => {
                        PolicySetContent31TypeDeserializerState::Policy(values, Some(deserializer))
                    }
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(PolicySetContent31TypeDeserializerState::Policy(_, Some(deserializer))) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_policy(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_policy(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        PolicySetContent31TypeDeserializerState::Policy(values, None),
                    )?;
                    *self.state__ = PolicySetContent31TypeDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ =
                        PolicySetContent31TypeDeserializerState::Policy(values, Some(deserializer));
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
        fn handle_policy_set_id_reference<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::PolicySetIdReference>,
            output: DeserializerOutput<'de, super::PolicySetIdReference>,
            fallback: &mut Option<PolicySetContent31TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None if values.is_none() => {
                        *self.state__ = PolicySetContent31TypeDeserializerState::Init__;
                        return Ok(ElementHandlerOutput::from_event(event, allow_any));
                    }
                    None => {
                        PolicySetContent31TypeDeserializerState::PolicySetIdReference(values, None)
                    }
                    Some(PolicySetContent31TypeDeserializerState::PolicySetIdReference(
                        _,
                        Some(deserializer),
                    )) => PolicySetContent31TypeDeserializerState::PolicySetIdReference(
                        values,
                        Some(deserializer),
                    ),
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(PolicySetContent31TypeDeserializerState::PolicySetIdReference(
                    _,
                    Some(deserializer),
                )) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_policy_set_id_reference(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_policy_set_id_reference(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        PolicySetContent31TypeDeserializerState::PolicySetIdReference(values, None),
                    )?;
                    *self.state__ = PolicySetContent31TypeDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ = PolicySetContent31TypeDeserializerState::PolicySetIdReference(
                        values,
                        Some(deserializer),
                    );
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
        fn handle_policy_id_reference<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::PolicyIdReference>,
            output: DeserializerOutput<'de, super::PolicyIdReference>,
            fallback: &mut Option<PolicySetContent31TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None if values.is_none() => {
                        *self.state__ = PolicySetContent31TypeDeserializerState::Init__;
                        return Ok(ElementHandlerOutput::from_event(event, allow_any));
                    }
                    None => {
                        PolicySetContent31TypeDeserializerState::PolicyIdReference(values, None)
                    }
                    Some(PolicySetContent31TypeDeserializerState::PolicyIdReference(
                        _,
                        Some(deserializer),
                    )) => PolicySetContent31TypeDeserializerState::PolicyIdReference(
                        values,
                        Some(deserializer),
                    ),
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(PolicySetContent31TypeDeserializerState::PolicyIdReference(
                    _,
                    Some(deserializer),
                )) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_policy_id_reference(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_policy_id_reference(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        PolicySetContent31TypeDeserializerState::PolicyIdReference(values, None),
                    )?;
                    *self.state__ = PolicySetContent31TypeDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ = PolicySetContent31TypeDeserializerState::PolicyIdReference(
                        values,
                        Some(deserializer),
                    );
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
        fn handle_combiner_parameters<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::CombinerParameters>,
            output: DeserializerOutput<'de, super::CombinerParameters>,
            fallback: &mut Option<PolicySetContent31TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None if values.is_none() => {
                        *self.state__ = PolicySetContent31TypeDeserializerState::Init__;
                        return Ok(ElementHandlerOutput::from_event(event, allow_any));
                    }
                    None => {
                        PolicySetContent31TypeDeserializerState::CombinerParameters(values, None)
                    }
                    Some(PolicySetContent31TypeDeserializerState::CombinerParameters(
                        _,
                        Some(deserializer),
                    )) => PolicySetContent31TypeDeserializerState::CombinerParameters(
                        values,
                        Some(deserializer),
                    ),
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(PolicySetContent31TypeDeserializerState::CombinerParameters(
                    _,
                    Some(deserializer),
                )) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_combiner_parameters(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_combiner_parameters(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        PolicySetContent31TypeDeserializerState::CombinerParameters(values, None),
                    )?;
                    *self.state__ = PolicySetContent31TypeDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ = PolicySetContent31TypeDeserializerState::CombinerParameters(
                        values,
                        Some(deserializer),
                    );
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
        fn handle_policy_combiner_parameters<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::PolicyCombinerParameters>,
            output: DeserializerOutput<'de, super::PolicyCombinerParameters>,
            fallback: &mut Option<PolicySetContent31TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None if values.is_none() => {
                        *self.state__ = PolicySetContent31TypeDeserializerState::Init__;
                        return Ok(ElementHandlerOutput::from_event(event, allow_any));
                    }
                    None => PolicySetContent31TypeDeserializerState::PolicyCombinerParameters(
                        values, None,
                    ),
                    Some(PolicySetContent31TypeDeserializerState::PolicyCombinerParameters(
                        _,
                        Some(deserializer),
                    )) => PolicySetContent31TypeDeserializerState::PolicyCombinerParameters(
                        values,
                        Some(deserializer),
                    ),
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(PolicySetContent31TypeDeserializerState::PolicyCombinerParameters(
                    _,
                    Some(deserializer),
                )) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_policy_combiner_parameters(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_policy_combiner_parameters(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        PolicySetContent31TypeDeserializerState::PolicyCombinerParameters(
                            values, None,
                        ),
                    )?;
                    *self.state__ = PolicySetContent31TypeDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ =
                        PolicySetContent31TypeDeserializerState::PolicyCombinerParameters(
                            values,
                            Some(deserializer),
                        );
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
        fn handle_policy_set_combiner_parameters<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::PolicySetCombinerParameters>,
            output: DeserializerOutput<'de, super::PolicySetCombinerParameters>,
            fallback: &mut Option<PolicySetContent31TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None if values.is_none() => {
                        *self.state__ = PolicySetContent31TypeDeserializerState::Init__;
                        return Ok(ElementHandlerOutput::from_event(event, allow_any));
                    }
                    None => PolicySetContent31TypeDeserializerState::PolicySetCombinerParameters(
                        values, None,
                    ),
                    Some(PolicySetContent31TypeDeserializerState::PolicySetCombinerParameters(
                        _,
                        Some(deserializer),
                    )) => PolicySetContent31TypeDeserializerState::PolicySetCombinerParameters(
                        values,
                        Some(deserializer),
                    ),
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(PolicySetContent31TypeDeserializerState::PolicySetCombinerParameters(
                    _,
                    Some(deserializer),
                )) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_policy_set_combiner_parameters(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_policy_set_combiner_parameters(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        PolicySetContent31TypeDeserializerState::PolicySetCombinerParameters(
                            values, None,
                        ),
                    )?;
                    *self.state__ = PolicySetContent31TypeDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ =
                        PolicySetContent31TypeDeserializerState::PolicySetCombinerParameters(
                            values,
                            Some(deserializer),
                        );
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::PolicySetContent31Type> for PolicySetContent31TypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::PolicySetContent31Type>
        where
            R: DeserializeReader,
        {
            let deserializer = Self {
                state__: Box::new(PolicySetContent31TypeDeserializerState::Init__),
            };
            let mut output = deserializer.next(reader, event)?;
            output.artifact = match output.artifact {
                DeserializerArtifact::Deserializer(x)
                    if matches!(&*x.state__, PolicySetContent31TypeDeserializerState::Init__) =>
                {
                    DeserializerArtifact::None
                }
                artifact => artifact,
            };
            Ok(output)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::PolicySetContent31Type>
        where
            R: DeserializeReader,
        {
            use PolicySetContent31TypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::PolicySet(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_policy_set(reader, values, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::Policy(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_policy(reader, values, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::PolicySetIdReference(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_policy_set_id_reference(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::PolicyIdReference(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_policy_id_reference(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::CombinerParameters(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_combiner_parameters(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::PolicyCombinerParameters(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_policy_combiner_parameters(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::PolicySetCombinerParameters(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_policy_set_combiner_parameters(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (state, event @ Event::End(_)) => {
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(Self::finish_state(
                                reader, state,
                            )?),
                            event: DeserializerEvent::Continue(event),
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => match self.find_suitable(reader, event, &mut fallback)? {
                        ElementHandlerOutput::Break { event, allow_any } => {
                            break (event, allow_any)
                        }
                        ElementHandlerOutput::Continue { event, .. } => event,
                    },
                    (S::PolicySet(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"PolicySet",
                            false,
                        )?;
                        match self.handle_policy_set(reader, values, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::Policy(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Policy",
                            false,
                        )?;
                        match self.handle_policy(reader, values, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::PolicySetIdReference(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"PolicySetIdReference",
                            false,
                        )?;
                        match self.handle_policy_set_id_reference(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::PolicyIdReference(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"PolicyIdReference",
                            false,
                        )?;
                        match self.handle_policy_id_reference(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::CombinerParameters(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"CombinerParameters",
                            false,
                        )?;
                        match self.handle_combiner_parameters(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::PolicyCombinerParameters(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"PolicyCombinerParameters",
                            false,
                        )?;
                        match self.handle_policy_combiner_parameters(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::PolicySetCombinerParameters(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"PolicySetCombinerParameters",
                            false,
                        )?;
                        match self.handle_policy_set_combiner_parameters(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (s @ S::Done__(_), event) => {
                        *self.state__ = s;
                        break (DeserializerEvent::Continue(event), false);
                    }
                    (S::Unknown__, _) => unreachable!(),
                }
            };
            let artifact = if matches!(&*self.state__, S::Done__(_)) {
                DeserializerArtifact::Data(self.finish(reader)?)
            } else {
                DeserializerArtifact::Deserializer(self)
            };
            Ok(DeserializerOutput {
                artifact,
                event,
                allow_any,
            })
        }
        fn finish<R>(self, reader: &R) -> std::result::Result<super::PolicySetContent31Type, Error>
        where
            R: DeserializeReader,
        {
            Self::finish_state(reader, *self.state__)
        }
    }
    #[derive(Debug)]
    pub struct PolicyContent41TypeDeserializer {
        state__: Box<PolicyContent41TypeDeserializerState>,
    }
    #[derive(Debug)]
    pub enum PolicyContent41TypeDeserializerState {
        Init__,
        CombinerParameters(
            Option<super::CombinerParameters>,
            Option<<super::CombinerParameters as WithDeserializer>::Deserializer>,
        ),
        RuleCombinerParameters(
            Option<super::RuleCombinerParameters>,
            Option<<super::RuleCombinerParameters as WithDeserializer>::Deserializer>,
        ),
        VariableDefinition(
            Option<super::VariableDefinition>,
            Option<<super::VariableDefinition as WithDeserializer>::Deserializer>,
        ),
        Rule(
            Option<super::Rule>,
            Option<<super::Rule as WithDeserializer>::Deserializer>,
        ),
        Done__(super::PolicyContent41Type),
        Unknown__,
    }
    impl PolicyContent41TypeDeserializer {
        fn find_suitable<'de, R>(
            &mut self,
            reader: &R,
            event: Event<'de>,
            fallback: &mut Option<PolicyContent41TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            if let Event::Start(x) | Event::Empty(x) = &event {
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"CombinerParameters")
                ) {
                    let output =
                        <super::CombinerParameters as WithDeserializer>::Deserializer::init(
                            reader, event,
                        )?;
                    return self.handle_combiner_parameters(
                        reader,
                        Default::default(),
                        output,
                        &mut *fallback,
                    );
                }
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"RuleCombinerParameters")
                ) {
                    let output =
                        <super::RuleCombinerParameters as WithDeserializer>::Deserializer::init(
                            reader, event,
                        )?;
                    return self.handle_rule_combiner_parameters(
                        reader,
                        Default::default(),
                        output,
                        &mut *fallback,
                    );
                }
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"VariableDefinition")
                ) {
                    let output =
                        <super::VariableDefinition as WithDeserializer>::Deserializer::init(
                            reader, event,
                        )?;
                    return self.handle_variable_definition(
                        reader,
                        Default::default(),
                        output,
                        &mut *fallback,
                    );
                }
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"Rule")
                ) {
                    let output =
                        <super::Rule as WithDeserializer>::Deserializer::init(reader, event)?;
                    return self.handle_rule(reader, Default::default(), output, &mut *fallback);
                }
            }
            *self.state__ = fallback
                .take()
                .unwrap_or(PolicyContent41TypeDeserializerState::Init__);
            Ok(ElementHandlerOutput::return_to_parent(event, false))
        }
        fn finish_state<R>(
            reader: &R,
            state: PolicyContent41TypeDeserializerState,
        ) -> std::result::Result<super::PolicyContent41Type, Error>
        where
            R: DeserializeReader,
        {
            use PolicyContent41TypeDeserializerState as S;
            match state {
                S::Init__ => Err(ErrorKind::MissingContent.into()),
                S::CombinerParameters(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_combiner_parameters(&mut values, value)?;
                    }
                    Ok(super::PolicyContent41Type::CombinerParameters(values))
                }
                S::RuleCombinerParameters(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_rule_combiner_parameters(&mut values, value)?;
                    }
                    Ok(super::PolicyContent41Type::RuleCombinerParameters(values))
                }
                S::VariableDefinition(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_variable_definition(&mut values, value)?;
                    }
                    Ok(super::PolicyContent41Type::VariableDefinition(
                        values.ok_or_else(|| {
                            ErrorKind::MissingElement("VariableDefinition".into())
                        })?,
                    ))
                }
                S::Rule(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_rule(&mut values, value)?;
                    }
                    Ok(super::PolicyContent41Type::Rule(
                        values.ok_or_else(|| ErrorKind::MissingElement("Rule".into()))?,
                    ))
                }
                S::Done__(data) => Ok(data),
                S::Unknown__ => unreachable!(),
            }
        }
        fn store_combiner_parameters(
            values: &mut Option<super::CombinerParameters>,
            value: super::CombinerParameters,
        ) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"CombinerParameters",
                )))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn store_rule_combiner_parameters(
            values: &mut Option<super::RuleCombinerParameters>,
            value: super::RuleCombinerParameters,
        ) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"RuleCombinerParameters",
                )))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn store_variable_definition(
            values: &mut Option<super::VariableDefinition>,
            value: super::VariableDefinition,
        ) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"VariableDefinition",
                )))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn store_rule(values: &mut Option<super::Rule>, value: super::Rule) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(b"Rule")))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn handle_combiner_parameters<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::CombinerParameters>,
            output: DeserializerOutput<'de, super::CombinerParameters>,
            fallback: &mut Option<PolicyContent41TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None => PolicyContent41TypeDeserializerState::CombinerParameters(values, None),
                    Some(PolicyContent41TypeDeserializerState::CombinerParameters(
                        _,
                        Some(deserializer),
                    )) => PolicyContent41TypeDeserializerState::CombinerParameters(
                        values,
                        Some(deserializer),
                    ),
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(PolicyContent41TypeDeserializerState::CombinerParameters(
                    _,
                    Some(deserializer),
                )) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_combiner_parameters(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_combiner_parameters(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        PolicyContent41TypeDeserializerState::CombinerParameters(values, None),
                    )?;
                    *self.state__ = PolicyContent41TypeDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ = PolicyContent41TypeDeserializerState::CombinerParameters(
                        values,
                        Some(deserializer),
                    );
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
        fn handle_rule_combiner_parameters<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::RuleCombinerParameters>,
            output: DeserializerOutput<'de, super::RuleCombinerParameters>,
            fallback: &mut Option<PolicyContent41TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None => {
                        PolicyContent41TypeDeserializerState::RuleCombinerParameters(values, None)
                    }
                    Some(PolicyContent41TypeDeserializerState::RuleCombinerParameters(
                        _,
                        Some(deserializer),
                    )) => PolicyContent41TypeDeserializerState::RuleCombinerParameters(
                        values,
                        Some(deserializer),
                    ),
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(PolicyContent41TypeDeserializerState::RuleCombinerParameters(
                    _,
                    Some(deserializer),
                )) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_rule_combiner_parameters(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_rule_combiner_parameters(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        PolicyContent41TypeDeserializerState::RuleCombinerParameters(values, None),
                    )?;
                    *self.state__ = PolicyContent41TypeDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ = PolicyContent41TypeDeserializerState::RuleCombinerParameters(
                        values,
                        Some(deserializer),
                    );
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
        fn handle_variable_definition<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::VariableDefinition>,
            output: DeserializerOutput<'de, super::VariableDefinition>,
            fallback: &mut Option<PolicyContent41TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None if values.is_none() => {
                        *self.state__ = PolicyContent41TypeDeserializerState::Init__;
                        return Ok(ElementHandlerOutput::from_event(event, allow_any));
                    }
                    None => PolicyContent41TypeDeserializerState::VariableDefinition(values, None),
                    Some(PolicyContent41TypeDeserializerState::VariableDefinition(
                        _,
                        Some(deserializer),
                    )) => PolicyContent41TypeDeserializerState::VariableDefinition(
                        values,
                        Some(deserializer),
                    ),
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(PolicyContent41TypeDeserializerState::VariableDefinition(
                    _,
                    Some(deserializer),
                )) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_variable_definition(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_variable_definition(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        PolicyContent41TypeDeserializerState::VariableDefinition(values, None),
                    )?;
                    *self.state__ = PolicyContent41TypeDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ = PolicyContent41TypeDeserializerState::VariableDefinition(
                        values,
                        Some(deserializer),
                    );
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
        fn handle_rule<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::Rule>,
            output: DeserializerOutput<'de, super::Rule>,
            fallback: &mut Option<PolicyContent41TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None if values.is_none() => {
                        *self.state__ = PolicyContent41TypeDeserializerState::Init__;
                        return Ok(ElementHandlerOutput::from_event(event, allow_any));
                    }
                    None => PolicyContent41TypeDeserializerState::Rule(values, None),
                    Some(PolicyContent41TypeDeserializerState::Rule(_, Some(deserializer))) => {
                        PolicyContent41TypeDeserializerState::Rule(values, Some(deserializer))
                    }
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(PolicyContent41TypeDeserializerState::Rule(_, Some(deserializer))) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_rule(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_rule(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        PolicyContent41TypeDeserializerState::Rule(values, None),
                    )?;
                    *self.state__ = PolicyContent41TypeDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ =
                        PolicyContent41TypeDeserializerState::Rule(values, Some(deserializer));
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::PolicyContent41Type> for PolicyContent41TypeDeserializer {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::PolicyContent41Type>
        where
            R: DeserializeReader,
        {
            let deserializer = Self {
                state__: Box::new(PolicyContent41TypeDeserializerState::Init__),
            };
            let mut output = deserializer.next(reader, event)?;
            output.artifact = match output.artifact {
                DeserializerArtifact::Deserializer(x)
                    if matches!(&*x.state__, PolicyContent41TypeDeserializerState::Init__) =>
                {
                    DeserializerArtifact::None
                }
                artifact => artifact,
            };
            Ok(output)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::PolicyContent41Type>
        where
            R: DeserializeReader,
        {
            use PolicyContent41TypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::CombinerParameters(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_combiner_parameters(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::RuleCombinerParameters(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_rule_combiner_parameters(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::VariableDefinition(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_variable_definition(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::Rule(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_rule(reader, values, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (state, event @ Event::End(_)) => {
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(Self::finish_state(
                                reader, state,
                            )?),
                            event: DeserializerEvent::Continue(event),
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => match self.find_suitable(reader, event, &mut fallback)? {
                        ElementHandlerOutput::Break { event, allow_any } => {
                            break (event, allow_any)
                        }
                        ElementHandlerOutput::Continue { event, .. } => event,
                    },
                    (S::CombinerParameters(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"CombinerParameters",
                            false,
                        )?;
                        match self.handle_combiner_parameters(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::RuleCombinerParameters(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"RuleCombinerParameters",
                            false,
                        )?;
                        match self.handle_rule_combiner_parameters(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::VariableDefinition(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"VariableDefinition",
                            false,
                        )?;
                        match self.handle_variable_definition(
                            reader,
                            values,
                            output,
                            &mut fallback,
                        )? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (S::Rule(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"Rule",
                            false,
                        )?;
                        match self.handle_rule(reader, values, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (s @ S::Done__(_), event) => {
                        *self.state__ = s;
                        break (DeserializerEvent::Continue(event), false);
                    }
                    (S::Unknown__, _) => unreachable!(),
                }
            };
            let artifact = if matches!(&*self.state__, S::Done__(_)) {
                DeserializerArtifact::Data(self.finish(reader)?)
            } else {
                DeserializerArtifact::Deserializer(self)
            };
            Ok(DeserializerOutput {
                artifact,
                event,
                allow_any,
            })
        }
        fn finish<R>(self, reader: &R) -> std::result::Result<super::PolicyContent41Type, Error>
        where
            R: DeserializeReader,
        {
            Self::finish_state(reader, *self.state__)
        }
    }
    #[derive(Debug)]
    pub struct RequestDefaultsContent3TypeDeserializer {
        state__: Box<RequestDefaultsContent3TypeDeserializerState>,
    }
    #[derive(Debug)]
    pub enum RequestDefaultsContent3TypeDeserializerState {
        Init__,
        XPathVersion(
            Option<super::XPathVersion>,
            Option<<super::XPathVersion as WithDeserializer>::Deserializer>,
        ),
        Done__(super::RequestDefaultsContent3Type),
        Unknown__,
    }
    impl RequestDefaultsContent3TypeDeserializer {
        fn find_suitable<'de, R>(
            &mut self,
            reader: &R,
            event: Event<'de>,
            fallback: &mut Option<RequestDefaultsContent3TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            if let Event::Start(x) | Event::Empty(x) = &event {
                if matches!(
                    reader.resolve_local_name(x.name(), &super::NS_XACML),
                    Some(b"XPathVersion")
                ) {
                    let output = <super::XPathVersion as WithDeserializer>::Deserializer::init(
                        reader, event,
                    )?;
                    return self.handle_x_path_version(
                        reader,
                        Default::default(),
                        output,
                        &mut *fallback,
                    );
                }
            }
            *self.state__ = fallback
                .take()
                .unwrap_or(RequestDefaultsContent3TypeDeserializerState::Init__);
            Ok(ElementHandlerOutput::return_to_parent(event, false))
        }
        fn finish_state<R>(
            reader: &R,
            state: RequestDefaultsContent3TypeDeserializerState,
        ) -> std::result::Result<super::RequestDefaultsContent3Type, Error>
        where
            R: DeserializeReader,
        {
            use RequestDefaultsContent3TypeDeserializerState as S;
            match state {
                S::Init__ => Err(ErrorKind::MissingContent.into()),
                S::XPathVersion(mut values, deserializer) => {
                    if let Some(deserializer) = deserializer {
                        let value = deserializer.finish(reader)?;
                        Self::store_x_path_version(&mut values, value)?;
                    }
                    Ok(super::RequestDefaultsContent3Type::XPathVersion(
                        values.ok_or_else(|| ErrorKind::MissingElement("XPathVersion".into()))?,
                    ))
                }
                S::Done__(data) => Ok(data),
                S::Unknown__ => unreachable!(),
            }
        }
        fn store_x_path_version(
            values: &mut Option<super::XPathVersion>,
            value: super::XPathVersion,
        ) -> std::result::Result<(), Error> {
            if values.is_some() {
                Err(ErrorKind::DuplicateElement(RawByteStr::from_slice(
                    b"XPathVersion",
                )))?;
            }
            *values = Some(value);
            Ok(())
        }
        fn handle_x_path_version<'de, R>(
            &mut self,
            reader: &R,
            mut values: Option<super::XPathVersion>,
            output: DeserializerOutput<'de, super::XPathVersion>,
            fallback: &mut Option<RequestDefaultsContent3TypeDeserializerState>,
        ) -> std::result::Result<ElementHandlerOutput<'de>, Error>
        where
            R: DeserializeReader,
        {
            let DeserializerOutput {
                artifact,
                event,
                allow_any,
            } = output;
            if artifact.is_none() {
                *self.state__ = match fallback.take() {
                    None if values.is_none() => {
                        *self.state__ = RequestDefaultsContent3TypeDeserializerState::Init__;
                        return Ok(ElementHandlerOutput::from_event(event, allow_any));
                    }
                    None => {
                        RequestDefaultsContent3TypeDeserializerState::XPathVersion(values, None)
                    }
                    Some(RequestDefaultsContent3TypeDeserializerState::XPathVersion(
                        _,
                        Some(deserializer),
                    )) => RequestDefaultsContent3TypeDeserializerState::XPathVersion(
                        values,
                        Some(deserializer),
                    ),
                    _ => unreachable!(),
                };
                return Ok(ElementHandlerOutput::break_(event, allow_any));
            }
            match fallback.take() {
                None => (),
                Some(RequestDefaultsContent3TypeDeserializerState::XPathVersion(
                    _,
                    Some(deserializer),
                )) => {
                    let data = deserializer.finish(reader)?;
                    Self::store_x_path_version(&mut values, data)?;
                }
                Some(_) => unreachable!(),
            }
            Ok(match artifact {
                DeserializerArtifact::None => unreachable!(),
                DeserializerArtifact::Data(data) => {
                    Self::store_x_path_version(&mut values, data)?;
                    let data = Self::finish_state(
                        reader,
                        RequestDefaultsContent3TypeDeserializerState::XPathVersion(values, None),
                    )?;
                    *self.state__ = RequestDefaultsContent3TypeDeserializerState::Done__(data);
                    ElementHandlerOutput::Break { event, allow_any }
                }
                DeserializerArtifact::Deserializer(deserializer) => {
                    *self.state__ = RequestDefaultsContent3TypeDeserializerState::XPathVersion(
                        values,
                        Some(deserializer),
                    );
                    ElementHandlerOutput::from_event_end(event, allow_any)
                }
            })
        }
    }
    impl<'de> Deserializer<'de, super::RequestDefaultsContent3Type>
        for RequestDefaultsContent3TypeDeserializer
    {
        fn init<R>(
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::RequestDefaultsContent3Type>
        where
            R: DeserializeReader,
        {
            let deserializer = Self {
                state__: Box::new(RequestDefaultsContent3TypeDeserializerState::Init__),
            };
            let mut output = deserializer.next(reader, event)?;
            output.artifact = match output.artifact {
                DeserializerArtifact::Deserializer(x)
                    if matches!(
                        &*x.state__,
                        RequestDefaultsContent3TypeDeserializerState::Init__
                    ) =>
                {
                    DeserializerArtifact::None
                }
                artifact => artifact,
            };
            Ok(output)
        }
        fn next<R>(
            mut self,
            reader: &R,
            event: Event<'de>,
        ) -> DeserializerResult<'de, super::RequestDefaultsContent3Type>
        where
            R: DeserializeReader,
        {
            use RequestDefaultsContent3TypeDeserializerState as S;
            let mut event = event;
            let mut fallback = None;
            let (event, allow_any) = loop {
                let state = replace(&mut *self.state__, S::Unknown__);
                event = match (state, event) {
                    (S::XPathVersion(values, Some(deserializer)), event) => {
                        let output = deserializer.next(reader, event)?;
                        match self.handle_x_path_version(reader, values, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (state, event @ Event::End(_)) => {
                        return Ok(DeserializerOutput {
                            artifact: DeserializerArtifact::Data(Self::finish_state(
                                reader, state,
                            )?),
                            event: DeserializerEvent::Continue(event),
                            allow_any: false,
                        });
                    }
                    (S::Init__, event) => match self.find_suitable(reader, event, &mut fallback)? {
                        ElementHandlerOutput::Break { event, allow_any } => {
                            break (event, allow_any)
                        }
                        ElementHandlerOutput::Continue { event, .. } => event,
                    },
                    (S::XPathVersion(values, None), event) => {
                        let output = reader.init_start_tag_deserializer(
                            event,
                            Some(&super::NS_XACML),
                            b"XPathVersion",
                            false,
                        )?;
                        match self.handle_x_path_version(reader, values, output, &mut fallback)? {
                            ElementHandlerOutput::Break { event, allow_any } => {
                                break (event, allow_any)
                            }
                            ElementHandlerOutput::Continue { event, .. } => event,
                        }
                    }
                    (s @ S::Done__(_), event) => {
                        *self.state__ = s;
                        break (DeserializerEvent::Continue(event), false);
                    }
                    (S::Unknown__, _) => unreachable!(),
                }
            };
            let artifact = if matches!(&*self.state__, S::Done__(_)) {
                DeserializerArtifact::Data(self.finish(reader)?)
            } else {
                DeserializerArtifact::Deserializer(self)
            };
            Ok(DeserializerOutput {
                artifact,
                event,
                allow_any,
            })
        }
        fn finish<R>(self, reader: &R) -> std::result::Result<super::RequestDefaultsContent3Type, Error>
        where
            R: DeserializeReader,
        {
            Self::finish_state(reader, *self.state__)
        }
    }
}
pub mod quick_xml_serialize {
    use core::iter::Iterator;
    use xsd_parser::{
        quick_xml::{
            write_attrib, write_attrib_opt, BytesEnd, BytesStart, Error, Event, IterSerializer,
            WithSerializer,
        },
        xml::Text,
    };
    #[derive(Debug)]
    pub struct AnyTypeSerializer<'ser> {
        pub(super) value: &'ser super::AnyType,
        pub(super) state: Box<AnyTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum AnyTypeSerializerState<'ser> {
        Init__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AnyTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AnyTypeSerializerState::Init__ => {
                        *self.state = AnyTypeSerializerState::Done__;
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Empty(bytes)));
                    }
                    AnyTypeSerializerState::Done__ => return Ok(None),
                    AnyTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for AnyTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AnyTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct AdviceExpressionTypeSerializer<'ser> {
        pub(super) value: &'ser super::AdviceExpressionType,
        pub(super) state: Box<AdviceExpressionTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum AdviceExpressionTypeSerializerState<'ser> {
        Init__,
        AttributeAssignmentExpression(
            IterSerializer<
                'ser,
                &'ser [super::AttributeAssignmentExpression],
                super::AttributeAssignmentExpression,
            >,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AdviceExpressionTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AdviceExpressionTypeSerializerState::Init__ => {
                        *self.state =
                            AdviceExpressionTypeSerializerState::AttributeAssignmentExpression(
                                IterSerializer::new(
                                    &self.value.attribute_assignment_expression[..],
                                    Some("xacml:AttributeAssignmentExpression"),
                                    false,
                                ),
                            );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "AdviceId", &self.value.advice_id)?;
                        write_attrib(&mut bytes, "AppliesTo", &self.value.applies_to)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    AdviceExpressionTypeSerializerState::AttributeAssignmentExpression(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = AdviceExpressionTypeSerializerState::End__,
                        }
                    }
                    AdviceExpressionTypeSerializerState::End__ => {
                        *self.state = AdviceExpressionTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    AdviceExpressionTypeSerializerState::Done__ => return Ok(None),
                    AdviceExpressionTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for AdviceExpressionTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AdviceExpressionTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct AdviceExpressionsTypeSerializer<'ser> {
        pub(super) value: &'ser super::AdviceExpressionsType,
        pub(super) state: Box<AdviceExpressionsTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum AdviceExpressionsTypeSerializerState<'ser> {
        Init__,
        AdviceExpression(
            IterSerializer<'ser, &'ser [super::AdviceExpression], super::AdviceExpression>,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AdviceExpressionsTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AdviceExpressionsTypeSerializerState::Init__ => {
                        *self.state = AdviceExpressionsTypeSerializerState::AdviceExpression(
                            IterSerializer::new(
                                &self.value.advice_expression[..],
                                Some("xacml:AdviceExpression"),
                                false,
                            ),
                        );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    AdviceExpressionsTypeSerializerState::AdviceExpression(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = AdviceExpressionsTypeSerializerState::End__,
                        }
                    }
                    AdviceExpressionsTypeSerializerState::End__ => {
                        *self.state = AdviceExpressionsTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    AdviceExpressionsTypeSerializerState::Done__ => return Ok(None),
                    AdviceExpressionsTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for AdviceExpressionsTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AdviceExpressionsTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct AdviceTypeSerializer<'ser> {
        pub(super) value: &'ser super::AdviceType,
        pub(super) state: Box<AdviceTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum AdviceTypeSerializerState<'ser> {
        Init__,
        AttributeAssignment(
            IterSerializer<'ser, &'ser [super::AttributeAssignment], super::AttributeAssignment>,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AdviceTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AdviceTypeSerializerState::Init__ => {
                        *self.state =
                            AdviceTypeSerializerState::AttributeAssignment(IterSerializer::new(
                                &self.value.attribute_assignment[..],
                                Some("xacml:AttributeAssignment"),
                                false,
                            ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "AdviceId", &self.value.advice_id)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    AdviceTypeSerializerState::AttributeAssignment(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = AdviceTypeSerializerState::End__,
                        }
                    }
                    AdviceTypeSerializerState::End__ => {
                        *self.state = AdviceTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    AdviceTypeSerializerState::Done__ => return Ok(None),
                    AdviceTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for AdviceTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AdviceTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct AllOfTypeSerializer<'ser> {
        pub(super) value: &'ser super::AllOfType,
        pub(super) state: Box<AllOfTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum AllOfTypeSerializerState<'ser> {
        Init__,
        Content__(IterSerializer<'ser, &'ser [super::AllOfTypeContent], super::AllOfTypeContent>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AllOfTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AllOfTypeSerializerState::Init__ => {
                        *self.state = AllOfTypeSerializerState::Content__(IterSerializer::new(
                            &self.value.content[..],
                            None,
                            false,
                        ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    AllOfTypeSerializerState::Content__(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = AllOfTypeSerializerState::End__,
                    },
                    AllOfTypeSerializerState::End__ => {
                        *self.state = AllOfTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    AllOfTypeSerializerState::Done__ => return Ok(None),
                    AllOfTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for AllOfTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AllOfTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct AllOfTypeContentSerializer<'ser> {
        pub(super) value: &'ser super::AllOfTypeContent,
        pub(super) state: Box<AllOfTypeContentSerializerState<'ser>>,
    }
    #[derive(Debug)]
    pub(super) enum AllOfTypeContentSerializerState<'ser> {
        Init__,
        Match(<super::Match as WithSerializer>::Serializer<'ser>),
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AllOfTypeContentSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AllOfTypeContentSerializerState::Init__ => {
                        *self.state =
                            AllOfTypeContentSerializerState::Match(WithSerializer::serializer(
                                &self.value.match_,
                                Some("xacml:Match"),
                                false,
                            )?);
                    }
                    AllOfTypeContentSerializerState::Match(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = AllOfTypeContentSerializerState::Done__,
                    },
                    AllOfTypeContentSerializerState::Done__ => return Ok(None),
                    AllOfTypeContentSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for AllOfTypeContentSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AllOfTypeContentSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct AnyOfTypeSerializer<'ser> {
        pub(super) value: &'ser super::AnyOfType,
        pub(super) state: Box<AnyOfTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum AnyOfTypeSerializerState<'ser> {
        Init__,
        Content__(IterSerializer<'ser, &'ser [super::AnyOfTypeContent], super::AnyOfTypeContent>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AnyOfTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AnyOfTypeSerializerState::Init__ => {
                        *self.state = AnyOfTypeSerializerState::Content__(IterSerializer::new(
                            &self.value.content[..],
                            None,
                            false,
                        ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    AnyOfTypeSerializerState::Content__(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = AnyOfTypeSerializerState::End__,
                    },
                    AnyOfTypeSerializerState::End__ => {
                        *self.state = AnyOfTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    AnyOfTypeSerializerState::Done__ => return Ok(None),
                    AnyOfTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for AnyOfTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AnyOfTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct AnyOfTypeContentSerializer<'ser> {
        pub(super) value: &'ser super::AnyOfTypeContent,
        pub(super) state: Box<AnyOfTypeContentSerializerState<'ser>>,
    }
    #[derive(Debug)]
    pub(super) enum AnyOfTypeContentSerializerState<'ser> {
        Init__,
        AllOf(<super::AllOf as WithSerializer>::Serializer<'ser>),
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AnyOfTypeContentSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AnyOfTypeContentSerializerState::Init__ => {
                        *self.state =
                            AnyOfTypeContentSerializerState::AllOf(WithSerializer::serializer(
                                &self.value.all_of,
                                Some("xacml:AllOf"),
                                false,
                            )?);
                    }
                    AnyOfTypeContentSerializerState::AllOf(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = AnyOfTypeContentSerializerState::Done__,
                    },
                    AnyOfTypeContentSerializerState::Done__ => return Ok(None),
                    AnyOfTypeContentSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for AnyOfTypeContentSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AnyOfTypeContentSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct ApplyTypeSerializer<'ser> {
        pub(super) value: &'ser super::ApplyType,
        pub(super) state: Box<ApplyTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum ApplyTypeSerializerState<'ser> {
        Init__,
        Description(IterSerializer<'ser, Option<&'ser super::Description>, super::Description>),
        Expression(IterSerializer<'ser, &'ser [super::Expression], super::Expression>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> ApplyTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    ApplyTypeSerializerState::Init__ => {
                        *self.state = ApplyTypeSerializerState::Description(IterSerializer::new(
                            self.value.description.as_ref(),
                            Some("xacml:Description"),
                            false,
                        ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "FunctionId", &self.value.function_id)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    ApplyTypeSerializerState::Description(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state = ApplyTypeSerializerState::Expression(IterSerializer::new(
                                &self.value.expression[..],
                                Some("xacml:Expression"),
                                false,
                            ))
                        }
                    },
                    ApplyTypeSerializerState::Expression(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = ApplyTypeSerializerState::End__,
                    },
                    ApplyTypeSerializerState::End__ => {
                        *self.state = ApplyTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    ApplyTypeSerializerState::Done__ => return Ok(None),
                    ApplyTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for ApplyTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = ApplyTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct AssociatedAdviceTypeSerializer<'ser> {
        pub(super) value: &'ser super::AssociatedAdviceType,
        pub(super) state: Box<AssociatedAdviceTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum AssociatedAdviceTypeSerializerState<'ser> {
        Init__,
        Advice(IterSerializer<'ser, &'ser [super::Advice], super::Advice>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AssociatedAdviceTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AssociatedAdviceTypeSerializerState::Init__ => {
                        *self.state =
                            AssociatedAdviceTypeSerializerState::Advice(IterSerializer::new(
                                &self.value.advice[..],
                                Some("xacml:Advice"),
                                false,
                            ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    AssociatedAdviceTypeSerializerState::Advice(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = AssociatedAdviceTypeSerializerState::End__,
                    },
                    AssociatedAdviceTypeSerializerState::End__ => {
                        *self.state = AssociatedAdviceTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    AssociatedAdviceTypeSerializerState::Done__ => return Ok(None),
                    AssociatedAdviceTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for AssociatedAdviceTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AssociatedAdviceTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct AttributeAssignmentExpressionTypeSerializer<'ser> {
        pub(super) value: &'ser super::AttributeAssignmentExpressionType,
        pub(super) state: Box<AttributeAssignmentExpressionTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum AttributeAssignmentExpressionTypeSerializerState<'ser> {
        Init__,
        Expression(<super::Expression as WithSerializer>::Serializer<'ser>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AttributeAssignmentExpressionTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AttributeAssignmentExpressionTypeSerializerState::Init__ => {
                        *self.state = AttributeAssignmentExpressionTypeSerializerState::Expression(
                            WithSerializer::serializer(
                                &self.value.expression,
                                Some("xacml:Expression"),
                                false,
                            )?,
                        );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "AttributeId", &self.value.attribute_id)?;
                        write_attrib_opt(&mut bytes, "Category", &self.value.category)?;
                        write_attrib_opt(&mut bytes, "Issuer", &self.value.issuer)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    AttributeAssignmentExpressionTypeSerializerState::Expression(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => {
                                *self.state =
                                    AttributeAssignmentExpressionTypeSerializerState::End__
                            }
                        }
                    }
                    AttributeAssignmentExpressionTypeSerializerState::End__ => {
                        *self.state = AttributeAssignmentExpressionTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    AttributeAssignmentExpressionTypeSerializerState::Done__ => return Ok(None),
                    AttributeAssignmentExpressionTypeSerializerState::Phantom__(_) => {
                        unreachable!()
                    }
                }
            }
        }
    }
    impl<'ser> Iterator for AttributeAssignmentExpressionTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AttributeAssignmentExpressionTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct AttributeAssignmentTypeSerializer<'ser> {
        pub(super) value: &'ser super::AttributeAssignmentType,
        pub(super) state: Box<AttributeAssignmentTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum AttributeAssignmentTypeSerializerState<'ser> {
        Init__,
        Text(IterSerializer<'ser, Option<&'ser Text>, Text>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AttributeAssignmentTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AttributeAssignmentTypeSerializerState::Init__ => {
                        *self.state = AttributeAssignmentTypeSerializerState::Text(
                            IterSerializer::new(self.value.text.as_ref(), Some(""), false),
                        );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "DataType", &self.value.data_type)?;
                        write_attrib(&mut bytes, "AttributeId", &self.value.attribute_id)?;
                        write_attrib_opt(&mut bytes, "Category", &self.value.category)?;
                        write_attrib_opt(&mut bytes, "Issuer", &self.value.issuer)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    AttributeAssignmentTypeSerializerState::Text(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = AttributeAssignmentTypeSerializerState::End__,
                        }
                    }
                    AttributeAssignmentTypeSerializerState::End__ => {
                        *self.state = AttributeAssignmentTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    AttributeAssignmentTypeSerializerState::Done__ => return Ok(None),
                    AttributeAssignmentTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for AttributeAssignmentTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AttributeAssignmentTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct AttributeDesignatorTypeSerializer<'ser> {
        pub(super) value: &'ser super::AttributeDesignatorType,
        pub(super) state: Box<AttributeDesignatorTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum AttributeDesignatorTypeSerializerState<'ser> {
        Init__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AttributeDesignatorTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AttributeDesignatorTypeSerializerState::Init__ => {
                        *self.state = AttributeDesignatorTypeSerializerState::Done__;
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "Category", &self.value.category)?;
                        write_attrib(&mut bytes, "AttributeId", &self.value.attribute_id)?;
                        write_attrib(&mut bytes, "DataType", &self.value.data_type)?;
                        write_attrib_opt(&mut bytes, "Issuer", &self.value.issuer)?;
                        write_attrib(&mut bytes, "MustBePresent", &self.value.must_be_present)?;
                        return Ok(Some(Event::Empty(bytes)));
                    }
                    AttributeDesignatorTypeSerializerState::Done__ => return Ok(None),
                    AttributeDesignatorTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for AttributeDesignatorTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AttributeDesignatorTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct AttributeSelectorTypeSerializer<'ser> {
        pub(super) value: &'ser super::AttributeSelectorType,
        pub(super) state: Box<AttributeSelectorTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum AttributeSelectorTypeSerializerState<'ser> {
        Init__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AttributeSelectorTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AttributeSelectorTypeSerializerState::Init__ => {
                        *self.state = AttributeSelectorTypeSerializerState::Done__;
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "Category", &self.value.category)?;
                        write_attrib_opt(
                            &mut bytes,
                            "ContextSelectorId",
                            &self.value.context_selector_id,
                        )?;
                        write_attrib(&mut bytes, "Path", &self.value.path)?;
                        write_attrib(&mut bytes, "DataType", &self.value.data_type)?;
                        write_attrib(&mut bytes, "MustBePresent", &self.value.must_be_present)?;
                        return Ok(Some(Event::Empty(bytes)));
                    }
                    AttributeSelectorTypeSerializerState::Done__ => return Ok(None),
                    AttributeSelectorTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for AttributeSelectorTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AttributeSelectorTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct AttributeTypeSerializer<'ser> {
        pub(super) value: &'ser super::AttributeType,
        pub(super) state: Box<AttributeTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum AttributeTypeSerializerState<'ser> {
        Init__,
        AttributeValue(IterSerializer<'ser, &'ser [super::AttributeValue], super::AttributeValue>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AttributeTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AttributeTypeSerializerState::Init__ => {
                        *self.state =
                            AttributeTypeSerializerState::AttributeValue(IterSerializer::new(
                                &self.value.attribute_value[..],
                                Some("xacml:AttributeValue"),
                                false,
                            ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "AttributeId", &self.value.attribute_id)?;
                        write_attrib_opt(&mut bytes, "Issuer", &self.value.issuer)?;
                        write_attrib(&mut bytes, "IncludeInResult", &self.value.include_in_result)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    AttributeTypeSerializerState::AttributeValue(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = AttributeTypeSerializerState::End__,
                        }
                    }
                    AttributeTypeSerializerState::End__ => {
                        *self.state = AttributeTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    AttributeTypeSerializerState::Done__ => return Ok(None),
                    AttributeTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for AttributeTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AttributeTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct AttributeValueTypeSerializer<'ser> {
        pub(super) value: &'ser super::AttributeValueType,
        pub(super) state: Box<AttributeValueTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum AttributeValueTypeSerializerState<'ser> {
        Init__,
        Text(IterSerializer<'ser, Option<&'ser Text>, Text>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AttributeValueTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AttributeValueTypeSerializerState::Init__ => {
                        *self.state = AttributeValueTypeSerializerState::Text(IterSerializer::new(
                            self.value.text.as_ref(),
                            Some(""),
                            false,
                        ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "DataType", &self.value.data_type)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    AttributeValueTypeSerializerState::Text(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = AttributeValueTypeSerializerState::End__,
                    },
                    AttributeValueTypeSerializerState::End__ => {
                        *self.state = AttributeValueTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    AttributeValueTypeSerializerState::Done__ => return Ok(None),
                    AttributeValueTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for AttributeValueTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AttributeValueTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct AttributesReferenceTypeSerializer<'ser> {
        pub(super) value: &'ser super::AttributesReferenceType,
        pub(super) state: Box<AttributesReferenceTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum AttributesReferenceTypeSerializerState<'ser> {
        Init__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AttributesReferenceTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AttributesReferenceTypeSerializerState::Init__ => {
                        *self.state = AttributesReferenceTypeSerializerState::Done__;
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "ReferenceId", &self.value.reference_id)?;
                        return Ok(Some(Event::Empty(bytes)));
                    }
                    AttributesReferenceTypeSerializerState::Done__ => return Ok(None),
                    AttributesReferenceTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for AttributesReferenceTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AttributesReferenceTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct AttributesTypeSerializer<'ser> {
        pub(super) value: &'ser super::AttributesType,
        pub(super) state: Box<AttributesTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum AttributesTypeSerializerState<'ser> {
        Init__,
        Content(IterSerializer<'ser, Option<&'ser super::Content>, super::Content>),
        Attribute(IterSerializer<'ser, &'ser [super::Attribute], super::Attribute>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> AttributesTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    AttributesTypeSerializerState::Init__ => {
                        *self.state = AttributesTypeSerializerState::Content(IterSerializer::new(
                            self.value.content.as_ref(),
                            Some("xacml:Content"),
                            false,
                        ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "Category", &self.value.category)?;
                        write_attrib_opt(&mut bytes, "id", &self.value.id)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    AttributesTypeSerializerState::Content(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                AttributesTypeSerializerState::Attribute(IterSerializer::new(
                                    &self.value.attribute[..],
                                    Some("xacml:Attribute"),
                                    false,
                                ))
                        }
                    },
                    AttributesTypeSerializerState::Attribute(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = AttributesTypeSerializerState::End__,
                    },
                    AttributesTypeSerializerState::End__ => {
                        *self.state = AttributesTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    AttributesTypeSerializerState::Done__ => return Ok(None),
                    AttributesTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for AttributesTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = AttributesTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct CombinerParameterTypeSerializer<'ser> {
        pub(super) value: &'ser super::CombinerParameterType,
        pub(super) state: Box<CombinerParameterTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum CombinerParameterTypeSerializerState<'ser> {
        Init__,
        AttributeValue(<super::AttributeValue as WithSerializer>::Serializer<'ser>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> CombinerParameterTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    CombinerParameterTypeSerializerState::Init__ => {
                        *self.state = CombinerParameterTypeSerializerState::AttributeValue(
                            WithSerializer::serializer(
                                &self.value.attribute_value,
                                Some("xacml:AttributeValue"),
                                false,
                            )?,
                        );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "ParameterName", &self.value.parameter_name)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    CombinerParameterTypeSerializerState::AttributeValue(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = CombinerParameterTypeSerializerState::End__,
                        }
                    }
                    CombinerParameterTypeSerializerState::End__ => {
                        *self.state = CombinerParameterTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    CombinerParameterTypeSerializerState::Done__ => return Ok(None),
                    CombinerParameterTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for CombinerParameterTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = CombinerParameterTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct CombinerParametersTypeSerializer<'ser> {
        pub(super) value: &'ser super::CombinerParametersType,
        pub(super) state: Box<CombinerParametersTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum CombinerParametersTypeSerializerState<'ser> {
        Init__,
        CombinerParameter(
            IterSerializer<'ser, &'ser [super::CombinerParameter], super::CombinerParameter>,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> CombinerParametersTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    CombinerParametersTypeSerializerState::Init__ => {
                        *self.state = CombinerParametersTypeSerializerState::CombinerParameter(
                            IterSerializer::new(
                                &self.value.combiner_parameter[..],
                                Some("xacml:CombinerParameter"),
                                false,
                            ),
                        );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    CombinerParametersTypeSerializerState::CombinerParameter(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = CombinerParametersTypeSerializerState::End__,
                        }
                    }
                    CombinerParametersTypeSerializerState::End__ => {
                        *self.state = CombinerParametersTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    CombinerParametersTypeSerializerState::Done__ => return Ok(None),
                    CombinerParametersTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for CombinerParametersTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = CombinerParametersTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct ConditionTypeSerializer<'ser> {
        pub(super) value: &'ser super::ConditionType,
        pub(super) state: Box<ConditionTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum ConditionTypeSerializerState<'ser> {
        Init__,
        Expression(<super::Expression as WithSerializer>::Serializer<'ser>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> ConditionTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    ConditionTypeSerializerState::Init__ => {
                        *self.state =
                            ConditionTypeSerializerState::Expression(WithSerializer::serializer(
                                &self.value.expression,
                                Some("xacml:Expression"),
                                false,
                            )?);
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    ConditionTypeSerializerState::Expression(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = ConditionTypeSerializerState::End__,
                    },
                    ConditionTypeSerializerState::End__ => {
                        *self.state = ConditionTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    ConditionTypeSerializerState::Done__ => return Ok(None),
                    ConditionTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for ConditionTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = ConditionTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct ContentTypeSerializer<'ser> {
        pub(super) value: &'ser super::ContentType,
        pub(super) state: Box<ContentTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum ContentTypeSerializerState<'ser> {
        Init__,
        TextBefore(IterSerializer<'ser, Option<&'ser Text>, Text>),
        TextAfterAny6(IterSerializer<'ser, Option<&'ser Text>, Text>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> ContentTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    ContentTypeSerializerState::Init__ => {
                        *self.state = ContentTypeSerializerState::TextBefore(IterSerializer::new(
                            self.value.text_before.as_ref(),
                            Some("text_before"),
                            false,
                        ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    ContentTypeSerializerState::TextBefore(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                ContentTypeSerializerState::TextAfterAny6(IterSerializer::new(
                                    self.value.text_after_any_6.as_ref(),
                                    Some("text_after_any6"),
                                    false,
                                ))
                        }
                    },
                    ContentTypeSerializerState::TextAfterAny6(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = ContentTypeSerializerState::End__,
                    },
                    ContentTypeSerializerState::End__ => {
                        *self.state = ContentTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    ContentTypeSerializerState::Done__ => return Ok(None),
                    ContentTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for ContentTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = ContentTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct DefaultsTypeSerializer<'ser> {
        pub(super) value: &'ser super::DefaultsType,
        pub(super) state: Box<DefaultsTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum DefaultsTypeSerializerState<'ser> {
        Init__,
        Content39(<super::DefaultsContent39Type as WithSerializer>::Serializer<'ser>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> DefaultsTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    DefaultsTypeSerializerState::Init__ => {
                        *self.state =
                            DefaultsTypeSerializerState::Content39(WithSerializer::serializer(
                                &self.value.content_39,
                                Some("Content39"),
                                false,
                            )?);
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    DefaultsTypeSerializerState::Content39(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = DefaultsTypeSerializerState::End__,
                    },
                    DefaultsTypeSerializerState::End__ => {
                        *self.state = DefaultsTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    DefaultsTypeSerializerState::Done__ => return Ok(None),
                    DefaultsTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for DefaultsTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = DefaultsTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct ExpressionTypeSerializer<'ser> {
        pub(super) value: &'ser super::ExpressionType,
        pub(super) state: Box<ExpressionTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum ExpressionTypeSerializerState<'ser> {
        Init__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> ExpressionTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    ExpressionTypeSerializerState::Init__ => {
                        *self.state = ExpressionTypeSerializerState::Done__;
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Empty(bytes)));
                    }
                    ExpressionTypeSerializerState::Done__ => return Ok(None),
                    ExpressionTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for ExpressionTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = ExpressionTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct FunctionTypeSerializer<'ser> {
        pub(super) value: &'ser super::FunctionType,
        pub(super) state: Box<FunctionTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum FunctionTypeSerializerState<'ser> {
        Init__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> FunctionTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    FunctionTypeSerializerState::Init__ => {
                        *self.state = FunctionTypeSerializerState::Done__;
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "FunctionId", &self.value.function_id)?;
                        return Ok(Some(Event::Empty(bytes)));
                    }
                    FunctionTypeSerializerState::Done__ => return Ok(None),
                    FunctionTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for FunctionTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = FunctionTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct IdReferenceTypeSerializer<'ser> {
        pub(super) value: &'ser super::IdReferenceType,
        pub(super) state: Box<IdReferenceTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum IdReferenceTypeSerializerState<'ser> {
        Init__,
        Content__(<super::AnyUriType as WithSerializer>::Serializer<'ser>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> IdReferenceTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    IdReferenceTypeSerializerState::Init__ => {
                        *self.state = IdReferenceTypeSerializerState::Content__(
                            WithSerializer::serializer(&self.value.content, None, false)?,
                        );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib_opt(&mut bytes, "Version", &self.value.version)?;
                        write_attrib_opt(
                            &mut bytes,
                            "EarliestVersion",
                            &self.value.earliest_version,
                        )?;
                        write_attrib_opt(&mut bytes, "LatestVersion", &self.value.latest_version)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    IdReferenceTypeSerializerState::Content__(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = IdReferenceTypeSerializerState::End__,
                    },
                    IdReferenceTypeSerializerState::End__ => {
                        *self.state = IdReferenceTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    IdReferenceTypeSerializerState::Done__ => return Ok(None),
                    IdReferenceTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for IdReferenceTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = IdReferenceTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct MatchTypeSerializer<'ser> {
        pub(super) value: &'ser super::MatchType,
        pub(super) state: Box<MatchTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum MatchTypeSerializerState<'ser> {
        Init__,
        AttributeValue(<super::AttributeValue as WithSerializer>::Serializer<'ser>),
        Content47(<super::MatchContent47Type as WithSerializer>::Serializer<'ser>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> MatchTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    MatchTypeSerializerState::Init__ => {
                        *self.state =
                            MatchTypeSerializerState::AttributeValue(WithSerializer::serializer(
                                &self.value.attribute_value,
                                Some("xacml:AttributeValue"),
                                false,
                            )?);
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "MatchId", &self.value.match_id)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    MatchTypeSerializerState::AttributeValue(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                MatchTypeSerializerState::Content47(WithSerializer::serializer(
                                    &self.value.content_47,
                                    Some("Content47"),
                                    false,
                                )?)
                        }
                    },
                    MatchTypeSerializerState::Content47(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = MatchTypeSerializerState::End__,
                    },
                    MatchTypeSerializerState::End__ => {
                        *self.state = MatchTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    MatchTypeSerializerState::Done__ => return Ok(None),
                    MatchTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for MatchTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = MatchTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct MissingAttributeDetailTypeSerializer<'ser> {
        pub(super) value: &'ser super::MissingAttributeDetailType,
        pub(super) state: Box<MissingAttributeDetailTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum MissingAttributeDetailTypeSerializerState<'ser> {
        Init__,
        AttributeValue(IterSerializer<'ser, &'ser [super::AttributeValue], super::AttributeValue>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> MissingAttributeDetailTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    MissingAttributeDetailTypeSerializerState::Init__ => {
                        *self.state = MissingAttributeDetailTypeSerializerState::AttributeValue(
                            IterSerializer::new(
                                &self.value.attribute_value[..],
                                Some("xacml:AttributeValue"),
                                false,
                            ),
                        );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "Category", &self.value.category)?;
                        write_attrib(&mut bytes, "AttributeId", &self.value.attribute_id)?;
                        write_attrib(&mut bytes, "DataType", &self.value.data_type)?;
                        write_attrib_opt(&mut bytes, "Issuer", &self.value.issuer)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    MissingAttributeDetailTypeSerializerState::AttributeValue(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = MissingAttributeDetailTypeSerializerState::End__,
                        }
                    }
                    MissingAttributeDetailTypeSerializerState::End__ => {
                        *self.state = MissingAttributeDetailTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    MissingAttributeDetailTypeSerializerState::Done__ => return Ok(None),
                    MissingAttributeDetailTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for MissingAttributeDetailTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = MissingAttributeDetailTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct MultiRequestsTypeSerializer<'ser> {
        pub(super) value: &'ser super::MultiRequestsType,
        pub(super) state: Box<MultiRequestsTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum MultiRequestsTypeSerializerState<'ser> {
        Init__,
        RequestReference(
            IterSerializer<'ser, &'ser [super::RequestReference], super::RequestReference>,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> MultiRequestsTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    MultiRequestsTypeSerializerState::Init__ => {
                        *self.state = MultiRequestsTypeSerializerState::RequestReference(
                            IterSerializer::new(
                                &self.value.request_reference[..],
                                Some("xacml:RequestReference"),
                                false,
                            ),
                        );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    MultiRequestsTypeSerializerState::RequestReference(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = MultiRequestsTypeSerializerState::End__,
                        }
                    }
                    MultiRequestsTypeSerializerState::End__ => {
                        *self.state = MultiRequestsTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    MultiRequestsTypeSerializerState::Done__ => return Ok(None),
                    MultiRequestsTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for MultiRequestsTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = MultiRequestsTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct ObligationExpressionTypeSerializer<'ser> {
        pub(super) value: &'ser super::ObligationExpressionType,
        pub(super) state: Box<ObligationExpressionTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum ObligationExpressionTypeSerializerState<'ser> {
        Init__,
        AttributeAssignmentExpression(
            IterSerializer<
                'ser,
                &'ser [super::AttributeAssignmentExpression],
                super::AttributeAssignmentExpression,
            >,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> ObligationExpressionTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    ObligationExpressionTypeSerializerState::Init__ => {
                        *self.state =
                            ObligationExpressionTypeSerializerState::AttributeAssignmentExpression(
                                IterSerializer::new(
                                    &self.value.attribute_assignment_expression[..],
                                    Some("xacml:AttributeAssignmentExpression"),
                                    false,
                                ),
                            );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "ObligationId", &self.value.obligation_id)?;
                        write_attrib(&mut bytes, "FulfillOn", &self.value.fulfill_on)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    ObligationExpressionTypeSerializerState::AttributeAssignmentExpression(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = ObligationExpressionTypeSerializerState::End__,
                        }
                    }
                    ObligationExpressionTypeSerializerState::End__ => {
                        *self.state = ObligationExpressionTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    ObligationExpressionTypeSerializerState::Done__ => return Ok(None),
                    ObligationExpressionTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for ObligationExpressionTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = ObligationExpressionTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct ObligationExpressionsTypeSerializer<'ser> {
        pub(super) value: &'ser super::ObligationExpressionsType,
        pub(super) state: Box<ObligationExpressionsTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum ObligationExpressionsTypeSerializerState<'ser> {
        Init__,
        ObligationExpression(
            IterSerializer<'ser, &'ser [super::ObligationExpression], super::ObligationExpression>,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> ObligationExpressionsTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    ObligationExpressionsTypeSerializerState::Init__ => {
                        *self.state =
                            ObligationExpressionsTypeSerializerState::ObligationExpression(
                                IterSerializer::new(
                                    &self.value.obligation_expression[..],
                                    Some("xacml:ObligationExpression"),
                                    false,
                                ),
                            );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    ObligationExpressionsTypeSerializerState::ObligationExpression(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = ObligationExpressionsTypeSerializerState::End__,
                        }
                    }
                    ObligationExpressionsTypeSerializerState::End__ => {
                        *self.state = ObligationExpressionsTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    ObligationExpressionsTypeSerializerState::Done__ => return Ok(None),
                    ObligationExpressionsTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for ObligationExpressionsTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = ObligationExpressionsTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct ObligationTypeSerializer<'ser> {
        pub(super) value: &'ser super::ObligationType,
        pub(super) state: Box<ObligationTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum ObligationTypeSerializerState<'ser> {
        Init__,
        AttributeAssignment(
            IterSerializer<'ser, &'ser [super::AttributeAssignment], super::AttributeAssignment>,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> ObligationTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    ObligationTypeSerializerState::Init__ => {
                        *self.state = ObligationTypeSerializerState::AttributeAssignment(
                            IterSerializer::new(
                                &self.value.attribute_assignment[..],
                                Some("xacml:AttributeAssignment"),
                                false,
                            ),
                        );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "ObligationId", &self.value.obligation_id)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    ObligationTypeSerializerState::AttributeAssignment(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = ObligationTypeSerializerState::End__,
                        }
                    }
                    ObligationTypeSerializerState::End__ => {
                        *self.state = ObligationTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    ObligationTypeSerializerState::Done__ => return Ok(None),
                    ObligationTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for ObligationTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = ObligationTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct ObligationsTypeSerializer<'ser> {
        pub(super) value: &'ser super::ObligationsType,
        pub(super) state: Box<ObligationsTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum ObligationsTypeSerializerState<'ser> {
        Init__,
        Obligation(IterSerializer<'ser, &'ser [super::Obligation], super::Obligation>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> ObligationsTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    ObligationsTypeSerializerState::Init__ => {
                        *self.state =
                            ObligationsTypeSerializerState::Obligation(IterSerializer::new(
                                &self.value.obligation[..],
                                Some("xacml:Obligation"),
                                false,
                            ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    ObligationsTypeSerializerState::Obligation(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = ObligationsTypeSerializerState::End__,
                    },
                    ObligationsTypeSerializerState::End__ => {
                        *self.state = ObligationsTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    ObligationsTypeSerializerState::Done__ => return Ok(None),
                    ObligationsTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for ObligationsTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = ObligationsTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct PolicyCombinerParametersTypeSerializer<'ser> {
        pub(super) value: &'ser super::PolicyCombinerParametersType,
        pub(super) state: Box<PolicyCombinerParametersTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum PolicyCombinerParametersTypeSerializerState<'ser> {
        Init__,
        CombinerParameter(
            IterSerializer<'ser, &'ser [super::CombinerParameter], super::CombinerParameter>,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> PolicyCombinerParametersTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    PolicyCombinerParametersTypeSerializerState::Init__ => {
                        *self.state =
                            PolicyCombinerParametersTypeSerializerState::CombinerParameter(
                                IterSerializer::new(
                                    &self.value.combiner_parameter[..],
                                    Some("xacml:CombinerParameter"),
                                    false,
                                ),
                            );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "PolicyIdRef", &self.value.policy_id_ref)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    PolicyCombinerParametersTypeSerializerState::CombinerParameter(x) => match x
                        .next()
                        .transpose()?
                    {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = PolicyCombinerParametersTypeSerializerState::End__,
                    },
                    PolicyCombinerParametersTypeSerializerState::End__ => {
                        *self.state = PolicyCombinerParametersTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    PolicyCombinerParametersTypeSerializerState::Done__ => return Ok(None),
                    PolicyCombinerParametersTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for PolicyCombinerParametersTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = PolicyCombinerParametersTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct PolicyIdentifierListTypeSerializer<'ser> {
        pub(super) value: &'ser super::PolicyIdentifierListType,
        pub(super) state: Box<PolicyIdentifierListTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum PolicyIdentifierListTypeSerializerState<'ser> {
        Init__,
        Content__(
            IterSerializer<
                'ser,
                &'ser [super::PolicyIdentifierListTypeContent],
                super::PolicyIdentifierListTypeContent,
            >,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> PolicyIdentifierListTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    PolicyIdentifierListTypeSerializerState::Init__ => {
                        *self.state = PolicyIdentifierListTypeSerializerState::Content__(
                            IterSerializer::new(&self.value.content[..], None, false),
                        );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    PolicyIdentifierListTypeSerializerState::Content__(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = PolicyIdentifierListTypeSerializerState::End__,
                        }
                    }
                    PolicyIdentifierListTypeSerializerState::End__ => {
                        *self.state = PolicyIdentifierListTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    PolicyIdentifierListTypeSerializerState::Done__ => return Ok(None),
                    PolicyIdentifierListTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for PolicyIdentifierListTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = PolicyIdentifierListTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct PolicyIdentifierListTypeContentSerializer<'ser> {
        pub(super) value: &'ser super::PolicyIdentifierListTypeContent,
        pub(super) state: Box<PolicyIdentifierListTypeContentSerializerState<'ser>>,
    }
    #[derive(Debug)]
    pub(super) enum PolicyIdentifierListTypeContentSerializerState<'ser> {
        Init__,
        PolicyIdReference(<super::PolicyIdReference as WithSerializer>::Serializer<'ser>),
        PolicySetIdReference(<super::PolicySetIdReference as WithSerializer>::Serializer<'ser>),
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> PolicyIdentifierListTypeContentSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    PolicyIdentifierListTypeContentSerializerState::Init__ => match self.value {
                        super::PolicyIdentifierListTypeContent::PolicyIdReference(x) => {
                            *self.state =
                                PolicyIdentifierListTypeContentSerializerState::PolicyIdReference(
                                    WithSerializer::serializer(
                                        x,
                                        Some("xacml:PolicyIdReference"),
                                        false,
                                    )?,
                                )
                        }
                        super::PolicyIdentifierListTypeContent::PolicySetIdReference(x) => {
                            *self.state =
                                PolicyIdentifierListTypeContentSerializerState::PolicySetIdReference(
                                    WithSerializer::serializer(
                                        x,
                                        Some("xacml:PolicySetIdReference"),
                                        false,
                                    )?,
                                )
                        }
                    },
                    PolicyIdentifierListTypeContentSerializerState::PolicyIdReference(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => {
                                *self.state = PolicyIdentifierListTypeContentSerializerState::Done__
                            }
                        }
                    }
                    PolicyIdentifierListTypeContentSerializerState::PolicySetIdReference(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => {
                                *self.state = PolicyIdentifierListTypeContentSerializerState::Done__
                            }
                        }
                    }
                    PolicyIdentifierListTypeContentSerializerState::Done__ => return Ok(None),
                    PolicyIdentifierListTypeContentSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for PolicyIdentifierListTypeContentSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = PolicyIdentifierListTypeContentSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct PolicyIssuerTypeSerializer<'ser> {
        pub(super) value: &'ser super::PolicyIssuerType,
        pub(super) state: Box<PolicyIssuerTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum PolicyIssuerTypeSerializerState<'ser> {
        Init__,
        Content(IterSerializer<'ser, Option<&'ser super::Content>, super::Content>),
        Attribute(IterSerializer<'ser, &'ser [super::Attribute], super::Attribute>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> PolicyIssuerTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    PolicyIssuerTypeSerializerState::Init__ => {
                        *self.state =
                            PolicyIssuerTypeSerializerState::Content(IterSerializer::new(
                                self.value.content.as_ref(),
                                Some("xacml:Content"),
                                false,
                            ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    PolicyIssuerTypeSerializerState::Content(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                PolicyIssuerTypeSerializerState::Attribute(IterSerializer::new(
                                    &self.value.attribute[..],
                                    Some("xacml:Attribute"),
                                    false,
                                ))
                        }
                    },
                    PolicyIssuerTypeSerializerState::Attribute(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = PolicyIssuerTypeSerializerState::End__,
                    },
                    PolicyIssuerTypeSerializerState::End__ => {
                        *self.state = PolicyIssuerTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    PolicyIssuerTypeSerializerState::Done__ => return Ok(None),
                    PolicyIssuerTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for PolicyIssuerTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = PolicyIssuerTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct PolicySetCombinerParametersTypeSerializer<'ser> {
        pub(super) value: &'ser super::PolicySetCombinerParametersType,
        pub(super) state: Box<PolicySetCombinerParametersTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum PolicySetCombinerParametersTypeSerializerState<'ser> {
        Init__,
        CombinerParameter(
            IterSerializer<'ser, &'ser [super::CombinerParameter], super::CombinerParameter>,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> PolicySetCombinerParametersTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    PolicySetCombinerParametersTypeSerializerState::Init__ => {
                        *self.state =
                            PolicySetCombinerParametersTypeSerializerState::CombinerParameter(
                                IterSerializer::new(
                                    &self.value.combiner_parameter[..],
                                    Some("xacml:CombinerParameter"),
                                    false,
                                ),
                            );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "PolicySetIdRef", &self.value.policy_set_id_ref)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    PolicySetCombinerParametersTypeSerializerState::CombinerParameter(x) => match x
                        .next()
                        .transpose()?
                    {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = PolicySetCombinerParametersTypeSerializerState::End__,
                    },
                    PolicySetCombinerParametersTypeSerializerState::End__ => {
                        *self.state = PolicySetCombinerParametersTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    PolicySetCombinerParametersTypeSerializerState::Done__ => return Ok(None),
                    PolicySetCombinerParametersTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for PolicySetCombinerParametersTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = PolicySetCombinerParametersTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct PolicySetTypeSerializer<'ser> {
        pub(super) value: &'ser super::PolicySetType,
        pub(super) state: Box<PolicySetTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum PolicySetTypeSerializerState<'ser> {
        Init__,
        Description(IterSerializer<'ser, Option<&'ser super::Description>, super::Description>),
        PolicyIssuer(IterSerializer<'ser, Option<&'ser super::PolicyIssuer>, super::PolicyIssuer>),
        PolicySetDefaults(
            IterSerializer<'ser, Option<&'ser super::PolicySetDefaults>, super::PolicySetDefaults>,
        ),
        Target(<super::Target as WithSerializer>::Serializer<'ser>),
        Content31(
            IterSerializer<
                'ser,
                &'ser [super::PolicySetContent31Type],
                super::PolicySetContent31Type,
            >,
        ),
        ObligationExpressions(
            IterSerializer<
                'ser,
                Option<&'ser super::ObligationExpressions>,
                super::ObligationExpressions,
            >,
        ),
        AdviceExpressions(
            IterSerializer<'ser, Option<&'ser super::AdviceExpressions>, super::AdviceExpressions>,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> PolicySetTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    PolicySetTypeSerializerState::Init__ => {
                        *self.state =
                            PolicySetTypeSerializerState::Description(IterSerializer::new(
                                self.value.description.as_ref(),
                                Some("xacml:Description"),
                                false,
                            ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "PolicySetId", &self.value.policy_set_id)?;
                        write_attrib(&mut bytes, "Version", &self.value.version)?;
                        write_attrib(
                            &mut bytes,
                            "PolicyCombiningAlgId",
                            &self.value.policy_combining_alg_id,
                        )?;
                        write_attrib_opt(
                            &mut bytes,
                            "MaxDelegationDepth",
                            &self.value.max_delegation_depth,
                        )?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    PolicySetTypeSerializerState::Description(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                PolicySetTypeSerializerState::PolicyIssuer(IterSerializer::new(
                                    self.value.policy_issuer.as_ref(),
                                    Some("xacml:PolicyIssuer"),
                                    false,
                                ))
                        }
                    },
                    PolicySetTypeSerializerState::PolicyIssuer(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state = PolicySetTypeSerializerState::PolicySetDefaults(
                                IterSerializer::new(
                                    self.value.policy_set_defaults.as_ref(),
                                    Some("xacml:PolicySetDefaults"),
                                    false,
                                ),
                            )
                        }
                    },
                    PolicySetTypeSerializerState::PolicySetDefaults(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => {
                                *self.state = PolicySetTypeSerializerState::Target(
                                    WithSerializer::serializer(
                                        &self.value.target,
                                        Some("xacml:Target"),
                                        false,
                                    )?,
                                )
                            }
                        }
                    }
                    PolicySetTypeSerializerState::Target(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                PolicySetTypeSerializerState::Content31(IterSerializer::new(
                                    &self.value.content_31[..],
                                    Some("Content31"),
                                    false,
                                ))
                        }
                    },
                    PolicySetTypeSerializerState::Content31(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state = PolicySetTypeSerializerState::ObligationExpressions(
                                IterSerializer::new(
                                    self.value.obligation_expressions.as_ref(),
                                    Some("xacml:ObligationExpressions"),
                                    false,
                                ),
                            )
                        }
                    },
                    PolicySetTypeSerializerState::ObligationExpressions(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => {
                                *self.state = PolicySetTypeSerializerState::AdviceExpressions(
                                    IterSerializer::new(
                                        self.value.advice_expressions.as_ref(),
                                        Some("xacml:AdviceExpressions"),
                                        false,
                                    ),
                                )
                            }
                        }
                    }
                    PolicySetTypeSerializerState::AdviceExpressions(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = PolicySetTypeSerializerState::End__,
                        }
                    }
                    PolicySetTypeSerializerState::End__ => {
                        *self.state = PolicySetTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    PolicySetTypeSerializerState::Done__ => return Ok(None),
                    PolicySetTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for PolicySetTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = PolicySetTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct PolicyTypeSerializer<'ser> {
        pub(super) value: &'ser super::PolicyType,
        pub(super) state: Box<PolicyTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum PolicyTypeSerializerState<'ser> {
        Init__,
        Description(IterSerializer<'ser, Option<&'ser super::Description>, super::Description>),
        PolicyIssuer(IterSerializer<'ser, Option<&'ser super::PolicyIssuer>, super::PolicyIssuer>),
        PolicyDefaults(
            IterSerializer<'ser, Option<&'ser super::PolicyDefaults>, super::PolicyDefaults>,
        ),
        Target(<super::Target as WithSerializer>::Serializer<'ser>),
        Content41(
            IterSerializer<'ser, &'ser [super::PolicyContent41Type], super::PolicyContent41Type>,
        ),
        ObligationExpressions(
            IterSerializer<
                'ser,
                Option<&'ser super::ObligationExpressions>,
                super::ObligationExpressions,
            >,
        ),
        AdviceExpressions(
            IterSerializer<'ser, Option<&'ser super::AdviceExpressions>, super::AdviceExpressions>,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> PolicyTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    PolicyTypeSerializerState::Init__ => {
                        *self.state = PolicyTypeSerializerState::Description(IterSerializer::new(
                            self.value.description.as_ref(),
                            Some("xacml:Description"),
                            false,
                        ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "PolicyId", &self.value.policy_id)?;
                        write_attrib(&mut bytes, "Version", &self.value.version)?;
                        write_attrib(
                            &mut bytes,
                            "RuleCombiningAlgId",
                            &self.value.rule_combining_alg_id,
                        )?;
                        write_attrib_opt(
                            &mut bytes,
                            "MaxDelegationDepth",
                            &self.value.max_delegation_depth,
                        )?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    PolicyTypeSerializerState::Description(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                PolicyTypeSerializerState::PolicyIssuer(IterSerializer::new(
                                    self.value.policy_issuer.as_ref(),
                                    Some("xacml:PolicyIssuer"),
                                    false,
                                ))
                        }
                    },
                    PolicyTypeSerializerState::PolicyIssuer(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                PolicyTypeSerializerState::PolicyDefaults(IterSerializer::new(
                                    self.value.policy_defaults.as_ref(),
                                    Some("xacml:PolicyDefaults"),
                                    false,
                                ))
                        }
                    },
                    PolicyTypeSerializerState::PolicyDefaults(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                PolicyTypeSerializerState::Target(WithSerializer::serializer(
                                    &self.value.target,
                                    Some("xacml:Target"),
                                    false,
                                )?)
                        }
                    },
                    PolicyTypeSerializerState::Target(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state = PolicyTypeSerializerState::Content41(IterSerializer::new(
                                &self.value.content_41[..],
                                Some("Content41"),
                                false,
                            ))
                        }
                    },
                    PolicyTypeSerializerState::Content41(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state = PolicyTypeSerializerState::ObligationExpressions(
                                IterSerializer::new(
                                    self.value.obligation_expressions.as_ref(),
                                    Some("xacml:ObligationExpressions"),
                                    false,
                                ),
                            )
                        }
                    },
                    PolicyTypeSerializerState::ObligationExpressions(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => {
                                *self.state = PolicyTypeSerializerState::AdviceExpressions(
                                    IterSerializer::new(
                                        self.value.advice_expressions.as_ref(),
                                        Some("xacml:AdviceExpressions"),
                                        false,
                                    ),
                                )
                            }
                        }
                    }
                    PolicyTypeSerializerState::AdviceExpressions(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = PolicyTypeSerializerState::End__,
                        }
                    }
                    PolicyTypeSerializerState::End__ => {
                        *self.state = PolicyTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    PolicyTypeSerializerState::Done__ => return Ok(None),
                    PolicyTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for PolicyTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = PolicyTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct RequestDefaultsTypeSerializer<'ser> {
        pub(super) value: &'ser super::RequestDefaultsType,
        pub(super) state: Box<RequestDefaultsTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum RequestDefaultsTypeSerializerState<'ser> {
        Init__,
        Content3(<super::RequestDefaultsContent3Type as WithSerializer>::Serializer<'ser>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> RequestDefaultsTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    RequestDefaultsTypeSerializerState::Init__ => {
                        *self.state = RequestDefaultsTypeSerializerState::Content3(
                            WithSerializer::serializer(
                                &self.value.content_3,
                                Some("Content3"),
                                false,
                            )?,
                        );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    RequestDefaultsTypeSerializerState::Content3(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = RequestDefaultsTypeSerializerState::End__,
                        }
                    }
                    RequestDefaultsTypeSerializerState::End__ => {
                        *self.state = RequestDefaultsTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    RequestDefaultsTypeSerializerState::Done__ => return Ok(None),
                    RequestDefaultsTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for RequestDefaultsTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = RequestDefaultsTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct RequestReferenceTypeSerializer<'ser> {
        pub(super) value: &'ser super::RequestReferenceType,
        pub(super) state: Box<RequestReferenceTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum RequestReferenceTypeSerializerState<'ser> {
        Init__,
        AttributesReference(
            IterSerializer<'ser, &'ser [super::AttributesReference], super::AttributesReference>,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> RequestReferenceTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    RequestReferenceTypeSerializerState::Init__ => {
                        *self.state = RequestReferenceTypeSerializerState::AttributesReference(
                            IterSerializer::new(
                                &self.value.attributes_reference[..],
                                Some("xacml:AttributesReference"),
                                false,
                            ),
                        );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    RequestReferenceTypeSerializerState::AttributesReference(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = RequestReferenceTypeSerializerState::End__,
                        }
                    }
                    RequestReferenceTypeSerializerState::End__ => {
                        *self.state = RequestReferenceTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    RequestReferenceTypeSerializerState::Done__ => return Ok(None),
                    RequestReferenceTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for RequestReferenceTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = RequestReferenceTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct RequestTypeSerializer<'ser> {
        pub(super) value: &'ser super::RequestType,
        pub(super) state: Box<RequestTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum RequestTypeSerializerState<'ser> {
        Init__,
        RequestDefaults(
            IterSerializer<'ser, Option<&'ser super::RequestDefaults>, super::RequestDefaults>,
        ),
        Attributes(IterSerializer<'ser, &'ser [super::Attributes], super::Attributes>),
        MultiRequests(
            IterSerializer<'ser, Option<&'ser super::MultiRequests>, super::MultiRequests>,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> RequestTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    RequestTypeSerializerState::Init__ => {
                        *self.state =
                            RequestTypeSerializerState::RequestDefaults(IterSerializer::new(
                                self.value.request_defaults.as_ref(),
                                Some("xacml:RequestDefaults"),
                                false,
                            ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(
                            &mut bytes,
                            "ReturnPolicyIdList",
                            &self.value.return_policy_id_list,
                        )?;
                        write_attrib(
                            &mut bytes,
                            "CombinedDecision",
                            &self.value.combined_decision,
                        )?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    RequestTypeSerializerState::RequestDefaults(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                RequestTypeSerializerState::Attributes(IterSerializer::new(
                                    &self.value.attributes[..],
                                    Some("xacml:Attributes"),
                                    false,
                                ))
                        }
                    },
                    RequestTypeSerializerState::Attributes(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                RequestTypeSerializerState::MultiRequests(IterSerializer::new(
                                    self.value.multi_requests.as_ref(),
                                    Some("xacml:MultiRequests"),
                                    false,
                                ))
                        }
                    },
                    RequestTypeSerializerState::MultiRequests(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = RequestTypeSerializerState::End__,
                    },
                    RequestTypeSerializerState::End__ => {
                        *self.state = RequestTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    RequestTypeSerializerState::Done__ => return Ok(None),
                    RequestTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for RequestTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = RequestTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct ResponseTypeSerializer<'ser> {
        pub(super) value: &'ser super::ResponseType,
        pub(super) state: Box<ResponseTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum ResponseTypeSerializerState<'ser> {
        Init__,
        Result(IterSerializer<'ser, &'ser [super::ResultT], super::ResultT>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> ResponseTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    ResponseTypeSerializerState::Init__ => {
                        *self.state = ResponseTypeSerializerState::Result(IterSerializer::new(
                            &self.value.result[..],
                            Some("xacml:Result"),
                            false,
                        ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    ResponseTypeSerializerState::Result(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = ResponseTypeSerializerState::End__,
                    },
                    ResponseTypeSerializerState::End__ => {
                        *self.state = ResponseTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    ResponseTypeSerializerState::Done__ => return Ok(None),
                    ResponseTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for ResponseTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = ResponseTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct ResultTypeSerializer<'ser> {
        pub(super) value: &'ser super::ResultType,
        pub(super) state: Box<ResultTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum ResultTypeSerializerState<'ser> {
        Init__,
        Decision(<super::Decision as WithSerializer>::Serializer<'ser>),
        Status(IterSerializer<'ser, Option<&'ser super::Status>, super::Status>),
        Obligations(IterSerializer<'ser, Option<&'ser super::Obligations>, super::Obligations>),
        AssociatedAdvice(
            IterSerializer<'ser, Option<&'ser super::AssociatedAdvice>, super::AssociatedAdvice>,
        ),
        Attributes(IterSerializer<'ser, &'ser [super::Attributes], super::Attributes>),
        PolicyIdentifierList(
            IterSerializer<
                'ser,
                Option<&'ser super::PolicyIdentifierList>,
                super::PolicyIdentifierList,
            >,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> ResultTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    ResultTypeSerializerState::Init__ => {
                        *self.state =
                            ResultTypeSerializerState::Decision(WithSerializer::serializer(
                                &self.value.decision,
                                Some("xacml:Decision"),
                                false,
                            )?);
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    ResultTypeSerializerState::Decision(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state = ResultTypeSerializerState::Status(IterSerializer::new(
                                self.value.status.as_ref(),
                                Some("xacml:Status"),
                                false,
                            ))
                        }
                    },
                    ResultTypeSerializerState::Status(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                ResultTypeSerializerState::Obligations(IterSerializer::new(
                                    self.value.obligations.as_ref(),
                                    Some("xacml:Obligations"),
                                    false,
                                ))
                        }
                    },
                    ResultTypeSerializerState::Obligations(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                ResultTypeSerializerState::AssociatedAdvice(IterSerializer::new(
                                    self.value.associated_advice.as_ref(),
                                    Some("xacml:AssociatedAdvice"),
                                    false,
                                ))
                        }
                    },
                    ResultTypeSerializerState::AssociatedAdvice(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                ResultTypeSerializerState::Attributes(IterSerializer::new(
                                    &self.value.attributes[..],
                                    Some("xacml:Attributes"),
                                    false,
                                ))
                        }
                    },
                    ResultTypeSerializerState::Attributes(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state = ResultTypeSerializerState::PolicyIdentifierList(
                                IterSerializer::new(
                                    self.value.policy_identifier_list.as_ref(),
                                    Some("xacml:PolicyIdentifierList"),
                                    false,
                                ),
                            )
                        }
                    },
                    ResultTypeSerializerState::PolicyIdentifierList(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = ResultTypeSerializerState::End__,
                        }
                    }
                    ResultTypeSerializerState::End__ => {
                        *self.state = ResultTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    ResultTypeSerializerState::Done__ => return Ok(None),
                    ResultTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for ResultTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = ResultTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct RuleCombinerParametersTypeSerializer<'ser> {
        pub(super) value: &'ser super::RuleCombinerParametersType,
        pub(super) state: Box<RuleCombinerParametersTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum RuleCombinerParametersTypeSerializerState<'ser> {
        Init__,
        CombinerParameter(
            IterSerializer<'ser, &'ser [super::CombinerParameter], super::CombinerParameter>,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> RuleCombinerParametersTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    RuleCombinerParametersTypeSerializerState::Init__ => {
                        *self.state = RuleCombinerParametersTypeSerializerState::CombinerParameter(
                            IterSerializer::new(
                                &self.value.combiner_parameter[..],
                                Some("xacml:CombinerParameter"),
                                false,
                            ),
                        );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "RuleIdRef", &self.value.rule_id_ref)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    RuleCombinerParametersTypeSerializerState::CombinerParameter(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = RuleCombinerParametersTypeSerializerState::End__,
                        }
                    }
                    RuleCombinerParametersTypeSerializerState::End__ => {
                        *self.state = RuleCombinerParametersTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    RuleCombinerParametersTypeSerializerState::Done__ => return Ok(None),
                    RuleCombinerParametersTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for RuleCombinerParametersTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = RuleCombinerParametersTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct RuleTypeSerializer<'ser> {
        pub(super) value: &'ser super::RuleType,
        pub(super) state: Box<RuleTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum RuleTypeSerializerState<'ser> {
        Init__,
        Description(IterSerializer<'ser, Option<&'ser super::Description>, super::Description>),
        Target(IterSerializer<'ser, Option<&'ser super::Target>, super::Target>),
        Condition(IterSerializer<'ser, Option<&'ser super::Condition>, super::Condition>),
        ObligationExpressions(
            IterSerializer<
                'ser,
                Option<&'ser super::ObligationExpressions>,
                super::ObligationExpressions,
            >,
        ),
        AdviceExpressions(
            IterSerializer<'ser, Option<&'ser super::AdviceExpressions>, super::AdviceExpressions>,
        ),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> RuleTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    RuleTypeSerializerState::Init__ => {
                        *self.state = RuleTypeSerializerState::Description(IterSerializer::new(
                            self.value.description.as_ref(),
                            Some("xacml:Description"),
                            false,
                        ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "RuleId", &self.value.rule_id)?;
                        write_attrib(&mut bytes, "Effect", &self.value.effect)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    RuleTypeSerializerState::Description(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state = RuleTypeSerializerState::Target(IterSerializer::new(
                                self.value.target.as_ref(),
                                Some("xacml:Target"),
                                false,
                            ))
                        }
                    },
                    RuleTypeSerializerState::Target(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state = RuleTypeSerializerState::Condition(IterSerializer::new(
                                self.value.condition.as_ref(),
                                Some("xacml:Condition"),
                                false,
                            ))
                        }
                    },
                    RuleTypeSerializerState::Condition(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                RuleTypeSerializerState::ObligationExpressions(IterSerializer::new(
                                    self.value.obligation_expressions.as_ref(),
                                    Some("xacml:ObligationExpressions"),
                                    false,
                                ))
                        }
                    },
                    RuleTypeSerializerState::ObligationExpressions(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => {
                                *self.state =
                                    RuleTypeSerializerState::AdviceExpressions(IterSerializer::new(
                                        self.value.advice_expressions.as_ref(),
                                        Some("xacml:AdviceExpressions"),
                                        false,
                                    ))
                            }
                        }
                    }
                    RuleTypeSerializerState::AdviceExpressions(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = RuleTypeSerializerState::End__,
                    },
                    RuleTypeSerializerState::End__ => {
                        *self.state = RuleTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    RuleTypeSerializerState::Done__ => return Ok(None),
                    RuleTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for RuleTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = RuleTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct StatusCodeTypeSerializer<'ser> {
        pub(super) value: &'ser super::StatusCodeType,
        pub(super) state: Box<StatusCodeTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum StatusCodeTypeSerializerState<'ser> {
        Init__,
        StatusCode(IterSerializer<'ser, Option<&'ser super::StatusCode>, super::StatusCode>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> StatusCodeTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    StatusCodeTypeSerializerState::Init__ => {
                        *self.state =
                            StatusCodeTypeSerializerState::StatusCode(IterSerializer::new(
                                self.value.status_code.as_ref().map(|x| &**x),
                                Some("xacml:StatusCode"),
                                false,
                            ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "Value", &self.value.value)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    StatusCodeTypeSerializerState::StatusCode(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = StatusCodeTypeSerializerState::End__,
                    },
                    StatusCodeTypeSerializerState::End__ => {
                        *self.state = StatusCodeTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    StatusCodeTypeSerializerState::Done__ => return Ok(None),
                    StatusCodeTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for StatusCodeTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = StatusCodeTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct StatusDetailTypeSerializer<'ser> {
        pub(super) value: &'ser super::StatusDetailType,
        pub(super) state: Box<StatusDetailTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum StatusDetailTypeSerializerState<'ser> {
        Init__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> StatusDetailTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    StatusDetailTypeSerializerState::Init__ => {
                        *self.state = StatusDetailTypeSerializerState::Done__;
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Empty(bytes)));
                    }
                    StatusDetailTypeSerializerState::Done__ => return Ok(None),
                    StatusDetailTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for StatusDetailTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = StatusDetailTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct StatusTypeSerializer<'ser> {
        pub(super) value: &'ser super::StatusType,
        pub(super) state: Box<StatusTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum StatusTypeSerializerState<'ser> {
        Init__,
        StatusCode(<super::StatusCode as WithSerializer>::Serializer<'ser>),
        StatusMessage(
            IterSerializer<'ser, Option<&'ser super::StatusMessage>, super::StatusMessage>,
        ),
        StatusDetail(IterSerializer<'ser, Option<&'ser super::StatusDetail>, super::StatusDetail>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> StatusTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    StatusTypeSerializerState::Init__ => {
                        *self.state =
                            StatusTypeSerializerState::StatusCode(WithSerializer::serializer(
                                &self.value.status_code,
                                Some("xacml:StatusCode"),
                                false,
                            )?);
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    StatusTypeSerializerState::StatusCode(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                StatusTypeSerializerState::StatusMessage(IterSerializer::new(
                                    self.value.status_message.as_ref(),
                                    Some("xacml:StatusMessage"),
                                    false,
                                ))
                        }
                    },
                    StatusTypeSerializerState::StatusMessage(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => {
                            *self.state =
                                StatusTypeSerializerState::StatusDetail(IterSerializer::new(
                                    self.value.status_detail.as_ref(),
                                    Some("xacml:StatusDetail"),
                                    false,
                                ))
                        }
                    },
                    StatusTypeSerializerState::StatusDetail(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = StatusTypeSerializerState::End__,
                    },
                    StatusTypeSerializerState::End__ => {
                        *self.state = StatusTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    StatusTypeSerializerState::Done__ => return Ok(None),
                    StatusTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for StatusTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = StatusTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct TargetTypeSerializer<'ser> {
        pub(super) value: &'ser super::TargetType,
        pub(super) state: Box<TargetTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum TargetTypeSerializerState<'ser> {
        Init__,
        Content__(IterSerializer<'ser, &'ser [super::TargetTypeContent], super::TargetTypeContent>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> TargetTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    TargetTypeSerializerState::Init__ => {
                        *self.state = TargetTypeSerializerState::Content__(IterSerializer::new(
                            &self.value.content[..],
                            None,
                            false,
                        ));
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        return Ok(Some(Event::Start(bytes)));
                    }
                    TargetTypeSerializerState::Content__(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = TargetTypeSerializerState::End__,
                    },
                    TargetTypeSerializerState::End__ => {
                        *self.state = TargetTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    TargetTypeSerializerState::Done__ => return Ok(None),
                    TargetTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for TargetTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = TargetTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct TargetTypeContentSerializer<'ser> {
        pub(super) value: &'ser super::TargetTypeContent,
        pub(super) state: Box<TargetTypeContentSerializerState<'ser>>,
    }
    #[derive(Debug)]
    pub(super) enum TargetTypeContentSerializerState<'ser> {
        Init__,
        AnyOf(<super::AnyOf as WithSerializer>::Serializer<'ser>),
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> TargetTypeContentSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    TargetTypeContentSerializerState::Init__ => {
                        *self.state =
                            TargetTypeContentSerializerState::AnyOf(WithSerializer::serializer(
                                &self.value.any_of,
                                Some("xacml:AnyOf"),
                                false,
                            )?);
                    }
                    TargetTypeContentSerializerState::AnyOf(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = TargetTypeContentSerializerState::Done__,
                    },
                    TargetTypeContentSerializerState::Done__ => return Ok(None),
                    TargetTypeContentSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for TargetTypeContentSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = TargetTypeContentSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct VariableDefinitionTypeSerializer<'ser> {
        pub(super) value: &'ser super::VariableDefinitionType,
        pub(super) state: Box<VariableDefinitionTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum VariableDefinitionTypeSerializerState<'ser> {
        Init__,
        Expression(<super::Expression as WithSerializer>::Serializer<'ser>),
        End__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> VariableDefinitionTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    VariableDefinitionTypeSerializerState::Init__ => {
                        *self.state = VariableDefinitionTypeSerializerState::Expression(
                            WithSerializer::serializer(
                                &self.value.expression,
                                Some("xacml:Expression"),
                                false,
                            )?,
                        );
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "VariableId", &self.value.variable_id)?;
                        return Ok(Some(Event::Start(bytes)));
                    }
                    VariableDefinitionTypeSerializerState::Expression(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = VariableDefinitionTypeSerializerState::End__,
                        }
                    }
                    VariableDefinitionTypeSerializerState::End__ => {
                        *self.state = VariableDefinitionTypeSerializerState::Done__;
                        return Ok(Some(Event::End(BytesEnd::new(self.name))));
                    }
                    VariableDefinitionTypeSerializerState::Done__ => return Ok(None),
                    VariableDefinitionTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for VariableDefinitionTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = VariableDefinitionTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct VariableReferenceTypeSerializer<'ser> {
        pub(super) value: &'ser super::VariableReferenceType,
        pub(super) state: Box<VariableReferenceTypeSerializerState<'ser>>,
        pub(super) name: &'ser str,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum VariableReferenceTypeSerializerState<'ser> {
        Init__,
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> VariableReferenceTypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    VariableReferenceTypeSerializerState::Init__ => {
                        *self.state = VariableReferenceTypeSerializerState::Done__;
                        let mut bytes = BytesStart::new(self.name);
                        if self.is_root {
                            bytes.push_attribute((&b"xmlns:xacml"[..], &super::NS_XACML[..]));
                        }
                        write_attrib(&mut bytes, "VariableId", &self.value.variable_id)?;
                        return Ok(Some(Event::Empty(bytes)));
                    }
                    VariableReferenceTypeSerializerState::Done__ => return Ok(None),
                    VariableReferenceTypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for VariableReferenceTypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = VariableReferenceTypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct DefaultsContent39TypeSerializer<'ser> {
        pub(super) value: &'ser super::DefaultsContent39Type,
        pub(super) state: Box<DefaultsContent39TypeSerializerState<'ser>>,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum DefaultsContent39TypeSerializerState<'ser> {
        Init__,
        XPathVersion(<super::XPathVersion as WithSerializer>::Serializer<'ser>),
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> DefaultsContent39TypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    DefaultsContent39TypeSerializerState::Init__ => match self.value {
                        super::DefaultsContent39Type::XPathVersion(x) => {
                            *self.state = DefaultsContent39TypeSerializerState::XPathVersion(
                                WithSerializer::serializer(
                                    x,
                                    Some("xacml:XPathVersion"),
                                    self.is_root,
                                )?,
                            )
                        }
                    },
                    DefaultsContent39TypeSerializerState::XPathVersion(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = DefaultsContent39TypeSerializerState::Done__,
                        }
                    }
                    DefaultsContent39TypeSerializerState::Done__ => return Ok(None),
                    DefaultsContent39TypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for DefaultsContent39TypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = DefaultsContent39TypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct MatchContent47TypeSerializer<'ser> {
        pub(super) value: &'ser super::MatchContent47Type,
        pub(super) state: Box<MatchContent47TypeSerializerState<'ser>>,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum MatchContent47TypeSerializerState<'ser> {
        Init__,
        AttributeDesignator(<super::AttributeDesignator as WithSerializer>::Serializer<'ser>),
        AttributeSelector(<super::AttributeSelector as WithSerializer>::Serializer<'ser>),
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> MatchContent47TypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    MatchContent47TypeSerializerState::Init__ => match self.value {
                        super::MatchContent47Type::AttributeDesignator(x) => {
                            *self.state = MatchContent47TypeSerializerState::AttributeDesignator(
                                WithSerializer::serializer(
                                    x,
                                    Some("xacml:AttributeDesignator"),
                                    self.is_root,
                                )?,
                            )
                        }
                        super::MatchContent47Type::AttributeSelector(x) => {
                            *self.state = MatchContent47TypeSerializerState::AttributeSelector(
                                WithSerializer::serializer(
                                    x,
                                    Some("xacml:AttributeSelector"),
                                    self.is_root,
                                )?,
                            )
                        }
                    },
                    MatchContent47TypeSerializerState::AttributeDesignator(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = MatchContent47TypeSerializerState::Done__,
                        }
                    }
                    MatchContent47TypeSerializerState::AttributeSelector(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = MatchContent47TypeSerializerState::Done__,
                        }
                    }
                    MatchContent47TypeSerializerState::Done__ => return Ok(None),
                    MatchContent47TypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for MatchContent47TypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = MatchContent47TypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct PolicySetContent31TypeSerializer<'ser> {
        pub(super) value: &'ser super::PolicySetContent31Type,
        pub(super) state: Box<PolicySetContent31TypeSerializerState<'ser>>,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum PolicySetContent31TypeSerializerState<'ser> {
        Init__,
        PolicySet(<super::PolicySet as WithSerializer>::Serializer<'ser>),
        Policy(<super::Policy as WithSerializer>::Serializer<'ser>),
        PolicySetIdReference(<super::PolicySetIdReference as WithSerializer>::Serializer<'ser>),
        PolicyIdReference(<super::PolicyIdReference as WithSerializer>::Serializer<'ser>),
        CombinerParameters(<super::CombinerParameters as WithSerializer>::Serializer<'ser>),
        PolicyCombinerParameters(
            <super::PolicyCombinerParameters as WithSerializer>::Serializer<'ser>,
        ),
        PolicySetCombinerParameters(
            <super::PolicySetCombinerParameters as WithSerializer>::Serializer<'ser>,
        ),
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> PolicySetContent31TypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    PolicySetContent31TypeSerializerState::Init__ => match self.value {
                        super::PolicySetContent31Type::PolicySet(x) => {
                            *self.state = PolicySetContent31TypeSerializerState::PolicySet(
                                WithSerializer::serializer(
                                    x,
                                    Some("xacml:PolicySet"),
                                    self.is_root,
                                )?,
                            )
                        }
                        super::PolicySetContent31Type::Policy(x) => {
                            *self.state = PolicySetContent31TypeSerializerState::Policy(
                                WithSerializer::serializer(x, Some("xacml:Policy"), self.is_root)?,
                            )
                        }
                        super::PolicySetContent31Type::PolicySetIdReference(x) => {
                            *self.state =
                                PolicySetContent31TypeSerializerState::PolicySetIdReference(
                                    WithSerializer::serializer(
                                        x,
                                        Some("xacml:PolicySetIdReference"),
                                        self.is_root,
                                    )?,
                                )
                        }
                        super::PolicySetContent31Type::PolicyIdReference(x) => {
                            *self.state = PolicySetContent31TypeSerializerState::PolicyIdReference(
                                WithSerializer::serializer(
                                    x,
                                    Some("xacml:PolicyIdReference"),
                                    self.is_root,
                                )?,
                            )
                        }
                        super::PolicySetContent31Type::CombinerParameters(x) => {
                            *self.state = PolicySetContent31TypeSerializerState::CombinerParameters(
                                WithSerializer::serializer(
                                    x,
                                    Some("xacml:CombinerParameters"),
                                    self.is_root,
                                )?,
                            )
                        }
                        super::PolicySetContent31Type::PolicyCombinerParameters(x) => {
                            *self.state =
                                PolicySetContent31TypeSerializerState::PolicyCombinerParameters(
                                    WithSerializer::serializer(
                                        x,
                                        Some("xacml:PolicyCombinerParameters"),
                                        self.is_root,
                                    )?,
                                )
                        }
                        super::PolicySetContent31Type::PolicySetCombinerParameters(x) => {
                            *self.state =
                                PolicySetContent31TypeSerializerState::PolicySetCombinerParameters(
                                    WithSerializer::serializer(
                                        x,
                                        Some("xacml:PolicySetCombinerParameters"),
                                        self.is_root,
                                    )?,
                                )
                        }
                    },
                    PolicySetContent31TypeSerializerState::PolicySet(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = PolicySetContent31TypeSerializerState::Done__,
                        }
                    }
                    PolicySetContent31TypeSerializerState::Policy(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = PolicySetContent31TypeSerializerState::Done__,
                        }
                    }
                    PolicySetContent31TypeSerializerState::PolicySetIdReference(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = PolicySetContent31TypeSerializerState::Done__,
                        }
                    }
                    PolicySetContent31TypeSerializerState::PolicyIdReference(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = PolicySetContent31TypeSerializerState::Done__,
                        }
                    }
                    PolicySetContent31TypeSerializerState::CombinerParameters(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = PolicySetContent31TypeSerializerState::Done__,
                        }
                    }
                    PolicySetContent31TypeSerializerState::PolicyCombinerParameters(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = PolicySetContent31TypeSerializerState::Done__,
                        }
                    }
                    PolicySetContent31TypeSerializerState::PolicySetCombinerParameters(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = PolicySetContent31TypeSerializerState::Done__,
                        }
                    }
                    PolicySetContent31TypeSerializerState::Done__ => return Ok(None),
                    PolicySetContent31TypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for PolicySetContent31TypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = PolicySetContent31TypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct PolicyContent41TypeSerializer<'ser> {
        pub(super) value: &'ser super::PolicyContent41Type,
        pub(super) state: Box<PolicyContent41TypeSerializerState<'ser>>,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum PolicyContent41TypeSerializerState<'ser> {
        Init__,
        CombinerParameters(
            IterSerializer<
                'ser,
                Option<&'ser super::CombinerParameters>,
                super::CombinerParameters,
            >,
        ),
        RuleCombinerParameters(
            IterSerializer<
                'ser,
                Option<&'ser super::RuleCombinerParameters>,
                super::RuleCombinerParameters,
            >,
        ),
        VariableDefinition(<super::VariableDefinition as WithSerializer>::Serializer<'ser>),
        Rule(<super::Rule as WithSerializer>::Serializer<'ser>),
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> PolicyContent41TypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    PolicyContent41TypeSerializerState::Init__ => match self.value {
                        super::PolicyContent41Type::CombinerParameters(x) => {
                            *self.state = PolicyContent41TypeSerializerState::CombinerParameters(
                                IterSerializer::new(
                                    x.as_ref(),
                                    Some("xacml:CombinerParameters"),
                                    self.is_root,
                                ),
                            )
                        }
                        super::PolicyContent41Type::RuleCombinerParameters(x) => {
                            *self.state = PolicyContent41TypeSerializerState::RuleCombinerParameters(
                                IterSerializer::new(
                                    x.as_ref(),
                                    Some("xacml:RuleCombinerParameters"),
                                    self.is_root,
                                ),
                            )
                        }
                        super::PolicyContent41Type::VariableDefinition(x) => {
                            *self.state = PolicyContent41TypeSerializerState::VariableDefinition(
                                WithSerializer::serializer(
                                    x,
                                    Some("xacml:VariableDefinition"),
                                    self.is_root,
                                )?,
                            )
                        }
                        super::PolicyContent41Type::Rule(x) => {
                            *self.state = PolicyContent41TypeSerializerState::Rule(
                                WithSerializer::serializer(x, Some("xacml:Rule"), self.is_root)?,
                            )
                        }
                    },
                    PolicyContent41TypeSerializerState::CombinerParameters(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = PolicyContent41TypeSerializerState::Done__,
                        }
                    }
                    PolicyContent41TypeSerializerState::RuleCombinerParameters(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = PolicyContent41TypeSerializerState::Done__,
                        }
                    }
                    PolicyContent41TypeSerializerState::VariableDefinition(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => *self.state = PolicyContent41TypeSerializerState::Done__,
                        }
                    }
                    PolicyContent41TypeSerializerState::Rule(x) => match x.next().transpose()? {
                        Some(event) => return Ok(Some(event)),
                        None => *self.state = PolicyContent41TypeSerializerState::Done__,
                    },
                    PolicyContent41TypeSerializerState::Done__ => return Ok(None),
                    PolicyContent41TypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for PolicyContent41TypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = PolicyContent41TypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
    #[derive(Debug)]
    pub struct RequestDefaultsContent3TypeSerializer<'ser> {
        pub(super) value: &'ser super::RequestDefaultsContent3Type,
        pub(super) state: Box<RequestDefaultsContent3TypeSerializerState<'ser>>,
        pub(super) is_root: bool,
    }
    #[derive(Debug)]
    pub(super) enum RequestDefaultsContent3TypeSerializerState<'ser> {
        Init__,
        XPathVersion(<super::XPathVersion as WithSerializer>::Serializer<'ser>),
        Done__,
        Phantom__(&'ser ()),
    }
    impl<'ser> RequestDefaultsContent3TypeSerializer<'ser> {
        fn next_event(&mut self) -> std::result::Result<Option<Event<'ser>>, Error> {
            loop {
                match &mut *self.state {
                    RequestDefaultsContent3TypeSerializerState::Init__ => match self.value {
                        super::RequestDefaultsContent3Type::XPathVersion(x) => {
                            *self.state = RequestDefaultsContent3TypeSerializerState::XPathVersion(
                                WithSerializer::serializer(
                                    x,
                                    Some("xacml:XPathVersion"),
                                    self.is_root,
                                )?,
                            )
                        }
                    },
                    RequestDefaultsContent3TypeSerializerState::XPathVersion(x) => {
                        match x.next().transpose()? {
                            Some(event) => return Ok(Some(event)),
                            None => {
                                *self.state = RequestDefaultsContent3TypeSerializerState::Done__
                            }
                        }
                    }
                    RequestDefaultsContent3TypeSerializerState::Done__ => return Ok(None),
                    RequestDefaultsContent3TypeSerializerState::Phantom__(_) => unreachable!(),
                }
            }
        }
    }
    impl<'ser> Iterator for RequestDefaultsContent3TypeSerializer<'ser> {
        type Item = std::result::Result<Event<'ser>, Error>;
        fn next(&mut self) -> Option<Self::Item> {
            match self.next_event() {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => {
                    *self.state = RequestDefaultsContent3TypeSerializerState::Done__;
                    Some(Err(error))
                }
            }
        }
    }
}
