import { useEffect, useState } from 'react';
import { Banner, Button, Space } from '@douyinfe/semi-ui';
import { apiGet } from '../api';
import { useI18n } from '../i18n';
import type { PageKey } from '../layout/AdminLayout';

/**
 * 客户端更新持续提醒（热更新 P4，我方新增）。
 *
 * 小程序 / App 不能由服务器热更：当最新产品 Release 的清单标志
 * （requiresMiniProgramUpdate / requiresAppUpdate，P1 系统更新状态口径）
 * 为真、且该渠道尚无覆盖该产品版本的「已发布登记」时，在此持续提醒，
 * 登记后自动消警（派生态，不存已读/提醒行）。数据不可达（非平台账号、
 * 系统更新未上线）时静默不渲染。
 */

interface SystemUpdateStatus {
  latestProductVersion?: string | null;
  requiresMiniProgramUpdate?: boolean;
  requiresAppUpdate?: boolean;
}

interface ChannelRelease {
  channel: string;
  covered: boolean;
}

interface ClientReleaseOverview {
  channels: ChannelRelease[];
}

interface ClientAlert {
  channel: 'mini_program' | 'app';
  version: string;
}

export default function ClientUpdateAlert({
  onNavigate,
}: {
  onNavigate: (page: PageKey) => void;
}) {
  const { t } = useI18n();
  const [alerts, setAlerts] = useState<ClientAlert[]>([]);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const status = await apiGet<SystemUpdateStatus>('/setting/systemUpdate/status');
        const version = status?.latestProductVersion || '';
        if (!version || (!status?.requiresMiniProgramUpdate && !status?.requiresAppUpdate)) {
          return;
        }
        const overview = await apiGet<ClientReleaseOverview>(
          '/setting/clientRelease/overview',
          { targetVersion: version },
        );
        if (!alive) return;
        const coveredOf = (channel: string) =>
          overview?.channels?.find((c) => c.channel === channel)?.covered ?? false;
        const next: ClientAlert[] = [];
        if (status.requiresMiniProgramUpdate && !coveredOf('mini_program')) {
          next.push({ channel: 'mini_program', version });
        }
        if (status.requiresAppUpdate && !coveredOf('app')) {
          next.push({ channel: 'app', version });
        }
        setAlerts(next);
      } catch {
        // 非平台账号或相关端点未上线：静默降级，不渲染提醒
      }
    })();
    return () => {
      alive = false;
    };
  }, []);

  if (alerts.length === 0) return null;

  return (
    <div style={{ width: '100%' }}>
      {alerts.map((alert) =>
        alert.channel === 'mini_program' ? (
          <Banner
            key={alert.channel}
            type="warning"
            closeIcon={null}
            style={{ marginBottom: 12 }}
            description={
              <Space vertical align="start" spacing={6}>
                <span>
                  {t('新版本 {version} 需要同步发布小程序', { version: alert.version })}
                  {' — '}
                  {t(
                    'Web 端更新不会自动更新小程序：请到微信公众平台提交本版小程序代码、审核通过后发布，否则小程序用户仍停留在旧版。',
                  )}
                </span>
                <Button
                  size="small"
                  theme="solid"
                  type="warning"
                  onClick={() => onNavigate('clientReleaseManage')}
                >
                  {t('发布后登记已发布')}
                </Button>
              </Space>
            }
          />
        ) : (
          <Banner
            key={alert.channel}
            type="warning"
            closeIcon={null}
            style={{ marginBottom: 12 }}
            description={
              <Space vertical align="start" spacing={6}>
                <span>
                  {t('新版本 {version} 需要同步发布 App', { version: alert.version })}
                  {' — '}
                  {t(
                    'Web 端更新不会自动更新 App：请先在「APP版本」页更新 Android / iOS 版本号与下载链接，再通过应用市场或下载渠道发布新版安装包。',
                  )}
                </span>
                <Space spacing={8}>
                  <Button size="small" onClick={() => onNavigate('appVersionManage')}>
                    {t('去更新 APP 版本配置')}
                  </Button>
                  <Button
                    size="small"
                    theme="solid"
                    type="warning"
                    onClick={() => onNavigate('clientReleaseManage')}
                  >
                    {t('发布后登记已发布')}
                  </Button>
                </Space>
              </Space>
            }
          />
        ),
      )}
    </div>
  );
}
