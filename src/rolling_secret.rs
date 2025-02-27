use base64::{engine::general_purpose::STANDARD, Engine as _};
use hmac::{Hmac, Mac};
use sha2::{Sha256, Sha512};
use sha2::Digest;
use rand::{rngs::StdRng, Rng, SeedableRng};
use aes_gcm::{Aes256Gcm, AesGcm, Key, Nonce};
use aes_gcm::aead::{Aead};
use std::convert::TryInto;
use aes_gcm::aead::consts::U12;
use aes_gcm::aes::Aes256;
use rand::seq::SliceRandom;

const RANDOM_SEED: u64 = 364324876;
const CHARACTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()";

type HmacSha512 = Hmac<Sha512>;

#[derive(Debug, Clone)]
pub struct RollingSecret {
    otp1_key: Vec<u8>,
    otp2_key: Vec<u8>,
    aes_password: String,
}

impl RollingSecret {
    pub fn new(otp1_key: Vec<u8>, otp2_key: Vec<u8>, aes_password: String) -> Self {
        Self { otp1_key, otp2_key, aes_password }
    }

    pub fn rolling_key(&self) -> Option<Vec<u8>> {
        let otp1 = self.compute_totp(&self.otp1_key)?;
        let otp2 = self.compute_totp(&self.otp2_key)?;
        let otp = otp1 as u64 * otp2 as u64;
        let password = self.scramble_password(&self.aes_password, otp)?;
        Some(Sha256::digest(password.as_bytes()).to_vec())
    }

    fn compute_totp(&self, key: &[u8]) -> Option<u32> {
        let hash = HmacSha512::new_from_slice(key).ok()?;
        Some(u32::from_be_bytes(hash.finalize().into_bytes()[..4].try_into().ok()?))
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

    pub fn encrypt(&self, data: &[u8]) -> Option<Vec<u8>> {
        let key = self.rolling_key()?;
        let cipher: Aes256Gcm = aes_gcm::KeyInit::new(Key::<Aes256Gcm>::from_slice(&key));
        let nonce = Nonce::from_slice(&key[32..44]);
        cipher.encrypt(nonce, data).ok()
    }

    pub fn decrypt(&self, data: &[u8]) -> Option<Vec<u8>> {
        let key = self.rolling_key()?;
        let cipher: Aes256Gcm = aes_gcm::KeyInit::new(Key::<Aes256Gcm>::from_slice(&key));
        let nonce = Nonce::from_slice(&key[32..44]);
        cipher.decrypt(nonce, data).ok()
    }

    pub fn from_string(encoded: &str) -> Option<Self> {
        let parts: Vec<&str> = encoded.split('§').collect();
        if parts.len() != 3 { return None; }

        let otp1_key = STANDARD.decode(parts[0]).ok()?;
        let otp2_key = STANDARD.decode(parts[1]).ok()?;
        let aes_password = parts[2].to_string();

        Some(Self::new(otp1_key, otp2_key, aes_password))
    }

    pub fn to_string(&self) -> String {
        format!("{}§{}§{}", STANDARD.encode(&self.otp1_key), STANDARD.encode(&self.otp2_key), self.aes_password)
    }
}
