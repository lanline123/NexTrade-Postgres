<template>
    <div>
        <p class="promInfo">
            <span class="span1">
                <i class="font-color">{{ $t("温馨提示") }}：</i><br />
                {{ $t("因厂家更改产品、产地等不作提前通知，且每位咨询者购买情况、提问时间等不同。") }}
                {{ $t("回复的内容仅在一定的时期内有参考意义，若由此给您带来不便请多多谅解，敬请谅解，谢谢！") }}
            </span>
            <span class="span2">
                <el-button type="primary" size="small" @click="onChangeVisible">{{ $t("咨询客服") }}</el-button>
                <br />
            </span>
        </p>
        <div class="message_wrapper" v-loading="loading">
            <div class="eachInfo" v-for="item in consultationList">
                <div class="fl flex">
                    <div class="tit">{{ item.username }}</div>
                    <div class="msg">{{ item.addTime }}</div>
                </div>
                <div class="fl flex">
                    <div class="tit">{{ $t("咨询内容") }}：</div>
                    <div class="msg">
                        {{ item.content }}
                    </div>
                </div>
                <div class="fl flex" v-if="item.reply">
                    <div class="tit red">{{ $t("客服回复") }}：</div>
                    <div class="msg red">
                        {{ item.reply?.content }}
                    </div>
                </div>
                <div class="fl flex" v-if="item.reply">
                    <div class="tit"></div>
                    <div class="msg">
                        {{ item.reply?.addTime }}
                    </div>
                </div>
            </div>
        </div>
        <div class="empty" v-if="consultationList.length == 0 && !loading">「{{ $t("暂无咨询") }}」</div>
        <div class="pagination" v-if="consultationList.length > 0">
            <Pagination v-model:page="page" :size="size" :total="total" @callback="fetchConsultation(true)" />
        </div>
        <div class="modal_region_box">
            <el-dialog v-model="visible" :title="$t('咨询提问')" :footer="null" top="15vh" width="650px">
                <div v-if="!loading" class="consultaion-pop">
                    <div class="card pop-tips">
                        <p>{{ $t("您可以对商品包装、规格、支付、发票、退换货和物流配送类的问题咨询客服。") }}</p>
                        <p>
                            {{ $t("客服人员回复的工作时间为：工作日9:30至18：30，请您耐心等待客服人员的回复。") }}
                            {{ $t("客服人员回复后您的咨询及回复内容会出现在商品的页面中，但您的账号会被隐藏。") }}
                        </p>
                        <p class="red">{{ $t("温馨提示：您可以在会员中心查看留言回复等消息动态。") }}</p>
                    </div>
                    <div class="card pop-product">
                        <img :src="productImage(product)" class="img" :alt="product.productName || ''" />
                        <p class="name">
                            {{ product.productName }}
                        </p>
                    </div>
                    <div class="pop-form">
                        <el-form ref="formRef" label-suffix="：" :model="formState" style="max-width: 550px">
                            <el-form-item :label="$t('咨询内容')" prop="content" :rules="[{ required: true, message: $t('咨询内容不能为空') }]">
                                <el-input v-model="formState.content" type="textarea" :rows="5" />
                            </el-form-item>
                            <el-form-item>
                                <el-button type="primary" @click="onSubmit">{{ $t("提交咨询") }}</el-button>
                            </el-form-item>
                        </el-form>
                    </div>
                </div>
            </el-dialog>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { ref, computed, unref, inject, shallowRef, watch } from "vue";
import type { PropType } from "vue";
import { addConsultation, getProductConsultationList } from "~/api/product/product";
import { Pagination } from "@/components/list";
import { productImage } from "@/utils/format";
import type {
    ProductConsultationItem,
    ProductConsultationFormState,
    ProductConsultationListParams,
    ProductItem
} from "~/types/product/product.d";
import { hasToken, isStore } from "@/utils/util";
// 跨境版 UI 开关（由商详页 Product.vue provide 下发；非跨境时默认原逻辑）
const detailUiVersion = inject<any>("detailUiVersion", null);
const isOverseasUi = computed(() => unref(detailUiVersion) === "overseas");
const props = defineProps({
    product: {
        type: Object as PropType<ProductItem>,
        required: true
    },
    productStock: {
        type: Number,
        default: 0
    },
    productNumber: {
        type: Number,
        default: 0
    }
});
const emit = defineEmits(["callback"]);
const { t } = useI18n();
const loading = ref(true);
const visible = ref(false);
const consultationList = ref<ProductConsultationItem[]>([]);
const total = ref(0);
const page = ref<number>(1);
// 跨境版每页 5 条，o2o 每页 10 条
const size = ref(isOverseasUi.value ? 5 : 10);
const formState = ref<ProductConsultationFormState>({ type: 2 });
const formRef = shallowRef();
const currentShopId = computed(() => {
    const shopId = props.product.shopId;
    return typeof shopId === "number" && Number.isFinite(shopId) ? shopId : undefined;
});
const getConsultationParams = (): ProductConsultationListParams => ({
    productId: props.product.productId,
    page: page.value,
    size: size.value,
    ...(currentShopId.value !== undefined ? { shopId: currentShopId.value } : {})
});
/** 获取商品咨询列表，scrollToTop 仅在分页切换时为 true，初始加载不滚动 */
const fetchConsultation = async (scrollToTop = false) => {
    if (isStore() && currentShopId.value === undefined) {
        consultationList.value = [];
        total.value = 0;
        loading.value = false;
        return;
    }
    try {
        const result = await getProductConsultationList(getConsultationParams());
        consultationList.value = result.records;
        total.value = result.total;
        if (scrollToTop) {
            const tabBarElement: any = document.querySelector(".lyecs_main");
            tabBarElement?.scrollIntoView({ behavior: "smooth", block: "start" });
        }
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
watch(
    [() => props.product.productId, currentShopId],
    () => {
        page.value = 1;
        fetchConsultation();
    },
    { immediate: true }
);
const callback = () => {
    formState.value.content = "";
    formState.value.email = "";
    visible.value = false;
};
const onChangeVisible = () => {
    if (hasToken()) visible.value = true;
};
// 表单通过验证后提交
const onSubmit = async () => {
    const shopId = currentShopId.value;
    if (isStore() && shopId === undefined) {
        message.error(t("门店信息尚未加载完成，请稍后重试"));
        return;
    }
    await formRef.value.validate();
    try {
        formState.value.type = 2;
        await addConsultation({
            ...formState.value,
            productId: props.product.productId,
            ...(shopId !== undefined ? { shopId } : {})
        });
        fetchConsultation();
        callback();
        message.success(t("咨询提交成功"));
    } catch (error: any) {
        message.error(error.message);
    }
};
</script>

<style lang="less" scoped>
.promInfo {
    background: #f8f9f9;
    border: 1px solid #eee;
    display: block;
    height: 100%;
    margin: 10px 0 0;
    overflow: hidden;
    padding: 15px 18px;
    display: flex;

    span {
        height: 62px;
        overflow: hidden;
        color: #666;
        display: block;
    }

    .span1 {
        border-right: 1px solid #eee;
        padding: 0 25px 0 0;
        width: 547px;
        height: 100%;
    }

    .span2 {
        padding: 7px 0 0 24px;
        text-align: center;
        min-width: 108px;

        .item_message {
            background-color: var(--general);
            border: 1px solid var(--general);
            color: var(--main-text);
            cursor: pointer;
            border-radius: 2px;
            display: inline-block;
            height: 16px;
            line-height: 16px;
            padding: 5px 12px;
            text-align: center;
            text-decoration: none;
        }
    }
}
.message_wrapper {
    .eachInfo {
        border-bottom: 1px solid #e8e8e8;
        padding: 15px 13px;
        line-height: 21px;

        .fl {
            margin-top: 5px;

            .tit {
                width: 77px;
            }

            .msg {
                width: 587px;
            }
        }
    }
}

.pc-pagination {
    padding: 15px 0 10px;
    font-size: 12px;
    text-align: right;

    .btn {
        background-color: #f4f4f4;
        border: 1px solid #eeeeee;
        border-radius: 2px;
        font-size: 14px;
        margin-left: 4px;
        padding: 4px 10px;
        color: #333333;

        &:hover {
            background-color: #ff4040;
            border-color: #ff4040;
            color: #ffffff;
            text-decoration: none;
        }
    }

    .gap {
        padding: 0 4px;
        line-height: 28px;
        color: #333333;
    }
}
.pagination {
    padding: 10px 0;
    display: flex;
    justify-content: right;
}
.empty {
    text-align: center;
    padding: 70px 0;
}
.consultaion-pop {
    .card {
        background: none repeat scroll 0 0 #f5f5f5;
        border: 1px solid #e8e8e8;
        padding: 15px 20px;
        margin-top: 10px;
    }
    .pop-tips {
        line-height: 22px;
        color: #666;
        font-size: 12px;
    }
    .pop-product {
        color: #666;
        font-size: 12px;
        display: flex;
        align-items: center;
        img {
            width: 70px;
            margin-right: 20px;
            flex-shrink: 0;
        }
        .name {
            flex: 1;
            min-width: 0;
            margin-right: 20px;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }
        .btn {
            background: #f7f7f7;
            border: 1px solid #ddd;
            color: #666;
            cursor: pointer;
            display: inline-block;
            border-radius: 2px;
            height: 16px;
            line-height: 16px;
            padding: 5px 12px;
            text-align: center;
        }
    }
    .pop-form {
        border: 1px solid #e8e8e8;
        border-top: none;
        padding: 15px 20px;
        .btn {
            background: var(--general);
            border: 1px solid var(--general);
            color: #fff;
            cursor: pointer;
            border-radius: 2px;
            display: inline-block;
            height: 16px;
            line-height: 16px;
            padding: 7px 12px;
            text-align: center;
            text-decoration: none;
        }
    }
}
</style>
