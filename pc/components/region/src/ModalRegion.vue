<template>
    <div @click="onClick" style="display: inline-flex; cursor: pointer">
        <slot></slot>
    </div>
    <div class="modal_region_box" v-if="loaded">
        <el-dialog
            v-model="visible"
            :title="addressId == 0 ? $t('添加收货地址') : $t('修改收货地址')"
            :footer="null"
            top="15vh"
            width="830px"
            :destroy-on-close="true"
            @closed="onAfterClose"
        >
            <div class="address_from" v-if="!loading">
                <div class="from_item">
                    <div class="tit"><span class="red">*</span> {{ $t("收货人") }}：</div>
                    <div class="inp">
                        <input class="tig-input" v-model="formState.consignee" type="text" style="width: 170px" />
                    </div>
                </div>
                <div class="from_item">
                    <div class="tit"><span class="red">*</span> {{ $t("所在地区") }}：</div>
                    <div class="inp">
                        <SelectRegion v-model="formState.regionIds"></SelectRegion>
                    </div>
                </div>
                <div class="from_item">
                    <div class="tit"><span class="red">*</span> {{ $t("详细地址") }}：</div>
                    <div class="inp">
                        <input class="tig-input" type="text" style="width: 270px" />
                    </div>
                </div>
                <div class="from_item">
                    <div class="tit"><span class="red">*</span> {{ $t("联系电话") }}：</div>
                    <div class="inp">
                        <input class="tig-input" type="text" style="width: 170px" />
                        <span>{{ $t("推荐手机，座机需加区号和“-”符号") }}</span>
                    </div>
                </div>
                <div class="from_item">
                    <div class="tit">{{ $t("电子邮箱") }}：</div>
                    <div class="inp">
                        <input class="tig-input" type="text" style="width: 270px" />
                        <span>{{ $t("用来接收订单提醒邮件") }}</span>
                    </div>
                </div>
                <div class="from_item">
                    <div class="tit"></div>
                    <div class="btn_box">
                        <div class="btn" @click="onOk()">{{ $t("确认修改地址") }}</div>
                        <div class="btn info" @click="visible = false">{{ $t("取消") }}</div>
                    </div>
                </div>
            </div>
        </el-dialog>
    </div>
</template>
<script setup lang="ts">
import { SelectRegion } from "@/components/region";
import { ref, reactive, onMounted } from "vue";
import { getAddressData, updateAddressData } from "@/api/user/address";
import { message } from "ant-design-vue";
const emit = defineEmits(["change"]);
const props = defineProps({
    addressId: { type: Number, default: 0 }
});
const loaded = ref(false);
const visible = ref<boolean>(false);
const initState = {
    consignee: "",
    regionIds: []
};
const formState = reactive({ ...initState });
const loading = ref(true);
const onClick = () => {
    visible.value = true;
    loaded.value = true;
    if (props.addressId > 0) {
        getAddress();
    } else {
        loading.value = false;
    }
};
onMounted(() => {});
const onAfterClose = () => {
    Object.assign(formState, { ...initState });
};
const getAddress = async () => {
    try {
        const result = await getAddressData(props.addressId);
        Object.assign(formState, result);
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
const onOk = async () => {
    try {
        const result = await updateAddressData(formState);
        emit("change", result.addressId);
        visible.value = false;
    } catch (error: any) {
        message.error(error.message);
    } finally {
    }
};
</script>
<style lang="less" scoped>
.address_from {
    .from_item {
        display: flex;
        margin-bottom: 10px;

        .tit {
            display: inline-block;
            font-family: "宋体";
            margin-right: 5px;
            width: 85px;
            text-align: right;
            line-height: 30px;
        }

        .inp {
            span {
                color: #888888;
                margin-left: 5px;
            }

            select {
                margin-right: 10px;
            }
        }

        .btn_box {
            display: flex;
            margin-top: 10px;
            margin-bottom: 10px;

            .btn {
                background: #ff6c6c;
                border: 1px solid #ff5e5e;
                color: #fff;
                cursor: pointer;
                border-radius: 2px;
                display: inline-block;
                height: 16px;
                line-height: 16px;
                padding: 10px 25px;
                text-align: center;
                text-decoration: none;
                font-size: 14px;
                margin-right: 20px;
            }

            .info {
                background: #f7f7f7;
                border: 1px solid #ddd;
                color: #666;
            }
        }
    }
}
</style>
