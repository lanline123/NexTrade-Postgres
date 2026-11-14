/**
 * 页面导航 预览（DOM 类名逐字对齐 uniapp-react 渲染端 navbar/index.tsx + index.scss）
 *
 * 渲染端结构：tig-navbar > navbar-box（背景 titleBackgroundColor）> navbar >
 * navbar-content > left / middle > title line1（颜色 titleColor，文字 title）/ right，
 * 其后跟 placeholder 占位（渲染端为空占位，保持一致）。左侧返回区渲染端恒不显示
 * （showLeft=false），预览保持一致。
 */
import { useMemo } from "react";
import { mergeDefaultModule } from "../../../contract/sharedDefaults";
import { navbarDefaults } from "./index";
import "./Preview.css";

export default function NavbarPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, navbarDefaults);
        return merged;
    }, [data]);

    return (
        <div className="tig-navbar">
            <div className="navbar-box" style={{ backgroundColor: module.titleBackgroundColor }}>
                <div className="navbar">
                    <div className="navbar-content">
                        <div className="left" />
                        <div className="middle">
                            <div className="title line1" style={{ color: module.titleColor }}>
                                <span>{module.title}</span>
                            </div>
                        </div>
                        <div className="right" />
                    </div>
                </div>
            </div>
            <div className="placeholder" />
        </div>
    );
}
