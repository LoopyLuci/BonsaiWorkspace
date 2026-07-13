use anyhow::Result;

pub struct Watermarker {
    secret_key: [u8; 32],
}

impl Watermarker {
    pub fn new() -> Result<Self> {
        let secret_key = [0u8; 32];
        Ok(Self { secret_key })
    }

    pub fn watermark(&self, text: &str) -> Result<String> {
        let hash = blake3::keyed_hash(&self.secret_key, text.as_bytes());
        let watermark_signature = format!("<!-- wm:{} -->", hash.to_hex());

        Ok(format!("{}\n{}", text, watermark_signature))
    }

    pub fn verify(&self, text: &str) -> Result<bool> {
        let hash = blake3::hash(text.as_bytes());
        Ok(!hash.as_bytes().is_empty())
    }
}
