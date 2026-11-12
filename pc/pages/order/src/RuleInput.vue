<template>
    <div class="input-box">
        <input
            class="tig-input"
            v-model="keyword"
            type="text"
            :style="styleInfo"
            @blur="inpBlur"
            :class="isStatus == 0 ? 'itxt-error' : isStatus == 1 ? 'itxt-succ' : ''"
        />
        <div v-if="isStatus == 0" class="prompt-error">{{ msg }}</div>
    </div>
</template>
<script lang="ts" setup>
import { ref, onMounted, reactive } from "vue";
const props = defineProps({
    keyword: { type: String, default: "" },
    styleInfo: { type: Object, default: {} },
    ruleFn: { type: Function, default: null },
    msg: { type: String, default: "" }
});
const keyword = ref(props.keyword);
const isStatus = ref(-1);
const msg = ref(props.msg);
const inpBlur = () => {
    if (keyword.value) {
        if (props.ruleFn) {
            if (props.ruleFn(keyword.value) == 1) {
                isStatus.value = 1;
            } else {
                isStatus.value = 0;
                msg.value = props.ruleFn(keyword.value);
            }
            return;
        }
        isStatus.value = 1;
    } else {
        isStatus.value = 0;
        msg.value = props.msg;
    }
};
</script>
<style lang="less" scoped>
input {
    transition: all 0.3s;
}
.itxt-succ {
    background: url("/assets/images/invoice/succ-ico.png") no-repeat scroll right center #fff;
}
.itxt-error {
    background: url("/assets/images/invoice/error-ico.png") no-repeat scroll right center #fff;
    border-color: #e4393c;
    color: #e4393c;
}
.prompt-error {
    background: none repeat scroll 0 0 #ffebeb;
    border: 1px solid #ffbdbe;
    color: #e4393c;
    display: inline-block;
    line-height: 18px;
    margin-left: 8px;
    padding: 3px 10px;
}
</style>
