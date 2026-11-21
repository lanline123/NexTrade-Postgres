import { useCallback, useEffect, useState } from 'react';
import {
  Badge,
  Button,
  Card,
  Empty,
  Input,
  Modal,
  Pagination,
  Popconfirm,
  Space,
  Spin,
  Table,
  TextArea,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';

const { Text, Title } = Typography;

/** 平台视角公告行（后端 AnnouncementVO）。 */
interface AnnouncementRow {
  msgId: number;
  title: string;
  content: string;
  sendTime: string;
  adminId: number;
  deliveredCount: number;
  readCount: number;
}

/** 商家视角公告行（复用消息中心 AdminMsgVO）。 */
interface ShopAnnouncementRow {
  msgId: number;
  title: string;
  content: string;
  sendTime: string;
  isReaded: number;
}

interface PageResult<T> {
  total?: number;
  list?: T[];
  records?: T[];
}

function pageList<T>(res: PageResult<T> | null | undefined): T[] {
  return res?.list ?? res?.records ?? [];
}

const PAGE_SIZE = 15;

/**
 * 平台公告（设置 → 消息设置，我方新增，论坛帖 587859 余子项）。
 *
 * 平台账号：发布公告（标题+正文，发布即向全部店铺各分发一行消息）、
 * 查看每条公告的送达/已读数、删除公告（主行与全部分发行一并删除）。
 * 店铺账号：本店公告列表，未读有红点，点开阅读即按消息中心既有口径
 * 标记已读（`/msg/adminMsg/setReaded`），铃铛未读数与消息中心分类树
 * 的「平台公告」未读数同源同消长，不另建已读状态。
 */
export default function PlatformAnnouncements() {
  const adminType = localStorage.getItem('adminType') || 'admin';
  const isPlatform = adminType === 'admin';
  return isPlatform ? <PlatformManage /> : <ShopAnnouncements />;
}

function PlatformManage() {
  const [loading, setLoading] = useState(true);
  const [publishing, setPublishing] = useState(false);
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [rows, setRows] = useState<AnnouncementRow[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);

  const load = useCallback(async (targetPage: number) => {
    setLoading(true);
    try {
      const res = await apiGet<PageResult<AnnouncementRow>>('/msg/announcement/list', {
        page: targetPage,
        size: PAGE_SIZE,
      });
      setRows(pageList(res));
      setTotal(Number(res?.total) || 0);
    } catch {
      Toast.error({ content: '加载公告列表失败' });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load(page);
  }, [load, page]);

  const publish = async () => {
    const t = title.trim();
    const c = content.trim();
    if (!t) {
      Toast.warning({ content: '请填写公告标题' });
      return;
    }
    if (!c) {
      Toast.warning({ content: '请填写公告内容' });
      return;
    }
    setPublishing(true);
    try {
      const vo = await apiPost<AnnouncementRow>('/msg/announcement/publish', { title: t, content: c });
      Toast.success({ content: `公告已发布，送达 ${vo?.deliveredCount ?? 0} 家店铺` });
      setTitle('');
      setContent('');
      setPage(1);
      void load(1);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : '发布失败' });
    } finally {
      setPublishing(false);
    }
  };

  const remove = async (row: AnnouncementRow) => {
    try {
      await apiPost('/msg/announcement/delete', { msgId: row.msgId });
      Toast.success({ content: '公告已删除' });
      void load(page);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : '删除失败' });
    }
  };

  const columns = [
    { title: '标题', dataIndex: 'title', width: 260 },
    { title: '内容', dataIndex: 'content', ellipsis: true },
    { title: '发布时间', dataIndex: 'sendTime', width: 170 },
    {
      title: '已读 / 送达',
      width: 130,
      render: (_: unknown, row: AnnouncementRow) => `${row.readCount} / ${row.deliveredCount}`,
    },
    {
      title: '操作',
      width: 90,
      render: (_: unknown, row: AnnouncementRow) => (
        <Popconfirm
          title="删除该公告？"
          content="删除后各店铺消息中心里的这条公告也会一并删除。"
          onConfirm={() => void remove(row)}
        >
          <Button type="danger" theme="borderless" size="small">删除</Button>
        </Popconfirm>
      ),
    },
  ];

  return (
    <div>
      <Card style={{ marginBottom: 16 }}>
        <Title heading={5} style={{ marginTop: 0 }}>发布平台公告</Title>
        <Text type="tertiary">
          公告将下发到全部店铺的消息中心（分类「其它消息 → 平台公告」），店铺阅读后各自标记已读。
        </Text>
        <div style={{ marginTop: 12 }}>
          <Input
            value={title}
            onChange={setTitle}
            placeholder="公告标题（不超过 100 字）"
            maxLength={100}
            showClear
          />
        </div>
        <div style={{ marginTop: 12 }}>
          <TextArea
            value={content}
            onChange={setContent}
            placeholder="公告内容（不超过 2000 字）"
            maxCount={2000}
            rows={5}
          />
        </div>
        <div style={{ marginTop: 12 }}>
          <Button type="primary" loading={publishing} onClick={() => void publish()}>
            发布公告
          </Button>
        </div>
      </Card>

      <Card title="已发布公告">
        <Table
          columns={columns}
          dataSource={rows}
          rowKey="msgId"
          loading={loading}
          pagination={false}
          empty={<Empty description="暂无公告" />}
        />
        {total > PAGE_SIZE && (
          <div style={{ marginTop: 12, textAlign: 'right' }}>
            <Pagination total={total} pageSize={PAGE_SIZE} currentPage={page} onPageChange={setPage} />
          </div>
        )}
      </Card>
    </div>
  );
}

function ShopAnnouncements() {
  const [loading, setLoading] = useState(true);
  const [rows, setRows] = useState<ShopAnnouncementRow[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [detail, setDetail] = useState<ShopAnnouncementRow | null>(null);

  const load = useCallback(async (targetPage: number) => {
    setLoading(true);
    try {
      const res = await apiGet<PageResult<ShopAnnouncementRow>>('/msg/adminMsg/list', {
        msgType: 59,
        page: targetPage,
        size: PAGE_SIZE,
      });
      setRows(pageList(res));
      setTotal(Number(res?.total) || 0);
    } catch {
      Toast.error({ content: '加载公告失败' });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load(page);
  }, [load, page]);

  const openDetail = async (row: ShopAnnouncementRow) => {
    setDetail(row);
    if (row.isReaded === 0) {
      try {
        await apiPost('/msg/adminMsg/setReaded', { msgId: row.msgId });
        setRows((prev) => prev.map((r) => (r.msgId === row.msgId ? { ...r, isReaded: 1 } : r)));
        setDetail({ ...row, isReaded: 1 });
      } catch {
        // 标记已读失败不阻断阅读，下次打开会重试。
      }
    }
  };

  return (
    <Card title="平台公告">
      {loading ? (
        <div style={{ textAlign: 'center', padding: 32 }}><Spin /></div>
      ) : rows.length === 0 ? (
        <Empty description="暂无平台公告" />
      ) : (
        <div>
          {rows.map((row) => (
            <button
              type="button"
              key={row.msgId}
              onClick={() => void openDetail(row)}
              style={{
                display: 'flex',
                width: '100%',
                alignItems: 'center',
                gap: 10,
                padding: '12px 8px',
                background: 'transparent',
                border: 'none',
                borderBottom: '1px solid var(--semi-color-border)',
                cursor: 'pointer',
                textAlign: 'left',
              }}
            >
              <Badge dot={row.isReaded === 0} type="danger">
                <span
                  style={{
                    fontWeight: row.isReaded === 0 ? 600 : 400,
                    color: 'var(--semi-color-text-0)',
                  }}
                >
                  {row.title}
                </span>
              </Badge>
              <span style={{ marginLeft: 'auto', color: 'var(--semi-color-text-2)', fontSize: 12 }}>
                {row.sendTime}
              </span>
            </button>
          ))}
          {total > PAGE_SIZE && (
            <div style={{ marginTop: 12, textAlign: 'right' }}>
              <Pagination total={total} pageSize={PAGE_SIZE} currentPage={page} onPageChange={setPage} />
            </div>
          )}
        </div>
      )}

      <Modal
        title={detail?.title}
        visible={Boolean(detail)}
        onCancel={() => setDetail(null)}
        footer={null}
        width={560}
      >
        <Space vertical align="start" style={{ width: '100%' }}>
          <Text type="tertiary">{detail?.sendTime}</Text>
          <div style={{ whiteSpace: 'pre-wrap', lineHeight: 1.7 }}>{detail?.content}</div>
        </Space>
      </Modal>
    </Card>
  );
}
