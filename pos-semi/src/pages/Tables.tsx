import { useEffect, useState } from 'react';
import { api } from '../api';
import { toast } from '../toast';

interface Area {
  areaId: number;
  areaName: string;
}

interface TableRow {
  tableId: number;
  tableNo: string;
  areaId: number;
  status: number;
  tableStatus: number;
  diningCount: number;
  openTime: number;
  sessionSn: string;
  transferFromTableNo: string;
}

const STATUS: Record<number, string> = {
  0: '未开台',
  1: '待下单',
  2: '待结账',
  3: '待清台',
};

function shopIdOf(): number | undefined {
  const raw = localStorage.getItem('shopId');
  const id = raw ? Number(raw) : NaN;
  return Number.isFinite(id) && id > 0 ? id : undefined;
}

function minutesSince(openTime: number): number {
  if (!openTime) return 0;
  return Math.max(0, Math.floor((Date.now() / 1000 - openTime) / 60));
}

export default function Tables() {
  const [areas, setAreas] = useState<Area[]>([]);
  const [tables, setTables] = useState<TableRow[]>([]);
  const [areaId, setAreaId] = useState(0);
  const [statusFilter, setStatusFilter] = useState(-1);
  const [loading, setLoading] = useState(false);

  const loadAreas = async () => {
    try {
      const result = await api.post<Area[] | { records: Area[] }>('/pos/table/area/list', {
        shopId: shopIdOf(),
      });
      setAreas(Array.isArray(result) ? result : result.records || []);
    } catch (error) {
      toast(error instanceof Error ? error.message : '区域加载失败', 'error');
    }
  };

  const loadTables = async () => {
    setLoading(true);
    try {
      const result = await api.get<TableRow[] | { records: TableRow[] }>('/pos/table/list', {
        shopId: shopIdOf(),
        areaId: areaId > 0 ? areaId : undefined,
        status: statusFilter >= 0 ? statusFilter : undefined,
      });
      setTables(Array.isArray(result) ? result : result.records || []);
    } catch (error) {
      toast(error instanceof Error ? error.message : '桌台加载失败', 'error');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadAreas();
  }, []);

  useEffect(() => {
    void loadTables();
  }, [areaId, statusFilter]);

  const summary = {
    total: tables.length,
    idle: tables.filter((t) => t.tableStatus === 0).length,
    pending: tables.filter((t) => t.tableStatus === 1).length,
    settle: tables.filter((t) => t.tableStatus === 2).length,
    clear: tables.filter((t) => t.tableStatus === 3).length,
  };

  return (
    <div className="container dining-board">
      <div className="board-toolbar">
        <div className="toolbar-cluster">
          <div className="area-tabs">
            <button type="button" className={areaId === 0 ? 'area-tab active' : 'area-tab'} onClick={() => setAreaId(0)}>
              全部
            </button>
            {areas.map((area) => (
              <button
                key={area.areaId}
                type="button"
                className={areaId === area.areaId ? 'area-tab area-name active' : 'area-tab area-name'}
                onClick={() => setAreaId(area.areaId)}
              >
                {area.areaName}
              </button>
            ))}
          </div>
        </div>
        <div className="toolbar-actions">
          <button type="button" className={loading ? 'action-btn is-loading' : 'action-btn'} disabled={loading} onClick={() => void loadTables()}>
            刷新
          </button>
        </div>
      </div>

      <div className="status-legend-wrap">
        <div className="status-legend">
          <button type="button" className={statusFilter === -1 ? 'legend-chip active' : 'legend-chip'} onClick={() => setStatusFilter(-1)}>
            全部 ({summary.total})
          </button>
          <button type="button" className={statusFilter === 0 ? 'legend-item active' : 'legend-item'} onClick={() => setStatusFilter(0)}>
            <span className="swatch status-0" /> 未开台 ({summary.idle})
          </button>
          <button type="button" className={statusFilter === 1 ? 'legend-item active' : 'legend-item'} onClick={() => setStatusFilter(1)}>
            <span className="swatch status-1" /> 待下单 ({summary.pending})
          </button>
          <button type="button" className={statusFilter === 2 ? 'legend-item active' : 'legend-item'} onClick={() => setStatusFilter(2)}>
            <span className="swatch status-2" /> 待结账 ({summary.settle})
          </button>
          <button type="button" className={statusFilter === 3 ? 'legend-item active' : 'legend-item'} onClick={() => setStatusFilter(3)}>
            <span className="swatch status-3" /> 待清台 ({summary.clear})
          </button>
        </div>
      </div>

      <div className="board-panel">
        <div className="table-grid">
          {tables.map((table) => (
            <div
              key={table.tableId}
              className={`table-card status-${table.tableStatus}`}
              onClick={() => toast(`${table.tableNo}：${STATUS[table.tableStatus] || '未知状态'}`)}
            >
              <div className="card-header">
                <span className="table-no">{table.tableNo}</span>
              </div>
              <div className="card-body">
                <div className="card-meta">
                  {table.tableStatus === 0
                    ? STATUS[0]
                    : `${table.diningCount}人 | ${minutesSince(table.openTime)}分钟`}
                </div>
                <div className="card-status">{STATUS[table.tableStatus] || '-'}</div>
                {table.transferFromTableNo ? <div className="card-meta">转自 {table.transferFromTableNo}</div> : null}
              </div>
              <div className="card-footer">
                <div className="card-footer-main" />
              </div>
            </div>
          ))}
        </div>
        {!loading && tables.length === 0 ? (
          <div className="board-empty">暂无桌台，请先到后台收银设置的桌号管理添加</div>
        ) : null}
      </div>
    </div>
  );
}
