<template>
    <div class="shop-qualification">
        <div class="score-list">
            <div class="score-row">
                <span class="score-label score-label--primary">{{ $t("综合体验") }}</span>
                <span class="score-right">
                    <el-rate
                        :model-value="Number(shopInfo.productScore || 0)"
                        disabled
                        :colors="rateColors"
                        disabled-void-color="#e5e5e5"
                    />
                    <span class="score-num">{{ ratingFormat(shopInfo.productScore) }}</span>
                </span>
            </div>
            <div class="score-row">
                <span class="score-label">{{ $t("宝贝质量") }}</span>
                <span class="score-num">{{ ratingFormat(shopInfo.productScore) }}</span>
            </div>
            <div class="score-row">
                <span class="score-label">{{ $t("物流速度") }}</span>
                <span class="score-num">{{ ratingFormat(shopInfo.shopStar) }}</span>
            </div>
            <div class="score-row">
                <span class="score-label">{{ $t("服务保障") }}</span>
                <span class="score-num">{{ ratingFormat(shopInfo.shopStar) }}</span>
            </div>
        </div>

        <div v-if="hasLicense" class="qual-block" @click.stop="openLicensePage">
            <div class="qual-text">
                <div class="qual-title">{{ $t("企业资质") }}</div>
                <div class="qual-link">{{ $t("查看商家公示信息") }}<el-icon><ArrowRight /></el-icon></div>
            </div>
            <img class="qual-thumb" :src="licenseThumb" :alt="$t('营业执照')" />
        </div>
    </div>
</template>

<script setup lang="ts">
/**
 * @description 店铺「查看资质」浮层
 * - 展示综合体验 / 宝贝质量 / 物流速度 / 服务保障
 * - 有执照快照时展示企业资质；入口用示例缩略图，点击进公示页看原图
 */
import { computed } from "vue";
import { ArrowRight } from "@element-plus/icons-vue";
import { ratingFormat } from "@/utils/format";
import licenseThumb from "@/assets/images/shop/license-thumb.png";

const props = defineProps<{
    shopInfo?: Record<string, any>;
    pic?: string | null;
    shopId?: number | string | null;
}>();

const router = useRouter();
const rateColors = ["#ff4d4f", "#ff4d4f", "#ff4d4f"];

const shopInfo = computed(() => props.shopInfo || {});
const licensePic = computed(() => shopInfo.value.businessLicensePic ?? props.pic);
const hasLicense = computed(() => typeof licensePic.value === "string" && licensePic.value.trim().length > 0);
const resolvedShopId = computed(() => Number(shopInfo.value.shopId || props.shopId || 0));

const openLicensePage = () => {
    if (!resolvedShopId.value) return;
    router.push({ path: "/shop/license", query: { shopId: String(resolvedShopId.value) } });
};
</script>

<style scoped lang="less">
.shop-qualification {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px 16px 14px;
}

.score-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.score-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    height: 22px;
    font-size: 12px;
    line-height: 18px;
    color: #333;
}

.score-label {
    // min-width: 48px;
    font-size: 14px;
    line-height: 14px;
    flex-shrink: 0;
    color: #11192d;
    font-weight: 400;

    &--primary {
        font-weight: 500;
    }
}

.score-right {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-left: auto;
}

.score-num {
    font-size: 18px;
    font-weight: 500;
    color: #333;
}

.score-right :deep(.el-rate) {
    height: 12px;
    --el-rate-icon-size: 12px;
    line-height: 12px;
}

.qual-block {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    cursor: pointer;
}

.qual-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
}

.qual-title {
    font-size: 14px;
    font-weight: 400;
    line-height: 22px;
    color: #09090aeb;
}

.qual-link {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    margin-top: 4px;
    font-size: 12px;
    line-height: 14px;
    color: rgba(9, 9, 10, .48);

    .el-icon {
        font-size: 12px;
        width: 12px;
        height: 12px;
    }
}

.qual-thumb {
    display: block;
    flex-shrink: 0;
    width: 68px;
    height: 42px;
    padding: 0;
    border: none;
    border-radius: 0;
    background: transparent;
    object-fit: contain;
    object-position: right center;
    box-sizing: border-box;
}
</style>

<style lang="less">
.shop-qualification-popper.el-popover.el-popper {
    border: 1px solid #ececec;
    border-radius: 4px;
    padding: 0;
    background: #fff;
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.12);
    overflow: hidden;
    will-change: transform, opacity;
}

.shop-qualification-popper.el-popover.el-popper[data-popper-placement^="bottom-start"],
.shop-qualification-popper.el-popover.el-popper[data-popper-placement^="top-start"] {
    transform-origin: left top;
}

.shop-qualification-popper.el-popover.el-popper[data-popper-placement^="bottom-end"],
.shop-qualification-popper.el-popover.el-popper[data-popper-placement^="top-end"] {
    transform-origin: right top;
}

.shop-qualification-popover-enter-active {
    transition:
        opacity 0.1s cubic-bezier(0.08, 0.82, 0.17, 1),
        transform 0.1s cubic-bezier(0.08, 0.82, 0.17, 1);
}

.shop-qualification-popover-leave-active {
    transition:
        opacity 0.1s cubic-bezier(0.78, 0.14, 0.15, 0.86),
        transform 0.1s cubic-bezier(0.78, 0.14, 0.15, 0.86);
}

.shop-qualification-popover-enter-from,
.shop-qualification-popover-leave-to {
    opacity: 0;
    transform: scale(0.8);
}

.shop-qualification-popover-enter-to,
.shop-qualification-popover-leave-from {
    opacity: 1;
    transform: scale(1);
}
</style>
