<template>
    <div class=".container-new">
        <div class="top-row">
            <NuxtLink to="/" class="site-logo">
                <img :src="imageFormat(commonStore.shopLogo)" alt="logo" />
            </NuxtLink>
            <div class="search-wrap">
                <input v-model.trim="keyword" type="text" :placeholder="$t('搜索宝贝')" @keydown.enter="onSiteSearch" />
                <button class="btn btn-site" :class="{ 'btn-site--only': !(hasShop && showShopSearch) }" @click="onSiteSearch">{{ $t("搜索") }}</button>
                <button v-if="hasShop && showShopSearch" class="btn btn-shop" @click="onShopSearch">{{ $t("搜本店") }}</button>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { computed, ref } from "vue";
import { imageFormat } from "@/utils/format";
import { useCommonStore } from "@/store/common";
import { isO2o } from "~/utils/util";

const props = defineProps({
    shopInfo: {
        type: Object,
        default: () => ({})
    }
});

const commonStore = useCommonStore();
const keyword = ref("");

const hasShop = computed(() => !!props.shopInfo?.shopId);
// o2o（门店）版不显示“搜本店”，其余版本按后台开关 searchSelfStore 控制
const showShopSearch = computed(() => commonStore.searchSelfStore === 1 && !isO2o());

const onSiteSearch = () => {
    location.href = `/search/?keyword=${encodeURIComponent(keyword.value)}`;
};

const onShopSearch = () => {
    if (props.shopInfo?.shopId) {
        location.href = `/search/?keyword=${encodeURIComponent(keyword.value)}&shopId=${props.shopInfo.shopId}`;
    }
};
</script>

<style scoped lang="less">
.container-new {
    max-width: 1700px;
    width: 100%;
    margin: 0 auto;
    box-sizing: border-box;
}
.top-row {
    height: 86px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding: 16px 0 0 0;
}
.site-logo {
    display: flex;
    align-items: center;
    img {
        width: 135px;
        height: auto;
        object-fit: contain;
    }
}

.search-wrap {
    width: 356px;
    height: 45px;
    background: #fff;
    border: 2px solid #e5e8ec;
    border-radius: 8px;
    display: flex;
    overflow: hidden;
    flex-shrink: 0;
    transition: border-color 0.2s ease;

    &:hover,
    &:focus-within {
        border-color: var(--general);
    }

    input {
        flex: 1;
        border: none;
        outline: none;
        background: transparent;
        padding: 0 14px;
        font-size: 13px;
        color: #333;
        &::placeholder {
            color: #bbb;
        }
    }

    .btn {
        border: none;
        cursor: pointer;
        width: 72px;
        flex-shrink: 0;
        padding: 0;
        font-size: 16px;
        font-weight: 600;
    }

    .btn-site {
        margin: 3px 3px;
        border-radius: 6px 0 0 6px;
        margin-right: 0;
        background: var(--general);
        color: #fff;
    }

    .btn-site--only {
        margin-right: 3px;
        border-radius: 6px;
    }

    .btn-shop {
        margin: 3px 3px;
        margin-left: 0;
        border-radius: 0 6px 6px 0;
        background: var(--primary-light-9, #fff0f3);
        color: var(--general);
    }
}
</style>
