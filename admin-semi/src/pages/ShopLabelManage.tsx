import { useCallback, useEffect, useState } from 'react';
import {
  Button,
  Form,
  Modal,
  Popconfirm,
  Select,
  Space,
  Spin,
  Switch,
  Table,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';

const { Text, Title } = Typography;

/**
 * 店铺标签（本平台新增，论坛帖 207878955346177229）。
 * 平台标签库：名称 / 颜色 / 排序 / 启停。给店铺打标在「店铺列表 → 标签」完成；
 * 买家侧在店铺名旁展示。与门店服务提示（标签管理 shopTipManage）是两套概念。
 * 二期：自动榜单授标规则——按成交额/成交单量/评分/收藏人数榜单定时重算，
 * 自动给上榜店铺授标、掉榜摘标；自动标签与手动打标并存、互不摘除。
 * 三期：榜单维度——全平台榜之外可开类目榜（限主营该一级类目的店铺）
 * 与地区榜（限登记地区在该省/市的店铺）；维度榜徽章在买家侧带维度
 * 前缀展示（如「手机数码类目·人气店铺」），不会被误读为全平台榜。
 */

interface ShopLabelItem {
  labelId: number;
  labelName: string;
  color: string;
  sortOrder: number;
  status: number;
  addTime: number;
}

interface ShopLabelRuleItem {
  ruleId: number;
  ruleName: string;
  labelId: number;
  labelName: string;
  metric: string;
  /** 三期维度：platform 全平台 / category 类目榜 / region 地区榜 */
  dimension: string;
  dimensionId: number;
  dimensionName: string;
  windowDays: number;
  rankTop: number;
  minValue: string | number;
  minSamples: number;
  status: number;
  lastRunTime: number;
  lastAwardCount: number;
  lastRemoveCount: number;
}

interface ShopLabelRuleLogItem {
  logId: number;
  ruleId: number;
  runTime: number;
  triggerType: string;
  awarded: number;
  removed: number;
  winners: string;
}

interface RuleRunSummary {
  ruleId: number;
  added: number;
  removed: number;
  awarded: number;
  winners: { shopId: number; value: string; samples: number }[];
}

const METRIC_OPTIONS = [
  { label: '成交额', value: 'sales_amount' },
  { label: '成交单量', value: 'sales_orders' },
  { label: '评分', value: 'rating' },
  { label: '收藏人数', value: 'collectors' },
];

const DIMENSION_OPTIONS = [
  { label: '全平台榜', value: 'platform' },
  { label: '类目榜', value: 'category' },
  { label: '地区榜', value: 'region' },
];

interface CategoryOption {
  categoryId: number;
  parentId: number;
  categoryName: string;
}

interface RegionOption {
  regionId: number;
  regionName: string;
  parentId: number;
  level: number;
}

function metricLabel(metric: string): string {
  return METRIC_OPTIONS.find((m) => m.value === metric)?.label || metric;
}

function formatTime(epochSeconds: number): string {
  if (!epochSeconds) return '--';
  return new Date(epochSeconds * 1000).toLocaleString();
}

const COLOR_PRESETS = ['#E93B3D', '#FF8800', '#F7B500', '#00B578', '#1677FF', '#722ED1', '#595959'];
const DEFAULT_COLOR = '#E93B3D';

export function LabelChip({ name, color }: { name: string; color?: string }) {
  return (
    <span
      style={{
        display: 'inline-block',
        padding: '0 6px',
        height: 18,
        lineHeight: '18px',
        fontSize: 11,
        color: '#fff',
        background: color || DEFAULT_COLOR,
        borderRadius: 0,
        whiteSpace: 'nowrap',
      }}
    >
      {name}
    </span>
  );
}

export default function ShopLabelManage() {
  const { t } = useI18n();
  const [rows, setRows] = useState<ShopLabelItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [modalVisible, setModalVisible] = useState(false);
  const [editing, setEditing] = useState<ShopLabelItem | null>(null);
  const [color, setColor] = useState(DEFAULT_COLOR);
  const [saving, setSaving] = useState(false);
  // 二期：自动授标规则
  const [rules, setRules] = useState<ShopLabelRuleItem[]>([]);
  const [rulesLoading, setRulesLoading] = useState(false);
  const [ruleModalVisible, setRuleModalVisible] = useState(false);
  const [editingRule, setEditingRule] = useState<ShopLabelRuleItem | null>(null);
  const [ruleSaving, setRuleSaving] = useState(false);
  const [runningRuleId, setRunningRuleId] = useState<number | null>(null);
  const [logsRule, setLogsRule] = useState<ShopLabelRuleItem | null>(null);
  const [logRows, setLogRows] = useState<ShopLabelRuleLogItem[]>([]);
  // 三期：维度选择（类目只列一级类目；地区省/市两级联动）
  const [rootCategories, setRootCategories] = useState<CategoryOption[]>([]);
  const [provinces, setProvinces] = useState<RegionOption[]>([]);
  const [cities, setCities] = useState<RegionOption[]>([]);
  const [ruleDimension, setRuleDimension] = useState('platform');
  const [ruleCategoryId, setRuleCategoryId] = useState<number | undefined>(undefined);
  const [regionProvinceId, setRegionProvinceId] = useState<number | undefined>(undefined);
  const [regionCityId, setRegionCityId] = useState<number | undefined>(undefined);

  const loadCities = useCallback(async (provinceId: number) => {
    try {
      const result = await apiGet<{ records?: RegionOption[] }>('/setting/region/list', {
        parentId: provinceId,
        page: 1,
        size: 200,
      });
      setCities(Array.isArray(result?.records) ? result.records : []);
    } catch {
      setCities([]);
    }
  }, []);

  useEffect(() => {
    // 一级类目（类目榜维度选项）：类目列表分页拉全后过滤 parentId=0
    void (async () => {
      try {
        const all: CategoryOption[] = [];
        let page = 1;
        let total = 0;
        do {
          const result = await apiGet<{ records: CategoryOption[]; total: number }>(
            '/product/category/list',
            { page, pageSize: 100 },
          );
          const records = result.records || [];
          all.push(...records);
          total = result.total || 0;
          page += 1;
          if (!records.length) break;
        } while (all.length < total && page <= 10);
        setRootCategories(all.filter((c) => !c.parentId));
      } catch {
        setRootCategories([]);
      }
    })();
    void (async () => {
      try {
        const result = await apiGet<{ records?: RegionOption[] }>('/setting/region/list', {
          parentId: 0,
          page: 1,
          size: 200,
        });
        setProvinces(Array.isArray(result?.records) ? result.records : []);
      } catch {
        setProvinces([]);
      }
    })();
  }, []);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await apiGet<ShopLabelItem[]>('/merchant/shopLabel/list');
      setRows(Array.isArray(data) ? data : []);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
    } finally {
      setLoading(false);
    }
  }, [t]);

  const loadRules = useCallback(async () => {
    setRulesLoading(true);
    try {
      const data = await apiGet<ShopLabelRuleItem[]>('/merchant/shopLabel/rules');
      setRules(Array.isArray(data) ? data : []);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
    } finally {
      setRulesLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void load();
    void loadRules();
  }, [load, loadRules]);

  const openRuleCreate = () => {
    setEditingRule(null);
    setRuleDimension('platform');
    setRuleCategoryId(undefined);
    setRegionProvinceId(undefined);
    setRegionCityId(undefined);
    setCities([]);
    setRuleModalVisible(true);
  };

  const openRuleEdit = async (row: ShopLabelRuleItem) => {
    setEditingRule(row);
    const dimension = row.dimension || 'platform';
    setRuleDimension(dimension);
    setRuleCategoryId(dimension === 'category' ? row.dimensionId : undefined);
    setRegionProvinceId(undefined);
    setRegionCityId(undefined);
    setCities([]);
    if (dimension === 'region' && row.dimensionId) {
      // 按 regionId 反查层级回显省/市选择（市榜需带出所属省）
      try {
        const result = await apiGet<{ records?: RegionOption[] }>('/setting/region/list', {
          regionId: row.dimensionId,
          page: 1,
          size: 1,
        });
        const rec = (result?.records || [])[0];
        if (rec && rec.level === 2) {
          setRegionProvinceId(rec.parentId);
          setRegionCityId(rec.regionId);
          void loadCities(rec.parentId);
        } else {
          setRegionProvinceId(row.dimensionId);
        }
      } catch {
        setRegionProvinceId(row.dimensionId);
      }
    }
    setRuleModalVisible(true);
  };

  const submitRule = async (values: Record<string, unknown>) => {
    const dimension = String(values.dimension || ruleDimension || 'platform');
    let dimensionId = 0;
    if (dimension === 'category') {
      if (!ruleCategoryId) {
        Toast.error({ content: t('请选择榜单类目') });
        return;
      }
      dimensionId = ruleCategoryId;
    } else if (dimension === 'region') {
      const target = regionCityId ?? regionProvinceId;
      if (!target) {
        Toast.error({ content: t('请选择榜单地区') });
        return;
      }
      dimensionId = target;
    }
    setRuleSaving(true);
    try {
      const payload = {
        ruleId: editingRule?.ruleId,
        ruleName: String(values.ruleName || '').trim(),
        labelId: Number(values.labelId || 0),
        metric: String(values.metric || 'sales_amount'),
        dimension,
        dimensionId,
        windowDays: Number(values.windowDays || 30),
        rankTop: Number(values.rankTop || 10),
        minValue: Number(values.minValue || 0),
        minSamples: Number(values.minSamples || 0),
        status: values.status === false ? 0 : 1,
      };
      if (editingRule) {
        await apiPost('/merchant/shopLabel/ruleUpdate', payload);
      } else {
        await apiPost('/merchant/shopLabel/ruleCreate', payload);
      }
      Toast.success({ content: t('保存成功') });
      setRuleModalVisible(false);
      void loadRules();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setRuleSaving(false);
    }
  };

  const toggleRuleStatus = async (row: ShopLabelRuleItem, checked: boolean) => {
    try {
      await apiPost('/merchant/shopLabel/ruleUpdate', {
        ruleId: row.ruleId,
        ruleName: row.ruleName,
        labelId: row.labelId,
        metric: row.metric,
        dimension: row.dimension || 'platform',
        dimensionId: row.dimensionId || 0,
        windowDays: row.windowDays,
        rankTop: row.rankTop,
        minValue: Number(row.minValue || 0),
        minSamples: row.minSamples,
        status: checked ? 1 : 0,
      });
      setRules((prev) =>
        prev.map((r) => (r.ruleId === row.ruleId ? { ...r, status: checked ? 1 : 0 } : r)),
      );
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    }
  };

  const removeRule = async (row: ShopLabelRuleItem) => {
    try {
      await apiPost('/merchant/shopLabel/ruleDel', { ruleId: row.ruleId });
      Toast.success({ content: t('删除成功') });
      void loadRules();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('删除失败') });
    }
  };

  const runRule = async (row: ShopLabelRuleItem) => {
    setRunningRuleId(row.ruleId);
    try {
      const summary = await apiPost<RuleRunSummary>('/merchant/shopLabel/ruleRun', {
        ruleId: row.ruleId,
      });
      Toast.success({
        content: `${t('重算完成')}：${t('新授')} ${summary.added} · ${t('摘标')} ${summary.removed} · ${t('当前上榜')} ${summary.awarded}`,
      });
      void loadRules();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('重算失败') });
    } finally {
      setRunningRuleId(null);
    }
  };

  const openLogs = async (row: ShopLabelRuleItem) => {
    setLogsRule(row);
    setLogRows([]);
    try {
      const data = await apiGet<ShopLabelRuleLogItem[]>(
        `/merchant/shopLabel/ruleLogs?ruleId=${row.ruleId}&limit=20`,
      );
      setLogRows(Array.isArray(data) ? data : []);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
    }
  };

  const ruleColumns = [
    { title: t('规则'), dataIndex: 'ruleName', width: 160 },
    {
      title: t('授标标签'),
      dataIndex: 'labelName',
      width: 130,
      render: (_: unknown, row: ShopLabelRuleItem) =>
        row.labelName ? (
          <LabelChip name={row.labelName} color={rows.find((l) => l.labelId === row.labelId)?.color} />
        ) : (
          <Text type="danger">{t('标签已删除')}</Text>
        ),
    },
    {
      title: t('榜单口径'),
      dataIndex: 'metric',
      width: 240,
      render: (_: unknown, row: ShopLabelRuleItem) => (
        <Text size="small">
          {row.dimension === 'category'
            ? `${t('类目')}：${row.dimensionName} · `
            : row.dimension === 'region'
              ? `${t('地区')}：${row.dimensionName} · `
              : `${t('全平台')} · `}
          {metricLabel(row.metric)}
          {row.metric === 'collectors'
            ? ` · ${t('总收藏')}`
            : ` · ${t('近')} ${row.windowDays} ${t('天')}`}
          {` · ${t('前')} ${row.rankTop} ${t('名')}`}
          {Number(row.minValue || 0) > 0 ? ` · ${t('门槛')} ${row.minValue}` : ''}
          {row.minSamples > 0 ? ` · ${t('样本≥')} ${row.minSamples}` : ''}
        </Text>
      ),
    },
    {
      title: t('启用'),
      dataIndex: 'status',
      width: 80,
      render: (v: number, row: ShopLabelRuleItem) => (
        <Switch checked={v === 1} onChange={(checked) => void toggleRuleStatus(row, checked)} />
      ),
    },
    {
      title: t('上次重算'),
      dataIndex: 'lastRunTime',
      width: 190,
      render: (_: unknown, row: ShopLabelRuleItem) => (
        <Text size="small">
          {formatTime(row.lastRunTime)}
          {row.lastRunTime
            ? ` · ${t('持标')} ${row.lastAwardCount} / ${t('摘标')} ${row.lastRemoveCount}`
            : ''}
        </Text>
      ),
    },
    {
      title: t('操作'),
      dataIndex: 'actions',
      width: 240,
      render: (_: unknown, row: ShopLabelRuleItem) => (
        <Space>
          <Button
            theme="borderless"
            size="small"
            loading={runningRuleId === row.ruleId}
            onClick={() => void runRule(row)}
          >
            {t('立即重算')}
          </Button>
          <Button theme="borderless" size="small" onClick={() => void openRuleEdit(row)}>
            {t('编辑')}
          </Button>
          <Button theme="borderless" size="small" onClick={() => void openLogs(row)}>
            {t('流水')}
          </Button>
          <Popconfirm
            title={t('删除该规则？')}
            content={t('删除后该规则已自动授出的标签将一并摘除，手动打标不受影响。')}
            onConfirm={() => void removeRule(row)}
          >
            <Button theme="borderless" size="small" type="danger">
              {t('删除')}
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  const openCreate = () => {
    setEditing(null);
    setColor(DEFAULT_COLOR);
    setModalVisible(true);
  };

  const openEdit = (row: ShopLabelItem) => {
    setEditing(row);
    setColor(row.color || DEFAULT_COLOR);
    setModalVisible(true);
  };

  const submit = async (values: Record<string, unknown>) => {
    setSaving(true);
    try {
      const payload = {
        labelId: editing?.labelId,
        labelName: String(values.labelName || '').trim(),
        color,
        sortOrder: Number(values.sortOrder || 0),
        status: values.status === false ? 0 : 1,
      };
      if (editing) {
        await apiPost('/merchant/shopLabel/update', payload);
      } else {
        await apiPost('/merchant/shopLabel/create', payload);
      }
      Toast.success({ content: t('保存成功') });
      setModalVisible(false);
      void load();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setSaving(false);
    }
  };

  const toggleStatus = async (row: ShopLabelItem, checked: boolean) => {
    try {
      await apiPost('/merchant/shopLabel/update', {
        labelId: row.labelId,
        labelName: row.labelName,
        color: row.color,
        sortOrder: row.sortOrder,
        status: checked ? 1 : 0,
      });
      setRows((prev) =>
        prev.map((r) => (r.labelId === row.labelId ? { ...r, status: checked ? 1 : 0 } : r)),
      );
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    }
  };

  const remove = async (row: ShopLabelItem) => {
    try {
      await apiPost('/merchant/shopLabel/del', { labelId: row.labelId });
      Toast.success({ content: t('删除成功') });
      void load();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('删除失败') });
    }
  };

  const columns = [
    {
      title: t('标签'),
      dataIndex: 'labelName',
      width: 220,
      render: (_: unknown, row: ShopLabelItem) => <LabelChip name={row.labelName} color={row.color} />,
    },
    { title: t('颜色'), dataIndex: 'color', width: 140, render: (v: string) => v || '--' },
    { title: t('排序'), dataIndex: 'sortOrder', width: 100 },
    {
      title: t('启用'),
      dataIndex: 'status',
      width: 100,
      render: (v: number, row: ShopLabelItem) => (
        <Switch checked={v === 1} onChange={(checked) => void toggleStatus(row, checked)} />
      ),
    },
    {
      title: t('操作'),
      dataIndex: 'actions',
      width: 160,
      render: (_: unknown, row: ShopLabelItem) => (
        <Space>
          <Button theme="borderless" size="small" onClick={() => openEdit(row)}>
            {t('编辑')}
          </Button>
          <Popconfirm
            title={t('删除该标签？')}
            content={t('删除后所有店铺上的该标签将一并移除。')}
            onConfirm={() => void remove(row)}
          >
            <Button theme="borderless" size="small" type="danger">
              {t('删除')}
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div style={{ padding: 16 }}>
      <Space style={{ marginBottom: 12, width: '100%', justifyContent: 'space-between' }}>
        <div>
          <Title heading={5} style={{ margin: 0 }}>
            {t('店铺标签')}
          </Title>
          <Text type="tertiary" size="small">
            {t('平台标签库，分配在「店铺列表 → 标签」；标签会展示在买家侧店铺名旁。')}
          </Text>
        </div>
        <Button type="primary" theme="solid" onClick={openCreate}>
          {t('新增标签')}
        </Button>
      </Space>
      <Spin spinning={loading}>
        <Table columns={columns} dataSource={rows} rowKey="labelId" pagination={false} />
      </Spin>

      <Space style={{ margin: '24px 0 12px', width: '100%', justifyContent: 'space-between' }}>
        <div>
          <Title heading={5} style={{ margin: 0 }}>
            {t('自动授标规则')}
          </Title>
          <Text type="tertiary" size="small">
            {t('按榜单定时重算（默认每小时）：上榜店铺自动授予标签，掉榜自动摘除；与手动打标并存、互不摘除。停用规则只停止重算，已授标签保留。类目榜/地区榜只在对应类目/地区的店铺内排名，徽章带维度前缀展示。')}
          </Text>
        </div>
        <Button type="primary" theme="solid" onClick={openRuleCreate}>
          {t('新增规则')}
        </Button>
      </Space>
      <Spin spinning={rulesLoading}>
        <Table columns={ruleColumns} dataSource={rules} rowKey="ruleId" pagination={false} />
      </Spin>

      <Modal
        title={editing ? t('编辑标签') : t('新增标签')}
        visible={modalVisible}
        onCancel={() => setModalVisible(false)}
        footer={null}
        width={440}
      >
        <Form
          key={editing ? `edit-${editing.labelId}` : 'create'}
          initValues={
            editing
              ? { labelName: editing.labelName, sortOrder: editing.sortOrder, status: editing.status === 1 }
              : { labelName: '', sortOrder: 0, status: true }
          }
          onSubmit={(values) => void submit(values as Record<string, unknown>)}
          labelPosition="left"
          labelWidth={72}
        >
          <Form.Input
            field="labelName"
            label={t('名称')}
            maxLength={10}
            showClear
            rules={[{ required: true, message: t('请输入标签名称') }]}
            placeholder={t('如：金牌商家')}
          />
          <Form.Slot label={t('颜色')}>
            <Space wrap>
              {COLOR_PRESETS.map((c) => (
                <span
                  key={c}
                  onClick={() => setColor(c)}
                  style={{
                    width: 22,
                    height: 22,
                    background: c,
                    cursor: 'pointer',
                    border: color === c ? '2px solid #1c1f23' : '2px solid transparent',
                    boxSizing: 'border-box',
                  }}
                />
              ))}
              <span style={{ fontSize: 12, color: 'var(--semi-color-text-2)' }}>{color}</span>
            </Space>
          </Form.Slot>
          <Form.InputNumber field="sortOrder" label={t('排序')} min={0} max={9999} />
          <Form.Switch field="status" label={t('启用')} />
          <div style={{ textAlign: 'right', marginTop: 8 }}>
            <Space>
              <Button onClick={() => setModalVisible(false)}>{t('取消')}</Button>
              <Button type="primary" theme="solid" htmlType="submit" loading={saving}>
                {t('保存')}
              </Button>
            </Space>
          </div>
        </Form>
      </Modal>

      <Modal
        title={editingRule ? t('编辑规则') : t('新增规则')}
        visible={ruleModalVisible}
        onCancel={() => setRuleModalVisible(false)}
        footer={null}
        width={480}
      >
        <Form
          key={editingRule ? `rule-edit-${editingRule.ruleId}` : 'rule-create'}
          initValues={
            editingRule
              ? {
                  ruleName: editingRule.ruleName,
                  labelId: editingRule.labelId,
                  metric: editingRule.metric,
                  dimension: editingRule.dimension || 'platform',
                  windowDays: editingRule.windowDays,
                  rankTop: editingRule.rankTop,
                  minValue: Number(editingRule.minValue || 0),
                  minSamples: editingRule.minSamples,
                  status: editingRule.status === 1,
                }
              : {
                  ruleName: '',
                  labelId: rows.find((l) => l.status === 1)?.labelId,
                  metric: 'sales_amount',
                  dimension: 'platform',
                  windowDays: 30,
                  rankTop: 10,
                  minValue: 0,
                  minSamples: 0,
                  status: true,
                }
          }
          onSubmit={(values) => void submitRule(values as Record<string, unknown>)}
          labelPosition="left"
          labelWidth={84}
        >
          <Form.Input
            field="ruleName"
            label={t('规则名称')}
            maxLength={20}
            showClear
            rules={[{ required: true, message: t('请输入规则名称') }]}
            placeholder={t('如：近30天成交额榜')}
          />
          <Form.Select
            field="labelId"
            label={t('授标标签')}
            rules={[{ required: true, message: t('请选择授标标签') }]}
            optionList={rows.map((l) => ({ label: l.labelName, value: l.labelId }))}
            style={{ width: '100%' }}
          />
          <Form.Select
            field="metric"
            label={t('榜单指标')}
            optionList={METRIC_OPTIONS.map((m) => ({ label: t(m.label), value: m.value }))}
            style={{ width: '100%' }}
          />
          <Form.Select
            field="dimension"
            label={t('榜单维度')}
            optionList={DIMENSION_OPTIONS.map((d) => ({ label: t(d.label), value: d.value }))}
            style={{ width: '100%' }}
            onChange={(v) => setRuleDimension(String(v || 'platform'))}
          />
          {ruleDimension === 'category' && (
            <Form.Slot label={t('榜单类目')}>
              <Select
                value={ruleCategoryId}
                placeholder={t('请选择一级类目')}
                filter
                onChange={(v) => setRuleCategoryId(typeof v === 'number' ? v : undefined)}
                style={{ width: '100%' }}
                optionList={rootCategories.map((c) => ({
                  label: c.categoryName,
                  value: c.categoryId,
                }))}
              />
              <div>
                <Text type="tertiary" size="small">
                  {t('仅一级类目。竞争范围 = 主营类目为该类目的店铺（主营按在售商品分布定）；买家侧徽章显示为「类目名类目·标签名」。')}
                </Text>
              </div>
            </Form.Slot>
          )}
          {ruleDimension === 'region' && (
            <Form.Slot label={t('榜单地区')}>
              <Space style={{ width: '100%' }}>
                <Select
                  value={regionProvinceId}
                  placeholder={t('请选择省')}
                  filter
                  onChange={(v) => {
                    const pid = typeof v === 'number' ? v : undefined;
                    setRegionProvinceId(pid);
                    setRegionCityId(undefined);
                    setCities([]);
                    if (pid) void loadCities(pid);
                  }}
                  style={{ width: 180 }}
                  optionList={provinces.map((r) => ({ label: r.regionName, value: r.regionId }))}
                />
                <Select
                  value={regionCityId}
                  placeholder={t('全省（可选市）')}
                  filter
                  showClear
                  onChange={(v) => setRegionCityId(typeof v === 'number' ? v : undefined)}
                  style={{ width: 180 }}
                  optionList={cities.map((r) => ({ label: r.regionName, value: r.regionId }))}
                />
              </Space>
              <div>
                <Text type="tertiary" size="small">
                  {t('选省即全省榜，再选市则为市榜。竞争范围 = 登记地区在该省/市内的店铺；买家侧徽章显示为「地区全名·标签名」。')}
                </Text>
              </div>
            </Form.Slot>
          )}
          <Form.InputNumber
            field="windowDays"
            label={t('统计窗口(天)')}
            min={1}
            max={365}
            extraText={t('收藏人数按当前总量排名，不用窗口。')}
          />
          <Form.InputNumber field="rankTop" label={t('上榜名额')} min={1} max={100} />
          <Form.InputNumber
            field="minValue"
            label={t('入榜门槛')}
            min={0}
            extraText={t('成交额=元 / 单量=单 / 评分=星 / 收藏=人，0 表示不设门槛。')}
          />
          <Form.InputNumber
            field="minSamples"
            label={t('样本下限')}
            min={0}
            extraText={t('防小样本霸榜：成交额看订单数、评分看评价数；单量与收藏不适用。评分榜建议 ≥5。')}
          />
          <Form.Switch field="status" label={t('启用')} />
          <div style={{ textAlign: 'right', marginTop: 8 }}>
            <Space>
              <Button onClick={() => setRuleModalVisible(false)}>{t('取消')}</Button>
              <Button type="primary" theme="solid" htmlType="submit" loading={ruleSaving}>
                {t('保存')}
              </Button>
            </Space>
          </div>
        </Form>
      </Modal>

      <Modal
        title={logsRule ? `${t('重算流水')} · ${logsRule.ruleName}` : t('重算流水')}
        visible={!!logsRule}
        onCancel={() => setLogsRule(null)}
        footer={null}
        width={640}
      >
        {logRows.length === 0 ? (
          <Text type="tertiary">{t('暂无重算记录。')}</Text>
        ) : (
          <Table
            columns={[
              { title: t('时间'), dataIndex: 'runTime', render: (v: number) => formatTime(v) },
              {
                title: t('触发'),
                dataIndex: 'triggerType',
                render: (v: string) => (v === 'manual' ? t('手动') : t('定时')),
              },
              { title: t('持标'), dataIndex: 'awarded' },
              { title: t('摘标'), dataIndex: 'removed' },
              {
                title: t('上榜名单（店铺ID:指标值）'),
                dataIndex: 'winners',
                render: (v: string) => {
                  try {
                    const list = JSON.parse(v || '[]') as { shopId: number; value: string }[];
                    return list.length
                      ? list.map((w) => `${w.shopId}:${w.value}`).join('，')
                      : '--';
                  } catch {
                    return '--';
                  }
                },
              },
            ]}
            dataSource={logRows}
            rowKey="logId"
            pagination={false}
            size="small"
          />
        )}
      </Modal>
    </div>
  );
}
