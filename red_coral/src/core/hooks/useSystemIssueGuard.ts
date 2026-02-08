/**
 * System Issue Guard Hook
 *
 * 仅 Server 模式启用。登录后查询 pending system issues，
 * 监听 server-message 事件以响应远程推送的 SystemIssue。
 * 返回当前需要展示的 issue 及 resolve 方法。
 */

import { useEffect, useState, useCallback } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { createTauriClient } from '@/infrastructure/api/tauri-client';
import { logger } from '@/utils/logger';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';
import { useBridgeStore } from '@/core/stores/bridge';
import { toast } from '@/presentation/components/Toast';
import { t } from '@/infrastructure/i18n';
import type { SystemIssue, ResolveSystemIssueRequest } from '@/core/domain/types/api';

const client = createTauriClient();

interface ServerMessageEvent {
  event_type: string;
  payload: unknown;
  correlation_id: string | null;
}

interface SyncPayload {
  resource: string;
  version: number;
  action: string;
  id: string;
  data?: unknown;
}

export function useSystemIssueGuard() {
  const user = useAuthStore(state => state.user);
  const appState = useBridgeStore(state => state.appState);
  const [issues, setIssues] = useState<SystemIssue[]>([]);

  const isServerAuthenticated = appState?.type === 'ServerAuthenticated';

  const fetchIssues = useCallback(async () => {
    if (!isServerAuthenticated) return;
    try {
      const pending = await client.getSystemIssues();
      setIssues(pending);
    } catch (err) {
      logger.error('Failed to fetch pending issues', err, { component: 'SystemIssueGuard' });
      toast.warning(t('system.issue_check_failed'));
    }
  }, [isServerAuthenticated]);

  const resolveIssue = useCallback(async (data: ResolveSystemIssueRequest) => {
    await client.resolveSystemIssue(data);
    // Refetch to show next issue or clear
    await fetchIssues();
  }, [fetchIssues]);

  // Fetch on mount when authenticated in Server mode
  useEffect(() => {
    if (isServerAuthenticated && user) {
      fetchIssues();
    } else {
      setIssues([]);
    }
  }, [isServerAuthenticated, user, fetchIssues]);

  // Listen for server-message events (remote SystemIssue push)
  useEffect(() => {
    if (!isServerAuthenticated) return;

    let unlisten: UnlistenFn | undefined;
    let isMounted = true;

    listen<ServerMessageEvent>('server-message', (event) => {
      if (!isMounted) return;
      const msg = event.payload;
      // 监听 sync 事件: 当 system_issue 资源变更时 refetch
      if (msg.event_type === 'sync') {
        try {
          const sync = (typeof msg.payload === 'string'
            ? JSON.parse(msg.payload)
            : msg.payload) as SyncPayload;
          if (sync.resource === 'system_issue') {
            logger.debug('system_issue sync received, refetching', { component: 'SystemIssueGuard' });
            fetchIssues();
          }
        } catch {
          // ignore parse errors
        }
      }
    }).then((fn) => {
      if (isMounted) {
        unlisten = fn;
      } else {
        fn();
      }
    });

    return () => {
      isMounted = false;
      unlisten?.();
    };
  }, [isServerAuthenticated, fetchIssues]);

  // Blocking issues take priority
  const currentIssue = issues.find(i => i.blocking) ?? issues[0] ?? null;

  return { currentIssue, issues, resolveIssue };
}
