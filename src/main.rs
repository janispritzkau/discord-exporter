#![allow(unused)]

mod api;
mod migration;

use std::time::Duration;

use crate::migration::run_migrations;
use api::DiscordApi;
use chrono::DateTime;
use clap::StructOpt;
use eyre::{Context, ContextCompat};
use reqwest::Url;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(clap::Parser)]
struct Args {
    // Database file path
    #[clap(long, default_value = "discord_exporter.db")]
    db: String,

    #[clap(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    Download {
        /// Id of the channel to fetch messages from
        channel_id: u64,

        /// Every message after id or date time
        #[clap(short, long)]
        after: Option<MessageInput>,

        /// Every message before id or date time
        #[clap(short, long)]
        before: Option<MessageInput>,

        /// Number of messages to fetch at once
        #[clap(long, default_value = "100")]
        fetch_limit: u32,

        /// Fetch interval in milliseconds
        #[clap(long, default_value = "1000")]
        fetch_interval: u64,
    },
}

#[derive(Debug)]
enum MessageInput {
    Id(u64),
    DateTime(chrono::NaiveDateTime),
}

impl std::str::FromStr for MessageInput {
    type Err = chrono::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if let Ok(id) = s.parse() {
            MessageInput::Id(id)
        } else if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%F") {
            MessageInput::DateTime(date.and_hms(0, 0, 0))
        } else {
            MessageInput::DateTime(chrono::DateTime::parse_from_str(s, "%+")?.naive_utc())
        })
    }
}

impl From<MessageInput> for u64 {
    fn from(m: MessageInput) -> Self {
        match m {
            MessageInput::Id(id) => id,
            MessageInput::DateTime(date_time) => {
                (date_time.timestamp_millis() as u64 - 1420070400000) << 22
            }
        }
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = Args::parse();

    let mut db_conn = rusqlite::Connection::open(args.db).wrap_err("could not open database")?;
    run_migrations(&mut db_conn).wrap_err("database migration failed")?;

    let mut http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let token = std::env::var("TOKEN").wrap_err("TOKEN environment required")?;
    let api = DiscordApi::new(&http_client, &token);

    let mut downloader = Downloader::new(&mut db_conn, &api);

    match args.command {
        Command::Download {
            channel_id,
            after,
            before,
            fetch_limit,
            fetch_interval,
        } => {
            let after: u64 = after.map_or(0, Into::into);
            let before: u64 = before.map_or(
                MessageInput::DateTime(chrono::Utc::now().naive_utc()).into(),
                Into::into,
            );

            downloader
                .download_messages(channel_id, after, before, fetch_limit, fetch_interval)
                .await?;
        }
    }

    Ok(())
}

struct Downloader<'a> {
    db_conn: &'a mut rusqlite::Connection,
    api: &'a DiscordApi<'a>,
}

impl<'a> Downloader<'a> {
    fn new(db_conn: &'a mut rusqlite::Connection, api: &'a DiscordApi) -> Self {
        Self { db_conn, api }
    }

    async fn download_messages(
        &mut self,
        channel_id: u64,
        after: u64,
        before: u64,
        fetch_limit: u32,
        fetch_interval: u64,
    ) -> eyre::Result<()> {
        let tx = self.db_conn.transaction()?;

        if tx
            .query_row(
                "SELECT id FROM channels WHERE id = ?",
                [channel_id],
                |row| Ok(()),
            )
            .optional()?
            .is_none()
        {
            let channel = self.api.fetch_channel(channel_id).await?;

            if let Some(guild_id) = channel.guild_id {
                if tx
                    .query_row("SELECT id FROM guilds WHERE id = ?", [guild_id], |_| Ok(()))
                    .optional()?
                    .is_none()
                {
                    let guild = self.api.fetch_guild(guild_id).await?;
                    tx.execute(
                        "INSERT INTO guilds (id, data, updated_at) VALUES (?, ?, unixepoch())",
                        params![guild.id, guild.data],
                    )?;
                }
            }

            tx.execute(
                "INSERT INTO channels (id, guild_id, data, updated_at) VALUES (?, ?, ?, unixepoch())",
                params![channel.id, channel.guild_id, channel.data],
            )?;
        }

        tx.commit()?;

        let (channel_name, guild_name): (Option<String>, Option<String>) = self.db_conn.query_row(
            "SELECT json_extract(c.data, '$.name'), json_extract(g.data, '$.name') FROM channels c LEFT JOIN guilds g ON g.id = c.guild_id WHERE c.id = ?",
            [channel_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        loop {
            let tx = self.db_conn.transaction()?;

            let range: Option<(u64, u64)> = tx
                .query_row(
                    concat!(
                        "SELECT after_id, last_id FROM message_ranges ",
                        "WHERE channel_id = ?1 ",
                        "AND after_id <= ?2 AND ?2 < last_id"
                    ),
                    params![channel_id, after],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;

            let after = range.map_or(after, |(_, last_id)| last_id);

            if let (Some(guild_name), Some(channel_name)) = (&guild_name, &channel_name) {
                println!("fetching messages after {after} in #{channel_name} of {guild_name}");
            } else if let Some(channel_name) = &channel_name {
                println!("fetching messages after {after} in #{channel_name}");
            } else {
                println!("fetching messages after {after}");
            }

            let messages = self
                .api
                .fetch_channel_messages(channel_id, Some(after), Some(before), Some(fetch_limit))
                .await?;

            let mut new_count = 0;
            let mut last_id = None;
            let mut insert_stmt = tx.prepare("INSERT INTO messages (id, channel_id, data) VALUES (?, ?, ?) ON CONFLICT DO NOTHING")?;

            for message in messages {
                last_id = Some(last_id.unwrap_or(0).max(message.id));
                new_count +=
                    insert_stmt.execute(params![message.id, message.channel_id, message.data])?;
            }

            drop(insert_stmt);

            if let Some(last_id) = last_id {
                let after_id = range.map_or(after, |(after_id, _)| after_id);

                let new_last_id = match tx.query_row(
                    concat!(
                        "SELECT MAX(last_id) FROM message_ranges ",
                        "WHERE channel_id = ? AND after_id > ? AND after_id <= ?",
                    ),
                    params![channel_id, after_id, last_id],
                    |row| row.get(0),
                )? {
                    Some(new_last_id) => last_id.max(new_last_id),
                    None => last_id,
                };

                let del_count = tx.execute(
                    "DELETE FROM message_ranges WHERE channel_id = ? AND after_id > ? AND after_id <= ?",
                    params![channel_id, after_id, last_id],
                )?;

                if del_count > 0 {
                    println!("merged with {del_count} message ranges")
                }

                tx.execute(
                    concat!(
                        "INSERT INTO message_ranges (channel_id, after_id, last_id) ",
                        "VALUES (?, ?, ?) ",
                        "ON CONFLICT (channel_id, after_id) DO UPDATE SET last_id = EXCLUDED.last_id",
                    ),
                    params![channel_id, after_id, new_last_id],
                )?;
            }

            tx.commit()?;

            if last_id.is_none() {
                break;
            }

            tokio::time::sleep(Duration::from_millis(fetch_interval)).await;
        }

        Ok(())
    }
}
