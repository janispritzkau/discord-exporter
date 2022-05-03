#![allow(unused)]

mod migration;

use crate::migration::run_migrations;
use chrono::DateTime;
use clap::StructOpt;
use eyre::Context;

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
        #[clap(long, default_value = "50")]
        fetch_limit: u64,
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

    match args.command {
        Command::Download {
            channel_id,
            after,
            before,
            fetch_limit,
        } => {
            let token = std::env::var("TOKEN").wrap_err("TOKEN environment required")?;

            let after: u64 = after.map_or(0, Into::into);
            let before: u64 = before.map_or(
                MessageInput::DateTime(chrono::Utc::now().naive_utc()).into(),
                Into::into,
            );

            download_messages(DownloadMessagesOpts {
                db_conn: &mut db_conn,
                http_client: &http_client,
                token: &token,
                channel_id,
            })
            .await?;
        }
    }

    Ok(())
}

#[derive(Debug)]
struct DownloadMessagesOpts<'a> {
    db_conn: &'a mut rusqlite::Connection,
    http_client: &'a reqwest::Client,
    token: &'a str,
    channel_id: u64,
}

async fn download_messages<'a>(opts: DownloadMessagesOpts<'a>) -> eyre::Result<()> {
    println!("{:?}", opts);

    let DownloadMessagesOpts {
        db_conn,
        http_client,
        token,
        channel_id,
    } = opts;

    Ok(())
}
