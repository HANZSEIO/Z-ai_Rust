use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use hound::{WavSpec, WavWriter};
use std::io::Cursor;
use reqwest::Client;
use serde_json::Value;

pub struct AudioListener {
    client: Client,
    groq_key: String,
}

impl AudioListener {
    pub fn new() -> Self {
        dotenv::dotenv().ok();
        let groq_key = std::env::var("GROQ_API_KEY")
            .or_else(|_| std::env::var("GROK_API_KEY"))
            .unwrap_or_default();
        Self {
            client: Client::new(),
            groq_key,
        }
    }

    pub async fn listen_and_record(&self) -> anyhow::Result<String> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device found"))?;

        let config = device.default_input_config()?;
        let sample_rate: u32 = config.sample_rate().into(); // Use into() or just the value if it's already u32
        let channels = config.channels();

        let recording = Arc::new(Mutex::new(Vec::new()));
        let is_recording = Arc::new(Mutex::new(false));
        let last_activity = Arc::new(Mutex::new(Instant::now()));

        let recording_clone = Arc::clone(&recording);
        let is_recording_clone = Arc::clone(&is_recording);
        let last_activity_clone = Arc::clone(&last_activity);

        let threshold = 0.08; // Lebih kebal terhadap ketukan kecil
        let silence_duration = Duration::from_millis(1500); // Berhenti setelah 1.5 detik hening

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
            return Err(anyhow::anyhow!("GROQ_API_KEY not found for Groq STT"));
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
