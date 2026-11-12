/**
 * 获取订单自提参数
 * @description 从订单数据中提取自提点信息
 * @param data - 订单数据数组
 * @returns 自提参数数组
 */
export const getOrderPickupParams = (data: any[]) => {
    return data?.map((item: any) => {
        return { shopId: item.shopId, pickupId: item.pickups[0].shopId, pickupTitle: item.pickups[0].shopTitle };
    });
};

/**
 * 获取营业时间字符串
 * @description 根据营业时间配置生成可读的时间字符串
 * @param data - 营业时间配置（type: 1全天, 2固定时段, 3按周配置）
 * @param t - 星期文案的翻译函数，由调用方传入组件内的 $t 以支持多语言
 * @returns 格式化的营业时间字符串
 */
export const getBusinessHours = (data: any, t: (key: string) => string = (key) => key) => {
    const formatWeeklyTime = (times: any[]) => {
        if (!times || times.length === 0) return "";

        const weekDayMap: { [key: string]: string } = {
            "1": "周一",
            "2": "周二",
            "3": "周三",
            "4": "周四",
            "5": "周五",
            "6": "周六",
            "7": "周日"
        };
        const weekDayText = (day: string) => (weekDayMap[day] ? t(weekDayMap[day]) : "");
        return times
            .map((timeItem) => {
                let weekText = "";
                if (timeItem.dayOfWeek.includes(",")) {
                    // 多个星期，如 "2,3,4"
                    const days = timeItem.dayOfWeek.split(",");
                    weekText = days.map(weekDayText).join("、");
                } else {
                    // 单个星期，如 "1"
                    weekText = weekDayText(timeItem.dayOfWeek);
                }

                // 处理时间段
                const rangesText = timeItem.ranges?.map((range: any) => `${range.start}-${range.end}`).join("，") || '';

                return `${weekText} ${rangesText}`;
            })
            .join("；");
    };

    let str = "";

    switch (data?.type) {
        case 1:
            str = "0:00-24:00";
            break;
        case 2:
            if (!data.times?.length) break;
            str = (data.times as Array<{ start: string; end: string }>).map(({ start, end }) => {
                if(!start && !end) {
                    return "";
                } else {
                    return `${start}-${end}`;
                }
            })?.filter(Boolean)?.join(",");
            break;
        case 3:
            if (!data.times?.length) break;
            str = formatWeeklyTime(data.times);
            break;
    }

    return str;
};

/**
 * 过滤已过期的时间段
 * @description 根据当前时间和预约提前量，过滤掉已不可选的时间段
 * @param list - 时间段列表
 * @param appointmentTime - 预约提前时间配置
 * @returns 可选的时间段列表
 */
export const filterDayList = (list: any[], appointmentTime?: { timeNum: number; timeUnit: "d" | "h" | "m" }) => {
    const now = new Date();

    // 计算需要提前的时间（转换为分钟）
    let advanceMinutes = 0;
    if (appointmentTime) {
        switch (appointmentTime.timeUnit) {
            case "h":
                advanceMinutes = appointmentTime.timeNum * 60;
                break;
            case "m":
                advanceMinutes = appointmentTime.timeNum;
                break;
        }
    }

    // 计算截止时间
    const deadlineTime = new Date(now.getTime() + advanceMinutes * 60 * 1000);
    const deadlineHour = deadlineTime.getHours();
    const deadlineMinute = deadlineTime.getMinutes();

    return list.filter((item: any) => {
        let itemHour = 0,
            itemMinute = 0;
        if (item.endTime) {
            const [h, m] = item.endTime.split(":").map(Number);
            itemHour = h;
            itemMinute = m;
        }

        // 保留大于截止时间的时间段
        return itemHour > deadlineHour || (itemHour === deadlineHour && itemMinute > deadlineMinute);
    });
};
