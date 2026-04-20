use std::io::{self, Write};
use z_ai::core::cloud_api::CloudAPI;
use std::sync::Arc;
use std::process::Command;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let api = Arc::new(CloudAPI::new());
    let mut history = Vec::new();

    println!("\n======================================");
    println!("             Z-project ");
    println!("======================================\n");
    println!("OS: {}", std::env::consts::OS);
    println!("(type 'exit' to exit)");

    loop {
        print!("\n[YOU]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() { continue; }
        if input == "exit" || input == "quit" { break; }

        print!("[Z] Thinking...");
        io::stdout().flush()?;

        match api.generate_response(input, &history).await {
            Ok(response) => {
                print!("\r"); // Hapus indikator thinking
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
                
                history.push(("User".to_string(), input.to_string()));
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
            // Di Windows kita buka via URL protocol spotify:search
            let url = format!("spotify:search:{}", query);
            let _ = Command::new("cmd").args(["/C", "start", &url]).spawn();
        },
        _ => {
            // Linux atau lainnya: Buka browser ke YouTube Music sebagai fallback universal
            let url = format!("https://music.youtube.com/search?q={}", query);
            let _ = Command::new("xdg-open").arg(url).spawn();
        }
    }
}
