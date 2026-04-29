# Z-AI: The Savage Personal Assistant

**Z-AI** is a high-performance, multi-modal personal assistant built in Rust. Unlike traditional assistants, Z is designed with a "Toxic, Savage, and Unfiltered" personality, offering a unique (and often roasting) interaction experience.

It supports voice and text input, integrates with multiple LLM providers, and can control system media across macOS, Linux, and Windows.

## Features

- **Multi-Modal Input**: Seamlessly switch between voice (Wake Word activated) and keyboard input.
- **Aggressive Personality**: Powered by custom system prompts to be sarky, toxic, and brutally honest.
- **Triple-Provider LLM Support**: 
  - **Google Gemini** (Primary: 2.0 Flash)
  - **OpenAI** (gpt-4o-mini)
  - **Groq/xAI** (Llama 3.3 / Grok-Beta)
- **High-Fidelity Voice**: 
  - **STT**: Fast transcription via Groq (Whisper-large-v3).
  - **TTS**: Premium voice synthesis via ElevenLabs, with local system fallbacks.
- **Universal Music Control**: Control Spotify or YouTube Music via voice commands (`PLAY`, `PAUSE`, `STOP`).
- **Cross-Platform**: Tailored commands for macOS (`osascript`), Linux (`playerctl`), and Windows (`powershell`).

## Getting Started

### Prerequisites

- **Rust**: [Install Rust](https://www.rust-lang.org/tools/install) (Edition 2021).
- **System Dependencies**:
  - **Linux**: `alsa` development files (e.g., `libasound2-dev`), `playerctl`.
  - **macOS**: Native support.
  - **Windows**: PowerShell.

### Environment Setup

Create a `.env` file in the root directory and add your API keys:

```env
GEMINI_API_KEY=your_gemini_key
OPENAI_API_KEY=your_openai_key
GROQ_API_KEY=your_groq_key
ELEVEN_LABS_API_KEY=your_elevenlabs_key
```

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/HANZSEIO/Z-ai_Rust.git
   cd Z-ai_Rust
   ```

2. Run the application:
   ```bash
   cargo run --release
   ```

## Usage

1. **Wake Word**: Say "Z", "Zee", or "Zed" to activate the assistant.
2. **Active Mode**: Once activated (indicated by a sound and UI prompt), you have 60 seconds to speak.
3. **Music Control**: Try saying:
   - *"Z, play some Phonk"*
   - *"Stop the music"*
4. **Text Mode**: Simply type your message in the terminal at any time.
5. **Exit**: Type `exit` or `quit`.

## Architecture

- **`src/main.rs`**: Orchestrates the async event loop using `tokio`, managing concurrent audio listening and terminal input.
- **`src/core/audio.rs`**: Handles low-level audio stream management (`cpal`), WAV encoding (`hound`), and STT/TTS API calls.
- **`src/core/cloud_api.rs`**: Manages the LLM logic, system prompts, and fallback mechanisms between providers.

## Disclaimer

Z-AI is configured to be **toxic and unfiltered**. It is intended for entertainment purposes and "toxic friend" simulations. But you can configure manualy Use at your own risk :).

---
*Built with 🦀 and a lot of attitude.*

by HANZS -
