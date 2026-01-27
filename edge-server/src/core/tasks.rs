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

use std::fmt;
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
    /// # 参数
    ///
    /// - `name`: 任务名称（用于日志和调试）
    /// - `kind`: 任务类型
    /// - `future`: 要执行的异步任务
    pub fn spawn<F>(&mut self, name: &'static str, kind: TaskKind, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let handle = tokio::spawn(future);
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
            "📋 Background tasks registered: {} total (Worker: {}, Listener: {}, Periodic: {}, Warmup: {})",
            self.tasks.len(),
            worker,
            listener,
            periodic,
            warmup
        );
    }

    /// Graceful shutdown - 取消所有任务并等待完成
    ///
    /// 发送取消信号后，等待所有任务完成或超时。
    pub async fn shutdown(self) {
        tracing::info!("🛑 Shutting down {} background tasks...", self.tasks.len());

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

        tracing::info!("✅ All background tasks stopped");
    }
}

impl Default for BackgroundTasks {
    fn default() -> Self {
        Self::new()
    }
}
