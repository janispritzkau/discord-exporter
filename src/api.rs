use eyre::ContextCompat;

const DISCORD_API_BASE: &str = "https://discord.com/api/v9";

pub struct DiscordApi<'a> {
    http_client: &'a reqwest::Client,
    token: &'a str,
}

impl<'a> DiscordApi<'a> {
    pub fn new(http_client: &'a reqwest::Client, token: &'a str) -> Self {
        Self { http_client, token }
    }

    pub async fn fetch_guild(&self, guild_id: u64) -> eyre::Result<Guild> {
        let json: serde_json::Value = self
            .http_client
            .get(format!("{DISCORD_API_BASE}/guilds/{guild_id}"))
            .header("Authorization", self.token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(Guild {
            id: json["id"].as_str().wrap_err("missing id")?.parse()?,
            data: json,
        })
    }

    pub async fn fetch_channel(&self, channel_id: u64) -> eyre::Result<Channel> {
        let json: serde_json::Value = self
            .http_client
            .get(format!("{DISCORD_API_BASE}/channels/{channel_id}"))
            .header("Authorization", self.token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(Channel {
            id: json["id"].as_str().wrap_err("missing id")?.parse()?,
            guild_id: match json["guild_id"].as_str() {
                Some(s) => Some(s.parse()?),
                None => None,
            },
            data: json,
        })
    }

    pub async fn fetch_channel_messages(
        &self,
        channel_id: u64,
        after: Option<u64>,
        before: Option<u64>,
        limit: Option<u32>,
    ) -> eyre::Result<Vec<ChannelMessage>> {
        let mut query: Vec<(&str, String)> = vec![];

        if let Some(after) = after {
            query.push(("after", after.to_string()));
        }

        if let Some(before) = before {
            query.push(("before", before.to_string()));
        }

        if let Some(limit) = limit {
            query.push(("limit", limit.to_string()));
        }

        let json: Vec<serde_json::Value> = self
            .http_client
            .get(format!("{DISCORD_API_BASE}/channels/{channel_id}/messages"))
            .query(&query)
            .header("Authorization", self.token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(json
            .into_iter()
            .map(|json| {
                Ok(ChannelMessage {
                    id: json["id"].as_str().wrap_err("missing id")?.parse()?,
                    channel_id: json["channel_id"]
                        .as_str()
                        .wrap_err("missing channel_id")?
                        .parse()?,
                    data: json,
                })
            })
            .collect::<eyre::Result<_>>()?)
    }
}

pub struct Guild {
    pub id: u64,
    pub data: serde_json::Value,
}

pub struct Channel {
    pub id: u64,
    pub guild_id: Option<u64>,
    pub data: serde_json::Value,
}

pub struct ChannelMessage {
    pub id: u64,
    pub channel_id: u64,
    pub data: serde_json::Value,
}
