curl https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
  "messages": [
    {
      "content": [
        {
          "text": "what is this",
          "type": "text"
        },
        {
          "image_url": {
            "url": "data:image/png;base64,aHR0cHM6Ly9pLmltZ3VyLmNvbS91Q2pHRll6LnBuZw=="
          },
          "type": "image_url"
        }
      ],
      "role": "user"
    }
  ],
  "model": "o1-preview",
  "stream": true,
  "user": "3725d597-c822-46b7-9cf2-4f5fd980d320"
}'

curl -v https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
  "messages": [
    {
      "content": [
        {
          "text": "hellp",
          "type": "text"
        }
      ],
      "role": "user"
    }
  ],
  "model": "o1",
  "stream": false,
  "user": "b4d7b1ac-05e2-4c40-8cad-f7e0839256b3"
}'
