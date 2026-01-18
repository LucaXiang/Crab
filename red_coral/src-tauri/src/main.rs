#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tokio::main]
async fn main() {
    red_coral_lib::run().await;
}
