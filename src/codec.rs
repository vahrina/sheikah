use serde::{
    Serialize,
    Deserialize,
};
use std::{
    fs::{read_to_string, write, rename},
    io,
};
use base64::{
    Engine,
    engine::general_purpose::STANDARD,
};

#[derive(Serialize, Deserialize, Debug)]
struct Eye {
    #[serde(rename = "v")]
    version: u8,
    #[serde(rename = "s")]
    salt: String,
    #[serde(rename = "n")]
    nonce: String,
    #[serde(rename = "c")]
    cipher: String,
}

fn write_to_file(path: &str, data: &Eye) -> io::Result<()> {
    let json = serde_json::to_string_pretty(data)?;
    let tmp = format!("{path}.tmp");
    write(&tmp, json)?;
    rename(tmp, path)
}

fn read_from_file(path: &str) -> io::Result<Eye> {
    let data = read_to_string(path)?;
    let file: Eye = serde_json::from_str(&data)
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(file)
}

fn build_file(salt: &[u8], nonce: &[u8], cipher: &[u8]) -> Eye {
    Eye {
        version: 1,
        salt: STANDARD.encode(salt),
        nonce: STANDARD.encode(nonce),
        cipher: STANDARD.encode(cipher),
    }
}

fn decode_file(file: &Eye) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), base64::DecodeError> {
    let salt = STANDARD.decode(&file.salt)?;
    let nonce = STANDARD.decode(&file.nonce)?;
    let cipher = STANDARD.decode(&file.cipher)?;
    Ok((salt, nonce, cipher))
}
