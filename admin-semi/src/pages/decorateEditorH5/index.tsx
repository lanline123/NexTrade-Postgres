/**
 * H5 装修编辑器（React 版，对齐 view/admin editor/Index.vue 移动端口径）
 *
 * 路由：?page=h5DecorateEditor&id={decorateId}&pageType={1首页|2会员|3活动|4分类|5登录|6搜索|8商品详情}
 * 链路：decorate/decorate/detail 加载 → store.initDecorate（url 回填 + 固定模块注入）→
 *       saveDraft/publish 保存（整体透传，契约红线）。
 * 语言子页会话（2026-11-21 补齐，原版有）：URL 另带 parentId+localeId 时编辑顶层页的
 * 语言子记录（顶栏 EditorLocaleSwitch 整页跳转切换）；id=0 = 子页未建行，保存按
 * parentId+localeId 建行并回填子 id，标题继承父页只读。
 *
 * 2026-11-19 入口加载门（用户拍板）：进入编辑器先出 DecorateLoading 加载动画，
 * 详情请求与 iframe 嵌入页改为并行启动（此前是串行：详情回来才挂载画布、才开始
 * 加载 iframe），「数据就绪 且 iframe 首帧已渲染（embedReadiness 的当前纪元
 * layout 回传）」双条件满足才撤门进入编辑；8 秒超时兜底直接进入、绝不锁人。
 * 加载门只在首次进入把守一次：模式切换、草稿恢复、失败重试都不再过门。
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Button, Dropdown, Modal, Spin, Toast } from "@douyinfe/semi-ui";
import { IconChevronDown, IconExit, IconRedo, IconUndo } from "@douyinfe/semi-icons";
import { apiGet, apiPost } from "../../api";
import TemplateImportModal, { type TemplateImportedInfo } from "../../components/TemplateImportModal";
import { editorStore, useEditorState } from "./store/editorStore";
import ModuleToolbar from "./components/ModuleToolbar";
import Canvas from "./components/Canvas";
import SettingsPanel from "./components/SettingsPanel";
import CopyPageDrawer from "./components/CopyPageDrawer";
import TemplateLibraryModal from "./components/TemplateLibraryModal";
import EditorShell from "../../components/decorate/EditorShell";
import EditorLocaleSwitch from "../../components/decorate/EditorLocaleSwitch";
import DecorateLoading from "../components/DecorateLoading";
import { getStoredRenderMode } from "./components/canvasBridge";
import { beginEmbedDataEpoch, resetEmbedReadiness, useEmbedReadiness } from "./components/embedReadiness";
import { setStorageUrl } from "./shared/format";
import { ensureFixedModules } from "./contract/normalize";
import { createNewPageModule } from "./contract/sharedDefaults";
import "./styles/css/module.css";
import "./styles/css/decorate.css";
import "../../components/decorate/editor-shell.css";
import "../decorate-editor.css";
import "./styles/editor.less";
import "./modules/index";

import { useI18n } from '../../i18n';
const DEFAULT_DECORATE_TITLE = "未命名页面";
/** 加载门超时兜底（毫秒）：到期未双就绪也撤门进入，绝不把用户锁在加载屏 */
const GATE_TIMEOUT_MS = 8000;

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
    /** 语言子页会话（原版有 · 2026-11-21 补齐）：URL 带 parentId+localeId 时编辑的是
     * 顶层页的语言子记录；id=0 表示子页尚未建行（首次保存由后端按
     * parentId+localeId 建行，禁止走 create 建顶层记录）。 */
    const queryParentId = Number(query.get("parentId") || 0);
    const queryLocaleId = Number(query.get("localeId") || 0);
    const isLocaleChild = queryParentId > 0 && queryLocaleId > 0;
    /** 顶层页 id（语言切换的目标解析基准） */
    const topPageId = isLocaleChild ? queryParentId : initialId;

    const [decorateTitleDraft, setDecorateTitleDraft] = useState("");
    const [saving, setSaving] = useState(false);
    /** 顶栏标题点击编辑态（原版 topbar-now-txt-title ↔ input 切换） */
    const [editingTitle, setEditingTitle] = useState(false);
    /** 预览态：隐藏编辑外壳（模块库/设置面板/画布操作件），只看画布 */
    const [preview, setPreview] = useState(false);
    /** 「从模板导入」弹窗（模板平台 tpl.recy.cc，我方新增）：导入成功后跳到新页继续编辑 */
    const [tplImportVisible, setTplImportVisible] = useState(false);
    /** 「从其他页面复制」抽屉开关 */
    const [copyVisible, setCopyVisible] = useState(false);
    /** 新建页模板库弹窗（原版有·我方补齐）：新建模式自动弹出一次，选空白/关闭
     * 后不再重弹（与原版一致，编辑器内无重开入口） */
    const [tplVisible, setTplVisible] = useState(false);
    const tplShownRef = useRef(false);
    /** 详情加载态（2026-11-18 立规、2026-11-19 加载门沿用）：加载失败（接口/H5
     * 服务未部署或不可用）出提示卡 + 重试，画布不落到假空页面上（防误存覆盖
     * 真实数据）；加载中则由入口加载门覆盖（见文件头），仅超时兜底后仍在途
     * 才回落到画布区状态卡 */
    const [detailLoading, setDetailLoading] = useState(!isAddMode || isLocaleChild);
    const [detailError, setDetailError] = useState("");
    /** 语言切换的目标数据：父页已有子页清单 + 父页标题（子页标题继承父页） */
    const [localeChildren, setLocaleChildren] = useState<Array<{ decorateId: number; localeId: number; language?: string }>>([]);
    const [parentTitle, setParentTitle] = useState("");
    /** 加载门：数据就绪（详情成功写入 store；新建模式无详情请求、天然就绪；
     * 语言子页会话须等详情回来判定子页是否已建行，不算天然就绪） */
    const [dataReady, setDataReady] = useState(isAddMode && !isLocaleChild);
    /** 加载门撤下闩锁：条件满足或超时撤下后，本次会话不再把守 */
    const [gateLifted, setGateLifted] = useState(false);
    const embed = useEmbedReadiness();
    /** 进入时的渲染模式（仅用于加载门判定：兼容模式本地渲染、无需等 iframe 首帧） */
    const initialModeRef = useRef(getStoredRenderMode());

    // 初始化 store（含固定模块注入，空白页也有默认结构）
    useEffect(() => {
        editorStore.reset();
        editorStore.initDecorate({
            decorateId: initialId,
            decorateType: decorateType as any,
            pageType,
            decorateTitle: query.get("title") || DEFAULT_DECORATE_TITLE,
            // 新建模式在此播种页面背景默认值 #f7f8fa（用户 2026-11-21 指令）：
            // 已有页随后由详情 initDecorate 以存量数据整体覆盖，本播种不触及存量页
            pageModule: isAddMode ? createNewPageModule() : undefined,
            moduleList: [],
            localeId: isLocaleChild ? queryLocaleId : 0,
            parentId: isLocaleChild ? queryParentId : 0,
            isDefault: !isLocaleChild
        });
        setDecorateTitleDraft(query.get("title") || DEFAULT_DECORATE_TITLE);
        // 嵌入页就绪状态机复位并开启首个数据纪元（初始结构一轮；详情成功后再推进一格）
        resetEmbedReadiness();
        beginEmbedDataEpoch();
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

    // 加载装修详情（失败保留错误态供提示卡重试，不再只弹 Toast 后留假空画布）
    const loadDetail = useCallback(() => {
        if (isAddMode && !isLocaleChild) return;
        setDetailLoading(true);
        setDetailError("");
        const detailParams = isLocaleChild
            ? { id: initialId, localeId: queryLocaleId, parentId: queryParentId, decorateType }
            : { id: initialId, decorateType };
        apiGet<any>("/decorate/decorate/detail", detailParams)
            .then(async (result) => {
                if (result == null) {
                    setDetailError(t("未获取到页面数据"));
                    return;
                }
                if (isLocaleChild) {
                    // 子页会话：另拉父页详情取 children（切换目标解析）与父标题
                    // （子页标题继承父页）；父页拉取失败不阻塞子页编辑本身
                    let parent: any = null;
                    try {
                        parent = await apiGet<any>("/decorate/decorate/detail", { id: queryParentId, decorateType });
                    } catch {
                        parent = null;
                    }
                    const pTitle = parent?.decorateTitle || query.get("title") || "";
                    setParentTitle(pTitle);
                    setLocaleChildren(parent?.children || []);
                    // 子页未建行时后端返回空 VO（decorateId=0）：以空骨架起步
                    // （原版口径，不复制父页内容），首次保存按 parentId+localeId 建行
                    const childExists = Number(result.decorateId || 0) > 0;
                    editorStore.initDecorate({
                        decorateId: childExists ? result.decorateId : 0,
                        decorateType: decorateType as any,
                        pageType,
                        decorateTitle: pTitle,
                        // 子页未建行 = 新建页面：骨架同样播种背景默认值（不复制父页内容，原口径不变）
                        pageModule: childExists ? result.data?.pageModule || {} : createNewPageModule(),
                        moduleList: childExists ? result.data?.moduleList || [] : [],
                        hasDraft: childExists ? !!result.draftData : false,
                        localeId: queryLocaleId,
                        parentId: queryParentId,
                        isDefault: false,
                        shopId: parent?.shopId || 0
                    });
                    setDecorateTitleDraft(pTitle);
                } else {
                    editorStore.initDecorate({
                        decorateId: result.decorateId || initialId,
                        decorateType: decorateType as any,
                        pageType,
                        decorateTitle: result.decorateTitle || query.get("title") || "",
                        pageModule: result.data?.pageModule || {},
                        moduleList: result.data?.moduleList || [],
                        hasDraft: !!result.draftData,
                        // 店铺模板页的所属店铺：真实渲染 iframe 据此拉真实店铺详情喂 shopInfo 模块
                        shopId: result.shopId || 0
                    });
                    setDecorateTitleDraft(result.decorateTitle || "");
                    setParentTitle(result.decorateTitle || "");
                    setLocaleChildren(result.children || []);
                }
                // 真实数据已写入 store：推进数据纪元（本纪元的 layout 回传才算
                // 首帧，防挂载初期空结构那一轮误开门）并标记数据就绪
                beginEmbedDataEpoch();
                setDataReady(true);
            })
            .catch((error) => {
                setDetailError(error.message || t("加载装修详情失败"));
            })
            .finally(() => setDetailLoading(false));
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [initialId]);

    useEffect(() => {
        loadDetail();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [initialId]);

    // 加载门撤下条件：数据就绪 且（兼容模式 || iframe 首帧已渲染）
    const frameReady = initialModeRef.current === "compat" || embed.firstFrameRendered;
    useEffect(() => {
        if (dataReady && frameReady) setGateLifted(true);
    }, [dataReady, frameReady]);

    // 加载门超时兜底：到期未双就绪也撤门（画布以自身加载提示继续完成）
    useEffect(() => {
        const timer = window.setTimeout(() => setGateLifted(true), GATE_TIMEOUT_MS);
        return () => window.clearTimeout(timer);
    }, []);

    // 新建模式：加载门撤下后自动弹出模板库（原版进编辑器即弹；我方等门撤下
    // 避免弹窗被加载屏覆盖）。仅弹一次；选模板走 applyTemplate、选空白/关闭
    // 保持空白页，创建仍延迟到首次保存（ensureCreated）。
    useEffect(() => {
        if (isAddMode && gateLifted && !tplShownRef.current) {
            tplShownRef.current = true;
            setTplVisible(true);
        }
    }, [isAddMode, gateLifted]);

    /** 延迟创建：新建场景首次保存时校验名称并创建记录 */
    const ensureCreated = async (): Promise<boolean> => {
        // 语言子页会话：子行由 saveDraft/publish 按 parentId+localeId 建行，
        // 绝不走 create（那会建出顶层记录，原版同禁）
        if (isLocaleChild) return true;
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

    const buildSavePayload = (data: Record<string, any>) => {
        const payload: Record<string, any> = {
            id: editorStore.getState().decorateId,
            decorateType,
            pageType,
            data
        };
        // 语言子页会话：带 parentId+localeId，后端据此 upsert 子行（原版保存口径）
        if (isLocaleChild) {
            payload.parentId = queryParentId;
            payload.localeId = queryLocaleId;
        }
        return payload;
    };

    /** 语言子页首次保存后回填子记录 id（原版 syncLocaleChildId）：
     * 按 parentId+localeId 回查详情，拿到子 id 后同步 store 与 URL */
    const syncLocaleChildId = async () => {
        if (!isLocaleChild) return;
        if (Number(editorStore.getState().decorateId) > 0) return;
        try {
            const result = await apiGet<any>("/decorate/decorate/detail", {
                id: 0,
                localeId: queryLocaleId,
                parentId: queryParentId,
                decorateType
            });
            const childId = Number(result?.decorateId || 0);
            if (childId > 0) {
                editorStore.setDecorateId(childId);
                const url = new URL(window.location.href);
                url.searchParams.set("id", String(childId));
                window.history.replaceState(null, "", url.toString());
                setLocaleChildren((prev) =>
                    prev.some((c) => Number(c.localeId) === queryLocaleId)
                        ? prev
                        : [...prev, { decorateId: childId, localeId: queryLocaleId }]
                );
            }
        } catch {
            // 回填失败不影响本次保存结果；下次保存仍会按 parentId+localeId upsert 同一行
        }
    };

    const onSavetoDraft = useCallback(async () => {
        if (saving) return;
        if (!(await ensureCreated())) return;
        setSaving(true);
        try {
            await apiPost("/decorate/decorate/saveDraft", buildSavePayload(editorStore.getSaveData()));
            Toast.success(t("操作成功"));
            await syncLocaleChildId();
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
            await syncLocaleChildId();
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
        // 语言子页标题继承父页，不独立改名（改名只在默认语言会话）
        if (isLocaleChild) return;
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
                hasDraft: editorStore.getState().hasDraft,
                shopId: editorStore.getState().shopId
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

    // 未保存提醒（2026-11-17）：脏标记用 store 既有 isDirty（任何增删改置 true，
    // 存至草稿/保存并发布成功后 markAsSaved 清零、初始化为 false）。
    const doExit = onExit ?? (() => (window.location.href = window.location.origin));
    /** 已确认退出标记：避免确认后整页跳转再被 beforeunload 二次拦截 */
    const exitConfirmedRef = useRef(false);
    const handleExit = useCallback(() => {
        if (!editorStore.getState().isDirty) {
            doExit();
            return;
        }
        Modal.confirm({
            title: t("有未保存的修改"),
            content: t("退出后未保存的修改将丢失，确定退出吗？"),
            okText: t("确定退出"),
            cancelText: t("取消"),
            onOk: () => {
                exitConfirmedRef.current = true;
                doExit();
            }
        });
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [onExit]);

    /** 模板导入成功：跳到新页面的编辑器继续编辑；当前页有未保存修改时先确认 */
    const handleTplImported = useCallback(
        (info: TemplateImportedInfo) => {
            setTplImportVisible(false);
            const target = `${window.location.origin}/?page=h5DecorateEditor&id=${info.decorateId}&pageType=${info.pageType}`;
            const go = () => {
                exitConfirmedRef.current = true;
                window.location.href = target;
            };
            if (editorStore.getState().isDirty) {
                Modal.confirm({
                    title: t("当前页面有未保存的修改"),
                    content: t("模板已导入为新页面，前往编辑新页面将丢失当前未保存的修改，确定前往吗？"),
                    okText: t("前往新页面"),
                    cancelText: t("留在当前页"),
                    onOk: go
                });
                return;
            }
            go();
        },
        // eslint-disable-next-line react-hooks/exhaustive-deps
        []
    );

    // 刷新/关闭页签拦截：仅脏时触发浏览器原生提示
    useEffect(() => {
        const onBeforeUnload = (e: BeforeUnloadEvent) => {
            if (exitConfirmedRef.current) return;
            if (editorStore.getState().isDirty) {
                e.preventDefault();
                e.returnValue = "";
            }
        };
        window.addEventListener("beforeunload", onBeforeUnload);
        return () => window.removeEventListener("beforeunload", onBeforeUnload);
    }, []);

    // 会话级撤销/重做快捷键（我方新增）：Ctrl+Z 撤销 / Ctrl+Shift+Z、Ctrl+Y 重做
    // （macOS 含 Cmd）。焦点在输入框/文本域/下拉/可编辑区时不拦截——文本编辑走
    // 控件自身的原生撤销，其值变化仍经 onChange 进 store 并被历史合并记录。
    useEffect(() => {
        const onKeyDown = (e: KeyboardEvent) => {
            if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
            const key = e.key.toLowerCase();
            if (key !== "z" && key !== "y") return;
            const target = e.target as HTMLElement | null;
            if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.tagName === "SELECT" || target.isContentEditable)) return;
            if (key === "y" || (key === "z" && e.shiftKey)) {
                e.preventDefault();
                editorStore.redo();
            } else {
                e.preventDefault();
                editorStore.undo();
            }
        };
        window.addEventListener("keydown", onKeyDown);
        return () => window.removeEventListener("keydown", onKeyDown);
    }, []);

    return (
        <div className="h5-editor-root">
            {/* 标准编辑器壳（Step 2）：顶栏槽 + 三栏（左库 295 / 画布 / 右栏 380），
                壳尺寸与底色见 components/decorate/editor-shell.css */}
            <EditorShell
                library={<ModuleToolbar pageType={pageType} isShop={localStorage.getItem("adminType") === "shop"} />}
                settings={<SettingsPanel />}
                preview={preview}
                topbar={
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
                            {isLocaleChild ? (
                                // 语言子页：标题继承父页，只读展示
                                <span className="topbar-now-txt-title">
                                    {decorateTitleDraft || s.decorateTitle || DEFAULT_DECORATE_TITLE}
                                </span>
                            ) : editingTitle ? (
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
                        {/* 语言切换（原版有 · 2026-11-21 补齐为真功能）：切换当前
                            装修页的语言版本（整页跳转）；新建模式无父页不渲染 */}
                        {(!isAddMode || isLocaleChild) && (
                            <EditorLocaleSwitch
                                parentId={topPageId}
                                currentLocaleId={isLocaleChild ? queryLocaleId : 0}
                                parentTitle={parentTitle || s.decorateTitle || ""}
                                pageType={pageType}
                                children={localeChildren}
                            />
                        )}
                    </div>
                    <div className="topbar-right">
                        <div className="topbar-btn-group">
                            {/* 会话级撤销/重做（我方新增）：可用性由 store 历史栈派生 */}
                            <button type="button" className="dec-topbar-btn" disabled={!s.canUndo} title={t('撤销 (Ctrl+Z)')} onClick={() => editorStore.undo()}>
                                <IconUndo style={{ marginRight: 4, verticalAlign: -2 }} />
                                {t('撤销')}
                            </button>
                            <button type="button" className="dec-topbar-btn" disabled={!s.canRedo} title={t('重做 (Ctrl+Shift+Z)')} onClick={() => editorStore.redo()}>
                                <IconRedo style={{ marginRight: 4, verticalAlign: -2 }} />
                                {t('重做')}
                            </button>
                            <button type="button" className="dec-topbar-btn" onClick={() => setPreview((v) => !v)}>
                                {preview ? t('退出预览') : t('预览')}
                            </button>
                            <button type="button" className="dec-topbar-btn" onClick={() => setTplImportVisible(true)}>
                                {t('从模板导入')}
                            </button>
                            <button type="button" className="dec-topbar-btn" disabled={saving} onClick={onSavetoDraft}>
                                {t('存至草稿')}
                            </button>
                            <button type="button" className="dec-topbar-btn-primary" disabled={saving} onClick={onPublish}>
                                {t('保存并发布')}
                            </button>
                        </div>
                        {/* 更多菜单：原版海外版含复制/删除页面；复制已功能化（原版有·我方补齐），删除不放（列表侧已有删除能力） */}
                        <Dropdown
                            trigger="click"
                            position="bottomRight"
                            render={
                                <Dropdown.Menu>
                                    <Dropdown.Item onClick={() => setCopyVisible(true)}>{t('从其他页面复制')}</Dropdown.Item>
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
                }
            >
            {/* tabIndex=0：让画布滚动容器可聚焦，聚焦后 PageDown/方向键可滚（原生滚动视口口径） */}
            <div className={`decorate-page-wrap${preview ? " is-preview" : ""}`} tabIndex={0}>
                {detailError ? (
                    <div className="h5-editor-state-card">
                        <div className="h5-editor-state-icon">!</div>
                        <div className="h5-editor-state-title">{t('页面加载失败')}</div>
                        <div className="h5-editor-state-desc">
                            {t('装修服务暂时不可用（可能尚未部署或网络异常）。页面数据未受影响，请重试；反复失败请联系管理员检查服务部署。')}
                        </div>
                        {detailError ? <div className="h5-editor-state-detail">{detailError}</div> : null}
                        <Button theme="solid" onClick={loadDetail}>{t('重新加载')}</Button>
                    </div>
                ) : detailLoading && gateLifted ? (
                    /* 仅超时兜底后详情仍在途才走这里：保留原状态卡（画布不挂载，
                       假空画布不会被误存覆盖真实数据——2026-11-18 立的规矩） */
                    <div className="h5-editor-state-card">
                        <Spin size="large" />
                        <div className="h5-editor-state-title">{t('页面加载中…')}</div>
                        <div className="h5-editor-state-desc">{t('正在加载装修数据，请稍候')}</div>
                    </div>
                ) : (
                    <>
                        {s.hasDraft && dataReady && !preview ? (
                            <div className="decorate-page-alert">
                                <span className="decorate-page-alert-txt">{t('该页面有一条草稿记录，您可以还原草稿继续编辑。')}</span>
                                <Button size="small" onClick={onContinueDraft}>{t('继续编辑')}</Button>
                            </div>
                        ) : null}
                        {/* 画布自编辑器挂载即渲染（与详情请求并行）：iframe 提前开始
                            加载嵌入页；双就绪前由加载门全屏覆盖，不可交互 */}
                        <Canvas preview={preview} />
                        {!gateLifted ? <DecorateLoading title={t('正在加载编辑器…')} /> : null}
                    </>
                )}
            </div>
            </EditorShell>
            <CopyPageDrawer visible={copyVisible} decorateType={decorateType} onClose={() => setCopyVisible(false)} />
            <TemplateLibraryModal visible={tplVisible} pageType={pageType} onClose={() => setTplVisible(false)} />
            <TemplateImportModal
                visible={tplImportVisible}
                platform="mobile"
                onClose={() => setTplImportVisible(false)}
                onImported={handleTplImported}
            />
        </div>
    );
}

// 保留引用避免 tree-shake（会员页固定模块注入依赖 ensureFixedModules 链路）
void ensureFixedModules;
