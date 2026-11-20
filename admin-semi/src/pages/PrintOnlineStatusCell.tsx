import { useCallback, useEffect, useState } from 'react';
import { Button, Tag } from '@douyinfe/semi-ui';
import { apiGet } from '../api';

/**
 * 打印设备在线状态单元格（我方新增，2026-11-21）。
 *
 * 挂载时向后端 `GET /print/print/onlineStatus?id=` 实时查询云打印平台
 * 返回的设备状态（不落库），并提供手动刷新。后端口径：查询失败 /
 * 未配置 / 平台暂不支持一律回「未知」（onlineStatus=3），
 * 这里请求本身失败时同样按「未知」呈现，不误标离线。
 */

interface OnlineStatusData {
  onlineStatus: number;
  onlineStatusText: string;
  rawStatus?: string;
  queryTime?: string;
}

const TAG_COLORS: Record<number, 'green' | 'orange' | 'red' | 'grey'> = {
  0: 'red',
  1: 'green',
  2: 'orange',
  3: 'grey',
};

export default function PrintOnlineStatusCell({ printId }: { printId: number }) {
  const [data, setData] = useState<OnlineStatusData | null>(null);
  const [loading, setLoading] = useState(false);
  const [failed, setFailed] = useState(false);

  const query = useCallback(async () => {
    setLoading(true);
    setFailed(false);
    try {
      const res = await apiGet<OnlineStatusData>('/print/print/onlineStatus', { id: printId });
      setData(res);
    } catch {
      setFailed(true);
    } finally {
      setLoading(false);
    }
  }, [printId]);

  useEffect(() => {
    void query();
  }, [query]);

  const status = failed ? 3 : data?.onlineStatus;
  const text = failed ? '未知' : data?.onlineStatusText;
  const hint = failed
    ? '状态查询请求失败'
    : [data?.rawStatus, data?.queryTime ? `查询于 ${data.queryTime}` : '']
        .filter(Boolean)
        .join(' · ');

  return (
    <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
      {status === undefined ? (
        <Tag color="grey">查询中…</Tag>
      ) : (
        <Tag color={TAG_COLORS[status] ?? 'grey'} title={hint || undefined}>
          {text}
        </Tag>
      )}
      <Button size="small" theme="borderless" loading={loading} onClick={() => void query()}>
        刷新
      </Button>
    </span>
  );
}
