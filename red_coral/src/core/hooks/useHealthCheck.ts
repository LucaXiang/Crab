/**
 * useHealthCheck - 周期性健康检查
 *
 * 每 5 秒调用后端 get_health_status 命令，返回数据库在线状态。
 * 用于 ActionBar 的连接指示灯。
 */

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

const HEALTH_CHECK_INTERVAL = 5000;

export function useHealthCheck(): boolean | null {
  const [isDbOnline, setIsDbOnline] = useState<boolean | null>(null);

  useEffect(() => {
    let mounted = true;

    const check = async () => {
      try {
        await invoke('get_health_status');
        if (mounted) setIsDbOnline(true);
      } catch {
        if (mounted) setIsDbOnline(false);
      }
    };

    check();
    const id = setInterval(check, HEALTH_CHECK_INTERVAL);

    return () => {
      mounted = false;
      clearInterval(id);
    };
  }, []);

  return isDbOnline;
}
