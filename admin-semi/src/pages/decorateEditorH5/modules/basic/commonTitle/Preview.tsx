/**
 * 通用标题 预览（DOM 类名逐字对齐 uniapp-react 渲染端 commonTitle/index.tsx + index.scss）
 *
 * 渲染端结构：image-ad-title ad-title_style__N（背景/圆角经 formatTitle 规则）>
 * image-ad-title-l > title-pic-box > img.title-pic（有背景图时）+ image-ad-title-h +
 * image-ad-title-d；showMore=1 时右侧 image-ad-title-r > more-link「查看更多」。
 * showTitle != 1 时渲染端返回 null，预览保持一致。背景渐变规则与渲染端一致：
 * titleBackground2 非空时线性渐变，否则 titleBackground 纯色。
 */
import { useMemo } from "react";
import { cssText, productImage } from "../../../shared/format";
import { mergeDefaultModule } from "../../../contract/sharedDefaults";
import { commonTitleDefaults } from "./index";
import "./Preview.css";

export default function CommonTitlePreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, commonTitleDefaults);
        return merged;
    }, [data]);

    const titleFormat = useMemo(() => {
        const radius = Number(module.titleRadius) || 0;
        const backgroundText = module.titleBackground2
            ? `background:linear-gradient(to right, ${module.titleBackground}, ${module.titleBackground2});`
            : module.titleBackground
              ? `background-color:${module.titleBackground};`
              : "";
        return {
            titleBackground: backgroundText,
            titleRadius: radius ? `border-radius:${radius}px ${radius}px 0 0;` : ""
        };
    }, [module]);

    if (Number(module.showTitle) !== 1) return null;

    const titleBackgroundImage = productImage(module.titleBackgroundPic);

    return (
        <div className={`image-ad-title ad-title_style__${module.titleStyle}`} style={cssText(titleFormat.titleBackground, titleFormat.titleRadius)}>
            <div className="image-ad-title-l">
                {titleBackgroundImage ? (
                    <div className="title-pic-box">
                        <img className="title-pic" src={titleBackgroundImage} alt="" />
                    </div>
                ) : null}
                <div className="image-ad-title-h" style={{ color: module.titleColor }}>
                    {module.titleText}
                </div>
                <div className="image-ad-title-d" style={{ color: module.descColor }}>
                    {module.descText}
                </div>
            </div>
            {Number(module.showMore) === 1 ? (
                <div className="image-ad-title-r">
                    <div className="more-link" style={{ color: module.moreColor }}>
                        <span>查看更多</span>
                        <span className="iconfont-h5 icon-youjiantou" />
                    </div>
                </div>
            ) : null}
        </div>
    );
}
