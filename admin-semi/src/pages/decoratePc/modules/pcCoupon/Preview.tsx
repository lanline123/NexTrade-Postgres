/**
 * PC 旧版优惠券模块（pcCoupon）实时预览。
 *
 * 前台渲染（权威）：pc-semi `decorate/components/modules.tsx` 的 `Coupon` 组件
 * （2059 行起），经 `contract.ts` 的 `adaptPcCoupon`（222 行起）适配：
 *  - pcCoupon 与新版 pc-home-coupon 在前台共用同一个 Coupon 组件、同一套
 *    `.dec-coupon__*` DOM 与 coupon.scss，因此本预览的 DOM/样式与新版预览同源
 *    （样式 import 新版目录的 `pcCoupon.generated.css`，单一数据源，不另写一份）。
 *  - 取数口径与新版预览一致：`GET /promotion/coupon/claimableList?page=1&size=20`
 *    （与 C 端 `home/home/getCoupon` 同源，is_show/有效期/可领取由后端过滤），
 *    展示上限 4 张与前台 `slice(0, 4)` 一致。
 *
 * 与新版预览的两点差异（均为旧版数据结构决定，非笔误）：
 *  1. 旧版 def 无任何 settings、defaults 为 `{}`，不存在「配置为空」一说——
 *     前台对空配置照常渲染券列表，故本预览不沿用新版的 isEmpty 占位早退，
 *     挂载即取数；无券/失败时展示与新版同款的空态文案。
 *  2. 标题不渲染：前台 adapt 把 title 压成字符串（`toTitle(m.title).titleText`），
 *     而 DecModuleTitle 只认对象上的 `showTitle === 1`，字符串 title 恒返回 null，
 *     即旧版优惠券在前台从不出现标题栏。本预览忠实镜像该行为，不挂标题组件。
 *
 * 容器口径对齐前台 Coupon 的 Container 入参：只读 `frame`
 * （boxPadding / boxPaddingTop / boxPaddingBottom / backgroundColor / boxRadius，
 * 缺省 0/0/0/透明/8），外层 padding 取顶层 itemPadding（旧版数据一般无此字段，缺省 0）。
 */
import '../pc-home-coupon/pcCoupon.generated.css';

import { useEffect, useState } from 'react';
import { apiGet } from '../../../../api';
import { useI18n } from '../../../../i18n';

/** 旧版 pcCoupon 预览默认值：仅 frame 与前台 Coupon 的缺省口径，其余字段只作兜底读取。 */
const COUPON_DEFAULTS = {
  itemPadding: 0,
  accentColor: '',
  itemBackgroundColor: '',
  frame: {
    boxPadding: 0,
    boxPaddingTop: 0,
    boxPaddingBottom: 0,
    backgroundColor: 'transparent',
    boxRadius: 8,
  },
};

type CouponConfig = typeof COUPON_DEFAULTS;

/** `CouponClientVO`（claimableList / getCoupon 同源返回体），字段口径同新版预览。 */
interface CouponClientVO {
  couponId?: number;
  couponName?: string;
  couponDesc?: string;
  couponType?: number;
  couponMoney?: string | number;
  couponDiscount?: string | number;
  minOrderAmount?: string | number;
  receiveNum?: number;
  sendNum?: number;
}

/** 简单深合并：数组直接覆盖，对象递归合并（同新版预览）。 */
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

/** 前台展示上限 4 张（模块展示语义，非可见性过滤；过滤在后端 claimable_list 内）。 */
const MAX_SHOWN = 4;

export default function PcCouponLegacyPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const config = deepMerge<CouponConfig>(COUPON_DEFAULTS, data);
  const frame = (config.frame || {}) as Record<string, any>;

  const [rows, setRows] = useState<CouponClientVO[]>([]);
  const [source, setSource] = useState<'loading' | 'live' | 'empty' | 'error'>('loading');

  useEffect(() => {
    let alive = true;
    setSource('loading');
    apiGet<CouponClientVO[]>('/promotion/coupon/claimableList', {
      page: 1,
      size: 20,
    })
      .then((res) => {
        if (!alive) return;
        const all = Array.isArray(res) ? res : ((res as any)?.records ?? []);
        const visible = all.slice(0, MAX_SHOWN);
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

  const itemPadding = Number(config.itemPadding ?? 0) || 0;
  const boxPadding = Number(frame.boxPadding ?? 0) || 0;
  const boxPaddingTop = Number(frame.boxPaddingTop ?? 0) || 0;
  const boxPaddingBottom = Number(frame.boxPaddingBottom ?? 0) || 0;
  const boxRadius = Number(frame.boxRadius ?? 8) || 0;
  const cols = Math.max(1, Math.min(rows.length || 1, MAX_SHOWN));

  return (
    <div style={{ padding: itemPadding }}>
      <div
        className="dec-coupon"
        style={{
          paddingLeft: boxPadding,
          paddingRight: boxPadding,
          paddingTop: boxPaddingTop,
          paddingBottom: boxPaddingBottom,
          borderRadius: boxRadius,
          background: frame.backgroundColor || undefined,
          overflow: 'hidden',
          ['--dec-coupon-cols' as any]: cols,
          ['--dec-coupon-accent' as any]: config.accentColor || undefined,
          ['--dec-coupon-surface' as any]: config.itemBackgroundColor || undefined,
          ['--dec-coupon-radius' as any]: `${boxRadius}px`,
        }}
      >
        {/* 旧版前台不渲染标题栏（见文件头注释第 2 点），此处直接进入券列表主体，
            DOM 与前台 Coupon / 新版预览逐层同构。 */}
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
