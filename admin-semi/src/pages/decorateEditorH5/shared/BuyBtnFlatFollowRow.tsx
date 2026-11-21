/**
 * 扁平色族购买按钮「样式来源」行（2026-11-21 三期，我方新增；
 * C 族三模块设置面共用：商品滑动/限时特惠/新人专享）。
 *
 * 与对象型 BuyBtnFollowRow 并列的扁平族版本：C 族按钮色以模块根
 * 扁平字段存（buyBtnGradientStart/End + buyBtnTextColor），跟随
 * 开关是同级扁平键 buyBtnFollowGlobal（对象型是嵌套对象的
 * followGlobal，两族数据形状不同、不可混用同一行组件）。
 *
 * 形态与设置面既有 RadioRowField 一致（自定义/跟随全局 二选一）。
 * 开启跟随 后：由调用方把渐变/文字色字段的显示值换成全局当前值
 * 并整体置灰只读（见各设置面接法），本行下方给状态说明 +
 * 「去全局设置」入口。
 *
 * 纪律：本行只写回模块自身的 buyBtnFollowGlobal 键，跟随期间
 * 模块颜色字段从不被全局值覆盖写回——切回「自定义」即恢复模块
 * 原值（解析在渲染层读时完成，见 components/buyStyleObj 的
 * resolveFlatBtnColors）。数字型 buyBtnStyle 不在跟随映射内，
 * 恒归模块自管。
 */
import { RadioRowField } from "./settingWidgets";

export default function BuyBtnFlatFollowRow({
    data,
    onChange
}: {
    data: Record<string, any>;
    onChange: (patch: Record<string, any>) => void;
}) {
    const following = Number(data?.buyBtnFollowGlobal) === 1;
    return (
        <>
            <RadioRowField
                data={{ ...data, buyBtnFollowGlobal: following ? 1 : 0 }}
                onChange={onChange}
                field="buyBtnFollowGlobal"
                label="按钮颜色来源"
                options={[
                    { value: 0, label: "自定义" },
                    { value: 1, label: "跟随全局" }
                ]}
            />
            {following ? (
                <div style={{ padding: "2px 16px 10px", fontSize: 12, color: "#969799", lineHeight: 1.7 }}>
                    已跟随全局「购买按钮样式」的对象型设置：按钮渐变取全局渐变色 A/B（为空表示跟随主题色）、文字色取全局文字色，下方颜色字段显示的是全局当前值（只读）；
                    <a href="?page=buyStyleSettings" target="_blank" rel="noreferrer">
                        去全局设置
                    </a>
                    。按钮样式（数字型）仍由本模块设置。切回「自定义」恢复本模块自己的颜色。
                </div>
            ) : null}
        </>
    );
}
