use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;

pub struct AudioEngine {
    _stream: Option<OutputStream>,
    _stream_handle: Option<rodio::OutputStreamHandle>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (_stream, _stream_handle) = match OutputStream::try_default() {
            Ok(s) => (Some(s.0), Some(s.1)),
            Err(_) => (None, None),
        };

        Self {
            _stream,
            _stream_handle,
        }
    }

    pub fn play_wav(&self, file_path: &str) {
        if let Some(handle) = &self._stream_handle {
            if let Ok(f) = File::open(file_path) {
                if let Ok(s) = Decoder::new(BufReader::new(f)) {
                    if let Ok(sk) = Sink::try_new(handle) {
                        sk.append(s);
                        sk.sleep_until_end();
                    }
                }
            }
        }
    }
}
