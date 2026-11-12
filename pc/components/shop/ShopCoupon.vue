<template>
    <div v-if="haveCoupon" class="home-warp">
        <div
            class="base-class"
            :style="{
                backgroundColor: module.backgroundColor,
                borderRadius: module.boxRadius + 'px',
                paddingTop: module.boxPaddingTop + 'px',
                paddingBottom: module.boxPaddingBottom + 'px',
                paddingLeft: module.boxPadding + 'px',
                paddingRight: module.boxPadding + 'px'
            }"
        >
            <div class="top-bar">
                <div class="left-title">
                    <div class="big">{{ module.title }}</div>
                    <div class="sm">{{ module.desc }}</div>
                </div>
                <NuxtLink target="_blank" :to="'/coupon/list?shopId=' + shopId" class="right-title">{{ $t("更多") }} ></NuxtLink>
            </div>
            <div class="coupon-list">
                <div :style="{ padding: module.itemPadding + 'px' }" v-for="item in couponList">
                    <div :style="{ padding: module.itemPadding + 'px' }">
                        <div
                            @click="onSubmit(item)"
                            class="coupon-item"
                            :class="['background-style-' + module.colorStyle, 'font-style-' + module.colorStyle]"
                        >
                            <div class="top">
                                <template v-if="item.couponType == 1">
                                    <FormatPrice
                                        v-model="item.couponMoney"
                                        :currencyStyle="{ fontSize: '12px', fontFamily: 'Verdana, serif', lineHeight: '20px' }"
                                        :fontStyle="{ fontSize: '24px', fontWeight: 'bold' }"
                                    ></FormatPrice>
                                </template>
                                <template v-else>
                                    <!--折扣券-->
                                    <div class="coupon_discount">{{ item.couponDiscount }}<span>{{ $t("折") }}</span></div>
                                </template>
                            </div>
                            <div class="desc">{{ item.couponName }}</div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
<script setup lang="ts">
import { addCoupon, getCouponList } from "~/api/user/coupon";
import type { CouponFilterParams } from "~/types/user/coupon";

const props = defineProps({
    module: { type: Object, default: {} },
    shopId: { type: Number, default: -1 }
});
const couponList: any = ref([]);
const filterParams = reactive<CouponFilterParams>({
    //初使化用于查询的参数
    page: 1,
    isShow: 1,
    shopId: 0
});
const route = useRoute();
filterParams.shopId = Number(route.params?.id);
const haveCoupon = ref(false);
const loadFilter = async () => {
    try {
        const result = await getCouponList({ ...filterParams });
        couponList.value = result.records;
        haveCoupon.value = couponList.value.some((coupon: any) => coupon.isReceive == 1);
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};
loadFilter();
const { t } = useI18n();
const onSubmit = async (value: any) => {
    if (value.isReceive === 1) {
        message.error(t("已领取"));
        return;
    }
    try {
        const result = await addCoupon({ couponId: value.couponId });
        await loadFilter();
        message.success(t("领取成功"));
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};
</script>
<style scoped lang="less">
.base-class {
    display: flex;
    flex-direction: column;
    align-items: center;

    .top-bar {
        display: flex;
        align-items: center;
        justify-content: space-between;
        flex-direction: row;
        width: 100%;

        .left-title {
            display: flex;
            align-items: end;

            .big {
                color: #2a3145;
                font-size: 16px;
                margin-right: 5px;
                font-weight: bold;
            }

            .sm {
                color: #aaa;
                font-size: 12px;
            }
        }

        .right-title {
            cursor: pointer;
        }
    }

    .coupon-list {
        display: flex;
        flex-direction: row;
        width: 100%;
        overflow-x: auto; /* 如果内部div宽度超过外部div，显示横向滚动条 */
        /* 隐藏滚动条但保持滚动功能 */
        -ms-overflow-style: none; /* IE 和 Edge */
        scrollbar-width: none; /* Firefox */

        &::-webkit-scrollbar {
            display: none; /* Chrome, Safari, Opera*/
        }

        .coupon-item {
            width: 90px;
            white-space: nowrap;
            position: relative;
            display: flex;
            flex-direction: column;
            padding: 16px;
            align-items: center;
            justify-content: center;
            cursor: pointer;

            .top {
                .coupon_discount {
                    font-size: 24px;
                    font-weight: bold;

                    & > span {
                        margin-left: 4px;
                        font-size: 12px;
                    }
                }
            }

            .desc {
                text-align: center;
                width: 100%;
                height: 16px;
                font-size: 10px;
                line-height: 16px;
                overflow: hidden; /* 隐藏超出容器的内容 */
                text-overflow: ellipsis; /* 超出部分用省略号表示 */
            }
        }

        .coupon-item:before {
            position: absolute;
            top: 0;
            left: -5px;
            width: 11px;
            height: 100%;
            background-size: 100% 100%;
            content: "";
            z-index: 1;
        }

        .coupon-item:after {
            position: absolute;
            top: 0;
            right: -10px;
            width: 11px;
            height: 100%;
            background-size: 100% 100%;
            content: "";
            z-index: 1;
            transform: rotateY(180deg);
        }

        .background-style-1 {
            background-color: #e74c2c;
        }

        .background-style-1:after,
        .background-style-1:before {
            background-image: url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAC0CAYAAACUlQzjAAAAAXNSR0IArs4c6QAAAdFJREFUaAXtmUFKA0EURH/PiCDEjbjQMyiuXYprXXoCvZW68wZuxUMInkEJ4saFIGTayWAM01N/+L+nN4HKJunf1UXyUkMxTHi7OI5S8FUV9OqsNsCw3tnfDXV1GiTcBpHJPFuP9ev98uRcpHmIUQ7XU9+nnuHyaGcam6f2qw72LNaDP+Xg8eW59bqzHEaagWEnqsM9Eltm0LDa3nu1HEYaaIiE1hk0bH4+j6wGqQ4ayiJep0LrehCNorH5D3ZmBpe/IsyvzmYds+5nxpvcQK+QbC2+P75WixLv+E+Z4EzDCfD+jm4AQ3aKsFPcSYfBZqe4OLJTXLigGOYQKo1DGhpBjcgqdgo7ZSQfyha89NgpCi08ZqdgLp4pzKHHINXSMCXiX7NThJ2SERt0hJ2CqKgzdoqKxrzBCjCjUoXlGfI+hZ2ixk3dgDlkp6i80AY7BVHxzWAOfRZ9NQ37PHJWvE/hfUpGbuClx05xkWSnuHBBMcwhVBqHNDSCGpGxU9gpI/HQtuClx07RcME5OwVicQ1hDl0OiZiGCZCMJTuFnZITG3SGnYKoqDN2iorGvMEKMKNSheUZ8nkKn6eocVM3YA7ZKSovtMFOQVR8M5hDn0Vf/QvYqJ3lQEIVBgAAAABJRU5ErkJggg==);
        }

        .background-style-2 {
            background-color: #ffe9b7;
        }

        .background-style-2:after,
        .background-style-2:before {
            background-image: url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAC0CAYAAACUlQzjAAAAAXNSR0IArs4c6QAAAdBJREFUaAXtWTFOw0AQ3HUQooCOhjfwAWoaHgK/Ajr+gJAQvIJHgOhoEFKy2JGjHL45Z/dsikiTJvbc7DgZz2mUWO390WTGVzOj1lpqHwT16EQWzYWI3orqZD819dA+ny5lZQ9idpbikeM/gt1gL/rcimZrHuHspujp1YuY3HmGEScTXJMWeo/IHgwLrg7fPMOIgwUR04lhwebn3Dmf0bDg0q4zphPIojFrbJJgZxdyfkBR+3g9ls6z7muq3NQGenPBA7HvL1n2p5N3sgi+KZvLVbxTsMK0wcg+eMhOYacMYrv7FAebnbLbuS2DnbL1ovYI57BWrZ2j4ATz+tFG2CnslHCO8NZjp0SMZKdE3MJcnEPMdaEUdNk0SmKntH/68b+v0YyARbz12CnAqiLETila417AOXSP50QK5p5EEXYKOyWamZaPtx47JWIlOyXiFubiHGKuC6Wgy6ZREjuFnTIaELyItx47BbuFUXYK9iWC4hxGFAZcCg4MqThlp7BTqmKDhtgpyJUSxk4pOePHWQF+r0rMf/CQz+j5PKUUtyKOc8hOKRoGFtgpwJQghHMYFEnpFEzdqDvm7xT+TqlIDt567JSIleyUiFuYi3OIuS70F1wX79Qz2wiaAAAAAElFTkSuQmCC);
        }

        .background-style-3 {
            background-color: #383b3e;
        }

        .background-style-3:after,
        .background-style-3:before {
            background-image: url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAC0CAYAAACUlQzjAAAAAXNSR0IArs4c6QAAAdVJREFUaAXtWcFKw0AU3E0jeNAPEAW9e/RSFEror+hfiTc/w9/wXMRP8CIYs2aFQHczL31vk0theml33uwkmU4YQvz6YRPcgp9qQa1/qSMQPD2pzmu/Wjvvn/tznu2n3/fwfrPdhq59DcFd7OOW34lg3BhFu9/2rf85mmmEVznp82O3u7q+uezxu3ymWcN/uXbVi2Yz4mDBOrwjsgaDgpqNEgcKtq2/lTYcwrGg6x4PbZTmo2jMjU1yhkOw+6OPDiSdUY77pmnOomdtvMwQnuaIRfH6+6f7yo8yZ51c8hyhYS8FByfKv4/AQ3aKY6eYEw6DzU4x+chOMdkFyTCHkKkEKag0aoJWsVPYKRP5EEbw1mOnCG5hmJ2CfbGgMIcWgZxLwdwR+5qd4tgpBbFBW9gpyBURY6eI1qgHrAC1VSJxeQ/5nMJOEeMmDmAO2SmiX2jATkGu2DCYQ5tEyqZg6kfJis8pfE4pyA289dgpJifZKSa7IBnmEDKVIAWVRk3Q2CnslIl4SCN467FTJLsgzk6BtphAmEOTQkamYGZIwZKdwk4piQ3aw05BrogYO0W0Rj1gBaitEonLe8j3KXyfIsZNHMAcslNEv9CAnYJcsWEwhzaJlP0HRISbhy7A7SwAAAAASUVORK5CYII=);
        }

        .background-style-4 {
            background-color: #e0f4e4;
        }

        .background-style-4:after,
        .background-style-4:before {
            background-image: url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAC0CAYAAACUlQzjAAAAAXNSR0IArs4c6QAAAc9JREFUaAXtWctOwzAQjNMIVB53uHHnB3rmV+CvgBufgfgL7j1wQL0WqIpoTEOp5Niz1q6TS6WpVDXenZ2ok7FGVtz8481XI37qEbn+qA6A8OT06HxSNzPn3P32O1hPF2o4X73fuPbnyfvqMqxbrnuE3eCOdPPsvU96GuLkoVxNL162gw+aYYRJCDtQ7SaPCKypQcLjaf2qGUYYSIiA2hokXK/aay1BjIOErd/cxkDtOrHGUNs04Z0DYyc3CnG5a7fwi7NOs/+/eVdq6P1Nmq/P7+V+McYvfChDiEk4RL3d7AFoyEypmClmp0NjM1NMOjJTTHJBMPQhRCqLJFQKlYHVzBRmSsYfQgtuPWaKoBYuM1OwLpYq9KGFIMaSMFbEvmamVMyUAtugEWYKUkWsMVNEadQNRoBaKhE4voY8pzBTRLuJDehDZoqoF2owU5Aqthr0oY2ijyZhX4+SFc8pPKcU+AZuPWaKSUlmikkuCIY+hEhlkYRKoTIwZgozJWMPqQW3HjNFkgvWmSlQFlMR+tDEEIFJGAlSsGSmMFNKbINmmClIFbHGTBGlUTcYAWqpROD4GvJ9Ct+niHYTG9CHzBRRL9RgpiBVbDXoQxtFH/0LaCUp7PtuoaoAAAAASUVORK5CYII=);
        }

        .font-style-1 {
            color: #ffffff;
        }

        .font-style-2 {
            color: #f39343;
        }

        .font-style-3 {
            color: #ffffff;
        }

        .font-style-4 {
            color: #4cac5a;
        }
    }
}
</style>
