<template>
    <div class="ele-menu">
        <CartFloatEntry variant="card" />
        <div class="card">
            <div class="feedback-menu">
                <el-popover placement="left" :width="174" trigger="click">
                    <div class="ima-div">
                        <el-image :src="qrCodeData" class="link-code-qrimg" fit="cover"></el-image>
                        <div class="phone-text">{{ $t("在手机端查看此商品") }}</div>
                    </div>
                    <template #reference>
                        <div ref="sweepRef" :class="{ 'check-item': isVisible }" class="wrap-btn" @click="onClickOutside">
                            <i class="iconfont-pc icon-saoyisao"></i>
                            {{ $t("扫一扫") }}
                        </div>
                    </template>
                </el-popover>
            </div>
        </div>
        <div class="to-top">
            <div v-if="shouldShowCard" class="card" @click="scrollToTop()">
                <div class="feedback-menu">
                    <div class="wrap-btn">
                        <i class="iconfont-pc icon-shangjiantou"></i>
                        {{ $t("顶部") }}
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { useCommonStore } from "@/store/common";
import { onMounted, onUnmounted, ref } from "vue";
import QRCode from "qrcode";
import CartFloatEntry from "~/components/common/CartFloatEntry.vue";

const commonStore = useCommonStore();
commonStore.loadNav();

const scrollTop = ref(0); // 定义一个响应式变量用于存储滚动距离
const shouldShowCard = ref(false); // 定义一个新的响应式变量用于控制显示和隐藏
function updateScroll() {
    scrollTop.value = window.scrollY; // 更新滚动距离
    shouldShowCard.value = scrollTop.value >= 630; // 根据滚动距离的变化来动态修改 shouldShowCard 的值，达到条件显示的效果
}

// 返回顶部的方法
const scrollToTop = () => {
    window.scrollTo({
        top: 0, // 顶部的位置，0表示最上面
        behavior: "smooth" // 平滑滚动
    });
};

const isVisible = ref(false);
const onClickOutside = () => {
    isVisible.value = !isVisible.value;
};

const qrCodeData = ref<string>("");
const generateQRCodeImage = async (url: string) => {
    const data = url; // 替换为你要转换的链接
    let opts = {
        margin: 0 //二维码留白边距
    };
    qrCodeData.value = await QRCode.toDataURL(data, opts);
};
onMounted(() => {
    generateQRCodeImage(window.location.href);
    window.addEventListener("scroll", updateScroll); // 组件挂载时添加滚动事件监听器
});
onUnmounted(() => {
    window.removeEventListener("scroll", updateScroll); // 组件卸载时移除滚动事件监听器
});
</script>

<style lang="less" scoped>
.ima-div {
    display: flex;
    justify-content: center;
    align-items: center;
    width: 150px;
    flex-direction: column;
    box-sizing: border-box;
    gap: 8px;

    .phone-text {
        font-size: 12px;
        color: rgba(0, 0, 0, 0.55);
    }

    .link-code-qrimg {
        width: 100%;
        height: 100%;
    }
}

.ele-menu {
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    position: fixed;
    right: 2vw;
    bottom: 100px;
    z-index: 11;
    gap: 8px;

    .to-top {
        height: 68px;
        width: 68px;
    }

    .card {
        font-size: 12px;
        position: relative;
        padding: 0;
        border-radius: 6px;
        overflow: hidden;
        box-shadow: 0 4px 10px 0 rgba(0, 0, 0, 0.08);

        .feedback-menu {
            height: 68px;
            width: 68px;
            box-shadow: 0 4px 10px 0 rgba(0, 0, 0, 0.08);

            .wrap-btn {
                height: 68px;
                width: 68px;
                background-color: #fff;
                color: #666666;
                display: flex;
                flex-direction: column;
                justify-content: center;
                align-items: center;
                cursor: pointer;
                border-bottom: 1px solid #e8e8e8;
                transition: background-color 0.3s;

                & > i {
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    height: 28px;
                    aspect-ratio: auto 28 / 28;
                    width: 28px;
                }
            }

            .wrap-btn:hover {
                color: var(--general);
                transition: 0.3s;
            }

            .check-item {
                color: var(--general);
                transition: 0.3s;
            }
        }
    }
}
</style>
