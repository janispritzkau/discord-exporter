SELECT
  CAST(data ->> '$.author.id' AS INT) AS id,
  data ->> '$.author.username' AS name,
  COUNT(*) AS message_count
FROM messages
GROUP BY 1
ORDER BY message_count DESC
LIMIT 20
