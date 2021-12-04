use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Image {
    pub height: Option<u32>,
    pub url: String,
    pub width: Option<u32>,
}
