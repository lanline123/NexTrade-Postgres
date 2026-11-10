import { useEffect, useState } from 'react';
import { api } from '../api';
import { toast } from '../toast';

interface OrderRow {
  orderId: number;
  orderSn: string;
  consignee: string;
  mobile: string;
  totalAmount: string | number;
  paidAmount: string | number;
  orderStatusName: string;
  payStatus: number;
  addTime: string;
  itemCount: number;
  orderSource?: string;
}

interface OrderPage {
  total: number;
  list: OrderRow[];
  page: number;
  pageSize: number;
  totalPage: number;
}

const PAGE_SIZE = 15;

const PAY_STATUS: Record<number, string> = {
  1: '未支付',
  2: '已支付',
};

export default function Orders() {
  const [mode, setMode] = useState<1 | 2>(1);
  const [rows, setRows] = useState<OrderRow[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);

  const load = async (nextPage: number, nextMode: 1 | 2) => {
    setLoading(true);
    try {
      const result = await api.get<OrderPage>('/order/order/list', {
        page: nextPage,
        size: PAGE_SIZE,
        order_source: nextMode === 1 ? 'pos' : undefined,
      });
      setRows(result.list || []);
      setTotal(result.total || 0);
      setPage(result.page || nextPage);
    } catch (error) {
      toast(error instanceof Error ? error.message : '订单加载失败', 'error');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load(1, mode);
  }, [mode]);

  const totalPage = Math.max(1, Math.ceil(total / PAGE_SIZE));

  return (
    <div className="order-container flex">
      <div className="menu-box">
        <div className="title">订单管理</div>
        <button type="button" className={mode === 1 ? 'menu-item active' : 'menu-item'} onClick={() => setMode(1)}>
          <span className="menu-item-text">线下销售</span>
        </button>
        <button type="button" className={mode === 2 ? 'menu-item active' : 'menu-item'} onClick={() => setMode(2)}>
          <span className="menu-item-text">商城订单</span>
        </button>
      </div>

      <div className="container pos-order-panel">
        <div className="pos-order-toolbar">
          <span>共 {total} 条</span>
          <button type="button" className="el-button" onClick={() => void load(page, mode)}>
            刷新
          </button>
        </div>

        <table className="pos-order-table">
          <thead>
            <tr>
              <th>订单号</th>
              <th>下单时间</th>
              <th>客户</th>
              <th>商品数</th>
              <th>金额</th>
              <th>支付状态</th>
              <th>订单状态</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.orderId}>
                <td>{row.orderSn}</td>
                <td>{row.addTime || '-'}</td>
                <td>
                  {row.consignee || '-'}
                  {row.mobile ? <span className="pos-order-mobile">{row.mobile}</span> : null}
                </td>
                <td>{row.itemCount}</td>
                <td>¥{Number(row.totalAmount ?? 0).toFixed(2)}</td>
                <td>{PAY_STATUS[row.payStatus] || '-'}</td>
                <td>{row.orderStatusName || '-'}</td>
              </tr>
            ))}
          </tbody>
        </table>

        {!loading && rows.length === 0 ? <div className="pos-order-empty">暂无订单</div> : null}

        <div className="pos-order-pager">
          <button type="button" className="el-button" disabled={page <= 1} onClick={() => void load(page - 1, mode)}>
            上一页
          </button>
          <span>
            {page} / {totalPage}
          </span>
          <button type="button" className="el-button" disabled={page >= totalPage} onClick={() => void load(page + 1, mode)}>
            下一页
          </button>
        </div>
      </div>
    </div>
  );
}
