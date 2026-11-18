import { useEffect, useState } from 'react';
import { Button, Modal, Space, Table, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { apiUpload, getToken, isDemo } from '../api';
import { useI18n } from '../i18n';

const { Text } = Typography;

export interface BlacklistImportErrorRow {
  row: number;
  userId: string;
  mobile: string;
  username: string;
  product: string;
  behavior: string;
  shopIds: string;
  errorMsg: string;
}

export interface BlacklistImportResult {
  total: number;
  successCount: number;
  mergedCount: number;
  failCount: number;
  errors: BlacklistImportErrorRow[];
}

/** 带鉴权的 blob 下载（照 Members 模板下载的 fetch+blob 写法）。 */
async function downloadBlob(url: string, filename: string, init?: RequestInit): Promise<void> {
  const res = await fetch(url, {
    ...init,
    headers: { Authorization: `Bearer ${getToken()}`, ...(init?.headers || {}) },
  });
  if (!res.ok) throw new Error('下载失败');
  const blob = await res.blob();
  const objectUrl = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = objectUrl;
  link.download = filename;
  link.click();
  URL.revokeObjectURL(objectUrl);
}

/**
 * 黑名单批量导入弹窗（方案 §6 #3，与 Members 导入弹窗同构）。
 * 与会员导入的差异：导入接口返回结构化结果，部分失败时**不关弹窗**，
 * 就地展示失败行并提供「下载失败明细」（端点版：重传暂存的原文件调
 * getImportErrorData，改后可直接重提）。组件定义在模块顶层（remount 红线）。
 */
export default function BlacklistImportModal({
  visible,
  onCancel,
  onSuccess,
}: {
  visible: boolean;
  onCancel: () => void;
  onSuccess: () => void;
}) {
  const { t } = useI18n();
  const [importing, setImporting] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [result, setResult] = useState<BlacklistImportResult | null>(null);
  const [lastFile, setLastFile] = useState<File | null>(null);

  useEffect(() => {
    if (visible) {
      setResult(null);
      setLastFile(null);
    }
  }, [visible]);

  const downloadTemplate = async () => {
    try {
      await downloadBlob('/adminapi/user/blacklist/downloadImportTemplate', '黑名单导入模板.xlsx');
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('模板下载失败') });
    }
  };

  const downloadErrors = async () => {
    if (!lastFile) return;
    setDownloading(true);
    try {
      const form = new FormData();
      form.append('file', lastFile);
      await downloadBlob('/adminapi/user/blacklist/getImportErrorData', '黑名单导入失败明细.xlsx', {
        method: 'POST',
        body: form,
      });
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('失败明细下载失败') });
    } finally {
      setDownloading(false);
    }
  };

  const handleFile = async (file: File) => {
    if (isDemo()) {
      Toast.warning({ content: t('演示模式不支持操作') });
      return;
    }
    setImporting(true);
    setLastFile(file);
    try {
      const res = await apiUpload<BlacklistImportResult>('/user/blacklist/import', file);
      setResult(res);
      if (res && res.successCount > 0) {
        onSuccess();
      }
      if (res && res.failCount > 0) {
        Toast.warning({
          content: t('导入完成：成功 {ok} 行，失败 {fail} 行', {
            ok: res.successCount,
            fail: res.failCount,
          }),
        });
      } else {
        Toast.success({ content: t('导入成功') });
        onCancel();
      }
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('导入失败') });
    } finally {
      setImporting(false);
    }
  };

  return (
    <Modal title={t('批量导入黑名单')} visible={visible} onCancel={onCancel} footer={null} width={640}>
      <div style={{ display: 'grid', gap: 12 }}>
        <Space>
          <Button onClick={downloadTemplate}>{t('下载导入模板')}</Button>
          <label style={{ cursor: 'pointer' }}>
            <input
              type="file"
              accept=".xlsx,.xls"
              style={{ display: 'none' }}
              onChange={(event) => {
                const file = event.target.files?.[0];
                event.target.value = '';
                if (file) void handleFile(file);
              }}
            />
            <Button theme="solid" type="primary" loading={importing}>
              {t('选择 Excel 文件并导入')}
            </Button>
          </label>
        </Space>
        <Text type="tertiary" size="small">
          {t(
            '按模板填写：会员ID / 手机号 / 用户名三选一精确定位会员（多填须指向同一人），一行一个「产品 × 行为」组合，同一人多行会自动合并；单次最多 1000 行。',
          )}
        </Text>

        {result && (
          <div style={{ display: 'grid', gap: 8 }}>
            <Space>
              <Tag color="blue">
                {t('共 {count} 行', { count: result.total })}
              </Tag>
              <Tag color="green">
                {t('成功 {count} 行', { count: result.successCount })}
              </Tag>
              {result.mergedCount > 0 && (
                <Tag color="orange">
                  {t('合并 {count} 人', { count: result.mergedCount })}
                </Tag>
              )}
              {result.failCount > 0 && (
                <Tag color="red">
                  {t('失败 {count} 行', { count: result.failCount })}
                </Tag>
              )}
              {result.failCount > 0 && (
                <Button size="small" loading={downloading} onClick={downloadErrors}>
                  {t('下载失败明细')}
                </Button>
              )}
            </Space>
            {result.errors.length > 0 && (
              <Table
                size="small"
                pagination={false}
                scroll={{ y: 240 }}
                dataSource={result.errors}
                rowKey="row"
                columns={[
                  { title: t('行号'), dataIndex: 'row', width: 60 },
                  {
                    title: t('会员标识'),
                    width: 180,
                    render: (_: unknown, record: BlacklistImportErrorRow) =>
                      [record.userId && `ID ${record.userId}`, record.mobile, record.username]
                        .filter(Boolean)
                        .join(' / ') || '-',
                  },
                  { title: t('失败原因'), dataIndex: 'errorMsg' },
                ]}
              />
            )}
          </div>
        )}
      </div>
    </Modal>
  );
}
