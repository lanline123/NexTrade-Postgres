/**
 * 顶栏授权徽标（我方新增，对接授权中心 auth.recy.cc 的判定）。
 *
 * 数据来自后端 `GET /adminapi/license/status`（经 `src/license.ts` 归一化）：
 * - 已授权：绿色「已授权 · 商业版 / 企业版」；
 * - 未授权（含已过期）：红色「未授权」，弹窗提示「本系统尚未获得授权，请联系授权方」；
 * - 待确认（从未成功上报 / 状态口不可达）：灰色「授权状态待确认」。
 *
 * 状态由 AdminLayout 统一拉取后经 props 传入（与菜单版本控制共用同一份
 * 状态）；无 props 时组件自行拉取兜底。fail-soft：拉取失败只显示灰色
 * 待确认，绝不报错弹窗、绝不影响顶栏其余功能。
 *
 * 注：与原版授权码桩（旧 `/setting/licensed/index`）无关——旧徽标逻辑
 * （有授权码才显示图标）已被本组件取代，旧接口保留给「授权信息」页。
 */

import { useEffect, useState } from 'react';
import { Modal, Tag, Typography } from '@douyinfe/semi-ui';
import {
  fetchLicenseStatus,
  licenseRank,
  type LicenseStatus,
} from '../license';
import { useI18n } from '../i18n';

const { Text } = Typography;

const TIER_NAME: Record<string, string> = {
  commercial: '商业版',
  enterprise: '企业版',
};

export default function LicensedBadge({ status: statusProp }: { status?: LicenseStatus }) {
  const { t } = useI18n();
  const [selfStatus, setSelfStatus] = useState<LicenseStatus | null>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (statusProp) return;
    let alive = true;
    void fetchLicenseStatus().then((status) => {
      if (alive) setSelfStatus(status);
    });
    return () => {
      alive = false;
    };
  }, [statusProp]);

  const status = statusProp ?? selfStatus;
  if (!status) return null;

  const rank = licenseRank(status);
  const isAuthorized = rank > 0;
  const isUnknown = status.status === 'unknown';
  const color = isAuthorized ? 'green' : isUnknown ? 'grey' : 'red';
  const label = isAuthorized
    ? `${t('已授权')} · ${t(TIER_NAME[status.tier ?? ''] ?? '')}`
    : isUnknown
      ? t('授权状态待确认')
      : t('未授权');

  return (
    <>
      <span title={t('授权信息')}>
        <Tag
          color={color}
          size="large"
          onClick={() => setVisible(true)}
          style={{ cursor: 'pointer' }}
        >
          {label}
        </Tag>
      </span>
      <Modal title={t('授权信息')} visible={visible} onCancel={() => setVisible(false)} footer={null} width={480}>
        <div className="licensed-info">
          {!isAuthorized && !isUnknown && (
            <div>
              <Text type="danger">{t('本系统尚未获得授权，请联系授权方')}</Text>
            </div>
          )}
          {isUnknown && (
            <div>
              <Text type="tertiary">{t('尚未完成首次授权校验，状态待确认')}</Text>
            </div>
          )}
          <div><Text type="tertiary">{t('授权状态')}</Text><span>{label}</span></div>
          {status.expired && (
            <div><Text type="tertiary">{t('到期状态')}</Text><span>{t('已过期')}</span></div>
          )}
          <div><Text type="tertiary">{t('到期时间')}</Text><span>{status.expiresAt || '-'}</span></div>
          {status.tier === 'enterprise' && (
            <div>
              <Text type="tertiary">{t('子域名额度')}</Text>
              <span>
                {status.subdomainsUsed ?? '-'} / {status.subdomainLimit ?? '-'}
              </span>
            </div>
          )}
          <div><Text type="tertiary">{t('校验时间')}</Text><span>{status.checkedAt || '-'}</span></div>
          <div><Text type="tertiary">{t('实例标识')}</Text><span className="licensed-code">{status.instanceKey || '-'}</span></div>
        </div>
      </Modal>
    </>
  );
}
