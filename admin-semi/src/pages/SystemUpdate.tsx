import { useCallback, useEffect, useRef, useState } from 'react';
import {
  Banner,
  Button,
  Card,
  Checkbox,
  Modal,
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
 * 系统更新（我方新增，admin 自助热更新 P1+P2）。
 *
 * P1 版本感知：四面当前 vs 最新版本、待执行迁移清单（语句级解析标注）、
 * 客户端渠道提醒；P2 静态三面一键更新：勾选面 → 确认页（全量差异 +
 * 破坏性迁移单列 + 密码再认证）→ 发起 job → 轮询进度与历史审计。
 * 后端（Rust）一键更新在 P3，本页明示不可选。
 */

interface SurfaceVersion {
  surface: string;
  label: string;
  configured: boolean;
  currentStamp: string | null;
  currentDetail: string | null;
  latestStamp: string | null;
  updateAvailable: boolean;
  updatable: boolean;
}

interface PendingMigration {
  file: string;
  sha256: string;
  summary: string;
  changes: string[];
  destructive: boolean;
  requiresBackendRestart: boolean;
  sqlPreview: string | null;
  sqlTruncated: boolean;
}

interface ShaMismatch {
  file: string;
  registeredSha256: string;
  manifestSha256: string;
}

interface UpdateJob {
  jobId: string;
  productVersion: string;
  surfaces: string[];
  status: string;
  stage: string;
  message: string;
  operatorName: string;
  destructiveAck: boolean;
  createdAt: number;
  updatedAt: number;
}

interface UpdateStatus {
  sourceConfigured: boolean;
  updaterConfigured: boolean;
  baselineReady: boolean;
  currentProductVersion: string;
  currentGitSha: string;
  latestProductVersion: string | null;
  updateAvailable: boolean;
  surfaces: SurfaceVersion[];
  pendingMigrations: PendingMigration[];
  appliedMigrationCount: number;
  shaMismatches: ShaMismatch[];
  requiresMiniProgramUpdate: boolean;
  requiresAppUpdate: boolean;
  releaseNotes: string | null;
  lastCheckedAt: number;
  activeJob: UpdateJob | null;
}

const STATUS_TAG: Record<string, { color: string; label: string }> = {
  queued: { color: 'grey', label: '排队中' },
  downloading: { color: 'blue', label: '下载中' },
  verifying: { color: 'blue', label: '校验中' },
  backing_up: { color: 'blue', label: '备份中' },
  swapping: { color: 'orange', label: '换版中' },
  health_checking: { color: 'orange', label: '健康检查' },
  done: { color: 'green', label: '已完成' },
  failed: { color: 'red', label: '失败' },
  rolling_back: { color: 'orange', label: '回滚中' },
  rolled_back: { color: 'amber', label: '已回滚' },
  rollback_failed: { color: 'red', label: '回滚失败' },
};

function fmtTime(sec: number): string {
  if (!sec) return '-';
  return new Date(sec * 1000).toLocaleString();
}

export default function SystemUpdate() {
  const { t } = useI18n();
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [jobs, setJobs] = useState<UpdateJob[]>([]);
  const [loading, setLoading] = useState(true);
  const [checking, setChecking] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [password, setPassword] = useState('');
  const [ackDestructive, setAckDestructive] = useState(false);
  const [creating, setCreating] = useState(false);
  const [expandedSql, setExpandedSql] = useState<string | null>(null);
  const timer = useRef<number | null>(null);

  const load = useCallback(async (refresh = false) => {
    try {
      const [s, j] = await Promise.all([
        refresh
          ? apiPost<UpdateStatus>('/setting/systemUpdate/refresh')
          : apiGet<UpdateStatus>('/setting/systemUpdate/status'),
        apiGet<UpdateJob[]>('/setting/systemUpdate/jobs'),
      ]);
      setStatus(s);
      setJobs(j || []);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
    } finally {
      setLoading(false);
      setChecking(false);
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  const hasActive = Boolean(status?.activeJob) || jobs.some((j) =>
    ['queued', 'downloading', 'verifying', 'backing_up', 'swapping', 'health_checking', 'rolling_back'].includes(j.status),
  );

  useEffect(() => {
    if (timer.current) window.clearInterval(timer.current);
    if (hasActive) {
      timer.current = window.setInterval(() => void load(), 3000);
    }
    return () => {
      if (timer.current) window.clearInterval(timer.current);
    };
  }, [hasActive, load]);

  const destructivePending = (status?.pendingMigrations || []).filter((m) => m.destructive);
  const canUpdate =
    status?.updaterConfigured &&
    status?.baselineReady &&
    status?.latestProductVersion &&
    !status?.activeJob;

  const openConfirm = () => {
    if (selected.length === 0) {
      Toast.warning({ content: t('请先勾选要更新的面') });
      return;
    }
    setPassword('');
    setAckDestructive(false);
    setConfirmOpen(true);
  };

  const createJob = async () => {
    if (!status?.latestProductVersion) return;
    if (destructivePending.length > 0 && !ackDestructive) {
      Toast.warning({ content: t('目标版本包含破坏性迁移，请先勾选知情确认') });
      return;
    }
    if (!password) {
      Toast.warning({ content: t('请输入当前登录密码确认') });
      return;
    }
    setCreating(true);
    try {
      await apiPost('/setting/systemUpdate/createJob', {
        targetVersion: status.latestProductVersion,
        surfaces: selected,
        password,
        confirmDestructive: ackDestructive,
      });
      Toast.success({ content: t('更新任务已发起') });
      setConfirmOpen(false);
      setSelected([]);
      await load();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('发起失败') });
    } finally {
      setCreating(false);
    }
  };

  if (loading) {
    return (
      <Card className="table-card" shadows="hover">
        <Spin tip={t('加载中')} />
      </Card>
    );
  }

  return (
    <Space vertical align="start" style={{ width: '100%' }} spacing="loose">
      <Card className="table-card" shadows="hover" style={{ width: '100%' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div>
            <h3 style={{ margin: 0 }}>{t('系统更新')}</h3>
            <p style={{ color: 'var(--semi-color-text-2)', margin: '6px 0 0' }}>
              {t('当前版本')} v{status?.currentProductVersion}（{status?.currentGitSha}）
              {status?.latestProductVersion
                ? ` · ${t('最新版本')} v${status.latestProductVersion}`
                : ` · ${t('尚未检查到最新版本')}`}
              {status?.lastCheckedAt ? ` · ${t('检查于')} ${fmtTime(status.lastCheckedAt)}` : ''}
            </p>
          </div>
          <Button
            loading={checking}
            onClick={() => {
              setChecking(true);
              void load(true);
            }}
          >
            {t('检查更新')}
          </Button>
        </div>

        {status && !status.sourceConfigured && (
          <Banner
            type="warning"
            description={t('更新源未配置或令牌不可读：当前只能看到本地版本，无法检查最新版本。请联系运维配置 update 段（github_repo / token_file）。')}
            style={{ marginTop: 12 }}
          />
        )}
        {status && !status.baselineReady && (
          <Banner
            type="warning"
            description={t('迁移登记基线未就绪（sys_schema_migration 为空）：在完成迁移 027 与基线回填前，不能发起任何更新。')}
            style={{ marginTop: 12 }}
          />
        )}
        {status && !status.updaterConfigured && (
          <Banner
            type="info"
            description={t('updater 未安装：当前为只读版本感知模式，一键更新不可用。')}
            style={{ marginTop: 12 }}
          />
        )}
        {status?.activeJob && (
          <Banner
            type="info"
            description={`${t('更新进行中')}：v${status.activeJob.productVersion} · ${
              STATUS_TAG[status.activeJob.status]?.label || status.activeJob.status
            } · ${status.activeJob.message || ''}`}
            style={{ marginTop: 12 }}
          />
        )}
        {(status?.requiresMiniProgramUpdate || status?.requiresAppUpdate) && (
          <Banner
            type="warning"
            description={t('最新版本包含客户端侧变更：小程序 / App 需要另行发版（服务器无法热更新客户端），请按发布流程处理，本提醒不影响 Web 四面更新。')}
            style={{ marginTop: 12 }}
          />
        )}
        {(status?.shaMismatches || []).map((m) => (
          <Banner
            key={m.file}
            type="danger"
            description={`${t('迁移文件已被修改')}：${m.file}（登记 sha 与清单不一致，请人工核查）`}
            style={{ marginTop: 12 }}
          />
        ))}
      </Card>

      <Card title={t('版本对比')} className="table-card" shadows="hover" style={{ width: '100%' }}>
        <Table
          pagination={false}
          dataSource={status?.surfaces || []}
          rowKey="surface"
          columns={[
            { title: t('面'), dataIndex: 'label', width: 140 },
            {
              title: t('当前版本'),
              dataIndex: 'currentStamp',
              render: (v: string | null, r: SurfaceVersion) =>
                r.configured ? (
                  <span>
                    {v || t('读不到')}
                    {r.currentDetail && (
                      <Typography.Text type="tertiary" size="small"> （{r.currentDetail}）</Typography.Text>
                    )}
                  </span>
                ) : (
                  <Typography.Text type="tertiary">{t('站点目录未配置')}</Typography.Text>
                ),
            },
            {
              title: t('最新版本'),
              dataIndex: 'latestStamp',
              render: (v: string | null) => v || '-',
            },
            {
              title: t('状态'),
              dataIndex: 'updateAvailable',
              width: 150,
              render: (v: boolean, r: SurfaceVersion) =>
                v ? (
                  <Tag color="orange">{t('有更新')}</Tag>
                ) : (
                  <Tag color="green">{t('已是最新')}</Tag>
                ),
            },
            {
              title: t('一键更新'),
              dataIndex: 'surface',
              width: 130,
              render: (s: string, r: SurfaceVersion) =>
                r.updatable ? (
                  <Checkbox
                    checked={selected.includes(s)}
                    disabled={!canUpdate || !r.updateAvailable}
                    onChange={(e) =>
                      setSelected((cur) =>
                        e.target.checked ? [...cur, s] : cur.filter((x) => x !== s),
                      )
                    }
                  >
                    {t('更新此面')}
                  </Checkbox>
                ) : (
                  <Typography.Text type="tertiary">{t('P3 开放')}</Typography.Text>
                ),
            },
          ]}
        />
        <div style={{ marginTop: 12, display: 'flex', gap: 8, alignItems: 'center' }}>
          <Button theme="solid" type="primary" disabled={!canUpdate || selected.length === 0} onClick={openConfirm}>
            {t('更新所选面')}
          </Button>
          <Typography.Text type="tertiary" size="small">
            {t('更新由服务器独立 updater 执行：下载 → 验 sha → 备份 → 换面 → 健康检查，失败自动回滚。')}
          </Typography.Text>
        </div>
        {status?.releaseNotes && (
          <details style={{ marginTop: 12 }}>
            <summary>{t('最新版更新说明')}</summary>
            <pre style={{ whiteSpace: 'pre-wrap', fontSize: 12 }}>{status.releaseNotes}</pre>
          </details>
        )}
      </Card>

      <Card
        title={`${t('待执行迁移')}（${status?.pendingMigrations.length || 0}）· ${t('已登记')} ${status?.appliedMigrationCount || 0}`}
        className="table-card"
        shadows="hover"
        style={{ width: '100%' }}
      >
        <Typography.Text type="tertiary" size="small">
          {t('以下标注按待执行迁移的 SQL 文本解析生成，非实时库结构比对；迁移的执行在 P3/部署轮处理，本期一键更新不执行迁移。')}
        </Typography.Text>
        <Table
          pagination={false}
          dataSource={status?.pendingMigrations || []}
          rowKey="file"
          columns={[
            { title: t('文件'), dataIndex: 'file', width: 240 },
            { title: t('摘要'), dataIndex: 'summary' },
            {
              title: t('字段变更（解析）'),
              dataIndex: 'changes',
              render: (changes: string[]) => (
                <Space wrap>
                  {changes.map((c) => (
                    <Tag key={c}>{c}</Tag>
                  ))}
                </Space>
              ),
            },
            {
              title: t('风险'),
              dataIndex: 'destructive',
              width: 110,
              render: (d: boolean) => (d ? <Tag color="red">{t('破坏性')}</Tag> : <Tag>{t('常规')}</Tag>),
            },
            {
              title: 'SQL',
              dataIndex: 'file',
              width: 90,
              render: (f: string, r: PendingMigration) => (
                <Button
                  size="small"
                  theme="borderless"
                  onClick={() => setExpandedSql(expandedSql === f ? null : f)}
                >
                  {expandedSql === f ? t('收起') : t('预览')}
                </Button>
              ),
            },
          ]}
        />
        {expandedSql &&
          (status?.pendingMigrations || [])
            .filter((m) => m.file === expandedSql)
            .map((m) => (
              <pre
                key={m.file}
                style={{ whiteSpace: 'pre-wrap', fontSize: 12, maxHeight: 320, overflow: 'auto' }}
              >
                {m.sqlPreview || t('清单未携带 SQL 预览')}
                {m.sqlTruncated ? `\n…（${t('已截断')}）` : ''}
              </pre>
            ))}
      </Card>

      <Card title={t('更新记录')} className="table-card" shadows="hover" style={{ width: '100%' }}>
        <Table
          pagination={false}
          dataSource={jobs}
          rowKey="jobId"
          columns={[
            { title: t('时间'), dataIndex: 'createdAt', width: 180, render: (v: number) => fmtTime(v) },
            { title: t('目标版本'), dataIndex: 'productVersion', width: 110, render: (v: string) => `v${v}` },
            {
              title: t('面'),
              dataIndex: 'surfaces',
              width: 160,
              render: (s: string[]) => s.join(', '),
            },
            {
              title: t('状态'),
              dataIndex: 'status',
              width: 120,
              render: (s: string) => <Tag color={(STATUS_TAG[s]?.color as never) || 'grey'}>{STATUS_TAG[s]?.label || s}</Tag>,
            },
            { title: t('说明'), dataIndex: 'message' },
            { title: t('操作人'), dataIndex: 'operatorName', width: 120 },
          ]}
        />
      </Card>

      <Modal
        title={t('确认系统更新')}
        visible={confirmOpen}
        onCancel={() => setConfirmOpen(false)}
        onOk={createJob}
        okText={t('确认更新')}
        cancelText={t('取消')}
        confirmLoading={creating}
        maskClosable={false}
      >
        <p>
          {t('目标版本')}：v{status?.latestProductVersion}；{t('更新面')}：
          {selected
            .map((s) => status?.surfaces.find((x) => x.surface === s)?.label || s)
            .join('、')}
        </p>
        <p>{t('执行流程：下载并校验 sha256 → 备份现行版本 → 换面 → 健康检查；任一步失败自动回滚到备份版本。数据库迁移不在本次执行范围内。')}</p>
        {destructivePending.length > 0 && (
          <div style={{ margin: '8px 0' }}>
            <Banner
              type="danger"
              description={`${t('目标版本包含破坏性迁移（删表/删字段/无条件删除等）')}：${destructivePending
                .map((m) => m.file)
                .join('、')}。${t('这些迁移不会被本次更新执行，但请确认已知悉。')}`}
            />
            <Checkbox checked={ackDestructive} onChange={(e) => setAckDestructive(Boolean(e.target.checked))}>
              {t('我已知悉上述破坏性迁移')}
            </Checkbox>
          </div>
        )}
        <div style={{ marginTop: 8 }}>
          <label style={{ display: 'block', marginBottom: 6 }}>{t('请输入当前登录密码确认')}</label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            style={{ width: '100%', padding: '6px 10px', border: '1px solid var(--semi-color-border)', borderRadius: 6 }}
            autoComplete="current-password"
          />
        </div>
      </Modal>
    </Space>
  );
}
