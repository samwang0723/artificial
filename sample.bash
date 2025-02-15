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

IMAGE_URL="https://upload.wikimedia.org/wikipedia/commons/a/a7/Camponotus_flavomarginatus_ant.jpg"
IMAGE_MEDIA_TYPE="image/jpeg"
IMAGE_BASE64=$(curl "$IMAGE_URL" | base64)

curl https://api.anthropic.com/v1/messages \
     --header "x-api-key: $ANTHROPIC_API_KEY" \
     --header "anthropic-version: 2023-06-01" \
     --header "content-type: application/json" \
     --data \
'{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 1024,
    "messages": [
        {"role": "user", "content": [
            {"type": "image", "source": {
                "type": "base64",
                "media_type": "'$IMAGE_MEDIA_TYPE'",
                "data": "'$IMAGE_BASE64'"
            }},
            {"type": "text", "text": "What is in the above image?"}
        ]}
    ]
}'
