/**
 * 统一的日志工具
 *
 * 特性：
 * - 开发环境：输出所有日志
 * - 生产环境：仅输出warn和error
 * - 自动添加时间戳和上下文
 * - 支持结构化日志
 */

type LogLevel = 'debug' | 'info' | 'warn' | 'error';

interface LogContext {
  component?: string;
  action?: string;
  [key: string]: unknown;
}

class Logger {
  private isDev: boolean;
  private logs: Array<{ level: LogLevel; message: string; context?: LogContext; timestamp: string }> = [];
  private subscribers: Array<(logs: Array<{ level: LogLevel; message: string; context?: LogContext; timestamp: string }>) => void> = [];

  constructor() {
    this.isDev = import.meta.env.DEV;
  }

  /**
   * 调试日志（仅开发环境）
   */
  debug(message: string, context?: LogContext): void {
    if (this.isDev) {
      this.log('debug', message, context);
    }
  }

  /**
   * 信息日志（仅开发环境）
   */
  info(message: string, context?: LogContext): void {
    if (this.isDev) {
      this.log('info', message, context);
    }
  }

  /**
   * 警告日志（所有环境）
   */
  warn(message: string, context?: LogContext): void {
    this.log('warn', message, context);
  }

  /**
   * 错误日志（所有环境）
   */
  error(message: string, error?: Error | unknown, context?: LogContext): void {
    const errorContext = {
      ...context,
      error: error instanceof Error ? {
        message: error.message,
        stack: error.stack,
      } : error,
    };

    this.log('error', message, errorContext);

    // 局域网部署：本地日志足够，无需 Sentry
    // 如需云端监控可在此集成
  }

  /**
   * 性能测量
   */
  measure(label: string): () => void {
    if (!this.isDev) return () => {};

    const start = performance.now();
    console.log(`[PERF] ${label} - Start`);

    return () => {
      const duration = performance.now() - start;
      console.log(`[PERF] ${label} - Duration: ${duration.toFixed(2)}ms`);
    };
  }

  /**
   * 统一日志输出
   */
  private log(level: LogLevel, message: string, context?: LogContext): void {
    const timestamp = new Date().toISOString();
    const prefix = `[${timestamp}] [${level.toUpperCase()}]`;

    const logMessage = context
      ? `${prefix} ${message}`
      : `${prefix} ${message}`;

    if (this.isDev) {
      this.logs.push({ level, message, context, timestamp });
      if (this.logs.length > 500) this.logs.shift();
      this.subscribers.forEach((fn) => fn(this.logs));
    }

    switch (level) {
      case 'debug':
        console.log(logMessage, context || '');
        break;
      case 'info':
        console.info(logMessage, context || '');
        break;
      case 'warn':
        console.warn(logMessage, context || '');
        break;
      case 'error':
        console.error(logMessage, context || '');
        break;
    }
  }

  getLogs() {
    return this.logs;
  }

  subscribe(fn: (logs: Array<{ level: LogLevel; message: string; context?: LogContext; timestamp: string }>) => void) {
    this.subscribers.push(fn);
    fn(this.logs);
    return () => {
      this.subscribers = this.subscribers.filter((f) => f !== fn);
    };
  }
}

// 导出单例实例
export const logger = new Logger();

// 导出类型供外部使用
export type { LogContext };
