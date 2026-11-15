import type { ComponentProps } from "react";
import { View } from "@tarojs/components";

import ModuleCommonCountdown from "@/pages/customPage/modules/commonCountdown/index";

import "./index.scss";

/**
 * 通用倒计时（components 层入口，补齐旧 components/commonCountdown/index.vue 的位置）。
 * 实现本体在装修模块层（pages/customPage/modules/commonCountdown），被 countdown 模块与
 * groupon 列表复用；此处包装为 components 级共享组件，避免其他页面反向引用 pages 路径。
 * props 与旧组件一致：countdownType/dailyTime/specifyTime/format/color/zh/valueStyle/timeStyle/unitStyle
 * + render-prop children({days,hours,minutes,seconds})。
 *
 * P1：旧共享件独有口径（模块版没有）——.time-item 字号 12px、.time-item-unit margin-left:2px，
 * 由包装层类 .shared-common-countdown 作用域补回（见 index.scss），不影响模块版渲染。
 */
export default function CommonCountdown(props: ComponentProps<typeof ModuleCommonCountdown>) {
    return (
        <View className="shared-common-countdown">
            <ModuleCommonCountdown {...props} />
        </View>
    );
}
