use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub recipients: Option<Vec<String>>,
    pub guild: Option<Guild>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guild {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct TopChannel {
    pub name: String,
    pub message_count: usize,
    pub guild_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TopDM {
    pub id: String,
    pub dm_user_id: String,
    pub message_count: usize,
    pub user_data: Option<super::user::UserData>,
}

