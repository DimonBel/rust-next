pub async fn analyze_document_with_ai(
    text: &str,
) -> Result<(String, String, String, String), String> {
    let api_key = std::env::var("GROQ_API_KEY")
        .map_err(|_| "GROQ_API_KEY environment variable not set".to_string())?;

    let client = reqwest::Client::new();
    let prompt = format!(
        "Extract the following from the document:\n\
        Summary (3-5 sentences):\n\
        Keywords (comma separated):\n\
        Entities (comma separated):\n\
        Topics (comma separated):\n\
        Document:\n{}",
        &text.chars().take(12000).collect::<String>()
    );

    let body = serde_json::json!({
        "model": "llama-3.1-8b-instant",
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful assistant that extracts structured information from documents. Always answer in the following format: Summary: ... Keywords: ... Entities: ... Topics: ..."
            },
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    let resp = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Groq API error: {}", e))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Groq JSON error: {}", e))?;

    eprintln!("Groq response: {:?}", json);

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("");

    eprintln!("Groq content: {}", content);

    Ok(parse_ai_response(content))
}

fn parse_ai_response(content: &str) -> (String, String, String, String) {
    let summary = content
        .split("Keywords:")
        .next()
        .unwrap_or("")
        .replace("Summary:", "")
        .trim()
        .to_string();

    let keywords = content
        .split("Keywords:")
        .nth(1)
        .and_then(|s| s.split("Entities:").next())
        .unwrap_or("")
        .trim()
        .to_string();

    let entities = content
        .split("Entities:")
        .nth(1)
        .and_then(|s| s.split("Topics:").next())
        .unwrap_or("")
        .trim()
        .to_string();

    let topics = content
        .split("Topics:")
        .nth(1)
        .unwrap_or("")
        .trim()
        .to_string();

    (summary, keywords, entities, topics)
}
