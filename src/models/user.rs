use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub discriminator: u16,
    pub avatar: Option<Vec<u8>>,
    pub default_avatar_url: Option<String>,
    pub payments: Vec<super::payment::Payment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserData {
    pub username: String,
    pub discriminator: u16,
    pub avatar: Option<String>,
}

