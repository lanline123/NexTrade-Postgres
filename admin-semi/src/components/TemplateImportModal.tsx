import { useCallback, useEffect, useState, type CSSProperties } from 'react';
import { Button, Empty, Modal, Spin, Toast, Typography} from '@douyinfe/semi-ui';
import { IconTick} from '@douyinfe/semi-icons';
import { apiPost} from '../api';
import { useI18n} from '../i18n';
import {
fetchTemplateCategories,
fetchTemplateImport,
fetchTemplates,
type TemplateCategory,
type TemplateItem,
type TemplateTarget,
} from '../config/templatePlatform';

const { Text} = Typography;

/** 导入成功后回传给宿主的新页面信息（宿主据此跳转编辑器继续编辑） */
export interface TemplateImportedInfo {
decorateId: number;
decorateTitle: string;
pageType: number;
decorateType: number;
}

interface Props {
visible: boolean;
/** 当前宿主端：pc 传 'pc'、移动端传 'mobile'（与平台契约 target 同值），只列该端模板 */
platform: TemplateTarget;
onClose: () => void;
onImported: (info: TemplateImportedInfo) => void;
}

/**
* 「从模板导入」弹窗：装修模板独立平台（tpl.recy.cc）
* 的本站入口。分类排在顶部，模板卡片网格单选；确定后取平台导入包
* （快照的 DecorateVO.data 原文），在本站走新建草稿页链落库
* （/decorate/decorate/create 建页 → /decorate/decorate/saveDraft 落草稿），
* 新页恒为未发布草稿、绝不自动发布，再把新页信息回给宿主跳转编辑。
* 平台数据层与契约见 config/templatePlatform.ts。
*/
export default function TemplateImportModal({ visible, platform, onClose, onImported}: Props) {
const { t} = useI18n();
const [categories, setCategories] = useState<TemplateCategory[]>([]);
const [categoryId, setCategoryId] = useState(0);
const [templates, setTemplates] = useState<TemplateItem[]>([]);
const [selectedId, setSelectedId] = useState(0);
const [loading, setLoading] = useState(false);
const [loadError, setLoadError] = useState('');
const [importing, setImporting] = useState(false);

const loadTemplates = useCallback(
async (catId: number) => {
setLoading(true);
setLoadError('');
try {
const list = await fetchTemplates(platform, catId || undefined);
setTemplates(list);
setSelectedId(0);
} catch (e: any) {
setTemplates([]);
setLoadError(e?.message || t('模板加载失败'));
} finally {
setLoading(false);
}
},
[platform, t],
);

// 打开时：拉分类（顶部排）+ 全部分类模板
useEffect(() => {
if (!visible) return;
setCategoryId(0);
setSelectedId(0);
fetchTemplateCategories()
.then(setCategories)
.catch(() => setCategories([]));
void loadTemplates(0);
}, [visible, loadTemplates]);

const handleCategory = (id: number) => {
setCategoryId(id);
void loadTemplates(id);
};

const selected = templates.find((item) => item.id === selectedId) || null;

const handleConfirm = async () => {
if (!selected) return;
setImporting(true);
try {
// 1) 取平台导入包（快照数据，导入永不回源分享链接）
const pkg = await fetchTemplateImport(selected.id);
const decorateType = Number(pkg.page.decorateType) || (platform === 'pc'? 2: 1);
const pageType = Number(pkg.page.pageType) || 1;
const decorateTitle = (pkg.page.decorateTitle || pkg.name || selected.name || '').trim() || t('导入模板');
// 2) 本站新建草稿页链：先建页（标题/端类型/用途），再把页面数据落草稿
const created = await apiPost<any>('/decorate/decorate/create', {
decorateType,
pageType,
decorateTitle,
});
const newId = Number(created?.decorateId?? created?.id?? 0);
if (!newId) throw new Error(t('导入失败'));
await apiPost('/decorate/decorate/saveDraft', {
id: newId,
decorateType,
pageType,
data: pkg.page.data,
});
Toast.success(t('导入成功'));
onImported({ decorateId: newId, decorateTitle, pageType, decorateType});
} catch (e: any) {
Toast.error(e?.message || t('导入失败'));
} finally {
setImporting(false);
}
};

const chipStyle = (active: boolean): CSSProperties => ({
padding: '4px 14px',
border: `1px solid ${active? 'var(--semi-color-primary)': 'var(--semi-color-border)'}`,
background: active? 'var(--semi-color-primary-light-default)': '#fff',
color: active? 'var(--semi-color-primary)': 'var(--semi-color-text-0)',
cursor: 'pointer',
fontSize: 13,
borderRadius: 0,
userSelect: 'none',
});

return (
// centered：标准居中（Semi 默认 margin-top:80px 顶置，长弹窗页脚易出视口需滚动）。
// motion={false}：当前构建产物中 Semi 弹窗的全局动画类（.semi-modal-content-animate-* /
// .semi-modal-mask-animate-* / .semi-modal-displayNone）缺失、仅剩 MapLocationPicker 的
// scoped 覆盖；Semi 开启动画时 visible=false 后须等关闭动画 animationend 才卸载，
// 动画类缺失使该事件永不触发 → 取消/X/Escape/遮罩四路关闭全部无效（2026-11-22 admin r9
// 浏览器实点实证）。关闭动画后 visible=false 立即卸载，四路恢复 Semi 常规口径。
<Modal
title={t('从模板导入')}
visible={visible}
width={880}
centered
motion={false}
onCancel={onClose}
footer={
<div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8}}>
<Button onClick={onClose}>{t('取消')}</Button>
<Button
theme="solid"
type="primary"
loading={importing}
disabled={!selected}
onClick={handleConfirm}
>
{t('确定')}
</Button>
</div>
}
>
{/* 分类排（顶部）：全部 + 平台分类 */}
<div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, marginBottom: 16}}>
<span style={chipStyle(categoryId === 0)} onClick={() => handleCategory(0)}>
{t('全部')}
</span>
{categories.map((c) => (
<span key={c.id} style={chipStyle(categoryId === c.id)} onClick={() => handleCategory(c.id)}>
{c.name}
</span>
))}
</div>

{loading? (
<div style={{ padding: '64px 0', textAlign: 'center'}}>
<Spin size="large" />
</div>
): loadError? (
<div style={{ padding: '32px 0', textAlign: 'center'}}>
<Text type="danger">{loadError}</Text>
<div style={{ marginTop: 12}}>
<Button onClick={() => void loadTemplates(categoryId)}>{t('重试')}</Button>
</div>
</div>
): templates.length === 0? (
<Empty title={t('暂无模板')} description={t('模板平台暂无该分类的模板')} style={{ padding: '32px 0'}} />
): (
<div
style={{
display: 'grid',
gridTemplateColumns: 'repeat(auto-fill, minmax(180px, 1fr))',
gap: 12,
maxHeight: '52vh',
overflowY: 'auto',
}}
>
{templates.map((item) => {
const active = item.id === selectedId;
return (
<div
key={item.id}
onClick={() => setSelectedId(item.id)}
style={{
border: `1px solid ${active? 'var(--semi-color-primary)': 'var(--semi-color-border)'}`,
borderRadius: 0,
cursor: 'pointer',
position: 'relative',
background: '#fff',
}}
>
<div
style={{
height: 110,
background: 'var(--semi-color-fill-0)',
display: 'flex',
alignItems: 'center',
justifyContent: 'center',
overflow: 'hidden',
}}
>
{item.cover? (
<img
src={item.cover}
alt={item.name}
style={{ width: '100%', height: '100%', objectFit: 'cover'}}
/>
): (
<Text type="tertiary">{t('无封面')}</Text>
)}
</div>
<div style={{ padding: '8px 10px'}}>
<div
style={{
fontSize: 13,
whiteSpace: 'nowrap',
overflow: 'hidden',
textOverflow: 'ellipsis',
}}
>
{item.name}
</div>
{typeof item.moduleCount === 'number'? (
<div style={{ fontSize: 12, color: 'var(--semi-color-text-2)', marginTop: 2}}>
{t('{count} 个模块', { count: item.moduleCount})}
</div>
): null}
</div>
{active && (
<span
style={{
position: 'absolute',
top: 0,
right: 0,
width: 22,
height: 22,
background: 'var(--semi-color-primary)',
color: '#fff',
display: 'flex',
alignItems: 'center',
justifyContent: 'center',
}}
>
<IconTick size="small" />
</span>
)}
</div>
);
})}
</div>
)}

<div style={{ marginTop: 12}}>
<Text type="tertiary" size="small">
{t('导入后生成未发布的新页面，可在编辑器中继续修改，确认无误后自行发布。')}
</Text>
</div>
</Modal>
);
}
