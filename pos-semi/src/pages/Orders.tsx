import { useEffect, useState } from 'react';
import { api } from '../api';
import Dialog from '../components/Dialog';
import { toast } from '../toast';

interface OrderRow {
  orderId: number;
  orderSn: string;
  consignee: string;
  mobile: string;
  totalAmount: string | number;
  paidAmount: string | number;
  orderStatus: number;
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

interface OrderItem {
  productName?: string;
  productSn?: string;
  price?: string | number;
  quantity?: number;
  subtotal?: string | number;
}

interface OrderDetail extends OrderRow {
  address?: string;
  buyerNote?: string;
  items?: OrderItem[];
}

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
  const [detail, setDetail] = useState<OrderDetail | null>(null);
  const [detailOpen, setDetailOpen] = useState(false);
  const [detailLoading, setDetailLoading] = useState(false);

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

  const openDetail = async (orderId: number) => {
    setDetailOpen(true);
    setDetail(null);
    setDetailLoading(true);
    try {
      const result = await api.get<OrderDetail>('/order/order/detail', { id: orderId });
      setDetail(result);
    } catch (error) {
      toast(error instanceof Error ? error.message : '订单详情加载失败', 'error');
      setDetailOpen(false);
    } finally {
      setDetailLoading(false);
    }
  };

  const cancelOrder = async () => {
    if (!detail) return;
    try {
      await api.post('/order/order/cancelOrder', { id: detail.orderId, description: 'POS 收银取消' });
      toast('订单已取消');
      setDetailOpen(false);
      await load(page, mode);
    } catch (error) {
      toast(error instanceof Error ? error.message : '取消失败', 'error');
    }
  };

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
              <tr key={row.orderId} onClick={() => void openDetail(row.orderId)} className="pos-order-row">
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

        {detailOpen ? (
          <Dialog
            open={detailOpen}
            title="订单详情"
            width={640}
            onClose={() => setDetailOpen(false)}
            footer={
              <>
                {detail && detail.orderStatus === 0 ? (
                  <button type="button" className="el-button" onClick={() => void cancelOrder()}>
                    <span>取消订单</span>
                  </button>
                ) : null}
                <button type="button" className="el-button" onClick={() => setDetailOpen(false)}>
                  <span>关闭</span>
                </button>
              </>
            }
          >
            {detailLoading || !detail ? (
              <div className="pos-order-empty">加载中…</div>
            ) : (
              <div className="pos-order-detail">
                <div className="member-row"><span>订单号</span><strong>{detail.orderSn}</strong></div>
                <div className="member-row"><span>下单时间</span><span>{detail.addTime || '-'}</span></div>
                <div className="member-row"><span>客户</span><span>{detail.consignee || '-'} {detail.mobile || ''}</span></div>
                <div className="member-row"><span>地址</span><span>{detail.address || '-'}</span></div>
                <div className="member-row"><span>买家备注</span><span>{detail.buyerNote || '-'}</span></div>
                <table className="pos-order-table pos-order-items">
                  <thead>
                    <tr>
                      <th>商品</th>
                      <th>单价</th>
                      <th>数量</th>
                      <th>小计</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(detail.items || []).map((item, index) => (
                      <tr key={index}>
                        <td>{item.productName || '-'}</td>
                        <td>¥{Number(item.price ?? 0).toFixed(2)}</td>
                        <td>{item.quantity ?? 0}</td>
                        <td>¥{Number(item.subtotal ?? Number(item.price ?? 0) * Number(item.quantity ?? 0)).toFixed(2)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                <div className="member-row">
                  <span>合计</span>
                  <strong>¥{Number(detail.totalAmount ?? 0).toFixed(2)}</strong>
                </div>
                <div className="member-row">
                  <span>状态</span>
                  <span>{detail.orderStatusName} / {PAY_STATUS[detail.payStatus] || '-'}</span>
                </div>
              </div>
            )}
          </Dialog>
        ) : null}

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
