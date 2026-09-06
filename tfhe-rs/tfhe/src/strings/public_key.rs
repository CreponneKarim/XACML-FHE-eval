use std::borrow::Borrow;
use crate::integer::{PublicKey as IntegerPublicKey, RadixCiphertext};
use crate::strings::ciphertext::{num_ascii_blocks, FheAsciiChar, FheString};
use rayon::prelude::*;

pub struct PublicKey<T>
where
    T: Borrow<IntegerPublicKey>,
{
    inner: T,
}

impl<T> PublicKey<T>
where
    T: Borrow<IntegerPublicKey>,
{
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> &IntegerPublicKey {
        self.inner.borrow()
    }
}

#[derive(Clone)]
pub struct EncU16 {
    cipher: RadixCiphertext,
    max: Option<u16>,
}

impl EncU16 {
    pub(crate) fn new(value: RadixCiphertext, max: Option<u16>) -> Self {
        Self { cipher: value, max }
    }
    pub fn cipher(&self) -> &RadixCiphertext {
        &self.cipher
    }

    pub fn max(&self) -> Option<u16> {
        self.max
    }
}

impl<T> PublicKey<T> 
where
    T: Borrow<IntegerPublicKey>,
{
    // #[cfg(test)]
    // pub fn trivial_encrypt_ascii(&self, str: &str, padding: Option<u32>) -> FheString {
    //     let pk = self.inner.borrow();

    //     super::ciphertext::trivial_encrypt_ascii(
    //         &pk.key,
    //         &crate::shortint::ClientKey::create_trivial,
    //         str,
    //         padding,
    //     )
    // }

    /// Encrypts an ASCII string, optionally padding it with the specified amount of 0s, and returns
    /// an [`FheString`].
    ///
    /// # Panics
    ///
    /// This function will panic if the provided string is not ASCII or contains null characters
    /// "\0".
    pub fn encrypt_ascii(&self, str: &str, padding: Option<u32>) -> FheString {
        let pk = self.inner.borrow();

        assert!(str.is_ascii() & !str.contains('\0'));

        let padded = padding.is_some_and(|p| p != 0);

        let num_blocks = self.num_ascii_blocks();
        
        let mut enc_string: Vec<_> = str
            .bytes()
            .collect::<Vec<_>>()
            .par_iter()
            .map(|&char| FheAsciiChar {
                enc_char: pk.encrypt_radix(char, num_blocks),
            })
            .collect();

        // Optional padding
        if let Some(count) = padding {
            let null = (0..count).map(|_| FheAsciiChar {
                enc_char: pk.encrypt_radix(0u8, num_blocks),
            });

            enc_string.extend(null);
        }

        FheString { enc_string, padded }
    }

    fn num_ascii_blocks(&self) -> usize {
        let pk = self.inner.borrow();

        assert_eq!(
            pk.parameters().message_modulus().0,
            pk.parameters().carry_modulus().0
        );

        num_ascii_blocks(pk.parameters().message_modulus())
    }

    // #[cfg(test)]
    // pub fn trivial_encrypt_u16(&self, val: u16, max: Option<u16>) -> EncU16 {
    //     let pk = self.inner.borrow();

    //     if let Some(max_val) = max {
    //         assert!(val <= max_val, "val cannot be greater than max")
    //     }

    //     EncU16 {
    //         cipher: pk.create_trivial_radix(val, 8),
    //         max,
    //     }
    // }

    /// Encrypts a u16 value. It also takes an optional `max` value to restrict the range
    /// of the encrypted u16.
    ///
    /// # Panics
    ///
    /// This function will panic if the u16 value exceeds the provided `max`.
    pub fn encrypt_u16(&self, val: u16, max: Option<u16>) -> EncU16 {
        let ck = self.inner.borrow();

        if let Some(max_val) = max {
            assert!(val <= max_val, "val cannot be greater than max")
        }

        EncU16 {
            cipher: ck.encrypt_radix(val, 8),
            max,
        }
    }
}