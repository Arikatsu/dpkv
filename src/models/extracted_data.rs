use super::user::User;
use super::channel::{TopChannel, TopDM};
use super::message::FavoriteWord;
use super::payment::PaymentInfo;

#[derive(Debug, Clone)]
pub struct ExtractedData {
    pub user: Option<User>,
    pub top_dms: Vec<TopDM>,
    pub top_channels: Vec<TopChannel>,
    pub guild_count: usize,
    pub dm_channel_count: usize,
    pub channel_count: usize,
    pub message_count: usize,
    pub character_count: usize,
    pub total_spent: f64,
    pub hours_values: Vec<usize>,
    pub favorite_words: Vec<FavoriteWord>,
    pub payments: PaymentInfo,
    pub open_count: Option<usize>,
    pub average_open_count_per_day: Option<usize>,
    pub notification_count: Option<usize>,
    pub join_voice_channel_count: Option<usize>,
    pub join_call_count: Option<usize>,
    pub add_reaction_count: Option<usize>,
    pub message_edited_count: Option<usize>,
    pub sent_message_count: Option<usize>,
    pub average_message_count_per_day: Option<usize>,
    pub slash_command_used_count: Option<usize>,
}

impl Default for ExtractedData {
    fn default() -> Self {
        Self {
            user: None,
            top_dms: Vec::new(),
            top_channels: Vec::new(),
            guild_count: 0,
            dm_channel_count: 0,
            channel_count: 0,
            message_count: 0,
            character_count: 0,
            total_spent: 0.0,
            hours_values: vec![0; 24],
            favorite_words: Vec::new(),
            payments: PaymentInfo {
                total: std::collections::HashMap::new(),
                list: String::new(),
            },
            open_count: None,
            average_open_count_per_day: None,
            notification_count: None,
            join_voice_channel_count: None,
            join_call_count: None,
            add_reaction_count: None,
            message_edited_count: None,
            sent_message_count: None,
            average_message_count_per_day: None,
            slash_command_used_count: None,
        }
    }
}

