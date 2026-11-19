/**
 * 用户信息2 预览（移植自 view/admin editor/modules/mobileMember/userInfoV2/Preview.vue）
 *
 * 仅头像+昵称+等级一行：头像 56×56 圆、昵称「演示账号」、等级图标+V6会员；
 * wrap 背景/圆角取 frame.backgroundColor/boxRadius，container padding 取 frame.innerPadding。
 */
import { useMemo } from "react";
import { cssText } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { userInfoV2Defaults } from "./index";
import "./Preview.css";

const AVATAR = "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202605/1779691335wtQOROIkUPPNOzuply.jpeg?x-oss-process=image/resize,m_pad,h_200,h_200";
const LEVEL_ICON = "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202605/17796914549H8jHXMHCnnkf8uOXg.jpeg?x-oss-process=image/resize,m_pad,h_200,h_200";

export default function UserInfoV2Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, userInfoV2Defaults);
        return merged;
    }, [data]);

    const frame = module.frame ?? {};
    const frameFormat = formatFrame(frame);

    return (
        <div
            className="module-ad-con module-member-user-info-v2"
            style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}
        >
            <div className="module-ad-content">
                <div
                    className="user-info-v2-wrap"
                    style={{ backgroundColor: frame.backgroundColor || "#ffffff", borderRadius: `${frame.boxRadius ?? 0}px` }}
                >
                    <div className="user-info-v2-container" style={{ padding: `${frame.innerPadding ?? 16}px` }}>
                        <div className="user-info-v2-profile">
                            <div className="user-info-v2-avatar">
                                <img src={AVATAR} alt="头像" />
                            </div>
                            <div className="user-info-v2-info">
                                <div className="user-info-v2-name">演示账号</div>
                                <div className="user-info-v2-level">
                                    <img className="user-info-v2-level-icon" src={LEVEL_ICON} alt="" />
                                    <span className="user-info-v2-level-text">V6会员</span>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
