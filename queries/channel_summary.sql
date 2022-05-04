SELECT
  json_extract(g.data, '$.name') AS guild,
  json_extract(c.data, '$.name') AS channel,
  c.id AS channel_id,
  datetime((min(m.id) >> 22) / 1000 + 1420070400, 'unixepoch') AS first_message,
  datetime((max(m.id) >> 22) / 1000 + 1420070400, 'unixepoch') AS last_message,
  count(*) AS message_count
FROM channels c
LEFT JOIN guilds g ON g.id = c.guild_id
LEFT JOIN messages m ON m.channel_id = c.id
GROUP BY c.id
ORDER BY message_count DESC
