import { useEffect, useState } from 'react';
import {
  Button,
  Checkbox,
  Input,
  Modal,
  Radio,
  RadioGroup,
  Select,
  Space,
  Tag,
  TextArea,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost, isDemo } from '../api';
import { useI18n } from '../i18n';

const { Text } = Typography;

/** 全部 3 产品 × 6 行为（Phase 2 起全量开放，拦截点已全部上线，见方案 §3.2/§8）。 */
export const BLACKLIST_RULE_OPTIONS: Array<{
  product: string;
  productLabel: string;
  behaviors: Array<{ behavior: string; label: string }>;
}> = [
  {
    product: 'mall',
    productLabel: '商城',
    behaviors: [
      { behavior: 'place_order', label: '下单' },
      { behavior: 'product_comment', label: '评价' },
    ],
  },
  {
    product: 'distribution',
    productLabel: '分销',
    behaviors: [
      { behavior: 'become_salesman', label: '自助成为分销员' },
      { behavior: 'commission_grant', label: '佣金发放' },
    ],
  },
  {
    product: 'buyer_show',
    productLabel: '买家秀',
    behaviors: [
      { behavior: 'buyer_show_publish', label: '发布' },
      { behavior: 'buyer_show_comment', label: '评论' },
    ],
  },
];

export interface BlacklistTargetUser {
  userId: number;
  username?: string;
  nickname?: string | null;
  mobile?: string;
}

interface ShopOption {
  shopId: number;
  shopTitle?: string | null;
}

/** 当前账号是否平台（adminType 为空/'admin' 即平台，与 api.ts getContextHeaders 同口径）。 */
export function isPlatformAccount(): boolean {
  const adminType = localStorage.getItem('adminType') || '';
  return !adminType || adminType === 'admin';
}

/**
 * 加入黑名单弹窗（黑名单列表页与会员列表共用）。
 * 单个模式：搜索框走 POST /user/user/search 精确查人（手机号/用户名）并回显，
 * 或由会员列表行操作直接传入 initialUser；批量模式：已选 N 人只读展示。
 * 组件定义在模块顶层（避免 React remount 吞输入，见 AGENTS.md 教训）。
 */
export default function BlacklistAddModal({
  visible,
  userIds,
  initialUser,
  source,
  onCancel,
  onSuccess,
}: {
  visible: boolean;
  /** 批量模式：已选会员 ID 列表（非空即批量）。 */
  userIds?: number[];
  /** 单个模式预置会员（会员列表行操作传入）。 */
  initialUser?: BlacklistTargetUser | null;
  /** 来源：1 会员列表批量 / 2 黑名单页批量 / 0 手动单个。 */
  source?: number;
  onCancel: () => void;
  onSuccess: () => void;
}) {
  const { t } = useI18n();
  const batch = Boolean(userIds && userIds.length);
  const platform = isPlatformAccount();
  const [keyword, setKeyword] = useState('');
  const [searching, setSearching] = useState(false);
  const [target, setTarget] = useState<BlacklistTargetUser | null>(null);
  const [selectedKeys, setSelectedKeys] = useState<string[]>(['mall:place_order']);
  const [applyMode, setApplyMode] = useState<number>(0);
  const [shopIds, setShopIds] = useState<number[]>([]);
  const [shops, setShops] = useState<ShopOption[]>([]);
  const [reason, setReason] = useState('');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!visible) return;
    setTarget(initialUser || null);
    setKeyword('');
    setSelectedKeys(['mall:place_order']);
    setApplyMode(0);
    setShopIds([]);
    setReason('');
    if (platform && shops.length === 0) {
      apiGet<{ records?: ShopOption[] }>('/user/shopMember/shopList', { size: 200 })
        .then((result) => setShops(result?.records || []))
        .catch(() => undefined);
    }
  }, [visible, initialUser, platform, shops.length]);

  const search = async () => {
    const kw = keyword.trim();
    if (!kw) {
      Toast.warning({ content: t('请输入手机号或用户名') });
      return;
    }
    setSearching(true);
    try {
      const result = await apiPost<BlacklistTargetUser | null>(
        `/user/user/search?search=${encodeURIComponent(kw)}`,
        {},
      );
      if (result && result.userId) {
        setTarget(result);
      } else {
        setTarget(null);
        Toast.warning({ content: t('未找到该会员') });
      }
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('搜索失败') });
    } finally {
      setSearching(false);
    }
  };

  const toggleRule = (key: string, checked: boolean) => {
    setSelectedKeys((current) =>
      checked ? [...current, key] : current.filter((item) => item !== key),
    );
  };

  const submit = async () => {
    if (isDemo()) {
      Toast.warning({ content: t('演示模式不支持操作') });
      return;
    }
    if (!batch && !target) {
      Toast.warning({ content: t('请先选择客户') });
      return;
    }
    if (!selectedKeys.length) {
      Toast.warning({ content: t('请至少选择一项限制规则') });
      return;
    }
    if (!reason.trim()) {
      Toast.warning({ content: t('限制原因不能为空') });
      return;
    }
    if (platform && applyMode === 1 && !shopIds.length) {
      Toast.warning({ content: t('请选择生效店铺') });
      return;
    }
    const rules = selectedKeys.map((key) => {
      const [product, behavior] = key.split(':');
      return { product, behavior };
    });
    setSaving(true);
    try {
      if (batch) {
        const result = await apiPost<{ successCount: number; mergedCount: number; failures: Array<{ msg: string }> }>(
          '/user/blacklist/batchAdd',
          {
            userIds,
            rules,
            applyMode: platform ? applyMode : undefined,
            shopIds: platform && applyMode === 1 ? shopIds : [],
            reason: reason.trim(),
            source: source ?? 2,
          },
        );
        const failCount = result?.failures?.length || 0;
        if (failCount > 0) {
          Toast.warning({
            content: t('成功 {ok} 人，失败 {fail} 人', {
              ok: result?.successCount ?? 0,
              fail: failCount,
            }),
          });
        } else {
          Toast.success({ content: t('加入黑名单成功') });
        }
      } else {
        const result = await apiPost<{ merged: boolean }>('/user/blacklist/add', {
          userId: target!.userId,
          rules,
          applyMode: platform ? applyMode : undefined,
          shopIds: platform && applyMode === 1 ? shopIds : [],
          reason: reason.trim(),
          source: source ?? 0,
        });
        Toast.success({
          content: result?.merged ? t('已合并到该客户现有的黑名单记录') : t('加入黑名单成功'),
        });
      }
      onSuccess();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加入黑名单失败') });
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      title={t('加入黑名单')}
      visible={visible}
      onCancel={onCancel}
      width={560}
      footer={
        <Space>
          <Button onClick={onCancel}>{t('取消')}</Button>
          <Button theme="solid" type="primary" loading={saving} onClick={submit}>
            {t('确认')}
          </Button>
        </Space>
      }
    >
      <div style={{ display: 'grid', gap: 16 }}>
        <div>
          <div style={{ marginBottom: 8 }}>{t('客户')}</div>
          {batch ? (
            <Text>{t('已选择 {count} 位客户', { count: userIds!.length })}</Text>
          ) : target ? (
            <Space>
              <Tag color="blue">{target.nickname || target.username || `ID ${target.userId}`}</Tag>
              <Text type="tertiary">{target.mobile || ''}</Text>
              <Text type="tertiary">ID {target.userId}</Text>
              {!initialUser && (
                <Button theme="borderless" size="small" onClick={() => setTarget(null)}>
                  {t('重新选择')}
                </Button>
              )}
            </Space>
          ) : (
            <Space>
              <Input
                placeholder={t('输入手机号或用户名精确搜索')}
                value={keyword}
                onChange={setKeyword}
                onEnterPress={search}
                style={{ width: 300 }}
              />
              <Button loading={searching} onClick={search}>
                {t('搜索')}
              </Button>
            </Space>
          )}
        </div>

        <div>
          <div style={{ marginBottom: 8 }}>{t('限制规则')}</div>
          <div style={{ display: 'grid', gap: 8 }}>
            {BLACKLIST_RULE_OPTIONS.map((group) => (
              <div key={group.product} style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                <Text strong style={{ width: 48 }}>
                  {t(group.productLabel)}
                </Text>
                <Space>
                  {group.behaviors.map((item) => {
                    const key = `${group.product}:${item.behavior}`;
                    return (
                      <Checkbox
                        key={key}
                        checked={selectedKeys.includes(key)}
                        onChange={(event) => toggleRule(key, Boolean(event.target.checked))}
                      >
                        {t(item.label)}
                      </Checkbox>
                    );
                  })}
                </Space>
              </div>
            ))}
          </div>
          <Text type="tertiary" size="small">
            {t('限制按「产品 × 行为」生效，不影响客户登录、浏览等其他功能。')}
          </Text>
        </div>

        {platform && (
          <div>
            <div style={{ marginBottom: 8 }}>{t('限制范围')}</div>
            <RadioGroup
              type="button"
              value={applyMode}
              onChange={(event) => setApplyMode(Number(event.target.value))}
            >
              <Radio value={0}>{t('全平台')}</Radio>
              <Radio value={1}>{t('指定店铺')}</Radio>
            </RadioGroup>
            {applyMode === 1 && (
              <Select
                multiple
                placeholder={t('请选择生效店铺')}
                value={shopIds}
                onChange={(next) => setShopIds((next as Array<string | number>).map(Number))}
                optionList={shops.map((shop) => ({
                  label: shop.shopTitle || `店铺 ${shop.shopId}`,
                  value: shop.shopId,
                }))}
                style={{ width: 360, marginTop: 8 }}
              />
            )}
          </div>
        )}

        <div>
          <div style={{ marginBottom: 8 }}>
            {t('限制原因')} <Text type="danger">*</Text>
          </div>
          <TextArea
            placeholder={t('必填，仅后台可见，客户不会看到')}
            value={reason}
            onChange={setReason}
            maxCount={500}
            rows={3}
          />
        </div>
      </div>
    </Modal>
  );
}
