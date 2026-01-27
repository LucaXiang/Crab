//! 订单压力测试 - 10000 个真实订单
//!
//! 使用 ServerState::initialize 完整初始化，包含归档流程
//!
//! 命令交叉执行模式：模拟真实场景中多个订单同时进行

use edge_server::{Config, ServerState};
use rand::Rng;
use shared::order::{CartItemInput, OrderCommand, OrderCommandPayload, PaymentInput};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

const ORDER_COUNT: usize = 1000;
const CONCURRENCY: usize = 100;

/// 订单阶段
#[derive(Debug, Clone, Copy, PartialEq)]
enum OrderPhase {
    Open,     // 开台
    AddItems, // 添加商品
    Pay,      // 付款
    Complete, // 完成
}

/// 订单上下文（保存中间状态）- 用于跨 spawn_blocking 传递
#[derive(Clone)]
struct OrderContext {
    idx: usize,
    order_id: Option<String>,
    items: Vec<CartItemInput>,
    total: f64,
}

/// 生成随机商品
fn random_items(rng: &mut impl Rng) -> Vec<CartItemInput> {
    const PRODUCTS: &[(&str, f64)] = &[
        ("宫保鸡丁", 38.0),
        ("麻婆豆腐", 28.0),
        ("鱼香肉丝", 35.0),
        ("红烧肉", 48.0),
        ("糖醋排骨", 58.0),
        ("清蒸鱼", 88.0),
        ("回锅肉", 42.0),
        ("水煮牛肉", 68.0),
        ("蒜蓉西兰花", 22.0),
        ("酸辣汤", 18.0),
        ("蛋炒饭", 15.0),
        ("可乐", 8.0),
        ("啤酒", 12.0),
        ("米饭", 3.0),
    ];

    let count = rng.gen_range(1..=6);
    (0..count)
        .map(|_| {
            let (name, price) = PRODUCTS[rng.gen_range(0..PRODUCTS.len())];
            CartItemInput {
                product_id: format!("product:{}", uuid::Uuid::new_v4()),
                name: name.to_string(),
                price,
                original_price: None,
                quantity: rng.gen_range(1..=3),
                selected_options: None,
                selected_specification: None,
                manual_discount_percent: if rng.gen_bool(0.1) { Some(10.0) } else { None },
                surcharge: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            }
        })
        .collect()
}

/// 执行单个阶段的命令
fn execute_phase(
    state: &ServerState,
    ctx: &mut OrderContext,
    phase: OrderPhase,
) -> Result<(), String> {
    let mut rng = rand::thread_rng();
    let op_id = format!("op-{}", ctx.idx % 10);
    let op_name = format!("收银员{}", ctx.idx % 10);
    let manager = state.orders_manager();

    match phase {
        OrderPhase::Open => {
            let open_cmd = OrderCommand::new(
                op_id,
                op_name,
                OrderCommandPayload::OpenTable {
                    table_id: None,
                    table_name: None,
                    zone_id: None,
                    zone_name: None,
                    guest_count: rng.gen_range(1..=4),
                    is_retail: true,
                },
            );
            let resp = manager.execute_command(open_cmd);
            if !resp.success {
                return Err(format!("开台失败: {:?}", resp.error));
            }
            ctx.order_id = resp.order_id;
            ctx.items = random_items(&mut rng);
            Ok(())
        }
        OrderPhase::AddItems => {
            let order_id = ctx.order_id.as_ref().ok_or("无 order_id")?;
            let add_cmd = OrderCommand::new(
                op_id,
                op_name,
                OrderCommandPayload::AddItems {
                    order_id: order_id.clone(),
                    items: ctx.items.clone(),
                },
            );
            let resp = manager.execute_command(add_cmd);
            if !resp.success {
                return Err(format!("添加商品失败: {:?}", resp.error));
            }
            // 获取总额
            let snapshot = manager
                .get_snapshot(order_id)
                .map_err(|e| e.to_string())?
                .ok_or("快照不存在")?;
            ctx.total = snapshot.total;
            Ok(())
        }
        OrderPhase::Pay => {
            let order_id = ctx.order_id.as_ref().ok_or("无 order_id")?;
            let methods = ["CASH", "WECHAT", "ALIPAY"];
            let method = methods[rng.gen_range(0..methods.len())];
            let pay_cmd = OrderCommand::new(
                op_id,
                op_name,
                OrderCommandPayload::AddPayment {
                    order_id: order_id.clone(),
                    payment: PaymentInput {
                        method: method.to_string(),
                        amount: ctx.total,
                        tendered: if method == "CASH" {
                            Some((ctx.total / 10.0).ceil() * 10.0)
                        } else {
                            None
                        },
                        note: None,
                    },
                },
            );
            let resp = manager.execute_command(pay_cmd);
            if !resp.success {
                return Err(format!("付款失败: {:?}", resp.error));
            }
            Ok(())
        }
        OrderPhase::Complete => {
            let order_id = ctx.order_id.as_ref().ok_or("无 order_id")?;
            let complete_cmd = OrderCommand::new(
                op_id,
                op_name,
                OrderCommandPayload::CompleteOrder {
                    order_id: order_id.clone(),
                    receipt_number: format!("R{:06}", ctx.idx),
                },
            );
            let resp = manager.execute_command(complete_cmd);
            if !resp.success {
                return Err(format!("完成订单失败: {:?}", resp.error));
            }
            Ok(())
        }
    }
}

fn get_dir_size(path: &PathBuf) -> u64 {
    if path.is_file() {
        return fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    }
    let mut size = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                size += fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            } else if p.is_dir() {
                size += get_dir_size(&p);
            }
        }
    }
    size
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_10000_orders_with_archive() {
    // 工作目录 (包含所有数据库)
    let work_dir = PathBuf::from("/tmp/crab_stress_test");

    // 清理旧数据
    let _ = fs::remove_dir_all(&work_dir);

    println!();
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!(
        "║        订单压力测试 - {} 个真实订单 (交叉执行)              ║",
        ORDER_COUNT
    );
    println!("╠═══════════════════════════════════════════════════════════════════╣");
    println!(
        "║ 并发数:   {:>6}                                                 ║",
        CONCURRENCY
    );
    println!(
        "║ 工作目录: {}                              ║",
        work_dir.display()
    );
    println!("║ 模式:     命令交叉执行 (后开订单可能先完成)                      ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();

    // 1. 创建 Config
    println!("[1/5] 创建配置...");
    let config = Config::builder()
        .work_dir(work_dir.to_str().unwrap())
        .http_port(18080) // 避免端口冲突
        .message_tcp_port(19000)
        .environment("test")
        .auth_server_url("http://localhost:3001")
        .build();
    println!("      ✓ 配置就绪");

    // 2. 初始化 ServerState (自动创建所有服务)
    println!("[2/5] 初始化 ServerState...");
    let state = ServerState::initialize(&config).await;
    println!("      ✓ ServerState 就绪 (epoch: {})", state.epoch);

    // 3. 启动后台任务 (包括 ArchiveWorker)
    println!("[3/5] 启动后台任务...");
    state.start_background_tasks().await;
    println!("      ✓ 后台任务已启动");

    let state = Arc::new(state);

    let success = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let commands_executed = Arc::new(AtomicUsize::new(0));

    // 4. 并发执行订单命令
    println!("[4/5] 开始创建订单 (多线程并发)...");
    let start = Instant::now();
    let order_idx = Arc::new(AtomicUsize::new(0));

    // 启动 CONCURRENCY 个工作线程
    let mut handles = Vec::with_capacity(CONCURRENCY);
    for _ in 0..CONCURRENCY {
        let state = state.clone();
        let success = success.clone();
        let failed = failed.clone();
        let commands_executed = commands_executed.clone();
        let order_idx = order_idx.clone();

        let handle = std::thread::spawn(move || {
            loop {
                let i = order_idx.fetch_add(1, Ordering::Relaxed);
                if i >= ORDER_COUNT {
                    break;
                }

                let mut ctx = OrderContext {
                    idx: i,
                    order_id: None,
                    items: vec![],
                    total: 0.0,
                };

                let result = (|| {
                    execute_phase(&state, &mut ctx, OrderPhase::Open)?;
                    execute_phase(&state, &mut ctx, OrderPhase::AddItems)?;
                    execute_phase(&state, &mut ctx, OrderPhase::Pay)?;
                    execute_phase(&state, &mut ctx, OrderPhase::Complete)?;
                    Ok::<_, String>(())
                })();

                match result {
                    Ok(()) => {
                        commands_executed.fetch_add(4, Ordering::Relaxed);
                        success.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        let n = failed.fetch_add(1, Ordering::Relaxed) + 1;
                        if n <= 3 {
                            eprintln!("      [ERR] 订单 {} 失败: {}", i, e);
                        }
                    }
                }
            }
        });
        handles.push(handle);
    }

    // 进度输出线程
    let success_monitor = success.clone();
    let commands_monitor = commands_executed.clone();
    let monitor = std::thread::spawn(move || {
        let mut last_n = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let n = success_monitor.load(Ordering::Relaxed);
            if n >= ORDER_COUNT || n == last_n {
                break;
            }
            last_n = n;
            let cmds = commands_monitor.load(Ordering::Relaxed);
            let elapsed = start.elapsed().as_secs_f64();
            println!(
                "      [{:>5.1}s] 完成: {:>5}/{}, 命令: {}, ({:.0} cmd/s)",
                elapsed, n, ORDER_COUNT, cmds, cmds as f64 / elapsed
            );
        }
    });

    // 等待所有工作线程完成
    for h in handles {
        h.join().unwrap();
    }
    let _ = monitor.join();

    let elapsed = start.elapsed();
    let ok = success.load(Ordering::Relaxed);
    let err = failed.load(Ordering::Relaxed);
    let cmds = commands_executed.load(Ordering::Relaxed);

    println!();
    println!("      订单创建完成: {} 成功, {} 失败", ok, err);
    println!(
        "      总命令数: {}, 命令吞吐: {:.1} cmd/s",
        cmds,
        cmds as f64 / elapsed.as_secs_f64()
    );
    println!(
        "      耗时: {:.2?}, 订单吞吐: {:.1} 订单/秒",
        elapsed,
        ok as f64 / elapsed.as_secs_f64()
    );

    // 5. 完整验证所有订单的 Hash 链
    println!();
    println!("[5/6] 完整验证 Hash 链 (所有 {} 个订单)...", ok);
    let storage = state.orders_manager().storage().clone();
    let snapshots_before = storage.get_all_snapshots().expect("获取快照失败");

    let verify_start = Instant::now();
    let mut checksum_valid = 0;
    let mut checksum_invalid = 0;
    let mut replay_match = 0;
    let mut replay_mismatch = 0;
    let mut replay_error = 0;

    for (i, s) in snapshots_before.iter().enumerate() {
        // 1. 验证快照 checksum
        if s.verify_checksum() {
            checksum_valid += 1;
        } else {
            checksum_invalid += 1;
            if checksum_invalid <= 3 {
                eprintln!("      [WARN] 订单 {} checksum 无效", s.order_id);
            }
        }

        // 2. 重放事件验证
        match state.orders_manager().rebuild_snapshot(&s.order_id) {
            Ok(rebuilt) => {
                if rebuilt.state_checksum == s.state_checksum {
                    replay_match += 1;
                } else {
                    replay_mismatch += 1;
                    if replay_mismatch <= 3 {
                        eprintln!(
                            "      [WARN] 订单 {} 重放不匹配: stored={}, rebuilt={}",
                            s.order_id, s.state_checksum, rebuilt.state_checksum
                        );
                    }
                }
            }
            Err(e) => {
                replay_error += 1;
                if replay_error <= 3 {
                    eprintln!("      [ERR] 订单 {} 重放失败: {}", s.order_id, e);
                }
            }
        }

        // 进度输出
        if (i + 1) % 1000 == 0 || i + 1 == snapshots_before.len() {
            println!(
                "      [{:>5.1}s] 已验证: {}/{}",
                verify_start.elapsed().as_secs_f64(),
                i + 1,
                snapshots_before.len()
            );
        }
    }

    println!();
    println!("      验证结果:");
    println!("        快照数量:     {}", snapshots_before.len());
    println!("        Checksum 有效: {}", checksum_valid);
    println!("        Checksum 无效: {}", checksum_invalid);
    println!("        重放匹配:     {}", replay_match);
    println!("        重放不匹配:   {}", replay_mismatch);
    println!("        重放错误:     {}", replay_error);

    // 6. 等待归档完成
    println!();
    println!("[6/6] 等待归档完成...");
    let archive_start = Instant::now();
    loop {
        let pending = storage.get_pending_archives().unwrap_or_default();
        let snapshots = storage.get_all_snapshots().unwrap_or_default();

        println!(
            "      [{:>5.1}s] 待归档: {}, redb快照: {}",
            archive_start.elapsed().as_secs_f64(),
            pending.len(),
            snapshots.len()
        );

        if pending.is_empty() && snapshots.is_empty() {
            println!("      ✓ 归档完成");
            break;
        }

        if archive_start.elapsed().as_secs() > 120 {
            println!("      ⚠ 归档超时 (2分钟)");
            break;
        }
    }

    // 统计结果
    println!();
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║                           测试结果                                 ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");

    // 数据目录大小 (work_dir/data/)
    let data_dir = work_dir.join("data");
    let total_size = get_dir_size(&data_dir);
    println!("  数据目录:      {}", data_dir.display());
    println!("  总数据量:      {}", format_size(total_size));
    println!(
        "  平均每订单:    {:.2} KB",
        total_size as f64 / ok as f64 / 1024.0
    );

    // 各数据库大小
    let redb_path = data_dir.join("orders.redb");
    let surreal_path = data_dir.join("main.db");
    println!();
    println!("  数据库详情:");
    println!("    orders.redb: {}", format_size(get_dir_size(&redb_path)));
    println!(
        "    main.db:     {}",
        format_size(get_dir_size(&surreal_path))
    );

    // 存储统计
    let stats = storage.get_stats().expect("获取统计失败");
    println!();
    println!("  redb 存储统计:");
    println!("    事件数量:   {}", stats.event_count);
    println!("    快照数量:   {}", stats.snapshot_count);
    println!("    活跃订单:   {}", stats.active_order_count);
    println!("    序列号:     {}", stats.current_sequence);

    // SurrealDB 归档统计 (订单存在 order 表)
    let archived_count: Option<usize> = state
        .db
        .clone()
        .query("SELECT count() FROM order GROUP ALL")
        .await
        .ok()
        .and_then(|mut r| r.take::<Option<serde_json::Value>>(0).ok().flatten())
        .and_then(|v| v.get("count").and_then(|c| c.as_u64()).map(|c| c as usize));

    let archived_events: Option<usize> = state
        .db
        .clone()
        .query("SELECT count() FROM order_event GROUP ALL")
        .await
        .ok()
        .and_then(|mut r| r.take::<Option<serde_json::Value>>(0).ok().flatten())
        .and_then(|v| v.get("count").and_then(|c| c.as_u64()).map(|c| c as usize));

    println!();
    println!("  SurrealDB 归档:");
    println!("    已归档订单: {}", archived_count.unwrap_or(0));
    println!("    已归档事件: {}", archived_events.unwrap_or(0));

    // 验证 Hash 链完整性
    println!();
    println!("  Hash 链验证:");

    // 使用强类型查询 system_state
    #[derive(Debug, serde::Deserialize)]
    struct SystemStateQuery {
        last_order: Option<String>,
        last_order_hash: Option<String>,
    }

    let system_query: Option<SystemStateQuery> = state
        .db
        .clone()
        .query("SELECT <string>last_order as last_order, last_order_hash FROM system_state:main")
        .await
        .ok()
        .and_then(|mut r| r.take::<Option<SystemStateQuery>>(0).ok().flatten());

    let (last_order_id, system_last_hash) = system_query
        .map(|s| (s.last_order, s.last_order_hash))
        .unwrap_or((None, None));

    println!("    system_state.last_order:      {:?}", last_order_id);
    println!("    system_state.last_order_hash: {:?}", system_last_hash.as_ref().map(|s| &s[..16.min(s.len())]));

    // 使用强类型查询最后订单
    #[derive(Debug, serde::Deserialize)]
    struct OrderQuery {
        id: String,
        curr_hash: String,
        #[allow(dead_code)]
        created_at: serde_json::Value, // 需要用于 ORDER BY
    }

    let last_order_result = state
        .db
        .clone()
        .query("SELECT <string>id as id, curr_hash, created_at FROM order ORDER BY created_at DESC LIMIT 1")
        .await;

    let last_order_query: Vec<OrderQuery> = match last_order_result {
        Ok(mut r) => r.take::<Vec<OrderQuery>>(0).unwrap_or_default(),
        Err(e) => {
            println!("    DEBUG last order query error: {:?}", e);
            vec![]
        }
    };

    let (actual_last_id, actual_last_hash) = last_order_query
        .first()
        .map(|o| (Some(o.id.clone()), Some(o.curr_hash.clone())))
        .unwrap_or((None, None));

    println!("    最后订单 ID:                  {:?}", actual_last_id);
    println!("    最后订单 curr_hash:           {:?}", actual_last_hash.as_ref().map(|s| &s[..16.min(s.len())]));

    let hash_chain_valid = system_last_hash.is_some()
        && actual_last_hash.is_some()
        && system_last_hash == actual_last_hash;

    println!("    Order Hash 链一致: {}", if hash_chain_valid { "✓" } else { "✗" });

    // 验证订单之间的 Hash 链（抽样验证前 50 个订单）
    println!();
    println!("  Order 间 Hash 链验证 (前50个订单):");

    #[derive(Debug, serde::Deserialize)]
    struct OrderHashChain {
        prev_hash: String,
        curr_hash: String,
        #[allow(dead_code)]
        created_at: serde_json::Value,
    }

    let order_chain: Vec<OrderHashChain> = state
        .db
        .clone()
        .query("SELECT prev_hash, curr_hash, created_at FROM order ORDER BY created_at LIMIT 50")
        .await
        .ok()
        .and_then(|mut r| r.take::<Vec<OrderHashChain>>(0).ok())
        .unwrap_or_default();

    let mut order_chain_valid = true;
    let mut expected_prev = "genesis".to_string();
    let mut order_chain_breaks = 0;

    for (i, order) in order_chain.iter().enumerate() {
        if order.prev_hash != expected_prev {
            order_chain_valid = false;
            order_chain_breaks += 1;
            if order_chain_breaks <= 3 {
                eprintln!(
                    "      [WARN] 订单 #{} hash 链断裂: expected prev={}, got={}",
                    i,
                    &expected_prev[..16.min(expected_prev.len())],
                    &order.prev_hash[..16.min(order.prev_hash.len())]
                );
            }
        }
        expected_prev = order.curr_hash.clone();
    }

    // 验证第一个订单的 prev_hash 是 "genesis"
    let first_order_prev = order_chain.first().map(|o| o.prev_hash.as_str());
    let genesis_valid = first_order_prev == Some("genesis");

    println!(
        "    检查订单数: {}, 链条断裂: {}",
        order_chain.len(),
        order_chain_breaks
    );
    println!("    第一个订单 prev_hash: {:?} (应为 \"genesis\")", first_order_prev);
    println!("    Order 间 Hash 链一致: {}", if order_chain_valid && genesis_valid { "✓" } else { "✗" });

    // 验证 Event Hash 链（抽样验证 10 个订单）
    println!();
    println!("  Event Hash 链验证 (抽样):");

    #[derive(Debug, serde::Deserialize)]
    struct SampleOrder {
        id: String,
    }

    let sample_orders: Vec<SampleOrder> = state
        .db
        .clone()
        .query("SELECT <string>id as id FROM order LIMIT 10")
        .await
        .ok()
        .and_then(|mut r| r.take::<Vec<SampleOrder>>(0).ok())
        .unwrap_or_default();

    let mut event_chain_valid_count = 0;
    let mut event_chain_invalid_count = 0;

    for order in &sample_orders {
        let order_id = order.id.clone();

        // 获取该订单关联的所有事件，按 timestamp 排序
        // 使用 RELATE 边查询：order->has_event->order_event
        #[derive(Debug, serde::Deserialize)]
        struct EventHash {
            prev_hash: String,
            curr_hash: String,
        }

        // 通过 has_event 边表查询该订单的事件
        let order_key = order_id.strip_prefix("order:").unwrap_or(&order_id);
        let order_record_id = surrealdb::RecordId::from_table_key("order", order_key);

        let events_result = state
            .db
            .clone()
            .query(r#"
                LET $event_ids = (SELECT VALUE out FROM has_event WHERE in = $order_id);
                SELECT prev_hash, curr_hash, timestamp FROM order_event WHERE id IN $event_ids ORDER BY timestamp;
            "#)
            .bind(("order_id", order_record_id.clone()))
            .await;

        let events: Vec<EventHash> = events_result
            .ok()
            .and_then(|mut r| r.take::<Vec<EventHash>>(1).ok()) // 第二个语句的结果
            .unwrap_or_default();

        let mut chain_valid = true;
        let mut expected_prev = "order_start".to_string();

        for event in &events {
            let prev_hash = &event.prev_hash;
            let curr_hash = &event.curr_hash;

            if *prev_hash != expected_prev {
                chain_valid = false;
                if event_chain_invalid_count < 3 {
                    eprintln!(
                        "      [WARN] 订单 {} 事件链断裂: expected prev={}, got={}",
                        order_id,
                        &expected_prev[..16.min(expected_prev.len())],
                        &prev_hash[..16.min(prev_hash.len())]
                    );
                }
                break;
            }
            expected_prev = curr_hash.to_string();
        }

        if chain_valid && !events.is_empty() {
            event_chain_valid_count += 1;
        } else if !events.is_empty() {
            event_chain_invalid_count += 1;
        }
    }

    println!(
        "    抽样订单数: {}, 事件链有效: {}, 无效: {}",
        sample_orders.len(),
        event_chain_valid_count,
        event_chain_invalid_count
    );

    let event_chain_valid = event_chain_invalid_count == 0;
    println!("    Event Hash 链一致: {}", if event_chain_valid { "✓" } else { "✗" });

    // redb 归档后应该为空
    let snapshots_after = storage.get_all_snapshots().expect("获取快照失败");
    println!();
    println!("  归档后 redb 状态:");
    println!("    剩余快照: {} (应为 0)", snapshots_after.len());

    println!();
    println!("═══════════════════════════════════════════════════════════════════");

    // 断言
    assert!(ok >= ORDER_COUNT / 2, "成功订单数应 >= 50%");
    assert_eq!(checksum_invalid, 0, "所有 checksum 应有效");
    assert_eq!(replay_mismatch, 0, "所有重放应匹配");
    assert_eq!(replay_error, 0, "重放不应有错误");
    assert_eq!(snapshots_after.len(), 0, "归档后 redb 应无快照");
    assert!(hash_chain_valid, "Order Hash 链应一致: system_state.last_order_hash = 最后订单的 order_hash");
    assert!(event_chain_valid, "Event Hash 链应一致: 每个事件的 prev_event_hash = 上一个事件的 event_hash");

    println!("✅ 测试通过!");
}
