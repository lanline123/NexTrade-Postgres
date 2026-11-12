<template>
    <div class="modal_region_box address-shell-YnEyMjgyZWw" :class="{ 'is-o2o': isStore(), 'is-overseas': isOverseas() }">
        <el-form
            label-position="top"
            label-width="auto"
            :inline="!isOverseas()"
            :inline-message="false"
            ref="formRef"
            :rules="formRules"
            :model="formModel"
            :class="{ 'cross-border-address-form': isOverseas() }"
            autocomplete="off"
            v-if="!loading"
            label-suffix=""
            require-asterisk-position="right"
        >
            <template v-if="!isOverseas()">
                <el-form-item class="form-col-left" :label="$t('收货人')" prop="consignee" :rules="[{ required: true, message: $t('请您填写收货人姓名') }]">
                    <el-input v-model="formState.consignee" clearable :maxlength="20" size="large" :placeholder="$t('姓名或公司名称')" />
                </el-form-item>
                <el-form-item class="form-col-right" :label="$t('手机号码')" prop="mobile">
                    <div class="wrap">
                        <MobileAreaCode v-if="isOverseas()" v-model="formState.mobileAreaCode"></MobileAreaCode>
                        <el-input
                            v-model="formState.mobile"
                            clearable
                            :maxlength="20"
                            :placeholder="$t('手机号码')"
                            size="large"
                            :style="isOverseas() ? 'width: 655px; margin-left: 8px;' : 'width: 100%'"
                        />
                    </div>
                </el-form-item>
            </template>

            <template v-if="isOverseas()">
                <el-form-item class="form-col-full country-form-item" :label="$t('国家/地区')" prop="countryCode" required>
                    <el-select v-model="countryCodeModel" size="large" style="width: 100%">
                        <template #label>
                            <span class="country-option">
                                <img class="country-option__flag" :src="countryFlagUrls[countryCodeModel]" alt="" aria-hidden="true" />
                                {{ $t(currentCountry.name) }}
                            </span>
                        </template>
                        <el-option v-for="country in countryOptions" :key="country.code" :label="$t(country.name)" :value="country.code">
                            <span class="country-option"
                                ><img class="country-option__flag" :src="countryFlagUrls[country.code]" alt="" aria-hidden="true" />{{ $t(country.name) }}</span
                            >
                        </el-option>
                    </el-select>
                </el-form-item>
                <el-form-item class="form-col-left" :label="$t('收货人')" prop="consignee" :rules="[{ required: true, message: $t('请您填写收货人姓名') }]">
                    <el-input v-model="formState.consignee" clearable :maxlength="20" size="large" :placeholder="$t('姓名或公司名称')" />
                </el-form-item>
                <el-form-item class="form-col-right mobile-form-item" :label="$t('手机号码')" prop="mobile" :rules="formRules.mobile">
                    <div class="wrap">
                        <MobileAreaCode v-model="formState.mobileAreaCode" :custom-style="{ width: '108px', minWidth: '108px' }" />
                        <el-input
                            v-model="formState.mobile"
                            clearable
                            :maxlength="20"
                            :placeholder="$t('手机号码')"
                            size="large"
                            style="flex: 1"
                            @input="refreshMobileValidation"
                        />
                    </div>
                </el-form-item>
                <el-form-item
                    v-if="hasDynamicRegionSelector"
                    class="form-col-full region-form-item"
                    :label="$t('所在地区')"
                    prop="regionNames"
                    :required="hasRequiredDynamicRegion"
                    :rules="formRules.regionNames"
                >
                    <div ref="dynamicRegionPop" class="dynamic-region-summary" @click="openDynamicRegionPicker">
                        <el-input :model-value="dynamicRegionDisplayText" readonly :placeholder="$t('点击选择所在地区')" />
                        <i class="dynamic-region-summary__arrow iconfont-pc icon-xiajiantou" aria-hidden="true"></i>
                    </div>
                    <teleport to="body">
                        <div v-if="showDynamicRegionPicker" class="dynamic-region-popup" :style="dynamicRegionPopupStyle" @click.stop>
                            <div class="dynamic-region-popup__title">
                                <span>{{ $t("选择地区") }}</span
                                ><button type="button" @click="showDynamicRegionPicker = false">×</button>
                            </div>
                            <div class="dynamic-region-popup__tabs">
                                <button
                                    v-for="item in visibleDynamicRegionLevels"
                                    :key="item.index"
                                    type="button"
                                    :class="{ active: activeDynamicRegionIndex === item.index }"
                                    @click="activateDynamicRegionLevel(item.index)"
                                >
                                    {{ dynamicRegionTabLabel(item.level) }}
                                </button>
                            </div>
                            <div v-if="activeDynamicRegionLevel" class="dynamic-region-popup__content">
                                <div
                                    v-if="activeDynamicRegionLevel.mode === 'select'"
                                    class="dynamic-region-popup__list"
                                    :class="{ 'is-three-columns': dynamicRegionOptionColumns === 3 }"
                                >
                                    <button
                                        v-for="option in activeDynamicRegionLevel.options"
                                        :key="getRegionOptionId(option)"
                                        type="button"
                                        :class="{ active: activeDynamicRegionLevel.selectedId === getRegionOptionId(option) }"
                                        @click="selectDynamicRegionOption(option)"
                                    >
                                        {{ option.regionName }}
                                    </button>
                                </div>
                                <div v-else-if="activeDynamicRegionLevel.mode === 'error'" class="dynamic-region-popup__status">
                                    {{ $t("地区加载失败，请重试") }}
                                    <el-button link type="primary" @click="retryActiveDynamicRegion">{{ $t("重试") }}</el-button>
                                </div>
                                <div v-else class="dynamic-region-popup__status">
                                    {{ $t(activeDynamicRegionLevel.mode === "loading" ? "加载中" : "请先完成上一级地区") }}
                                </div>
                            </div>
                        </div>
                    </teleport>
                </el-form-item>
                <el-form-item
                    v-for="level in textDynamicRegionLevels"
                    :key="level.key"
                    class="form-col-left"
                    :label="$t(level.label)"
                    :prop="level.key"
                    :rules="formRules[level.key]"
                >
                    <el-input v-model="crossBorderFields[level.key]" clearable :maxlength="100" :placeholder="$t(level.placeholder)" />
                </el-form-item>
                <el-form-item
                    v-for="field in currentCountry.fields"
                    :key="field.key"
                    class="form-col-left"
                    :label="$t(field.label)"
                    :prop="field.key"
                    :rules="formRules[field.key]"
                >
                    <el-input
                        v-model="crossBorderFields[field.key]"
                        clearable
                        :maxlength="field.key === 'address' || field.key.startsWith('addressLine') ? 255 : 100"
                        :placeholder="$t(field.placeholder)"
                    />
                </el-form-item>
                <template v-if="currentCountry.code === 'CN'">
                    <el-form-item class="form-col-left" :label="$t('固定电话')" prop="telephone"
                        ><el-input v-model="formState.telephone" clearable :maxlength="20" size="large" :placeholder="$t('请输入固定电话')"
                    /></el-form-item>
                    <el-form-item class="form-col-right" :label="$t('邮箱地址')" prop="email"
                        ><el-input v-model="formState.email" clearable :maxlength="50" size="large" :placeholder="$t('请输入邮箱地址')"
                    /></el-form-item>
                </template>
            </template>


            <template v-if="!isOverseas() && !isStore()">
                <el-form-item
                    class="form-col-left"
                    :label="$t('所在地区')"
                    prop="regionIds"
                    :rules="isOverseas() ? [] : [{ required: true, message: $t('请选择收货人所在地区') }]"
                    autoLink
                >
                    <SelectRegion
                        v-model="formState.regionIds"
                        v-model:regionNames="formState.regionNames"
                        :country-region-id="selectedCountry?.regionId"
                        @change="onRegionIdsChange"
                    ></SelectRegion>
                </el-form-item>
                <el-form-item
                    class="form-col-right"
                    :label="$t('详细地址')"
                    prop="address"
                    :rules="[
                        { required: true, message: $t('请您填写详细收货地址') },
                        { min: 3, message: $t('收货人地址过短') }
                    ]"
                >
                    <el-input v-model="formState.address" clearable :maxlength="100" size="large" :placeholder="$t('门牌号、建筑物、街道')" />
                </el-form-item>
                <el-form-item
                    class="form-col-left"
                    :label="$t('固定电话')"
                    prop="telephone"
                    :rules="[]"
                    :wrapper-col="{ span: 8 }"
                    extra=""
                >
                    <el-input v-model="formState.telephone" clearable :maxlength="20" size="large" />
                </el-form-item>
                <el-form-item
                    class="form-col-right"
                    :required="false"
                    :label="$t('邮箱地址')"
                    prop="email"
                    :rules="[]"
                    :wrapper-col="{ span: 8 }"
                >
                    <el-input v-model="formState.email" clearable :maxlength="50" size="large" />
                </el-form-item>
            </template>
        </el-form>
        <div class="address_from">
            <div class="from_item">
                <div class="tit"></div>
                <div class="btn_box">
                    <el-button type="primary" @click="onOk">{{ $t("保存") }}</el-button>
                </div>
            </div>
        </div>
    </div>
</template>
<script setup lang="ts">
import { SelectRegion } from "@/components/region";
import { ref, reactive, onMounted, onUnmounted, computed, nextTick } from "vue";
import { getAddressData, updateAddressData } from "@/api/user/address";
import { getCountryList, getRegionListByRegionId, type CountryRegion } from "@/api/region";
import type { AddressFormState } from "~/types/user/address.d";
import { message } from "ant-design-vue";
import { useCommonStore } from "~/store/common";
import { isOverseas, isStore, staticResource } from "@/utils/util";
import MobileAreaCode from "@/components/form/src/MobileAreaCode.vue";
import {
    CROSS_BORDER_COUNTRIES,
    buildCrossBorderAddressData,
    emptyCrossBorderAddressFields,
    formatCrossBorderAddress,
    getCrossBorderAddressDataFieldKey,
    getCrossBorderCountry,
    normalizeInternationalMobile,
    validateInternationalMobile,
    type CrossBorderAddressField,
    type CrossBorderAddressFieldKey,
    type CrossBorderCountry
} from "@/utils/crossBorderAddress";

const commonStore = useCommonStore();
const emit = defineEmits(["submitCallback", "update:confirmLoading", "close", "change"]);
const props = defineProps({
    id: Number,
    act: { type: String, default: "add" },
    isDialog: Boolean,
    addressId: { type: Number, default: 0 },
    showClose: { type: Boolean, default: true }
});
const query = useRouter().currentRoute.value.query;
const action = ref<string>(props.isDialog ? props.act : String(query.act));
const operation = ref<string>(action.value === "add" ? "create" : "update");
const showPostcode = ref(false);
const countries = ref<CountryRegion[]>([]);
type CountryOption = CrossBorderCountry & { id: string; regionId?: number; regionName: string; regionInputMode?: string };
const countryOptions = ref<CountryOption[]>([]);
const countryFlagUrls = Object.fromEntries(
    CROSS_BORDER_COUNTRIES.map((country) => [country.code, staticResource(`country-flags/${country.code.toLowerCase()}.svg`)])
) as Record<CrossBorderCountry["code"], string>;
showPostcode.value = isOverseas();
const formState = reactive<AddressFormState>({
    consignee: "",
    regionIds: [],
    address: "",
    positionAddress: "",
    longitude: 0,
    latitude: 0,
    mobile: "",
    postcode: "",
    telephone: "",
    email: "",
    mobileAreaCode: "",
    countryCode: "CN",
    regionNames: [],
    addressData: {}
});
const { t } = useI18n();
const loading = ref(true);
const crossBorderFields = reactive<Record<CrossBorderAddressFieldKey, string>>(emptyCrossBorderAddressFields());
const currentCountry = computed<CountryOption>(
    () =>
        countryOptions.value.find((item) => item.code === formState.countryCode) || {
            ...getCrossBorderCountry(formState.countryCode),
            id: String(formState.countryCode || "CN"),
            regionName: getCrossBorderCountry(formState.countryCode).name
        }
);
const countryCodeModel = computed({ get: () => String(formState.countryCode || "CN"), set: (value: string) => applyCountryChange(value) });
type DynamicRegionMode = "pending" | "loading" | "select" | "text" | "error";
type DynamicRegionLevel = CrossBorderAddressField & { mode: DynamicRegionMode; options: CountryRegion[]; selectedId: string; parentId: string };
const dynamicRegionLevels = ref<DynamicRegionLevel[]>([]);
const showDynamicRegionPicker = ref(false);
const dynamicRegionPop = ref<HTMLElement | null>(null);
const activeDynamicRegionIndex = ref(0);
const dynamicRegionPopupPosition = reactive({ top: 0, left: 0 });
const dynamicRegionPopupStyle = computed(() => ({ top: `${dynamicRegionPopupPosition.top}px`, left: `${dynamicRegionPopupPosition.left}px` }));
const activeDynamicRegionLevel = computed(() => dynamicRegionLevels.value[activeDynamicRegionIndex.value]);
const hasDynamicRegionSelector = computed(() => dynamicRegionLevels.value.some((level) => level.mode !== "text"));
const textDynamicRegionLevels = computed(() => dynamicRegionLevels.value.filter((level) => level.mode === "text"));
const hasRequiredDynamicRegion = computed(() => dynamicRegionLevels.value.some((level) => level.required && level.mode !== "text"));
const dynamicRegionDisplayText = computed(() =>
    dynamicRegionLevels.value
        .filter((level) => level.mode !== "text")
        .map((level) => crossBorderFields[level.key])
        .filter(Boolean)
        .join(" ")
);
const visibleDynamicRegionLevels = computed(() =>
    dynamicRegionLevels.value.flatMap((level, index) => {
        if (level.mode === "text") return [];
        if (index === 0) return [{ level, index }];
        const previous = dynamicRegionLevels.value[index - 1];
        return previous?.mode === "select" && Boolean(previous.selectedId) && level.mode === "select" ? [{ level, index }] : [];
    })
);
const dynamicRegionOptionColumns = computed<3 | 4>(() => {
    const max = (activeDynamicRegionLevel.value?.options || []).reduce((value, item) => Math.max(value, String(item.regionName || "").length), 0);
    return max > 16 ? 3 : 4;
});
let regionRequestVersion = 0;
const getRegionOptionId = (item?: CountryRegion) => String(item?.regionId || "");
const createDynamicRegionLevels = () =>
    currentCountry.value.regionFields.map((field) => ({ ...field, mode: "pending" as DynamicRegionMode, options: [], selectedId: "", parentId: "" }));
const resetDynamicRegionLevelsAfter = (index: number) =>
    dynamicRegionLevels.value.slice(index + 1).forEach((level) => {
        level.mode = "pending";
        level.options = [];
        level.selectedId = "";
        level.parentId = "";
        crossBorderFields[level.key] = "";
    });
const setTextRegionLevels = (index: number) =>
    dynamicRegionLevels.value.slice(index).forEach((level) => {
        level.mode = "text";
        level.options = [];
        level.selectedId = "";
        level.parentId = "";
    });
// 下级接口无数据时，当前节点即为叶子节点，移除无效的后续选择层级。
const removeDynamicRegionLevelsAfter = (index: number) => {
    dynamicRegionLevels.value.slice(index + 1).forEach((level) => {
        crossBorderFields[level.key] = "";
    });
    dynamicRegionLevels.value.splice(index + 1);
    activeDynamicRegionIndex.value = Math.min(activeDynamicRegionIndex.value, Math.max(0, dynamicRegionLevels.value.length - 1));
};
const loadDynamicRegionLevel = async (
    index: number,
    parentId: string,
    restoreIds: string[] = [],
    restoreNames: string[] = [],
    version = regionRequestVersion
): Promise<void> => {
    const level = dynamicRegionLevels.value[index];
    if (!level || version !== regionRequestVersion) return;
    level.mode = "loading";
    level.parentId = parentId;
    try {
        const options = await getRegionListByRegionId(parentId);
        if (version !== regionRequestVersion) return;
        if (!options.length) {
            // 根节点无选项时保留文本输入兜底；已选择的节点没有下级时直接结束选择。
            if (index === 0) setTextRegionLevels(index);
            else {
                crossBorderFields[level.key] = "";
                dynamicRegionLevels.value.splice(index);
            }
            return;
        }
        level.options = options;
        level.mode = "select";
        const selected = options.find((item) => getRegionOptionId(item) === String(restoreIds[index] || ""));
        if (selected) {
            level.selectedId = getRegionOptionId(selected);
            crossBorderFields[level.key] = selected.regionName;
            if (dynamicRegionLevels.value[index + 1]) await loadDynamicRegionLevel(index + 1, level.selectedId, restoreIds, restoreNames, version);
        } else if (restoreNames[index]) {
            crossBorderFields[level.key] = "";
        }
    } catch (error) {
        if (version === regionRequestVersion) level.mode = "error";
    }
};
const initializeDynamicRegions = async (restoreIds: string[] = [], restoreNames: string[] = []) => {
    const version = ++regionRequestVersion;
    dynamicRegionLevels.value = createDynamicRegionLevels();
    activeDynamicRegionIndex.value = 0;
    const firstLevel = dynamicRegionLevels.value[0];
    if (!firstLevel) return;
    const rootId = String(currentCountry.value.id || "");
    if (!rootId || !Number.isFinite(Number(rootId)) || Number(rootId) <= 0) {
        firstLevel.mode = "error";
        message.error(t("当前地区暂不支持添加地址，请联系客服解决"));
        return;
    }
    await loadDynamicRegionLevel(0, rootId, restoreIds, restoreNames, version);
};
const loadCountryOptions = async () => {
    const result = await getCountryList();
    countries.value = result || [];
    countryOptions.value = countries.value
        .map((region) => {
            const country = CROSS_BORDER_COUNTRIES.find((item) => item.code === String(region.standardCode || "").toUpperCase());
            return country
                ? {
                      ...country,
                      id: String(region.regionId),
                      regionId: region.regionId,
                      name: region.regionName || country.name,
                      regionName: region.regionName || country.name,
                      dialCode: String(region.mobileAreaCode || country.dialCode).replace(/\D/g, "") || country.dialCode
                  }
                : null;
        })
        .filter((item): item is CountryOption => Boolean(item));
    const selected = countryOptions.value.find((item) => item.code === formState.countryCode) || countryOptions.value[0];
    if (selected) {
        formState.countryCode = selected.code;
        if (!formState.mobileAreaCode) formState.mobileAreaCode = selected.dialCode;
    }
};
const applyCountryChange = (code: string) => {
    const country = countryOptions.value.find((item) => item.code === code) || getCrossBorderCountry(code);
    Object.assign(crossBorderFields, emptyCrossBorderAddressFields());
    formState.countryCode = country.code;
    formState.countryName = country.name;
    formState.mobileAreaCode = country.dialCode;
    formState.regionIds = [];
    formState.regionNames = [];
    formState.address = "";
    formState.postcode = "";
    formState.addressData = {};
    if (country.code !== "CN") {
        formState.telephone = "";
        formState.email = "";
    }
    void initializeDynamicRegions();
};
const selectedCountry = computed(() => countries.value.find((item) => item.standardCode === formState.countryCode));
const onMountedLoad = async () => {
    // 新增跨境地址时，国家列表不是显示基础表单的前置条件。
    // 接口较慢或暂时不可用时也要先展示弹框，避免只显示标题的空白弹窗。
    if (isOverseas() && props.addressId <= 0) {
        loading.value = false;
        try {
            await loadCountryOptions();
            await initializeDynamicRegions();
        } catch (error: any) {
            message.error(error.message);
        }
        return;
    }
    try {
        if (isOverseas()) await loadCountryOptions();
        if (props.addressId > 0) await getAddress();
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
onMounted(onMountedLoad);
const handleDynamicRegionOutsideClick = (event: MouseEvent) => {
    if (dynamicRegionPop.value && !dynamicRegionPop.value.contains(event.target as Node)) showDynamicRegionPicker.value = false;
};
onMounted(() => window.addEventListener("click", handleDynamicRegionOutsideClick));
onUnmounted(() => window.removeEventListener("click", handleDynamicRegionOutsideClick));
const getAddress = async () => {
    try {
        const result = await getAddressData(action.value, { id: props.addressId });
        Object.assign(formState, result);
        if (isOverseas()) {
            const countryCode = String(result.countryCode || "CN").toUpperCase();
            formState.countryCode = countryCode;
            formState.mobileAreaCode = String(result.mobileAreaCode || result.addressData?.mobileAreaCode || currentCountry.value.dialCode).replace(/\D/g, "");
            Object.keys(crossBorderFields).forEach((key) => {
                const fieldKey = key as CrossBorderAddressFieldKey;
                crossBorderFields[fieldKey] = String((result as any)[fieldKey] || result.addressData?.[getCrossBorderAddressDataFieldKey(fieldKey)] || "");
            });
            if (!crossBorderFields.address && result.address && countryCode === "CN") crossBorderFields.address = result.address;
            if (!crossBorderFields.addressLine1 && result.address && countryCode !== "CN") crossBorderFields.addressLine1 = result.address;
            crossBorderFields.postcode = String(result.postcode || result.addressData?.postcode || "");
            const names = Array.isArray(result.regionNames) ? result.regionNames.map(String) : [];
            currentCountry.value.regionFields.forEach((field, index) => {
                crossBorderFields[field.key] = names[index] || crossBorderFields[field.key] || "";
            });
            await initializeDynamicRegions(Array.isArray(result.regionIds) ? result.regionIds.map(String) : [], names);
        }
        formState.postcode = String(result.postcode ?? "");
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const validateMobile = (rule: any, value: any, callback: any) => {
    const regex = commonStore.isOpenMobileAreaCode ? /^(\+?[1-9]\d{1,14})$/ : /^(?:\+86)?1(?:3[0-9]|4[5-9]|5[0-35-9]|6[5-7]|7[0-8]|8[0-9]|9[189])\d{8}$/;
    if (!value) {
        return callback(new Error(t("手机号不能为空")));
    } else if (isOverseas()) {
        const error = validateInternationalMobile(formState.countryCode, String(value), formState.mobileAreaCode);
        if (error) return callback(new Error(t(error)));
        callback();
    } else if (!regex.test(value)) {
        return callback(new Error(t("格式错误，请输入正确的手机号码")));
    } else {
        callback();
    }
};
const validateMail = (rule: any, value: any, callback: any) => {
    if (value && !/^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/.test(value)) {
        return callback(new Error(t("格式错误，请输入正确的邮箱地址")));
    } else {
        callback();
    }
};
const validatePostcode = (rule: any, value: any, callback: any) => {
    if (
        isOverseas() &&
        currentCountry.value.postcodePattern &&
        String(value || "").trim() &&
        !currentCountry.value.postcodePattern.test(String(value).trim())                                                                                   
    ) {
        return callback(new Error(t(currentCountry.value.postcodeMessage || "邮编格式不正确")));
    }
    if (isOverseas() && !currentCountry.value.fields.some((field) => field.key === "postcode" && field.required)) return callback();
    if (showPostcode.value && !String(value ?? "").trim()) {
        return callback(new Error(t("请您填写邮政编码")));
    }
    callback();
};
const formRules = reactive<any>({
    mobile: [{ validator: validateMobile, trigger: "blur", required: true }],
    postcode: [{ validator: validatePostcode, trigger: "blur" }],
    email: [{ validator: validateMail, trigger: "blur" }]
});
const formModel = computed(() =>
    isOverseas() ? { ...formState, ...crossBorderFields, regionNames: dynamicRegionDisplayText.value } : formState
);
const refreshMobileValidation = async () => {
    await nextTick();
    formRef.value?.clearValidate?.("mobile");
    formRef.value?.validateField?.("mobile");
};
const formRulesForCrossBorder = () => {
    if (!isOverseas()) return;
    [...currentCountry.value.regionFields, ...currentCountry.value.fields].forEach((field) => {
        formRules[field.key] = [{ required: field.required, message: `${t("请输入")}${t(field.label)}`, trigger: ["blur", "change"] }];
    });
    formRules.regionNames = [
        {
            validator: (_rule: any, _value: any, callback: any) => {
                if (
                    dynamicRegionLevels.value.some(
                        (level) =>
                            level.required &&
                            (level.mode === "error" || level.mode === "loading" || level.mode === "pending" || (level.mode === "select" && !level.selectedId))
                    )
                )
                    return callback(new Error(t("请选择所在地区")));
                callback();
            },
            trigger: ["blur", "change"]
        }
    ];
};

const formRef = ref<any>(null);
const onOk = async () => {
    formState.postcode = formState.postcode?.trim() ?? "";
    formRulesForCrossBorder();
    await formRef.value.validate();
    try {
        emit("update:confirmLoading", true);
        let payload: Record<string, unknown> = { ...formState, ...(props.addressId ? { addressId: props.addressId } : {}) };
        if (isOverseas()) {
            payload = {
                ...(props.addressId ? { addressId: props.addressId } : {}),
                consignee: String(formState.consignee || "").trim(),
                mobile: normalizeInternationalMobile(formState.countryCode, String(formState.mobile || ""), formState.mobileAreaCode),
                mobileAreaCode: String(formState.mobileAreaCode || currentCountry.value.dialCode).replace(/\D/g, ""),
                countryCode: currentCountry.value.code,
                regionIds: dynamicRegionLevels.value.filter((level) => level.mode === "select" && level.selectedId).map((level) => Number(level.selectedId)),
                regionNames: dynamicRegionLevels.value.map((level) => String(crossBorderFields[level.key] || "").trim()).filter(Boolean),
                address: formatCrossBorderAddress(currentCountry.value.code, crossBorderFields),
                addressData: buildCrossBorderAddressData(currentCountry.value.code, crossBorderFields),
                postcode: String(crossBorderFields.postcode || "").trim()
            };
            if (currentCountry.value.code === "CN") {
                payload.telephone = formState.telephone;
                payload.email = formState.email;
            }
        }
        if (!showPostcode.value) {
            delete payload.postcode;
        }
        const result = await updateAddressData(operation.value, payload);
        message.success(t("操作成功"));
        emit("submitCallback", result);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        emit("update:confirmLoading", false);
    }
};
const onRegionIdsChange = () => {
    // SelectRegion updates regionNames through the same change event when a
    // dynamic cross-border tree is used. Keep the array present for the API
    // contract even when the domestic selector has no names to report.
    formState.regionNames = formState.regionNames || [];
    formRef.value.validateField("regionIds");
};
const dynamicRegionTabLabel = (level: DynamicRegionLevel) => String(crossBorderFields[level.key] || "").trim() || t("请选择");
const canActivateDynamicRegionLevel = (index: number) => {
    if (index === 0) return true;
    const previous = dynamicRegionLevels.value[index - 1];
    return Boolean(
        previous &&
            ((previous.mode === "select" && previous.selectedId) || (previous.mode === "text" && (!previous.required || crossBorderFields[previous.key])))
    );
};
const activateDynamicRegionLevel = (index: number) => {
    if (canActivateDynamicRegionLevel(index)) activeDynamicRegionIndex.value = index;
};
const selectDynamicRegionOption = async (option: CountryRegion) => {
    const index = activeDynamicRegionIndex.value;
    const level = dynamicRegionLevels.value[index];
    if (!level) return;
    level.selectedId = getRegionOptionId(option);
    crossBorderFields[level.key] = option.regionName;
    resetDynamicRegionLevelsAfter(index);
    const next = dynamicRegionLevels.value[index + 1];
    if (next) {
        const version = ++regionRequestVersion;
        await loadDynamicRegionLevel(index + 1, level.selectedId, [], [], version);
        if (dynamicRegionLevels.value[index + 1]?.mode === "select") activeDynamicRegionIndex.value = index + 1;
        else {
            removeDynamicRegionLevelsAfter(index);
            showDynamicRegionPicker.value = false;
        }
    } else {
        removeDynamicRegionLevelsAfter(index);
        showDynamicRegionPicker.value = false;
    }
};
const retryActiveDynamicRegion = async () => {
    const level = activeDynamicRegionLevel.value;
    if (!level) return;
    const version = ++regionRequestVersion;
    await loadDynamicRegionLevel(activeDynamicRegionIndex.value, level.parentId, [], [], version);
};
const openDynamicRegionPicker = () => {
    if (!hasDynamicRegionSelector.value) return;
    const index = dynamicRegionLevels.value.findIndex(
        (level) => level.mode === "pending" || level.mode === "loading" || level.mode === "error" || (level.mode === "select" && !level.selectedId)
    );
    activeDynamicRegionIndex.value = index >= 0 ? index : Math.max(dynamicRegionLevels.value.length - 1, 0);
    const rect = dynamicRegionPop.value?.getBoundingClientRect();
    const width = Math.min(760, window.innerWidth - 48);
    if (rect) {
        dynamicRegionPopupPosition.top = rect.bottom + 4;
        dynamicRegionPopupPosition.left = Math.max(24, Math.min(rect.left, window.innerWidth - width - 24));
    }
    showDynamicRegionPicker.value = true;
};
const onFormSubmit = () => {
    onOk();
};

defineExpose({ onFormSubmit });
</script>
<style lang="less" scoped>
.modal_region_box {
    box-sizing: border-box;

    &.is-overseas {
        padding: 0 24px 12px;
    }

    :deep(.el-form-item__error) {
        position: relative;
        top: 0;
        padding-left: 10px;
    }

    :deep(.is-error .select_region) {
        box-shadow: 0 0 0 1px #ff4d4f inset;
    }
    :deep(.el-form-item__label) {
        font-size: 14px;
        color: #1a1a1a;
    }
    :deep(.el-input__inner) {
        font-size: 14px;
        color: #1a1a1a;
    }
    :deep(.el-textarea__inner) {
        font-size: 14px;
        color: #1a1a1a;
    }
    :deep(.el-select__wrapper) {
        font-size: 14px;
        color: #1a1a1a;
    }
}

.modal_region_box.is-overseas {
    :deep(.cross-border-address-form) {
        display: grid;
        width: 100%;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        align-items: start;
        column-gap: 24px;

        > .el-form-item {
            width: 100%;
            min-width: 0;
            margin-right: 0;
            box-sizing: border-box;
        }

        > .country-form-item,
        > .region-form-item {
            grid-column: 1 / -1;
            width: calc(50% - 12px);
        }

        > .country-form-item .el-form-item__content,
        > .region-form-item .el-form-item__content {
            width: 100%;
            min-width: 0;
        }

        .el-form-item__content,
        .el-input,
        .el-select,
        .dynamic-region-summary {
            width: 100%;
            min-width: 0;
            box-sizing: border-box;
        }

        :deep(.el-input__wrapper),
        :deep(.el-select__wrapper) {
            height: 40px;
            box-sizing: border-box;
        }

        .el-form-item {
            margin-bottom: 16px;
        }

        .el-form-item__label {
            display: flex;
            height: 28px;
            padding: 0;
            line-height: 28px;
        }

        > .el-form-item.form-col-full {
            display: block !important;
            width: calc(50% - 12px);
        }

        .el-form-item__error {
            min-height: 20px;
            margin-top: 0;
            padding-top: 0;
            line-height: 20px;
        }
    }

    :deep(.mobile-area-code .el-select__wrapper) {
        width: 108px !important;
        min-width: 108px !important;
    }

    :deep(.cross-border-address-form > .el-form-item.form-col-left),
    :deep(.cross-border-address-form > .el-form-item.form-col-right) {
        width: 100%;
        margin-right: 0;
    }

    :deep(.cross-border-address-form > .el-form-item .el-form-item__content) {
        width: 100%;
        min-width: 0;
        min-height: 40px;
        align-items: flex-start;
    }

    :deep(.cross-border-address-form .el-input),
    :deep(.cross-border-address-form .el-input__wrapper),
    :deep(.cross-border-address-form .el-select),
    :deep(.cross-border-address-form .el-select__wrapper),
    :deep(.cross-border-address-form .dynamic-region-summary) {
        min-height: 40px;
        height: 40px;
        box-sizing: border-box;
    }
}

.address_from {
    .from_item {
        display: flex;
        justify-content: end;
        margin-bottom: 10px;

        .tit {
            display: inline-block;
            font-family: "宋体";
            margin-right: 5px;
            width: 85px;
            text-align: right;
            line-height: 30px;
        }

        .btn_box {
            display: flex;
            margin-top: 10px;
            margin-bottom: 10px;
        }
    }
}
:deep(.el-form-item__content) {
    width: 412px;
}
:deep(.el-input__wrapper) {
    border-radius: 4px;
}
:deep(.el-form-item.form-col-left) {
    margin-right: 24px;
    margin-bottom: 16px;
    vertical-align: top;
}
:deep(.el-form-item.form-col-right) {
    margin-right: 0;
    margin-bottom: 16px;
    vertical-align: top;
}
:deep(.el-form-item.form-col-full) {
    display: flex !important;
    width: 100%;
    margin-right: 0 !important;
    margin-bottom: 16px;
    .el-form-item__content {
        width: 100%;
        max-width: 100%;
    }
}
:deep(.el-form-item) {
    .el-form-item__label {
        color: #1a1a1a;
        font-size: 14px;
    }
}
:deep(.select_region_box) {
    width: 100%;

    .select_region {
        width: 100%;
        width: 412px;
        box-sizing: border-box;
        height: 40px;
    }
    .select_popup {
        width: 100%;
    }
    .select_region i {
        margin-left: auto;
    }
}

:deep(.region-select-full-width .el-select__wrapper) {
    width: 150px !important;
    height: 40px !important;
    box-shadow: 0 0 0 1px var(--el-border-color) inset !important;
}

.wrap {
    display: flex;
    justify-content: space-between;
    width: 100%;
}
:deep(.form-col-full) {
    width: 100%;
    margin-right: 0 !important;
}
.country-option {
    display: inline-flex;
    align-items: center;
    gap: 8px;
}
.country-option__flag {
    width: 24px;
    height: 18px;
    flex: 0 0 24px;
    object-fit: cover;
    border: 1px solid rgba(0, 0, 0, 0.1);
    border-radius: 2px;
}
.dynamic-region-summary {
    position: relative;
    width: 100%;
    cursor: pointer;
}
.dynamic-region-summary__arrow {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: #999;
    font-size: 12px;
    line-height: 1;
    pointer-events: none;
}
.dynamic-region-popup {
    position: fixed;
    z-index: 3000;
    width: 760px;
    max-width: calc(100vw - 48px);
    padding: 14px 10px 10px;
    box-sizing: border-box;
    border: 1px solid #eee;
    border-radius: 12px;
    background: #fff;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.14);
    color: #333;
}
.dynamic-region-popup__title {
    display: flex;
    justify-content: space-between;
    align-items: center;
    height: 42px;
    border-bottom: 1px solid #eee;
    font-size: 18px;
    font-weight: 600;
}
.dynamic-region-popup__title button {
    border: 0;
    background: transparent;
    font-size: 24px;
    cursor: pointer;
}
.dynamic-region-popup__tabs {
    display: flex;
    overflow-x: auto;
    border-bottom: 1px solid #eee;
}
.dynamic-region-popup__tabs button {
    min-width: 130px;
    height: 44px;
    padding: 0 15px;
    border: 0;
    background: transparent;
    color: #666;
    cursor: pointer;
    font-size: 14px;
    line-height: 44px;
    white-space: nowrap;
}
.dynamic-region-popup__tabs button.active {
    color: var(--general);
    border-bottom: 2px solid var(--general);
}
.dynamic-region-popup__content {
    max-height: 230px;
    overflow: auto;
    padding: 10px 4px 0;
}
.dynamic-region-popup__list {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
}
.dynamic-region-popup__list.is-three-columns {
    grid-template-columns: repeat(3, minmax(0, 1fr));
}
.dynamic-region-popup__list button {
    min-height: 40px;
    padding: 8px 15px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: #333;
    cursor: pointer;
    font-size: 14px;
    line-height: 22px;
    text-align: left;
    white-space: normal;
    overflow-wrap: anywhere;
    word-break: break-word;
}
.dynamic-region-popup__list button:hover,
.dynamic-region-popup__list button.active {
    background: var(--main-bg, #f0f5ff);
    color: var(--main-text, var(--general, #1677ff));
}
.dynamic-region-popup__status {
    padding: 30px 10px;
    text-align: center;
    color: #999;
}
:deep(.el-select__wrapper) {
    width: 100%;
    height: 40px;
    box-shadow: 0 0 0 1px var(--el-border-color) inset !important;
}
:deep(.mobile-area-code .el-select__wrapper) {
    width: 150px !important;
    height: 40px !important;
    box-shadow: 0 0 0 1px var(--el-border-color) inset !important;
}
</style>
