import { useEffect, useState } from 'react';
import { api } from '../api';
import { toast } from '../toast';

interface ReservationRow {
  orderReservationsId: number;
  orderId: number;
  orderSn: string;
  consignee: string;
  mobile: string;
  reservationDate: string;
  startTime: string;
  endTime: string;
  timeSlotDescription: string;
  addTime: number;
  orderStatus: number;
}

interface ReservationPage {
  records: ReservationRow[];
  total: number;
  page: number;
  size: number;
}

const PAGE_SIZE = 15;

function formatTime(value: number): string {
  if (!value) return '-';
  const date = new Date(value * 1000);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export default function Reservations() {
  const [keyword, setKeyword] = useState('');
  const [rows, setRows] = useState<ReservationRow[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);

  const load = async (nextPage: number, nextKeyword: string) => {
    setLoading(true);
    try {
      const result = await api.get<ReservationPage>('/pos/order/OrderReservation/list', {
        page: nextPage,
        size: PAGE_SIZE,
        keyword: nextKeyword.trim() || undefined,
      });
      setRows(result.records || []);
      setTotal(result.total || 0);
      setPage(result.page || nextPage);
    } catch (error) {
      toast(error instanceof Error ? error.message : '预约加载失败', 'error');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load(1, '');
  }, []);

  const totalPage = Math.max(1, Math.ceil(total / PAGE_SIZE));

  return (
    <div className="container pos-member-panel">
      <div className="pos-order-toolbar">
        <div className="price-input pos-member-search">
          <div className="el-input">
            <div className="el-input__wrapper">
              <input
                className="el-input__inner"
                placeholder="订单号 / 收货人 / 手机号"
                value={keyword}
                onChange={(event) => setKeyword(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') void load(1, keyword);
                }}
              />
            </div>
          </div>
          <button type="button" className="el-button" onClick={() => void load(1, keyword)}>
            查询
          </button>
        </div>
        <span>共 {total} 条预约</span>
      </div>

      <table className="pos-order-table">
        <thead>
          <tr>
            <th>预约ID</th>
            <th>订单号</th>
            <th>客户</th>
            <th>手机号</th>
            <th>预约日期</th>
            <th>时段</th>
            <th>下单时间</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.orderReservationsId}>
              <td>{row.orderReservationsId}</td>
              <td>{row.orderSn || '-'}</td>
              <td>{row.consignee || '-'}</td>
              <td>{row.mobile || '-'}</td>
              <td>{row.reservationDate || '-'}</td>
              <td>{row.timeSlotDescription || `${row.startTime || ''}-${row.endTime || ''}`}</td>
              <td>{formatTime(row.addTime)}</td>
            </tr>
          ))}
        </tbody>
      </table>

      {!loading && rows.length === 0 ? <div className="pos-order-empty">暂无预约</div> : null}

      <div className="pos-order-pager">
        <button type="button" className="el-button" disabled={page <= 1} onClick={() => void load(page - 1, keyword)}>
          上一页
        </button>
        <span>
          {page} / {totalPage}
        </span>
        <button type="button" className="el-button" disabled={page >= totalPage} onClick={() => void load(page + 1, keyword)}>
          下一页
        </button>
      </div>
    </div>
  );
}
