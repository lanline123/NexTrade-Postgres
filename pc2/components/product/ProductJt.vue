<template>
    <!-- 新版 UI：与 SKU / 属性新版无图标签风格一致 -->
    <div class="product-sku-info new-ui" v-if="uiVersion === 'new' && groupNums.length > 0">
        <div class="sku-item">
            <div class="sku-header">
                <div class="tit">{{ $t("阶梯") }}</div>
            </div>
            <div class="sku-tag flex-wrap">
                <template v-for="(value, i) in groupNums">
                    <div :class="{
                        tag: true,
                        checked: value.checked,
                        disabled: !value.checked && type === 'invited'
                    }" @click="onChecked(value, i)">
                        <p>{{ value.groupNum }}{{ $t("人团") }}</p>
                    </div>
                </template>
            </div>
        </div>
    </div>
    <!-- 旧版 UI：保持原有展示 -->
    <div class="product-sku-info" v-else-if="groupNums.length > 0">
        <div class="sku-item flex align-start">
            <div class="tit">{{ $t("阶梯") }}：</div>
            <div class="sku-tag flex-wrap">
                <template v-for="(value, i) in groupNums">
                    <div :class="{
                        tag: true,
                        checked: value.checked,
                        disabled: !value.checked && type === 'invited'
                    }" @click="onChecked(value, i)">
                        <p>{{ value.groupNum }}{{ $t("人团") }}</p>
                        <i class="icon-gou"></i>
                    </div>
                </template>
            </div>
        </div>
    </div>
</template>

<script setup>
const groupNums = defineModel("groupNums", {
    default: [{}]
})

const groupNumKey = defineModel("groupNumKey")
const currentGroupNum = defineModel("currentGroupNum")

const props = defineProps({
    type: {
        type: String,
        default: ""
    },
    // UI 版本：old 老版，new 新版
    uiVersion: {
        type: String,
        default: "old"
    }
})

watch(() => groupNumKey, (value) => {
    groupNums.value.forEach((item) => {
        item.checked = item.groupNumKey === groupNumKey.value;
    });
}, { deep: true, immediate: true })

const onChecked = (value, i) => {
    if (value.groupNumKey !== groupNumKey.value && props.type === 'invited') {
        return;
    }

    groupNums.value.forEach((item, index) => {
        item.checked = false;
    });
    value.checked = true;
    groupNumKey.value = value?.groupNumKey;
    currentGroupNum.value = value?.groupNum;
};
</script>
<style lang="less" scoped>
.outstock {
    color: #c5c5c5 !important;
}

.product-sku-info {
    .sku-item {
        margin-bottom: 25px;

        .tit {
            color: #999;
            height: 33px;
            line-height: 32px;
            padding-right: 10px;
        }

        .sku-tag {
            flex: 1;
            gap: 10px;

            .tag {
                background-color: #ffffff;
                border: 1px solid #e7e7e7;
                color: #333;
                height: 30px;
                line-height: 32px;
                padding: 0 6px;
                cursor: pointer;
                min-width: 30px;
                text-align: center;
                position: relative;
                justify-content: center;
                display: flex;
                align-items: center;

                img {
                    width: 20px;
                    height: 20px;
                    margin-right: 5px;
                }
            }

            .checked {
                border-color: var(--general);
                color: #333;

                .icon-gou {
                    display: block;
                }
            }

            .disabled {
                border: 1px dashed #eee;
                color: #999;
                cursor: not-allowed;
            }
        }
    }
}

// 新版 UI：与 SKU / 属性新版无图标签风格保持一致
.product-sku-info.new-ui {
    padding: 0;
    margin-bottom: 24px;

    .sku-item {
        margin-bottom: 24px;

        &:last-child {
            margin-bottom: 0;
        }
    }

    .sku-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 12px;

        .tit {
            height: auto;
            margin-bottom: 0;
            line-height: 22px;
            padding-right: 0;
            color: #999;
            font-size: 14px;
            font-weight: 500;
        }
    }

    .sku-tag {
        display: flex;
        flex-wrap: wrap;

        .tag {
            align-items: center;
            background-color: #fff;
            border: 1px solid rgba(31, 31, 51, 0.1);
            border-radius: 8px;
            box-sizing: border-box;
            color: rgba(9, 9, 10, 0.92);
            cursor: pointer;
            display: flex;
            flex-direction: row;
            font-weight: 400;
            height: 40px;
            justify-content: center;
            margin: 0 8px 12px 0;
            max-width: calc(100% - 8px);
            min-width: 48px;
            padding: 0 14px;
            position: relative;
            overflow: hidden;
            font-size: 14px;
            line-height: 1;

            p {
                margin: 0;
            }
        }

        .checked {
            border-color: var(--general);
            color: var(--general);
        }

        .disabled {
            border-style: dashed;
            color: #999;
            cursor: not-allowed;
        }
    }
}
</style>
