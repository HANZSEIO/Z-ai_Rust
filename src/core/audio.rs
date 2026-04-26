use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use hound::{WavSpec, WavWriter};
use std::io::Cursor;
use reqwest::Client;
use serde_json::{Value, json};
use rodio::{Decoder, OutputStream, Sink};

pub struct AudioListener {
    client: Client,
    groq_key: String,
    elevenlabs_key: String,
}

impl AudioListener {
    pub fn new() -> Self {
        dotenv::dotenv().ok();
        let groq_key = std::env::var("GROQ_API_KEY")
            .or_else(|_| std::env::var("GROK_API_KEY"))
            .unwrap_or_default();
        let elevenlabs_key = std::env::var("ELEVEN_LABS_API_KEY").unwrap_or_default();
        
        Self {
            client: Client::new(),
            groq_key,
            elevenlabs_key,
        }
    }

    pub async fn speak(&self, text: &str) -> anyhow::Result<()> {
        if !self.elevenlabs_key.is_empty() {
            match self.speak_elevenlabs(text).await {
                Ok(_) => return Ok(()),
                Err(_) => {} 
            }
        }

        self.speak_system(text).await
    }

    async fn speak_elevenlabs(&self, text: &str) -> anyhow::Result<()> {
        let voice_id = "EXAVITQu4vr4xnSDxMaL"; 
        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}", voice_id);

        let res = self.client.post(url)
            .header("xi-api-key", &self.elevenlabs_key)
            .json(&json!({
                "text": text,
                "model_id": "eleven_multilingual_v2",
                "voice_settings": {
                    "stability": 0.4,
                    "similarity_boost": 0.8
                }
            }))
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!("ElevenLabs Error"));
        }

        let audio_data = res.bytes().await?;
        self.play_audio(audio_data.to_vec()).await
    }

    async fn play_audio(&self, audio_data: Vec<u8>) -> anyhow::Result<()> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        let cursor = Cursor::new(audio_data);
        let source = Decoder::new(cursor)?;
        
        sink.append(source);
        sink.sleep_until_end();
        Ok(())
    }

    async fn speak_system(&self, text: &str) -> anyhow::Result<()> {
        let clean_text = text.replace("\"", "").replace("'", "");
        let os = std::env::consts::OS;

        match os {
            "macos" => {
                let _ = std::process::Command::new("say")
                    .arg(clean_text)
                    .spawn();
            },
            "linux" => {
                let _ = std::process::Command::new("spd-say")
                    .arg("-t")
                    .arg("female1")
                    .arg(clean_text)
                    .spawn();
            },
            "windows" => {
                let ps_command = format!("Add-Type -AssemblyName System.Speech; (New-Object System.Speech.Synthesis.SpeechSynthesizer).Speak('{}')", clean_text);
                let _ = std::process::Command::new("powershell")
                    .arg("-Command")
                    .arg(ps_command)
                    .spawn();
            },
            _ => {}
        }
        Ok(())
    }

    pub async fn listen_and_record(&self) -> anyhow::Result<String> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device found"))?;

        let config = device.default_input_config()?;
        let sample_rate: u32 = config.sample_rate().into(); 
        let channels = config.channels();

        let recording = Arc::new(Mutex::new(Vec::new()));
        let is_recording = Arc::new(Mutex::new(false));
        let last_activity = Arc::new(Mutex::new(Instant::now()));

        let recording_clone = Arc::clone(&recording);
        let is_recording_clone = Arc::clone(&is_recording);
        let last_activity_clone = Arc::clone(&last_activity);

        let threshold = 0.08; 
        let silence_duration = Duration::from_millis(2000); 

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut max_val = 0.0;
                for &sample in data {
                    if sample.abs() > max_val { max_val = sample.abs(); }
                }

                let mut recording_guard = recording_clone.lock().unwrap();
                let mut is_rec = is_recording_clone.lock().unwrap();
                let mut last_act = last_activity_clone.lock().unwrap();

                if max_val > threshold {
                    if !*is_rec {
                        *is_rec = true;
                    }
                    *last_act = Instant::now();
                }

                if *is_rec {
                    recording_guard.extend_from_slice(data);
                    if last_act.elapsed() > silence_duration {
                        *is_rec = false;
                    }
                }
            },
            |err| eprintln!("Stream error: {}", err),
            None
        )?;

        stream.play()?;

        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let is_rec = is_recording.lock().unwrap();
            let recording_len = recording.lock().unwrap().len();
            if !*is_rec && recording_len > 0 {
                break;
            }
        }

        drop(stream); 

        let samples = recording.lock().unwrap().clone();
        if samples.is_empty() {
            return Ok("".to_string());
        }

        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = WavWriter::new(&mut cursor, spec)?;
            for sample in samples {
                writer.write_sample(sample)?;
            }
            writer.finalize()?;
        }

        let wav_data = cursor.into_inner();
        self.speech_to_text(wav_data).await
    }

    async fn speech_to_text(&self, wav_data: Vec<u8>) -> anyhow::Result<String> {
        if self.groq_key.is_empty() {
            return Err(anyhow::anyhow!("GROQ_API_KEY tidak ditemukan untuk Groq STT"));
        }

        let part = reqwest::multipart::Part::bytes(wav_data)
            .file_name("audio.wav")
            .mime_str("audio/wav")?;

        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", "whisper-large-v3");

        let res = self.client.post("https://api.groq.com/openai/v1/audio/transcriptions")
            .bearer_auth(&self.groq_key)
            .multipart(form)
            .send()
            .await?;

        if res.status().is_success() {
            let json: Value = res.json().await?;
            Ok(json["text"].as_str().unwrap_or_default().to_string())
        } else {
            let err_text = res.text().await?;
            Err(anyhow::anyhow!("Groq STT Error: {}", err_text))
        }
    }
}
