<!--
  系统安装向导页面
  - 引导用户完成系统初始化安装流程
  - 分4个步骤：1.软件许可协议 2.系统环境检测 3.创建数据 4.安装结果
  - 动态加载对应步骤的子组件（Agreement、Detection、CreateData、Success）
  - 支持上一步/下一步导航，提交安装表单
-->
<template>
    <div class="installation-wizard-index">
        <div class="head-row">
            <div class="img-box">
                <img :src="logoBlue" alt="Logo" />
                <Steps :current="current" type="navigation" size="default" :style="stepStyle" :items="steps" />
            </div>
        </div>
        <div class="content-row">
            <component ref="currentComponentRef" :is="currentComponent" :userInfo="userInfo" @onDisabled="onDisabled" />
        </div>
        <div class="btn-row">
            <div class="btns">
                <el-button style="width: 90px" v-if="current != 3" size="large" :icon="ArrowLeft" :disabled="current === 0" @click="prevStep">上一步</el-button>
                <el-button style="width: 90px" v-if="current == 2" type="primary" size="large" @click="onSubmit">提 交</el-button>
                <el-button style="width: 90px" v-if="current < 2" size="large" type="primary" @click="nextStep" :disabled="disabled">
                    下一步<el-icon class="el-icon--right"><ArrowRight /></el-icon>
                </el-button>
            </div>
            <div class="tips" v-show="current === 0">点击下一步, 表示您已经阅读并同意了<a>《软件许可协议》</a>的内容。</div>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { Steps } from "ant-design-vue";
import logoBlue from "@/assets/images/install/logo_blue.png";
import { ref, computed, watch } from "vue";
import Agreement from "./src/Agreement.vue";
import Detection from "./src/Detection.vue";
import CreateData from "./src/CreateData.vue";
import Success from "./src/Success.vue";
import { ArrowLeft, ArrowRight } from "@element-plus/icons-vue";
import { useRouter } from "vue-router";
const query = useRouter().currentRoute.value.query;
const currentComponentRef = ref<any>();
const current = ref<any>(query.current || 0);
const stepStyle = {
    width: "900px",
    marginBottom: "0px"
};
const disabled = ref<boolean>(false);

const steps = [{ title: "软件许可协议" }, { title: "系统环境检测" }, { title: "创建数据" }, { title: "安装结果" }];

const currentComponent = computed(() => {
    if (current.value == 0) {
        return Agreement;
    }
    if (current.value == 1) {
        return Detection;
    }
    if (current.value == 2) {
        return CreateData;
    }
    if (current.value == 3) {
        return Success;
    }
});
const router = useRouter();
const route = useRoute();

const updateUrl = () => {
    router.push({
        query: {
            ...route.query,
            current: current.value
        }
    });
};
const nextStep = () => {
    if (current.value < 3) {
        current.value++;
    }
    updateUrl();
};

const prevStep = () => {
    if (current.value > 0) {
        current.value--;
    }
    updateUrl();
};
const userInfo = ref({
    account: "admin",
    password: ""
});
const onDisabled = (data?: any) => {
    if (data.type == "over") {
        current.value = 3;
        Object.assign(userInfo.value, data.userInfo);
        nextStep();
    } else {
        disabled.value = data;
    }
};
// 表单通过验证后提交
const onSubmit = async () => {
    currentComponentRef.value.onSubmit();
};
</script>

<style lang="less" scoped>
.installation-wizard-index {
    --el-color-primary: #1677ff !important;
    --el-color-primary-light-3: #50a1ff !important;
    --el-color-primary-dark-1: #0050b3 !important;
    --el-color-primary-dark-2: #003a8c !important;
    --el-color-primary-light-9: #ecf5ff !important;
    --el-color-primary-light-7: #a1d9ff !important;
    height: 100vh;
    background-color: #fff;
    .head-row {
        position: fixed;
        top: 0;
        left: 0;
        background-color: #fff;
        z-index: 999;
    }
    .head-row,
    .content-row {
        width: 100%;
        border-bottom: 1px solid #ddd;
        display: flex;
        flex-direction: column;
        justify-content: center;
        align-items: center;
        .img-box {
            height: 90px;
            display: flex;
            justify-content: space-between;
            align-items: center;
            width: 1190px;
            img {
                width: 150px;
            }
            :deep(:where(.css-dev-only-do-not-override-19iuou).ant-steps.ant-steps-navigation .ant-steps-item::before) {
                bottom: -17px !important;
            }
        }
    }
    .content-row {
        border: none;
        padding: 90px 0 100px 0;
        background-color: #fff;
    }
    .btn-row {
        position: fixed;
        bottom: 0;
        left: 0;
        width: 100%;
        border-top: 1px solid #ddd;
        display: flex;
        flex-direction: column;
        justify-content: center;
        align-items: center;
        text-align: center;
        background-color: #fff;
        .btns {
            width: 1190px;
            padding: 30px 0;
        }
        .tips {
            position: absolute;
            bottom: 8px;
        }
    }
}
</style>
