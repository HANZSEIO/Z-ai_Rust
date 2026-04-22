use std::io::{self, Write};
use z_ai::core::cloud_api::CloudAPI;
use z_ai::core::audio::AudioListener;
use std::sync::Arc;
use std::process::Command;
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let api = Arc::new(CloudAPI::new());
    let listener = Arc::new(AudioListener::new());
    let mut history = Vec::new();
    let mut active_until: Option<Instant> = None;
    let wake_variants = ["z", "zee", "zed", "jay", "see", "the"];

    println!("\n======================================");
    println!("             Z-project ");
    println!("======================================\n");
    println!("OS: {}", std::env::consts::OS);
    println!("Wake Words: {:?} (Aktif 60s)", wake_variants);
    println!("(type 'exit' to exit)");

    // Channel untuk input suara (background listener)
    let (tx_voice, mut rx_voice) = mpsc::channel::<String>(32);
    let listener_clone = Arc::clone(&listener);
    tokio::spawn(async move {
        loop {
            if let Ok(text) = listener_clone.listen_and_record().await {
                if !text.trim().is_empty() {
                    let _ = tx_voice.send(text).await;
                }
            }
        }
    });

    let (tx_text, mut rx_text) = mpsc::channel::<String>(32);
    tokio::spawn(async move {
        loop {
            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_ok() {
                let input = input.trim().to_string();
                if !input.is_empty() {
                    let _ = tx_text.send(input).await;
                }
            }
        }
    });

    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        let is_active = active_until.map(|t| t > Instant::now()).unwrap_or(false);
        
        let input = tokio::select! {
            _ = interval.tick() => {
                if is_active {
                    let remaining = active_until.unwrap().duration_since(Instant::now()).as_secs();
                    print!("\r[Z]: ACTIVE MODE ({}s) - Silakan bicara... ", remaining);
                } else {
                    print!("\r[Z]: IDLE - Sebut 'Z' untuk memulai... ");
                }
                io::stdout().flush()?;
                continue;
            }

            voice_text = rx_voice.recv() => {
                let text = voice_text.unwrap_or_default();
                let text_low = text.to_lowercase();
                
                if !is_active {
                    print!("\r[DEBUG] Z mendengar: \"{}\"          ", text);
                    io::stdout().flush()?;
                    
                    let found_wake = wake_variants.iter().any(|&v| text_low.contains(v));
                    if found_wake {
                        println!("\n[SYSTEM]: Wake word terdeteksi!");
                        active_until = Some(Instant::now() + Duration::from_secs(60));
                        text // Berikan teks aslinya
                    } else {
                        continue;
                    }
                } else {
                    active_until = Some(Instant::now() + Duration::from_secs(60));
                    text
                }
            }

            // Terima teks manual
            text_input = rx_text.recv() => {
                let text = text_input.unwrap_or_default();
                if text == "exit" || text == "quit" { break; }
                active_until = Some(Instant::now() + Duration::from_secs(60));
                text
            }
        };

        println!("\n[YOU]: {}", input);
        print!("[Z] Thinking...");
        io::stdout().flush()?;

        match api.generate_response(&input, &history).await {
            Ok(response) => {
                print!("\r");
                io::stdout().flush()?;

                // DETEKSI AKSI UNIVERSAL
                if response.contains("[ACTION:PLAY_MUSIC:") {
                    if let Some(start) = response.find("[ACTION:PLAY_MUSIC:") {
                        if let Some(end) = response[start..].find("]") {
                            let song_title = &response[start + 19 .. start + end];
                            execute_universal_music(song_title);
                        }
                    }
                }

                let clean_text = if let Some(idx) = response.find("[ACTION:") {
                    response[..idx].trim().to_string()
                } else {
                    response.clone()
                };
                
                println!("[Z]: {}", clean_text);
                
                history.push(("User".to_string(), input.clone()));
                history.push(("Z".to_string(), clean_text));
                if history.len() > 10 { history.drain(0..2); }
            }
            Err(e) => {
                println!("\r[SYSTEM ERROR]: {}", e);
            }
        }
    }

    println!("\n--- SYSTEM Z : SHUTDOWN ---");
    Ok(())
}

fn execute_universal_music(query: &str) {
    let os = std::env::consts::OS;
    println!("\n--- AGENT ACTION: Mencari '{}' di {} ---", query, os);

    match os {
        "macos" => {
            let script = format!("tell application \"Spotify\" to play track \"spotify:search:{}\"", query);
            let _ = Command::new("osascript").arg("-e").arg(script).spawn();
        },
        "windows" => {
            let url = format!("spotify:search:{}", query);
            let _ = Command::new("cmd").args(["/C", "start", &url]).spawn();
        },
        _ => {
            let url = format!("https://music.youtube.com/search?q={}", query);
            let _ = Command::new("xdg-open").arg(url).spawn();
        }
    }
}
