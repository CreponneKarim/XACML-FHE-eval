/// TODO for the future 

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};
use types::homomorphic::types_traits::HXacmlSpecializedTypes::{
    HAnyUriType,
    HBase64BinaryType,
    HDateTimeType,
    HDayTimeDurationType,
    HDnsNameType,
    HHexBinaryType,
    HIpAddressType,
    HRfc822NameType,
    HTimeType,
    HX500NameType,
    HYearMonthDurationType,};
use quote::quote;

#[proc_macro_derive(HTimeType)]
pub fn derive_h_time_type(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // Generate an impl that implements type_name::TypeName for the type
    // We refer to the trait via the crate name `type_name` (path dependency in Cargo.toml).
    let expanded = quote! {
        impl HTimeType for #name {
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(HDateTimeType)]
pub fn derive_h_date_time_type(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // Generate an impl that implements type_name::TypeName for the type
    // We refer to the trait via the crate name `type_name` (path dependency in Cargo.toml).
    let expanded = quote! {
        impl HDateTimeType for #name {
        }
    };

    TokenStream::from(expanded)
}


#[proc_macro_derive(HDayTimeDurationType)]
pub fn derive_h_day_time_duration_type(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // Generate an impl that implements type_name::TypeName for the type
    // We refer to the trait via the crate name `type_name` (path dependency in Cargo.toml).
    let expanded = quote! {
        impl HDayTimeDurationType for #name {
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(HAnyUriType)]
pub fn derive_h_any_uri_type(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // Generate an impl that implements type_name::TypeName for the type
    // We refer to the trait via the crate name `type_name` (path dependency in Cargo.toml).
    let expanded = quote! {
        impl HAnyUriType for #name {
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(HBase64BinaryType)]
pub fn derive_h_base_64_binary_type(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // Generate an impl that implements type_name::TypeName for the type
    // We refer to the trait via the crate name `type_name` (path dependency in Cargo.toml).
    let expanded = quote! {
        impl HBase64BinaryType for #name {
        }
    };

    TokenStream::from(expanded)
}
// #[proc_macro_derive(HBase64BinaryType)]
// pub fn derive_h_rfc_822_name_type(input: TokenStream) -> TokenStream {
//     // Parse the input tokens into a syntax tree
//     let input = parse_macro_input!(input as DeriveInput);
//     let name = input.ident;

//     // Generate an impl that implements type_name::TypeName for the type
//     // We refer to the trait via the crate name `type_name` (path dependency in Cargo.toml).
//     let expanded = quote! {
//         impl HRfc822NameType for #name {
//         }
//     };

//     TokenStream::from(expanded)
// }
// #[proc_macro_derive(HBase64BinaryType)]
// pub fn derive_h_x_500_name_type(input: TokenStream) -> TokenStream {
//     // Parse the input tokens into a syntax tree
//     let input = parse_macro_input!(input as DeriveInput);
//     let name = input.ident;

//     // Generate an impl that implements type_name::TypeName for the type
//     // We refer to the trait via the crate name `type_name` (path dependency in Cargo.toml).
//     let expanded = quote! {
//         impl HX500NameType for #name {
//         }
//     };

//     TokenStream::from(expanded)
// }

// #[proc_macro_derive(HBase64BinaryType)]
// pub fn derive_h_ip_address_type(input: TokenStream) -> TokenStream {
//     // Parse the input tokens into a syntax tree
//     let input = parse_macro_input!(input as DeriveInput);
//     let name = input.ident;

//     // Generate an impl that implements type_name::TypeName for the type
//     // We refer to the trait via the crate name `type_name` (path dependency in Cargo.toml).
//     let expanded = quote! {
//         impl HX500NameType for #name {
//         }
//     };

//     TokenStream::from(expanded)
// }
// #[proc_macro_derive(HBase64BinaryType)]
// pub fn derive_h_dns_name_type(input: TokenStream) -> TokenStream {
//     // Parse the input tokens into a syntax tree
//     let input = parse_macro_input!(input as DeriveInput);
//     let name = input.ident;

//     // Generate an impl that implements type_name::TypeName for the type
//     // We refer to the trait via the crate name `type_name` (path dependency in Cargo.toml).
//     let expanded = quote! {
//         impl HDnsNameType for #name {
//         }
//     };

//     TokenStream::from(expanded)
// }