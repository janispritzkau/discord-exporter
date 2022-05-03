CREATE TABLE guilds (
  id INT PRIMARY KEY,
  data TEXT NOT NULL,
  updated_at INT NOT NULL
);

CREATE TABLE channels (
  id INT PRIMARY KEY,
  guild_id INT REFERENCES guilds(id),
  data TEXT NOT NULL,
  updated_at INT NOT NULL
);

CREATE INDEX channels_guild_id_idx ON channels (guild_id);

CREATE TABLE messages (
  id INT PRIMARY KEY,
  channel_id INT NOT NULL REFERENCES channels(id),
  data TEXT NOT NULL
);

CREATE INDEX messages_channel_id_idx ON messages (channel_id);

CREATE TABLE message_ranges (
  channel_id INT NOT NULL REFERENCES channels(id),
  after_id INT NOT NULL,
  last_id INT NOT NULL,
  UNIQUE (channel_id, after_id)
);

CREATE INDEX message_ranges_last_idx ON message_ranges (last_id);
