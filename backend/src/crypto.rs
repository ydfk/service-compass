use std::fs;

use aes_gcm::{
    Aes256Gcm, KeyInit,
    aead::{Aead, OsRng, rand_core::RngCore},
};
use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};

use crate::config::Config;

pub struct SecretBox {
    cipher: Aes256Gcm,
}

impl SecretBox {
    pub fn load(config: &Config) -> Result<Self> {
        let material = if let Some(secret) = &config.secret_key {
            secret.as_bytes().to_vec()
        } else if config.secret_file.exists() {
            fs::read(&config.secret_file)?
        } else {
            let mut generated = [0_u8; 32];
            OsRng.fill_bytes(&mut generated);
            if let Some(parent) = config.secret_file.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&config.secret_file, generated)?;
            tracing::warn!(path = %config.secret_file.display(), "已生成本地加密密钥，请妥善备份");
            generated.to_vec()
        };
        let key = Sha256::digest(material);
        Ok(Self {
            cipher: Aes256Gcm::new(&key),
        })
    }

    pub fn encrypt(&self, value: &str) -> Result<String> {
        let mut nonce = [0_u8; 12];
        OsRng.fill_bytes(&mut nonce);
        let encrypted = self
            .cipher
            .encrypt((&nonce).into(), value.as_bytes())
            .map_err(|_| anyhow::anyhow!("敏感配置加密失败"))?;
        let mut payload = nonce.to_vec();
        payload.extend(encrypted);
        Ok(STANDARD.encode(payload))
    }

    pub fn decrypt(&self, value: &str) -> Result<String> {
        let payload = STANDARD.decode(value).context("敏感配置编码无效")?;
        let (nonce, encrypted) = payload.split_at_checked(12).context("敏感配置内容无效")?;
        let decrypted = self
            .cipher
            .decrypt(nonce.into(), encrypted)
            .map_err(|_| anyhow::anyhow!("敏感配置解密失败"))?;
        String::from_utf8(decrypted).context("敏感配置不是有效文本")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypted_secret_can_round_trip() {
        let cipher = SecretBox {
            cipher: Aes256Gcm::new(&Sha256::digest(b"test-key")),
        };
        let encrypted = cipher.encrypt("password").unwrap();
        assert_ne!(encrypted, "password");
        assert_eq!(cipher.decrypt(&encrypted).unwrap(), "password");
    }
}
