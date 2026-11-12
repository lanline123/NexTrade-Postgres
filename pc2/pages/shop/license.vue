<template>
    <div v-loading="loading" class="license-doc">
        <div class="watermark" aria-hidden="true">
            <span v-for="n in 48" :key="n">{{ watermarkText }}</span>
        </div>
        <div data-content-rev="bWptMjI4Mmo" class="doc-inner">
            <h1 class="doc-title">{{ $t("网店经营者营业执照信息") }}</h1>
            <p class="doc-notice">
                <el-icon class="notice-icon"><InfoFilled /></el-icon>
                {{ $t("根据相关法律法规要求，网店经营相关资质信息公示如下：") }}
            </p>
            <h2 class="doc-section">{{ $t("证照") }}</h2>
            <div v-if="licenseUrl" class="doc-cert">
                <img :src="licenseUrl" :alt="$t('营业执照')" />
            </div>
            <el-empty v-else-if="!loading" :description="$t('暂无营业执照')" />
            <div class="doc-notes">
                <p>{{ $t("注：以上营业执照信息来源于卖家自行申报或市监部门系统数据，具体以市监部门登记为准。") }}</p>
                <p>{{ $t("经营者需确保信息真实有效，平台也将定期核查，如与实际不符，为避免违规，请联系当地市监部门或平台客服更新。") }}</p>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
/**
 * @description 营业执照公示页（独立白页，无站点头尾）
 * - 店铺「查看资质」浮层、门店信息弹窗点击后进入
 * - 只认 businessLicensePic，公示页展示真实执照原图
 */
import { computed, onMounted, ref, watch } from "vue";
import { useRoute } from "vue-router";
import { InfoFilled } from "@element-plus/icons-vue";
import { getShopDetail } from "~/api/product/product";
import { imageFormat } from "@/utils/format";
import { useCommonStore } from "@/store/common";

const { t } = useI18n();
const route = useRoute();
const commonStore = useCommonStore();
const loading = ref(true);
const licensePic = ref("");

const shopId = computed(() => Number(route.query.shopId || 0));
const licenseUrl = computed(() => (licensePic.value ? imageFormat(licensePic.value) : ""));
const watermarkText = computed(() => `${commonStore.shopTitle || commonStore.shopName || ""}${t("营业执照信息公示专用")}`);

useSeo({
    title: computed(() => t("网店经营者营业执照信息"))
});

const loadLicense = async () => {
    loading.value = true;
    licensePic.value = "";
    if (!shopId.value) {
        loading.value = false;
        return;
    }
    try {
        let result: any;
            result = await getShopDetail(shopId.value);
        const pic = result?.businessLicensePic;
        licensePic.value = typeof pic === "string" && pic.trim() ? pic.trim() : "";
    } catch (error) {
        console.error(error);
    } finally {
        loading.value = false;
    }
};

onMounted(loadLicense);
watch(shopId, loadLicense);
</script>

<style scoped lang="less">
.license-doc {
    position: relative;
    min-height: 100vh;
    background: #fff;
    overflow: hidden;
}

.watermark {
    position: absolute;
    inset: -40px;
    z-index: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 80px 40px;
    pointer-events: none;
    transform: rotate(-22deg);
    user-select: none;

    span {
        font-size: 14px;
        color: rgba(0, 0, 0, 0.06);
        white-space: nowrap;
        line-height: 1;
    }
}

.doc-inner {
    position: relative;
    z-index: 1;
    max-width: 920px;
    padding: 28px 40px 48px;
}

.doc-title {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
    line-height: 26px;
    color: #111;
}

.doc-notice {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin: 16px 0 28px;
    font-size: 13px;
    line-height: 20px;
    color: #999;
}

.notice-icon {
    margin-top: 2px;
    color: #409eff;
    font-size: 14px;
}

.doc-section {
    margin: 0 0 16px;
    font-size: 15px;
    font-weight: 700;
    line-height: 22px;
    color: #111;
}

.doc-cert {
    img {
        display: block;
        max-width: 100%;
        height: auto;
        object-fit: contain;
    }
}

.doc-notes {
    margin-top: 40px;
    font-size: 13px;
    line-height: 22px;
    color: #111;
    font-weight: 600;

    p {
        margin: 0 0 10px;
    }
}
</style>
