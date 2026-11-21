/**
 * 编辑器顶栏语言切换（原版有 · 我方未迁补齐，2026-11-21 编辑器登记项 ③）。
 *
 * 原版口径（view/admin decorate/editor/Index.vue 的 changeLanguage）：
 * - 语言列表 = 语言管理中启用的语言（GET /lang/locales/list?isEnabled=1），
 *   默认语言带「（默认）」标记，当前语言高亮；
 * - 一个装修页 = 顶层记录（默认语言）+ 每语言至多一条子记录（整页数据副本，
 *   parent_id + locale_id 定位）；切换 = 整页跳转：默认语言跳顶层页 id，
 *   已有子页跳子 id，未建子页跳 id=0 + parentId + localeId + 父标题，
 *   编辑器以空骨架起步、首次保存时由后端按 parentId+localeId 建子行。
 *
 * 语言列表为空/请求失败时不渲染（不留死占位）。切换为整页导航，
 * H5 编辑器的 beforeunload 脏拦截会自动兜底未保存修改。
 */
import { useEffect, useState } from "react";
import { Dropdown } from "@douyinfe/semi-ui";
import { IconChevronDown, IconLanguage } from "@douyinfe/semi-icons";
import { apiGet } from "../../api";
import { useI18n } from "../../i18n";

export interface DecorateLocaleChild {
    decorateId: number;
    localeId: number;
    language?: string;
}

interface LocaleItem {
    id: number;
    language: string;
    isDefault: number;
}

interface Props {
    /** 顶层页（默认语言版本）id；0/未保存页不渲染 */
    parentId: number;
    /** 当前会话语言：0 = 默认语言会话；>0 = 该语言子页会话 */
    currentLocaleId: number;
    /** 父页标题（未建子页跳转时透传给编辑器作标题） */
    parentTitle: string;
    pageType: number;
    /** 父页已有语言子页（detail.children） */
    children?: DecorateLocaleChild[];
}

export default function EditorLocaleSwitch({
    parentId,
    currentLocaleId,
    parentTitle,
    pageType,
    children = []
}: Props) {
    const { t } = useI18n();
    const [locales, setLocales] = useState<LocaleItem[]>([]);

    useEffect(() => {
        apiGet<any>("/lang/locales/list", { isEnabled: 1, size: 100, page: 1 })
            .then((res) => {
                const rows: LocaleItem[] = (res?.records || []).map((r: any) => ({
                    id: Number(r.id),
                    language: r.language || r.localeCode || String(r.id),
                    isDefault: Number(r.isDefault) || 0
                }));
                setLocales(rows);
            })
            .catch(() => setLocales([]));
    }, []);

    if (!parentId || locales.length === 0) return null;

    const defaultLocale = locales.find((l) => l.isDefault === 1);
    const current =
        currentLocaleId > 0
            ? locales.find((l) => l.id === currentLocaleId)
            : defaultLocale;
    const currentLabel = current?.language || t("简体中文");

    const navigate = (target: { id: number; parentId?: number; localeId?: number }) => {
        const url = new URL(window.location.href);
        url.searchParams.set("id", String(target.id));
        if (target.parentId) url.searchParams.set("parentId", String(target.parentId));
        else url.searchParams.delete("parentId");
        if (target.localeId) url.searchParams.set("localeId", String(target.localeId));
        else url.searchParams.delete("localeId");
        url.searchParams.set("pageType", String(pageType));
        if (parentTitle) url.searchParams.set("title", parentTitle);
        window.location.href = url.toString();
    };

    const onSelect = (locale: LocaleItem) => {
        if (locale.isDefault === 1) {
            if (currentLocaleId === 0) return;
            navigate({ id: parentId });
            return;
        }
        if (locale.id === currentLocaleId) return;
        const child = children.find((c) => Number(c.localeId) === locale.id);
        if (child && Number(child.decorateId) > 0) {
            navigate({ id: Number(child.decorateId), parentId, localeId: locale.id });
        } else {
            navigate({ id: 0, parentId, localeId: locale.id });
        }
    };

    const isActive = (locale: LocaleItem) =>
        currentLocaleId > 0 ? locale.id === currentLocaleId : locale.isDefault === 1;

    return (
        <Dropdown
            trigger="click"
            position="bottomLeft"
            render={
                <Dropdown.Menu>
                    {locales.map((locale) => (
                        <Dropdown.Item
                            key={locale.id}
                            active={isActive(locale)}
                            onClick={() => onSelect(locale)}
                        >
                            {locale.language}
                            {locale.isDefault === 1 ? `（${t("默认")}）` : ""}
                        </Dropdown.Item>
                    ))}
                </Dropdown.Menu>
            }
        >
            {/* .lang-btn 样式在 H5 编辑器作用域内有定义；内联同值保证 PC 编辑器
                （白色顶栏、无该样式作用域）下视觉一致（原版 .lang-btn 深色块） */}
            <div
                className="lang-btn"
                style={{
                    cursor: "pointer",
                    color: "#fff",
                    backgroundColor: "#444454",
                    padding: "8px 15px",
                    display: "flex",
                    alignItems: "center",
                    borderRadius: 2,
                    fontSize: 14,
                    lineHeight: 1,
                    userSelect: "none"
                }}
            >
                <IconLanguage style={{ marginRight: 6 }} />
                {t("语言：{languageName}", { languageName: currentLabel })}
                <IconChevronDown style={{ marginLeft: 6 }} />
            </div>
        </Dropdown>
    );
}
