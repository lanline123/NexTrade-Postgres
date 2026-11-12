<template>
    <div class="font-color button-ico">
        <span v-if="buyBtnStyle == 1" class="iconfont-pc icon-gouwuche"></span>
        <span v-if="buyBtnStyle == 2" class="iconfont-pc icon-gouwuche1"></span>
        <span v-if="buyBtnStyle == 3" class="iconfont-pc icon-jia"></span>
        <span v-if="buyBtnStyle == 4" class="iconfont-pc icon-jia1"></span>
        <template v-if="buyBtnStyle == 5">
            <el-button>{{ $t("购买") }}</el-button>
        </template>
        <template v-if="buyBtnStyle == 6">
            <el-button round type="primary">{{ $t("购买") }}</el-button>
        </template>
        <template v-if="buyBtnStyle == 7">
            <el-button>{{ $t("马上抢") }}</el-button>
        </template>
        <template v-if="buyBtnStyle == 8">
            <el-button round type="primary">{{ $t("马上抢") }}</el-button>
        </template>
    </div>
</template>
<script setup lang="ts">
const props = defineProps({
    buyBtnStyle: { type: Number },
    disabled: { type: Number, default: false }
});
</script>
<style scoped lang="less">
.button-ico {
    & > span {
        font-size: 24px;
    }
}
</style>
