use anyhow::{Result, anyhow};
use chrono::{DateTime, Timelike};
use regex::Regex;
use serde::{Deserialize};
use std::collections::HashMap;
use std::io::{BufReader, Read};
use zip::ZipArchive;

use crate::models::{
    Channel, ExtractedData, FavoriteWord, Message, ParsedMessage, 
    Payment, TopChannel, TopDM, User, UserData,
};

pub struct Parser {
    // client: reqwest::Client,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            // client: reqwest::Client::new(),
        }
    }

    pub async fn extract_data<R: Read + std::io::Seek, F>(
        &self,
        mut archive: ZipArchive<R>,
        progress_callback: F,
    ) -> Result<ExtractedData>
    where
        F: Fn(String) + Send + Sync,
    {
        let mut extracted_data = ExtractedData::default();

        progress_callback("Analyzing package structure...".to_string());

        let file_names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();

        let messages_root = self.get_messages_root(&file_names)?;
        let servers_root = self.get_servers_root(&file_names)?;
        let user_root = self.get_user_root(&file_names)?;

        println!("[debug] Found messages root: {}", messages_root);
        println!("[debug] Found servers root: {}", servers_root);
        println!("[debug] Found user root: {}", user_root);

        progress_callback("Loading user information...".to_string());

        let user_path = format!("{}/user.json", user_root);
        let user_content = self.read_file(&mut archive, &user_path)?;

        if let Some(content) = user_content {
            println!("[debug] Loading user info from: {}", user_path);
            match self.parse_json::<User>(&content) {
                Ok(mut user) => {
                    let fetched_user =
                        self.fetch_user(&user.id)
                            .await
                            .unwrap_or_else(|_| UserData {
                                username: "Unknown".to_string(),
                                discriminator: 0,
                                avatar: None,
                            });

                    user.username = fetched_user.username;
                    user.discriminator = fetched_user.discriminator;
                    user.avatar_hash = fetched_user.avatar;

                    // Process payments
                    self.process_payments(&mut extracted_data, &user);
                    extracted_data.user = Some(user);
                }
                Err(e) => {
                    println!("[debug] Failed to parse user.json: {}", e);
                }
            }
        }

        progress_callback("Loading messages index...".to_string());

        let messages_index_path = format!("{}/index.json", messages_root);
        let index_content = self.read_file(&mut archive, &messages_index_path)?;
        let messages_index: HashMap<String, String> = if let Some(content) = index_content {
            println!(
                "[debug] Loading messages index from: {}",
                messages_index_path
            );
            self.parse_json(&content).unwrap_or_else(|e| {
                println!("[debug] Failed to parse messages index: {}", e);
                HashMap::new()
            })
        } else {
            HashMap::new()
        };

        progress_callback("Processing channels and messages...".to_string());

        self.process_channels(
            &mut archive,
            &mut extracted_data,
            &messages_root,
            &messages_index,
            &progress_callback,
        )
        .await?;

        progress_callback("Loading guild information...".to_string());

        let guild_index_path = format!("{}/index.json", servers_root);
        let guild_content = self.read_file(&mut archive, &guild_index_path)?;

        if let Some(content) = guild_content {
            println!("[debug] Loading guild index from: {}", guild_index_path);
            match self.parse_json::<HashMap<String, String>>(&content) {
                Ok(guild_index) => {
                    extracted_data.guild_count = guild_index.len();
                }
                Err(e) => {
                    println!("[debug] Failed to parse guild index: {}", e);
                }
            }
        }

        progress_callback("Processing analytics...".to_string());

        self.process_analytics(&mut archive, &mut extracted_data, &file_names)
            .await?;

        progress_callback("Finalizing extraction...".to_string());

        println!("[debug] Extraction complete");
        Ok(extracted_data)
    }

    fn get_messages_root(&self, files: &[String]) -> Result<String> {
        let regex = Regex::new(r"/c?[0-9]{16,32}/channel\.json$")?;
        let sample = files
            .iter()
            .find(|f| regex.is_match(f))
            .ok_or_else(|| anyhow!("Could not find Messages folder structure"))?;

        let segments: Vec<&str> = sample.split('/').collect();
        Ok(segments[..segments.len() - 2].join("/"))
    }

    fn get_servers_root(&self, files: &[String]) -> Result<String> {
        let regex = Regex::new(r"/[0-9]{16,32}/guild\.json$")?;
        let sample = files
            .iter()
            .find(|f| regex.is_match(f))
            .ok_or_else(|| anyhow!("Could not find Servers folder structure"))?;

        let segments: Vec<&str> = sample.split('/').collect();
        Ok(segments[..segments.len() - 2].join("/"))
    }

    fn get_user_root(&self, files: &[String]) -> Result<String> {
        let regex = Regex::new(r"^([^/]+)/user\.json$")?;
        let sample = files
            .iter()
            .find(|f| regex.is_match(f))
            .ok_or_else(|| anyhow!("Could not find User folder structure"))?;

        let segments: Vec<&str> = sample.split('/').collect();
        Ok(segments[..segments.len() - 1].join("/"))
    }

    fn read_file<R: Read + std::io::Seek>(
        &self,
        archive: &mut ZipArchive<R>,
        path: &str,
    ) -> Result<Option<String>> {
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            if file.name() == path {
                let mut content = String::new();
                let mut reader = BufReader::new(file);
                reader.read_to_string(&mut content)?;

                let cleaned_content = content.trim_start_matches('\u{FEFF}').trim();

                if cleaned_content.is_empty() {
                    println!("[debug] Warning: File {} is empty", path);
                    return Ok(None);
                }

                return Ok(Some(cleaned_content.to_string()));
            }
        }
        Ok(None)
    }

    fn parse_json<T>(&self, content: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        if content.trim().is_empty() {
            return Err(anyhow!("Empty JSON content"));
        }

        let trimmed = content.trim();
        if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
            return Err(anyhow!("Invalid JSON format - doesn't start with {{ or ["));
        }

        let mut data = content.as_bytes().to_vec();
        match simd_json::from_slice::<T>(&mut data) {
            Ok(result) => {
                println!("[debug] Successfully parsed with simd_json");
                Ok(result)
            }
            Err(e) => {
                println!(
                    "[debug] simd_json failed: {}, falling back to standard JSON parser",
                    e
                );

                match serde_json::from_str::<T>(content) {
                    Ok(result) => {
                        println!("[debug] Successfully parsed with serde_json");
                        Ok(result)
                    }
                    Err(e2) => Err(anyhow!(
                        "Both JSON parsers failed. simd_json: {}, serde_json: {}",
                        e,
                        e2
                    )),
                }
            }
        }
    }

    async fn fetch_user(&self, _user_id: &str) -> Result<UserData> {
        // TODO: Implement actual user fetching logic
        Ok(UserData {
            username: "Unknown".to_string(),
            discriminator: 0,
            avatar: None,
        })
    }

    fn process_payments(&self, extracted_data: &mut ExtractedData, user: &User) {
        let confirmed_payments: Vec<&Payment> =
            user.payments.iter().filter(|p| p.status == 1).collect();

        if !confirmed_payments.is_empty() {
            let mut currencies = std::collections::HashSet::new();
            for payment in &confirmed_payments {
                currencies.insert(payment.currency.clone());
                *extracted_data
                    .payments
                    .total
                    .entry(payment.currency.clone())
                    .or_insert(0.0) += payment.amount as f64 / 100.0;
            }

            let mut sorted_payments = confirmed_payments;
            sorted_payments.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            extracted_data.payments.list = sorted_payments
                .iter()
                .map(|p| {
                    format!(
                        "{} ({} {:.2})",
                        p.description,
                        p.currency.to_uppercase(),
                        p.amount as f64 / 100.0
                    )
                })
                .collect::<Vec<_>>()
                .join("<br>");
        }
    }

    async fn process_channels<R: Read + std::io::Seek, F>(
        &self,
        archive: &mut ZipArchive<R>,
        extracted_data: &mut ExtractedData,
        messages_root: &str,
        messages_index: &HashMap<String, String>,
        progress_callback: &F,
    ) -> Result<()>
    where
        F: Fn(String) + Send + Sync,
    {
        let channel_regex = Regex::new(r"c?([0-9]{16,32})/$")?;
        let mut channel_ids = Vec::new();

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name();
            if name.starts_with(messages_root) && channel_regex.is_match(name) {
                if let Some(captures) = channel_regex.captures(name) {
                    channel_ids.push(captures[1].to_string());
                }
            }
        }

        progress_callback(format!("Found {} channels to process", channel_ids.len()));

        let is_old_package = channel_ids.iter().any(|id| {
            let path = format!("{}/{}/channel.json", messages_root, id);
            self.file_exists(archive, &path).unwrap_or(false)
        });

        let is_old_package_v2 = !channel_ids.iter().any(|id| {
            let path = format!("{}/c{}/messages.json", messages_root, id);
            self.file_exists(archive, &path).unwrap_or(false)
        });

        println!("[debug] Old package (2021): {}", is_old_package);
        println!("[debug] Old package (2024): {}", is_old_package_v2);

        let mut word_counts: HashMap<String, usize> = HashMap::new();
        let mut channel_message_counts: Vec<(String, usize, String)> = Vec::new();
        let mut dm_message_counts: Vec<(String, String, usize)> = Vec::new(); 

        for (index, channel_id) in channel_ids.iter().enumerate() {
            if index % (channel_ids.len() / 10 + 1) == 0 {
                progress_callback(format!(
                    "Processing channel {} of {} (ID: {})",
                    index + 1,
                    channel_ids.len(),
                    channel_id
                ));
            }
            
            let prefix = if is_old_package { "" } else { "c" };
            let extension = if is_old_package_v2 { "csv" } else { "json" };
            
            let channel_data_path =
                format!("{}/{}{}/channel.json", messages_root, prefix, channel_id);
            let channel_messages_path = format!(
                "{}/{}{}/messages.{}",
                messages_root, prefix, channel_id, extension
            );
            
            let channel_data = self.read_file(archive, &channel_data_path)?;
            let channel_messages_content = self.read_file(archive, &channel_messages_path)?;
            
            if let (Some(data_content), Some(messages_content)) =
                (channel_data, channel_messages_content)
            {
                let channel: Channel = match self.parse_json(&data_content) {
                    Ok(ch) => ch,
                    Err(e) => {
                        println!(
                            "[debug] Failed to parse channel data for {}: {}",
                            channel_id, e
                        );
                        continue;
                    }
                };
                
                let messages: Vec<ParsedMessage> = if extension == "csv" {
                    self.parse_csv(&messages_content)?
                } else {
                    match self.parse_json_messages(&messages_content) {
                        Ok(m) => m,
                        Err(e) => {
                            println!("[debug] Failed to parse messages for {}: {}", channel_id, e);
                            Vec::new()
                        }
                    }
                };
                
                let name = messages_index
                    .get(&channel.id)
                    .cloned()
                    .unwrap_or_else(|| channel.id.clone());
                
                let is_dm = channel.recipients.as_ref().map_or(false, |r| r.len() == 2);
                
                let dm_user_id = if is_dm {
                    channel.recipients.as_ref().and_then(|recipients| {
                        extracted_data
                            .user
                            .as_ref()
                            .and_then(|user| recipients.iter().find(|&id| id != &user.id).cloned())
                    })
                } else {
                    None
                }; 
                
                for message in &messages {
                    extracted_data.character_count += message.length;
                    if let Ok(dt) = DateTime::parse_from_rfc3339(&message.timestamp) {
                        extracted_data.hours_values[dt.hour() as usize] += 1;
                    }
                    for word in &message.words {
                        if word.len() > 5 {
                            *word_counts.entry(word.clone()).or_insert(0) += 1;
                        }
                    }
                } 
                
                if is_dm {
                    if let Some(dm_id) = dm_user_id {
                        dm_message_counts.push((channel.id.clone(), dm_id, messages.len()));
                    }
                } else if let Some(guild) = &channel.guild {
                    channel_message_counts.push((name.clone(), messages.len(), guild.name.clone()));
                }

                drop(messages);
            }
        }

        extracted_data.channel_count = channel_message_counts.len();
        extracted_data.dm_channel_count = dm_message_counts.len();
        extracted_data.message_count = channel_message_counts
            .iter()
            .map(|(_, count, _)| *count)
            .sum::<usize>()
            + dm_message_counts
                .iter()
                .map(|(_, _, count)| *count)
                .sum::<usize>();

        // Top channels
        channel_message_counts.sort_by(|a, b| b.1.cmp(&a.1));
        extracted_data.top_channels = channel_message_counts
            .into_iter()
            .take(10)
            .map(|(name, count, guild)| TopChannel {
                name,
                message_count: count,
                guild_name: Some(guild),
            })
            .collect();

        // Top DMs
        dm_message_counts.sort_by(|a, b| b.2.cmp(&a.2));
        extracted_data.top_dms = dm_message_counts
            .into_iter()
            .take(10)
            .map(|(id, user_id, count)| TopDM {
                id,
                dm_user_id: user_id,
                message_count: count,
                user_data: None,
            })
            .collect();

        // Favorite words
        let mut word_vec: Vec<_> = word_counts.into_iter().collect();
        word_vec.sort_by(|a, b| b.1.cmp(&a.1));
        extracted_data.favorite_words = word_vec
            .into_iter()
            .take(10)
            .map(|(word, count)| FavoriteWord { word, count })
            .collect();
        
        Ok(())
    }
    
    fn parse_csv(&self, content: &str) -> Result<Vec<ParsedMessage>> {
        let mut reader = csv::Reader::from_reader(content.as_bytes());
        let mut messages = Vec::new();
        
        for result in reader.deserialize() {
            let record: Message = result?;
            if !record.contents.is_empty() {
                messages.push(ParsedMessage {
                    id: record.id,
                    timestamp: record.timestamp,
                    length: record.contents.len(),
                    words: record
                        .contents
                        .split_whitespace()
                        .map(|s| s.to_string())
                        .collect(),
                });
            }
        }
        
        Ok(messages)
    }
    
    fn parse_json_messages(&self, content: &str) -> Result<Vec<ParsedMessage>> {
        let messages: Vec<Message> = match self.parse_json(content) {
            Ok(m) => m,
            Err(_) => {
                // Try parsing as a single message object instead of array
                match self.parse_json::<Message>(content) {
                    Ok(msg) => vec![msg],
                    Err(e) => return Err(e),
                }
            }
        };
        
        Ok(messages
            .into_iter()
            .filter(|m| !m.contents.is_empty())
            .map(|m| ParsedMessage {
                id: m.id,
                timestamp: m.timestamp,
                length: m.contents.len(),
                words: m
                    .contents
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect(),
            })
            .collect())
    }
    
    async fn process_analytics<R: Read + std::io::Seek>(
        &self,
        archive: &mut ZipArchive<R>,
        extracted_data: &mut ExtractedData,
        file_names: &[String],
    ) -> Result<()> {
        let analytics_regex = Regex::new(r"analytics/events-[0-9]{4}-[0-9]{5}-of-[0-9]{5}\.json$")?;
        let analytics_file = file_names.iter().find(|f| analytics_regex.is_match(f));
        
        if let Some(file_name) = analytics_file {
            // if let Some(_content) = self.read_file(archive, file_name)? {
                // TODO: Parse the analytics file efficiently
                extracted_data.open_count = Some(0);
                extracted_data.notification_count = Some(0);
                extracted_data.join_voice_channel_count = Some(0);
                extracted_data.join_call_count = Some(0);
                extracted_data.add_reaction_count = Some(0);
                extracted_data.message_edited_count = Some(0);
                extracted_data.sent_message_count = Some(0);
                extracted_data.slash_command_used_count = Some(0);
            // }
        }
        
        Ok(())
    }
    fn file_exists<R: Read + std::io::Seek>(
        &self,
        archive: &mut ZipArchive<R>,
        path: &str,
    ) -> Result<bool> {
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            if file.name() == path {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
