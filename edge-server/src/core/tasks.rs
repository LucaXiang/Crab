//! 后台任务管理
//!
//! 统一管理所有后台任务的注册、启动和关闭。
//!
//! # 任务类型
//!
//! - [`TaskKind::Warmup`] - 启动预热任务（同步执行，运行一次）
//! - [`TaskKind::Worker`] - 长期后台工作者
//! - [`TaskKind::Listener`] - 事件监听器
//! - [`TaskKind::Periodic`] - 定时任务

use futures::FutureExt;
use std::fmt;
use std::panic::AssertUnwindSafe;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// 任务类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    /// 启动预热任务（同步执行，运行一次）
    Warmup,
    /// 长期后台工作者
    Worker,
    /// 事件监听器
    Listener,
    /// 定时任务
    Periodic,
}

impl fmt::Display for TaskKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskKind::Warmup => write!(f, "Warmup"),
            TaskKind::Worker => write!(f, "Worker"),
            TaskKind::Listener => write!(f, "Listener"),
            TaskKind::Periodic => write!(f, "Periodic"),
        }
    }
}

/// 已注册的后台任务
struct RegisteredTask {
    /// 任务名称
    name: &'static str,
    /// 任务类型
    kind: TaskKind,
    /// 任务句柄
    handle: JoinHandle<()>,
}

/// 后台任务管理器
///
/// 统一管理所有后台任务的注册和生命周期。
///
/// # 使用示例
///
/// ```ignore
/// let mut tasks = BackgroundTasks::new();
///
/// // 注册 Worker 任务
/// tasks.spawn("archive_worker", TaskKind::Worker, async move {
///     // 任务逻辑
/// });
///
/// // 注册 Listener 任务
/// tasks.spawn("event_listener", TaskKind::Listener, async move {
///     // 监听逻辑
/// });
///
/// // Graceful shutdown
/// tasks.shutdown().await;
/// ```
pub struct BackgroundTasks {
    /// 已注册的任务列表
    tasks: Vec<RegisteredTask>,
    /// 全局取消令牌
    shutdown: CancellationToken,
}

impl BackgroundTasks {
    /// 创建新的任务管理器
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            shutdown: CancellationToken::new(),
        }
    }

    /// 获取取消令牌（用于任务内部监听 shutdown 信号）
    pub fn shutdown_token(&self) -> CancellationToken {
        self.shutdown.clone()
    }

    /// 注册并启动一个后台任务
    ///
    /// 任务会被包装以捕获 panic，如果任务异常退出会记录错误日志。
    ///
    /// # 参数
    ///
    /// - `name`: 任务名称（用于日志和调试）
    /// - `kind`: 任务类型
    /// - `future`: 要执行的异步任务
    pub fn spawn<F>(&mut self, name: &'static str, kind: TaskKind, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        // Wrap the future to catch panics and log errors
        let wrapped_future = async move {
            let result: Result<(), Box<dyn std::any::Any + Send>> =
                AssertUnwindSafe(future).catch_unwind().await;
            match result {
                Ok(()) => {
                    // Normal completion - only log for non-Warmup tasks
                    if kind != TaskKind::Warmup {
                        tracing::warn!(task = %name, kind = %kind, "Background task completed unexpectedly");
                    }
                }
                Err(panic_info) => {
                    // Task panicked - log error with panic info
                    let panic_msg: String = if let Some(s) = panic_info.downcast_ref::<&str>() {
                        (*s).to_string()
                    } else if let Some(s) = panic_info.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "Unknown panic".to_string()
                    };
                    tracing::error!(
                        task = %name,
                        kind = %kind,
                        panic = %panic_msg,
                        "Background task panicked! This is a bug that should be reported."
                    );
                }
            }
        };

        let handle = tokio::spawn(wrapped_future);
        tracing::debug!(task = %name, kind = %kind, "Registered background task");
        self.tasks.push(RegisteredTask { name, kind, handle });
    }

    /// 获取已注册任务数量
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// 检查是否没有注册任务
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// 按类型统计任务数量
    pub fn count_by_kind(&self) -> (usize, usize, usize, usize) {
        let mut warmup = 0;
        let mut worker = 0;
        let mut listener = 0;
        let mut periodic = 0;

        for task in &self.tasks {
            match task.kind {
                TaskKind::Warmup => warmup += 1,
                TaskKind::Worker => worker += 1,
                TaskKind::Listener => listener += 1,
                TaskKind::Periodic => periodic += 1,
            }
        }

        (warmup, worker, listener, periodic)
    }

    /// 打印任务摘要
    pub fn log_summary(&self) {
        let (warmup, worker, listener, periodic) = self.count_by_kind();
        tracing::info!(
            "Background tasks registered: {} total (Worker: {}, Listener: {}, Periodic: {}, Warmup: {})",
            self.tasks.len(),
            worker,
            listener,
            periodic,
            warmup
        );
    }

    /// 检查所有任务健康状态
    ///
    /// 返回异常终止的任务数量。如果有任务 panic 或意外退出，会记录错误日志。
    pub fn check_health(&self) -> usize {
        let mut failed_count = 0;
        for task in &self.tasks {
            if task.handle.is_finished() {
                tracing::error!(
                    task = %task.name,
                    kind = %task.kind,
                    "Background task unexpectedly finished! This may indicate a panic or error."
                );
                failed_count += 1;
            }
        }
        if failed_count > 0 {
            tracing::error!(
                failed = failed_count,
                total = self.tasks.len(),
                "Background task health check: {} task(s) failed",
                failed_count
            );
        }
        failed_count
    }

    /// Graceful shutdown - 取消所有任务并等待完成
    ///
    /// 发送取消信号后，等待所有任务完成或超时。
    pub async fn shutdown(self) {
        tracing::info!("Shutting down {} background tasks...", self.tasks.len());

        // 发送取消信号
        self.shutdown.cancel();

        // 等待所有任务完成
        for task in self.tasks {
            match task.handle.await {
                Ok(()) => {
                    tracing::debug!(task = %task.name, "Task completed");
                }
                Err(e) if e.is_cancelled() => {
                    tracing::debug!(task = %task.name, "Task cancelled");
                }
                Err(e) => {
                    tracing::error!(task = %task.name, error = ?e, "Task panicked");
                }
            }
        }

        tracing::info!("All background tasks stopped");
    }
}

impl Default for BackgroundTasks {
    fn default() -> Self {
        Self::new()
    }
}
