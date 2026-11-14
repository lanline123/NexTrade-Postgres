/**
 * 账户信息 预览（类名对齐渲染端 dec-account__*；非跨境版渲染端会过滤语言类条目，预览全量展示）
 */
import { useMemo } from "react";
import { cssText, imageFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { accountDefaults } from "./index";
import "./Preview.css";

export default function AccountPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, accountDefaults);
        return merged;
    }, [data]);
    const frameFormat = formatFrame(module.frame ?? {});
    const accountList: any[] = (module.accountList?.length ? module.accountList : accountDefaults.accountList).filter((it: any) => Number(it.isShow ?? 1) !== 0);
    return (
        <div className="module-ad-con" style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}>
            <div className="module-ad-content" style={cssText(frameFormat.backgroundColor, frameFormat.boxRadius)}>
                <div className="dec-account">
                    <div className="dec-account__container" style={{ padding: module.frame?.innerPadding ?? 15 }}>
                        {module.moduleTitle ? (
                            <div className="dec-account__header">
                                <div className="dec-account__title">{module.moduleTitle}</div>
                            </div>
                        ) : null}
                        <div className="dec-account__list">
                            {accountList.map((item, index) => (
                                <div className="dec-account__item" key={index}>
                                    <div className="dec-account__item-left">
                                        <div className="dec-account__item-icon">
                                            {item.picUrl ? <img src={imageFormat(item.picUrl)} alt="" /> : <span className="dec-account__icon-ph">{(item.picTitle || "账").slice(0, 1)}</span>}
                                        </div>
                                        <div className="dec-account__item-title">{item.picTitle}</div>
                                    </div>
                                    <div className="dec-account__item-right">
                                        {item.picDesc ? <span className="dec-account__item-subtitle">{item.picDesc}</span> : null}
                                        <span className="dec-account__item-arrow">›</span>
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
