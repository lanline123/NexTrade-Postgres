import { useEffect, useState } from 'react';
import {
  Button,
  Checkbox,
  Empty,
  Input,
  InputNumber,
  Modal,
  Popconfirm,
  Radio,
  RadioGroup,
  Select,
  SideSheet,
  Space,
  Switch,
  Table,
  Tag,
  TextArea,
  Toast,
} from '@douyinfe/semi-ui';
import { apiDownload, apiGet, apiPost, apiUpload } from '../api';
import { useI18n } from '../i18n';
import ImageUploader from '../components/ImageUploader';
import type { ColumnProps } from '@douyinfe/semi-ui/lib/es/table';

/**
 * 礼品卡（giftCardManage）—— 对齐旧版 promotion/giftCard：
 * - 顶部「储值卡 / 兑换卡」双入口（groupType 1/2，储值卡仅平台侧可见）；
 * - 编辑表单按 groupType 分流：储值卡有面额，兑换卡需绑定适用商品；
 * - 查看卡列表（giftCards/list）、删除；
 * 契约：rust gift_card_groups.rs / gift_card.rs（GiftCardGroupsCreateParam 等，校验文案在后端）。
 */

interface GiftCardProductDTO {
  productId: number;
}

interface GroupRow {
  cardGroupsId: number;
  groupType: number; // 1 储值卡 2 兑换卡
  cardType: number; // 1 电子卡 2 实体卡
  cardGroupName: string;
  imageUrl: string;
  validityType: number; // 1 永久 2 天数 3 固定时间
  validityDays?: number;
  expireTime?: string;
  faceAmount?: string;
  status: number;
  addTime: string;
  groupTypeName: string;
  cardTypeName?: string;
  validityTypeName?: string;
  remark: string;
  giftCardProducts?: Array<{ productId: number; productName?: string; productImg?: string }>;
  canDelete?: boolean;
  exchangeMode?: number; // P1：1 固定套餐 2 多件任选（仅兑换卡）
  selectCount?: number;
}

interface GroupForm {
  cardType: number;
  cardGroupName: string;
  imageUrl: string;
  faceAmount: number | undefined;
  validityType: number;
  validityDays: number | undefined;
  expireTime: string;
  status: number;
  productIds: string[];
  remark: string;
  canDelete?: boolean;
  exchangeMode: number;
  selectCount: number | undefined;
}

const CARD_DEFAULT_BG = 'https://oss.tigshop.com/img/gallery/202511/1763106657gduKxujm9Fx7WZNjJZ.jpeg';

const groupLabel = (t: (k: string) => string, groupType: number) => (groupType === 1 ? t('储值卡') : t('兑换卡'));

const numOrNull = (v: unknown) => (v === '' || v === null || v === undefined ? undefined : Number(v));

/* ------------------------------ 卡组编辑弹窗 ------------------------------ */

function GroupEditModal({
  visible,
  groupType,
  row,
  onClose,
  onSaved,
}: {
  visible: boolean;
  groupType: number;
  row: GroupRow | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const { t } = useI18n();
  const isTopUp = groupType === 1;
  const label = groupLabel(t, groupType);
  const [form, setForm] = useState<GroupForm>({
    cardType: 1,
    cardGroupName: '',
    imageUrl: CARD_DEFAULT_BG,
    faceAmount: undefined,
    validityType: 1,
    validityDays: undefined,
    expireTime: '',
    status: 1,
    productIds: [],
    remark: '',
    exchangeMode: 1,
    selectCount: undefined,
  });
  const [productOptions, setProductOptions] = useState<Array<{ value: string; label: string }>>([]);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!visible) return;
    if (row) {
      setForm({
        cardType: row.cardType || 1,
        cardGroupName: row.cardGroupName || '',
        imageUrl: row.imageUrl || CARD_DEFAULT_BG,
        faceAmount: row.faceAmount ? Number(row.faceAmount) : undefined,
        validityType: row.validityType || 1,
        validityDays: row.validityDays || undefined,
        expireTime: row.expireTime || '',
        status: row.status ?? 1,
        productIds: (row.giftCardProducts || []).map((p) => String(p.productId)),
        remark: row.remark || '',
        canDelete: row.canDelete,
        exchangeMode: row.exchangeMode || 1,
        selectCount: row.selectCount || undefined,
      });
    } else {
      setForm({
        cardType: 1,
        cardGroupName: '',
        imageUrl: CARD_DEFAULT_BG,
        faceAmount: undefined,
        validityType: 1,
        validityDays: undefined,
        expireTime: '',
        status: 1,
        productIds: [],
        remark: '',
        exchangeMode: 1,
        selectCount: undefined,
      });
    }
    // 商品选项（兑换卡绑定用）
    apiGet<{ records: any[] }>('/product/product/list', { page: 1, size: 100 })
      .then((res) =>
        setProductOptions(
          (res.records || []).map((p) => ({
            value: String(p.productId ?? p.id),
            label: p.name ?? p.goodsName ?? `#${p.productId ?? p.id}`,
          })),
        ),
      )
      .catch(() => setProductOptions([]));
  }, [visible, row]);

  const submit = async () => {
    if (!form.cardGroupName.trim()) {
      Toast.warning({ content: t('请输入{name}名称', { name: label }) });
      return;
    }
    if (isTopUp && (!form.faceAmount || form.faceAmount <= 0)) {
      Toast.warning({ content: t('请输入储值卡面额') });
      return;
    }
    if (form.validityType === 2 && (!form.validityDays || form.validityDays <= 0)) {
      Toast.warning({ content: t('请输入有效天数') });
      return;
    }
    if (form.validityType === 3 && !form.expireTime) {
      Toast.warning({ content: t('请选择截止时间') });
      return;
    }
    if (!isTopUp && form.productIds.length === 0) {
      Toast.warning({ content: t('请选择适用商品') });
      return;
    }
    if (!isTopUp && form.exchangeMode === 2) {
      if (!form.selectCount || form.selectCount < 1) {
        Toast.warning({ content: t('任选模式必须填写任选件数') });
        return;
      }
      if (form.selectCount > form.productIds.length) {
        Toast.warning({ content: t('任选件数不能超过绑定商品数') });
        return;
      }
    }
    setSaving(true);
    try {
      const products: GiftCardProductDTO[] = form.productIds.map((id) => ({ productId: Number(id) }));
      if (row) {
        await apiPost('/promotion/giftCardGroups/update', {
          cardGroupsId: row.cardGroupsId,
          cardGroupName: form.cardGroupName.trim(),
          imageUrl: form.imageUrl,
          validityType: form.validityType,
          validityDays: form.validityType === 2 ? form.validityDays : undefined,
          expireTime: form.validityType === 3 ? form.expireTime : undefined,
          faceAmount: isTopUp ? form.faceAmount : undefined,
          remark: form.remark,
          giftCardProducts: isTopUp ? undefined : products,
          exchangeMode: isTopUp ? undefined : form.exchangeMode,
          selectCount: isTopUp ? undefined : form.exchangeMode === 2 ? form.selectCount : 0,
        });
      } else {
        await apiPost('/promotion/giftCardGroups/create', {
          groupType,
          cardType: form.cardType,
          cardGroupName: form.cardGroupName.trim(),
          imageUrl: form.imageUrl,
          validityType: form.validityType,
          validityDays: form.validityType === 2 ? form.validityDays : undefined,
          expireTime: form.validityType === 3 ? form.expireTime : undefined,
          faceAmount: isTopUp ? form.faceAmount : undefined,
          status: form.status,
          remark: form.remark,
          giftCardProducts: isTopUp ? undefined : products,
          exchangeMode: isTopUp ? undefined : form.exchangeMode,
          selectCount: isTopUp ? undefined : form.exchangeMode === 2 ? form.selectCount : 0,
        });
      }
      Toast.success({ content: t('保存成功') });
      onSaved();
      onClose();
    } catch (e) {
      Toast.error({ content: e instanceof Error ? e.message : t('保存失败') });
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      title={`${row ? t('编辑') : t('添加')}${label}`}
      visible={visible}
      onOk={submit}
      onCancel={onClose}
      width={560}
      okText={t('提交')}
      okButtonProps={{ loading: saving }}
      cancelText={t('取消')}
    >
      <div style={{ display: 'grid', gap: 12 }}>
        {!row && (
          <div style={{ display: 'grid', gap: 4 }}>
            <span>{label}{t('类型')}</span>
            <RadioGroup value={form.cardType} onChange={(e) => setForm({ ...form, cardType: Number(e.target.value) })}>
              <Space>
                <Radio value={1}>{t('电子卡')}</Radio>
                <Radio value={2}>{t('实体卡')}</Radio>
              </Space>
            </RadioGroup>
            <span style={{ fontSize: 12, color: 'var(--semi-color-text-2)' }}>
              {form.cardType === 1
                ? t('虚拟卡号，无实体卡片；购买后可在订单详情查看卡号及卡密')
                : t('快递发货实物卡片；收货后刮开图层查看卡密进行兑换')}
            </span>
          </div>
        )}
        <label style={{ display: 'grid', gap: 4 }}>
          <span>{label}{t('名称')}</span>
          <Input style={{ width: 300 }} value={form.cardGroupName} maxLength={50} onChange={(v) => setForm({ ...form, cardGroupName: v })} placeholder={t('请输入{name}名称', { name: label })} />
        </label>
        <div style={{ display: 'grid', gap: 4 }}>
          <span>{label}{t('图片')}</span>
          <ImageUploader single max={1} value={form.imageUrl ? [form.imageUrl] : []} onChange={(next) => setForm({ ...form, imageUrl: next[0] ?? '' })} />
        </div>
        {isTopUp && (
          <label style={{ display: 'grid', gap: 4 }}>
            <span>{t('储值卡面额')}</span>
            <Space>
              <InputNumber style={{ width: 200 }} min={0.01} precision={2} value={form.faceAmount} onChange={(v) => setForm({ ...form, faceAmount: numOrNull(v) })} placeholder={t('请输入金额')} />
              <span>{t('元')}</span>
            </Space>
            <span style={{ fontSize: 12, color: 'var(--semi-color-text-2)' }}>{t('此金额代表可兑换至余额中的金额')}</span>
          </label>
        )}
        <div style={{ display: 'grid', gap: 4 }}>
          <span>{t('有效期')}</span>
          <RadioGroup
            value={form.validityType}
            onChange={(e) => {
              const v = Number(e.target.value);
              setForm({ ...form, validityType: v, validityDays: v === 2 && !form.validityDays ? 30 : form.validityDays });
            }}
          >
            <Space spacing={16}>
              <Radio value={1}>{t('永久有效')}</Radio>
              {form.cardType !== 2 && <Radio value={2}>{t('天数')}</Radio>}
              <Radio value={3}>{t('固定时间')}</Radio>
            </Space>
          </RadioGroup>
          {form.validityType === 2 && form.cardType !== 2 && (
            <Space>
              <span>{t('购买后立即生效，')}</span>
              <InputNumber style={{ width: 120 }} min={1} precision={0} value={form.validityDays} onChange={(v) => setForm({ ...form, validityDays: numOrNull(v) })} />
              <span>{t('天有效')}</span>
            </Space>
          )}
          {form.validityType === 3 && (
            <Space>
              <span>{t('购买后立即生效，使用时间截止为')}</span>
              <Input style={{ width: 200 }} value={form.expireTime} placeholder="YYYY-MM-DD HH:mm:ss" onChange={(v) => setForm({ ...form, expireTime: v })} />
            </Space>
          )}
        </div>
        {!row && (
          <div style={{ display: 'grid', gap: 4 }}>
            <span>{label}{t('状态')}</span>
            <Switch checked={form.status === 1} onChange={(v) => setForm({ ...form, status: v ? 1 : 0 })} />
            <span style={{ fontSize: 12, color: 'var(--semi-color-text-2)' }}>{t('禁用后此卡项，不可出售')}</span>
          </div>
        )}
        {!isTopUp && (
          <div style={{ display: 'grid', gap: 4 }}>
            <span>{t('兑换模式')}</span>
            <RadioGroup value={form.exchangeMode} onChange={(e) => setForm({ ...form, exchangeMode: Number(e.target.value) })}>
              <Space>
                <Radio value={1}>{t('固定套餐（绑定商品全兑）')}</Radio>
                <Radio value={2}>{t('多件任选')}</Radio>
              </Space>
            </RadioGroup>
            {form.exchangeMode === 2 && (
              <Space>
                <span>{t('任选件数')}</span>
                <InputNumber
                  style={{ width: 120 }}
                  min={1}
                  max={Math.max(form.productIds.length, 1)}
                  precision={0}
                  value={form.selectCount}
                  onChange={(v) => setForm({ ...form, selectCount: numOrNull(v) })}
                />
                <span style={{ fontSize: 12, color: 'var(--semi-color-text-2)' }}>
                  {t('持卡人可从绑定的 {count} 件商品中任选 {n} 件兑换', { count: form.productIds.length, n: form.selectCount ?? 0 })}
                </span>
              </Space>
            )}
          </div>
        )}
        {!isTopUp && (
          <label style={{ display: 'grid', gap: 4 }}>
            <span>{t('适用商品')}</span>
            <Select
              style={{ width: '100%' }}
              multiple
              filter
              placeholder={t('请选择适用商品')}
              optionList={productOptions}
              value={form.productIds}
              onChange={(v) => setForm({ ...form, productIds: (v as string[]).map(String) })}
            />
          </label>
        )}
        <label style={{ display: 'grid', gap: 4 }}>
          <span>{t('备注')}</span>
          <TextArea rows={3} value={form.remark} onChange={(v: string) => setForm({ ...form, remark: v })} />
        </label>
      </div>
    </Modal>
  );
}

/* ------------------------------ 查看卡列表弹窗 ------------------------------ */

function CardListModal({ visible, row, onClose }: { visible: boolean; row: GroupRow | null; onClose: () => void }) {
  const { t } = useI18n();
  const [rows, setRows] = useState<any[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);
  const [keyword, setKeyword] = useState('');
  const [importing, setImporting] = useState(false);

  const load = (targetPage = 1) => {
    if (!row) return;
    setLoading(true);
    apiGet<{ records: any[]; total: number }>('/promotion/giftCards/list', {
      cardGroupsId: row.cardGroupsId,
      page: targetPage,
      size: 10,
      keyword: keyword || undefined,
    })
      .then((res) => {
        setRows(res.records || []);
        setTotal(res.total || 0);
      })
      .catch((e) => Toast.error({ content: e.message }))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    if (visible && row) {
      setPage(1);
      setKeyword('');
      load(1);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible, row]);

  const statusText = (r: any) => {
    if (Number(r.isExchange) === 1) return { text: t('已兑换'), color: 'grey' };
    if (Number(r.isSold) === 1) return { text: t('已售出'), color: 'green' };
    return { text: t('未售出'), color: 'blue' };
  };

  const onImportFile = async (file: File | null) => {
    if (!file || !row) return;
    setImporting(true);
    try {
      const res = await apiUpload<{ successCount: number; failCount: number; failList?: string[] }>(
        '/promotion/giftCards/importList',
        file,
        { cardGroupsId: String(row.cardGroupsId) },
      );
      if (res.failCount > 0) {
        Toast.warning({
          content: t('导入完成，成功 {success} 张，失败 {fail} 张', { success: res.successCount, fail: res.failCount }),
        });
      } else {
        Toast.success({ content: t('导入成功，共 {count} 张', { count: res.successCount }) });
      }
      load(1);
    } catch (e) {
      Toast.error({ content: e instanceof Error ? e.message : t('导入失败') });
    } finally {
      setImporting(false);
    }
  };

  return (
    <Modal title={`${t('查看卡列表')} — ${row?.cardGroupName ?? ''}`} visible={visible} onCancel={onClose} footer={null} width={820}>
      <Space style={{ marginBottom: 12 }} wrap>
        <Input style={{ width: 220 }} value={keyword} onChange={setKeyword} placeholder={t('卡号')} />
        <Button type="primary" onClick={() => { setPage(1); load(1); }}>{t('搜索')}</Button>
        <span style={{ flex: 1 }} />
        <Button
          loading={importing}
          onClick={() => document.getElementById('gift-card-import-file')?.click()}
        >
          {t('导入卡密')}
        </Button>
        <input
          id="gift-card-import-file"
          type="file"
          accept=".xlsx,.xls"
          style={{ display: 'none' }}
          onChange={(e) => {
            void onImportFile(e.target.files?.[0] ?? null);
            e.target.value = '';
          }}
        />
        <Button onClick={() => void apiDownload('/promotion/giftCards/importTemplate', undefined, 'gift_card_template.xlsx')}>
          {t('下载导入模板')}
        </Button>
        <Button
          onClick={() => {
            if (row) void apiDownload('/promotion/giftCards/exportList', { cardGroupsId: row.cardGroupsId }, 'gift_cards.xlsx');
          }}
        >
          {t('导出卡列表')}
        </Button>
      </Space>
      <Table
        loading={loading}
        dataSource={rows}
        rowKey="giftCardsId"
        pagination={{ currentPage: page, pageSize: 10, total, onPageChange: (p) => { setPage(p); load(p); } }}
        empty={<Empty description={t('暂无数据')} />}
        columns={[
          { title: t('卡号'), dataIndex: 'cardNumber', width: 170 },
          { title: t('卡密'), dataIndex: 'cardPassword', width: 140 },
          {
            title: t('状态'),
            width: 100,
            render: (_: any, r: any) => {
              const s = statusText(r);
              return <Tag color={s.color as any}>{s.text}</Tag>;
            },
          },
          { title: t('兑换商品'), render: (_: any, r: any) => ((r.giftCardProducts || []).map((p: any) => p.productName).join('、') || '-') },
        ] as ColumnProps<any>[]}
      />
    </Modal>
  );
}

/* ------------------------------ 发放礼品卡弹窗（P1） ------------------------------ */

/** 预检结果（仅定向发放 USER/RANK/TAG 使用；Excel 发卡走 issueByExcel 单次提交，不做预检） */
interface IssuePreview {
  validCount: number;
  failCount: number;
  needCards?: number;
  availableCards?: number;
  canIssue: boolean;
  shortage: number;
  failLines: string[];
}

/** Excel 发卡（issueByExcel）单次提交结果：成功/失败数 + 逐行失败明细 */
interface ExcelIssueResult {
  successCount: number;
  failCount: number;
  failLines: string[];
}

type IssueMode = 'USER' | 'RANK' | 'TAG' | 'EXCEL';

function IssueModal({ visible, row, onClose }: { visible: boolean; row: GroupRow | null; onClose: () => void }) {
  const { t } = useI18n();
  const [mode, setMode] = useState<IssueMode>('USER');
  const [remark, setRemark] = useState('');
  const [userKeyword, setUserKeyword] = useState('');
  const [users, setUsers] = useState<any[]>([]);
  const [userIds, setUserIds] = useState<number[]>([]);
  const [rankOptions, setRankOptions] = useState<Array<{ label: string; value: number }>>([]);
  const [rankIds, setRankIds] = useState<number[]>([]);
  const [tagOptions, setTagOptions] = useState<Array<{ label: string; value: number }>>([]);
  const [tagIds, setTagIds] = useState<number[]>([]);
  const [tagMatchAll, setTagMatchAll] = useState(false);
  const [excelFile, setExcelFile] = useState<File | null>(null);
  const [preview, setPreview] = useState<IssuePreview | null>(null);
  const [excelResult, setExcelResult] = useState<ExcelIssueResult | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!visible) return;
    setMode('USER');
    setRemark('');
    setUserIds([]);
    setRankIds([]);
    setTagIds([]);
    setTagMatchAll(false);
    setExcelFile(null);
    setPreview(null);
    setExcelResult(null);
    setUsers([]);
    setUserKeyword('');
    apiGet<any>('/promotion/coupon/config')
      .then((res) => setRankOptions((res?.rankList || []).map((r: any) => ({ label: r.rankName, value: r.rankId }))))
      .catch(() => setRankOptions([]));
    apiGet<any>('/user/memberTagGroup/list', { page: 1, size: 100 })
      .then(async (res) => {
        const groups = Array.isArray(res) ? res : res?.records || res?.list || [];
        const nested = await Promise.all(
          groups.map((g: any) =>
            apiGet<any[]>('/user/memberTag/listByGroup', { groupId: g.groupId })
              .then((tags) =>
                (tags || []).map((tag: any) => ({
                  label: `${g.groupName || g.groupId} / ${tag.tagName || tag.tagId}`,
                  value: Number(tag.tagId),
                })),
              )
              .catch(() => []),
          ),
        );
        setTagOptions(nested.flat());
      })
      .catch(() => setTagOptions([]));
  }, [visible, row]);

  const searchUsers = (keyword = '') => {
    apiGet<{ records?: any[] }>('/user/user/list', { page: 1, size: 20, keyword })
      .then((res) => setUsers(res.records || []))
      .catch(() => undefined);
  };

  const buildPayload = (): Record<string, any> | null => {
    if (!row) return null;
    // D3：一人一卡，原版 issue payload 无每人张数字段，不再传 cardsPerUser
    const payload: Record<string, any> = {
      cardGroupsId: row.cardGroupsId,
      issueType: mode,
      remark: remark || undefined,
    };
    if (mode === 'USER') payload.userIds = userIds;
    if (mode === 'RANK') payload.rankIds = rankIds;
    if (mode === 'TAG') {
      payload.tagIds = tagIds;
      payload.tagMatchAll = tagMatchAll;
    }
    return payload;
  };

  /** 定向发放（USER/RANK/TAG）预检：保留 P1 的 issuePreview/issueSend 两段式（D1 保留·标改口径） */
  const doPreview = async () => {
    if (!row || mode === 'EXCEL') return;
    setBusy(true);
    try {
      const payload = buildPayload();
      if (!payload) return;
      const res = await apiPost<any>('/promotion/giftCards/issuePreview', payload);
      setPreview({
        validCount: res.memberCount ?? 0,
        failCount: res.failCount ?? 0,
        needCards: res.needCount,
        availableCards: res.availableStock,
        canIssue: Boolean(res.canIssue),
        shortage: res.shortage ?? 0,
        failLines: (res.failDetails || []).map(
          (f: any) => `${f.memberName || f.mobile || `#${f.userId}`}：${f.reason}`,
        ),
      });
    } catch (e) {
      Toast.error({ content: e instanceof Error ? e.message : t('预检失败') });
    } finally {
      setBusy(false);
    }
  };

  /** 定向发放确认（一人一卡，无每人张数） */
  const doSend = async () => {
    if (!row || !preview || mode === 'EXCEL') return;
    if (!preview.canIssue) {
      Toast.warning({ content: t('库存不足或无可发放会员，无法发放') });
      return;
    }
    setBusy(true);
    try {
      const payload = buildPayload();
      if (!payload) return;
      const res = await apiPost<{ successCount: number; failCount: number }>('/promotion/giftCards/issueSend', payload);
      if (res.failCount > 0) {
        Toast.warning({
          content: t('发放完成，成功 {success} 人，失败 {fail} 人', { success: res.successCount, fail: res.failCount }),
        });
      } else {
        Toast.success({ content: t('发放成功，共 {count} 人', { count: res.successCount }) });
      }
      onClose();
    } catch (e) {
      Toast.error({ content: e instanceof Error ? e.message : t('发放失败') });
    } finally {
      setBusy(false);
    }
  };

  /**
   * Excel 发卡（D2 回归原版）：单次 multipart 提交 issueByExcel，
   * 逐行把模板中指定的已存在未使用卡发给指定会员，失败行跳过、其余继续；
   * 结果直接展示成功/失败数与逐行失败明细，不再走 excelPreview/excelSend 两段。
   */
  const doExcelIssue = async () => {
    if (!row) return;
    if (!excelFile) {
      Toast.warning({ content: t('请先选择 Excel 文件') });
      return;
    }
    setBusy(true);
    setExcelResult(null);
    try {
      const res = await apiUpload<any>('/promotion/giftCards/issueByExcel', excelFile, {
        cardGroupsId: String(row.cardGroupsId),
      });
      const failDetails: any[] = res.failDetails || res.failList || res.failures || [];
      const result: ExcelIssueResult = {
        successCount: res.successCount ?? 0,
        failCount: res.failCount ?? failDetails.length ?? 0,
        failLines: failDetails.map((f: any) =>
          typeof f === 'string'
            ? f
            : `${t('第 {row} 行', { row: f.row ?? f.line ?? '-' })}${f.rawValue ? `（${f.rawValue}）` : ''}：${f.reason || f.message || ''}`,
        ),
      };
      setExcelResult(result);
      if (result.failCount > 0) {
        Toast.warning({
          content: t('发放完成，成功 {success} 张，失败 {fail} 张', { success: result.successCount, fail: result.failCount }),
        });
      } else {
        Toast.success({ content: t('发放成功，共 {count} 张', { count: result.successCount }) });
      }
    } catch (e) {
      Toast.error({ content: e instanceof Error ? e.message : t('发放失败') });
    } finally {
      setBusy(false);
    }
  };

  /** Excel 发卡模板下载（issueTemplate，与卡密导入模板 importTemplate 是两个不同模板，勿混淆） */
  const downloadIssueTemplate = () => {
    void apiDownload('/promotion/giftCards/issueTemplate', undefined, 'gift_card_issue_template.xlsx');
  };

  return (
    <Modal
      title={`${t('发放礼品卡')} — ${row?.cardGroupName ?? ''}`}
      visible={visible}
      onCancel={onClose}
      width={640}
      footer={
        mode === 'EXCEL' ? (
          <Space>
            <Button onClick={onClose}>{t('取消')}</Button>
            <Button type="primary" loading={busy} disabled={!excelFile} onClick={() => void doExcelIssue()}>
              {t('确认发放')}
            </Button>
          </Space>
        ) : (
          <Space>
            <Button onClick={onClose}>{t('取消')}</Button>
            <Button loading={busy} onClick={() => void doPreview()}>{t('预检')}</Button>
            <Button type="primary" loading={busy} disabled={!preview?.canIssue} onClick={() => void doSend()}>
              {t('确认发放')}
            </Button>
          </Space>
        )
      }
    >
      <div style={{ display: 'grid', gap: 12 }}>
        <div style={{ display: 'grid', gap: 4 }}>
          <span>{t('发放对象')}</span>
          <RadioGroup value={mode} onChange={(e) => { setMode(e.target.value as IssueMode); setPreview(null); setExcelResult(null); }}>
            <Space>
              <Radio value="USER">{t('指定会员')}</Radio>
              <Radio value="RANK">{t('会员等级')}</Radio>
              <Radio value="TAG">{t('会员标签')}</Radio>
              <Radio value="EXCEL">{t('Excel 发卡')}</Radio>
            </Space>
          </RadioGroup>
        </div>
        {mode === 'USER' && (
          <>
            <Space>
              <Input style={{ width: 240 }} placeholder={t('会员名称/手机号')} value={userKeyword} onChange={setUserKeyword} onEnterPress={() => searchUsers(userKeyword)} />
              <Button onClick={() => searchUsers(userKeyword)}>{t('搜索')}</Button>
            </Space>
            <div style={{ display: 'grid', gap: 6, maxHeight: 200, overflow: 'auto' }}>
              {users.map((u) => (
                <label key={u.userId} style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                  <Checkbox
                    checked={userIds.includes(u.userId)}
                    onChange={(e) =>
                      setUserIds((cur) => (e.target.checked ? [...cur, u.userId] : cur.filter((id) => id !== u.userId)))
                    }
                  />
                  <span>{u.nickname || u.username || '-'}</span>
                  <span style={{ color: 'var(--semi-color-text-2)', fontSize: 12 }}>{u.mobile || ''}</span>
                </label>
              ))}
              {!users.length && <span style={{ color: 'var(--semi-color-text-2)', fontSize: 12 }}>{t('输入关键词搜索会员')}</span>}
            </div>
            <span style={{ fontSize: 12, color: 'var(--semi-color-text-2)' }}>{t('已选 {count} 位会员', { count: userIds.length })}</span>
          </>
        )}
        {mode === 'RANK' && (
          <Select
            placeholder={t('请选择会员等级')}
            multiple
            filter
            style={{ width: '100%' }}
            optionList={rankOptions}
            value={rankIds}
            onChange={(v) => setRankIds((v as number[]) || [])}
          />
        )}
        {mode === 'TAG' && (
          <>
            <Select
              placeholder={t('请选择会员标签')}
              multiple
              filter
              style={{ width: '100%' }}
              optionList={tagOptions}
              value={tagIds}
              onChange={(v) => setTagIds((v as number[]) || [])}
            />
            <Space>
              <span>{t('标签匹配')}</span>
              <RadioGroup value={tagMatchAll ? 'all' : 'any'} onChange={(e) => setTagMatchAll(e.target.value === 'all')}>
                <Radio value="any">{t('任一标签')}</Radio>
                <Radio value="all">{t('全部标签')}</Radio>
              </RadioGroup>
            </Space>
          </>
        )}
        {mode === 'EXCEL' && (
          <div style={{ display: 'grid', gap: 8 }}>
            {/* 说明文案逐字对齐原版 IssueExcel 弹窗（giftmall 源码审计 REPORT 2.3） */}
            <div style={{ display: 'grid', gap: 4, fontSize: 12, color: 'var(--semi-color-text-2)' }}>
              <span>{t('请先下载模板，按会员名、手机号、卡号、卡密、备注填写。卡号卡密必须已存在且未使用。')}</span>
              <span>{t('找不到会员或卡已使用的行会失败，其余行继续发卡。')}</span>
              <span>{t('文件大小不超过10MB。')}</span>
              <span>{t('导入未完成之前，请勿关闭页面，否则可能数据错误。')}</span>
            </div>
            <Space>
              <Button onClick={() => document.getElementById('gift-card-issue-file')?.click()}>
                {excelFile ? excelFile.name : t('选择 Excel 文件')}
              </Button>
              <Button theme="borderless" type="primary" onClick={downloadIssueTemplate}>{t('下载 Excel 发卡模板')}</Button>
            </Space>
            <input
              id="gift-card-issue-file"
              type="file"
              accept=".xlsx,.xls"
              style={{ display: 'none' }}
              onChange={(e) => {
                setExcelFile(e.target.files?.[0] ?? null);
                setExcelResult(null);
                e.target.value = '';
              }}
            />
            {excelResult && (
              <div style={{ background: 'var(--semi-color-fill-0)', borderRadius: 6, padding: 12, display: 'grid', gap: 6 }}>
                <div>
                  {t('发放完成，成功 {success} 张，失败 {fail} 张', { success: excelResult.successCount, fail: excelResult.failCount })}
                </div>
                {excelResult.failLines.length > 0 && (
                  <div style={{ maxHeight: 140, overflow: 'auto', fontSize: 12 }}>
                    {excelResult.failLines.map((line, i) => (
                      <div key={i}>{line}</div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        )}
        {mode !== 'EXCEL' && (
          <label style={{ display: 'grid', gap: 4 }}>
            <span>{t('备注')}</span>
            <Input value={remark} onChange={setRemark} placeholder={t('选填，记录在发放批次中')} />
          </label>
        )}
        {preview && mode !== 'EXCEL' && (
          <div style={{ background: 'var(--semi-color-fill-0)', borderRadius: 6, padding: 12, display: 'grid', gap: 6 }}>
            <div>
              {preview.needCards !== undefined
                ? t('有效会员 {valid} 人，需发 {need} 张，可用库存 {available} 张，失败 {fail} 人', {
                    valid: preview.validCount,
                    need: preview.needCards,
                    available: preview.availableCards ?? 0,
                    fail: preview.failCount,
                  })
                : t('名单解析成功 {valid} 人，失败 {fail} 人（库存将在发放时校验）', {
                    valid: preview.validCount,
                    fail: preview.failCount,
                  })}
            </div>
            {!preview.canIssue && (
              <div style={{ color: 'var(--semi-color-danger)' }}>
                {preview.shortage > 0
                  ? t('库存不足，还差 {n} 张，请先在卡列表中补充制卡/导入后再发放', { n: preview.shortage })
                  : t('没有可发放的会员')}
              </div>
            )}
            {preview.failLines.length > 0 && (
              <div style={{ maxHeight: 140, overflow: 'auto', fontSize: 12 }}>
                {preview.failLines.map((line, i) => (
                  <div key={i}>{line}</div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </Modal>
  );
}

/* ------------------------------ 发放记录抽屉（P1） ------------------------------ */

function IssueRecordsDrawer({ visible, row, onClose }: { visible: boolean; row: GroupRow | null; onClose: () => void }) {
  const { t } = useI18n();
  const [batches, setBatches] = useState<any[]>([]);
  const [batchTotal, setBatchTotal] = useState(0);
  const [batchPage, setBatchPage] = useState(1);
  const [loading, setLoading] = useState(false);
  const [activeBatch, setActiveBatch] = useState<any | null>(null);
  const [logs, setLogs] = useState<any[]>([]);
  const [logTotal, setLogTotal] = useState(0);
  const [logPage, setLogPage] = useState(1);

  const loadBatches = (targetPage = 1) => {
    if (!row) return;
    setLoading(true);
    apiGet<{ records: any[]; total: number }>('/promotion/giftCards/issueBatch', {
      cardGroupsId: row.cardGroupsId,
      page: targetPage,
      size: 10,
    })
      .then((res) => {
        setBatches(res.records || []);
        setBatchTotal(res.total || 0);
      })
      .catch((e) => Toast.error({ content: e.message }))
      .finally(() => setLoading(false));
  };

  const loadLogs = (batch: any, targetPage = 1) => {
    setLoading(true);
    apiGet<{ records: any[]; total: number }>('/promotion/giftCards/issueLog', {
      batchId: batch.id,
      page: targetPage,
      size: 10,
    })
      .then((res) => {
        setLogs(res.records || []);
        setLogTotal(res.total || 0);
      })
      .catch((e) => Toast.error({ content: e.message }))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    if (visible && row) {
      setActiveBatch(null);
      setBatchPage(1);
      loadBatches(1);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible, row]);

  const fmtTime = (ts: number) => (ts ? new Date(ts * 1000).toLocaleString() : '-');

  return (
    <SideSheet
      title={activeBatch ? `${t('发放明细')} — ${t('批次')} #${activeBatch.id}` : `${t('发放记录')} — ${row?.cardGroupName ?? ''}`}
      visible={visible}
      onCancel={onClose}
      width={720}
    >
      {activeBatch ? (
        <>
          <Button theme="borderless" type="primary" style={{ marginBottom: 12 }} onClick={() => setActiveBatch(null)}>
            {t('← 返回批次列表')}
          </Button>
          <Table
            loading={loading}
            dataSource={logs}
            rowKey="id"
            pagination={{
              currentPage: logPage,
              pageSize: 10,
              total: logTotal,
              onPageChange: (p) => { setLogPage(p); loadLogs(activeBatch, p); },
            }}
            empty={<Empty description={t('暂无数据')} />}
            columns={[
              { title: t('会员'), width: 160, render: (_: any, r: any) => r.nickname || r.username || `#${r.userId}` },
              { title: t('手机号'), dataIndex: 'mobile', width: 130 },
              { title: t('卡号（脱敏）'), dataIndex: 'cardNumber', width: 180 },
              { title: t('发放时间'), width: 170, render: (_: any, r: any) => fmtTime(r.issueTime) },
            ] as ColumnProps<any>[]}
          />
        </>
      ) : (
        <Table
          loading={loading}
          dataSource={batches}
          rowKey="id"
          pagination={{
            currentPage: batchPage,
            pageSize: 10,
            total: batchTotal,
            onPageChange: (p) => { setBatchPage(p); loadBatches(p); },
          }}
          empty={<Empty description={t('暂无数据')} />}
          columns={[
            { title: t('批次'), dataIndex: 'id', width: 70 },
            { title: t('发放方式'), dataIndex: 'issueTypeName', width: 110 },
            {
              title: t('成功/失败'),
              width: 110,
              render: (_: any, r: any) => `${r.successCount} / ${r.failCount}`,
            },
            { title: t('备注'), dataIndex: 'remark', render: (v: string) => v || '-' },
            { title: t('发放时间'), width: 170, render: (_: any, r: any) => fmtTime(r.addTime) },
            {
              title: t('操作'),
              width: 90,
              render: (_: any, r: any) => (
                <Button
                  theme="borderless"
                  type="primary"
                  onClick={() => { setActiveBatch(r); setLogPage(1); loadLogs(r, 1); }}
                >
                  {t('明细')}
                </Button>
              ),
            },
          ] as ColumnProps<any>[]}
        />
      )}
    </SideSheet>
  );
}

/* ---------------------------------- 列表页 ---------------------------------- */

export default function GiftCards() {
  const { t } = useI18n();
  const [rows, setRows] = useState<GroupRow[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(true);
  const [keyword, setKeyword] = useState('');
  const [edit, setEdit] = useState<{ open: boolean; groupType: number; row: GroupRow | null }>({ open: false, groupType: 2, row: null });
  const [cardListTarget, setCardListTarget] = useState<GroupRow | null>(null);
  const [issueTarget, setIssueTarget] = useState<GroupRow | null>(null);
  const [recordsTarget, setRecordsTarget] = useState<GroupRow | null>(null);
  const isAdmin = localStorage.getItem('adminType') === 'admin';

  const load = (targetPage = page) => {
    setLoading(true);
    apiGet<{ records: GroupRow[]; total: number }>('/promotion/giftCardGroups/list', {
      page: targetPage,
      size: 15,
      keyword: keyword || undefined,
    })
      .then((res) => {
        setRows(res.records || []);
        setTotal(res.total || 0);
      })
      .catch((e) => Toast.error({ content: e.message }))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    load(1);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const toggleStatus = async (row: GroupRow, checked: boolean) => {
    try {
      await apiPost('/promotion/giftCardGroups/update', { cardGroupsId: row.cardGroupsId, status: checked ? 1 : 0 });
      load();
    } catch (e) {
      Toast.error({ content: e instanceof Error ? e.message : t('操作失败') });
      load();
    }
  };

  const deleteGroup = async (row: GroupRow) => {
    try {
      await apiPost('/promotion/giftCardGroups/delete', { giftCardGroupsId: row.cardGroupsId });
      Toast.success({ content: t('删除成功') });
      load();
    } catch (e) {
      Toast.error({ content: e instanceof Error ? e.message : t('删除失败') });
    }
  };

  const columns: ColumnProps<GroupRow>[] = [
    { title: t('礼品卡名称'), dataIndex: 'cardGroupName', width: 180 },
    {
      title: t('状态'),
      dataIndex: 'status',
      width: 90,
      render: (_: any, row: GroupRow) => (
        <Switch checked={row.status === 1} disabled={row.canDelete === false} onChange={(v) => toggleStatus(row, v)} />
      ),
    },
    { title: t('备注'), dataIndex: 'remark', width: 200, render: (v: string) => v || '-' },
    { title: t('卡类型'), dataIndex: 'groupTypeName', width: 100, render: (v: string) => <Tag color={'blue' as const}>{v || '-'}</Tag> },
    { title: t('创建时间'), dataIndex: 'addTime', width: 170 },
    {
      title: t('操作'),
      width: 420,
      render: (_: any, row: GroupRow) => (
        <Space>
          <Button theme="borderless" type="primary" onClick={() => setCardListTarget(row)}>{t('查看卡列表')}</Button>
          <Button theme="borderless" type="primary" onClick={() => setIssueTarget(row)}>{t('发放礼品卡')}</Button>
          <Button theme="borderless" type="primary" onClick={() => setRecordsTarget(row)}>{t('发放记录')}</Button>
          <Button theme="borderless" type="primary" onClick={() => setEdit({ open: true, groupType: row.groupType, row })}>{t('编辑')}</Button>
          {(row.canDelete !== false || row.groupType === 2) && (
            <Popconfirm title={t('确认删除该{name}？', { name: groupLabel(t, row.groupType) })} onConfirm={() => deleteGroup(row)}>
              <Button theme="borderless" type="danger" disabled={row.canDelete === false}>{t('删除')}</Button>
            </Popconfirm>
          )}
        </Space>
      ),
    },
  ];

  return (
    <div style={{ padding: 20 }}>
      <p style={{ fontWeight: 500, marginBottom: 8 }}>{t('按使用场景快速创建')}</p>
      <Space style={{ marginBottom: 20 }} wrap>
        {isAdmin && (
          <div style={{ minWidth: 200, padding: '14px 18px', background: 'var(--semi-color-fill-0)', borderRadius: 6, display: 'flex', flexDirection: 'column', gap: 8 }}>
            <div style={{ fontSize: 15, fontWeight: 600 }}>{t('储值卡')}</div>
            <div style={{ fontSize: 12, color: 'var(--semi-color-text-2)' }}>{t('例：储值卡购买可进行兑换')}<br />{t('金额直接充入账户余额中')}</div>
            <Button type="primary" style={{ alignSelf: 'flex-start' }} onClick={() => setEdit({ open: true, groupType: 1, row: null })}>{t('立即创建')}</Button>
          </div>
        )}
        <div style={{ minWidth: 200, padding: '14px 18px', background: 'var(--semi-color-fill-0)', borderRadius: 6, display: 'flex', flexDirection: 'column', gap: 8 }}>
          <div style={{ fontSize: 15, fontWeight: 600 }}>{t('兑换卡')}</div>
          <div style={{ fontSize: 12, color: 'var(--semi-color-text-2)' }}>{t('例：可兑换该卡绑定的商品')}<br />{t('增强品牌与营销效果')}</div>
          <Button type="primary" style={{ alignSelf: 'flex-start' }} onClick={() => setEdit({ open: true, groupType: 2, row: null })}>{t('立即创建')}</Button>
        </div>
      </Space>
      <Space style={{ marginBottom: 16 }}>
        <Input style={{ width: 240 }} value={keyword} onChange={setKeyword} placeholder={t('输入礼品卡名称')} />
        <Button type="primary" onClick={() => { setPage(1); load(1); }}>{t('搜索')}</Button>
      </Space>
      <Table
        loading={loading}
        dataSource={rows}
        rowKey="cardGroupsId"
        columns={columns}
        empty={<Empty description={t('暂无数据')} />}
        pagination={{ currentPage: page, pageSize: 15, total, onPageChange: (p) => { setPage(p); load(p); } }}
      />
      <GroupEditModal
        visible={edit.open}
        groupType={edit.groupType}
        row={edit.row}
        onClose={() => setEdit({ open: false, groupType: 2, row: null })}
        onSaved={() => load()}
      />
      <CardListModal visible={!!cardListTarget} row={cardListTarget} onClose={() => setCardListTarget(null)} />
      <IssueModal visible={!!issueTarget} row={issueTarget} onClose={() => setIssueTarget(null)} />
      <IssueRecordsDrawer visible={!!recordsTarget} row={recordsTarget} onClose={() => setRecordsTarget(null)} />
    </div>
  );
}
