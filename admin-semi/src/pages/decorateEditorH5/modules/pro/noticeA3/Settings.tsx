/**
 * 商城公告3 设置（对齐 view/admin mobilePro/noticeA3/Settings.vue）
 *
 * 页签：基本设置 / 扩展设置。
 * 基本设置：公告卡片列表（标题/前缀描述/链接，可增删、上限 12、余数>1 可删；
 * 原版拖拽排序以卡内上移/下移承接）+ 公告头部区域/头部图宽度（max 200）；
 * 颜色设置：公告文字颜色/公告头部图/公告边框色/公告内容背景色/公告前缀/公告边框。
 * 扩展页：ModuleStyleEdit + ContentStyleEdit（type="noticeA3"）。
 * 注：逐条链接原版为 SelectLink（未移植），以文本输入写回同一 link 结构。
 */
import { Input } from "@douyinfe/semi-ui";
import { SettingGroup, RangeField, ColorField, SwitchField, SinglePicField } from "../../../shared/settingWidgets";
import { SettingsTabs, ExtensionStyleSettings } from "../../../shared/styleEdits";

import { useI18n } from '../../../../../i18n';
const MAX_NOTICE_COUNT = 12;

function NoticeListEditor({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const { t } = useI18n();
    const list: Array<Record<string, any>> = Array.isArray(data?.noticeList) ? data.noticeList : [];
    const setList = (next: Array<Record<string, any>>) => onChange({ noticeList: next });
    const patchItem = (index: number, patch: Record<string, any>) => setList(list.map((it, i) => (i === index ? { ...it, ...patch } : it)));
    const moveItem = (index: number, delta: number) => {
        const target = index + delta;
        if (target < 0 || target >= list.length) return;
        const next = [...list];
        const [it] = next.splice(index, 1);
        next.splice(target, 0, it);
        setList(next);
    };
    const fieldRow = (label: string, control: React.ReactNode) => (
        <div style={{ display: "flex", alignItems: "center", marginBottom: 10 }}>
            <div style={{ width: 60, flexShrink: 0, fontSize: 13, color: "#333", textAlign: "right", marginRight: 10 }}>{t(label)}</div>
            <div style={{ flex: 1 }}>{control}</div>
        </div>
    );
    return (
        <div style={{ padding: "0 10px 15px" }}>
            {list.map((item, index) => {
                const linkUrl = typeof item.link === "string" ? item.link : item.link?.link?.link ?? "";
                return (
                    <div
                        key={index}
                        style={{
                            width: "100%",
                            position: "relative",
                            marginTop: 12,
                            borderRadius: 2,
                            backgroundColor: "#fff",
                            boxShadow: "0 0 4px 0 rgba(10, 42, 97, 0.2)"
                        }}
                    >
                        <div style={{ padding: "12px 10px" }}>
                            {fieldRow(
                                "标题",
                                <Input
                                    value={item.title ?? ""}
                                    placeholder={t('请输入公告标题')}
                                    onChange={(v: string) => patchItem(index, { title: v })}
                                />
                            )}
                            {fieldRow(
                                "前缀描述",
                                <Input
                                    value={item.prefix ?? ""}
                                    placeholder={t('请输入前缀')}
                                    onChange={(v: string) => patchItem(index, { prefix: v })}
                                />
                            )}
                            {fieldRow(
                                "链接",
                                <Input
                                    value={linkUrl}
                                    placeholder={t('跳转链接，如 /pages/index/index')}
                                    onChange={(v: string) =>
                                        patchItem(index, {
                                            link:
                                                item.link && typeof item.link === "object"
                                                    ? { ...item.link, link: { ...(item.link.link || {}), link: v } }
                                                    : { link: { link: v, title: "" }, linkTitle: "" }
                                        })
                                    }
                                />
                            )}
                            <div style={{ display: "flex", gap: 10, fontSize: 12, justifyContent: "flex-end" }}>
                                <a className={index === 0 ? "is-disabled" : ""} onClick={() => moveItem(index, -1)}>
                                    {t('上移')}
                                </a>
                                <a className={index === list.length - 1 ? "is-disabled" : ""} onClick={() => moveItem(index, 1)}>
                                    {t('下移')}
                                </a>
                            </div>
                        </div>
                        {list.length > 1 ? (
                            <span
                                title={t('删除')}
                                onClick={() => setList(list.filter((_, i) => i !== index))}
                                style={{
                                    position: "absolute",
                                    right: -10,
                                    top: -10,
                                    color: "#fff",
                                    background: "#bbb",
                                    border: "2px solid #fff",
                                    borderRadius: "50%",
                                    zIndex: 2,
                                    width: 18,
                                    height: 18,
                                    textAlign: "center",
                                    lineHeight: "16px",
                                    cursor: "pointer",
                                    fontSize: 12,
                                    boxSizing: "border-box"
                                }}
                            >
                                ×
                            </span>
                        ) : null}
                    </div>
                );
            })}
            {list.length < MAX_NOTICE_COUNT ? (
                <a className="dec-pic-add-btn" style={{ marginTop: 12 }} onClick={() => setList([...list, { title: "", prefix: "", link: {} }])}>
                    {t('添加公告')}
                </a>
            ) : null}
        </div>
    );
}

export default function NoticeA3Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["基本设置", "扩展设置"]}>
                <>
                    <SettingGroup title="基本设置">
                        <NoticeListEditor data={data} onChange={onChange} />
                        <SwitchField data={data} onChange={onChange} field="showHeader" label="公告头部区域" />
                        {Number(data?.showHeader) === 1 ? (
                            <RangeField data={data} onChange={onChange} field="headerImageWidth" label="头部图宽度" max={200} />
                        ) : null}
                    </SettingGroup>
                    <SettingGroup title="颜色设置">
                        <ColorField data={data} onChange={onChange} field="textColor" label="公告文字颜色" resetValue="#666666" />
                        <SinglePicField data={data} onChange={onChange} field="headerImage" label="公告头部图" />
                        <ColorField data={data} onChange={onChange} field="borderColor" label="公告边框色" resetValue="" />
                        <ColorField data={data} onChange={onChange} field="contentBgColor" label="公告内容背景色" resetValue="" />
                        <SwitchField data={data} onChange={onChange} field="showPrefix" label="公告前缀" />
                        <SwitchField data={data} onChange={onChange} field="showBorder" label="公告边框" />
                    </SettingGroup>
                </>
                <ExtensionStyleSettings data={data} onChange={onChange} type="noticeA3" />
            </SettingsTabs>
        </div>
    );
}
