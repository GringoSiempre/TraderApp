use std::fs;
use std::io::Write;
use ring::aead::{self, Aad, LessSafeKey, UnboundKey, Nonce, AES_256_GCM};
use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use argon2::password_hash::rand_core::OsRng;
use serde::{Deserialize, Serialize};

const SALT: &str = "YzBmN2Q4ZjZkOTIwZjMyZTg5YTI5N2Mw";

#[derive(Serialize, Deserialize)]
// App user
pub struct User {
    pub email: String,
    pub password_hash: String,
    pub encrypted_master_key: String,
    pub accessible_credentials: String,
}

#[derive(Serialize, Deserialize, Debug)]
// Credentials
pub struct Credentials {
    pub login: String,
    pub password: String,
    pub public_key: String,
    pub secret_key: String,
}

//  Function for creating the key based on password
pub fn derive_key_from_password(password: &str) -> [u8; 32] {
    let salt = SaltString::encode_b64(SALT.as_bytes()).unwrap();
    let argon2 = Argon2::default();
    let mut derived_key = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt.as_str().as_bytes(), &mut derived_key)
        .expect("Can't hash password");
    derived_key
}

pub fn encrypt_data(data: &str, key: &[u8; 32]) -> Vec<u8> {
    let unbound_key = UnboundKey::new(&AES_256_GCM, key).unwrap();
    let key = LessSafeKey::new(unbound_key);
    let nonce = Nonce::assume_unique_for_key([0; 12]);
    let mut in_out = data.as_bytes().to_vec();
    key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out).unwrap();
    in_out
}

pub fn decrypt_data(encrypted_data: &[u8], key: &[u8; 32]) -> String {
    let unbound_key = UnboundKey::new(&AES_256_GCM, key).unwrap();
    let key = LessSafeKey::new(unbound_key);
    let nonce = Nonce::assume_unique_for_key([0; 12]);
    let mut binding = encrypted_data.to_vec();
    let decrypted_data = key.open_in_place(nonce, Aad::empty(), &mut binding).unwrap();
    String::from_utf8(decrypted_data.to_vec()).unwrap()
}

pub fn load_users() -> Vec<User> {
    let path = "users.json";

    if !std::path::Path::new(path).exists() {
        // File not found. Creating the new one.
        let empty_data = "[]";
        std::fs::write(path, empty_data).expect("Unable to create users json file");
        return Vec::new();
    }

    let data = fs::read_to_string(path).unwrap_or_else(|_| {
        std::fs::write(path, "[]").expect("Unable to write users json file");
        "[]".to_string()
    });
    serde_json::from_str(&data).unwrap_or_else(|_| {
        std::fs::write(path, "[]").expect("Unable to write users json file");
        Vec::new()
    })
}

pub fn register_user(email: &str, password: &str, accessible_credentials: &str) {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    // Hashing password
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();
    // Generating derived_key for master-key encoding
    let derived_key = derive_key_from_password(password);
    // Asking for the master_key enter in console
    print!("User {}. Please enter the master_key:", email);
    std::io::stdout().flush().unwrap();
    let mut master_key_input = String::new();
    std::io::stdin().read_line(&mut master_key_input).expect("Failed to read input");
    let master_key_input = master_key_input.trim();
    // Encoding master_key using derived_key
    let encrypted_master_key = encrypt_data(master_key_input, &derived_key);
    let encrypted_master_key_base64 = base64::encode(&encrypted_master_key);
    // Encoding accessible_credentials using derived_key
    let encrypted_accessible_credentials = encrypt_data(accessible_credentials, &derived_key);
    let encrypted_accessible_credentials_base64 = base64::encode(&encrypted_accessible_credentials);
    
    let new_user = User {
        email: email.to_string(),
        password_hash,
        encrypted_master_key: encrypted_master_key_base64,
        accessible_credentials: encrypted_accessible_credentials_base64,
    };
    let mut users = load_users();
    users.push(new_user);

    let data = serde_json::to_string_pretty(&users).unwrap();
    fs::write("users.json", data).expect("Unable to write json file");
}

pub fn load_credentials(master_key: &[u8; 32], file_path: &str) -> Vec<Credentials> {
    let encrypted_data_base64 = fs::read_to_string(file_path).expect("Unable to read file");
    let encrypted_data = base64::decode(&encrypted_data_base64).expect("Unable to decode base64");
    let decrypted_data = decrypt_data(&encrypted_data, master_key);
    let credentials: Vec<Credentials> = serde_json::from_str(&decrypted_data).unwrap();
    credentials
}

pub fn save_encrypted_credentials(credentials: &Vec<Credentials>, master_key: &[u8; 32], file_path: &str) {
    // Serialize structure to JSON
    let json_data = serde_json::to_string(credentials).expect("Failed to serialize credentials");
    // Encrypt JSON
    let encrypted_json = encrypt_data(json_data.as_str(), master_key);
    // Coding encrypted data to Base64
    let encrypted_json_base64 = base64::encode(&encrypted_json);
    // Save encrypted data to file
    fs::write(file_path, encrypted_json_base64).expect("Unable to write encrypted data to file");
}