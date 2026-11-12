<template>
    <div class="success-container">
        <div class="content">
            <div class="tit flex align-center justify-center">
                <el-icon color="#87b848" size="40"><CircleCheckFilled /></el-icon>
                <div>安装成功</div>
            </div>
            <div class="text">
                <div>安装完成后建议删除public/install下的文件</div>
                <div v-if="userInfo">初始后台管理员账户（账号：{{ userInfo.account }} 密码：{{ userInfo.password }} ）</div>
            </div>
            <div class="btns">
                <el-popover placement="bottom" :width="200" offset="30" trigger="hover">
                    <template #reference>
                        <el-button @click="toPage(url + '/mobile/')" class="mr10" size="large">进入前台(H5端)</el-button>
                    </template>
                    <template #default>
                        <QRCode :size="180" :value="url + '/mobile/'"></QRCode>
                        <div style="text-align: center; margin-top: 10px">
                            <div>打开手机微信扫码访问</div>
                            <div>或者直接点击上方按钮访问</div>
                        </div>
                    </template>
                </el-popover>
                <el-button @click="toPage(url + '/')" class="mr10" size="large">进入前台(PC端)</el-button>
                <el-button @click="toPage(url + '/admin/')" type="primary" size="large">进入后台</el-button>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { computed } from "vue";
import { QRCode } from "ant-design-vue";
import { CircleCheckFilled } from "@element-plus/icons-vue";

const props = defineProps({
    userInfo: {
        type: Object,
        default: {}
    }
});
// 获取当前地址栏的完整链接
const url = computed(() => {
    if (process.client) {
        return `${window.location.protocol}//${window.location.host.split(":")[0]}`;
    } else {
        // 在服务端返回空字符串或默认值
        return "";
    }
});

const toPage = (path: string) => {
    window.open(path, "_blank");
};
</script>
<style lang="less" scoped>
.success-container {
    width: 1190px;
    .content {
        width: 100%;
        font-size: 14px;
        line-height: 28px;
        .tit {
            margin-top: 20%;
            font-size: 30px;
            font-weight: bold;
            div {
                margin-left: 20px;
                margin-bottom: 5px;
            }
        }
        .text {
            margin-top: 6%;
            font-size: 16px;
            text-align: center;
        }
        .btns {
            margin-top: 6%;
            text-align: center;
        }
    }
}
</style>
