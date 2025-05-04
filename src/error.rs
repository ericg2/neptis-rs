// #[derive(Debug, thiserror::Error)]
// pub enum NeptisError {
//     #[error(transparent)]
//     Io(#[from] std::io::Error),
//     #[error(transparent)]
//     Json(#[from] serde_json::Error),
//     #[error(transparent)]
//     Parse(#[from] std::string::ParseError),
//     #[error(transparent)]
//     ChronoParse(#[from] chrono::ParseError),
//     #[error("{0}")]
//     Str(String),
// }

// // we must manually implement serde::Serialize
// impl serde::Serialize for NeptisError {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::ser::Serializer,
//     {
//         serializer.serialize_str(self.to_string().as_ref())
//     }
// }