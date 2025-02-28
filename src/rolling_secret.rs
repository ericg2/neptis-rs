﻿use aes::Aes256;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyInit};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use cbc::cipher::KeyIvInit;
use cbc::{Decryptor, Encryptor};
use hmac::{Hmac, Mac};
use rand::seq::{IteratorRandom, SliceRandom};
use rand::{Rng, SeedableRng, rng, rngs::StdRng, thread_rng};
use rand::{RngCore, rngs::OsRng};
use sha2::Digest;
use sha2::{Sha256, Sha512};
use std::convert::TryInto;
use std::vec::Vec;
use cbc::cipher::block_padding::Pkcs7;
use totp_rs::Algorithm::SHA512;
use totp_rs::{Secret, TOTP};

const RANDOM_SEED: u64 = 364324876;
const CHARACTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()";

type Aes256CbcEnc = Encryptor<Aes256>;
type Aes256CbcDec = Decryptor<Aes256>;

#[derive(Debug, Clone)]
pub struct RollingSecret {
    otp_a: TOTP,
    otp_b: TOTP,
    aes_password: String,
}

impl RollingSecret {
    fn from_key(key_a: &[u8], key_b: &[u8], aes_password: &str) -> Option<Self> {
        let otp_a = TOTP::new(SHA512, 8, 0, 60, key_a.to_vec()).ok()?;
        let otp_b = TOTP::new(SHA512, 8, 0, 60, key_b.to_vec()).ok()?;
        Some(RollingSecret {
            otp_a,
            otp_b,
            aes_password: aes_password.to_string(),
        })
    }

    fn generate_key() -> Vec<u8> {
        let mut rng = rng();
        let mut random_bytes = [0u8; 64]; // 64 bytes for a 512-bit input
        rng.fill_bytes(&mut random_bytes);
        let hash = Sha512::digest(&random_bytes);
        hash.to_vec()
    }

    fn generate_password(length: usize) -> String {
        let mut rng = rng();
        (0..length)
            .map(|_| *CHARACTERS.choose(&mut rng).unwrap() as char)
            .collect()
    }

    pub fn generate() -> Option<Self> {
        Self::from_key(
            Self::generate_key().as_slice(),
            Self::generate_key().as_slice(),
            Self::generate_password(16).as_str(),
        )
    }

    pub fn from_string(encoded: &str) -> Option<Self> {
        let parts: Vec<&str> = encoded.split('§').collect();
        if parts.len() != 3 {
            return None;
        }

        let otp1_key = STANDARD.decode(parts[0]).ok()?;
        let otp2_key = STANDARD.decode(parts[1]).ok()?;
        let aes_password = parts[2].to_string();

        Self::from_key(
            otp1_key.as_slice(),
            otp2_key.as_slice(),
            aes_password.as_str(),
        )
    }

    pub fn rolling_key(&self) -> Option<Vec<u8>> {
        let otp1 = self.otp_a.generate_current().ok()?.parse::<i64>().ok()?;
        let otp2 = self.otp_b.generate_current().ok()?.parse::<i64>().ok()?;
        let otp = otp1 as u64 * otp2 as u64;
        let password = self.scramble_password(&self.aes_password, otp)?;
        Some(Sha256::digest(password.as_bytes()).to_vec())
    }

    fn scramble_password(&self, password: &str, secret_key: u64) -> Option<String> {
        let shuffled_chars = Self::get_shuffled_characters();
        let key = Self::number_to_string(secret_key);
        let key_length = key.len();
        let mut scrambled = String::with_capacity(password.len());

        for (i, c) in password.chars().enumerate() {
            let key_char = key.chars().nth(i % key_length)?;
            let password_index = shuffled_chars.find(c)?;
            let key_index = shuffled_chars.find(key_char)?;
            let scrambled_index = (password_index + key_index) % shuffled_chars.len();
            scrambled.push(shuffled_chars.chars().nth(scrambled_index)?);
        }

        Some(scrambled)
    }

    fn get_shuffled_characters() -> String {
        let mut rng = StdRng::seed_from_u64(RANDOM_SEED);
        let mut chars: Vec<char> = CHARACTERS.chars().collect();
        chars.shuffle(&mut rng);
        chars.into_iter().collect()
    }

    fn number_to_string(mut number: u64) -> String {
        let shuffled_chars = Self::get_shuffled_characters();
        let base = shuffled_chars.len() as u64;
        let mut result = String::new();
        while number > 0 {
            let mod_index = (number % base) as usize;
            result.insert(0, shuffled_chars.chars().nth(mod_index).unwrap());
            number /= base;
        }
        result
    }

    pub fn encrypt(data: &[u8], key: &[u8]) -> Option<Vec<u8>> {
        if key.len() != 32 {
            return None; // AES-256 requires a 32-byte key
        }

        let mut iv = [0u8; 16];
        OsRng.fill_bytes(&mut iv); // Generate a random IV (same as C#)

        let cipher = Aes256CbcEnc::new_from_slices(key, &iv).ok()?;
        let e_bytes = cipher.encrypt_padded_vec_mut::<Pkcs7>(data);

        let mut result = iv.to_vec();
        result.extend(e_bytes);
        Some(result)
    }

    /// Decrypts AES-256-CBC data, extracting the IV from the first 16 bytes
    pub fn decrypt(encrypted_data: &[u8], key: &[u8]) -> Option<Vec<u8>> {
        if key.len() != 32 || encrypted_data.len() < 16 {
            return None;
        }

        let (iv, ciphertext) = encrypted_data.split_at(16);
        let cipher = Aes256CbcDec::new_from_slices(key, iv).ok()?;
        let mut buffer = ciphertext.to_vec();
        cipher
            .decrypt_padded_vec_mut::<Pkcs7>(&mut buffer)
            .ok()
    }
}
