CREATE TABLE guids (
  id INT PRIMARY KEY,
  data TEXT NOT NULL
);

CREATE TABLE channels (
  id INT PRIMARY KEY,
  guild_id INT NOT NULL REFERENCES guilds(id)
);

CREATE TABLE messages (
  id INT PRIMARY KEY,
  channel_id INT NOT NULL REFERENCES channels(id),
  data TEXT NOT NULL
);

CREATE TABLE message_ranges (
  channel_id INT NOT NULL REFERENCES channels(id),
  from_id INT NOT NULL,
  to_id INT NOT NULL,
  UNIQUE (channel_id, from_id)
);
