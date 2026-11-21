import { useCallback, useEffect, useState } from 'react';
import {
  Button,
  Card,
  Col,
  Form,
  Modal,
  Row,
  Space,
  Spin,
  Table,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';

/**
 * 客户端发版登记（热更新 P4，我方新增）。
 *
 * 小程序 / App 无法由服务器热更：运营在微信公众平台 / 应用市场完成发版后，
 * 在此按渠道登记「已针对哪个产品版本发布了哪个客户端版本」。登记即消警：
 * 首页的持续提醒（ClientUpdateAlert）为派生态，登记覆盖最新产品版本后自动
 * 消失。App 渠道同时展示「APP版本」页（原版有）的现行配置版本号供对照，
 * 并可深链过去更新。
 */

const { Text } = Typography;

interface SystemUpdateStatus {
  latestProductVersion?: string | null;
  currentProductVersion?: string;
  requiresMiniProgramUpdate?: boolean;
  requiresAppUpdate?: boolean;
}

interface ReleaseRow {
  id: number;
  channel: string;
  channelLabel: string;
  productVersion: string;
  clientVersion: string;
  note: string;
  operatorName: string;
  createdAt: number;
}

interface ChannelOverview {
  channel: string;
  label: string;
  latest?: ReleaseRow | null;
  covered: boolean;
  configuredAndroidVersion?: string | null;
  configuredIosVersion?: string | null;
}

interface Overview {
  targetProductVersion?: string | null;
  channels: ChannelOverview[];
  recent: ReleaseRow[];
}

function formatTime(sec: number) {
  if (!sec) return '-';
  return new Date(sec * 1000).toLocaleString();
}

function RequirementTag({
  required,
  covered,
  hasVersion,
}: {
  required: boolean;
  covered: boolean;
  hasVersion: boolean;
}) {
  const { t } = useI18n();
  if (!hasVersion) return <Tag color="grey">{t('暂无新版本信息')}</Tag>;
  if (!required) return <Tag color="grey">{t('本版无发版要求')}</Tag>;
  if (covered) return <Tag color="green">{t('已登记覆盖')}</Tag>;
  return <Tag color="orange">{t('待发布登记')}</Tag>;
}

function RegisterCard({
  channel,
  latestProductVersion,
  onDone,
}: {
  channel: string;
  latestProductVersion: string;
  onDone: () => void;
}) {
  const { t } = useI18n();
  const [formApi, setFormApi] = useState<any>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (formApi && latestProductVersion) {
      formApi.setValue('productVersion', latestProductVersion);
    }
  }, [formApi, latestProductVersion]);

  const submit = async () => {
    if (!formApi) return;
    try {
      const values = await formApi.validate();
      setSaving(true);
      await apiPost('/setting/clientRelease/register', { channel, ...values });
      Toast.success({ content: t('登记成功，提醒将自动消警') });
      formApi.setValue('clientVersion', '');
      formApi.setValue('note', '');
      onDone();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('登记失败') });
    } finally {
      setSaving(false);
    }
  };

  return (
    <Form
      getFormApi={setFormApi}
      labelPosition="left"
      labelAlign="right"
      labelWidth={130}
      initValues={{ productVersion: latestProductVersion }}
    >
      <Form.Input
        field="productVersion"
        label={t('对应产品版本')}
        placeholder={t('如 0.0.3')}
        rules={[{ required: true, message: t('请填写产品版本') }]}
      />
      <Form.Input
        field="clientVersion"
        label={channel === 'mini_program' ? t('小程序版本号') : t('App 版本号')}
        placeholder={t('已在渠道发布的版本号')}
        rules={[{ required: true, message: t('请填写已发布客户端版本') }]}
      />
      <Form.Input field="note" label={t('备注')} placeholder={t('如提审单号、发布渠道（可空）')} />
      <div style={{ paddingLeft: 130 }}>
        <Button theme="solid" loading={saving} onClick={submit}>
          {t('登记已发布')}
        </Button>
      </div>
    </Form>
  );
}

export default function ClientRelease({ onNavigate }: { onNavigate?: (page: string) => void }) {
  const { t } = useI18n();
  const [status, setStatus] = useState<SystemUpdateStatus | null>(null);
  const [overview, setOverview] = useState<Overview | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const st = await apiGet<SystemUpdateStatus>('/setting/systemUpdate/status').catch(
        () => null,
      );
      setStatus(st);
      const target = st?.latestProductVersion || undefined;
      const ov = await apiGet<Overview>('/setting/clientRelease/overview', {
        targetVersion: target,
      }).catch(() => null);
      setOverview(ov);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const remove = (row: ReleaseRow) => {
    Modal.confirm({
      title: t('删除这条登记？'),
      content: t('删除后若该版本仍有发版要求，提醒会重新出现。纠错可删除后重新登记。'),
      onOk: async () => {
        try {
          await apiPost('/setting/clientRelease/remove', { id: row.id });
          Toast.success({ content: t('已删除') });
          void load();
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('删除失败') });
        }
      },
    });
  };

  const latestVersion = status?.latestProductVersion || '';
  const channelOf = (channel: string) => overview?.channels?.find((c) => c.channel === channel);
  const requiredOf = (channel: string) =>
    channel === 'mini_program'
      ? !!status?.requiresMiniProgramUpdate
      : !!status?.requiresAppUpdate;

  const guideOf = (channel: string) =>
    channel === 'mini_program'
      ? t(
          '小程序代码只能在微信公众平台提交审核并发布，服务器无法自动更新。发布完成后在此登记，首页提醒自动消警。',
        )
      : t(
          'App 只能通过应用市场或下载链接发布新版：先在「APP版本」页更新 Android / iOS 版本号与下载链接，再到渠道发布安装包，完成后在此登记。',
        );

  return (
    <Spin spinning={loading}>
      <Space vertical align="start" spacing={16} style={{ width: '100%' }}>
        <Card className="table-card" shadows="hover" style={{ width: '100%' }}>
          <h3>{t('客户端发版登记')}</h3>
          <p>
            {t(
              '最新产品版本 {version}。小程序与 App 不能随 Web 端热更新，发版后在此登记即可消警；未登记者会在首页持续提醒。',
              { version: latestVersion || '-' },
            )}
          </p>
        </Card>

        <Row gutter={16} style={{ width: '100%' }}>
          {(['mini_program', 'app'] as const).map((channel) => {
            const ch = channelOf(channel);
            const label = channel === 'mini_program' ? t('小程序') : 'App';
            return (
              <Col span={12} key={channel}>
                <Card className="table-card" shadows="hover" style={{ width: '100%' }}>
                  <Space spacing={8} style={{ marginBottom: 8 }}>
                    <h4 style={{ margin: 0 }}>{label}</h4>
                    <RequirementTag
                      required={requiredOf(channel)}
                      covered={!!ch?.covered}
                      hasVersion={!!latestVersion}
                    />
                  </Space>
                  <p>
                    <Text type="tertiary">{guideOf(channel)}</Text>
                  </p>
                  {channel === 'app' && (
                    <p>
                      <Text type="tertiary">
                        {t('「APP版本」页现行配置：Android {android} / iOS {ios}', {
                          android: ch?.configuredAndroidVersion || '-',
                          ios: ch?.configuredIosVersion || '-',
                        })}
                      </Text>{' '}
                      {onNavigate && (
                        <Button
                          size="small"
                          theme="borderless"
                          onClick={() => onNavigate('appVersionManage')}
                        >
                          {t('去更新')}
                        </Button>
                      )}
                    </p>
                  )}
                  <p>
                    <Text type="tertiary">
                      {ch?.latest
                        ? t('最近登记：产品版本 {pv} → 客户端版本 {cv}（{by}，{at}）', {
                            pv: ch.latest.productVersion,
                            cv: ch.latest.clientVersion,
                            by: ch.latest.operatorName || '-',
                            at: formatTime(ch.latest.createdAt),
                          })
                        : t('暂无登记')}
                    </Text>
                  </p>
                  <RegisterCard
                    channel={channel}
                    latestProductVersion={latestVersion}
                    onDone={load}
                  />
                </Card>
              </Col>
            );
          })}
        </Row>

        <Card className="table-card" shadows="hover" style={{ width: '100%' }}>
          <h4>{t('登记记录')}</h4>
          <Table
            pagination={false}
            dataSource={overview?.recent || []}
            rowKey="id"
            columns={[
              {
                title: t('登记时间'),
                dataIndex: 'createdAt',
                render: (v: number) => formatTime(v),
              },
              { title: t('渠道'), dataIndex: 'channelLabel' },
              { title: t('对应产品版本'), dataIndex: 'productVersion' },
              { title: t('已发布客户端版本'), dataIndex: 'clientVersion' },
              { title: t('备注'), dataIndex: 'note', render: (v: string) => v || '-' },
              { title: t('登记人'), dataIndex: 'operatorName' },
              {
                title: t('操作'),
                render: (_: unknown, row: ReleaseRow) => (
                  <Button size="small" type="danger" theme="borderless" onClick={() => remove(row)}>
                    {t('删除')}
                  </Button>
                ),
              },
            ]}
          />
        </Card>
      </Space>
    </Spin>
  );
}
