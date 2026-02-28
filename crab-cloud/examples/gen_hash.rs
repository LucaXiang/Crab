use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use argon2::{Argon2, PasswordHasher};

fn main() {
    let password = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "devpassword123".into());
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("hash failed");
    println!("{hash}");
}
