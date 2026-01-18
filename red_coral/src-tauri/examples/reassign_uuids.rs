use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use std::env;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --example reassign_uuids -- <PATH_TO_POS.DB>");
        eprintln!("Example: cargo run --example reassign_uuids -- \"C:\\Users\\xzy\\AppData\\Roaming\\com.xzy.pos\\pos.db\"");
        return Ok(());
    }
    let db_path = &args[1];

    println!("Connecting to database at: {}", db_path);

    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path))?
        .create_if_missing(false)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(false);

    let pool = SqlitePoolOptions::new()
        .connect_with(opts)
        .await
        .map_err(|e| {
            eprintln!("\n❌ 数据库连接失败！");
            eprintln!("请确保路径正确，且 POS 应用程序已关闭。");
            eprintln!("错误详情: {}", e);
            Box::new(e) as Box<dyn std::error::Error>
        })?;

    // Reset Printer Settings
    reset_printer_settings(&pool).await?;

    println!("\n✨ 任务全部完成。请重新启动 POS 应用程序以查看更改。");

    Ok(())
}

async fn reset_printer_settings(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n正在重置所有菜品和分类的打印机设置 (启用打印, 设置为默认)...");

    // 1. Update Products
    let product_res =
        sqlx::query("UPDATE products SET is_kitchen_print_enabled = 1, kitchen_printer_id = NULL")
            .execute(pool)
            .await;

    match product_res {
        Ok(res) => println!("✅ 已更新 {} 个菜品的打印设置。", res.rows_affected()),
        Err(e) => eprintln!("❌ 更新菜品打印设置失败: {}", e),
    }

    // 2. Update Categories
    let category_res = sqlx::query(
        "UPDATE categories SET is_kitchen_print_enabled = 1, kitchen_printer_id = NULL",
    )
    .execute(pool)
    .await;

    match category_res {
        Ok(res) => println!("✅ 已更新 {} 个分类的打印设置。", res.rows_affected()),
        Err(e) => eprintln!("❌ 更新分类打印设置失败: {}", e),
    }

    Ok(())
}
