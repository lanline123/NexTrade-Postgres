import Link from 'next/link';
import type { ShopListItem } from '@/lib/api';
import { getServerT } from '@/i18n/server';

/**
 * 店铺街行卡片：左侧店铺信息卡。
 * 源码对照：view/pc/pages/shop/src/ShopInfo.vue（<template> 类名逐条照抄）
 * 样式：app/shop-page.scss → .pg-shop-info
 *
 * - .shop-info-card（320px 定宽）> .shop-logo > a > img.logo-img
 * - .shop-content > .shop-name > a.name-link > span.ellipsis
 * -               > .shop-rating-row > .rating-box-container > .rating-box（.rating-score / .separator / .stars）/ 无评分时纯文本
 *                                   + span.fans-count（"N 粉丝"）
 * -               > .enter-shop-btn > a.btn-link（进入店铺）
 *
 * el-image / el-rate 是 Element Plus 组件，仓库里没有它们的默认样式；这里用旧版
 * :deep() 里已经写到的 .el-rate / .el-rate__item 类名做静态替身（同 fnd_23 的做法）。
 */

export function formatFansCount(count?: number): string {
  const n = Number(count || 0);
  if (n > 999999) return `${Math.floor(n / 100) / 100}w`;
  return String(n);
}

function ratingScore(score?: number): string {
  const n = Number(score || 0);
  return n ? n.toFixed(1) : '';
}

export default function ShopInfoCard({ item }: { item: ShopListItem }) {
  const t = getServerT();
  const star = Number(item.shopStar || 0);
  const href = `/shop/${item.shopId}`;

  return (
    <div className="shop-info-card">
      <div className="shop-logo">
        <Link href={href} target="_blank">
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img className="logo-img" src={item.shopLogo || ''} alt={item.shopTitle || ''} />
        </Link>
      </div>
      <div className="shop-content">
        <div className="shop-name" title={item.shopTitle || ''}>
          <Link href={href} target="_blank" className="name-link">
            <span className="ellipsis">{item.shopTitle}</span>
          </Link>
          {/* 店铺街推荐角标：置顶=「精选」（平台运营位）、付费推广=「推广」，主题色跟随 --general。
              shop-page.scss 为生成文件勿手改，故角标用内联样式（颜色全走主题变量）。 */}
          {item.isTop ? (
            <span
              style={{
                display: 'inline-block',
                verticalAlign: '2px',
                marginLeft: 8,
                padding: '1px 6px',
                borderRadius: 3,
                fontSize: 11,
                fontWeight: 'normal',
                lineHeight: '16px',
                color: 'var(--main-text)',
                background: 'var(--general)',
              }}
            >
              {t('精选')}
            </span>
          ) : null}
          {item.isPromoted ? (
            <span
              style={{
                display: 'inline-block',
                verticalAlign: '2px',
                marginLeft: item.isTop ? 6 : 8,
                padding: '0 5px',
                borderRadius: 3,
                border: '1px solid var(--general)',
                fontSize: 11,
                fontWeight: 'normal',
                lineHeight: '16px',
                color: 'var(--general)',
                background: 'var(--vice-bg)',
              }}
            >
              {t('推广')}
            </span>
          ) : null}
        </div>

        <div className="shop-rating-row">
          <div className="rating-box-container">
            {star ? (
              <div className="rating-box">
                <span className="rating-score">{ratingScore(star)}</span>
                <span className="separator">|</span>
                <div className="stars">
                  <div className="el-rate">
                    {[1, 2, 3, 4, 5].map((index) => (
                      <div className="el-rate__item" key={index}>
                        <i
                          className="el-rate__icon"
                          style={{
                            fontStyle: 'normal',
                            fontSize: 10,
                            lineHeight: '10px',
                            color:
                              index <= Math.round(star)
                                ? 'var(--el-rate-fill-color)'
                                : 'var(--el-rate-void-color)',
                          }}
                        >
                          ★
                        </i>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            ) : (
              <div>{t('暂无评分')}</div>
            )}
          </div>
          <span className="fans-count">
            {formatFansCount(item.collectCount)} {t('粉丝')}
          </span>
        </div>
        <div className="enter-shop-btn">
          <Link href={href} target="_blank" className="btn-link">
            {t('进入店铺')}
          </Link>
        </div>
      </div>
    </div>
  );
}
