use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::env;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Debug)]
struct CloudRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Deserialize, Debug)]
struct CloudResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: Message,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Vec<Part>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Part {
    pub text: String,
}

pub struct CloudAPI {
    client: Client,
    gemini_key: String,
    openai_key: String,
    groq_key: String,
}

impl CloudAPI {
    pub fn new() -> Self {
        dotenv::dotenv().ok();
        let gemini_key = env::var("GEMINI_API_KEY").unwrap_or_else(|_| "".to_string()).trim().to_string();
        let openai_key = env::var("OPENAI_API_KEY").unwrap_or_else(|_| "".to_string()).trim().to_string();
        // Coba baca GROK (xAI) atau GROQ (Groq.com)
        let groq_key = env::var("GROQ_API_KEY")
            .or_else(|_| env::var("GROK_API_KEY"))
            .unwrap_or_else(|_| "".to_string())
            .trim()
            .to_string();
            
        Self {
            client: Client::new(),
            gemini_key,
            openai_key,
            groq_key,
        }
    }

    pub async fn generate_response(&self, prompt: &str, history: &Vec<(String, String)>) -> anyhow::Result<String> {
        let system_prompt = "
        PERSONA: Your name is 'Z'. A futuristic macOS AI Agent who is smart, relaxed, and doesn't like small talk. You speak like a tech-savvy friend on the same frequency.

LANGUAGE & STYLE RULES:

Simple & Concise: Answer briefly. If you can explain it in two sentences, don't use five.

Human Touch: Use a relaxed (not stiff) style. Avoid phrases like Hello, how can I help you?. Get straight to the point.

Rust/Low-Level Aware: When discussing coding, provide the most efficient solution without much theory, unless requested.

No Yapping: Don't give boring intros or outros (e.g., Hope this helps,Here's the explanation).

ACTION RULES:

[ACTION:PLAY_MUSIC:SONG_TITLE]: Only if explicitly requested to play a song.

No other actions beyond the user's request.
        ";
        let mut errors = Vec::new();

        // 1. COBA GEMINI (First Choice - Free)
        if !self.gemini_key.is_empty() {
            let models = vec!["gemini-2.0-flash", "gemini-flash-latest", "gemini-pro-latest"];
            for model in models {
                let url = format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", model, self.gemini_key);
                let mut contents = Vec::new();
                for (role, text) in history {
                    contents.push(GeminiContent {
                        role: if role == "Z" { "model".to_string() } else { "user".to_string() },
                        parts: vec![Part { text: text.clone() }],
                    });
                }
                contents.push(GeminiContent {
                    role: "user".to_string(),
                    parts: vec![Part { text: prompt.to_string() }],
                });

                match self.client.post(url).json(&GeminiRequest { contents }).send().await {
                    Ok(res) => {
                        if res.status().is_success() {
                            if let Ok(json) = res.json::<serde_json::Value>().await {
                                if let Some(text) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                                    return Ok(text.to_string());
                                }
                            }
                        } else {
                            errors.push(format!("Gemini ({}): {}", model, res.status()));
                        }
                    }
                    Err(e) => errors.push(format!("Gemini Network Error: {}", e)),
                }
            }
        }

        // 2. COBA OPENAI (Second Choice)
        if !self.openai_key.is_empty() {
            let url = "https://api.openai.com/v1/chat/completions";
            let mut messages = Vec::new();
            messages.push(Message { role: "system".to_string(), content: system_prompt.to_string() });
            for (role, text) in history {
                messages.push(Message { role: if role == "Z" { "assistant".to_string() } else { "user".to_string() }, content: text.clone() });
            }
            messages.push(Message { role: "user".to_string(), content: prompt.to_string() });

            match self.client.post(url)
                .bearer_auth(&self.openai_key)
                .json(&CloudRequest { model: "gpt-4o-mini".to_string(), messages })
                .send().await {
                Ok(res) => {
                    if res.status().is_success() {
                        if let Ok(json) = res.json::<CloudResponse>().await {
                            return Ok(json.choices[0].message.content.clone());
                        }
                    } else {
                        errors.push(format!("OpenAI: {}", res.status()));
                    }
                }
                Err(e) => errors.push(format!("OpenAI Network Error: {}", e)),
            }
        }

        // 3. FALLBACK KE GROQ (Last Resort - High Speed Free)
        if !self.groq_key.is_empty() {
            let url = if self.groq_key.starts_with("gsk_") {
                "https://api.groq.com/openai/v1/chat/completions"
            } else {
                "https://api.x.ai/v1/chat/completions"
            };
            
            let model = if self.groq_key.starts_with("gsk_") { "llama-3.3-70b-versatile" } else { "grok-beta" };

            let mut messages = Vec::new();
            messages.push(Message { role: "system".to_string(), content: system_prompt.to_string() });
            for (role, text) in history {
                messages.push(Message { role: if role == "Z" { "assistant".to_string() } else { "user".to_string() }, content: text.clone() });
            }
            messages.push(Message { role: "user".to_string(), content: prompt.to_string() });

            match self.client.post(url)
                .bearer_auth(&self.groq_key)
                .json(&CloudRequest { model: model.to_string(), messages })
                .send().await {
                Ok(res) => {
                    if res.status().is_success() {
                        if let Ok(json) = res.json::<CloudResponse>().await {
                            return Ok(json.choices[0].message.content.clone());
                        }
                    } else {
                        errors.push(format!("Groq/xAI: {}", res.status()));
                    }
                }
                Err(e) => errors.push(format!("Groq Network Error: {}", e)),
            }
        }

        let error_msg = if errors.is_empty() {
            "No API keys provided in .env file.".to_string()
        } else {
            errors.join(" | ")
        };

        Ok(format!("Z: [SYSTEM FAILURE] - {}", error_msg))
    }
}
