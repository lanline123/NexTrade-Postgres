/**
 * 对象型购买按钮「样式来源」行（2026-11-21，我方新增；四个对象型模块设置面
 * 共用：商品分组/分组滑动/商品轮播/商品A1）。
 *
 * 形态与设置面既有 RadioRowField 一致（自定义/跟随全局 二选一）。开启跟随
 * 后：由调用方把下方字段的显示值换成全局当前值并整体置灰只读（见各设置面
 * BuyBtnFields 的接法），本行下方给状态说明 + 「去全局设置」入口。
 *
 * 纪律：本行只写回模块对象自身的 followGlobal 键，跟随期间模块其余字段
 * 从不被全局值覆盖写回——切回「自定义」即恢复模块原值（解析在渲染层读时
 * 完成，见 components/buyStyleObj 的 resolveObjBuyButton）。
 */
import { RadioRowField } from "./settingWidgets";

export default function BuyBtnFollowRow({
    data,
    onChange
}: {
    data: Record<string, any>;
    onChange: (patch: Record<string, any>) => void;
}) {
    const following = Number(data?.followGlobal) === 1;
    return (
        <>
            <RadioRowField
                data={{ ...data, followGlobal: following ? 1 : 0 }}
                onChange={onChange}
                field="followGlobal"
                label="样式来源"
                options={[
                    { value: 0, label: "自定义" },
                    { value: 1, label: "跟随全局" }
                ]}
            />
            {following ? (
                <div style={{ padding: "2px 16px 10px", fontSize: 12, color: "#969799", lineHeight: 1.7 }}>
                    已跟随全局「购买按钮样式」的对象型设置，下方外观字段显示的是全局当前值（只读；按钮换行等模块级设置仍可改）；
                    <a href="?page=buyStyleSettings" target="_blank" rel="noreferrer">
                        去全局设置
                    </a>
                    。切回「自定义」恢复本模块自己的设置。
                </div>
            ) : null}
        </>
    );
}
