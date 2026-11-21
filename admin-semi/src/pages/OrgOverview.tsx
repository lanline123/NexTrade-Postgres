// 我方新增（万象互联 P1）：组织概览页（万象互联 → 组织概览）。
import { useEffect, useState } from 'react';
import { Card, Col, Row, Spin, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { apiGet } from '../api';
import { useI18n } from '../i18n';
import { normalizeFederationSettings } from '../federation';

const { Text, Title } = Typography;

interface HubOverview {
  settings?: Record<string, any>;
  linksPending?: number;
  linksActive?: number;
  linksDisabled?: number;
  credentials?: number;
  aggregationProducts?: number;
}

/** 组织概览：GET /hub/overview 计数卡片 + 当前联邦设置摘要。 */
export default function OrgOverview() {
  const { t } = useI18n();
  const [loading, setLoading] = useState(true);
  const [overview, setOverview] = useState<HubOverview | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const data = await apiGet<HubOverview>('/hub/overview');
        setOverview(data || {});
      } catch (error) {
        Toast.error({ content: error instanceof Error ? error.message : t('加载组织概览失败') });
      } finally {
        setLoading(false);
      }
    })();
  }, [t]);

  if (loading) {
    return (
      <div style={{ padding: 40, textAlign: 'center' }}>
        <Spin size="large" />
      </div>
    );
  }

  const settings = normalizeFederationSettings(overview?.settings);
  const cards: Array<{ label: string; value: number; color: string }> = [
    { label: '待审批连接', value: overview?.linksPending || 0, color: 'orange' },
    { label: '已激活连接', value: overview?.linksActive || 0, color: 'green' },
    { label: '已停用连接', value: overview?.linksDisabled || 0, color: 'grey' },
    { label: 'API 凭证', value: overview?.credentials || 0, color: 'blue' },
    { label: '聚合商品', value: overview?.aggregationProducts || 0, color: 'violet' },
  ];
  const boolTag = (value: boolean) => (
    <Tag color={value ? 'green' : 'grey'}>{value ? t('开') : t('关')}</Tag>
  );

  return (
    <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 16 }}>
      <Title heading={4} style={{ margin: 0 }}>
        {t('组织概览')}
      </Title>
      <Row gutter={16}>
        {cards.map((card) => (
          <Col span={4} key={card.label} style={{ marginBottom: 16 }}>
            <Card shadows="hover" style={{ textAlign: 'center' }}>
              <div style={{ fontSize: 28, fontWeight: 600 }}>{card.value}</div>
              <Tag color={card.color as any} style={{ marginTop: 8 }}>
                {t(card.label)}
              </Tag>
            </Card>
          </Col>
        ))}
      </Row>

      <Card title={t('当前联邦设置')}>
        <div style={{ display: 'grid', gap: 10, maxWidth: 720 }}>
          <div>
            <Text strong>{t('总开关')}：</Text>
            {boolTag(settings.enabled)}
          </div>
          <div>
            <Text strong>{t('实例名称')}：</Text>
            <Text>{settings.instanceName || '-'}</Text>
          </div>
          <div>
            <Text strong>{t('实例 UUID')}：</Text>
            <Text>{settings.instanceUuid || '-'}</Text>
          </div>
          <div>
            <Text strong>{t('对外地址')}：</Text>
            <Text>{settings.instanceBaseUrl || '-'}</Text>
          </div>
          <div style={{ display: 'flex', gap: 16, flexWrap: 'wrap' }}>
            <span>
              <Text strong>{t('对外供货')}：</Text>
              {boolTag(settings.capabilities.supplyOut)}
            </span>
            <span>
              <Text strong>{t('接入供货')}：</Text>
              {boolTag(settings.capabilities.supplyIn)}
            </span>
            <span>
              <Text strong>{t('全联邦')}：</Text>
              {boolTag(settings.capabilities.federation)}
            </span>
          </div>
          <div style={{ display: 'flex', gap: 16, flexWrap: 'wrap' }}>
            <span>
              <Text strong>{t('商品模块')}：</Text>
              {boolTag(settings.modules.product)}
            </span>
            <span>
              <Text strong>{t('会员模块')}：</Text>
              {boolTag(settings.modules.member)}
            </span>
            <span>
              <Text strong>{t('订单模块')}：</Text>
              {boolTag(settings.modules.order)}
            </span>
          </div>
        </div>
      </Card>
    </div>
  );
}
