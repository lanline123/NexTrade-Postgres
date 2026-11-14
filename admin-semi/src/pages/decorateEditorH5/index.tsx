/**
 * H5 装修编辑器（React 版，对齐 view/admin editor/Index.vue 移动端口径）
 *
 * 路由：?page=h5DecorateEditor&id={decorateId}&pageType={1首页|2会员|3活动|4分类|5登录|6搜索|8商品详情}
 * 链路：decorate/decorate/detail 加载 → store.initDecorate（url 回填 + 固定模块注入）→
 *       saveDraft/publish 保存（整体透传，契约红线）。
 */
import { useCallback, useEffect, useMemo, useState } from "react";
import { Button, Dropdown, Toast } from "@douyinfe/semi-ui";
import { IconChevronDown, IconExit, IconLanguage } from "@douyinfe/semi-icons";
import { apiGet, apiPost } from "../../api";
import { editorStore, useEditorState } from "./store/editorStore";
import ModuleToolbar from "./components/ModuleToolbar";
import Canvas from "./components/Canvas";
import SettingsPanel from "./components/SettingsPanel";
import { setStorageUrl } from "./shared/format";
import { ensureFixedModules } from "./contract/normalize";
import "./styles/css/module.css";
import "./styles/css/decorate.css";
import "./styles/editor.less";
import "./modules/index";

import { useI18n } from '../../i18n';
const DEFAULT_DECORATE_TITLE = "未命名页面";

interface Props {
    decorateId?: number;
    pageType?: number;
    decorateType?: number;
    onExit?: () => void;
}

export default function H5DecorateEditor({ decorateId: idProp, pageType: pageTypeProp, decorateType = 1, onExit }: Props) {
    const { t } = useI18n();
    const s = useEditorState();

    const query = useMemo(() => new URLSearchParams(window.location.search), []);
    const initialId = idProp ?? Number(query.get("id") || 0);
    const pageType = pageTypeProp ?? Number(query.get("pageType") || 1);
    const isAddMode = !initialId;

    const [decorateTitleDraft, setDecorateTitleDraft] = useState("");
    const [saving, setSaving] = useState(false);
    /** 顶栏标题点击编辑态（原版 topbar-now-txt-title ↔ input 切换） */
    const [editingTitle, setEditingTitle] = useState(false);
    /** 预览态：隐藏编辑外壳（模块库/设置面板/画布操作件），只看画布 */
    const [preview, setPreview] = useState(false);

    // 初始化 store（含固定模块注入，空白页也有默认结构）
    useEffect(() => {
        editorStore.reset();
        editorStore.initDecorate({
            decorateId: initialId,
            decorateType: decorateType as any,
            pageType,
            decorateTitle: query.get("title") || DEFAULT_DECORATE_TITLE,
            moduleList: []
        });
        setDecorateTitleDraft(query.get("title") || DEFAULT_DECORATE_TITLE);
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    // 注入云存储 URL（相对路径图片回显）
    useEffect(() => {
        apiGet<any>("/setting/config/getAdminBase")
            .then((result) => {
                const url = result?.storageUrl || result?.storageLocalUrl || "";
                if (url) setStorageUrl(url);
            })
            .catch(() => {});
    }, []);

    // 加载装修详情
    useEffect(() => {
        if (isAddMode) return;
        apiGet<any>("/decorate/decorate/detail", { id: initialId, decorateType })
            .then((result) => {
                if (result == null) return;
                editorStore.initDecorate({
                    decorateId: result.decorateId || initialId,
                    decorateType: decorateType as any,
                    pageType,
                    decorateTitle: result.decorateTitle || query.get("title") || "",
                    pageModule: result.data?.pageModule || {},
                    moduleList: result.data?.moduleList || [],
                    hasDraft: !!result.draftData
                });
                setDecorateTitleDraft(result.decorateTitle || "");
            })
            .catch((error) => Toast.error(error.message || t("加载装修详情失败")));
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [initialId]);

    /** 延迟创建：新建场景首次保存时校验名称并创建记录 */
    const ensureCreated = async (): Promise<boolean> => {
        if (editorStore.getState().decorateId) return true;
        const title = (editorStore.getState().decorateTitle || "").trim();
        if (!title || title === DEFAULT_DECORATE_TITLE) {
            Toast.warning(t("请先修改页面名称"));
            return false;
        }
        try {
            const result = await apiPost<any>("/decorate/decorate/create", {
                decorateType,
                pageType,
                decorateTitle: title
            });
            const newId = result?.decorateId ?? result?.id ?? 0;
            if (!newId) {
                Toast.error(t("创建失败"));
                return false;
            }
            const url = new URL(window.location.href);
            url.searchParams.set("id", String(newId));
            window.history.replaceState(null, "", url.toString());
            return true;
        } catch (error: any) {
            Toast.error(error.message || t("创建失败"));
            return false;
        }
    };

    const buildSavePayload = (data: Record<string, any>) => ({
        id: editorStore.getState().decorateId,
        decorateType,
        pageType,
        data
    });

    const onSavetoDraft = useCallback(async () => {
        if (saving) return;
        if (!(await ensureCreated())) return;
        setSaving(true);
        try {
            await apiPost("/decorate/decorate/saveDraft", buildSavePayload(editorStore.getSaveData()));
            Toast.success(t("操作成功"));
            editorStore.markAsSaved();
        } catch (error: any) {
            Toast.error(error.message || t("保存草稿失败"));
        } finally {
            setSaving(false);
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [saving]);

    const onPublish = useCallback(async () => {
        if (saving) return;
        if (!(await ensureCreated())) return;
        setSaving(true);
        try {
            const saveData = editorStore.getSaveData();
            // 个人中心页：pageModule 标记为新版，供前台识别（与 view/admin 一致）
            if (pageType === 2) {
                saveData.pageModule = { ...saveData.pageModule, homeStyleVersion: "new" };
            }
            await apiPost("/decorate/decorate/publish", buildSavePayload(saveData));
            Toast.success(t("操作成功"));
            editorStore.markAsSaved();
        } catch (error: any) {
            Toast.error(error.message || t("发布失败"));
        } finally {
            setSaving(false);
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [saving]);

    const onContinueDraft = useCallback(async () => {
        try {
            const result = await apiGet<any>("/decorate/decorate/loadDraftData", { id: editorStore.getState().decorateId });
            editorStore.restoreFromDraft({
                pageModule: result?.pageModule ?? editorStore.getState().pageModule,
                moduleList: result?.moduleList ?? []
            });
        } catch (error: any) {
            Toast.error(error.message || t("恢复草稿失败"));
        }
    }, []);

    const commitTitle = async () => {
        const val = (decorateTitleDraft || "").trim();
        const oldVal = (editorStore.getState().decorateTitle || "").trim();
        if (!val) {
            Toast.warning(t("页面名称不能为空"));
            return;
        }
        if (val === oldVal) return;
        if (!editorStore.getState().decorateId) {
            editorStore.updatePageModule({});
            // 未创建记录：仅本地暂存，首次保存时随 create 提交
            editorStore.initDecorate({
                decorateId: editorStore.getState().decorateId,
                decorateType: decorateType as any,
                pageType,
                decorateTitle: val,
                pageModule: editorStore.getState().pageModule,
                moduleList: editorStore.getState().moduleList,
                hasDraft: editorStore.getState().hasDraft
            });
            return;
        }
        try {
            await apiPost("/decorate/decorate/updateField", { id: editorStore.getState().decorateId, field: "decorateTitle", val });
            Toast.success(t("页面名称修改成功"));
        } catch (error: any) {
            Toast.error(error.message || t("修改失败"));
        }
    };

    // 会员中心页固定模块：此处已在 initDecorate/restoreFromDraft 内注入

    const handleExit = onExit ?? (() => (window.location.href = window.location.origin));

    return (
        <div className="h5-editor-root">
            {/* 顶栏（原版 .decorate-topbar-warp：fixed 54px 深色 #202033） */}
            <div className="decorate-topbar-warp">
                <div className="decorate-topbar-con">
                    <div className="topbar-left">
                        <div className="topbar-logo" />
                        <div className="topbar-now-txt h5-editor-exit" onClick={handleExit}>
                            <IconExit style={{ marginRight: 8 }} />
                            {t('退出编辑')}
                        </div>
                        <div className="topbar-now-txt topbar-now-txt-edit">
                            <span className="tit">{t('正在装修：')}</span>
                            {editingTitle ? (
                                <input
                                    className="topbar-now-txt-input"
                                    value={decorateTitleDraft}
                                    maxLength={50}
                                    placeholder={t('请输入页面名称')}
                                    autoFocus
                                    onChange={(e) => setDecorateTitleDraft(e.target.value)}
                                    onBlur={() => {
                                        setEditingTitle(false);
                                        void commitTitle();
                                    }}
                                    onKeyDown={(e) => {
                                        if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                                        if (e.key === "Escape") {
                                            setDecorateTitleDraft(s.decorateTitle || "");
                                            setEditingTitle(false);
                                        }
                                    }}
                                />
                            ) : (
                                <span
                                    className="topbar-now-txt-title"
                                    onClick={() => {
                                        setDecorateTitleDraft(s.decorateTitle || decorateTitleDraft || "");
                                        setEditingTitle(true);
                                    }}
                                >
                                    {decorateTitleDraft || s.decorateTitle || DEFAULT_DECORATE_TITLE}
                                </span>
                            )}
                        </div>
                        {/* 语言下拉：原版为海外版多语言装修入口，此处仅视觉占位（项为空操作） */}
                        <Dropdown
                            trigger="click"
                            position="bottomLeft"
                            render={
                                <Dropdown.Menu>
                                    <Dropdown.Item active>{t('简体中文')}</Dropdown.Item>
                                    <Dropdown.Item>{t('English')}</Dropdown.Item>
                                </Dropdown.Menu>
                            }
                        >
                            <div className="lang-btn">
                                <IconLanguage style={{ marginRight: 6 }} />
                                {t('语言：简体中文')}
                                <IconChevronDown style={{ marginLeft: 6 }} />
                            </div>
                        </Dropdown>
                    </div>
                    <div className="topbar-right">
                        <div className="topbar-btn-group">
                            <button type="button" className="dec-topbar-btn" onClick={() => setPreview((v) => !v)}>
                                {preview ? t('退出预览') : t('预览')}
                            </button>
                            <button type="button" className="dec-topbar-btn" disabled={saving} onClick={onSavetoDraft}>
                                {t('存至草稿')}
                            </button>
                            <button type="button" className="dec-topbar-btn-primary" disabled={saving} onClick={onPublish}>
                                {t('保存并发布')}
                            </button>
                        </div>
                        {/* 更多菜单：原版海外版含复制/删除页面，此处仅占位（不放真实删除） */}
                        <Dropdown
                            trigger="click"
                            position="bottomRight"
                            render={
                                <Dropdown.Menu>
                                    <Dropdown.Item>{t('从其他页面复制')}</Dropdown.Item>
                                </Dropdown.Menu>
                            }
                        >
                            <div className="lang-btn topbar-more-btn">
                                {t('更多')}
                                <IconChevronDown style={{ marginLeft: 6 }} />
                            </div>
                        </Dropdown>
                    </div>
                </div>
            </div>

            {!preview ? <ModuleToolbar pageType={pageType} isShop={localStorage.getItem("adminType") === "shop"} /> : null}

            <div className={`decorate-page-wrap${preview ? " is-preview" : ""}`}>
                {s.hasDraft && !preview ? (
                    <div className="decorate-page-alert">
                        <span className="decorate-page-alert-txt">{t('该页面有一条草稿记录，您可以还原草稿继续编辑。')}</span>
                        <Button size="small" onClick={onContinueDraft}>{t('继续编辑')}</Button>
                    </div>
                ) : null}
                <Canvas preview={preview} />
            </div>

            {!preview ? <SettingsPanel /> : null}
        </div>
    );
}

// 保留引用避免 tree-shake（会员页固定模块注入依赖 ensureFixedModules 链路）
void ensureFixedModules;
