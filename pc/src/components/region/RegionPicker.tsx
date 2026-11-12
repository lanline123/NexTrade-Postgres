'use client';

import { useCallback, useEffect, useState } from 'react';
import { Button, Modal, Toast } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { fetchProvinces, fetchRegionChildren, fetchRegionPaths, ROOT_REGION_ID, type RegionNode } from '@/lib/region';

/**
 * Vue 基准：components/region/src/SelectRegion.vue（级联选择器）
 *
 * 交互按 Vue 原版：点击展开 → 顶部按层级分 Tab → 点选某层节点即加载下一层，
 * 最末层（无子节点）点选后自动收起并回填。
 *
 * 与 Vue 版的差异：
 * - 不做「未选完整时重置」，改为确定键 + 末层自动提交，语义更明确；
 * - 展开时若已有值，用 sys/region/getRegion 一次性还原层级，避免逐层请求。
 */
interface Props {
    value?: number[];
    names?: string[];
    onChange: (ids: number[], names: string[]) => void;
    placeholder?: string;
    disabled?: boolean;
}

export default function RegionPicker({ value, names, onChange, placeholder, disabled }: Props) {
    const { t } = useTranslation();
    const [open, setOpen] = useState(false);
    const [loading, setLoading] = useState(false);
    const [levels, setLevels] = useState<RegionNode[][]>([]);
    const [active, setActive] = useState(0);
    const [picked, setPicked] = useState<number[]>([]);
    const [pickedNames, setPickedNames] = useState<string[]>([]);

    const selectedIds = value ?? [];
    const selectedNames = names ?? [];

    const bootstrap = useCallback(async () => {
        setLoading(true);
        try {
            const ids = selectedIds.filter((id) => Number(id) !== ROOT_REGION_ID);
            if (ids.length) {
                const groups = await fetchRegionPaths([ROOT_REGION_ID, ...ids]);
                // groups[0] 是国家层，地址只关心省/市/区。
                const tiers = groups.slice(1);
                if (tiers.length) {
                    setLevels(tiers);
                    setPicked(ids);
                    setPickedNames(
                        ids.map((id) => tiers.flat().find((node) => Number(node.regionId) === Number(id))?.regionName ?? ''),
                    );
                    setActive(Math.min(tiers.length - 1, ids.length - 1));
                    return;
                }
            }
            setLevels([await fetchProvinces()]);
            setPicked([]);
            setPickedNames([]);
            setActive(0);
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setLoading(false);
        }
    }, [selectedIds.join(',')]);

    useEffect(() => {
        if (open) void bootstrap();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [open]);

    const commit = (ids: number[], nextNames: string[]) => {
        onChange(ids, nextNames);
        setOpen(false);
    };

    const onPick = async (tierIndex: number, node: RegionNode) => {
        const nextIds = [...picked.slice(0, tierIndex), Number(node.regionId)];
        const nextNames = [...pickedNames.slice(0, tierIndex), node.regionName];
        setPicked(nextIds);
        setPickedNames(nextNames);

        let children: RegionNode[] = [];
        try {
            children = await fetchRegionChildren(node.regionId);
        } catch {
            children = [];
        }

        if (children.length === 0) {
            setLevels((prev) => prev.slice(0, tierIndex + 1));
            commit([ROOT_REGION_ID, ...nextIds], ['中国', ...nextNames]);
            return;
        }
        setLevels((prev) => [...prev.slice(0, tierIndex + 1), children]);
        setActive(tierIndex + 1);
    };

    const label = selectedNames.filter(Boolean).join(' ') || placeholder || t('请选择地区');

    return (
        <>
            <Button
                className="pc-region-trigger"
                theme="light"
                disabled={disabled}
                onClick={() => setOpen(true)}
                icon={null}
            >
                <span className={selectedNames.length ? undefined : 'pc-muted'}>{label}</span>
            </Button>

            <Modal
                visible={open}
                title={t('请选择地区')}
                onCancel={() => setOpen(false)}
                footer={
                    <div className="pc-region-foot">
                        <Button theme="borderless" onClick={() => setOpen(false)}>
                            {t('取消')}
                        </Button>
                        <Button
                            theme="solid"
                            type="primary"
                            disabled={picked.length === 0}
                            onClick={() => commit([ROOT_REGION_ID, ...picked], ['中国', ...pickedNames])}
                        >
                            {t('确定')}
                        </Button>
                    </div>
                }
                width={640}
            >
                {loading ? (
                    <div className="pc-loading">{t('加载中')}</div>
                ) : (
                    <>
                        <div className="pc-region-tabs">
                            {levels.map((_, index) => (
                                <span
                                    key={index}
                                    className={`pc-region-tab${active === index ? ' is-active' : ''}`}
                                    onClick={() => setActive(index)}
                                >
                                    {pickedNames[index] || t('请选择')}
                                </span>
                            ))}
                        </div>
                        <div className="pc-region-body">
                            {(levels[active] ?? []).map((node) => (
                                <span
                                    key={node.regionId}
                                    className={`pc-region-item${
                                        Number(picked[active]) === Number(node.regionId) ? ' is-active' : ''
                                    }`}
                                    onClick={() => void onPick(active, node)}
                                >
                                    {node.regionName}
                                </span>
                            ))}
                        </div>
                    </>
                )}
            </Modal>
        </>
    );
}
