<template>
    <div style="background-color: #fff">
        <CommonHeader title="招商入驻"></CommonHeader>
        <joinHeader></joinHeader>
        <div class="banner-warp">
                <swiper v-model:swiperOption="swiperOption" :itemList="joinStore?.module1?.value0?.images">
                    <template v-slot:default="{ item, index }">
                        <li>
                            <NuxtLink :to="urlFormat(item.picLink)" target="_blank">
                                <el-image :src="imageFormat(item.picUrl)" class="item-img" loading="lazy" />
                            </NuxtLink>
                        </li>
                    </template>
                </swiper>
            </div>
            <div data-content-rev="dmJpZzIyODI" class="zs-info">
                <CommonPageContainer>
                    <h2 class="com__title">{{ $t("我要合作") }}</h2>
                    <div class="zs-info__wrap">
                        <div class="cooperation-col">
                            <div class="cooperation-image plan"></div>
                            <h3>{{ $t("招商计划") }}</h3>
                            <p>{{ $t("查询发布的热招类目和品牌") }}</p>
                            <NuxtLink class="cooperation-button" target="_blank" to="/join/plan/">{{ $t("查看招商计划") }}</NuxtLink>
                        </div>
                        <div class="cooperation-col">
                            <div class="cooperation-image mode"></div>
                            <h3>{{ $t("合作模式") }}</h3>
                            <p>{{ $t("了解可以合作那些模式，各模式的优势和区别") }}。</p>
                            <div class="cooperation-button" @click="scrollToElement">{{ $t("查看合作模式") }}</div>
                        </div>
                        <div class="cooperation-col">
                            <div class="cooperation-image standard"></div>
                            <h3>{{ $t("招商标准") }}</h3>
                            <p>{{ $t("查询自身行业所需要提交的资质") }}</p>
                            <NuxtLink class="cooperation-button" target="_blank" to="/join/standard/">{{ $t("查看招商标准") }} </NuxtLink>
                        </div>
                        <div class="cooperation-col">
                            <div class="cooperation-image join"></div>
                            <h3>{{ $t("我要入驻") }}</h3>
                            <p>{{ $t("我已了解所有内容，需要立即入驻") }}</p>
                            <NuxtLink class="cooperation-button active" target="_blank" to="/join/intro">{{ $t("立即入驻") }}</NuxtLink>
                        </div>
                    </div>
                </CommonPageContainer>
            </div>
            <div ref="targetElement" class="ms-info">
                <CommonPageContainer>
                    <h2 class="com__title">{{ $t("我要合作") }}</h2>
                    <div class="mode-list">
                        <div :class="{ current: tabActive === 1 }" class="mode-col mode-1" @mouseenter="tabClick(1)">
                            <span>{{ $t("平台模式") }}</span>
                        </div>
                        <div :class="{ current: tabActive === 2 }" class="mode-col mode-2" @mouseenter="tabClick(2)">
                            <span>{{ $t("自营模式") }}</span>
                        </div>
                    </div>
                    <div class="mode-detail" v-if="Object.keys(joinStore?.module1).length">
                        <div v-show="tabActive === 1" class="mode-detail-content">
                            <div class="mode-detail-pic">
                                <img :src="imageFormat(joinStore?.module1?.value1?.image)" />
                            </div>
                            <h3>{{ joinStore?.module1?.value1?.title }}</h3>
                            <span class="content" v-html="joinStore?.module1?.value1?.content"></span>
                            <NuxtLink class="learn-more" to="/">{{ $t("了解详情") }}</NuxtLink>
                        </div>
                        <div v-show="tabActive === 2" class="mode-detail-content">
                            <div class="mode-detail-pic">
                                <img :src="imageFormat(joinStore?.module1?.value2?.image)" />
                            </div>
                            <h3>{{ joinStore?.module1?.value2?.title }}</h3>
                            <span class="content" v-html="joinStore?.module1?.value2?.content"></span>
                            <NuxtLink class="learn-more" to="/">{{ $t("了解详情") }}</NuxtLink>
                        </div>
                    </div>
                </CommonPageContainer>
            </div>
    </div>
    <CommonPageFooter :mt30="false"></CommonPageFooter>
</template>
<script lang="ts" setup>
/**
 * 招商入驻首页
 * - 仅多商户模式(Merchant)下可访问
 * - 展示招商Banner轮播图
 * - 提供四个入口：招商计划、合作模式、招商标准、立即入驻
 * - 展示平台模式和自营模式两种合作方式介绍
 * - 数据从后台装修配置获取
 */
import { nextTick, onMounted, ref } from "vue";
import joinHeader from "~/pages/join/joinHeader.vue";
import { useJoinStore } from "~/store/join";
import { getMobileNav } from "~/api/home/home";
import { urlFormat } from "~/utils/format";
const loading = ref(true);
const swiperOption = ref<any>({
    // Swiper的配置选项
    // 根据实际需求进行配置
    // 例如：autoplay, loop, navigation等
    autoplay: {
        delay: 5000,
        disableOnInteraction: false, //用户操作swiper之后，是否禁止autoplay
        pauseOnMouseEnter: true //鼠标置于swiper是否时暂停自动切换
    },
    navigation: true,
    effect: "slide",
    loop: true
});

const tabActive = ref(1);
const tabClick = (value: number) => {
    tabActive.value = value;
};

const targetElement = ref<any>(null);
const scrollToElement = () => {
    nextTick(() => {
        if (targetElement.value) {
            targetElement.value.scrollIntoView({ behavior: "smooth" });
        }
    });
};
const joinStore = useJoinStore();
const getBackgroundPicture = async () => {
    try {
        const result = await getMobileNav({ decorateSn: "investmentPromotionHomepage" });
        let temp = result?.data;
        let tempObj = {
            value0: {
                images: temp[0].images
            },
            value1: {
                title: temp[1].title,
                content: temp[1].content,
                image: temp[1].image
            },
            value2: {
                title: temp[2].title,
                content: temp[2].content,
                image: temp[2].image
            }
        };
        Object.assign(joinStore.module1, tempObj);
    } catch (error) {
        console.log(error);
    } finally {
        joinStore.module1Loaded = true;
    }
};
onMounted(async () => {
    if (joinStore.module1Loaded === false) {
        await getBackgroundPicture();
    }
});
</script>
<style lang="less" scoped>
.com__title {
    color: #333333;
    font-size: 26px;
    font-weight: 400;
    padding: 44px 0;
    text-align: center;
}

.banner-warp {
    position: relative;

    .item-img {
        display: block;
        width: 100%;
        height: auto;
        transition: opacity 0.2s;
        background-color: #eeeeee;
    }

    :deep(.el-image__inner) {
        display: block;
        width: 100%;
        height: auto;
    }
}

:deep(.swiper-button-prev),
:deep(.swiper-button-next) {
    box-sizing: border-box;
    top: 50%;
    width: 40px;
    height: 48px;
    margin-top: -24px;
    background-color: rgba(0, 0, 0, 0.122);
    z-index: 2;
    border: none;
    outline: none;
    transition: background-color 0.2s ease;

    &:hover {
        background-color: rgba(0, 0, 0, 0.239);
    }
}

:deep(.swiper-button-prev) {
    left: 0;
    padding-right: 7px;
    border-radius: 0 24px 24px 0;
}

:deep(.swiper-button-next) {
    right: 0;
    padding-left: 7px;
    border-radius: 24px 0 0 24px;
}

:deep(.swiper-button-prev:after),
:deep(.swiper-button-next:after) {
    color: #fff;
    font-size: 16px;
    font-weight: 700;
}

:deep(.swiper-button-prev.swiper-button-disabled),
:deep(.swiper-button-next.swiper-button-disabled) {
    pointer-events: fill;
}

:deep(.swiper-pagination) {
    padding-bottom: 10px;
}

:deep(.swiper-pagination-bullet) {
    position: relative;
    display: inline-block;
    transition: background 0.2s ease;
    width: 8px;
    height: 8px;
    margin-right: 4px;
    border: 1px solid rgba(0, 0, 0, 0.05);
    background: rgba(255, 255, 255, 0.8);
}

:deep(.swiper-pagination-bullet-active) {
    width: 9px;
    height: 9px;
    top: 2px;
    left: 0;
    border: 3px solid rgba(0, 0, 0, 0.1);
}

:deep(.swiper-pagination-bullet-active:before) {
    content: " ";
    width: 9px;
    height: 9px;
    position: absolute;
    border-radius: 50%;
    left: 0;
    background-color: white;
}

.zs-info {
    background-color: #f5f5f5;

    .zs-info__wrap {
        display: flex;
        justify-content: space-between;
        padding: 60px 0 50px;
    }

    .cooperation-col {
        background: #ffffff none repeat scroll 0 0;
        border: 1px solid #eeeeee;
        position: relative;
        text-align: center;
        transition: all 0.3s ease 0s;
        width: 24%;

        &:first-child {
            margin-left: 0;
        }

        .cooperation-image {
            background: url("/assets/images/join/zhaoshang_z.png");
            border: 1px solid #efefef;
            border-radius: 50%;
            height: 118px;
            left: 50%;
            margin-left: -60px;
            position: absolute;
            top: -60px;
            transition: border 0.3s ease 0s;
            width: 118px;
            z-index: 100;
        }

        .standard {
            background-position: -130px 0;
        }

        .mode {
            background-position: -130px -130px;
        }

        .join {
            background-position: 0 -130px;
        }

        h3 {
            color: #333333;
            font-size: 20px;
            font-weight: 700;
            margin: 70px 0 10px;
        }

        p {
            color: #999999;
            height: 36px;
            padding: 3px 50px;
        }

        .cooperation-button {
            background: #ffffff none repeat scroll 0 0;
            border: 1px solid #ffae00;
            color: #ffae00;
            display: block;
            font-size: 14px;
            height: 38px;
            line-height: 38px;
            margin: 20px auto 40px;
            transition: all 0.3s ease 0s;
            width: 138px;
            cursor: pointer;

            &.active {
                background: #ffae00;
                color: #fff;
                font-weight: 700;
            }
        }
    }
}

.ms-info {
    .mode-list {
        display: flex;
        justify-content: center;
        border-bottom: 1px solid #d9deea;
        position: relative;
        text-align: center;

        .mode-col {
            padding-top: 50px;
            height: 35px;
            background-position: top center;
            background-repeat: no-repeat;
            font-size: 16px;
            color: #919fbc;
            text-align: center;
            transition: all 0.2s;
            width: 40%;

            span {
                display: inline-block;
                vertical-align: middle;
                position: relative;
                padding: 0 5px;
                width: auto;
                height: 35px;
            }

            span::after {
                display: none;
                content: "";
                position: absolute;
                bottom: -6px;
                left: 50%;
                margin-left: -5px;
                border: 5px solid #3998f0;
                border-bottom-width: 0;
                border-left-color: transparent;
                border-right-color: transparent;
            }

            &.current span:after {
                display: block;
            }

            &.mode-1 {
                background: url("/assets/images/join/pic01.png") top center no-repeat;

                &.current {
                    background: url("/assets/images/join/picon01.png") top center no-repeat;
                    color: #3998f0;

                    span {
                        border-bottom: 2px solid #3998f0;
                    }

                    span::after {
                        border-bottom-color: #3998f0;
                    }
                }
            }

            &.mode-2 {
                background: url("/assets/images/join/pic02.png") top center no-repeat;

                &.current {
                    background: url("/assets/images/join/picon02.png") top center no-repeat;
                    color: #86c300;

                    span {
                        border-bottom: 2px solid #86c300;
                    }

                    span::after {
                        border-bottom-color: #86c300;
                        border-top-color: #86c300;
                    }
                }
            }
        }
    }

    .mode-detail {
        padding-top: 70px;
        background: center right no-repeat;
        position: relative;

        .mode-detail-content {
            height: 450px;

            .mode-detail-pic {
                position: absolute;
                right: 0;
                top: 50px;
                width: 730px;
            }

            h3 {
                margin: 20px 0;
                font-size: 20px;
                font-weight: 700;
                color: #333;
            }

            p {
                font-size: 14px;
                line-height: 2;
            }

            .learn-more {
                display: block;
                margin-top: 20px;
                padding-left: 30px;
                width: 130px;
                height: 40px;
                line-height: 40px;
                color: #fff;
                font-size: 14px;
                background: url("/assets/images/join/zhaoshang_z.png") no-repeat 0px -420px;
            }
        }
    }
}

.content {
    box-sizing: border-box;
    font-size: 14px;
    line-height: 2.5;
}
</style>
