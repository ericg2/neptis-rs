use base64::{Engine as _, engine::general_purpose::STANDARD};
use rand::RngCore;
use rand::rng;
use sha1::Digest;
use totp_lite::{DEFAULT_STEP, Sha1, totp_custom};

/// A basic encoding layer which ONLY guards against replay
/// attacks when used correctly! Not intended for sensitive data.
#[derive(Debug, Clone)]
pub struct ArduinoSecret {
    key_a: Vec<u8>,
    key_b: Vec<u8>,
}

impl ToString for ArduinoSecret {
    fn to_string(&self) -> String {
        let otp_a_key = STANDARD.encode(self.key_a.as_slice());
        let otp_b_key = STANDARD.encode(self.key_b.as_slice());
        format!("{}§{}", otp_a_key, otp_b_key)
    }
}

impl ArduinoSecret {
    fn from_key(key_a: &[u8], key_b: &[u8]) -> Self {
        ArduinoSecret {
            key_a: key_a.to_vec(),
            key_b: key_b.to_vec(),
        }
    }

    pub fn from_string(encoded: &str) -> Option<Self> {
        let parts: Vec<&str> = encoded.split('§').collect();
        if parts.len() != 2 {
            return None;
        }

        let otp1_key = STANDARD.decode(parts[0]).ok()?;
        let otp2_key = STANDARD.decode(parts[1]).ok()?;

        Some(Self::from_key(otp1_key.as_slice(), otp2_key.as_slice()))
    }

    pub fn rolling_key(&self) -> Option<usize> {
        use chrono::Utc;
        let now = Utc::now().timestamp() as u64;
        let o1 = totp_custom::<Sha1>(15, 6, self.key_a.as_slice(), now)
            .parse::<usize>()
            .ok()?;
        let o2 = totp_custom::<Sha1>(15, 6, self.key_b.as_slice(), now)
            .parse::<usize>()
            .ok()?;
        Some(o1 + o2)
    }
}
