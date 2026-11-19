import { useRef, useState, type ReactNode } from "react";
import { View } from "@tarojs/components";
import type { ITouchEvent } from "@tarojs/components/types/common";

interface Props {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    threshold?: number;
    rightAction: ReactNode;
    children: ReactNode;
}

/** Handle the gesture within this product row so taps elsewhere retain native scrolling and navigation. */
export default function CartSwipe({ open, onOpenChange, threshold = 20, rightAction, children }: Props) {
    const start = useRef({ x: 0, y: 0 });
    const axis = useRef<"x" | "y" | null>(null);
    const [offset, setOffset] = useState<number | null>(null);

    return (
        <View className="cart-swipe">
            <View
                className={`cart-swipe__track${open ? " is-open" : ""}`}
                style={offset === null ? undefined : {
                    transform: `translateX(clamp(calc(0px - var(--cart-actions-width)), calc(${open ? "0px - var(--cart-actions-width) + " : ""}${offset}px), 0px))`,
                    transition: "none"
                }}
                onTouchStart={(event) => {
                    const touch = (event as ITouchEvent).touches[0];
                    if (!touch) return;
                    start.current = { x: touch.clientX, y: touch.clientY };
                    axis.current = null;
                }}
                onTouchMove={(event) => {
                    const touch = (event as ITouchEvent).touches[0];
                    if (!touch) return;
                    const dx = touch.clientX - start.current.x;
                    const dy = touch.clientY - start.current.y;
                    if (!axis.current && Math.max(Math.abs(dx), Math.abs(dy)) > 4) {
                        axis.current = Math.abs(dx) > Math.abs(dy) ? "x" : "y";
                    }
                    if (axis.current === "x") setOffset(dx);
                }}
                onTouchEnd={(event) => {
                    const touch = (event as ITouchEvent).changedTouches[0];
                    if (axis.current === "x" && touch) {
                        const dx = touch.clientX - start.current.x;
                        if (Math.abs(dx) > threshold) onOpenChange(dx < 0);
                    }
                    setOffset(null);
                }}
                onTouchCancel={() => setOffset(null)}
            >
                {children}
                <View className="cart-swipe__actions">{rightAction}</View>
            </View>
        </View>
    );
}
