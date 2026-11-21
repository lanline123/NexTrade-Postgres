import { useCallback, useEffect, useRef, useState } from 'react';
import {
  Banner,
  Button,
  DatePicker,
  Input,
  InputNumber,
  Modal,
  Space,
  Table,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { IconPlus, IconSearch } from '@douyinfe/semi-icons';
import { apiDownload, apiGet, apiPost, apiUpload, getContextHeaders, getToken } from '../services/api';
import { isPlatformAccount } from '../components/BlacklistAddModal';
import TagFilter from '../framework/TagFilter';
import { useI18n } from '../i18n/I18nContext';

const { Text } = Typography;

/**
 * 店铺客户（店铺会员）页。
 *
 * 对应原版 view/admin merchant/customer/customer/List.vue + 平台
 * shopMember/Index.vue 的店铺下钻：平台账号先看店铺列表（带会员数），
 * 点进某店看该店客户；商家账号直接看本店客户（后端按登录作用域强制
 * 本店，shopId 入参忽略）。数据只读自 shop_member 沉淀表 + 平台会员
 * 资料联表展示，不触碰平台会员等级/积分/余额口径。
 *
 * 写操作（添加/导入）仅商家可用且受平台四开关门控（总开关冻结时后端
 * 亦拒绝）；「分组」复用平台会员标签体系（user_member_tag 本带 shop_id
 * 维度），经 /user/userMemberTag/batchBindTags 打标。
 */

interface SettingVO {
  shopMemberEnabled: number;
  shopMemberDesensitize: number;
  shopMemberAllowAdd: number;
  shopMemberAllowImport: number;
}

interface ShopRow {
  shopId: number;
  shopTitle: string;
  shopLogo: string | null;
  status: number;
  shopType: number;
  memberCount: number;
}

interface MemberRow {
  userId: number;
  username?: string | null;
  nickname: string | null;
  mobile: string | null;
  email?: string | null;
  avatar: string | null;
  orderCount: number;
  orderAmount: string | number;
  joinTime: number | null;
  lastConsumeTime: number | null;
}

interface PageResult<T> {
  records: T[];
  total: number;
}

const PAGE_SIZE = 20;

function formatTs(sec?: number | null): string {
  if (!sec) return '-';
  const d = new Date(sec * 1000);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function formatMoney(v: string | number | null | undefined): string {
  const n = Number(v ?? 0);
  return Number.isFinite(n) ? n.toFixed(2) : '0.00';
}

function rangeToSeconds(range: unknown): [number | undefined, number | undefined] {
  if (!Array.isArray(range) || range.length !== 2) return [undefined, undefined];
  const toSec = (d: unknown) => {
    const t = d instanceof Date ? d.getTime() : typeof d === 'number' ? d : NaN;
    return Number.isFinite(t) ? Math.floor(t / 1000) : undefined;
  };
  return [toSec(range[0]), toSec(range[1])];
}

export default function ShopCustomers() {
  const { t } = useI18n();
  const platform = isPlatformAccount();

  const [setting, setSetting] = useState<SettingVO | null>(null);

  // 平台：店铺列表
  const [shopKeyword, setShopKeyword] = useState('');
  const [shops, setShops] = useState<ShopRow[]>([]);
  const [shopTotal, setShopTotal] = useState(0);
  const [shopPage, setShopPage] = useState(1);
  const [shopLoading, setShopLoading] = useState(false);
  const [selectedShop, setSelectedShop] = useState<ShopRow | null>(null);

  // 客户列表
  const [members, setMembers] = useState<MemberRow[]>([]);
  const [memberTotal, setMemberTotal] = useState(0);
  const [memberPage, setMemberPage] = useState(1);
  const [memberLoading, setMemberLoading] = useState(false);
  const [keyword, setKeyword] = useState('');
  const [amountMin, setAmountMin] = useState<number | string>('');
  const [amountMax, setAmountMax] = useState<number | string>('');
  const [joinRange, setJoinRange] = useState<unknown>(undefined);
  const [consumeRange, setConsumeRange] = useState<unknown>(undefined);
  const [selectedKeys, setSelectedKeys] = useState<number[]>([]);

  // 弹窗
  const [detailRow, setDetailRow] = useState<MemberRow | null>(null);
  const [addVisible, setAddVisible] = useState(false);
  const [addMobile, setAddMobile] = useState('');
  const [addSaving, setAddSaving] = useState(false);
  const [importVisible, setImportVisible] = useState(false);
  const [importFile, setImportFile] = useState<File | null>(null);
  const [importing, setImporting] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [tagVisible, setTagVisible] = useState(false);
  const [tagIds, setTagIds] = useState<number[]>([]);
  const [tagSaving, setTagSaving] = useState(false);

  const inMemberView = !platform || selectedShop !== null;
  const frozen = setting !== null && setting.shopMemberEnabled !== 1;
  const canAdd = !platform && !frozen && setting?.shopMemberAllowAdd === 1;
  const canImport = !platform && !frozen && setting?.shopMemberAllowImport === 1;

  useEffect(() => {
    apiGet<SettingVO>('/user/shopMember/setting/get')
      .then((s) => setSetting(s))
      .catch(() => setSetting(null));
  }, []);

  const loadShops = useCallback(async () => {
    if (!platform || selectedShop) return;
    setShopLoading(true);
    try {
      const data = await apiGet<PageResult<ShopRow>>('/user/shopMember/shopList', {
        page: shopPage,
        size: PAGE_SIZE,
        shopTitle: shopKeyword.trim() || undefined,
      });
      setShops(Array.isArray(data?.records) ? data.records : []);
      setShopTotal(Number(data?.total || 0));
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
    } finally {
      setShopLoading(false);
    }
  }, [platform, selectedShop, shopPage, shopKeyword, t]);

  useEffect(() => {
    void loadShops();
  }, [loadShops]);

  const loadMembers = useCallback(async () => {
    if (!inMemberView) return;
    setMemberLoading(true);
    try {
      const [joinStart, joinEnd] = rangeToSeconds(joinRange);
      const [consumeStart, consumeEnd] = rangeToSeconds(consumeRange);
      const data = await apiGet<PageResult<MemberRow>>('/user/shopMember/list', {
        page: memberPage,
        size: PAGE_SIZE,
        shopId: platform ? selectedShop?.shopId : undefined,
        keyword: keyword.trim() || undefined,
        orderAmountMin: amountMin === '' ? undefined : Number(amountMin),
        orderAmountMax: amountMax === '' ? undefined : Number(amountMax),
        joinTimeStart: joinStart,
        joinTimeEnd: joinEnd,
        lastConsumeTimeStart: consumeStart,
        lastConsumeTimeEnd: consumeEnd,
      });
      setMembers(Array.isArray(data?.records) ? data.records : []);
      setMemberTotal(Number(data?.total || 0));
      setSelectedKeys([]);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
    } finally {
      setMemberLoading(false);
    }
  }, [inMemberView, platform, selectedShop, memberPage, keyword, amountMin, amountMax, joinRange, consumeRange, t]);

  useEffect(() => {
    void loadMembers();
  }, [loadMembers]);

  const openDetail = async (row: MemberRow) => {
    try {
      const data = await apiGet<MemberRow>('/user/shopMember/detail', {
        userId: row.userId,
        shopId: platform ? selectedShop?.shopId : undefined,
      });
      setDetailRow(data || row);
    } catch {
      setDetailRow(row);
    }
  };

  const submitAdd = async () => {
    const mobile = addMobile.trim();
    if (!mobile) {
      Toast.warning({ content: t('请输入手机号') });
      return;
    }
    setAddSaving(true);
    try {
      await apiPost<number>('/user/shopMember/add', { mobile });
      Toast.success({ content: t('添加成功') });
      setAddVisible(false);
      setAddMobile('');
      setMemberPage(1);
      void loadMembers();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('添加失败') });
    } finally {
      setAddSaving(false);
    }
  };

  const submitImport = async () => {
    if (!importFile) {
      Toast.warning({ content: t('请选择要导入的文件') });
      return;
    }
    setImporting(true);
    try {
      const ok = await apiUpload<unknown[]>('/user/shopMember/import', importFile);
      Toast.success({ content: t('导入成功') + `：${Array.isArray(ok) ? ok.length : 0}` });
      setImportVisible(false);
      setImportFile(null);
      setMemberPage(1);
      void loadMembers();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('导入失败') });
    } finally {
      setImporting(false);
    }
  };

  const downloadTemplate = async () => {
    try {
      await apiDownload('/user/shopMember/downloadImportTemplate', undefined, '店铺会员导入模板.xlsx');
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('下载失败') });
    }
  };

  const downloadErrors = async () => {
    if (!importFile) {
      Toast.warning({ content: t('请先选择已导入过的文件') });
      return;
    }
    try {
      const form = new FormData();
      form.append('file', importFile);
      const res = await fetch('/adminapi/user/shopMember/importErrorData', {
        method: 'POST',
        headers: { Authorization: <redacted>, ...getContextHeaders() },
        body: form,
      });
      if (!res.ok) throw new Error(t('下载失败'));
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = '店铺会员导入失败数据.xlsx';
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('下载失败') });
    }
  };

  const openTag = (keys: number[]) => {
    if (keys.length === 0) {
      Toast.warning({ content: t('请先选择客户') });
      return;
    }
    setSelectedKeys(keys);
    setTagIds([]);
    setTagVisible(true);
  };

  const submitTag = async () => {
    if (tagIds.length === 0) {
      Toast.warning({ content: t('请选择标签') });
      return;
    }
    setTagSaving(true);
    try {
      await apiPost('/user/userMemberTag/batchBindTags', {
        userIds: selectedKeys,
        tagIds,
      });
      Toast.success({ content: t('打标签成功') });
      setTagVisible(false);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    } finally {
      setTagSaving(false);
    }
  };

  const shopColumns = [
    {
      title: t('店铺'),
      dataIndex: 'shopTitle',
      render: (_: unknown, row: ShopRow) => (
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {row.shopLogo ? (
            <img src={row.shopLogo} alt="" style={{ width: 32, height: 32, objectFit: 'cover' }} />
          ) : null}
          <div>
            <div>{row.shopTitle || '-'}</div>
            <Text type="tertiary" size="small">ID {row.shopId}</Text>
          </div>
        </div>
      ),
    },
    {
      title: t('店铺会员数'),
      dataIndex: 'memberCount',
      render: (v: number) => <span>{v ?? 0}</span>,
    },
    {
      title: t('操作'),
      dataIndex: 'actions',
      render: (_: unknown, row: ShopRow) => (
        <Button
          theme="borderless"
          onClick={() => {
            setSelectedShop(row);
            setMemberPage(1);
          }}
        >
          {t('查看客户')}
        </Button>
      ),
    },
  ];

  const memberColumns = [
    {
      title: t('客户'),
      dataIndex: 'nickname',
      render: (_: unknown, row: MemberRow) => (
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {row.avatar ? (
            <img src={row.avatar} alt="" style={{ width: 32, height: 32, borderRadius: '50%', objectFit: 'cover' }} />
          ) : null}
          <div>
            <div>{row.nickname || '-'}</div>
            <Text type="tertiary" size="small">ID {row.userId}</Text>
          </div>
        </div>
      ),
    },
    { title: t('手机号'), dataIndex: 'mobile', render: (v: string | null) => <span>{v || '-'}</span> },
    {
      title: t('累计消费金额'),
      dataIndex: 'orderAmount',
      render: (v: string | number) => <span>{formatMoney(v)}</span>,
    },
    { title: t('累计订单数'), dataIndex: 'orderCount' },
    {
      title: t('入会时间'),
      dataIndex: 'joinTime',
      render: (v: number | null) => <span>{formatTs(v)}</span>,
    },
    {
      title: t('最近消费时间'),
      dataIndex: 'lastConsumeTime',
      render: (v: number | null) => <span>{formatTs(v)}</span>,
    },
    {
      title: t('操作'),
      dataIndex: 'actions',
      render: (_: unknown, row: MemberRow) => (
        <Space>
          <Button theme="borderless" onClick={() => void openDetail(row)}>
            {t('详情')}
          </Button>
          {!platform && (
            <Button theme="borderless" onClick={() => openTag([row.userId])}>
              {t('标签')}
            </Button>
          )}
        </Space>
      ),
    },
  ];

  return (
    <div>
      {frozen && (
        <Banner
          type="warning"
          fullMode={false}
          description={t(
            '店铺会员功能已关闭（冻结状态）：客户信息强制脱敏，添加与导入不可用。总开关在「会员设置」中开启。',
          )}
          style={{ marginBottom: 12 }}
        />
      )}

      {platform && !selectedShop ? (
        <>
          <Space style={{ marginBottom: 12 }}>
            <Input
              prefix={<IconSearch />}
              placeholder={t('店铺名称')}
              value={shopKeyword}
              onChange={(v) => setShopKeyword(v)}
              onEnterPress={() => {
                setShopPage(1);
                void loadShops();
              }}
              style={{ width: 240 }}
              showClear
            />
            <Button
              onClick={() => {
                setShopPage(1);
                void loadShops();
              }}
            >
              {t('搜索')}
            </Button>
          </Space>
          <Table
            columns={shopColumns}
            dataSource={shops}
            rowKey="shopId"
            loading={shopLoading}
            pagination={{
              currentPage: shopPage,
              pageSize: PAGE_SIZE,
              total: shopTotal,
              onPageChange: (p: number) => setShopPage(p),
            }}
          />
        </>
      ) : (
        <>
          <Space style={{ marginBottom: 12 }} wrap>
            {platform && (
              <Button onClick={() => setSelectedShop(null)}>{t('返回店铺列表')}</Button>
            )}
            {platform && selectedShop && (
              <Text strong>
                {selectedShop.shopTitle} · {t('店铺客户')}
              </Text>
            )}
            <Input
              prefix={<IconSearch />}
              placeholder={t('昵称 / 手机号 / 邮箱')}
              value={keyword}
              onChange={(v) => setKeyword(v)}
              onEnterPress={() => {
                setMemberPage(1);
                void loadMembers();
              }}
              style={{ width: 220 }}
              showClear
            />
            <InputNumber
              placeholder={t('消费金额起')}
              value={amountMin}
              onChange={(v) => setAmountMin(v ?? '')}
              style={{ width: 130 }}
            />
            <InputNumber
              placeholder={t('消费金额止')}
              value={amountMax}
              onChange={(v) => setAmountMax(v ?? '')}
              style={{ width: 130 }}
            />
            <DatePicker
              type="dateRange"
              placeholder={[t('入会开始'), t('入会结束')]}
              onChange={(d: unknown) => setJoinRange(d)}
              style={{ width: 260 }}
            />
            <DatePicker
              type="dateRange"
              placeholder={[t('最近消费开始'), t('最近消费结束')]}
              onChange={(d: unknown) => setConsumeRange(d)}
              style={{ width: 260 }}
            />
            <Button
              type="primary"
              onClick={() => {
                setMemberPage(1);
                void loadMembers();
              }}
            >
              {t('搜索')}
            </Button>
            {!platform && (
              <>
                <Button icon={<IconPlus />} disabled={!canAdd} onClick={() => setAddVisible(true)}>
                  {t('添加客户')}
                </Button>
                <Button disabled={!canImport} onClick={() => setImportVisible(true)}>
                  {t('导入客户')}
                </Button>
                <Button disabled={selectedKeys.length === 0} onClick={() => openTag(selectedKeys)}>
                  {t('批量打标签')}
                </Button>
              </>
            )}
          </Space>
          <Table
            columns={memberColumns}
            dataSource={members}
            rowKey="userId"
            loading={memberLoading}
            rowSelection={
              platform
                ? undefined
                : {
                    selectedRowKeys: selectedKeys,
                    onChange: (keys?: (string | number)[]) =>
                      setSelectedKeys((keys || []).map((k) => Number(k))),
                  }
            }
            pagination={{
              currentPage: memberPage,
              pageSize: PAGE_SIZE,
              total: memberTotal,
              onPageChange: (p: number) => setMemberPage(p),
            }}
          />
        </>
      )}

      <Modal
        title={t('客户详情')}
        visible={detailRow !== null}
        onCancel={() => setDetailRow(null)}
        footer={<Button onClick={() => setDetailRow(null)}>{t('关闭')}</Button>}
      >
        {detailRow && (
          <div style={{ display: 'grid', gridTemplateColumns: '110px 1fr', rowGap: 10 }}>
            <Text type="tertiary">{t('客户')}</Text>
            <span>
              {detailRow.nickname || '-'}（ID {detailRow.userId}）
            </span>
            <Text type="tertiary">{t('用户名')}</Text>
            <span>{detailRow.username || '-'}</span>
            <Text type="tertiary">{t('手机号')}</Text>
            <span>{detailRow.mobile || '-'}</span>
            <Text type="tertiary">{t('邮箱')}</Text>
            <span>{detailRow.email || '-'}</span>
            <Text type="tertiary">{t('累计消费金额')}</Text>
            <span>{formatMoney(detailRow.orderAmount)}</span>
            <Text type="tertiary">{t('累计订单数')}</Text>
            <span>{detailRow.orderCount}</span>
            <Text type="tertiary">{t('入会时间')}</Text>
            <span>{formatTs(detailRow.joinTime)}</span>
            <Text type="tertiary">{t('最近消费时间')}</Text>
            <span>{formatTs(detailRow.lastConsumeTime)}</span>
          </div>
        )}
      </Modal>

      <Modal
        title={t('添加客户')}
        visible={addVisible}
        onCancel={() => setAddVisible(false)}
        onOk={() => void submitAdd()}
        confirmLoading={addSaving}
        okText={t('添加')}
        cancelText={t('取消')}
      >
        <div style={{ display: 'grid', gap: 10 }}>
          <Input
            placeholder={t('请输入客户手机号')}
            value={addMobile}
            onChange={(v) => setAddMobile(v)}
            maxLength={20}
          />
          <Text type="tertiary" size="small">
            {t('仅可添加已注册的平台会员：按手机号匹配，匹配不到将提示不存在。')}
          </Text>
        </div>
      </Modal>

      <Modal
        title={t('导入客户')}
        visible={importVisible}
        onCancel={() => setImportVisible(false)}
        onOk={() => void submitImport()}
        confirmLoading={importing}
        okText={t('导入')}
        cancelText={t('取消')}
      >
        <div style={{ display: 'grid', gap: 12 }}>
          <Space>
            <Button onClick={() => void downloadTemplate()}>{t('下载导入模板')}</Button>
            <Button onClick={() => fileInputRef.current?.click()}>{t('选择文件')}</Button>
            <input
              ref={fileInputRef}
              type="file"
              accept=".xlsx,.xls"
              style={{ display: 'none' }}
              onChange={(e) => setImportFile(e.target.files?.[0] || null)}
            />
            <Text type="tertiary">{importFile ? importFile.name : t('未选择文件')}</Text>
          </Space>
          <Text type="tertiary" size="small">
            {t('单次最多导入 1000 条；仅绑定已注册会员，未匹配行将计入失败。导入后如有失败，可导出失败数据核对。')}
          </Text>
          <div>
            <Button theme="borderless" disabled={!importFile} onClick={() => void downloadErrors()}>
              {t('导出失败数据')}
            </Button>
          </div>
        </div>
      </Modal>

      <Modal
        title={t('打标签')}
        visible={tagVisible}
        onCancel={() => setTagVisible(false)}
        onOk={() => void submitTag()}
        confirmLoading={tagSaving}
        okText={t('确定')}
        cancelText={t('取消')}
      >
        <div style={{ display: 'grid', gap: 10 }}>
          <Text type="tertiary">
            {t('已选客户')}：{selectedKeys.length} {t('人')}
          </Text>
          <TagFilter value={tagIds} onChange={setTagIds} width={360} />
        </div>
      </Modal>
    </div>
  );
}
