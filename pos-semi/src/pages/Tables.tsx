import { useEffect, useState } from 'react';
import { api } from '../api';
import Dialog from '../components/Dialog';
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
  const [openTable, setOpenTable] = useState<TableRow | null>(null);
  const [diningCount, setDiningCount] = useState('2');
  const [clearTable, setClearTable] = useState<TableRow | null>(null);
  const [transferMode, setTransferMode] = useState(false);
  const [transferFrom, setTransferFrom] = useState<TableRow | null>(null);

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

  const openTableNow = async () => {
    if (!openTable) return;
    const count = Number(diningCount);
    if (!Number.isFinite(count) || count <= 0) {
      toast('请输入正确的就餐人数', 'error');
      return;
    }
    try {
      await api.post('/pos/table/operate/open', { tableId: openTable.tableId, diningCount: count });
      setOpenTable(null);
      await loadTables();
      toast('开台成功');
    } catch (error) {
      toast(error instanceof Error ? error.message : '开台失败', 'error');
    }
  };

  const clearTableNow = async () => {
    if (!clearTable) return;
    try {
      await api.post('/pos/table/operate/clear', { tableId: clearTable.tableId });
      setClearTable(null);
      await loadTables();
      toast('清台成功');
    } catch (error) {
      toast(error instanceof Error ? error.message : '清台失败', 'error');
    }
  };

  const handleCardClick = (table: TableRow) => {
    if (transferMode) {
      if (!transferFrom) {
        if (table.tableStatus === 0) {
          toast('请选择已开台的桌台', 'error');
          return;
        }
        setTransferFrom(table);
        return;
      }
      if (table.tableId === transferFrom.tableId) {
        setTransferFrom(null);
        return;
      }
      if (table.tableStatus !== 0) {
        toast('目标桌台已被占用', 'error');
        return;
      }
      void (async () => {
        try {
          await api.post('/pos/table/operate/transfer', {
            tableId: transferFrom.tableId,
            targetTableId: table.tableId,
          });
          setTransferMode(false);
          setTransferFrom(null);
          await loadTables();
          toast('转台成功');
        } catch (error) {
          toast(error instanceof Error ? error.message : '转台失败', 'error');
        }
      })();
      return;
    }
    if (table.tableStatus === 0) {
      setOpenTable(table);
      setDiningCount('2');
      return;
    }
    if (table.tableStatus === 3) {
      setClearTable(table);
      return;
    }
    toast(`${table.tableNo}：${STATUS[table.tableStatus] || '未知状态'}`);
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
          <button
            type="button"
            className={transferMode ? 'action-btn active' : 'action-btn'}
            onClick={() => {
              setTransferMode((value) => !value);
              setTransferFrom(null);
            }}
          >
            转台
          </button>
          <button type="button" className={loading ? 'action-btn is-loading' : 'action-btn'} disabled={loading} onClick={() => void loadTables()}>
            刷新
          </button>
        </div>
      </div>

      {transferMode ? (
        <div className="pos-transfer-bar">
          <span>{transferFrom ? `已选：${transferFrom.tableNo}，请选择目标桌台` : '请选择要转出的桌台'}</span>
          <button type="button" className="el-button" onClick={() => { setTransferMode(false); setTransferFrom(null); }}>
            取消
          </button>
        </div>
      ) : null}

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
              className={`table-card status-${table.tableStatus}${transferFrom?.tableId === table.tableId ? ' is-selected' : ''}`}
              onClick={() => handleCardClick(table)}
            >
              <div className="card-header">
                <span className="table-no">{table.tableNo}</span>
              </div>
              <div className="card-body">
                <div className="card-meta">
                  {table.tableStatus === 0 ? STATUS[0] : `${table.diningCount}人 | ${minutesSince(table.openTime)}分钟`}
                </div>
                <div className="card-status">{STATUS[table.tableStatus] || '-'}</div>
                {table.transferFromTableNo ? <div className="card-meta">转自 {table.transferFromTableNo}</div> : null}
              </div>
              <div className="card-footer">
                <div className="card-footer-main">
                  {table.tableStatus === 3 ? (
                    <button
                      type="button"
                      className="el-button"
                      onClick={(event) => {
                        event.stopPropagation();
                        setClearTable(table);
                      }}
                    >
                      清台
                    </button>
                  ) : null}
                </div>
              </div>
            </div>
          ))}
        </div>
        {!loading && tables.length === 0 ? (
          <div className="board-empty">暂无桌台，请先到后台收银设置的桌号管理添加</div>
        ) : null}
      </div>

      <Dialog
        open={!!openTable}
        title="开台"
        width={420}
        onClose={() => setOpenTable(null)}
        footer={
          <>
            <button type="button" className="el-button" onClick={() => setOpenTable(null)}>
              <span>取消</span>
            </button>
            <button type="button" className="el-button el-button--primary" onClick={() => void openTableNow()}>
              <span>确认开台</span>
            </button>
          </>
        }
      >
        <div className="member-row">
          <span>桌号</span>
          <strong>{openTable?.tableNo || '-'}</strong>
        </div>
        <div className="price-input">
          <div className="el-input">
            <div className="el-input__wrapper">
              <input
                className="el-input__inner"
                placeholder="请输入就餐人数"
                value={diningCount}
                onChange={(event) => setDiningCount(event.target.value)}
              />
            </div>
          </div>
          <span className="price-unit">人</span>
        </div>
      </Dialog>

      <Dialog
        open={!!clearTable}
        title="清台"
        width={420}
        onClose={() => setClearTable(null)}
        footer={
          <>
            <button type="button" className="el-button" onClick={() => setClearTable(null)}>
              <span>取消</span>
            </button>
            <button type="button" className="el-button el-button--primary" onClick={() => void clearTableNow()}>
              <span>确认清台</span>
            </button>
          </>
        }
      >
        <p className="confirm-text">确定要清台 {clearTable?.tableNo || ''} 吗？</p>
      </Dialog>
    </div>
  );
}
