//! Minimal reproduction: SurrealDB embedded SDK LIMIT bug
//! Run: cargo test -p edge-server --test surreal_limit_bug -- --nocapture

use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;

#[tokio::test]
async fn repro_limit_drops_first_record() {
    let tmp = tempfile::tempdir().unwrap();
    let db: Surreal<Db> = Surreal::new::<RocksDb>(tmp.path()).await.unwrap();
    db.use_ns("test").use_db("test").await.unwrap();

    // Setup schema
    db.query("DEFINE TABLE test SCHEMAFULL").await.unwrap();
    db.query("DEFINE FIELD end_time ON test TYPE int").await.unwrap();
    db.query("DEFINE FIELD status ON test TYPE string").await.unwrap();
    db.query("DEFINE FIELD name ON test TYPE string").await.unwrap();

    // Insert 7 records
    for i in 1..=7 {
        db.query("CREATE test SET end_time = $t, status = 'completed', name = $n")
            .bind(("t", i * 1000))
            .bind(("n", format!("{:03}", i)))
            .await
            .unwrap();
    }

    // Query WITHOUT LIMIT
    let mut res = db
        .query("SELECT <string>id AS test_id, name, string::uppercase(status) AS status, end_time FROM test ORDER BY end_time DESC")
        .await
        .unwrap();
    let all: Vec<serde_json::Value> = res.take(0).unwrap();
    println!("Without LIMIT: {} records", all.len());
    for r in &all {
        println!("  name={} end_time={}", r["name"], r["end_time"]);
    }
    assert_eq!(all.len(), 7, "Should return all 7 records");
    assert_eq!(all[0]["name"], "007", "First should be 007 (highest end_time)");

    // Query WITH LIMIT 6 (no WHERE)
    let mut res = db
        .query("SELECT <string>id AS test_id, name, string::uppercase(status) AS status, end_time FROM test ORDER BY end_time DESC LIMIT 6")
        .await
        .unwrap();
    let limited: Vec<serde_json::Value> = res.take(0).unwrap();
    println!("\nWith LIMIT 6 (no WHERE): {} records, first={}", limited.len(), limited[0]["name"]);
    assert_eq!(limited[0]["name"], "007", "no-WHERE: first should be 007");

    // Query WITH WHERE + LIMIT 6
    let mut res = db
        .query("SELECT <string>id AS test_id, name, string::uppercase(status) AS status, end_time FROM test WHERE end_time >= $start AND end_time <= $end ORDER BY end_time DESC LIMIT 6")
        .bind(("start", 0))
        .bind(("end", 99999))
        .await
        .unwrap();
    let limited: Vec<serde_json::Value> = res.take(0).unwrap();
    println!("With WHERE + LIMIT 6: {} records, first={}", limited.len(), limited[0]["name"]);
    assert_eq!(
        limited[0]["name"], "007",
        "BUG: WHERE+LIMIT first should be 007 but got {}",
        limited[0]["name"]
    );

    // Query WITH WHERE (i64 params) + LIMIT 6
    let mut res = db
        .query("SELECT <string>id AS test_id, name, string::uppercase(status) AS status, end_time FROM test WHERE end_time >= $start AND end_time <= $end ORDER BY end_time DESC LIMIT 6")
        .bind(("start", 0i64))
        .bind(("end", 99999i64))
        .await
        .unwrap();
    let limited: Vec<serde_json::Value> = res.take(0).unwrap();
    println!("With WHERE (i64) + LIMIT 6: {} records, first={}", limited.len(), limited[0]["name"]);
    assert_eq!(
        limited[0]["name"], "007",
        "BUG: WHERE(i64)+LIMIT first should be 007 but got {}",
        limited[0]["name"]
    );

    // Query WITH INDEX on end_time + WHERE + LIMIT 6
    // This matches production: DEFINE INDEX order_end_time ON order FIELDS end_time
    db.query("DELETE test").await.unwrap();
    db.query("DEFINE INDEX test_end_time ON test FIELDS end_time").await.unwrap();

    let base: i64 = 1769879000000;
    for i in 1..=7i64 {
        db.query("CREATE test SET end_time = $t, status = 'completed', name = $n")
            .bind(("t", base + i * 10000))
            .bind(("n", format!("{:03}", i)))
            .await
            .unwrap();
    }

    let mut res = db
        .query("SELECT <string>id AS test_id, name, string::uppercase(status) AS status, end_time FROM test WHERE end_time >= $start AND end_time <= $end ORDER BY end_time DESC LIMIT 6")
        .bind(("start", base))
        .bind(("end", base + 999999i64))
        .await
        .unwrap();
    let limited: Vec<serde_json::Value> = res.take(0).unwrap();
    println!("With INDEX + WHERE + LIMIT 6: {} records, first={}", limited.len(), limited[0]["name"]);
    assert_eq!(
        limited[0]["name"], "007",
        "BUG: INDEX+WHERE+LIMIT first should be 007 but got {}",
        limited[0]["name"]
    );
}
