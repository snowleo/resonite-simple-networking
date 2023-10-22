use std::{env, fs};
use std::path::{Path, PathBuf};
use aes_gcm::{Aes128Gcm, Nonce, aead::{Aead, OsRng, rand_core::RngCore, KeyInit}, AeadCore, KeySizeUser};
use base64::Engine;
use log::{info, warn};
use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};
use crate::access::{AccessCheck, UserId};

pub type Cipher = Aes128Gcm;
pub type Key = aes_gcm::Key<Aes128Gcm>;

pub fn create_keys(key: Key) -> Result<[String; 2], warp::Rejection> {
    let id = OsRng.next_u64();

    let cipher: Cipher = Cipher::new(&key);

    return Ok([
        id_to_encrypted_string(&cipher, id.as_read_only())?,
        id_to_encrypted_string(&cipher, id.as_write_only())?
    ]);
}

fn id_to_encrypted_string(cipher: &Cipher, user_id: UserId) -> Result<String, warp::Rejection> {
    let bytes = user_id.to_be_bytes();
    let nonce = Cipher::generate_nonce(&mut OsRng);
    let result = cipher.encrypt(&nonce, bytes.as_ref()).map_err(|e| {
        warn!("Unable to encrypt bytes {:?} {}", bytes, e);
        warp::reject()
    })?;

    return Ok( URL_SAFE_NO_PAD.encode([nonce.to_vec(), result].concat()));
}

pub fn decrypt_id(user_id: String, key: Key) -> Result<UserId, warp::Rejection> {
    let data = URL_SAFE_NO_PAD.decode(&user_id).map_err(|_| {
        warn!("Unable to base64 decode {:?}", user_id);
        warp::reject()
    })?;
    let cipher: Cipher = Cipher::new(&key);
    if data.len() <= 12 {
        warn!("Key too short {:?}", data);
        return Err(warp::reject::not_found());
    }
    let nonce = Nonce::from_slice(&data[0..12]);
    let ciphertext = &data[12..data.len()];
    let decrypted = cipher.decrypt(nonce, ciphertext)
        .map_err(|_| {
            warn!("Unable to decrypt {:?} {:?}", nonce, ciphertext);
            warp::reject()
        })?;
    let data: [u8; 8] = decrypted.try_into().map_err(|_| warp::reject())?;
    let id = u64::from_be_bytes(data);
    return Ok(id);
}

pub fn load_or_create_key() -> Key {
    return load_key_from_disk().unwrap_or_else(create_key);
}

fn create_key() -> Key {
    let key = Cipher::generate_key(&mut OsRng);
    info!("Created encryption key: {}", URL_SAFE.encode(&key));
    key
}

fn load_key_from_disk() -> Option<Key> {
    env::var("CREDENTIALS_DIRECTORY").ok()
        .and_then(|dir| {
        fs::read_to_string(Path::new(dir.as_str()).join("ENCRYPTION_KEY")).ok()
    }).and_then(|var| {
        URL_SAFE.decode(var.trim()).ok()
    }).and_then(|k| {
        if k.len() >= Cipher::key_size() {
            Some(Key::clone_from_slice(&k[0..Aes128Gcm::key_size()]))
        } else { None }
    })
}

pub struct TlsCredentials {
    pub cert: PathBuf,
    pub key: PathBuf,
}

pub fn load_tls_cert() -> Option<TlsCredentials> {
    env::var("CREDENTIALS_DIRECTORY").ok()
        .and_then(|dir| {
        let base = Path::new(dir.as_str());
        let cert = base.join("TLS_CERT");
        let key = base.join("TLS_KEY");
        if cert.is_file() && key.is_file() {
            Some(TlsCredentials { cert, key })
        } else { None }
    })
}

#[cfg(test)]
mod tests {
    use aes_gcm::aead::{OsRng, KeyInit, rand_core::RngCore};
    use aes_gcm::Aes128Gcm;
    use crate::cipher::{decrypt_id, id_to_encrypted_string, create_key};

    #[test]
    fn encrypt_decrypt() {
        let key = create_key();
        let cipher = Aes128Gcm::new(&key);
        let id = OsRng.next_u64();
        let encrypted = id_to_encrypted_string(&cipher, id).unwrap();
        let decrypted = decrypt_id(encrypted, key).unwrap();
        assert_eq!(decrypted, id);
    }

    #[test]
    fn decrypt_failures() {
        let key = create_key();
        let decrypted = decrypt_id(String::from(""), key.clone());
        assert_eq!(decrypted.is_err(), true);
        let decrypted = decrypt_id(String::from("!"), key.clone());
        assert_eq!(decrypted.is_err(), true);
        let decrypted = decrypt_id(String::from("aaaaaaaaaaaaaaaaaaaa"), key.clone());
        assert_eq!(decrypted.is_err(), true);
    }
}