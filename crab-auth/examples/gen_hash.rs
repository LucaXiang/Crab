use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};

fn main() {
    let password = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "test123".to_string());
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();
    println!("{hash}");
}
