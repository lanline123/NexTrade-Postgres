import type { ReactNode } from "react";
import { Popup } from "@nutui/nutui-react-taro";

interface Props {
    /** 是否显示（旧 show，对应 update:show） */
    show?: boolean;
    showClose?: boolean;
    overlay?: boolean;
    position?: "top" | "bottom" | "center" | "right" | "left";
    round?: string | number;
    backgroundColor?: string;
    customStyle?: Record<string, any>;
    onUpdateShow?: (show: boolean) => void;
    /** 旧 change 事件：{show, position} */
    onChange?: (e: { show: boolean; position: string }) => void;
    /** 旧 handleShade：点击遮罩 */
    onHandleShade?: () => void;
    onClose?: () => void;
    children?: ReactNode;
}

/**
 * 弹窗（自旧 tig-popup.vue 等价封装，旧内部封装 uview-plus up-popup）。
 * 此前各处直接使用 NutUI Popup（多处注释注明“旧 tig-popup → NutUI Popup”）；
 * 本组件按旧 props/events 收口为统一入口，底层即 NutUI Popup。
 */
export default function TigPopup({
    show = false,
    showClose = true,
    overlay = true,
    position = "bottom",
    round = 10,
    backgroundColor = "#fff",
    customStyle,
    onUpdateShow,
    onChange,
    onHandleShade,
    onClose,
    children
}: Props) {
    const close = () => {
        onUpdateShow?.(false);
        onChange?.({ show: false, position });
        onClose?.();
    };
    return (
        <Popup
            visible={show}
            position={position as any}
            closeable={showClose}
            overlay={overlay}
            round={!!round}
            style={{ background: backgroundColor, ...(customStyle || {}) }}
            onOverlayClick={() => {
                onHandleShade?.();
                close();
            }}
            onClose={close}
        >
            {children}
        </Popup>
    );
}
