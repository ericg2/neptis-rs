use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct FileSize {
    bytes: u64,
    text: String,
}

impl From<u64> for FileSize {
    fn from(value: u64) -> Self {
        FileSize::from_bytes(value)
    }
}

impl From<i64> for FileSize {
    fn from(value: i64) -> Self {
        FileSize::from_bytes(value as u64)
    }
}

impl FromStr for FileSize {
    type Err = String;
    fn from_str(human: &str) -> Result<Self, Self::Err> {
        let human = human.trim().to_uppercase();
        if human == "0" || human == "0 B" {
            return Ok(FileSize::new(0, "0B".into()));
        }

        // Split into number and unit parts
        let split_pos = human
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(human.len()); // Default to whole string if no unit found

        let (num_part, unit_part) = human.split_at(split_pos);
        let num = f64::from_str(num_part.trim()).map_err(|e| format!("Invalid number: {}", e))?;

        let unit_part = unit_part.trim();

        // If no unit specified, assume bytes
        if unit_part.is_empty() {
            return Ok(FileSize::new(num as u64, human.into()));
        }

        // Match the unit (case insensitive)
        let unit = match unit_part {
            "B" | "BYTE" | "BYTES" => 0,
            "KB" | "KIB" | "KILOBYTE" | "KILOBYTES" => 1,
            "MB" | "MIB" | "MEGABYTE" | "MEGABYTES" => 2,
            "GB" | "GIB" | "GIGABYTE" | "GIGABYTES" => 3,
            "TB" | "TIB" | "TERABYTE" | "TERABYTES" => 4,
            "PB" | "PIB" | "PETABYTE" | "PETABYTES" => 5,
            "EB" | "EIB" | "EXABYTE" | "EXABYTES" => 6,
            "ZB" | "ZIB" | "ZETTABYTE" | "ZETTABYTES" => 7,
            "YB" | "YIB" | "YOTTABYTE" | "YOTTABYTES" => 8,
            _ => return Err(format!("Unknown unit: {}", unit_part)),
        };

        let bytes = num * 1024f64.powi(unit);
        if bytes.is_infinite() || bytes > u64::MAX as f64 {
            Err("Value too large".to_string())
        } else {
            Ok(FileSize::new(bytes as u64, human.into()))
        }
    }
}

impl ToString for FileSize {
    fn to_string(&self) -> String {
        self.get_text().to_string()
    }
}

impl FileSize {
    pub fn get_bytes(&self) -> u64 {
        self.bytes
    }
    pub fn get_text(&self) -> &str {
        self.text.as_str()
    }

    pub fn prettify(bytes: u64) -> String {
        Self::from_bytes(bytes).to_string()
    }

    fn new(bytes: u64, text: String) -> Self {
        FileSize { bytes, text }
    }

    pub fn from_bytes(bytes: u64) -> FileSize {
        const UNITS: [&str; 9] = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
        const UNIT_SIZE: f64 = 1024.0;

        if bytes == 0 {
            return FileSize::new(bytes, "0B".into());
        }

        let magnitude = (bytes as f64).log(UNIT_SIZE).floor() as u32;
        let magnitude = magnitude.min((UNITS.len() - 1) as u32);
        let adjusted_size = bytes as f64 / UNIT_SIZE.powi(magnitude as i32);

        // Format to 1 decimal place if needed, otherwise as integer
        FileSize::new(
            bytes,
            if adjusted_size.fract() > 1e-9 && adjusted_size < 10.0 {
                format!("{:.1} {}", adjusted_size, UNITS[magnitude as usize])
            } else {
                format!("{} {}", adjusted_size.floor(), UNITS[magnitude as usize])
            },
        )
    }
}
