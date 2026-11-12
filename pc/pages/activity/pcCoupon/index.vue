<template>
    <div :style="{ padding: itemPadding + 'px' }">
        <TotalContainer :module="frameModule" :class="['module-ad-con', 'module-pc-coupon']">
            <Title :module="titleModule"></Title>
            <Coupon></Coupon>
        </TotalContainer>
    </div>
</template>

<script lang="ts" setup>
/**
 * PC端装修优惠券模块
 * - 展示可领取的优惠券列表
 * - 支持自定义容器样式（边距、圆角、背景色等）
 * - 支持店铺专属优惠券（通过 shopId 过滤）
 */
import { computed } from "vue";
import TotalContainer from "~/components/shop/src/TotalContainer.vue";
import Title from "~/pages/activity/components/Title.vue";
import Coupon from "~/components/home/Coupon.vue";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    },
    shopId: {
        type: Number,
        default: -1
    }
});

const frameModule = computed(() => {
    const frame = props.module ?? {};
    return {
        boxPaddingTop: frame.boxPaddingTop ?? 0,
        boxPaddingBottom: frame.boxPaddingBottom ?? 0,
        boxPadding: frame.boxPadding ?? 0,
        backgroundColor: "transparent",
        boxRadius: frame.boxRadius ?? 0
    };
});
// 组件边距
const itemPadding = computed(() => props.module?.itemPadding ?? 0);

const titleModule = computed(() => {
    const module = props.module ?? {};
    return {
        showTitle: module.showTitle == 1,
        titleText: module.title || "",
        descText: module.desc || "",
        titleColor: module.titleColor ?? "#000"
    };
});
</script>

<style lang="less" scoped>
.module-pc-coupon {
    .coupon-wrapper {
        display: flex;
        flex-direction: column;
        gap: 16px;
        padding: 10px;
    }

    .coupon-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0 10px;

        .coupon-header-text {
            display: flex;
            align-items: baseline;
            gap: 8px;

            .coupon-main {
                color: #2a3145;
                font-size: 20px;
                font-weight: 600;
            }

            .coupon-sub {
                color: #969799;
                font-size: 14px;
            }
        }

        .coupon-more {
            display: inline-flex;
            align-items: center;
            gap: 4px;
            color: #999;
            font-size: 14px;
            text-decoration: none;

            .arrow {
                font-size: 16px;
            }
        }
    }

    .coupon-list-wrapper {
        overflow-x: auto;
        overflow-y: visible;
        -webkit-overflow-scrolling: touch;
        scrollbar-width: thin;
        scrollbar-color: rgba(0, 0, 0, 0.2) transparent;
        padding: 0 6px;

        &::-webkit-scrollbar {
            height: 6px;
        }

        &::-webkit-scrollbar-track {
            background: transparent;
        }

        &::-webkit-scrollbar-thumb {
            background: rgba(0, 0, 0, 0.2);
            border-radius: 3px;
        }
    }

    .coupon-list {
        display: flex;
        flex-wrap: nowrap;
        padding-bottom: 10px;
        gap: 30px;
    }

    .coupon-card {
        flex-shrink: 0;
        width: 200px;
        min-height: 100px;
        color: #fff;
        background-color: #e74c2c;
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        justify-content: center;
        gap: 12px;
        position: relative;
        cursor: pointer;
        transition:
            transform 0.2s ease,
            box-shadow 0.2s ease;
        border-radius: 8px;
        overflow: visible;

        // &:hover {
        //     transform: translateY(-2px);
        //     box-shadow: 0 8px 18px rgba(0, 0, 0, 0.15);
        // }

        &::before,
        &::after {
            content: "";
            position: absolute;
            top: 0;
            width: 12px;
            height: 100%;
            background-repeat: repeat-y;
            z-index: 2;
            pointer-events: none;
        }

        &::before {
            left: -6px;
            background-image: url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAC0CAYAAACUlQzjAAAAAXNSR0IArs4c6QAAAdFJREFUaAXtmUFKA0EURH/PiCDEjbjQMyiuXYprXXoCvZW68wZuxUMInkEJ4saFIGTayWAM01N/+L+nN4HKJunf1UXyUkMxTHi7OI5S8FUV9OqsNsCw3tnfDXV1GiTcBpHJPFuP9ev98uRcpHmIUQ7XU9+nnuHyaGcam6f2qw72LNaDP+Xg8eW59bqzHEaagWEnqsM9Eltm0LDa3nu1HEYaaIiE1hk0bH4+j6wGqQ4ayiJep0LrehCNorH5D3ZmBpe/IsyvzmYds+5nxpvcQK+QbC2+P75WixLv+E+Z4EzDCfD+jm4AQ3aKsFPcSYfBZqe4OLJTXLigGOYQKo1DGhpBjcgqdgo7ZSQfyha89NgpCi08ZqdgLp4pzKHHINXSMCXiX7NThJ2SERt0hJ2CqKgzdoqKxrzBCjCjUoXlGfI+hZ2ixk3dgDlkp6i80AY7BVHxzWAOfRZ9NQ37PHJWvE/hfUpGbuClx05xkWSnuHBBMcwhVBqHNDSCGpGxU9gpI/HQtuClx07RcME5OwVicQ1hDl0OiZiGCZCMJTuFnZITG3SGnYKoqDN2iorGvMEKMKNSheUZ8nkKn6eocVM3YA7ZKSovtMFOQVR8M5hDn0Vf/QvYqJ3lQEIVBgAAAABJRU5ErkJggg==);
        }

        &::after {
            right: -6px;
            background-image: url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAC0CAYAAACUlQzjAAAAAXNSR0IArs4c6QAAAdFJREFUaAXtmUFKA0EURH/PiCDEjbjQMyiuXYprXXoCvZW68wZuxUMInkEJ4saFIGTayWAM01N/+L+nN4HKJunf1UXyUkMxTHi7OI5S8FUV9OqsNsCw3tnfDXV1GiTcBpHJPFuP9ev98uRcpHmIUQ7XU9+nnuHyaGcam6f2qw72LNaDP+Xg8eW59bqzHEaagWEnqsM9Eltm0LDa3nu1HEYaaIiE1hk0bH4+j6wGqQ4ayiJep0LrehCNorH5D3ZmBpe/IsyvzmYds+5nxpvcQK+QbC2+P75WixLv+E+Z4EzDCfD+jm4AQ3aKsFPcSYfBZqe4OLJTXLigGOYQKo1DGhpBjcgqdgo7ZSQfyha89NgpCi08ZqdgLp4pzKHHINXSMCXiX7NThJ2SERt0hJ2CqKgzdoqKxrzBCjCjUoXlGfI+hZ2ixk3dgDlkp6i80AY7BVHxzWAOfRZ9NQ37PHJWvE/hfUpGbuClx05xkWSnuHBBMcwhVBqHNDSCGpGxU9gpI/HQtuClx07RcME5OwVicQ1hDl0OiZiGCZCMJTuFnZITG3SGnYKoqDN2iorGvMEKMKNSheUZ8nkKn6eocVM3YA7ZKSovtMFOQVR8M5hDn0Vf/QvYqJ3lQEIVBgAAAABJRU5ErkJggg==);
            transform: scaleX(-1);
        }

        .coupon-value {
            flex-shrink: 0;
            display: flex;
            align-items: flex-end;
            gap: 2px;
            padding-left: 16px;
            min-width: 90px;

            :deep(.currency) {
                font-size: 18px !important;
            }

            :deep(.decimals) {
                font-size: 20px !important;
            }

            :deep(.num) {
                font-size: 32px !important;
                font-weight: 700 !important;
                line-height: 1 !important;
            }

            .discount {
                font-size: 32px;
                font-weight: 700;
                line-height: 1;
            }

            .unit {
                font-size: 18px;
                margin-bottom: 4px;
            }
        }

        .coupon-info {
            flex: 1;
            display: flex;
            flex-direction: column;
            gap: 4px;
            padding: 0 8px;

            .coupon-name {
                font-size: 14px;
                font-weight: 500;
                line-height: 1.4;
                word-break: break-all;
            }

            .coupon-remark {
                font-size: 12px;
                opacity: 0.9;
                line-height: 1.4;
            }
        }

        .coupon-action {
            flex-shrink: 0;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            gap: 2px;
            font-size: 14px;
            padding-right: 16px;
            min-width: 60px;

            .arrow {
                font-size: 16px;
            }
        }
    }

    .ad-color_style__2 .coupon-card {
        background-color: #ffe9b7 !important;
        color: #fff;
    }

    .ad-color_style__3 .coupon-card {
        background-color: #383b3e !important;
        color: #fff;
    }

    .ad-color_style__4 .coupon-card {
        background-color: #e0f4e4 !important;
        color: #fff;
    }

    .ad-color_style__1 .coupon-card::before,
    .ad-color_style__1 .coupon-card::after {
        background-image: url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAC0CAYAAACUlQzjAAAAAXNSR0IArs4c6QAAAdFJREFUaAXtmUFKA0EURH/PiCDEjbjQMyiuXYprXXoCvZW68wZuxUMInkEJ4saFIGTayWAM01N/+L+nN4HKJunf1UXyUkMxTHi7OI5S8FUV9OqsNsCw3tnfDXV1GiTcBpHJPFuP9ev98uRcpHmIUQ7XU9+nnuHyaGcam6f2qw72LNaDP+Xg8eW59bqzHEaagWEnqsM9Eltm0LDa3nu1HEYaaIiE1hk0bH4+j6wGqQ4ayiJep0LrehCNorH5D3ZmBpe/IsyvzmYds+5nxpvcQK+QbC2+P75WixLv+E+Z4EzDCfD+jm4AQ3aKsFPcSYfBZqe4OLJTXLigGOYQKo1DGhpBjcgqdgo7ZSQfyha89NgpCi08ZqdgLp4pzKHHINXSMCXiX7NThJ2SERt0hJ2CqKgzdoqKxrzBCjCjUoXlGfI+hZ2ixk3dgDlkp6i80AY7BVHxzWAOfRZ9NQ37PHJWvE/hfUpGbuClx05xkWSnuHBBMcwhVBqHNDSCGpGxU9gpI/HQtuClx07RcME5OwVicQ1hDl0OiZiGCZCMJTuFnZITG3SGnYKoqDN2iorGvMEKMKNSheUZ8nkKn6eocVM3YA7ZKSovtMFOQVR8M5hDn0Vf/QvYqJ3lQEIVBgAAAABJRU5ErkJggg==);
    }

    .ad-color_style__2 .coupon-card::before,
    .ad-color_style__2 .coupon-card::after {
        background-image: url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAC0CAYAAACUlQzjAAAAAXNSR0IArs4c6QAAAdBJREFUaAXtWTFOw0AQ3HUQooCOhjfwAWoaHgK/Ajr+gJAQvIJHgOhoEFKy2JGjHL45Z/dsikiTJvbc7DgZz2mUWO390WTGVzOj1lpqHwT16EQWzYWI3orqZD819dA+ny5lZQ9idpbikeM/gt1gL/rcimZrHuHspujp1YuY3HmGEScTXJMWeo/IHgwLrg7fPMOIgwUR04lhwebn3Dmf0bDg0q4zphPIojFrbJJgZxdyfkBR+3g9ls6z7muq3NQGenPBA7HvL1n2p5N3sgi+KZvLVbxTsMK0wcg+eMhOYacMYrv7FAebnbLbuS2DnbL1ovYI57BWrZ2j4ATz+tFG2CnslHCO8NZjp0SMZKdE3MJcnEPMdaEUdNk0SmKntH/68b+v0YyARbz12CnAqiLETila417AOXSP50QK5p5EEXYKOyWamZaPtx47JWIlOyXiFubiHGKuC6Wgy6ZREjuFnTIaELyItx47BbuFUXYK9iWC4hxGFAZcCg4MqThlp7BTqmKDhtgpyJUSxk4pOePHWQF+r0rMf/CQz+j5PKUUtyKOc8hOKRoGFtgpwJQghHMYFEnpFEzdqDvm7xT+TqlIDt567JSIleyUiFuYi3OIuS70F1wX79Qz2wiaAAAAAElFTkSuQmCC);
    }

    .ad-color_style__3 .coupon-card::before,
    .ad-color_style__3 .coupon-card::after {
        background-image: url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAC0CAYAAACUlQzjAAAAAXNSR0IArs4c6QAAAdVJREFUaAXtWcFKw0AU3E0jeNAPEAW9e/RSFEror+hfiTc/w9/wXMRP8CIYs2aFQHczL31vk0theml33uwkmU4YQvz6YRPcgp9qQa1/qSMQPD2pzmu/Wjvvn/tznu2n3/fwfrPdhq59DcFd7OOW34lg3BhFu9/2rf85mmmEVznp82O3u7q+uezxu3ymWcN/uXbVi2Yz4mDBOrwjsgaDgpqNEgcKtq2/lTYcwrGg6x4PbZTmo2jMjU1yhkOw+6OPDiSdUY77pmnOomdtvMwQnuaIRfH6+6f7yo8yZ51c8hyhYS8FByfKv4/AQ3aKY6eYEw6DzU4x+chOMdkFyTCHkKkEKag0aoJWsVPYKRP5EEbw1mOnCG5hmJ2CfbGgMIcWgZxLwdwR+5qd4tgpBbFBW9gpyBURY6eI1qgHrAC1VSJxeQ/5nMJOEeMmDmAO2SmiX2jATkGu2DCYQ5tEyqZg6kfJis8pfE4pyA289dgpJifZKSa7IBnmEDKVIAWVRk3Q2CnslIl4SCN467FTJLsgzk6BtphAmEOTQkamYGZIwZKdwk4piQ3aw05BrogYO0W0Rj1gBaitEonLe8j3KXyfIsZNHMAcslNEv9CAnYJcsWEwhzaJlP0HRISbhy7A7SwAAAAASUVORK5CYII=);
    }

    .ad-color_style__4 .coupon-card::before,
    .ad-color_style__4 .coupon-card::after {
        background-image: url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAC0CAYAAACUlQzjAAAAAXNSR0IArs4c6QAAAc9JREFUaAXtWctOwzAQjNMIVB53uHHnB3rmV+CvgBufgfgL7j1wQL0WqIpoTEOp5Niz1q6TS6WpVDXenZ2ok7FGVtz8481XI37qEbn+qA6A8OT06HxSNzPn3P32O1hPF2o4X73fuPbnyfvqMqxbrnuE3eCOdPPsvU96GuLkoVxNL162gw+aYYRJCDtQ7SaPCKypQcLjaf2qGUYYSIiA2hokXK/aay1BjIOErd/cxkDtOrHGUNs04Z0DYyc3CnG5a7fwi7NOs/+/eVdq6P1Nmq/P7+V+McYvfChDiEk4RL3d7AFoyEypmClmp0NjM1NMOjJTTHJBMPQhRCqLJFQKlYHVzBRmSsYfQgtuPWaKoBYuM1OwLpYq9KGFIMaSMFbEvmamVMyUAtugEWYKUkWsMVNEadQNRoBaKhE4voY8pzBTRLuJDehDZoqoF2owU5Aqthr0oY2ijyZhX4+SFc8pPKcU+AZuPWaKSUlmikkuCIY+hEhlkYRKoTIwZgozJWMPqQW3HjNFkgvWmSlQFlMR+tDEEIFJGAlSsGSmMFNKbINmmClIFbHGTBGlUTcYAWqpROD4GvJ9Ct+niHYTG9CHzBRRL9RgpiBVbDXoQxtFH/0LaCUp7PtuoaoAAAAASUVORK5CYII=);
    }
}

:deep(.mod_coupon) {
    margin-top: 0 !important;
}
</style>
