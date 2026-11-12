import { useEffect, useMemo, useState } from 'react';
import {
  Button,
  Checkbox,
  Input,
  InputNumber,
  Modal,
  Spin,
  Switch,
  Tag,
  Toast,
  Tooltip,
  Typography,
} from '@douyinfe/semi-ui';
import {
  IconAlertCircle,
  IconChevronLeft,
  IconChevronRight,
  IconPlus,
  IconSearch,
  IconTick,
} from '@douyinfe/semi-icons';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import type { GrouponSummary } from './GrouponItemManager';
import './groupon-item-editor.css';

const { Text } = Typography;

/** 商品规格（`productSkus` / 详情同形） */
interface EditorSku {
  skuId: number;
  skuValue?: string;
  skuSn?: string;
  skuStock?: number;
  skuPrice?: number | string;
  isEnabled?: number;
}

/** 选品列表行：真实返回是 { product, productSkus, isAdded, ... } 两层结构 */
interface ProductListRow {
  product?: {
    productId?: number;
    productName?: string;
    productSn?: string;
    picThumb?: string;
    picUrl?: string;
    productPrice?: number | string;
    productStock?: number;
  };
  productSkus?: EditorSku[];
  isAdded?: boolean;
  isOtherGrouponProduct?: boolean;
  isReservationProduct?: boolean;
  isPresaleProduct?: boolean;
  isSeckillProduct?: boolean;
  isTimeDiscountProduct?: boolean;
}

interface PickedProduct {
  productId: number;
  productName: string;
  productSn: string;
  picThumb: string;
  productPrice: number;
  productStock: number;
  skus: EditorSku[];
}

/** 成团阶梯：key 为后端 `groupNumKey`，与 group_price 里的 key 必须一致 */
interface Tier {
  key: number;
  num: number;
}

interface SkuState {
  isJoin: boolean;
  limitNum: number;
  prices: Record<number, string>;
}

interface Props {
  visible: boolean;
  groupon: GrouponSummary | null;
  /** 编辑时：列表行 + 详情合并后的对象；新建时为占位对象 */
  initial: Record<string, any> | null;
  saving: boolean;
  onCancel: () => void;
  onSaved: () => void;
  onSavingChange: (saving: boolean) => void;
}

const money = (value: unknown): string => {
  const num = Number(value);
  if (!Number.isFinite(num)) return '—';
  return `¥${num.toLocaleString('zh-CN', {
    minimumFractionDigits: Math.abs(num % 1) > 0.0001 ? 2 : 0,
    maximumFractionDigits: 2,
  })}`;
};

/** 把「详情 + 列表行」归一成选品结果，兼容两种来源 */
const normalizePicked = (initial: Record<string, any> | null): PickedProduct | null => {
  if (!initial) return null;
  const productId = Number(initial.productId ?? initial.product?.productId ?? 0);
  if (!productId) return null;
  const skus: EditorSku[] = (initial.productSkus || initial.product?.productSkus || []).map((sku: any) => ({
    ...sku,
    skuId: Number(sku.skuId),
    skuPrice: sku.skuPrice,
  }));
  return {
    productId,
    productName: initial.productName || initial.product?.productName || '',
    productSn: initial.productSn || initial.product?.productSn || '',
    picThumb: initial.picThumb || initial.product?.picThumb || '',
    productPrice: Number(initial.productPrice ?? initial.product?.productPrice ?? 0),
    productStock: Number(initial.productStock ?? initial.product?.productStock ?? 0),
    skus,
  };
};

/** 规格状态：优先用详情里的明细与阶梯价，未出现的规格默认不参加 */
const buildSkuState = (
  skus: EditorSku[],
  details: any[] | undefined,
  tiers: Tier[],
  isEdit: boolean,
): Record<number, SkuState> => {
  const map: Record<number, SkuState> = {};
  skus.forEach((sku) => {
    const matched = (details || []).find((d) => Number(d.skuId) === Number(sku.skuId));
    const prices: Record<number, string> = {};
    (matched?.groupPromotionDetailPrices || []).forEach((p: any) => {
      const key = Number(p.groupNumKey);
      prices[key] = p.price === null || p.price === undefined ? '' : String(p.price);
    });
    map[sku.skuId] = {
      // 编辑：明细里没有该规格 = 当时没勾选；新建：默认全选
      isJoin: matched ? Number(matched.isJoin ?? 1) === 1 : !isEdit,
      limitNum: Number(matched?.limitNum ?? sku.skuStock ?? 0),
      prices,
    };
  });
  // 兜底：若详情带了明细里没有的规格，按阶梯补空价格
  tiers.forEach((tier) => {
    Object.values(map).forEach((state) => {
      if (state.prices[tier.key] === undefined) state.prices[tier.key] = '';
    });
  });
  return map;
};

const buildTiers = (initial: Record<string, any> | null): Tier[] => {
  const raw = initial?.groupNums;
  if (Array.isArray(raw) && raw.length) {
    return raw.map((tier: any, index: number) => ({
      key: Number(tier.groupNumKey ?? index),
      num: Number(tier.groupNum ?? 2),
    }));
  }
  return [{ key: 0, num: 2 }];
};

export default function GrouponItemEditor({
  visible,
  groupon,
  initial,
  saving,
  onCancel,
  onSaved,
  onSavingChange,
}: Props) {
  const { t } = useI18n();
  const isEdit = Boolean(initial?.grouponPromotionItemId);
  const groupType = Number(groupon?.groupType ?? initial?.groupType ?? 1);
  const isLadder = groupType === 2;

  const [picked, setPicked] = useState<PickedProduct | null>(() => normalizePicked(initial));
  const [picking, setPicking] = useState<boolean>(() => !normalizePicked(initial));
  const [tiers, setTiers] = useState<Tier[]>(() => buildTiers(initial));
  const [skuState, setSkuState] = useState<Record<number, SkuState>>(() => {
    const product = normalizePicked(initial);
    return product
      ? buildSkuState(product.skus, initial?.grouponPromotionDetails, buildTiers(initial), isEdit)
      : {};
  });
  const [orderLimitNum, setOrderLimitNum] = useState<number>(Number(initial?.orderLimitNum ?? 0));
  const [isAutoGroup, setIsAutoGroup] = useState<boolean>(Number(initial?.isAutoGroup ?? 0) === 1);
  const [sort, setSort] = useState<number>(Number(initial?.sort ?? 0));

  // —— 选品 ——
  const [keyword, setKeyword] = useState('');
  const [rows, setRows] = useState<ProductListRow[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [listLoading, setListLoading] = useState(false);
  const PAGE_SIZE = 8;

  const loadProducts = async (nextPage = page, nextKeyword = keyword) => {
    if (!groupon) return;
    setListLoading(true);
    try {
      const result = await apiGet<any>('/promotion/groupon/item/productList', {
        keyword: nextKeyword.trim() || undefined,
        grouponPromotionId: groupon.grouponPromotionId,
        page: nextPage,
        size: PAGE_SIZE,
      });
      setRows(result?.records || result?.list || []);
      setTotal(Number(result?.total ?? 0));
      setPage(nextPage);
    } catch (error) {
      setRows([]);
      Toast.error({ content: error instanceof Error ? error.message : t('商品加载失败') });
    } finally {
      setListLoading(false);
    }
  };

  useEffect(() => {
    if (!visible || !picking) return;
    loadProducts(1, '');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible, picking]);

  const choose = (row: ProductListRow) => {
    const product = row.product || {};
    const skus: EditorSku[] = (row.productSkus || []).map((sku) => ({
      ...sku,
      skuId: Number(sku.skuId),
    }));
    setPicked({
      productId: Number(product.productId ?? 0),
      productName: product.productName || '',
      productSn: product.productSn || '',
      picThumb: product.picThumb || product.picUrl || '',
      productPrice: Number(product.productPrice ?? 0),
      productStock: Number(product.productStock ?? 0),
      skus,
    });
    setSkuState(buildSkuState(skus, undefined, tiers, false));
    setPicking(false);
  };

  const blockedReason = (row: ProductListRow): string | null => {
    if (row.isAdded) return t('已加入本活动');
    if (row.isOtherGrouponProduct) return t('已参加其它拼团');
    if (row.isSeckillProduct) return t('正在秒杀');
    if (row.isTimeDiscountProduct) return t('正在限时折扣');
    return null;
  };

  const joined = useMemo(
    () => (picked ? picked.skus.filter((sku) => skuState[sku.skuId]?.isJoin) : []),
    [picked, skuState],
  );

  const addTier = () => {
    const maxKey = tiers.reduce((acc, tier) => Math.max(acc, tier.key), -1);
    const lastNum = tiers[tiers.length - 1]?.num ?? 2;
    const next: Tier = { key: maxKey + 1, num: lastNum + 1 };
    setTiers((current) => [...current, next]);
  };

  const removeTier = (key: number) => {
    setTiers((current) => current.filter((tier) => tier.key !== key));
  };

  const patchSku = (skuId: number, patch: Partial<SkuState>) => {
    setSkuState((current) => ({ ...current, [skuId]: { ...current[skuId], ...patch } }));
  };

  const setPrice = (skuId: number, key: number, value: string) => {
    setSkuState((current) => ({
      ...current,
      [skuId]: { ...current[skuId], prices: { ...(current[skuId]?.prices || {}), [key]: value } },
    }));
  };

  const fillWithOriginalPrice = () => {
    if (!picked) return;
    setSkuState((current) => {
      const next = { ...current };
      picked.skus.forEach((sku) => {
        const state = next[sku.skuId];
        if (!state?.isJoin) return;
        const prices = { ...state.prices };
        tiers.forEach((tier) => {
          if (!prices[tier.key]) prices[tier.key] = String(Number(sku.skuPrice ?? 0));
        });
        next[sku.skuId] = { ...state, prices };
      });
      return next;
    });
    Toast.info({ content: t('已用原价填充空白的拼团价') });
  };

  const submit = async () => {
    if (!groupon || !picked?.productId) {
      Toast.warning({ content: t('请选择商品') });
      return;
    }
    if (!picked.skus.length) {
      Toast.warning({ content: t('该商品没有可用规格，无法参加拼团') });
      return;
    }
    if (!joined.length) {
      Toast.warning({ content: t('请至少选择一个参加拼团的规格') });
      return;
    }
    if (tiers.some((tier) => Number(tier.num) < 2)) {
      Toast.warning({ content: t('成团人数不能小于 2') });
      return;
    }
    const missing = joined.some((sku) =>
      tiers.some((tier) => {
        const value = Number(skuState[sku.skuId]?.prices?.[tier.key]);
        return !Number.isFinite(value) || value <= 0;
      }),
    );
    if (missing) {
      Toast.warning({ content: t('请为每个规格填写拼团价') });
      return;
    }

    const payload: Record<string, any> = {
      grouponPromotionId: groupon.grouponPromotionId,
      productId: picked.productId,
      groupType,
      orderLimitNum: Number(orderLimitNum) || 0,
      isAutoGroup: isAutoGroup ? 1 : 0,
      minGroupNum: Math.min(...tiers.map((tier) => Number(tier.num))),
      minGroupPrice: null,
      sort: Number(sort) || 0,
      groupNums: tiers.map((tier) => ({ groupNumKey: tier.key, groupNum: Number(tier.num) })),
      grouponPromotionDetails: joined.map((sku) => {
        const state = skuState[sku.skuId];
        return {
          productId: picked.productId,
          skuId: sku.skuId,
          limitNum: Number(state?.limitNum) || 0,
          isJoin: 1,
          groupPromotionDetailPrices: tiers.map((tier) => ({
            groupNumKey: tier.key,
            price: Number(state?.prices?.[tier.key] ?? 0),
          })),
        };
      }),
    };
    if (isEdit) payload.grouponPromotionItemId = initial?.grouponPromotionItemId;

    onSavingChange(true);
    try {
      await apiPost(
        isEdit ? '/promotion/groupon/item/update' : '/promotion/groupon/item/create',
        payload,
      );
      Toast.success({ content: isEdit ? t('更新成功') : t('创建成功') });
      onSaved();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
      onSavingChange(false);
    }
  };

  return (
    <Modal
      className="gie-modal"
      visible={visible}
      title={null}
      header={null}
      footer={null}
      centered
      width={960}
      onCancel={onCancel}
      closeOnEsc
    >
      <div className="gie">
        <header className="gie-hero">
          <div className="gie-hero-main">
            <div className="gie-hero-eyebrow">
              <span className="gie-hero-dot" />
              {t('拼团活动')}
              <span className="gie-hero-sep" />
              {isEdit ? t('编辑拼团商品') : t('添加拼团商品')}
            </div>
            <h2 className="gie-hero-title">{groupon?.name || '—'}</h2>
            <div className="gie-hero-meta">
              <Tag size="small" color={isLadder ? 'purple' : 'blue'}>
                {isLadder ? t('阶梯团') : t('普通团')}
              </Tag>
              <span>
                {groupon?.startTime || '—'} <em>→</em> {groupon?.endTime || '—'}
              </span>
            </div>
          </div>
          <Button
            className="gie-close"
            theme="borderless"
            type="tertiary"
            icon={<span aria-hidden>×</span>}
            onClick={onCancel}
          />
        </header>

        <div className="gie-body">
          {picking ? (
            <>
              <div className="gie-search-bar">
                <Input
                  prefix={<IconSearch />}
                  placeholder={t('搜索商品名称 / 编号')}
                  value={keyword}
                  showClear
                  onChange={setKeyword}
                  onEnterPress={() => loadProducts(1, keyword)}
                />
                <Button theme="solid" type="primary" onClick={() => loadProducts(1, keyword)}>
                  {t('搜索')}
                </Button>
                <Text type="tertiary" size="small">
                  {t('共')} {total} {t('件可选')}
                </Text>
              </div>

              <div className="gie-picker">
                <Spin spinning={listLoading}>
                  {rows.length ? (
                    rows.map((row) => {
                      const product = row.product || {};
                      const reason = blockedReason(row);
                      const disabled = Boolean(reason);
                      return (
                        <button
                          type="button"
                          key={String(product.productId)}
                          className={`gie-pick-row${disabled ? ' is-disabled' : ''}`}
                          onClick={() => !disabled && choose(row)}
                          disabled={disabled}
                        >
                          <span className="gie-pick-thumb">
                            {product.picThumb || product.picUrl ? (
                              <img src={product.picThumb || product.picUrl} alt="" loading="lazy" />
                            ) : (
                              <i>{(product.productName || '?').slice(0, 1)}</i>
                            )}
                          </span>
                          <span className="gie-pick-main">
                            <span className="gie-pick-name">{product.productName}</span>
                            <span className="gie-pick-sub">
                              <span className="gie-sn">{product.productSn}</span>
                              <span className="gie-pick-price">{money(product.productPrice)}</span>
                              <span className="gie-pick-stock">
                                {t('库存')} {Number(product.productStock ?? 0).toLocaleString('zh-CN')}
                              </span>
                              <span className="gie-pick-sku">
                                {(row.productSkus || []).length
                                  ? `${(row.productSkus || []).length} ${t('个规格')}`
                                  : t('无规格')}
                              </span>
                            </span>
                          </span>
                          {reason ? (
                            <Tag size="small" color="grey">
                              {reason}
                            </Tag>
                          ) : (
                            <span className="gie-pick-go">
                              {t('选择')} <IconChevronRight size="small" />
                            </span>
                          )}
                        </button>
                      );
                    })
                  ) : (
                    <div className="gie-picker-empty">{listLoading ? '' : t('没有匹配的商品')}</div>
                  )}
                </Spin>
              </div>

              <div className="gie-pager">
                <Button
                  theme="borderless"
                  type="tertiary"
                  size="small"
                  icon={<IconChevronLeft />}
                  disabled={page <= 1 || listLoading}
                  onClick={() => loadProducts(page - 1, keyword)}
                >
                  {t('上一页')}
                </Button>
                <span className="gie-pager-info">
                  {page} / {Math.max(1, Math.ceil(total / PAGE_SIZE))}
                </span>
                <Button
                  theme="borderless"
                  type="tertiary"
                  size="small"
                  disabled={page >= Math.ceil(total / PAGE_SIZE) || listLoading}
                  onClick={() => loadProducts(page + 1, keyword)}
                >
                  {t('下一页')} <IconChevronRight size="small" />
                </Button>
              </div>
            </>
          ) : (
            <>
              {/* 已选商品摘要 */}
              <section className="gie-summary">
                <span className="gie-summary-thumb">
                  {picked?.picThumb ? <img src={picked.picThumb} alt="" /> : <i>{(picked?.productName || '?').slice(0, 1)}</i>}
                </span>
                <div className="gie-summary-main">
                  <div className="gie-summary-name">{picked?.productName}</div>
                  <div className="gie-summary-sub">
                    <span className="gie-sn">{picked?.productSn}</span>
                    <span>{t('原价')} {money(picked?.productPrice)}</span>
                    <span>{t('库存')} {Number(picked?.productStock ?? 0).toLocaleString('zh-CN')}</span>
                    <span>{picked?.skus.length || 0} {t('个规格')}</span>
                  </div>
                </div>
                {!isEdit ? (
                  <Button theme="borderless" type="tertiary" onClick={() => setPicking(true)}>
                    {t('更换商品')}
                  </Button>
                ) : null}
              </section>

              {picked && !picked.skus.length ? (
                <div className="gie-alert">
                  <IconAlertCircle />
                  <span>{t('该商品没有规格（SKU），无法参加拼团。请先在商品里生成规格。')}</span>
                </div>
              ) : null}

              {/* 成团人数 */}
              <section className="gie-card">
                <div className="gie-card-head">
                  <h3>{t('成团人数')}</h3>
                  <span className="gie-card-desc">
                    {isLadder ? t('阶梯团可设置多档人数，每档单独定价') : t('普通团只需设置一档人数')}
                  </span>
                </div>
                <div className="gie-tier-list">
                  {tiers.map((tier, index) => (
                    <div className="gie-tier" key={tier.key}>
                      <span className="gie-tier-label">
                        {isLadder ? `${t('第')} ${index + 1} ${t('档')}` : t('成团人数')}
                      </span>
                      <InputNumber
                        min={2}
                        precision={0}
                        value={tier.num}
                        style={{ width: 120 }}
                        onChange={(next) =>
                          setTiers((current) =>
                            current.map((item) =>
                              item.key === tier.key ? { ...item, num: Number(next ?? 2) } : item,
                            ),
                          )
                        }
                      />
                      <span className="gie-tier-unit">{t('人')}</span>
                      {isLadder && tiers.length > 1 ? (
                        <Button
                          theme="borderless"
                          type="danger"
                          size="small"
                          onClick={() => removeTier(tier.key)}
                        >
                          {t('删除')}
                        </Button>
                      ) : null}
                    </div>
                  ))}
                  {isLadder ? (
                    <Button size="small" icon={<IconPlus />} onClick={addTier}>
                      {t('添加阶梯')}
                    </Button>
                  ) : null}
                </div>
              </section>

              {/* 规格与拼团价（自绘表格：Semi Table 在受控输入列会触发无限更新循环） */}
              <section className="gie-card is-grow">
                <div className="gie-card-head">
                  <h3>{t('规格与拼团价')}</h3>
                  <div className="gie-card-tools">
                    <Text type="tertiary" size="small">
                      {t('已选')} {joined.length} / {picked?.skus.length || 0} {t('个规格')}
                    </Text>
                    <Button theme="borderless" size="small" onClick={fillWithOriginalPrice}>
                      {t('用原价填充')}
                    </Button>
                  </div>
                </div>
                {picked?.skus.length ? (
                  <div className="gie-grid" style={{ '--gie-price-cols': String(tiers.length) } as any}>
                    <div className="gie-grid-head">
                      <span>{t('参加')}</span>
                      <span>{t('规格')}</span>
                      <span>{t('原价')}</span>
                      {tiers.map((tier) => (
                        <span key={tier.key}>
                          {tiers.length > 1 ? `${t('拼团价')} · ${tier.num}${t('人')}` : t('拼团价')}
                        </span>
                      ))}
                      <span>{t('可售数量')}</span>
                    </div>
                    {picked.skus.map((sku) => {
                      const state = skuState[sku.skuId];
                      return (
                        <div className={`gie-grid-row${state?.isJoin ? '' : ' is-off'}`} key={sku.skuId}>
                          <span>
                            <Checkbox
                              checked={Boolean(state?.isJoin)}
                              onChange={(event) => patchSku(sku.skuId, { isJoin: event.target.checked })}
                            />
                          </span>
                          <span className="gie-sku-cell">
                            <span className="gie-sku-name">{sku.skuValue || t('默认规格')}</span>
                            {sku.skuSn ? <span className="gie-sn">{sku.skuSn}</span> : null}
                          </span>
                          <span className="gie-muted">{money(sku.skuPrice)}</span>
                          {tiers.map((tier) => (
                            <span key={tier.key}>
                              <InputNumber
                                className="gie-price-input"
                                size="small"
                                min={0}
                                precision={2}
                                placeholder={String(Number(sku.skuPrice ?? 0))}
                                disabled={!state?.isJoin}
                                value={state?.prices?.[tier.key] ?? ''}
                                onChange={(next) =>
                                  setPrice(
                                    sku.skuId,
                                    tier.key,
                                    next === '' || next === undefined || next === null ? '' : String(next),
                                  )
                                }
                              />
                            </span>
                          ))}
                          <span>
                            <InputNumber
                              size="small"
                              min={0}
                              precision={0}
                              disabled={!state?.isJoin}
                              value={state?.limitNum ?? 0}
                              onChange={(next) => patchSku(sku.skuId, { limitNum: Number(next ?? 0) })}
                            />
                          </span>
                        </div>
                      );
                    })}
                  </div>
                ) : (
                  <div className="gie-grid-empty">{t('该商品没有规格')}</div>
                )}
              </section>

              {/* 其他设置 */}
              <section className="gie-card">
                <div className="gie-card-head">
                  <h3>{t('其他设置')}</h3>
                </div>
                <div className="gie-row">
                  <div className="gie-field">
                    <span className="gie-field-label">{t('每单限购')}</span>
                    <InputNumber
                      min={0}
                      precision={0}
                      style={{ width: 120 }}
                      value={orderLimitNum}
                      onChange={(next) => setOrderLimitNum(Number(next ?? 0))}
                    />
                    <span className="gie-field-hint">{t('0 为不限购')}</span>
                  </div>
                  <div className="gie-field">
                    <span className="gie-field-label">{t('自动成团')}</span>
                    <Tooltip content={t('开启后，到期未满员的团由系统自动成团，避免买家组团失败')}>
                      <Switch checked={isAutoGroup} onChange={setIsAutoGroup} />
                    </Tooltip>
                  </div>
                  <div className="gie-field">
                    <span className="gie-field-label">{t('排序')}</span>
                    <InputNumber
                      precision={0}
                      style={{ width: 120 }}
                      value={sort}
                      onChange={(next) => setSort(Number(next ?? 0))}
                    />
                    <span className="gie-field-hint">{t('数字越小越靠前')}</span>
                  </div>
                </div>
              </section>
            </>
          )}
        </div>

        <footer className="gie-footer">
          <div className="gie-footer-hint">
            {picking ? (
              <>{t('点击商品即可进入配置')}</>
            ) : (
              <>
                <IconTick className="gie-ok" />
                {t('保存后会立即生效，买家可在拼团页参与')}
              </>
            )}
          </div>
          <div className="gie-footer-actions">
            <Button theme="borderless" type="tertiary" onClick={onCancel}>
              {t('取消')}
            </Button>
            {!picking ? (
              <Button theme="light" type="primary" onClick={submit} loading={saving}>
                {isEdit ? t('保存修改') : t('确认添加')}
              </Button>
            ) : null}
          </div>
        </footer>
      </div>
    </Modal>
  );
}
