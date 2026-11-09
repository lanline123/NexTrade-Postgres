import { useEffect, useState } from 'react';
import { Button, Card, Switch, Toast, Typography } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import RichTextEditor from '../components/RichTextEditor';

const { Text } = Typography;

interface AgreementPageProps {
  code: string;
  title: string;
  description: string;
}

export default function AgreementPage({ code, title, description }: AgreementPageProps) {
  const [show, setShow] = useState(0);
  const [content, setContent] = useState('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setLoading(true);
    apiGet<{ show?: number; content?: string }>('/setting/config/getLoginProtocolContent', { code })
      .then((result) => {
        setShow(Number(result?.show || 0));
        setContent(result?.content || '');
      })
      .catch((error) => Toast.error({ content: error instanceof Error ? error.message : '加载失败' }))
      .finally(() => setLoading(false));
  }, [code]);

  const save = async () => {
    setSaving(true);
    try {
      await apiPost('/setting/config/saveLoginProtocol', { code, show, content });
      Toast.success({ content: '保存成功' });
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : '保存失败' });
    } finally {
      setSaving(false);
    }
  };

  return (
    <Card className="agreement-page" shadows="hover">
      <div className="agreement-head">
        <div>
          <h3>{title}</h3>
          <Text type="tertiary" size="small">
            {description}
          </Text>
        </div>
        <div className="agreement-switch">
          <Text type="tertiary" size="small">
            启用协议
          </Text>
          <Switch checked={show === 1} onChange={(next) => setShow(next ? 1 : 0)} />
        </div>
      </div>
      <div className="agreement-editor">
        <RichTextEditor value={content} onChange={setContent} height={520} />
      </div>
      <div className="agreement-actions">
        <Button theme="solid" type="primary" loading={saving || loading} onClick={save}>
          保存
        </Button>
      </div>
    </Card>
  );
}
