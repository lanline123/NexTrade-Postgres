import { useEffect, useState } from 'react';
import { apiGet } from '../api';

interface PriceInquiryListResponse {
  records?: unknown[];
  total?: number;
}

/**
 * 商品询价未回复计数（供左侧菜单「商品 / 询价管理 / 商品询价」红标共用）。
 *
 * 计数口径与商品询价页面（pages/Enquiries）一致：未回复 = `status=0`
 * （1=已回复）。接口 `GET /product/priceInquiry/list` 现成、无需改后端；
 * 该接口按当前组织（平台/店铺，x-admin-type/x-shop-id）自动过滤，
 * 与询价列表页看到的范围一致。这里只取 size=1 读 `total`，不拉整页数据。
 *
 * 行为与 usePendingApprovalCount 同构：挂载立即拉一次，之后每 60 秒轮询；
 * refreshKey 变化（如路由切换）时立即再拉一次。拉取失败静默处理——
 * 保留上一次的计数（首次失败即 0，红标不显示），绝不向上抛错影响后台主流程。
 * 本 hook 只在登录后的 AdminLayout 内使用，登录页不会请求。
 */
export function useUnrepliedInquiryCount(refreshKey?: unknown): number {
  const [count, setCount] = useState(0);

  useEffect(() => {
    let alive = true;
    const refresh = () => {
      void apiGet<PriceInquiryListResponse>('/product/priceInquiry/list', { status: 0, size: 1 })
        .then((res) => {
          if (!alive) return;
          const total =
            typeof res?.total === 'number'
              ? res.total
              : Array.isArray(res?.records)
                ? res.records.length
                : 0;
          setCount(Math.max(0, total));
        })
        .catch(() => {
          // 静默：询价接口异常时不动红标
        });
    };
    refresh();
    const timer = window.setInterval(refresh, 60000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [refreshKey]);

  return count;
}
