import { useEffect, useMemo, useState } from 'react';
import {
  Button,
  Input,
  Modal,
  Spin,
  Tag,
  Toast,
  Tooltip,
  Typography,
} from '@douyinfe/semi-ui';
import {
  IconDelete,
  IconEdit,
  IconImage,
  IconPlus,
  IconRefresh,
  IconSearch,
} from '@douyinfe/semi-icons';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import './groupon-item-manager.css';

const { Text } = Typography;

export interface GrouponSummary {
  grouponPromotionId: number;
  name?: string;
  status?: number;
  statusName?: string;
  startTime?: string;
  endTime?: string;
  groupType?: number;
}

export interface GrouponItemRow {
  grouponPromotionItemId?: number;
  itemId?: number;
  productId?: number;
  productName?: string;
  productSn?: string;
  picThumb?: string;
  picUrl?: string;
  productPrice?: number | string;
  grouponPrice?: number | string;
  minGroupPrice?: number | string;
  maxGroupPrice?: number | string;
  minGroupNum?: number;
  maxGroupNum?: number;
  minLimitNum?: number;
  maxLimitNum?: number;
  groupNum?: string | Array<{ groupNum?: number }>;
  sort?: number;
  [key: string]: unknown;
}

interface Props {
  visible: boolean;
  groupon: GrouponSummary | null;
  /** 外部（编辑保存、活动切换）触发的重新拉取信号 */
  refreshToken?: number;
  onClose: () => void;
  onEdit: (item?: GrouponItemRow) => void;
  onPoster: (item: GrouponItemRow) => void;
}

const itemKey = (row: GrouponItemRow): string =>
  String(row.grouponPromotionItemId ?? row.itemId ?? row.productId ?? '');

/** 金额：整数不带小数，非整数保留两位，带千分位 */
const formatMoney = (value: unknown): string => {
  const num = Number(value);
  if (!Number.isFinite(num)) return '—';
  return `¥${num.toLocaleString('zh-CN', {
    minimumFractionDigits: Math.abs(num % 1) > 0.0001 ? 2 : 0,
    maximumFractionDigits: 2,
  })}`;
};

/** 成团人数阶梯：可能是数组，也可能是 JSON 字符串 */
const parseTiers = (raw: GrouponItemRow['groupNum']): number[] => {
  const pick = (list: unknown): number[] => {
    if (!Array.isArray(list)) return [];
    return list
      .map((entry) => Number((entry as { groupNum?: number })?.groupNum))
      .filter((num) => Number.isFinite(num) && num > 0);
  };
  if (Array.isArray(raw)) return pick(raw);
  if (typeof raw === 'string' && raw.trim()) {
    try {
      return pick(JSON.parse(raw));
    } catch {
      return [];
    }
  }
  return [];
};

/** 缩略图：加载失败降级为首字占位，避免破图 */
function Thumb({ src, name }: { src?: string; name?: string }) {
  const [broken, setBroken] = useState(false);
  useEffect(() => setBroken(false), [src]);
  if (!src || broken) {
    return <span className="gpm-thumb-fallback">{(name || '?').slice(0, 1)}</span>;
  }
  return <img src={src} alt="" loading="lazy" onError={() => setBroken(true)} />;
}

export default function GrouponItemManager({
  visible,
  groupon,
  refreshToken = 0,
  onClose,
  onEdit,
  onPoster,
}: Props) {
  const { t } = useI18n();
  const [rows, setRows] = useState<GrouponItemRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [keyword, setKeyword] = useState('');
  const [innerRefresh, setInnerRefresh] = useState(0);
  const [removingId, setRemovingId] = useState<string | null>(null);

  const promotionId = groupon?.grouponPromotionId;
  const finished = Number(groupon?.status) === 2;

  useEffect(() => {
    if (!visible || !promotionId) return;
    let alive = true;
    setLoading(true);
    apiGet<any>('/promotion/groupon/item/list', {
      grouponPromotionId: promotionId,
      page: 1,
      size: 50,
    })
      .then((result) => {
        if (!alive) return;
        setRows(result?.records || result?.list || []);
      })
      .catch((error) => {
        if (alive) {
          setRows([]);
          Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
        }
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible, promotionId, refreshToken, innerRefresh]);

  // 切换活动时清空搜索词，避免上次的过滤条件带着走
  useEffect(() => {
    if (visible) setKeyword('');
  }, [promotionId, visible]);

  const filtered = useMemo(() => {
    const text = keyword.trim().toLowerCase();
    if (!text) return rows;
    return rows.filter((row) =>
      `${row.productName || row.name || ''} ${row.productSn || ''}`.toLowerCase().includes(text),
    );
  }, [rows, keyword]);

  const stats = useMemo(() => {
    const prices = rows
      .map((row) => Number(row.grouponPrice ?? row.minGroupPrice))
      .filter((num) => Number.isFinite(num) && num > 0);
    return {
      count: rows.length,
      lowest: prices.length ? Math.min(...prices) : null,
    };
  }, [rows]);

  const remove = (row: GrouponItemRow) => {
    const key = itemKey(row);
    Modal.confirm({
      title: t('确认移除该拼团商品？'),
      content: row.productName || '',
      okButtonProps: { type: 'danger', theme: 'solid' },
      onOk: async () => {
        setRemovingId(key);
        try {
          await apiPost('/promotion/groupon/item/del', {
            id: row.grouponPromotionItemId ?? row.itemId,
          });
          Toast.success({ content: t('删除成功') });
          setInnerRefresh((value) => value + 1);
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('删除失败') });
          throw error;
        } finally {
          setRemovingId(null);
        }
      },
    });
  };

  const statusColor = finished ? 'grey' : Number(groupon?.status) === 1 ? 'green' : 'orange';

  return (
    <Modal
      className="gpm-modal"
      visible={visible}
      title={null}
      header={null}
      footer={null}
      centered
      width={980}
      onCancel={onClose}
      closeOnEsc
    >
      <div className="gpm">
        {/* 活动概览 */}
        <header className="gpm-hero">
          <div className="gpm-hero-main">
            <div className="gpm-hero-eyebrow">
              <span className="gpm-hero-dot" />
              {t('拼团活动')}
              <Tag size="small" color={statusColor as any} className="gpm-hero-tag">
                {groupon?.statusName || t('进行中')}
              </Tag>
            </div>
            <h2 className="gpm-hero-title">{groupon?.name || t('管理商品')}</h2>
            <div className="gpm-hero-meta">
              <span className="gpm-hero-range">
                {groupon?.startTime || '—'} <em>→</em> {groupon?.endTime || '—'}
              </span>
              <span className="gpm-hero-sep" />
              <span>
                {t('商品')} <b>{stats.count}</b> {t('件')}
              </span>
              {stats.lowest !== null ? (
                <>
                  <span className="gpm-hero-sep" />
                  <span>
                    {t('最低拼团价')} <b className="gpm-hero-price">{formatMoney(stats.lowest)}</b>
                  </span>
                </>
              ) : null}
            </div>
          </div>
          <Button
            className="gpm-close"
            theme="borderless"
            type="tertiary"
            onClick={onClose}
            icon={<span aria-hidden>×</span>}
          />
        </header>

        {/* 工具条 */}
        <div className="gpm-toolbar">
          <div className="gpm-toolbar-left">
            <Button
              theme="solid"
              type="primary"
              icon={<IconPlus />}
              disabled={finished}
              onClick={() => onEdit()}
            >
              {t('添加拼团商品')}
            </Button>
            {finished ? (
              <Text type="tertiary" size="small">
                {t('活动已结束，不可再添加商品')}
              </Text>
            ) : (
              <Text type="tertiary" size="small">
                {t('添加后可设置成团人数、阶梯价与虚拟成团')}
              </Text>
            )}
          </div>
          <div className="gpm-toolbar-right">
            <Input
              className="gpm-search"
              prefix={<IconSearch />}
              placeholder={t('搜索商品名称 / 编号')}
              value={keyword}
              showClear
              onChange={setKeyword}
            />
            <Tooltip content={t('刷新')}>
              <Button
                theme="borderless"
                type="tertiary"
                icon={<IconRefresh spin={loading} />}
                onClick={() => setInnerRefresh((value) => value + 1)}
              />
            </Tooltip>
          </div>
        </div>

        {/* 商品列表 */}
        <div className="gpm-body">
          {loading && !rows.length ? (
            <div className="gpm-skeleton">
              {[0, 1, 2].map((key) => (
                <div className="gpm-skeleton-row" key={key}>
                  <span className="gpm-skeleton-thumb" />
                  <span className="gpm-skeleton-lines">
                    <i />
                    <i />
                  </span>
                </div>
              ))}
            </div>
          ) : filtered.length ? (
            <Spin spinning={loading}>
              <div className="gpm-list">
                {filtered.map((row) => {
                  const name = row.productName || (row.name as string) || '—';
                  const tiers = parseTiers(row.groupNum);
                  const lowTier = Number(row.minGroupNum ?? (tiers.length ? Math.min(...tiers) : 0));
                  const highTier = Number(row.maxGroupNum ?? (tiers.length ? Math.max(...tiers) : 0));
                  const groupLow = Number(row.grouponPrice ?? row.minGroupPrice);
                  const groupHigh = Number(row.grouponPrice ?? row.maxGroupPrice);
                  const original = Number(row.productPrice);
                  const save =
                    Number.isFinite(original) && Number.isFinite(groupLow) && original > groupLow
                      ? original - groupLow
                      : 0;
                  const discount =
                    Number.isFinite(original) && original > 0 && Number.isFinite(groupLow) && groupLow > 0
                      ? (groupLow / original) * 10
                      : 0;
                  return (
                    <article className="gpm-card" key={itemKey(row)}>
                      <div className="gpm-thumb">
                        <Thumb src={row.picThumb || row.picUrl} name={name} />
                      </div>

                      <div className="gpm-main">
                        <div className="gpm-name" title={name}>
                          {name}
                        </div>
                        <div className="gpm-sub">
                          {row.productSn ? <span className="gpm-sn">{row.productSn}</span> : null}
                          <span className="gpm-chip">
                            {t('成团')} {highTier > lowTier && lowTier ? `${lowTier}~${highTier}` : lowTier || '—'}{' '}
                            {t('人')}
                          </span>
                          {tiers.length > 1 ? (
                            <span className="gpm-chip">
                              {t('阶梯')} {tiers.length} {t('档')}
                            </span>
                          ) : null}
                          {row.minLimitNum || row.maxLimitNum ? (
                            <span className="gpm-chip">
                              {t('可售')}{' '}
                              {Number(row.maxLimitNum ?? 0) > Number(row.minLimitNum ?? 0)
                                ? `${row.minLimitNum ?? 0}~${row.maxLimitNum}`
                                : row.minLimitNum ?? row.maxLimitNum}
                            </span>
                          ) : null}
                          {row.sort !== undefined && row.sort !== null && Number(row.sort) !== 0 ? (
                            <span className="gpm-chip is-ghost">
                              {t('排序')} {row.sort}
                            </span>
                          ) : null}
                        </div>
                      </div>

                      <div className="gpm-price">
                        <div className="gpm-price-now">
                          {Number.isFinite(groupLow) ? (
                            groupHigh > groupLow ? (
                              <span className="is-range">
                                {formatMoney(groupLow)} <em>~</em> {formatMoney(groupHigh)}
                              </span>
                            ) : (
                              formatMoney(groupLow)
                            )
                          ) : (
                            '—'
                          )}
                        </div>
                        <div className="gpm-price-meta">
                          {Number.isFinite(original) && original > 0 ? (
                            <span className="gpm-price-old">{formatMoney(original)}</span>
                          ) : null}
                          {save > 0 ? (
                            <span className="gpm-price-save">
                              {t('省')} {formatMoney(save)}
                              {discount > 0 && discount < 10
                                ? ` · ${Number(discount.toFixed(1))}${t('折')}`
                                : ''}
                            </span>
                          ) : null}
                        </div>
                      </div>

                      <div className="gpm-actions">
                        <Tooltip content={t('编辑')}>
                          <Button
                            theme="borderless"
                            type="tertiary"
                            icon={<IconEdit />}
                            onClick={() => onEdit(row)}
                          />
                        </Tooltip>
                        <Tooltip content={t('推广海报')}>
                          <Button
                            theme="borderless"
                            type="tertiary"
                            icon={<IconImage />}
                            onClick={() => onPoster(row)}
                          />
                        </Tooltip>
                        <Tooltip content={t('移除')}>
                          <Button
                            className="gpm-danger"
                            theme="borderless"
                            type="danger"
                            icon={<IconDelete />}
                            loading={removingId === itemKey(row)}
                            onClick={() => remove(row)}
                          />
                        </Tooltip>
                      </div>
                    </article>
                  );
                })}
              </div>
            </Spin>
          ) : (
            <div className="gpm-empty">
              <div className="gpm-empty-icon">
                <IconImage size="extra-large" />
              </div>
              <div className="gpm-empty-title">
                {rows.length ? t('没有匹配的商品') : t('还没有拼团商品')}
              </div>
              <div className="gpm-empty-desc">
                {rows.length
                  ? t('换个关键词试试')
                  : t('添加商品并设置拼团价后，买家即可在拼团页参与')}
              </div>
              {rows.length ? (
                <Button theme="light" onClick={() => setKeyword('')}>
                  {t('清空搜索')}
                </Button>
              ) : (
                <Button
                  theme="solid"
                  type="primary"
                  icon={<IconPlus />}
                  disabled={finished}
                  onClick={() => onEdit()}
                >
                  {t('添加拼团商品')}
                </Button>
              )}
            </div>
          )}
        </div>
      </div>
    </Modal>
  );
}
