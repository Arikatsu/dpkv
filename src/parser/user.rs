use anyhow::Result;
use std::io::Read;
use zip::ZipArchive;

use crate::models::{ExtractedData, Payment, User};
use crate::parser::Parser;

impl Parser {
    pub(super) async fn load_user<R: Read + std::io::Seek, F>(
        &self,
        archive: &mut ZipArchive<R>,
        user_root: &str,
        extracted_data: &mut ExtractedData,
        progress_callback: &F,
    ) -> Result<()>
    where
        F: Fn(String) + Send + Sync,
    {
        progress_callback("Loading user information...".to_string());

        let user_path = format!("{}/user.json", user_root);

        if let Some(content) = self.read_file(archive, &user_path)? {
            println!("[debug] Loading user info from: {}", user_path);

            if let Ok(mut user) = self.parse_json::<User>(&content) {
                user.avatar = self.load_user_avatar(archive, user_root)?;
                user.default_avatar_url = Some(self.get_default_avatar_url(
                    &user.id,
                    user.discriminator,
                ));
                self.process_payments(extracted_data, &user);
                extracted_data.user = Some(user);
            } else {
                println!("[debug] Failed to parse user.json");
            }
        }

        Ok(())
    }

    fn load_user_avatar<R: Read + std::io::Seek>(
        &self,
        archive: &mut ZipArchive<R>,
        user_root: &str,
    ) -> Result<Option<Vec<u8>>> {
        let extensions = ["png", "jpeg", "jpg", "gif"];

        for ext in &extensions {
            let avatar_path = format!("{}/avatar.{}", user_root, ext);
            if !self.file_exists(&avatar_path) {
                continue;
            }
            if let Some(content) = self.read_binary_file(archive, &avatar_path)? {
                println!("[debug] Found avatar: {}", avatar_path);
                return Ok(Some(content));
            }
        }

        println!("[debug] No avatar found in {}", user_root);
        Ok(None)
    }

    fn get_default_avatar_url(
        &self,
        user_id: &str,
        discriminator: u16
    ) -> String {
        let avatar_index = if discriminator == 0 {
            ((user_id.parse::<u64>().unwrap_or(0) >> 22) % 6) as u16
        } else {
            discriminator % 5
        };
        format!("https://cdn.discordapp.com/embed/avatars/{}.png", avatar_index)
    }

    fn process_payments(&self, extracted_data: &mut ExtractedData, user: &User) {
        let confirmed: Vec<&Payment> = user.payments.iter().filter(|p| p.status == 1).collect();
        if confirmed.is_empty() {
            return;
        }
        for payment in &confirmed {
            *extracted_data
                .payments
                .total
                .entry(payment.currency.clone())
                .or_insert(0.0) += payment.amount as f64 / 100.0;
        }
        let mut sorted = confirmed;
        sorted.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        extracted_data.payments.list = sorted
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
