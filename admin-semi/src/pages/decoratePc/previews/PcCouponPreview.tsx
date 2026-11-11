/**
 * PC 优惠券模块预览（2026-11-11 重做）。
 *
 * 【本次重写的三个原因】
 *  1. 旧实现是纯 mock：固定 4 张假券（满60减5 ×2 / 8折 ×2），而后台真实数据只有 1 张
 *     「万物新生 · 满50000减1000」。运营在后台看到 4 张、前台实际只有 1 张。
 *  2. 旧 DOM 用 `pc-coupon__*` 类名，PC 前台用 `.mod_coupon/.item/.use_btn`，
 *     两端各写一套 CSS，必然漂移（后台卡片 245.98px 而容器仅 821px，第 4 张被裁掉一半）。
 *  3. 旧版按 106px 固定高度 + 246px 固定宽度排版，券少时右侧大片空白。
 *
 * 【现在的做法】
 *  - DOM 与 pc-semi 的 Coupon 组件逐层同构，类名统一 `dec-coupon__*`。
 *  - 样式不在本目录另写，直接 import `pcCoupon.generated.css`
 *    （由 admin-semi/scripts/gen-coupon-css.mjs 从
 *     pc-semi/src/decorate/coupon/coupon.scss 生成，单一数据源）。
 *  - 数据走真实接口：dev 环境 /adminapi 有代理，取后台券列表并套用与 C 端
 *    getCoupon 完全相同的可见性规则，所以「后台预览 == 前台看到的券」。
 */
import './pcCoupon.generated.css';

import { useEffect, useState } from 'react';
import { apiGet } from '../../../api';
import { useI18n } from '../../../i18n';

/** 旧版 `pcCouponDefaults`（`_shared/coupon/defaults.ts`）。 */
const COUPON_DEFAULTS = {
  showTitle: 0,
  title: '每日领券',
  desc: '天天优惠领不停',
  colorStyle: 0,
  itemBackgroundColor: '#ffffff',
  backgroundColor: '#ffffff',
  itemPadding: 10,
  boxRadius: 8,
  boxPadding: 10,
  boxPaddingTop: 5,
  boxPaddingBottom: 5,
};

type CouponConfig = typeof COUPON_DEFAULTS;

interface CouponRow {
  couponId?: number;
  couponName?: string;
  couponDesc?: string;
  couponType?: number;
  couponMoney?: string | number;
  couponDiscount?: string | number;
  minOrderAmount?: string | number;
  isShow?: number;
  enabledClickGet?: number;
  isDelete?: number;
  sendType?: number;
  useEndDate?: number;
  receiveNum?: number;
  sendNum?: number;
}

/** 简单深合并：数组直接覆盖，对象递归合并。 */
function deepMerge<T extends Record<string, any>>(defaults: T, data?: Record<string, any>): T {
  const out: Record<string, any> = { ...defaults };
  Object.entries(data || {}).forEach(([key, value]) => {
    const base = out[key];
    if (
      value &&
      typeof value === 'object' &&
      !Array.isArray(value) &&
      base &&
      typeof base === 'object' &&
      !Array.isArray(base)
    ) {
      out[key] = deepMerge(base as Record<string, any>, value as Record<string, any>);
    } else if (value !== undefined) {
      out[key] = value;
    }
  });
  return out as T;
}

/**
 * 复刻 C 端可见性：后端 `paginate_claimable` 的 WHERE 只有这四项
 * （rust/crates/infrastructure/src/db/repo/marketing/coupon.rs）：
 *   is_delete = 0 AND is_show = 1 AND enabled_click_get = 1
 *   AND (send_type = 0 OR use_end_date >= now)
 * 不看 send_num / limit_num / receive_num / use_start_date。
 */
function isVisibleOnPc(c: CouponRow): boolean {
  const now = Math.floor(Date.now() / 1000);
  if (Number(c.isDelete ?? 0) !== 0) return false;
  if (Number(c.isShow ?? 0) !== 1) return false;
  if (Number(c.enabledClickGet ?? 0) !== 1) return false;
  if (Number(c.sendType ?? 0) === 0) return true;
  return Number(c.useEndDate ?? 0) >= now;
}

/** 前台 `getCoupon` 按 add_time DESC 返回，模块再取前 4 张。 */
const MAX_SHOWN = 4;

export default function PcCouponPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const moduleConfig = deepMerge<CouponConfig>(COUPON_DEFAULTS, data);
  const isEmpty = !data || Object.keys(data).length === 0;

  const [rows, setRows] = useState<CouponRow[]>([]);
  const [source, setSource] = useState<'loading' | 'live' | 'empty' | 'error'>('loading');

  useEffect(() => {
    let alive = true;
    setSource('loading');
    apiGet<{ records?: CouponRow[]; list?: CouponRow[] }>('/promotion/coupon/list', {
      page: 1,
      size: 100,
    })
      .then((res) => {
        if (!alive) return;
        const all = res?.records || res?.list || [];
        // 后台列表默认按 add_time DESC，与 C 端 getCoupon 同序；再套前台可见性规则
        const visible = all.filter(isVisibleOnPc).slice(0, MAX_SHOWN);
        setRows(visible);
        setSource(visible.length ? 'live' : 'empty');
      })
      .catch(() => {
        if (!alive) return;
        setRows([]);
        setSource('error');
      });
    return () => {
      alive = false;
    };
  }, []);

  const itemPadding = Number(moduleConfig.itemPadding ?? 0) || 0;
  const boxPadding = Number(moduleConfig.boxPadding ?? 10);
  const boxPaddingTop = Number(moduleConfig.boxPaddingTop ?? 5);
  const boxPaddingBottom = Number(moduleConfig.boxPaddingBottom ?? 5);
  const boxRadius = Number(moduleConfig.boxRadius ?? 8) || 0;
  const showTitle = Number(moduleConfig.showTitle) === 1;

  // 模块配置为空时沿用旧版空态占位
  if (isEmpty) {
    return (
      <div className="dec-coupon" style={{ padding: itemPadding }}>
        <div className="dec-coupon__empty">
          <div className="dec-coupon__empty-icon" />
          <div>{t('优惠券')}</div>
        </div>
      </div>
    );
  }

  const cols = Math.max(1, Math.min(rows.length || 1, MAX_SHOWN));

  return (
    <div className="dec-coupon" style={{ padding: itemPadding }}>
      <div
        className="dec-coupon"
        style={{
          paddingLeft: boxPadding,
          paddingRight: boxPadding,
          paddingTop: boxPaddingTop,
          paddingBottom: boxPaddingBottom,
          borderRadius: boxRadius,
          background: moduleConfig.backgroundColor || undefined,
          overflow: 'hidden',
          ['--dec-coupon-cols' as any]: cols,
        }}
      >
        {showTitle && (
          <div className="dec-coupon__header">
            <p className="dec-coupon__header-title">{moduleConfig.title}</p>
            {moduleConfig.desc ? <span className="dec-coupon__header-desc">{moduleConfig.desc}</span> : null}
          </div>
        )}

        <div className="dec-coupon__main">
          <div className="dec-coupon__stub" aria-hidden>
            <p className="dec-coupon__stub-zh">{t('领券中心')}</p>
            <p className="dec-coupon__stub-en">COUPON</p>
            <p className="dec-coupon__stub-action">
              GO <em className="dec-coupon__action-arrow">&gt;&gt;</em>
            </p>
          </div>

          {source === 'loading' ? (
            <div className="dec-coupon__list">
              {Array.from({ length: cols }).map((_, i) => (
                <div key={i} className="dec-coupon__item" style={{ opacity: 0.45 }}>
                  <div className="dec-coupon__amount">
                    <span className="dec-coupon__amount-currency">¥</span>
                    <span className="dec-coupon__amount-value">--</span>
                  </div>
                  <div className="dec-coupon__info">
                    <p className="dec-coupon__name">{t('加载中')}</p>
                  </div>
                  <span className="dec-coupon__action is-disabled">{t('领取')}</span>
                </div>
              ))}
            </div>
          ) : rows.length ? (
            <div className="dec-coupon__list">
              {rows.map((c, index) => {
                const isDiscount = Number(c.couponType) === 2;
                const soldOut = Number(c.sendNum ?? 0) > 0 && Number(c.receiveNum ?? 0) >= Number(c.sendNum);
                return (
                  <div className="dec-coupon__item" key={c.couponId ?? index}>
                    <div className="dec-coupon__amount">
                      {isDiscount ? (
                        <span className="dec-coupon__discount">
                          <span className="dec-coupon__discount-value">{c.couponDiscount}</span>
                          <span className="dec-coupon__discount-unit">{t('折')}</span>
                        </span>
                      ) : (
                        <>
                          <span className="dec-coupon__amount-currency">¥</span>
                          <span className="dec-coupon__amount-value">
                            {Number(c.couponMoney || 0).toFixed(0)}
                          </span>
                        </>
                      )}
                    </div>
                    <div className="dec-coupon__info">
                      <p className="dec-coupon__name">{c.couponName}</p>
                      {c.couponDesc ? <p className="dec-coupon__desc">{c.couponDesc}</p> : null}
                    </div>
                    <span className={`dec-coupon__action${soldOut ? ' is-disabled' : ''}`}>
                      {soldOut ? t('已领取') : t('马上领')}
                      <em className="dec-coupon__action-arrow">&gt;</em>
                    </span>
                  </div>
                );
              })}
            </div>
          ) : (
            <div className="dec-coupon__list">
              <div className="dec-coupon__empty" style={{ gridColumn: '1 / -1' }}>
                <div className="dec-coupon__empty-icon" />
                <div>
                  {source === 'error'
                    ? t('券数据加载失败，请检查网络后重试')
                    : t('当前没有可在前台领取的优惠券（需开启「显示」与「可点击领取」且在有效期内）')}
                </div>
              </div>
            </div>
          )}

          <span className="dec-coupon__more">{t('查看更多')}</span>
        </div>
      </div>
    </div>
  );
}