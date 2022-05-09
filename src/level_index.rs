use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YoleckLevelFileEntry {
    pub filename: String,
}
