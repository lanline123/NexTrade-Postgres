<template>
    <div>
        <CommonPopover trigger="click" width="230" @close="showSelectCurrency = false">
            <template #reference>
                <div class="lang-box">
                    <div class="iconfont-pc icon-diqiu">
                        <div class="lang-text">{{ currentLanguage }}</div>
                        <span class="iconfont-pc icon-sanjiaoright"></span>
                    </div>
                </div>
            </template>
            <template #default v-if="i18nStore.XLocaleCode">
                <div class="common-title">
                    {{ $t("语言设置") }}
                </div>
                <el-radio-group :model-value="i18nStore.XLocaleCode" @change="handleRadioChange">
                    <div class="lang-list">
                        <template v-for="item in i18nStore.langList" :key="item.id">
                            <div class="lang-item">
                                <el-radio :value="item.localeCode">{{ item.language }}</el-radio>
                            </div>
                        </template>
                    </div>
                </el-radio-group>

                <div class="common-title">
                    {{ $t("货币设置") }}
                </div>
                <div class="currency-box">
                    <template v-if="!showSelectCurrency">
                        <div class="">{{ currentCurrencyData?.symbol + " - " + currentCurrencyData?.name }}</div>
                        <div class="currency-btn" @click="showSelectCurrency = true">{{ $t("更改") }}</div>
                    </template>
                    <template v-else>
                        <el-select @change="handleChange" :teleported="false" :model-value="currentCurrencyData?.id">
                            <el-option v-for="item in currencyStore?.currencyList" :key="item.id" :label="item.name + ' - ' + item.symbol" :value="item.id" />
                        </el-select>
                    </template>
                </div>
            </template>
        </CommonPopover>
    </div>
</template>
<script lang="ts" setup>
import { useI18nStore } from "@/store/i18n";
import { useCurrencyStore } from "@/store/currency";
const i18nStore = useI18nStore();
const currencyStore = useCurrencyStore();
const showSelectCurrency = ref(false);
const currentCurrencyData = useCookie("currentCurrencyData");
const currentLanguage = useCookie("currentLanguage");
const handleRadioChange = async (value: string | number | boolean | undefined) => {
    if (typeof value === "string") {
        const currentLang = i18nStore.langList.find((item) => item.localeCode === value);

        // 跳转前不写语言 Cookie：目标域名会自行判定语言，写了只会让当前域名残留另一种语言的状态
        const targetDomain = currentLang?.domain?.trim();
        if (targetDomain) {
            const { protocol, pathname, search, hash } = window.location;
            window.location.href = `${protocol}//${targetDomain}${pathname}${search}${hash}`;
            return;
        }

        currentLanguage.value = currentLang?.language;
        i18nStore.setXLocaleCode(value);
        i18nStore.setReacquireLanguage(true);
        window.location.reload();
    }
};
const handleChange = (e: any) => {
    showSelectCurrency.value = false;
    currentCurrencyData.value = currencyStore.currencyList.find((item) => item.id === e)!;
    window.location.reload();
};

const handleStorageChange = (event: any) => {
    // console.log('event', event)
    // return
    if (event.key === "cacheLang") {
        window.location.reload();
    }
};

onMounted(() => {
    window.addEventListener("storage", handleStorageChange);
});
onUnmounted(() => {
    window.removeEventListener("storage", handleStorageChange);
});
</script>
<style lang="less" scoped>
.lang-box {
    .lang-text {
        padding-left: 5px;
        font-size: 12px;
    }

    .icon-diqiu {
        font-size: 19px;
        position: relative;
        display: flex;
        align-items: center;

        .icon-sanjiaoright {
            position: absolute;
            right: -15px;
            bottom: -2px;
            font-size: 12px;
            color: #999;
            transform: rotate(90deg);
        }
    }
}

.el-radio-group {
    width: 100%;
}

.common-title {
    font-weight: 500;
}

.lang-list {
    width: 100%;
    padding-bottom: 5px;
    border-bottom: 1px solid #eee;
    margin-bottom: 10px;
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 5px;
    .lang-item {
        height: 30px;
        font-weight: 400 !important;
        :deep(.el-radio__label) {
            font-weight: normal !important;
        }
    }
}
.currency-box {
    display: flex;
    align-items: center;
    justify-content: space-between;

    padding: 5px 0 0;

    .currency-btn {
        color: var(--general);
        cursor: pointer;
    }
}
</style>
