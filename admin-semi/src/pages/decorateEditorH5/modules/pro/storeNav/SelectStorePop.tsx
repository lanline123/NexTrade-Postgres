/**
 * 门店导航 - 指定门店选择弹窗（复刻 view/admin mobilePro/storeNav/SelectStorePop.vue 口径）
 *
 * 关键词搜索（回车/按钮触发、重置到第 1 页）+ 分页表格多选（跨页保留勾选）+
 * 门店名 + LOGO（28×28）列；上限 STORE_NAV_MAX_STORE=20，超出时提示
 * 「最多支持20个门店」并回滚本次勾选；确认后才回写，写回条目为
 * { shopId, shopTitle, shopLogo }（与原版 toStoreItem 一致，提交时用当前页
 * 记录补齐 shopTitle/shopLogo）。
 * 数据接口：GET /merchant/shop/list（管理端门店列表同源接口），参数与原版
 * getShopList 一致：shopType=2、status=1、includeSelfOperated=1、keyword、page、size。
 */
import { useEffect, useState } from "react";
import { Button, Checkbox, Input, Modal, Pagination, Spin, Toast } from "@douyinfe/semi-ui";
import { apiGet } from "../../../../../api";
import { useI18n } from "../../../../../i18n";

/** 指定门店时保存的门店条目（与原版 StoreNavShopItem 一致） */
export interface StoreNavShopItem {
    shopId: number;
    shopTitle: string;
    shopLogo?: string;
}

/** 最多可选门店数（与原版 STORE_NAV_MAX_STORE 一致） */
export const STORE_NAV_MAX_STORE = 20;

const PAGE_SIZE = 10;

interface Props {
    visible: boolean;
    /** 已选门店，用于回显 */
    selectedList: StoreNavShopItem[];
    onCancel: () => void;
    onConfirm: (list: StoreNavShopItem[]) => void;
}

export default function SelectStorePop({ visible, selectedList, onCancel, onConfirm }: Props) {
    const { t } = useI18n();
    const [keyword, setKeyword] = useState("");
    const [appliedKeyword, setAppliedKeyword] = useState("");
    const [page, setPage] = useState(1);
    const [records, setRecords] = useState<any[]>([]);
    const [total, setTotal] = useState(0);
    const [loading, setLoading] = useState(false);
    /** 弹窗内的选择结果，确认后才回写模块（原版同口径） */
    const [selected, setSelected] = useState<StoreNavShopItem[]>([]);

    // 打开弹窗时以当前已选回显，并重置搜索与分页
    useEffect(() => {
        if (!visible) return;
        setSelected(selectedList.map((item) => ({ ...item })));
        setKeyword("");
        setAppliedKeyword("");
        setPage(1);
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [visible]);

    useEffect(() => {
        if (!visible) return;
        let alive = true;
        setLoading(true);
        apiGet<any>("/merchant/shop/list", {
            shopType: 2,
            status: 1,
            includeSelfOperated: 1,
            keyword: appliedKeyword || undefined,
            page,
            size: PAGE_SIZE
        })
            .then((res: any) => {
                if (!alive) return;
                setRecords(res?.records || res?.list || []);
                setTotal(Number(res?.total || 0));
            })
            .catch((error: any) => {
                if (!alive) return;
                setRecords([]);
                setTotal(0);
                Toast.error(error?.message || t("加载门店列表失败"));
            })
            .finally(() => {
                if (alive) setLoading(false);
            });
        return () => {
            alive = false;
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [visible, page, appliedKeyword]);

    const onSearch = () => {
        setPage(1);
        setAppliedKeyword(keyword.trim());
    };

    const isChecked = (shopId: number) => selected.some((item) => item.shopId === shopId);

    const toggleRow = (row: any, checked: boolean) => {
        const shopId = Number(row.shopId);
        if (!checked) {
            setSelected((prev) => prev.filter((item) => item.shopId !== shopId));
            return;
        }
        if (isChecked(shopId)) return;
        if (selected.length >= STORE_NAV_MAX_STORE) {
            Toast.warning(t("最多支持20个门店"));
            return;
        }
        setSelected((prev) => [...prev, { shopId, shopTitle: row.shopTitle || "", shopLogo: row.shopLogo || "" }]);
    };

    const pageIds = records.map((row) => Number(row.shopId));
    const allPageChecked = pageIds.length > 0 && pageIds.every((id) => isChecked(id));
    const togglePage = (checked: boolean) => {
        if (!checked) {
            setSelected((prev) => prev.filter((item) => !pageIds.includes(item.shopId)));
            return;
        }
        const additions = records.filter((row) => !isChecked(Number(row.shopId)));
        if (selected.length + additions.length > STORE_NAV_MAX_STORE) {
            Toast.warning(t("最多支持20个门店"));
            // 与原版一致：超限时仅回滚放不下的部分，能放下的照常加入
            const room = STORE_NAV_MAX_STORE - selected.length;
            if (room <= 0) return;
            setSelected((prev) => [
                ...prev,
                ...additions.slice(0, room).map((row) => ({ shopId: Number(row.shopId), shopTitle: row.shopTitle || "", shopLogo: row.shopLogo || "" }))
            ]);
            return;
        }
        setSelected((prev) => [
            ...prev,
            ...additions.map((row) => ({ shopId: Number(row.shopId), shopTitle: row.shopTitle || "", shopLogo: row.shopLogo || "" }))
        ]);
    };

    const handleOk = () => {
        // 提交时用当前页记录补齐 shopTitle/shopLogo，确保写入装修数据（原版 onFormSubmit 同口径）
        const result = selected.map((item) => {
            const row = records.find((r) => Number(r.shopId) === item.shopId);
            return {
                shopId: item.shopId,
                shopTitle: row?.shopTitle || item.shopTitle || "",
                shopLogo: row?.shopLogo || item.shopLogo || ""
            };
        });
        onConfirm(result);
    };

    return (
        <Modal title={t("选择门店")} visible={visible} onCancel={onCancel} onOk={handleOk} okText={t("确定")} cancelText={t("取消")} width={600}>
            <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 16 }}>
                <Input
                    value={keyword}
                    placeholder={t("请输入搜索关键字")}
                    style={{ width: 240 }}
                    onChange={(v: string) => setKeyword(v)}
                    onEnterPress={onSearch}
                />
                <Button theme="solid" type="primary" onClick={onSearch}>
                    {t("搜索")}
                </Button>
                <span style={{ color: "#999", fontSize: 12 }}>
                    {t("已选")} {selected.length} / {STORE_NAV_MAX_STORE}
                </span>
            </div>
            <Spin spinning={loading}>
                <div style={{ minHeight: 360 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "8px 4px", borderBottom: "1px solid #f0f0f0", color: "#646a73", fontSize: 13 }}>
                        <Checkbox checked={allPageChecked} onChange={(e: any) => togglePage(!!e?.target?.checked)} />
                        <span>{t("门店名称")}</span>
                    </div>
                    {records.map((row) => {
                        const shopId = Number(row.shopId);
                        return (
                            <div key={shopId} style={{ display: "flex", alignItems: "center", gap: 8, padding: "8px 4px", borderBottom: "1px solid #f5f5f5" }}>
                                <Checkbox checked={isChecked(shopId)} onChange={(e: any) => toggleRow(row, !!e?.target?.checked)} />
                                {row.shopLogo ? (
                                    <img src={row.shopLogo} alt="" style={{ width: 28, height: 28, flexShrink: 0, borderRadius: 4, objectFit: "contain" }} />
                                ) : null}
                                <span style={{ fontSize: 13 }}>{row.shopTitle || "--"}</span>
                            </div>
                        );
                    })}
                    {!loading && records.length === 0 ? (
                        <div style={{ padding: "48px 0", textAlign: "center", color: "#999", fontSize: 13 }}>{t("暂无数据")}</div>
                    ) : null}
                </div>
            </Spin>
            {total > 0 ? (
                <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 12 }}>
                    <Pagination currentPage={page} pageSize={PAGE_SIZE} total={total} onPageChange={(next: number) => setPage(next)} />
                </div>
            ) : null}
        </Modal>
    );
}
