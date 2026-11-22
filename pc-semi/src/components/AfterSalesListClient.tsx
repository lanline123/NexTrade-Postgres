'use client';

import { useEffect, useState } from 'react';
import Link from 'next/link';
import { Button, Card } from '@douyinfe/semi-ui';
import { apiGet } from '@/lib/client-api';

import { useTranslation } from 'react-i18next';
interface Item {
  productName?: string;
  picThumb?: string;
  productSn?: string;
  quantity?: number;
}

interface Record {
  aftersaleId: number;
  aftersalesSn?: string;
  aftersalesTypeName?: string;
  statusName?: string;
  status?: number;
  addTime?: string;
  refundAmount?: string;
  orderSn?: string;
  orderId?: number;
  items?: Item[];
  aftersalesItems?: Item[];
}

const PAGE_SIZE = 20;

export default function AfterSalesListClient() {
    const { t } = useTranslation();
  const [rows, setRows] = useState<Record[]>([]);
  const [loading, setLoading] = useState(true);
  const [needLogin, setNeedLogin] = useState(false);
  // 分页与错误态（2026-11-22 体检补）：此前写死第 1 页 20 条、第 21 条起不可达；
  // 且非登录类失败时 rows 保持空，被误呈现为「暂无已申请退换货订单」
  const [page, setPage] = useState(1);
  const [total, setTotal] = useState(0);
  const [error, setError] = useState('');
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    setLoading(true);
    setError('');
    apiGet<{ records?: Record[]; total?: number }>('user/aftersales/getRecord', { page, size: PAGE_SIZE })
      .then((result) => {
        setRows(result.records || []);
        setTotal(Number(result.total || 0));
      })
      .catch((caught) => {
        const message = caught instanceof Error ? caught.message : '';
        if (message.includes('登录') || message.includes('401')) {
          setNeedLogin(true);
        } else {
          setError(message || t('售后列表加载失败'));
        }
      })
      .finally(() => setLoading(false));
  }, [page, t, reloadKey]);

  const pages = Math.max(1, Math.ceil(total / PAGE_SIZE));

  if (needLogin) {
    return (
      <Card className="cart-state">
        <h2>{t('登录后查看售后')}</h2>
        <Link href="/login?redirect=/member/aftersales"><Button theme="solid" type="primary" size="large">{t('去登录')}</Button></Link>
      </Card>
    );
  }

  /**
   * 对照旧版 `pages/member/return/List.vue`：
   * `.info-row > .title-or-tabs > .tag-and-input > .tig-tabs` + `.return-content > table.custom-table`。
   * 旧版这一页列的是「订单编号 / 订单商品 / 发货时间」（按订单聚合），新版接口返回的是售后单，
   * 所以列名沿用同一套表头样式，内容换成「售后单号 / 订单商品 / 申请时间」。
   */
  return (
    <div className="info-row mp-return-list">
      <div className="title-or-tabs">
        <div className="tag-and-input">
          <div className="tig-tabs">
            <div className="tab active">
              <p>
                <span>{t('售后列表')}</span>
              </p>
            </div>
          </div>
        </div>
      </div>
      <div className="return-content">
        {error ? (
          <div className="no-result">
            <div>
              <span>{error}</span>{' '}
              <Button onClick={() => setReloadKey((key) => key + 1)}>{t('重试')}</Button>
            </div>
          </div>
        ) : rows.length ? (
          <>
          <table className="custom-table">
            <thead className="table-header">
              <tr>
                <th className="header-cell header-cell-first" style={{ width: 200 }}>{t('售后单号')}</th>
                <th className="header-cell">{t('订单商品')}</th>
                {/* 类型/状态/退款金额接口都有下发，此前表格不展示，用户看不出处理进度（2026-11-22 体检补） */}
                <th className="header-cell" style={{ width: 110 }}>{t('售后类型')}</th>
                <th className="header-cell" style={{ width: 110 }}>{t('状态')}</th>
                <th className="header-cell" style={{ width: 110 }}>{t('退款金额')}</th>
                <th className="header-cell" style={{ width: 140 }}>{t('申请时间')}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((record) => (
                <tr key={record.aftersaleId}>
                  <td>{record.aftersalesSn || '-'}</td>
                  <td>
                    <div className="product-list">
                      <div>
                        <img className="img" src={(record.aftersalesItems || record.items || [])[0]?.picThumb || ''} alt="" />
                        <Link href={`/member/aftersales/detail?id=${record.aftersaleId}`}>
                          <Button type="primary" theme="solid">{t('详情')}</Button>
                        </Link>
                      </div>
                    </div>
                  </td>
                  <td>{record.aftersalesTypeName || '-'}</td>
                  <td>{record.statusName || '-'}</td>
                  <td>{record.refundAmount ? `¥${record.refundAmount}` : '-'}</td>
                  <td>{record.addTime || '-'}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {pages > 1 ? (
            <div className="pc-pagination" style={{ marginTop: 16, textAlign: 'right' }}>
              <Button disabled={page <= 1 || loading} onClick={() => setPage(page - 1)}>{t('上一页')}</Button>{' '}
              <span>{page} / {pages}</span>{' '}
              <Button disabled={page >= pages || loading} onClick={() => setPage(page + 1)}>{t('下一页')}</Button>
            </div>
          ) : null}
          </>
        ) : (
          <div className="no-result">
            <div>
              <span>{loading ? '' : t('暂无已申请退换货订单')}</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
