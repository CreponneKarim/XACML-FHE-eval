// src/bin/bench_tfhe_dispatch.rs
//
// This benchmark assumes:
// - your EncryptedTypes enum exists as shown
// - your evaluate_function_consume(urn, args) is available
// - your XacmlFunction enum has a .urn() or equivalent helper
//
// If you do not have XacmlFunction::urn(), I provide a small helper below
// where you can map each enum to its URN string manually.
//
// IMPORTANT:
// Some TFHE APIs for strings / byte arrays differ slightly across versions.
// The integer/bool/public-key pattern below follows TFHE-rs public-key docs,
// but you may need to adapt the exact constructors for:
//   - FheAsciiString
//   - CpuFheUint8Array
//
// Also: your dispatcher currently does NOT enable Base64Binary equality
// (the match arm is commented out), so this harness does not benchmark it.

use std::fs::OpenOptions;
use std::iter::zip;
use std::time::{Duration, Instant};

use csv::Writer;
use itertools::Itertools;
use serde::Deserialize;
use tfhe::{prelude::*, set_server_key};
use tfhe::{ConfigBuilder, PublicKey, generate_keys};

// Keep these imports only if your local TFHE version exposes them at crate root.
use tfhe::CpuFheUint8Array;
use tfhe::FheAsciiString;
use types::encryptor::types_extensions::EncryptType;
use types::homomorphic::funcs::evaluate_function_consume;
use types::homomorphic::policy::EncryptedTypes;
use types::homomorphic::types_impls::{
    HBoolean, HLongInteger, HNormalInteger, HUnsignedSmallInteger,
};
use types::xacml::constants::XacmlFunction;

#[derive(Deserialize)]
pub struct Config {
    polpath: String,
    remoteurl: String,
}
// use your_crate::homomorphic::policy::EncryptedTypes;
// use your_crate::homomorphic::types_impls::{
//     HBoolean,
//     HLongInteger,
//     HNormalInteger,
//     HUnsignedSmallInteger,
// };
// use your_crate::xacml::constants::XacmlFunction;
// use your_crate::evaluate_function_consume;

// ------------------------------------------------------------
// Small benchmark result structure
// ------------------------------------------------------------

#[derive(Debug)]
struct BenchRow {
    name: &'static str,
    iterations: usize,
    total: Duration,
    avg: Duration,
    stddev: Duration,
    rse: f64,
    opts: Option<OptionalBenchInfo>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
enum OptionalBenchInfo {
    StringBenchInfo(StringBenchInfo),
    HigherBaggingBenchInfo(HigherBaggingBenchInfo),
    HexBinaryBenchInfo(HexBinaryBenchInfo),
    Base64BinaryBenchInfo(Base64BinaryBenchInfo),
}
#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct StringBenchInfo {
    string_len: usize,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct HigherBaggingBenchInfo {
    first_bag_len: usize,
    second_bag_len: usize,

    func_used: String,

    second_bag_types_used: Vec<TypeUsed>,
    first_bag_types_used: Vec<TypeUsed>,
}
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct TypeUsed {
    t: String,
    s: Option<usize>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct HexBinaryBenchInfo {
    hexa_vars_nbs: usize,
}
#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Base64BinaryBenchInfo {
    string_len: usize,
}

fn bench_op<F>(
    name: &'static str,
    min_iters: usize,
    max_iters: usize,
    target_rse: f64,
    opts: Option<OptionalBenchInfo>,
    mut f: F,
) -> BenchRow
where
    F: FnMut(),
{
    let bench_start = Instant::now();

    let mut n = 0usize;
    let mut mean = 0.0f64;
    let mut m2 = 0.0f64;
    let mut total_nanos: u128 = 0;

    loop {
        let op_start = Instant::now();
        f();
        let sample = op_start.elapsed().as_nanos() as f64;

        total_nanos += sample as u128;
        n += 1;

        // Welford online mean/variance
        let delta = sample - mean;
        mean += delta / n as f64;
        let delta2 = sample - mean;
        m2 += delta * delta2;

        if n >= min_iters {
            let variance = if n > 1 { m2 / (n - 1) as f64 } else { 0.0 };
            let stddev = variance.sqrt();
            let stderr = stddev / (n as f64).sqrt();
            let rse = if mean > 0.0 {
                stderr / mean
            } else {
                f64::INFINITY
            };

            if rse <= target_rse || n >= max_iters {
                return BenchRow {
                    name,
                    iterations: n,
                    total: bench_start.elapsed(),
                    avg: Duration::from_nanos(mean as u64),
                    stddev: Duration::from_nanos(stddev as u64),
                    rse,
                    opts: opts,
                };
            }
        }

        if n >= max_iters {
            let variance = if n > 1 { m2 / (n - 1) as f64 } else { 0.0 };
            let stddev = variance.sqrt();
            let stderr = stddev / (n as f64).sqrt();
            let rse = if mean > 0.0 {
                stderr / mean
            } else {
                f64::INFINITY
            };

            return BenchRow {
                name,
                iterations: n,
                total: bench_start.elapsed(),
                avg: Duration::from_nanos(mean as u64),
                stddev: Duration::from_nanos(stddev as u64),
                rse,
                opts: opts,
            };
        }
    }
}

fn print_results(rows: &[BenchRow]) {
    println!(
        "{:<42} | {:>8} | {:>14} | {:>14} | {:>14} | {:>10}",
        "operation", "iters", "total(ms)", "avg(us)", "stddev(us)", "rse(%)"
    );
    println!("{}", "-".repeat(153));
    for r in rows {
        println!(
            "{:<70} | {:>8} | {:>14.3} | {:>14.3} | {:>14.3} | {:>10.3} | {:>14}",
            r.name,
            r.iterations,
            r.total.as_secs_f64() * 1_000.0,
            r.avg.as_secs_f64() * 1_000_000.0,
            r.stddev.as_secs_f64() * 1_000_000.0,
            r.rse * 100.0,
            serde_json::to_string(&r.opts).unwrap()
        );
    }
}

// ------------------------------------------------------------
// Encryption helpers
// ------------------------------------------------------------
//
// These helpers assume your wrapper types are aliases/newtypes over TFHE
// ciphertext types that expose try_encrypt(..., &public_key).
//
// If your wrapper constructors differ, only these functions need adjustment.
//

fn enc_bool(v: bool, pk: &PublicKey) -> EncryptedTypes {
    // Example if HBoolean = tfhe::FheBool or a wrapper around it
    let ct = HBoolean::try_encrypt(v, pk).expect("encrypt bool");
    EncryptedTypes::Boolean(ct)
}

fn enc_i64(v: i64, pk: &PublicKey) -> EncryptedTypes {
    let ct = HNormalInteger::try_encrypt(v, pk).expect("encrypt i64");
    EncryptedTypes::Integer(ct)
}

fn enc_double_encoded(v: i64, pk: &PublicKey) -> EncryptedTypes {
    // Your file says encrypted double => HLongInteger.
    // So this helper expects the VALUE ALREADY ENCODED as your fixed-point integer.
    let ct = HLongInteger::try_encrypt(v, pk).expect("encrypt encoded double");
    EncryptedTypes::Double(ct)
}

fn enc_time(v: u16, pk: &PublicKey) -> EncryptedTypes {
    let ct = HUnsignedSmallInteger::try_encrypt(v, pk).expect("encrypt time");
    EncryptedTypes::Time(ct)
}

fn enc_datetime(v: i64, pk: &PublicKey) -> EncryptedTypes {
    let ct = HNormalInteger::try_encrypt(v, pk).expect("encrypt datetime");
    EncryptedTypes::DateTime(ct)
}

fn enc_daytime_duration(v: i64, pk: &PublicKey) -> EncryptedTypes {
    let ct = HNormalInteger::try_encrypt(v, pk).expect("encrypt dayTimeDuration");
    EncryptedTypes::DayTimeDuration(ct)
}

fn enc_yearmonth_duration(v: i64, pk: &PublicKey) -> EncryptedTypes {
    let ct = HNormalInteger::try_encrypt(v, pk).expect("encrypt yearMonthDuration");
    EncryptedTypes::YearMonthDuration(ct)
}

fn enc_string(v: &str, pk: &PublicKey) -> EncryptedTypes {
    // Depending on your TFHE version this may be:
    //   FheAsciiString::try_encrypt(v, pk)
    // or a constructor on a wrapper type.
    let ct = FheAsciiString::try_encrypt(v, pk).expect("encrypt string");
    EncryptedTypes::String(ct)
}

fn enc_anyuri(v: &str, pk: &PublicKey) -> EncryptedTypes {
    let ct = FheAsciiString::try_encrypt(v, pk).expect("encrypt anyUri");
    EncryptedTypes::AnyUri(ct)
}

fn enc_ip(v: &str, pk: &PublicKey) -> EncryptedTypes {
    let ct = FheAsciiString::try_encrypt(v, pk).expect("encrypt ipAddress");
    EncryptedTypes::IpAddress(ct)
}

#[allow(dead_code)]
fn enc_base64_bytes(v: &[u8], pk: &PublicKey) -> EncryptedTypes {
    // Your dispatcher does not currently benchmark Base64Binary,
    // but this is the placeholder if you enable it later.
    //
    // Adjust to your local CpuFheUint8Array constructor/API.
    let ct = CpuFheUint8Array::try_encrypt(v, pk).expect("encrypt base64 bytes");
    EncryptedTypes::Base64Binary(ct)
}

fn append_to_csv(path: &str, row: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let file = OpenOptions::new().create(true).append(true).open(path)?;

    let mut wtr = Writer::from_writer(file);

    wtr.write_record(&row)?;
    wtr.flush()?; // ensure write to disk

    Ok(())
}
fn enc_hex_bytes(v: &[u8], pk: &PublicKey) -> EncryptedTypes {
    let ct = CpuFheUint8Array::try_encrypt(v, pk).expect("encrypt hex bytes");
    EncryptedTypes::HexBinary(ct)
}

fn enc_bag(v: Vec<EncryptedTypes>) -> EncryptedTypes {
    EncryptedTypes::Bag(v)
}

use types::homomorphic::types_impls::HRfc822Name;

fn enc_rfc822_name(local: &str, domain: &str, pk: &PublicKey) -> EncryptedTypes {
    EncryptedTypes::Rfc822Name(HRfc822Name {
        localpart: FheAsciiString::try_encrypt(local, pk).expect("encrypt localpart"),
        domainpart: FheAsciiString::try_encrypt(domain, pk).expect("encrypt domainpart"),
    })
}

use std::collections::HashMap;
use types::homomorphic::types_impls::HX500Name;

fn enc_x500_name(entries: &[(&str, &str)], pk: &PublicKey) -> EncryptedTypes {
    let mut map = HashMap::new();
    for (k, v) in entries.to_owned() {
        map.insert(
            (*k).to_string(),
            FheAsciiString::try_encrypt(v, pk).expect("encrypt x500 value"),
        );
    }

    EncryptedTypes::X500Name(HX500Name { val: map })
}

fn string_type_vec(count: usize, string_len: usize) -> Vec<TypeUsed> {
    (0..count)
        .map(|_| TypeUsed {
            t: "String".to_string(),
            s: Some(string_len),
        })
        .collect()
}

fn make_string_is_in_case_from_existing(
    string_len: usize,
    target: EncryptedTypes,
    other: EncryptedTypes,
) -> (EncryptedTypes, EncryptedTypes, Option<OptionalBenchInfo>) {
    let bag_len = 5;

    let bag = enc_bag(vec![
        other.clone(),
        other.clone(),
        target.clone(),
        other.clone(),
        other,
    ]);

    let info = Some(OptionalBenchInfo::HigherBaggingBenchInfo(
        HigherBaggingBenchInfo {
            first_bag_len: 1,
            second_bag_len: bag_len,
            func_used: "string-is-in".to_string(),
            first_bag_types_used: string_type_vec(1, string_len),
            second_bag_types_used: string_type_vec(bag_len, string_len),
        },
    ));

    (target, bag, info)
}

// ------------------------------------------------------------
// Runner
// ------------------------------------------------------------

pub fn bench_ops(min_iters: usize, max_iters: usize, rse: f64) {
    let min_iters: usize = min_iters;
    let max_iters: usize = max_iters;
    let rse: f64 = rse;

    let config = ConfigBuilder::default().build();
    let (client_key, _server_key) = generate_keys(config);

    set_server_key(_server_key);

    // Public-key encryption
    let public_key = PublicKey::new(&client_key);

    // -------------------------
    // Prepare encrypted samples
    // -------------------------

    // Bool
    let b_true = enc_bool(true, &public_key);
    let b_false = enc_bool(false, &public_key);

    // Integer
    let i_a = enc_i64(42, &public_key);
    let i_b = enc_i64(7, &public_key);
    let i_c = enc_i64(-32, &public_key);
    let i_d = enc_i64(30993, &public_key);
    let i_neg = enc_i64(-99, &public_key);

    // Double (already encoded as integer representation)
    let d_a = enc_double_encoded(4200, &public_key); // e.g. 42.00 with scale=100
    let d_b = enc_double_encoded(700, &public_key); // e.g. 7.00
    let d_neg = enc_double_encoded(-9900, &public_key);

    // Time
    let t_now = enc_time(14 * 60 + 30, &public_key); // 14:30 encoded as minutes
    let t_start = enc_time(9 * 60, &public_key); // 09:00
    let t_end = enc_time(18 * 60, &public_key); // 18:00

    // DateTime and durations
    let dt_a = enc_datetime(1_700_000_000, &public_key);
    let dt_b = enc_datetime(1_700_100_000, &public_key);
    let dur_day = enc_daytime_duration(86_400, &public_key);
    let dur_month = enc_yearmonth_duration(3, &public_key);

    // Strings
    // let s_hello = enc_string("hello", &public_key);
    // let s_world = enc_string("world", &public_key);
    // let s_hello_caps = enc_string("HELLO", &public_key);
    // let s_hello_world = enc_string("hello world", &public_key);
    // let s_he = enc_string("he", &public_key);
    // let s_lo = enc_string("lo", &public_key);

    let a_5 = enc_string("ze1dt", &public_key);
    let b_5 = enc_string("0ldkc", &public_key);

    let a_10 = enc_string("ze1dtyalkc", &public_key);
    let b_10 = enc_string("0ldkcmamtg", &public_key);
    let c_10 = enc_string("zb1dhyalkc", &public_key);
    let d_10 = enc_string("32dkcmamtg", &public_key);
    let e_10 = enc_string("0ldkd3omtg", &public_key);

    let a_15 = enc_string("ze1dtyalkcl23ca", &public_key);
    let b_15 = enc_string("0ldkcmamtghtndc", &public_key);

    let a_20 = enc_string("ze1dtyalkcl23caybfpd", &public_key);
    let b_20 = enc_string("0ldkcmamtghtndcbsldf", &public_key);

    let encrypted_string_pairs = zip(
        vec![a_5.clone(), a_10.clone(), a_15.clone(), a_20.clone()],
        vec![b_5.clone(), b_10.clone(), b_15.clone(), b_20.clone()],
    );

    //	bags
    let (string_is_in_target_5, string_is_in_bag_5, string_is_in_info_5) =
        make_string_is_in_case_from_existing(5, a_5.clone(), b_5.clone());
    
    let (string_is_in_target_10, string_is_in_bag_10, string_is_in_info_10) =
        make_string_is_in_case_from_existing(10, a_10.clone(), b_10.clone());
    
    let (string_is_in_target_15, string_is_in_bag_15, string_is_in_info_15) =
        make_string_is_in_case_from_existing(15, a_15.clone(), b_15.clone());
    
    let (string_is_in_target_20, string_is_in_bag_20, string_is_in_info_20) =
        make_string_is_in_case_from_existing(20, a_20.clone(), b_20.clone());
    
    // AnyUri
    let uri_a = enc_anyuri("https://example.com/docs/index.html", &public_key);
    let uri_b = enc_anyuri("https://example.com/docs/", &public_key);
    let uri_fragment = enc_string("/docs/", &public_key);

    // IpAddress example values if/when you add operators for them
    let _ip_a = enc_ip("192.168.1.10", &public_key);
    let _ip_b = enc_ip("192.168.1.11", &public_key);

    // -------------------------
    // Additional samples for missing benchmarks
    // -------------------------

    let rfc_a = enc_rfc822_name("alice", "example.com", &public_key);
    let rfc_b = enc_rfc822_name("alice", "example.com", &public_key);

    let x500_a = enc_x500_name(
        &[("CN", "Alice"), ("O", "ExampleCorp"), ("C", "FR")],
        &public_key,
    );

    let x500_b = enc_x500_name(
        &[("CN", "Alice"), ("O", "ExampleCorp"), ("C", "FR")],
        &public_key,
    );

    let base64_a = enc_base64_bytes(b"hello-world", &public_key);
    let base64_b = enc_base64_bytes(b"hello-world", &public_key);
    let base64_info = Some(OptionalBenchInfo::Base64BinaryBenchInfo(
        Base64BinaryBenchInfo { string_len: 11 },
    ));

    let hex_a = enc_hex_bytes(&[0xAA, 0xBB, 0xCC, 0xDD], &public_key);
    let hex_b = enc_hex_bytes(&[0xAA, 0xBB, 0xCC, 0xDD], &public_key);
    let hex_info = Some(OptionalBenchInfo::Base64BinaryBenchInfo(
        Base64BinaryBenchInfo { string_len: 4 },
    ));

    // Bags for higher-order functions using StringEqual as inner predicate
    let bag_any = enc_bag(vec![b_10.clone(), a_10.clone(), c_10.clone(), d_10.clone(), e_10.clone()]);
    let bag_any_len = 5;
    let bag_any_types = vec![
        TypeUsed {
            t: "String".to_string(),
            s: Some(10),
        },
        TypeUsed {
            t: "String".to_string(),
            s: Some(10),
        },
        TypeUsed {
            t: "String".to_string(),
            s: Some(10),
        },
        TypeUsed {
            t: "String".to_string(),
            s: Some(10),
        },
        TypeUsed {
            t: "String".to_string(),
            s: Some(10),
        },
    ];
    
    let bag_any_integer = enc_bag(vec![i_a.clone(), i_b.clone(), i_c.clone(), i_d.clone(), i_neg.clone()]);
    let bag_any_integer_len = 5;
    let bag_any_integer_types = vec![
        TypeUsed {
            t: "Integer".to_string(),
            s: None,
        },
        TypeUsed {
            t: "Integer".to_string(),
            s: None,
        },
        TypeUsed {
            t: "Integer".to_string(),
            s: None,
        },
        TypeUsed {
            t: "Integer".to_string(),
            s: None,
        },
        TypeUsed {
            t: "Integer".to_string(),
            s: None,
        },
    ];

    let bag_all =enc_bag(vec![b_10.clone(), a_10.clone(), c_10.clone(), d_10.clone(), e_10.clone()]);
    let bag_all_len = 5;
    let bag_all_types = vec![
        TypeUsed {
            t: "String".to_string(),
            s: Some(10),
        },
        TypeUsed {
            t: "String".to_string(),
            s: Some(10),
        },
        TypeUsed {
            t: "String".to_string(),
            s: Some(10),
        },
        TypeUsed {
            t: "String".to_string(),
            s: Some(10),
        },
        TypeUsed {
            t: "String".to_string(),
            s: Some(10),
        },
    ];
    
    let bag_all_integer = enc_bag(vec![i_a.clone(), i_b.clone(), i_c.clone(), i_d.clone(), i_neg.clone()]);
    let bag_all_integer_len = 5;
    let bag_all_integer_types = vec![
        TypeUsed {
            t: "Integer".to_string(),
            s: None,
        },
        TypeUsed {
            t: "Integer".to_string(),
            s: None,
        },
        TypeUsed {
            t: "Integer".to_string(),
            s: None,
        },
        TypeUsed {
            t: "Integer".to_string(),
            s: None,
        },
        TypeUsed {
            t: "Integer".to_string(),
            s: None,
        },
    ];

    // let bag_left = enc_bag(vec![a_5.clone(), a_10.clone()]);
    // let bag_left_len = 2;
    // let bag_left_types = vec![
    //     TypeUsed {
    //         t: "String".to_string(),
    //         s: Some(5),
    //     },
    //     TypeUsed {
    //         t: "String".to_string(),
    //         s: Some(10),
    //     },
    // ];

    // let bag_right_superset = enc_bag(vec![a_5.clone(), a_10.clone(), b_15.clone()]);
    // let bag_right_superset_len = 3;
    // let bag_right_superset_types = vec![
    //     TypeUsed {
    //         t: "String".to_string(),
    //         s: Some(5),
    //     },
    //     TypeUsed {
    //         t: "String".to_string(),
    //         s: Some(10),
    //     },
    //     TypeUsed {
    //         t: "String".to_string(),
    //         s: Some(15),
    //     },
    // ];

    // let bag_right_subset = enc_bag(vec![a_5.clone(), a_10.clone()]);
    // let bag_right_subset_len = 2;
    // let bag_right_subset_types = vec![
    //     TypeUsed {
    //         t: "String".to_string(),
    //         s: Some(5),
    //     },
    //     TypeUsed {
    //         t: "String".to_string(),
    //         s: Some(10),
    //     },
    // ];

    // let bag_all_left = enc_bag(vec![a_5.clone(), a_5.clone()]);
    // let bag_all_left_len = 2;
    // let bag_all_left_types = vec![
    //     TypeUsed {
    //         t: "String".to_string(),
    //         s: Some(5),
    //     },
    //     TypeUsed {
    //         t: "String".to_string(),
    //         s: Some(5),
    //     },
    // ];

    // let bag_all_right = enc_bag(vec![a_5.clone(), a_5.clone()]);
    // let bag_all_right_len = 2;
    // let bag_all_right_types = vec![
    //     TypeUsed {
    //         t: "String".to_string(),
    //         s: Some(5),
    //     },
    //     TypeUsed {
    //         t: "String".to_string(),
    //         s: Some(5),
    //     },
    // ];

    // -------------------------
    // Benchmark configuration
    // -------------------------
    let mut rows = Vec::new();

    // Tune as you wish
    // let iters_arith =  iterations;//10;
    // let iters_cmp =  iterations;//10;
    // let iters_strings =  iterations;//5;

    // -------------------------
    // Integer ops
    // -------------------------
    rows.push(bench_op(
        "integer-equal",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::IntegerEqual.urn(),
                None,
                vec![Some(i_a.clone()), Some(i_b.clone())],
            )
            .unwrap();
        },
    ));
    println!("> benching integer comparison operators");
    for f in [
        XacmlFunction::IntegerGreaterThan,
        XacmlFunction::IntegerGreaterThanOrEqual,
        XacmlFunction::IntegerLessThan,
        XacmlFunction::IntegerLessThanOrEqual,
    ] {
        rows.push(bench_op(f.urn().split(':').collect::<Vec<_>>().last().unwrap(), min_iters, max_iters, rse, None, || {
            let _ = evaluate_function_consume(
                f.urn(),
                None,
                vec![Some(i_a.clone()), Some(i_b.clone())],
            )
            .unwrap();
        }));
    }

    println!("> benching integer arithmetic operators");
    for f in [
        XacmlFunction::IntegerAdd,
        XacmlFunction::IntegerSubtract,
        XacmlFunction::IntegerMultiply,
        XacmlFunction::IntegerDivide,
    ] {
        rows.push(bench_op(f.urn().split(':').collect::<Vec<_>>().last().unwrap(), min_iters, max_iters, rse, None, || {
            let _ = evaluate_function_consume(
                f.urn(),
                None,
                vec![Some(i_a.clone()), Some(i_b.clone())],
            )
            .unwrap();
        }));
    }

    rows.push(bench_op(
        "integer-abs",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::IntegerAbs.urn(),
                None,
                vec![Some(i_neg.clone())],
            )
            .unwrap();
        },
    ));

    // -------------------------
    // Boolean ops
    // -------------------------

    println!("> benching boolean operators");
    rows.push(bench_op("not", min_iters, max_iters, rse, None, || {
        let _ =
            evaluate_function_consume(XacmlFunction::Not.urn(), None, vec![Some(b_true.clone())])
                .unwrap();
    }));

    for f in [
        XacmlFunction::And,
        XacmlFunction::Or,
        XacmlFunction::BooleanEqual,
    ] {
        rows.push(bench_op(f.urn().split(':').collect::<Vec<_>>().last().unwrap(), min_iters, max_iters, rse, None, || {
            let _ = evaluate_function_consume(
                f.urn(),
                None,
                vec![Some(b_true.clone()), Some(b_false.clone())],
            )
            .unwrap();
        }));
    }

    rows.push(bench_op("if-else", min_iters, max_iters, rse, None, || {
        let _ = evaluate_function_consume(
            XacmlFunction::IfElse.urn(),
            None,
            vec![
                Some(b_true.clone()),
                Some(b_false.clone()),
                Some(b_true.clone()),
            ],
        )
        .unwrap();
    }));

    // -------------------------
    // Double ops
    // -------------------------

    println!("> benching double operators");
    rows.push(bench_op(
        "double-equal",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::DoubleEqual.urn(),
                None,
                vec![Some(d_a.clone()), Some(d_b.clone())],
            )
            .unwrap();
        },
    ));

    for f in [
        XacmlFunction::DoubleGreaterThan,
        XacmlFunction::DoubleGreaterThanOrEqual,
        XacmlFunction::DoubleLessThan,
        XacmlFunction::DoubleLessThanOrEqual,
    ] {
        rows.push(bench_op(f.urn().split(':').collect::<Vec<_>>().last().unwrap(), min_iters, max_iters, rse, None, || {
            let _ = evaluate_function_consume(
                f.urn(),
                None,
                vec![Some(d_a.clone()), Some(d_b.clone())],
            )
            .unwrap();
        }));
    }

    for f in [
        XacmlFunction::DoubleAdd,
        XacmlFunction::DoubleSubtract,
        XacmlFunction::DoubleMultiply,
        XacmlFunction::DoubleDivide,
    ] {
        rows.push(bench_op(f.urn().split(':').collect::<Vec<_>>().last().unwrap(), min_iters, max_iters, rse, None, || {
            let _ = evaluate_function_consume(
                f.urn(),
                None,
                vec![Some(d_a.clone()), Some(d_b.clone())],
            )
            .unwrap();
        }));
    }

    rows.push(bench_op(
        "double-abs",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::DoubleAbs.urn(),
                None,
                vec![Some(d_neg.clone())],
            )
            .unwrap();
        },
    ));

    // -------------------------
    // Time ops
    // -------------------------

    println!("> benching Time operators");
    rows.push(bench_op(
        "time-equal",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::TimeEqual.urn(),
                None,
                vec![Some(t_now.clone()), Some(t_start.clone())],
            )
            .unwrap();
        },
    ));

    for f in [
        XacmlFunction::TimeGreaterThan,
        XacmlFunction::TimeGreaterThanOrEqual,
        XacmlFunction::TimeLessThan,
        XacmlFunction::TimeLessThanOrEqual,
    ] {
        rows.push(bench_op(f.urn().split(':').collect::<Vec<_>>().last().unwrap(), min_iters, max_iters, rse, None, || {
            let _ = evaluate_function_consume(
                f.urn(),
                None,
                vec![Some(t_now.clone()), Some(t_start.clone())],
            )
            .unwrap();
        }));
    }

    rows.push(bench_op(
        "time-in-range",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::TimeInRange.urn(),
                None,
                vec![
                    Some(t_now.clone()),
                    Some(t_start.clone()),
                    Some(t_end.clone()),
                ],
            )
            .unwrap();
        },
    ));

    // -------------------------
    // DateTime ops
    // -------------------------

    println!("> benching DateTime operators");
    rows.push(bench_op(
        "dateTime-equal",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::DateTimeEqual.urn(),
                None,
                vec![Some(dt_a.clone()), Some(dt_b.clone())],
            )
            .unwrap();
        },
    ));

    for f in [
        XacmlFunction::DateTimeGreaterThan,
        XacmlFunction::DateTimeGreaterThanOrEqual,
        XacmlFunction::DateTimeLessThan,
        XacmlFunction::DateTimeLessThanOrEqual,
    ] {
        rows.push(bench_op(f.urn().split(':').collect::<Vec<_>>().last().unwrap(), min_iters, max_iters, rse, None, || {
            let _ = evaluate_function_consume(
                f.urn(),
                None,
                vec![Some(dt_a.clone()), Some(dt_b.clone())],
            )
            .unwrap();
        }));
    }

    for f in [
        XacmlFunction::DateTimeAddDayTimeDuration,
        XacmlFunction::DateTimeSubtractDayTimeDuration,
    ] {
        rows.push(bench_op(f.urn().split(':').collect::<Vec<_>>().last().unwrap(), min_iters, max_iters, rse, None, || {
            let _ = evaluate_function_consume(
                f.urn(),
                None,
                vec![Some(dt_a.clone()), Some(dur_day.clone())],
            )
            .unwrap();
        }));
    }

    for f in [
        XacmlFunction::DateTimeAddYearMonthDuration,
        XacmlFunction::DateTimeSubtractYearMonthDuration,
    ] {
        rows.push(bench_op(f.urn().split(':').collect::<Vec<_>>().last().unwrap(), min_iters, max_iters, rse, None, || {
            let _ = evaluate_function_consume(
                f.urn(),
                None,
                vec![Some(dt_a.clone()), Some(dur_month.clone())],
            )
            .unwrap();
        }));
    }

    // -------------------------
    // Duration equality ops
    // -------------------------
    println!("> benching Duration operators");
    rows.push(bench_op(
        "dayTimeDuration-equal",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::DayTimeDurationEqual.urn(),
                None,
                vec![Some(dur_day.clone()), Some(dur_day.clone())],
            )
            .unwrap();
        },
    ));

    rows.push(bench_op(
        "yearMonthDuration-equal",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::YearMonthDurationEqual.urn(),
                None,
                vec![Some(dur_month.clone()), Some(dur_month.clone())],
            )
            .unwrap();
        },
    ));

    // -------------------------
    // String ops
    // -------------------------
    println!("> benching String operators");
    for (a, b) in encrypted_string_pairs {
        rows.push(bench_op(
            "string-equal",
            min_iters,
            max_iters,
            rse,
            None,
            || {
                let _ = evaluate_function_consume(
                    XacmlFunction::StringEqual.urn(),
                    None,
                    vec![Some(a.clone()), Some(b.clone())],
                )
                .unwrap();
            },
        ));

        rows.push(bench_op(
            "string-equal-ignore-case",
            min_iters,
            max_iters,
            rse,
            None,
            || {
                let _ = evaluate_function_consume(
                    XacmlFunction::StringEqualIgnoreCase.urn(),
                    None,
                    vec![Some(a.clone()), Some(b.clone())],
                )
                .unwrap();
            },
        ));

        rows.push(bench_op(
            "string-concatenate",
            min_iters,
            max_iters,
            rse,
            None,
            || {
                let _ = evaluate_function_consume(
                    XacmlFunction::StringConcatenate.urn(),
                    None,
                    vec![Some(a.clone()), Some(b.clone())],
                )
                .unwrap();
            },
        ));

        rows.push(bench_op(
            "string-contains",
            min_iters,
            max_iters,
            rse,
            None,
            || {
                let _ = evaluate_function_consume(
                    XacmlFunction::StringContains.urn(),
                    None,
                    vec![Some(a.clone()), Some(b.clone())],
                )
                .unwrap();
            },
        ));

        rows.push(bench_op(
            "string-starts-with",
            min_iters,
            max_iters,
            rse,
            None,
            || {
                let _ = evaluate_function_consume(
                    XacmlFunction::StringStartsWith.urn(),
                    None,
                    vec![Some(a.clone()), Some(b.clone())],
                )
                .unwrap();
            },
        ));

        rows.push(bench_op(
            "string-ends-with",
            min_iters,
            max_iters,
            rse,
            None,
            || {
                let _ = evaluate_function_consume(
                    XacmlFunction::StringEndsWith.urn(),
                    None,
                    vec![Some(a.clone()), Some(b.clone())],
                )
                .unwrap();
            },
        ));
    }

    // -------------------------
    // AnyUri ops
    // -------------------------
    println!("> benching AnyUri operators");
    rows.push(bench_op(
        "anyURI-equal",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::AnyUriEqual.urn(),
                None,
                vec![Some(uri_a.clone()), Some(uri_b.clone())],
            )
            .unwrap();
        },
    ));

    rows.push(bench_op(
        "anyURI-contains",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::AnyUriContains.urn(),
                None,
                vec![Some(uri_a.clone()), Some(uri_fragment.clone())],
            )
            .unwrap();
        },
    ));

    rows.push(bench_op(
        "anyURI-starts-with",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::AnyUriStartsWith.urn(),
                None,
                vec![Some(uri_a.clone()), Some(uri_b.clone())],
            )
            .unwrap();
        },
    ));

    rows.push(bench_op(
        "anyURI-ends-with",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::AnyUriEndsWith.urn(),
                None,
                vec![
                    Some(uri_a.clone()),
                    Some(enc_string("index.html", &public_key)),
                ],
            )
            .unwrap();
        },
    ));
    // -------------------------
    // Rfc822Name ops
    // -------------------------
    println!("> benching Rfc822Name operators");
    rows.push(bench_op(
        "rfc822Name-equal",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::Rfc822NameEqual.urn(),
                None,
                vec![Some(rfc_a.clone()), Some(rfc_b.clone())],
            )
            .unwrap();
        },
    ));

    // -------------------------
    // X500Name ops
    // -------------------------
    println!("> benching X500Name operators");
    rows.push(bench_op(
        "x500Name-equal",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::X500NameEqual.urn(),
                None,
                vec![Some(x500_a.clone()), Some(x500_b.clone())],
            )
            .unwrap();
        },
    ));

    // -------------------------
    // Binary blob ops
    // -------------------------
    println!("> benching binary equality operators");

    rows.push(bench_op(
        "base64Binary-equal",
        min_iters,
        max_iters,
        rse,
        base64_info,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::Base64BinaryEqual.urn(),
                None,
                vec![Some(base64_a.clone()), Some(base64_b.clone())],
            )
            .unwrap();
        },
    ));

    rows.push(bench_op(
        "hexBinary-equal",
        min_iters,
        max_iters,
        rse,
        hex_info,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::HexBinaryEqual.urn(),
                None,
                vec![Some(hex_a.clone()), Some(hex_b.clone())],
            )
            .unwrap();
        },
    ));

    // -------------------------
    // Higher-order bag ops
    // -------------------------
    println!("> benching higher-order bag operators");

    // any-of(string-equal, value, bag)
    let anyof_info = Some(OptionalBenchInfo::HigherBaggingBenchInfo(
        HigherBaggingBenchInfo {
            first_bag_len: 1,
            second_bag_len: bag_any_len,
            func_used: "any-of(string-equal)".to_string(),
            first_bag_types_used: vec![TypeUsed {
                t: "String".to_string(),
                s: Some(10),
            }],
            second_bag_types_used: bag_any_types.clone(),
        },
    ));
    rows.push(bench_op(
        "any-of(string-equal)",
        min_iters,
        max_iters,
        rse,
        anyof_info,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::AnyOf(None).urn(),
                Some(XacmlFunction::StringEqual.urn()),
                vec![Some(e_10.clone()), Some(bag_any.clone())],
            )
            .unwrap();
        },
    ));

    // all-of(string-equal, value, bag)
    let allof_info = Some(OptionalBenchInfo::HigherBaggingBenchInfo(
        HigherBaggingBenchInfo {
            first_bag_len: 1,
            second_bag_len: bag_all_len,
            func_used: "all-of(string-equal)".to_string(),
            first_bag_types_used: vec![TypeUsed {
                t: "String".to_string(),
                s: Some(10),
            }],
            second_bag_types_used: bag_all_types.clone(),
        },
    ));

    rows.push(bench_op(
        "all-of(string-equal)",
        min_iters,
        max_iters,
        rse,
        allof_info,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::AllOf(None).urn(),
                Some(XacmlFunction::StringEqual.urn()),
                vec![Some(d_10.clone()), Some(bag_all.clone())],
            )
            .unwrap();
        },
    ));

    // any-of-any(string-equal, bag1, bag2)
    let anyofany_info = Some(OptionalBenchInfo::HigherBaggingBenchInfo(
        HigherBaggingBenchInfo {
            first_bag_len: bag_any_len.clone(),
            second_bag_len: bag_all_len.clone(),
            func_used: "any-of-any(string-equal)".to_string(),
            first_bag_types_used: bag_any_types.clone(),
            second_bag_types_used: bag_all_types.clone(),
        },
    ));

    rows.push(bench_op(
        "any-of-any(string-equal)",
        min_iters,
        max_iters,
        rse,
        anyofany_info,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::AnyOfAny(None).urn(),
                Some(XacmlFunction::StringEqual.urn()),
                vec![Some(bag_any.clone()), Some(bag_all.clone())],
            )
            .unwrap();
        },
    ));

    // all-of-any(string-equal, bag1, bag2)
    let allofany_info = Some(OptionalBenchInfo::HigherBaggingBenchInfo(
        HigherBaggingBenchInfo {
	        first_bag_len: bag_any_len.clone(),
	        second_bag_len: bag_all_len.clone(),
	        func_used: "any-of-any(string-equal)".to_string(),
	        first_bag_types_used: bag_any_types.clone(),
	        second_bag_types_used: bag_all_types.clone(),
        },
    ));

    rows.push(bench_op(
        "all-of-any(string-equal)",
        min_iters,
        max_iters,
        rse,
        allofany_info,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::AllOfAny(None).urn(),
                Some(XacmlFunction::StringEqual.urn()),
                vec![Some(bag_any.clone()), Some(bag_all.clone())],
            )
            .unwrap();
        },
    ));

    // any-of-all(string-equal, bag1, bag2)
    let anyofall_info = Some(OptionalBenchInfo::HigherBaggingBenchInfo(
        HigherBaggingBenchInfo {
        	first_bag_len: bag_any_len.clone(),
	        second_bag_len: bag_all_len.clone(),
	        func_used: "any-of-any(string-equal)".to_string(),
	        first_bag_types_used: bag_any_types.clone(),
	        second_bag_types_used: bag_all_types.clone(),
        },
    ));

    rows.push(bench_op(
        "any-of-all(string-equal)",
        min_iters,
        max_iters,
        rse,
        anyofall_info,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::AnyOfAll(None).urn(),
                Some(XacmlFunction::StringEqual.urn()),
                vec![Some(bag_any.clone()), Some(bag_all.clone())],
            )
            .unwrap();
        },
    ));

    // all-of-all(string-equal, bag1, bag2)
    let allofall_info = Some(OptionalBenchInfo::HigherBaggingBenchInfo(
        HigherBaggingBenchInfo {
       	first_bag_len: bag_any_len.clone(),
	        second_bag_len: bag_all_len.clone(),
	        func_used: "any-of-any(string-equal)".to_string(),
	        first_bag_types_used: bag_any_types.clone(),
	        second_bag_types_used: bag_all_types.clone(),
        },
    ));
    rows.push(bench_op(
        "all-of-all(string-equal)",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::AllOfAll(None).urn(),
                Some(XacmlFunction::StringEqual.urn()),
                vec![Some(bag_any.clone()), Some(bag_all.clone())],
            )
            .unwrap();
        },
    ));
    
    //	for integers now ---------------------------------
    // any-of(string-equal, value, bag)
    let anyof_info = Some(OptionalBenchInfo::HigherBaggingBenchInfo(
        HigherBaggingBenchInfo {
            first_bag_len: 1,
            second_bag_len: bag_any_integer_len,
            func_used: "any-of(integer-equal)".to_string(),
            first_bag_types_used: vec![TypeUsed {
                t: "Integer".to_string(),
                s: None,
            }],
            second_bag_types_used: bag_any_integer_types.clone(),
        },
    ));
    rows.push(bench_op(
        "any-of(integer-equal)",
        min_iters,
        max_iters,
        rse,
        anyof_info,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::AnyOf(None).urn(),
                Some(XacmlFunction::IntegerEqual.urn()),
                vec![Some(i_c.clone()), Some(bag_any_integer.clone())],
            )
            .unwrap();
        },
    ));

    // all-of(string-equal, value, bag)
    let allof_info = Some(OptionalBenchInfo::HigherBaggingBenchInfo(
        HigherBaggingBenchInfo {
            first_bag_len: 1,
            second_bag_len: bag_all_integer_len,
            func_used: "all-of(integer-equal)".to_string(),
            first_bag_types_used: vec![TypeUsed {
                t: "Integer".to_string(),
                s: None,
            }],
            second_bag_types_used: bag_all_integer_types.clone(),
        },
    ));

    rows.push(bench_op(
        "all-of(integer-equal)",
        min_iters,
        max_iters,
        rse,
        allof_info,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::AllOf(None).urn(),
                Some(XacmlFunction::IntegerEqual.urn()),
                vec![Some(i_d.clone()), Some(bag_all_integer.clone())],
            )
            .unwrap();
        },
    ));

    // any-of-any(string-equal, bag1, bag2)
    let anyofany_info = Some(OptionalBenchInfo::HigherBaggingBenchInfo(
        HigherBaggingBenchInfo {
            first_bag_len: bag_any_integer_len.clone(),
            second_bag_len: bag_all_integer_len.clone(),
            func_used: "any-of-any(integer-equal)".to_string(),
            first_bag_types_used: bag_any_integer_types.clone(),
            second_bag_types_used: bag_all_integer_types.clone(),
        },
    ));

    rows.push(bench_op(
        "any-of-any(integer-equal)",
        min_iters,
        max_iters,
        rse,
        anyofany_info,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::AnyOfAny(None).urn(),
                Some(XacmlFunction::IntegerEqual.urn()),
                vec![Some(bag_any_integer.clone()), Some(bag_all_integer.clone())],
            )
            .unwrap();
        },
    ));

    // all-of-any(string-equal, bag1, bag2)
    let allofany_info = Some(OptionalBenchInfo::HigherBaggingBenchInfo(
        HigherBaggingBenchInfo {
	        first_bag_len: bag_any_integer_len.clone(),
	        second_bag_len: bag_all_integer_len.clone(),
	        func_used: "any-of-any(integer-equal)".to_string(),
	        first_bag_types_used: bag_any_integer_types.clone(),
	        second_bag_types_used: bag_all_integer_types.clone(),
        },
    ));

    rows.push(bench_op(
        "all-of-any(integer-equal)",
        min_iters,
        max_iters,
        rse,
        allofany_info,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::AllOfAny(None).urn(),
                Some(XacmlFunction::IntegerEqual.urn()),
                vec![Some(bag_any_integer.clone()), Some(bag_all_integer.clone())],
            )
            .unwrap();
        },
    ));

    // any-of-all(string-equal, bag1, bag2)
    let anyofall_info = Some(OptionalBenchInfo::HigherBaggingBenchInfo(
        HigherBaggingBenchInfo {
        	first_bag_len: bag_any_integer_len.clone(),
	        second_bag_len: bag_all_integer_len.clone(),
	        func_used: "any-of-any(integer-equal)".to_string(),
	        first_bag_types_used: bag_any_integer_types.clone(),
	        second_bag_types_used: bag_all_integer_types.clone(),
        },
    ));

    rows.push(bench_op(
        "any-of-all(integer-equal)",
        min_iters,
        max_iters,
        rse,
        anyofall_info,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::AnyOfAll(None).urn(),
                Some(XacmlFunction::IntegerEqual.urn()),
                vec![Some(bag_any_integer.clone()), Some(bag_all_integer.clone())],
            )
            .unwrap();
        },
    ));

    // all-of-all(string-equal, bag1, bag2)
    let allofall_info = Some(OptionalBenchInfo::HigherBaggingBenchInfo(
        HigherBaggingBenchInfo {
       	first_bag_len: bag_any_integer_len.clone(),
	        second_bag_len: bag_all_integer_len.clone(),
	        func_used: "any-of-any(integer-equal)".to_string(),
	        first_bag_types_used: bag_any_integer_types.clone(),
	        second_bag_types_used: bag_all_integer_types.clone(),
        },
    ));
    rows.push(bench_op(
        "all-of-all(integer-equal)",
        min_iters,
        max_iters,
        rse,
        None,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::AllOfAll(None).urn(),
                Some(XacmlFunction::IntegerEqual.urn()),
                vec![Some(bag_any_integer.clone()), Some(bag_all_integer.clone())],
            )
            .unwrap();
        },
    ));
    
    // -------------------------
    // String bagging ops
    // -------------------------
    println!("> benching String bagging operators");
    
    rows.push(bench_op(
        "string-is-in-5-char-strings",
        min_iters,
        max_iters,
        rse,
        string_is_in_info_5,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::StringIsIn.urn(),
                None,
                vec![
                    Some(string_is_in_target_5.clone()),
                    Some(string_is_in_bag_5.clone()),
                ],
            )
            .unwrap();
        },
    ));
    
    rows.push(bench_op(
        "string-is-in-10-char-strings",
        min_iters,
        max_iters,
        rse,
        string_is_in_info_10,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::StringIsIn.urn(),
                None,
                vec![
                    Some(string_is_in_target_10.clone()),
                    Some(string_is_in_bag_10.clone()),
                ],
            )
            .unwrap();
        },
    ));
    
    rows.push(bench_op(
        "string-is-in-15-char-strings",
        min_iters,
        max_iters,
        rse,
        string_is_in_info_15,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::StringIsIn.urn(),
                None,
                vec![
                    Some(string_is_in_target_15.clone()),
                    Some(string_is_in_bag_15.clone()),
                ],
            )
            .unwrap();
        },
    ));
    
    rows.push(bench_op(
        "string-is-in-20-char-strings",
        min_iters,
        max_iters,
        rse,
        string_is_in_info_20,
        || {
            let _ = evaluate_function_consume(
                XacmlFunction::StringIsIn.urn(),
                None,
                vec![
                    Some(string_is_in_target_20.clone()),
                    Some(string_is_in_bag_20.clone()),
                ],
            )
            .unwrap();
        },
    ));

    // -------------------------
    // Print
    // -------------------------
    append_to_csv(
        "./metrics_ops.csv",
        vec![
            "name".to_string(),
            "iterations".to_string(),
            "total_us".to_string(),
            "avg_us".to_string(),
            "stddev_us".to_string(),
            "rse_percent".to_string(),
            "options".to_string(),
        ],
    )
    .unwrap();

    for r in &rows {
        append_to_csv(
            "./metrics_ops.csv",
            vec![
                r.name.to_string(),
                r.iterations.to_string(),
                r.total.as_micros().to_string(),
                r.avg.as_micros().to_string(),
                r.stddev.as_micros().to_string(),
                format!("{:.6}", r.rse * 100.0),
                serde_json::to_string(&r.opts).unwrap(),
            ],
        )
        .unwrap();
    }

    print_results(&rows);
}
