use std::collections::HashMap;

use quick_xml::de::Text;
use regex::Regex;
use crate::{intermediate::{self, IntermediateAttributeValueType}, xacml::{constants::{LongInteger, NormalInteger, SmallInteger, USmallInteger, XacmlDataTypesStrings}, policy::Loadout, types::{AttributeValueType, AttributeValueWithId}}};
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, TimeDelta, Utc};
use uuid::Uuid;
use xsd_parser::xml::Value;


pub trait Transform<T,S> {
    const BASE_DATE: &str = "2025-12-14T";
    fn transform(in_data:T) -> S;
}

pub struct DayTimeDuration {
    negative: bool,
    days: Option<u32>,
    hours: Option<u32>,
    minutes: Option<u32>,
    seconds: Option<u32>,
}

pub struct YearMonthDuration {
    negative: bool,
    years: Option<u32>,
    months: Option<u32>,
}

impl DayTimeDuration {
    fn parse(input: &str) -> Option<DayTimeDuration> {
        let re = Regex::new(
            r"^(?P<sign>-)?P(?:(?P<days>\d+)D)?(?:(?P<hours>\d+)H)?(?:(?P<minutes>\d+)M)?(?:(?P<seconds>\d+)S)?$"
        ).unwrap();

        let caps = re.captures(input)?;

        Some(DayTimeDuration {
            negative: caps.name("sign").is_some(),
            days: caps.name("days").map(|m| m.as_str().parse().ok()).flatten(),
            hours: caps.name("hours").map(|m| m.as_str().parse().ok()).flatten(),
            minutes: caps.name("minutes").map(|m| m.as_str().parse().ok()).flatten(),
            seconds: caps.name("seconds").map(|m| m.as_str().parse().ok()).flatten(),
        })
    }
}

impl YearMonthDuration {
    fn parse(input: &str) -> Option<YearMonthDuration> {
        let re = Regex::new(
            r"^(?P<sign>-)?P(?:(?P<years>\d+)Y)?(?:(?P<months>\d+)M)?$"
        ).unwrap();

        let caps = re.captures(input)?;

        Some(YearMonthDuration {
            negative: caps.name("sign").is_some(),
            years: caps.name("years").map(|m| m.as_str().parse().ok()).flatten(),
            months: caps.name("months").map(|m| m.as_str().parse().ok()).flatten(),
        })
    }
}

pub struct TransformTime;
pub struct TransformDate;
pub struct TransformDateTime;
pub struct TransformDayTimeDuration;
pub struct TransformYearMonthDuration;
pub struct TransformDnsName;
pub struct TransformX500Name;

impl Transform<String, USmallInteger> for TransformTime {
    fn transform(in_data:String) -> USmallInteger {
        //  convert time to int
        // let mut result_string = Self::BASE_DATE.to_string();
        // result_string.push_str(&in_data);
        //  time to seconds
        let splits: Vec<&str> = in_data.split(":").map(|entry| entry.trim()).collect();
        assert_eq!(splits.len(), 3);
        let hours = splits[0].parse::<i64>().unwrap();
        let minutes = splits[1].parse::<i64>().unwrap();
        let seconds = splits[2].parse::<i64>().unwrap();
        let seconds_since_start = chrono::TimeDelta::hours(hours).num_seconds() + 
            chrono::TimeDelta::minutes(minutes).num_seconds() +
            chrono::TimeDelta::seconds(seconds).num_seconds();
        // let seconds_since_start = TransformDateTime::transform(result_string);
        assert!(seconds_since_start>=0);
        assert!(seconds_since_start<u32::MAX as i64);

        return seconds_since_start as u32 
    }
} 


impl Transform<String, NormalInteger> for TransformDate {
    fn transform(in_data:String) -> NormalInteger {
    let date = NaiveDate::parse_from_str(&in_data, "%Y-%m-%d").unwrap();
    
        let epoch = NaiveDate::from_ymd_opt(1, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
    
        let date_as_datetime = date
            .and_hms_opt(0, 0, 0)
            .unwrap();
    
        let seconds_since_start = date_as_datetime
            .signed_duration_since(epoch)
            .num_seconds();
    
        seconds_since_start
    }
}

impl Transform<String, NormalInteger> for TransformDateTime {
    fn transform(in_data:String) -> NormalInteger {
        let dt: DateTime<FixedOffset> = DateTime::parse_from_rfc3339(&in_data).unwrap();
        let epoch = NaiveDate::from_ymd_opt(1, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let dt_utc = dt.with_timezone(&Utc);
        let seconds_since_start = dt_utc.naive_utc().signed_duration_since(epoch).num_seconds();
        
        return seconds_since_start
    }
}

impl Transform<String,NormalInteger> for TransformDayTimeDuration{
    fn transform(in_data:String) -> NormalInteger {
        //  separate the days from the time and transform it
        let vector= in_data.split("D")
            .collect::<Vec<&str>>();
        let days = vector.get(0)
            .unwrap()
            .parse::<i64>()
            .unwrap();
        let time = vector.get(1)
            .map_or(
                "00H00M00S",
            |v|v).to_string();
        let mut times = time.split(|c| c=='H' || c=='M' || c=='S');
        
        let hours: NormalInteger    = times.next().unwrap().parse().unwrap();
        let minutes: NormalInteger  = times.next().unwrap().parse().unwrap();
        let seconds: NormalInteger  = times.next().unwrap().parse().unwrap();
        
        let time_converted = chrono::TimeDelta::hours(hours).checked_add(
            &TimeDelta::minutes(minutes).checked_add(
                &TimeDelta::seconds(seconds)
            ).unwrap()
        ).unwrap();

        let time_duration = chrono::Duration::days(days).checked_add(&time_converted).unwrap();

        time_duration.num_seconds()

    }
}

impl Transform<String,NormalInteger> for TransformYearMonthDuration {
    fn transform(in_data:String) -> NormalInteger {
        let duration = YearMonthDuration::parse(&in_data).unwrap();
        
        let total_duration = 
            (duration.years.map_or(0u32, |v|v) as i64) * 12 + 
            (duration.months.map_or(0u32, |v|v) as i64);
        total_duration * (
            match duration.negative {
                true => -1i64,
                false => 1i64
            }
        )
    }
}

impl Transform<String,(String,USmallInteger,USmallInteger)> for TransformDnsName {
    fn transform(in_data:String) -> (String,USmallInteger,USmallInteger) {
        //  read string 
        let re = Regex::new(
            r"^(?P<hostname>\d+):(?P<startport>\d+)?-?(?P<endport>\d+)?$"
        ).unwrap();

     
        let caps = re.captures(in_data.as_str()).unwrap();
        let hostname = caps.name("hostname").map(|m| m.as_str().to_string()).unwrap();
        (
            hostname,
            caps.name("startport").map(|m| m.as_str().parse::<SmallInteger>().ok()).flatten().map_or(1, |v|v as u32),
            caps.name("endport").map(|m| m.as_str().parse::<SmallInteger>().ok()).flatten().map_or(2^16, |v|v as u32),
        )   
    }
}

impl Transform<String, HashMap<String,String>> for TransformX500Name {
    fn transform(in_data:String) -> HashMap<String,String> {
        let splits = in_data.split(",");
        let mut entries: HashMap<String,String> = HashMap::new();
        for split in splits.into_iter() {
            let split_splits:Vec<String> = split.split("=").map(|e|e.to_string()).collect();
            let key = split_splits.get(0).unwrap();
            let value = split_splits.get(0).unwrap();

            entries.insert(key.to_owned(), value.to_owned());
        }
        entries
    }
}

impl Transform<&AttributeValueWithId, IntermediateAttributeValueType> for IntermediateAttributeValueType{
    fn transform(in_data:&AttributeValueWithId) -> IntermediateAttributeValueType {
        let t = XacmlDataTypesStrings::from_str(&in_data.att_val.data_type).unwrap();
        match t {
            XacmlDataTypesStrings::String => IntermediateAttributeValueType { 
                data_type: in_data.att_val.data_type.clone(),
                att_id: in_data.id.clone(),
                
                text: intermediate::IntermediateTypes::String(
                    in_data.att_val.text.clone().unwrap().0.trim().to_string()
                ), 
                
            },
            XacmlDataTypesStrings::Time => IntermediateAttributeValueType { 
                data_type: in_data.att_val.data_type.clone(),
                att_id: in_data.id.clone(), 
                text: intermediate::IntermediateTypes::Time(
                    TransformTime::transform(in_data.att_val.text.clone().unwrap().0.trim().to_string())
                ), 
                
            },
            
            XacmlDataTypesStrings::Date => IntermediateAttributeValueType { 
                data_type: in_data.att_val.data_type.clone(),
                att_id: in_data.id.clone(), 
                text: intermediate::IntermediateTypes::Date(
                    TransformDate::transform(in_data.att_val.text.clone().unwrap().0.trim().to_string())
                ), 
                
            },
            XacmlDataTypesStrings::DateTime => IntermediateAttributeValueType { 
                data_type: in_data.att_val.data_type.clone(),
                att_id: in_data.id.clone(),
                text: intermediate::IntermediateTypes::DateTime(
                    TransformDateTime::transform(in_data.att_val.text.clone().unwrap().0.trim().to_string())
                ), 
                
            },
            XacmlDataTypesStrings::DayTimeDuration => IntermediateAttributeValueType { 
                data_type: in_data.att_val.data_type.clone(),
                att_id: in_data.id.clone(),
                text: intermediate::IntermediateTypes::DayTimeDuration(
                    TransformDayTimeDuration::transform(in_data.att_val.text.clone().unwrap().0.trim().to_string())
                ), 
                
            },
            XacmlDataTypesStrings::YearMonthDuration => IntermediateAttributeValueType { 
                data_type: in_data.att_val.data_type.clone(),
                att_id: in_data.id.clone(),
                text: intermediate::IntermediateTypes::YearMonthDuration(
                    TransformYearMonthDuration::transform(in_data.att_val.text.clone().unwrap().0.trim().to_string())
                ), 
                
            },
            XacmlDataTypesStrings::Integer => IntermediateAttributeValueType { 
                data_type: in_data.att_val.data_type.clone(),
                att_id: in_data.id.clone(),
                text: intermediate::IntermediateTypes::Integer(
                    in_data
                    .att_val
                    .text
                    .clone()
                    .unwrap()
                    .0
                    .trim()
                    .to_string()
                    .parse::<NormalInteger>()
                    .unwrap()), 
                
            },
            XacmlDataTypesStrings::Double => IntermediateAttributeValueType { 
                data_type: in_data.att_val.data_type.clone(),
                att_id: in_data.id.clone(),
                text: intermediate::IntermediateTypes::Double(
                    in_data
                    .att_val
                    .text
                    .clone()
                    .unwrap()
                    .0
                    .trim()
                    .to_string()
                    .parse::<LongInteger>()
                    .unwrap()), 
                
            },
            XacmlDataTypesStrings::Boolean => IntermediateAttributeValueType { 
                data_type: in_data.att_val.data_type.clone(),
                att_id: in_data.id.clone(),
                text: intermediate::IntermediateTypes::Boolean(
                    match in_data
                        .att_val
                        .text
                        .clone()
                        .unwrap()
                        .0
                        .trim() 
                    {
                        "True" => {true},
                        _ => {false},
                    }
                ), 
                
            },
            XacmlDataTypesStrings::Base64Binary => IntermediateAttributeValueType { 
                data_type: in_data.att_val.data_type.clone(),
                att_id: in_data.id.clone(),
                text: intermediate::IntermediateTypes::Base64Binary(
                    in_data.att_val.text.clone().unwrap().0.trim().to_string().into()
                ), 
                
            },
            XacmlDataTypesStrings::DnsName =>{
                     
                let (hostname, startport, endport) = TransformDnsName::transform(in_data.att_val.text.clone().unwrap().0.trim().to_string());
                IntermediateAttributeValueType { 
                    data_type: in_data.att_val.data_type.clone(),
                att_id: in_data.id.clone(),
                    text: intermediate::IntermediateTypes::DnsName(
                        hostname, startport, endport
                    ), 
                    
                }
            },
            XacmlDataTypesStrings::HexBinary => {
                IntermediateAttributeValueType {
                    data_type: in_data.att_val.data_type.clone(),
                att_id: in_data.id.clone(),
                    text: intermediate::IntermediateTypes::HexBinary(
                        in_data.att_val.text.clone().unwrap().0.trim().to_string().into()
                    ), 
                    
                }
            },
            XacmlDataTypesStrings::IpAddress => {
                IntermediateAttributeValueType {
                    data_type: in_data.att_val.data_type.clone(),
                att_id: in_data.id.clone(),
                    text: intermediate::IntermediateTypes::IpAddress(
                        in_data.att_val.text.clone().unwrap().0.trim().to_string()
                    ), 
                    
                }
            },
            XacmlDataTypesStrings::Rfc822Name => {
                let s = in_data.att_val.text.clone().unwrap().0.trim().to_string();
                let splits: Vec<&str> = s.split("@").collect();
                let (local_part, domain_part) = (splits.get(0).unwrap().to_string(), splits.get(1).unwrap().to_string());
                IntermediateAttributeValueType {
                    data_type: in_data.att_val.data_type.clone(),
                    att_id: in_data.id.clone(),
                    text: intermediate::IntermediateTypes::Rfc822Name(
                        local_part, domain_part
                    ), 
                    
                }
            },
            XacmlDataTypesStrings::X500Name => {
                let result = TransformX500Name::transform(in_data.att_val.text.clone().unwrap().0.trim().to_string());

                IntermediateAttributeValueType {
                    data_type: in_data.att_val.data_type.clone(),
                att_id: in_data.id.clone(),
                    text: intermediate::IntermediateTypes::X500Name(
                        result
                    ), 
                    
                }
            },
            XacmlDataTypesStrings::AnyUri => {
                IntermediateAttributeValueType {
                    data_type: in_data.att_val.data_type.clone(),
                    att_id: in_data.id.clone(),
                    text: intermediate::IntermediateTypes::AnyUri(
                        in_data.att_val.text.clone().unwrap().0.trim().to_string()
                    ), 
                    
                }
            }
            //  TODO finish
        }
    }
}

pub struct TransformLoadout;
impl Transform<HashMap<Uuid, AttributeValueWithId>, HashMap<Uuid, IntermediateAttributeValueType>> for TransformLoadout {
    fn transform(in_data:HashMap<Uuid, AttributeValueWithId>) -> HashMap<Uuid, IntermediateAttributeValueType> {
        let mut result_hashmap: HashMap<Uuid, IntermediateAttributeValueType> = HashMap::new();
        for (key,value) in in_data.iter() {
            let intermediate_attribute_value = IntermediateAttributeValueType::transform(&value);
            result_hashmap.insert(key.to_owned(), intermediate_attribute_value);
        }
        result_hashmap
    }
}
