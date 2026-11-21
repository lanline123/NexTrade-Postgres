/**
 * 从其他页面复制（原版有·我方补齐）：顶栏「更多」入口，此前为无行为占位。
 *
 * 原版口径（v5.8.61 实证）：
 * - editor/Index.vue：「更多」→ DialogForm 抽屉（标题「选择模板」，700px）
 *   挂 DecorateLanguagesList，参数 { id: 当前页, decorateType }；
 * - DecorateLanguagesList.vue：getDecorateList 拉同 decorateType 的全部页面
 *   （不按 pageType 过滤、含当前页本身；parentId=-1 在原版表示不限），
 *   列为模板名称 / 模板语言 / 更新时间，每行一个「复制」；
 * - 点「复制」先弹确认「复制该模板将清空当前模板装修所有设置，是否继续操作？」，
 *   确认后回调 copyDecorate(id)：拉源页 decorateDetail 的已发布 data（非草稿）
 *   → store.applyTemplate（moduleList 全量替换 + url 回填 + 当前页型固定模块
 *   注入，pageModule 合并），置脏走既有存草稿/发布流。
 *
 * 我方适配（口径等价、形态因地制宜）：
 * - 我方后端 list 的 parentId 为等值过滤，传 -1 会查空——故不传 parentId，
 *   「不限」的范围与原版等价；
 * - 我方列表 VO 只有 localeId、无语言名称（多语言编辑未迁），原版「模板语言」
 *   列改为「页面类型」列；多商户口径下店铺页在名称后标注店铺名。
 */
import { useCallback, useEffect, useState } from "react";
import { Button, Modal, Pagination, SideSheet, Table, Toast } from "@douyinfe/semi-ui";
import { apiGet } from "../../../api";
import { editorStore } from "../store/editorStore";
import { useI18n } from "../../../i18n";

const PAGE_SIZE = 10;

/** 页面类型文案（与移动装修管理页页签口径一致） */
const PAGE_TYPE_LABELS: Record<number, string> = {
    1: "首页",
    2: "个人中心",
    3: "活动页",
    4: "分类页",
    5: "登录页",
    6: "搜索页",
    8: "商品详情页"
};

/** 更新时间格式化（毫秒时间戳，与装修管理页 formatDecorateTime 同口径） */
function formatDecorateTime(value: any): string {
    if (value === undefined || value === null || value === "") return "--";
    const raw = Number(value);
    const date = Number.isFinite(raw) ? new Date(raw > 1e12 ? raw : raw * 1000) : new Date(String(value));
    if (Number.isNaN(date.getTime())) return String(value);
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

interface Props {
    visible: boolean;
    decorateType: number;
    onClose: () => void;
}

export default function CopyPageDrawer({ visible, decorateType, onClose }: Props) {
    const { t } = useI18n();
    const [rows, setRows] = useState<any[]>([]);
    const [total, setTotal] = useState(0);
    const [page, setPage] = useState(1);
    const [loading, setLoading] = useState(false);
    const [copying, setCopying] = useState(false);

    const load = useCallback(
        (p: number) => {
            setLoading(true);
            apiGet<any>("/decorate/decorate/list", { decorateType, page: p, size: PAGE_SIZE })
                .then((result) => {
                    const list = Array.isArray(result) ? result : result?.records || result?.list || [];
                    setRows(list);
                    setTotal(typeof result?.total === "number" ? result.total : list.length);
                })
                .catch((error: any) => {
                    setRows([]);
                    setTotal(0);
                    Toast.error(error?.message || t("加载页面列表失败"));
                })
                .finally(() => setLoading(false));
        },
        [decorateType, t]
    );

    useEffect(() => {
        if (visible) {
            setPage(1);
            load(1);
        }
    }, [visible, load]);

    /** 复制：先确认（原版文案），再拉源页已发布数据套用（applyTemplate 置脏走既有保存流） */
    const doCopy = useCallback(
        (row: any) => {
            Modal.confirm({
                title: t("提示"),
                content: t("复制该模板将清空当前模板装修所有设置，是否继续操作？"),
                okText: t("确定"),
                cancelText: t("取消"),
                onOk: async () => {
                    setCopying(true);
                    try {
                        const result = await apiGet<any>("/decorate/decorate/detail", { id: row.decorateId, decorateType });
                        editorStore.applyTemplate({
                            moduleList: result?.data?.moduleList || [],
                            pageModule: result?.data?.pageModule || {}
                        });
                        onClose();
                    } catch (error: any) {
                        Toast.error(error?.message || t("复制失败"));
                    } finally {
                        setCopying(false);
                    }
                }
            });
        },
        [decorateType, onClose, t]
    );

    const columns = [
        {
            title: t("模板名称"),
            dataIndex: "decorateTitle",
            render: (_: any, row: any) => (
                <span>
                    {row.decorateTitle || t("未命名页面")}
                    {row.shopId ? <span style={{ color: "var(--semi-color-text-2)", marginLeft: 6 }}>{row.shopName || t("店铺页")}</span> : null}
                </span>
            )
        },
        {
            title: t("页面类型"),
            dataIndex: "pageType",
            width: 110,
            render: (v: any) => t(PAGE_TYPE_LABELS[Number(v)] || "页面")
        },
        {
            title: t("更新时间"),
            dataIndex: "updateTime",
            width: 170,
            render: (v: any) => formatDecorateTime(v)
        },
        {
            title: t("操作"),
            width: 90,
            render: (_: any, row: any) => (
                <Button theme="borderless" size="small" disabled={copying} onClick={() => doCopy(row)}>
                    {t("复制")}
                </Button>
            )
        }
    ];

    return (
        <SideSheet title={t("选择模板")} visible={visible} width={700} onCancel={onClose} placement="right">
            <Table columns={columns} dataSource={rows} rowKey="decorateId" loading={loading} pagination={false} />
            {total > PAGE_SIZE ? (
                <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 12 }}>
                    <Pagination
                        total={total}
                        currentPage={page}
                        pageSize={PAGE_SIZE}
                        onPageChange={(p) => {
                            setPage(p);
                            load(p);
                        }}
                    />
                </div>
            ) : null}
        </SideSheet>
    );
}
