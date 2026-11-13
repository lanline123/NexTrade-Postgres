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
 *  - 数据走 `GET /adminapi/promotion/coupon/claimableList?page=1&size=20`
 *    （2026-11-11 起）：该接口在 backend 直接复用与 C 端
 *    `GET /api/home/home/getCoupon` 相同的 `CouponService::claimable_list`，
 *    同样强制 is_show=1 / valid_date=1 / receive_flag=1，返回 `CouponClientVO`
 *    （camelCase，逐字段与 getCoupon 相同；`data` 直接是数组，无 records 包装）。
 *    可见性判定因此**只由后端负责**，本文件不再复刻任何过滤规则
 *    （旧的可见性判定函数已整体删除），从根上消除「后台看到 A、前台显示 B」的漂移。
 *    这是游客口径（isReceive 恒 0），与未登录访客在首页所见一致。
 *  - 接口失败时展示错误态，不回退 mock 假券；后端未部署时该接口 404/400 属预期。
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
  titleFontSize: 18,
  descFontSize: 12,
  titleAlign: 1,
  descLayout: 1,
  titleColor: '',
  accentColor: '#16a34a',
  itemBackgroundColor: '#ffffff',
  backgroundColor: '#ffffff',
  itemPadding: 10,
  boxRadius: 0,
  boxPadding: 10,
  boxPaddingTop: 5,
  boxPaddingBottom: 5,
};

type CouponConfig = typeof COUPON_DEFAULTS;

/**
 * `CouponClientVO`（claimableList / getCoupon 同源返回体）。
 * 这里只声明预览渲染实际用到的字段；可见性相关字段（is_show / 有效期 / 可点击领取）
 * 由后端在 claimable_list 内过滤，前端不再读取、不再判定。
 */
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
 * 前台模块同样只取前 4 张 —— 这是模块展示上限（语义），不是可见性过滤规则。
 * 过滤发生在后端 claimable_list 内，前端只做 slice 截断。
 */
const MAX_SHOWN = 4;

export default function PcCouponPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const moduleConfig = deepMerge<CouponConfig>(COUPON_DEFAULTS, data);
  const isEmpty = !data || Object.keys(data).length === 0;

  const [rows, setRows] = useState<CouponClientVO[]>([]);
  const [source, setSource] = useState<'loading' | 'live' | 'empty' | 'error'>('loading');

  useEffect(() => {
    let alive = true;
    setSource('loading');
    // 与 C 端 getCoupon 同源的「可领取券」接口，可见性由后端过滤，前端不再复刻
    apiGet<CouponClientVO[]>('/promotion/coupon/claimableList', {
      page: 1,
      size: 20,
    })
      .then((res) => {
        if (!alive) return;
        // 契约：data 直接是 CouponClientVO[]（无 records 包装）；对包装体做一次兜底
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

  const itemPadding = Number(moduleConfig.itemPadding ?? 0) || 0;
  const boxPadding = Number(moduleConfig.boxPadding ?? 10);
  const boxPaddingTop = Number(moduleConfig.boxPaddingTop ?? 5);
  const boxPaddingBottom = Number(moduleConfig.boxPaddingBottom ?? 5);
  const boxRadius = Number(moduleConfig.boxRadius ?? 8) || 0;
  // 标题契约归一（白屏修复）：编辑器侧 title 已是 standardTitle 对象
  // （{ showTitle, titleText, descText, titleFontSize, ... }，见 PcDecorateEditor 的
  // defaults 与 normalizeLegacyTitle）；老数据才是平铺字段 + 字符串 title。
  // 此前下方直接渲染 {moduleConfig.title}，title 为对象时触发 React #31 并整页白屏。
  // 渲染只用这里归一出的标量：契约值优先、平铺值兜底，老数据行为不变。
  const titleObj = moduleConfig.title && typeof moduleConfig.title === 'object'
    ? (moduleConfig.title as unknown as Record<string, any>)
    : null;
  const headerTitle = titleObj ? String(titleObj.titleText || '') : String(moduleConfig.title || '');
  const headerDesc = titleObj ? String(titleObj.descText || '') : String(moduleConfig.desc || '');
  const headerTitleColor = (titleObj?.titleColor as string) || moduleConfig.titleColor || '';
  const showTitle = titleObj ? Number(titleObj.showTitle) === 1 : Number(moduleConfig.showTitle) === 1;
  // 共享标题契约：字号 / 对齐 / 副标题排版（默认 18px、左对齐、跟随主标题后，老数据零跳变）
  const titleFontSize = Number(titleObj?.titleFontSize ?? moduleConfig.titleFontSize) || 18;
  const descFontSize = Number(titleObj?.descFontSize ?? moduleConfig.descFontSize) || 12;
  const titleAlign = Number(titleObj?.titleAlign ?? moduleConfig.titleAlign) || 1;
  const descBelow = Number(titleObj?.descLayout ?? moduleConfig.descLayout ?? 1) === 2;
  const headerClass = [
    'dec-coupon__header',
    titleAlign !== 1 ? `is-align-${titleAlign}` : '',
    descBelow ? 'is-desc-below' : '',
  ].filter(Boolean).join(' ');
  // 标题边距（共享契约口径，与 PC 端 DecModuleTitle 对齐）：左右未配置默认 10、始终内联，
  // 否则会被共享 CSS .dec-coupon__header 的固定 padding 盖住；上下仅在显式配置且 >0 时覆盖。
  // 此前标题栏完全没读边距字段，设置里的标题边距滑杆不生效。
  const flatTitle = moduleConfig as unknown as Record<string, any>;
  const rawPadL = titleObj?.titlePaddingLeft ?? flatTitle.titlePaddingLeft;
  const rawPadR = titleObj?.titlePaddingRight ?? flatTitle.titlePaddingRight;
  const headerPadL = rawPadL == null || rawPadL === '' ? 10 : Number(rawPadL);
  const headerPadR = rawPadR == null || rawPadR === '' ? 10 : Number(rawPadR);
  const headerStyle: Record<string, any> = {
    paddingLeft: `${Number.isFinite(headerPadL) ? headerPadL : 10}px`,
    paddingRight: `${Number.isFinite(headerPadR) ? headerPadR : 10}px`,
  };
  const rawPadT = titleObj?.titlePaddingTop ?? flatTitle.titlePaddingTop;
  const rawPadB = titleObj?.titlePaddingBottom ?? flatTitle.titlePaddingBottom;
  if (rawPadT != null && rawPadT !== '' && Number(rawPadT) > 0) headerStyle.paddingTop = `${Number(rawPadT)}px`;
  if (rawPadB != null && rawPadB !== '' && Number(rawPadB) > 0) headerStyle.paddingBottom = `${Number(rawPadB)}px`;

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
          background: (moduleConfig as any).frame?.backgroundColor || moduleConfig.backgroundColor || undefined,
          overflow: 'hidden',
          ['--dec-coupon-cols' as any]: cols,
          ['--dec-coupon-accent' as any]: moduleConfig.accentColor || undefined,
          ['--dec-coupon-surface' as any]: moduleConfig.itemBackgroundColor || undefined,
          ['--dec-coupon-radius' as any]: `${boxRadius}px`,
        }}
      >
        {showTitle && (
          <div className={headerClass} style={headerStyle}>
            <p
              className="dec-coupon__header-title"
              style={{ color: headerTitleColor || undefined, fontSize: `${titleFontSize}px` }}
            >
              {headerTitle}
            </p>
            {headerDesc ? (
              <span className="dec-coupon__header-desc" style={{ fontSize: `${descFontSize}px` }}>
                {headerDesc}
              </span>
            ) : null}
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