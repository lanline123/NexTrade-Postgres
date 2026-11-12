'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { Button, Card, Empty, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiUpload } from '@/lib/client-api';
import ProductResults from '@/components/catalog/ProductResults';

/**
 * 以图搜图。
 * Vue 基准：components/imageSearch/{ImageSearchEntry,ImageSearchPanel}.vue + api/search/imageSearch.ts
 *
 * 真实接口：
 * - GET  search/image/capabilities → { enabled, pcEnabled, shopEnabled, formats[] }
 * - POST search/image/search（multipart：file、client=pc、size）
 *        → { searchToken }
 * - 列表复用 search/search/getProduct，带上 searchToken 即限定在本次召回集合内。
 *
 * 与 Vue 版差异：不做「刷新后恢复上次查询」（原版把原图暂存在 IndexedDB），
 * 换图即重新检索；缩略图仅在当前页面用 objectURL 展示。
 */
interface Capabilities {
    enabled?: boolean;
    pcEnabled?: boolean;
    formats?: string[];
}

const PAGE_SIZE = 30;

export default function ImageSearchPage() {
    const { t } = useTranslation();
    const inputRef = useRef<HTMLInputElement | null>(null);
    const [capabilities, setCapabilities] = useState<Capabilities | null>(null);
    const [searchToken, setSearchToken] = useState('');
    const [thumb, setThumb] = useState('');
    const [uploading, setUploading] = useState(false);

    useEffect(() => {
        apiGet<Capabilities>('search/image/capabilities')
            .then((result) => setCapabilities(result ?? {}))
            .catch(() => setCapabilities({ enabled: false }));
    }, []);

    useEffect(() => () => {
        if (thumb) URL.revokeObjectURL(thumb);
    }, [thumb]);

    const accept = (capabilities?.formats ?? ['JPEG', 'PNG', 'WEBP', 'BMP', 'GIF'])
        .map((item) => `.${String(item).toLowerCase()}`)
        .join(',');

    const onFiles = useCallback(
        async (files: FileList | null) => {
            const file = files?.[0];
            if (!file) return;
            if (!file.type.startsWith('image/')) {
                Toast.warning(t('请选择图片文件'));
                return;
            }
            if (thumb) URL.revokeObjectURL(thumb);
            setThumb(URL.createObjectURL(file));
            setSearchToken('');
            setUploading(true);
            try {
                const form = new FormData();
                form.append('file', file);
                form.append('client', 'pc');
                form.append('size', String(PAGE_SIZE));
                const session = await apiUpload<{ searchToken?: string }>('search/image/search', form, 90000);
                if (!session?.searchToken) throw new Error(t('图片搜索失败，请重试'));
                setSearchToken(session.searchToken);
            } catch (e) {
                Toast.error((e as Error).message);
            } finally {
                setUploading(false);
            }
        },
        [t, thumb],
    );

    if (capabilities && (!capabilities.enabled || capabilities.pcEnabled === false)) {
        return <Empty title={t('以图搜图暂未开放')} />;
    }

    return (
        <div className="pc-image-search">
            <Typography.Title heading={4}>{t('以图搜图')}</Typography.Title>

            <input
                ref={inputRef}
                type="file"
                accept={accept || undefined}
                style={{ display: 'none' }}
                onChange={(event) => void onFiles(event.target.files)}
            />

            <Card className="pc-image-search-drop">
                <div
                    className="pc-image-search-zone"
                    onClick={() => inputRef.current?.click()}
                    onDragOver={(event) => event.preventDefault()}
                    onDrop={(event) => {
                        event.preventDefault();
                        void onFiles(event.dataTransfer.files);
                    }}
                >
                    {thumb ? (
                        <img src={thumb} alt="" className="pc-image-search-thumb" />
                    ) : (
                        <div className="pc-muted">
                            <div>{t('点击或拖拽图片到此处')}</div>
                            <div style={{ fontSize: 12, marginTop: 6 }}>
                                {t('支持格式')}：{(capabilities?.formats ?? []).join(' / ') || 'JPG / PNG / WEBP'}
                            </div>
                        </div>
                    )}
                </div>
                <div className="pc-image-search-actions">
                    <Button theme="light" loading={uploading} onClick={() => inputRef.current?.click()}>
                        {thumb ? t('换一张图片') : t('选择图片')}
                    </Button>
                    {thumb ? (
                        <Button
                            theme="borderless"
                            onClick={() => {
                                if (thumb) URL.revokeObjectURL(thumb);
                                setThumb('');
                                setSearchToken('');
                            }}
                        >
                            {t('清空')}
                        </Button>
                    ) : null}
                </div>
            </Card>

            {searchToken ? (
                <div className="pc-image-search-results">
                    <Typography.Title heading={5}>{t('相似商品')}</Typography.Title>
                    <ProductResults searchToken={searchToken} pageSize={PAGE_SIZE} emptyText={t('没有找到相似商品')} />
                </div>
            ) : null}
        </div>
    );
}
