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
                let trimmed = text_low.trim();
                
                let hallucinations = ["thank you", "thanks for watching", "subtitles by", "please subscribe", "thank you.", "thanks for watching."];
                if hallucinations.iter().any(|&h| trimmed == h) {
                    continue; 
                }

                if trimmed.split_whitespace().count() < 2 && !is_active && !wake_variants.iter().any(|&v| trimmed.contains(v)) {
                    continue;
                }

                if !is_active {
                    print!("\r[DEBUG] Z mendengar: \"{}\"          ", text);
                    io::stdout().flush()?;
                    
                    let found_wake = wake_variants.iter().any(|&v| text_low.contains(v));
                    if found_wake {
                        if std::env::consts::OS == "macos" {
                            let _ = Command::new("afplay").arg("/System/Library/Sounds/Tink.aiff").spawn();
                        }
                        
                        println!("\n[SYSTEM]: Wake word terdeteksi!");
                        active_until = Some(Instant::now() + Duration::from_secs(60));
                        text 
                    } else {
                        continue;
                    }
                } else {
                    println!("\n[YOU (Voice)]: {}", text);
                    active_until = Some(Instant::now() + Duration::from_secs(60));
                    text
                }
            }

            text_input = rx_text.recv() => {
                let text = text_input.unwrap_or_default();
                if text == "exit" || text == "quit" { break; }
                active_until = Some(Instant::now() + Duration::from_secs(60));
                text
            }
        };

        println!("\n[YOU]: {}", input);
        
        if std::env::consts::OS == "macos" {
            let _ = Command::new("afplay").arg("/System/Library/Sounds/Pop.aiff").spawn();
        }
        
        print!("[Z] Thinking...");
        io::stdout().flush()?;

        match api.generate_response(&input, &history).await {
            Ok(response) => {
                print!("\r");
                io::stdout().flush()?;

                if response.contains("[ACTION:PLAY_MUSIC:") {
                    if let Some(start) = response.find("[ACTION:PLAY_MUSIC:") {
                        if let Some(end) = response[start..].find("]") {
                            let song_title = &response[start + 19 .. start + end];
                            execute_universal_music("PLAY", song_title);
                        }
                    }
                } else if response.contains("[ACTION:PAUSE_MUSIC]") {
                    execute_universal_music("PAUSE", "");
                } else if response.contains("[ACTION:STOP_MUSIC]") {
                    execute_universal_music("STOP", "");
                }

                let mut clean_text = if let Some(idx) = response.find("[ACTION:") {
                    response[..idx].trim().to_string()
                } else {
                    response.clone()
                };
                
                if clean_text.is_empty() && response.contains("[ACTION:") {
                    clean_text = "Oke, sudah.".to_string();
                }
                
                if !clean_text.is_empty() {
                    println!("[Z]: {}", clean_text);
                    let listener_tts = Arc::clone(&listener);
                    let text_to_speak = clean_text.clone();
                    tokio::spawn(async move {
                        let _ = listener_tts.speak(&text_to_speak).await;
                    });
                }
                
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

fn execute_universal_music(action: &str, query: &str) {
    let os = std::env::consts::OS;
    
    match action {
        "PLAY" => {
            println!("\n--- AGENT ACTION: Memutar '{}' di {} ---", query, os);
            match os {
                "macos" => {
                    let script = format!("tell application \"Spotify\" to play track \"spotify:search:{}\"", query);
                    let _ = Command::new("osascript").arg("-e").arg(script).spawn();
                },
                "linux" => {
                    let _ = Command::new("playerctl").args(["-p", "spotify", "play"]).spawn();
                    let url = format!("https://music.youtube.com/search?q={}", query);
                    let _ = Command::new("xdg-open").arg(url).spawn();
                },
                "windows" => {
                    let url = format!("spotify:search:{}", query);
                    let _ = Command::new("cmd").args(["/C", "start", &url]).spawn();
                },
                _ => {}
            }
        },
        "PAUSE" | "STOP" => {
            println!("\n--- AGENT ACTION: Memberhentikan Musik ---");
            match os {
                "macos" => {
                    let _ = Command::new("osascript").arg("-e").arg("tell application \"Spotify\" to pause").spawn();
                },
                "linux" => {
                    let _ = Command::new("playerctl").arg("pause").spawn();
                },
                "windows" => {
                    let _ = Command::new("powershell").args(["-Command", "(New-Object -ComObject WScript.Shell).SendKeys([char]179)"]).spawn();
                },
                _ => {}
            }
        },
        _ => {}
    }
}
