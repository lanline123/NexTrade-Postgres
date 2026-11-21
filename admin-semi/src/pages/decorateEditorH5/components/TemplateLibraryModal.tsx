/**
 * 新建页模板库（原版有·我方补齐）：H5 编辑器新建模式自动弹出，选空白或
 * 内置模板作为装修起点；与「从其他页面复制」的边界——复制取的是本站已有
 * 页面的已发布数据，模板库给的是与本站页面无关的预置整页起点。
 *
 * 原版口径（v5.8.61 实证）：
 * - editor/Index.vue：新建模式（act=add 或无 id）进编辑器自动挂 DialogForm
 *   （1192px、无头无脚）→ decorate/mobileDecorate/TemplateLibrary，参数
 *   { decorateType, pageType, scene: 'editor' }；
 * - TemplateLibrary.vue：头部模板编码搜索；行业 / 节日 / 颜色三行筛选
 *   （官网分类维度 6/19/28）；网格第一张恒为「新建空白模板」卡，模板卡
 *   悬浮遮罩出「免费使用」；数据经后端 decorate/template/* 转发官网模板
 *   市场（授权码取数），详情 json 剥外层取 { pageModule, moduleList }；
 * - 编辑器回调 onTemplatePicked：useTemplate 时走与复制同一归一化链填充
 *   （url 回填 + 固定模块注入 + pageModule 合并），blank 时保持空白页；
 *   创建延迟到首次保存/预览（与我方 ensureCreated 延迟创建同构）。
 *
 * 我方适配（见 templateLibrary/templates.ts 文件头）：模板数据为内置定义
 * （模块 defaults 现组装），筛选与搜索在前端本地完成（模板量小，无分页；
 * 原版市场海量才走后端分页）；弹窗形态以 Semi Modal 承载原版布局。
 * 关闭弹窗（X / 选空白）即按空白页继续，不再自动重弹（与原版一致）。
 */
import { useMemo, useState } from "react";
import { Button, Input, Modal, Toast } from "@douyinfe/semi-ui";
import { IconClose, IconSearch } from "@douyinfe/semi-icons";
import { editorStore } from "../store/editorStore";
import { fillDemoProducts } from "../../../components/demoProductFill";
import { useI18n } from "../../../i18n";
import {
    buildTemplateData,
    templateCover,
    templateScopeForPageType,
    templatesForScope,
    type BuiltinTemplate
} from "./templateLibrary/templates";
import "./TemplateLibraryModal.css";

interface Props {
    visible: boolean;
    pageType: number;
    onClose: () => void;
}

const ALL = "__all__";

export default function TemplateLibraryModal({ visible, pageType, onClose }: Props) {
    const { t } = useI18n();
    const [keyword, setKeyword] = useState("");
    const [industry, setIndustry] = useState(ALL);
    const [festival, setFestival] = useState(ALL);
    const [color, setColor] = useState(ALL);

    const scoped = useMemo(() => templatesForScope(templateScopeForPageType(pageType)), [pageType]);

    /** 筛选选项集：由当前范围模板派生（原版为官网分类树，我方内置小集合，形态一致） */
    const industries = useMemo(() => Array.from(new Set(scoped.map((tpl) => tpl.industry))), [scoped]);
    const festivals = useMemo(() => Array.from(new Set(scoped.map((tpl) => tpl.festival))), [scoped]);
    const colors = useMemo(() => {
        const seen = new Map<string, string>();
        scoped.forEach((tpl) => {
            if (!seen.has(tpl.color)) seen.set(tpl.color, tpl.colorName);
        });
        return Array.from(seen.entries()).map(([hex, name]) => ({ hex, name }));
    }, [scoped]);

    const filtered = useMemo(() => {
        const kw = keyword.trim().toLowerCase();
        return scoped.filter((tpl) => {
            if (industry !== ALL && tpl.industry !== industry) return false;
            if (festival !== ALL && tpl.festival !== festival) return false;
            if (color !== ALL && tpl.color !== color) return false;
            if (kw && !tpl.name.toLowerCase().includes(kw) && !tpl.sn.toLowerCase().includes(kw)) return false;
            return true;
        });
    }, [scoped, keyword, industry, festival, color]);

    /** 封面：模板自身首图（组装派生，见 templates.templateCover），按模板缓存 */
    const covers = useMemo(() => {
        const map = new Map<string, string>();
        scoped.forEach((tpl) => map.set(tpl.sn, templateCover(tpl)));
        return map;
    }, [scoped]);

    /** 使用模板：组装整页 → 各模块先过演示商品真实化（用户口径 2026-11-21，
     * 与工具栏添加同机制）→ applyTemplate（与复制同链：url 回填 + 固定模块注入
     * + pageModule 合并，整页可撤销）→ 置脏走既有保存流 */
    const onUseTemplate = async (tpl: BuiltinTemplate) => {
        const data = buildTemplateData(tpl);
        for (const m of data.moduleList ?? []) {
            await fillDemoProducts((m.module ?? {}) as Record<string, any>);
        }
        editorStore.applyTemplate(data);
        Toast.success(t("已套用模板，可继续编辑后保存"));
        onClose();
    };

    return (
        <Modal
            visible={visible}
            onCancel={onClose}
            width={1192}
            footer={null}
            title={null}
            closable={false}
            bodyStyle={{ padding: "16px 20px 12px" }}
        >
            <div className="tpl-lib">
                <div className="tpl-lib-header">
                    <Input
                        className="tpl-lib-search"
                        value={keyword}
                        placeholder={t("输入模板编码或名称")}
                        prefix={<IconSearch />}
                        showClear
                        onChange={(v) => setKeyword(v)}
                    />
                    <button type="button" className="tpl-lib-close" onClick={onClose} aria-label={t("关闭")}>
                        <IconClose size="large" />
                    </button>
                </div>

                <div className="tpl-lib-filters">
                    <div className="tpl-lib-filter-row">
                        <span className="tpl-lib-filter-label">{t("行业：")}</span>
                        <div className="tpl-lib-filter-options">
                            <span className={`tpl-lib-tag${industry === ALL ? " active" : ""}`} onClick={() => setIndustry(ALL)}>
                                {t("全部")}
                            </span>
                            {industries.map((name) => (
                                <span key={name} className={`tpl-lib-tag${industry === name ? " active" : ""}`} onClick={() => setIndustry(name)}>
                                    {t(name)}
                                </span>
                            ))}
                        </div>
                    </div>
                    <div className="tpl-lib-filter-row">
                        <span className="tpl-lib-filter-label">{t("节日：")}</span>
                        <div className="tpl-lib-filter-options">
                            <span className={`tpl-lib-tag${festival === ALL ? " active" : ""}`} onClick={() => setFestival(ALL)}>
                                {t("全部")}
                            </span>
                            {festivals.map((name) => (
                                <span key={name} className={`tpl-lib-tag${festival === name ? " active" : ""}`} onClick={() => setFestival(name)}>
                                    {t(name)}
                                </span>
                            ))}
                        </div>
                    </div>
                    <div className="tpl-lib-filter-row">
                        <span className="tpl-lib-filter-label">{t("颜色：")}</span>
                        <div className="tpl-lib-filter-options">
                            <span
                                className={`tpl-lib-color-dot is-all${color === ALL ? " active" : ""}`}
                                title={t("全部")}
                                onClick={() => setColor(ALL)}
                            >
                                {t("全")}
                            </span>
                            {colors.map(({ hex, name }) => (
                                <span
                                    key={hex}
                                    className={`tpl-lib-color-dot${color === hex ? " active" : ""}`}
                                    style={{ backgroundColor: hex }}
                                    title={t(name)}
                                    onClick={() => setColor(hex)}
                                >
                                    {color === hex ? "✓" : ""}
                                </span>
                            ))}
                        </div>
                    </div>
                </div>

                <div className="tpl-lib-body">
                    <div className="tpl-lib-grid">
                        {/* 第一张恒为新建空白卡：仅关弹窗保持空白页（固定模块已由 initDecorate 注入），不创建记录 */}
                        <div className="tpl-lib-card tpl-lib-card-blank" onClick={onClose}>
                            <span className="tpl-lib-blank-plus">+</span>
                            <span className="tpl-lib-blank-text">{t("新建空白模板")}</span>
                        </div>
                        {filtered.map((tpl) => {
                            const cover = covers.get(tpl.sn) || "";
                            return (
                                <div key={tpl.sn} className="tpl-lib-card">
                                    <div className="tpl-lib-thumb">
                                        {cover ? (
                                            <img src={cover} alt={tpl.name} />
                                        ) : (
                                            <div className="tpl-lib-thumb-placeholder">{t(tpl.name)}</div>
                                        )}
                                        <div className="tpl-lib-mask">
                                            <Button theme="solid" type="primary" onClick={() => onUseTemplate(tpl)}>
                                                {t("免费使用")}
                                            </Button>
                                        </div>
                                    </div>
                                    <div className="tpl-lib-card-meta">
                                        <div className="tpl-lib-card-name">{t(tpl.name)}</div>
                                        <div className="tpl-lib-card-sub">
                                            <span className="tpl-lib-card-color" style={{ backgroundColor: tpl.color }} />
                                            <span>
                                                {t(tpl.industry)} · {t(tpl.festival)} · {tpl.sn}
                                            </span>
                                        </div>
                                    </div>
                                </div>
                            );
                        })}
                        {filtered.length === 0 ? <div className="tpl-lib-empty">{t("没有符合条件的模板")}</div> : null}
                    </div>
                </div>

                <div className="tpl-lib-footer">{t("共 {count} 个模板", { count: filtered.length })}</div>
            </div>
        </Modal>
    );
}
