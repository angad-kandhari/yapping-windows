//! sherpa-onnx offline transducer wrapper around Parakeet-TDT 0.6B v2.
//! Decode happens once per release; sherpa-rs 0.6 has no streaming API
//! and the app has no live-text UI, so offline is the right shape.

use sherpa_rs::transducer::{TransducerConfig, TransducerRecognizer};
use std::path::Path;

pub struct Engine {
    recognizer: TransducerRecognizer,
}

impl Engine {
    pub fn new(model_dir: &Path) -> anyhow::Result<Self> {
        let path = |name: &str| model_dir.join(name).to_string_lossy().into_owned();
        let config = TransducerConfig {
            encoder: path("encoder.int8.onnx"),
            decoder: path("decoder.int8.onnx"),
            joiner: path("joiner.int8.onnx"),
            tokens: path("tokens.txt"),
            model_type: "nemo_transducer".into(),
            num_threads: 4,
            sample_rate: 16_000,
            feature_dim: 80,
            decoding_method: "greedy_search".into(),
            debug: false,
            ..Default::default()
        };
        let recognizer = TransducerRecognizer::new(config)
            .map_err(|e| anyhow::anyhow!("speech model failed to load: {e}"))?;
        Ok(Self { recognizer })
    }

    /// Expects 16 kHz mono f32 samples.
    pub fn transcribe(&mut self, samples: &[f32]) -> String {
        if samples.is_empty() {
            return String::new();
        }
        self.recognizer.transcribe(16_000, samples)
    }
}
