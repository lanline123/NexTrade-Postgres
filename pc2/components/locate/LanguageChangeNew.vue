<template>
    <div>
        <CommonPopover v-model="popoverVisible" :trigger="triggerMode" :width="254" @close="handleClose">
            <template #reference>
                <slot name="trigger">
                    <div class="lang-box" :class="{ white: isWhite }">
                        <LanguageIcon class="language" width="20" height="20"></LanguageIcon>
                        <div class="lang-text">{{ currentLanguageText }}</div>
                        <ArrowIcon class="arrow" :class="{ rotate: popoverVisible }" width="14" height="14"></ArrowIcon>
                    </div>
                </slot>
            </template>
            <template #default>
                <template v-if="i18nStore.XLocaleCode">
                    <template v-if="showLang">
                        <div class="common-title">
                            {{ $t("语言") }}
                        </div>
                        <el-radio-group :model-value="i18nStore.XLocaleCode" @change="handleRadioChange">
                            <div class="lang-list" :class="{ 'show-lang': showLang && !showCurrency }">
                                <template v-for="item in i18nStore.langList" :key="item.id">
                                    <div class="lang-item">
                                        <el-radio :value="item.localeCode">{{ item.language }} - {{ item.localeCode.toUpperCase() }}</el-radio>
                                    </div>
                                </template>
                            </div>
                        </el-radio-group>
                    </template>
                    <template v-if="showCurrency">
                        <div class="common-title">
                            {{ $t("货币") }}
                        </div>
                        <div ref="currencyBoxRef" class="currency-box">
                            <template v-if="!showSelectCurrency">
                                <div>{{ currentCurrencyData?.symbol + " - " + $t(currentCurrencyData?.name || "") }}</div>
                                <div class="currency-btn" @click="showSelectCurrency = true">{{ $t("更改货币") }}</div>
                            </template>
                            <template v-else>
                                <el-select :model-value="currentCurrencyData?.id" :teleported="false" style="width: 230px" @change="handleChange">
                                    <el-option v-for="item in currencyStore?.currencyList" :key="item.id" :label="formatLabel(item)" :value="item.id" />
                                </el-select>
                            </template>
                        </div>
                    </template>
                </template>
            </template>
        </CommonPopover>
    </div>
</template>
<script lang="ts" setup>
// ===================== Vue 核心 =====================
import { computed, onMounted, onUnmounted, ref, type PropType } from "vue";

// ===================== 图标（内联 svg，SSR 友好） =====================
import ArrowIcon from "~/components/icons/ArrowIcon.vue";
import LanguageIcon from "~/components/icons/LanguageIcon.vue";

// ===================== 组件 =====================
import CommonPopover from "~/components/common/Popover.vue";

// ===================== Store =====================
import { useI18nStore } from "~/store/i18n";
import { useCurrencyStore } from "~/store/currency";

// ===================== 组合式函数 =====================
const { t } = useI18n();

// ===================== Props & Emits =====================
const props = defineProps({
    isWhite: {
        type: Boolean,
        default: false
    },
    showLang: {
        type: Boolean,
        default: true
    },
    showCurrency: {
        type: Boolean,
        default: true
    },
    top: {
        type: Boolean,
        default: true
    },
    triggerMode: {
        type: String as PropType<"hover" | "click">,
        default: "hover"
    },
    langLabelMode: {
        type: String as PropType<"language" | "localeCode">,
        default: "language"
    }
});

// ===================== 响应式状态 =====================
const i18nStore = useI18nStore();
const currencyStore = useCurrencyStore();
const showSelectCurrency = ref<boolean>(false);
const currencyBoxRef = ref<HTMLElement>();
const popoverVisible = ref<boolean>(false);

type CurrencyCookieData = {
    id: number | string;
    symbol: string;
    name: string;
};
const currentCurrencyData = useCookie<CurrencyCookieData | null>("currentCurrencyData");
const currentLanguage = useCookie<string>("currentLanguage");

const currentLanguageText = computed(() => {
    const activeLocale = String(i18nStore.XLocaleCode || "");
    if (props.langLabelMode === "localeCode") {
        return activeLocale ? activeLocale.toUpperCase() : String(currentLanguage.value || "");
    }
    if (!activeLocale) return String(currentLanguage.value || "");
    const matched = i18nStore.langList.find((item) => item.localeCode === activeLocale);
    return matched?.language || String(currentLanguage.value || activeLocale.toUpperCase());
});

// ===================== 工具方法 =====================
const formatLabel = (item: unknown): string => {
    const it = item as { name?: string; symbol?: string };
    return `${t(it.name ?? "")} - ${it.symbol ?? ""}`;
};

const resolveLocaleCode = (value: unknown): string => {
    if (typeof value === "string") return value;
    if (typeof value === "number") return String(value);
    if (value && typeof value === "object") {
        const raw = value as Record<string, unknown>;
        const direct = raw.value;
        if (typeof direct === "string") return direct;
        if (typeof direct === "number") return String(direct);
    }
    return "";
};

// ===================== 事件处理方法 =====================
const handleClose = (): void => {
    if (showSelectCurrency.value) {
        showSelectCurrency.value = false;
    }
};

const handleRadioChange = async (value: string | number | boolean | undefined): Promise<void> => {
    const localeCode = resolveLocaleCode(value);
    if (!localeCode) return;
    if (localeCode === i18nStore.XLocaleCode) {
        popoverVisible.value = false;
        return;
    }

    const currentLang = i18nStore?.langList?.find((item) => item.localeCode === localeCode);

    // 跳转前不写语言 Cookie：目标域名会自行判定语言，写了只会让当前域名残留另一种语言的状态
    const targetDomain = currentLang?.domain?.trim();
    if (targetDomain) {
        const { protocol, pathname, search, hash } = window.location;
        window.location.href = `${protocol}//${targetDomain}${pathname}${search}${hash}`;
        return;
    }

    currentLanguage.value = currentLang?.language || currentLang?.localeCode?.toUpperCase() || localeCode.toUpperCase();
    i18nStore.setXLocaleCode(localeCode);
    i18nStore.setReacquireLanguage(true);

    if (import.meta.client) {
        localStorage.removeItem("cacheLang");
    }
    await nextTick();
    window.location.reload();
};

const handleChange = async (e: unknown): Promise<void> => {
    showSelectCurrency.value = false;
    currentCurrencyData.value = currencyStore.currencyList.find((item) => item.id === e)!;
    await nextTick();
    window.location.reload();
};

const handleStorageChange = (event: StorageEvent): void => {
    if (event.key === "cacheLang") {
        const now = Date.now();
        const w = window as Window & { __lang_reload_at?: number };
        if (w.__lang_reload_at && now - w.__lang_reload_at < 1000) return;
        w.__lang_reload_at = now;
        window.location.reload();
    }
};

// ===================== 生命周期 =====================
onMounted(() => {
    window.addEventListener("storage", handleStorageChange);
});

onUnmounted(() => {
    window.removeEventListener("storage", handleStorageChange);
});
</script>
<style lang="less" scoped>
.lang-box {
    display: flex;
    align-items: center;
    cursor: pointer;
    color: #fff;
    &.white {
        color: #1a1a1a;
        &:hover {
            color: var(--primary-dark-2);
            .arrow {
                color: var(--primary-dark-2);
            }
        }
    }
    .lang-text {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        margin-left: 6px;
        font-size: 14px;
        font-weight: 500;
    }
    .arrow {
        align-self: self-end;
        margin-left: 3px;
        margin-bottom: 2px;
        transition: all 0.2s;
        &.rotate {
            transform: rotate(180deg);
        }
    }
    &:hover {
        color: #fffc;
        .arrow {
            color: #fffc;
        }
    }
}

.common-title {
    font-size: 14px;
    color: #1a1a1a;
    font-weight: bold;
    text-transform: capitalize;
}

.lang-list {
    width: 100%;
    padding-bottom: 5px;
    margin-top: 15px;
    margin-bottom: 10px;
    display: flex;
    flex-direction: column;
    gap: 5px;
    .lang-item {
        height: 30px;
        font-weight: 400 !important;
        :deep(.el-radio__label) {
            font-weight: normal !important;
        }
    }
    &.show-lang {
        padding-bottom: 0;
        border: none;
    }
}
.currency-box {
    position: relative;
    padding: 18px 0 0 0;

    .currency-btn {
        margin-top: 12px;
        color: #1869f5;
        cursor: pointer;
    }
}
</style>
