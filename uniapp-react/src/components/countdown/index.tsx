import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Text, View } from "@tarojs/components";

import "./index.scss";

export interface CountdownTimeData {
    days: number;
    hours: number;
    minutes: number;
    seconds: number;
}

interface Props {
    /** 剩余时间：timestamp=true 时为目标时间戳，否则为剩余秒/毫秒（旧版按量级自动判断） */
    time?: number;
    /** time 是否为时间戳 */
    timestamp?: boolean;
    /** 是否显示天数（默认 true；false 时天数折算进小时） */
    showDays?: boolean;
    /** 是否自动开始（默认 true） */
    autoStart?: boolean;
    /** 数字是否补零（默认 true） */
    format?: boolean;
    /** 是否按日期格式展示（保留旧 prop，展示形态与默认一致） */
    isDateFormat?: boolean;
    onFinish?: () => void;
    onChange?: (timeData: CountdownTimeData) => void;
    /** 默认插槽：({days,hours,minutes,seconds}) => ReactNode，不传则渲染默认文本 */
    children?: (data: CountdownTimeData) => ReactNode;
}

const pad = (n: number, format: boolean) => (format ? String(n).padStart(2, "0") : String(n));

function split(ms: number, showDays: boolean): CountdownTimeData {
    const totalSeconds = Math.max(0, Math.floor(ms / 1000));
    const days = showDays ? Math.floor(totalSeconds / 86400) : 0;
    const rest = totalSeconds - days * 86400;
    return {
        days,
        hours: Math.floor(rest / 3600),
        minutes: Math.floor((rest % 3600) / 60),
        seconds: rest % 60
    };
}

/**
 * 通用倒计时组件（自旧 components/countdown/index.vue 等价移植；
 * 注意与装修模块 modules/countdown 是不同物，后者是营销模块）。
 * 旧 props：time/timeStamp/showDays/autoStart/format/isDateFormat，
 * emits：finish、change(timeData)，默认插槽暴露 days/hours/minutes/seconds。
 */
export default function Countdown({
    time = 0,
    timestamp = false,
    showDays = true,
    autoStart = true,
    format = true,
    onFinish,
    onChange,
    children
}: Props) {
    const target = useMemo(() => {
        if (!time) return 0;
        if (timestamp) return time < 1e12 ? time * 1000 : time;
        // 非时间戳：>1e11 视为毫秒时长，>1e8 视为毫秒级剩余，其余按秒
        return time > 1e8 ? time : time * 1000;
    }, [time, timestamp]);

    const [remain, setRemain] = useState(() => (timestamp ? Math.max(0, target - Date.now()) : target));
    const finishedRef = useRef(false);
    const callbacksRef = useRef({ onFinish, onChange });
    callbacksRef.current = { onFinish, onChange };

    useEffect(() => {
        finishedRef.current = false;
        setRemain(timestamp ? Math.max(0, target - Date.now()) : target);
        if (!autoStart || !target) return;
        const timer = setInterval(() => {
            setRemain((prev) => {
                const next = timestamp ? Math.max(0, target - Date.now()) : Math.max(0, prev - 1000);
                if (next <= 0) {
                    clearInterval(timer);
                    if (!finishedRef.current) {
                        finishedRef.current = true;
                        callbacksRef.current.onFinish?.();
                    }
                }
                return next;
            });
        }, 1000);
        return () => clearInterval(timer);
    }, [target, timestamp, autoStart]);

    const data = split(remain, showDays);

    useEffect(() => {
        callbacksRef.current.onChange?.(data);
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [data.days, data.hours, data.minutes, data.seconds]);

    if (children) {
        return <>{children(data)}</>;
    }

    return (
        <View className="tig-countdown-text">
            {showDays && data.days > 0 ? <Text>{`${pad(data.days, format)}:`}</Text> : null}
            <Text>{`${pad(data.hours, format)}:${pad(data.minutes, format)}:${pad(data.seconds, format)}`}</Text>
        </View>
    );
}
