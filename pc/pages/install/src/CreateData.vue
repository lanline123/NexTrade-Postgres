<template>
    <div class="create-container">
        <h2 class="tit">创建数据</h2>
        <div class="content">
            <el-form :model="formState" label-width="auto" size="large" ref="formRef">
                <el-form-item label="后台管理员账号：" prop="account" :rules="[{ required: true, message: '请输入后台管理员账号!' }]">
                    <el-input v-model="formState.account" placeholder="请输入后台管理员账号" />
                    <div class="tips">您的后台管理员账号，用于登录后台管理系统， 请牢记！</div>
                </el-form-item>
                <el-form-item label="后台管理员密码：" prop="password" :rules="[{ required: true, message: '请输入后台管理员密码!' }]">
                    <el-input type="password" v-model="formState.password" placeholder="请输入后台管理员密码" />
                    <div class="tips">您的后台管理员密码，用于登录后台管理系统， 请牢记！</div>
                </el-form-item>
                <el-form-item label="数据库类型：" :rules="[{ required: true, message: '请选择数据库类型!' }]">
                    <el-radio-group v-model="formState.sql">
                        <el-radio :value="0">带演示数据</el-radio>
                        <el-radio :value="1">纯净版(无演示数据)</el-radio>
                    </el-radio-group>
                </el-form-item>
                <el-form-item label="数据库地址：" prop="dbhost" :rules="[{ required: true, message: '请输入数据库地址!' }]">
                    <el-input v-model="formState.dbhost" placeholder="请输入数据库地址" />
                </el-form-item>
                <el-form-item label="数据库名称：" prop="dbname" :rules="[{ required: true, message: '请输入数据库名称!' }]">
                    <el-input v-model="formState.dbname" placeholder="请输入数据库名称" />
                </el-form-item>
                <el-form-item label="数据库端口：" prop="dbport" :rules="[{ required: true, message: '请输入数据库端口!' }]">
                    <el-input v-model="formState.dbport" placeholder="请输入数据库端口" />
                </el-form-item>
                <el-form-item label="数据库用户名：" prop="dbuser" :rules="[{ required: true, message: '请输入数据库用户名!' }]">
                    <el-input v-model="formState.dbuser" placeholder="请输入数据库用户名" />
                </el-form-item>
                <el-form-item label="数据库密码：" prop="dbpwd">
                    <el-input type="password" v-model="formState.dbpwd" placeholder="请输入数据库密码" />
                </el-form-item>
                <el-form-item label="Redis地址：" prop="redishost" :rules="[{ required: true, message: '请输入Redis地址!' }]">
                    <el-input v-model="formState.redishost" placeholder="请输入Redis地址" />
                </el-form-item>
                <el-form-item label="Redis端口：" prop="redisport" :rules="[{ required: true, message: '请输入Redis端口!' }]">
                    <el-input v-model="formState.redisport" placeholder="请输入Redis端口" />
                </el-form-item>
                <el-form-item label="Redis密码：" prop="redispwd">
                    <el-input type="password" v-model="formState.redispwd" placeholder="请输入Redis密码" />
                </el-form-item>
                <el-form-item label="RedisSelect库：" prop="redisselect" :rules="[{ required: true, message: '请输入RedisSelect库!' }]">
                    <el-input type="number" :min="0" :max="16" v-model="formState.redisselect" placeholder="请输入RedisSelect库" @input="validateInput" />
                    <div class="tips">只能填0-16之间的数字，0表示使用默认库</div>
                </el-form-item>
                <!-- <el-form-item label=" ">
                    <el-button style="width: 100%;" type="primary" size="large" @click="onSubmit">提 交</el-button>
                </el-form-item> -->
            </el-form>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, onMounted, computed, shallowRef } from "vue";
import { updateData } from "@/api/install/install";
import { message } from "ant-design-vue";
const emit = defineEmits(["onDisabled"]);
const formRef = shallowRef();
const formState = ref<any>({
    account: "admin",
    password: "",
    sql: 0,
    dbhost: "127.0.0.1",
    dbname: "nextrade",

    dbport: "5432",

    dbuser: "root",
    dbpwd: "",
    redispwd: "",

    redishost: "127.0.0.1",
    redisport: "6379",

    redisselect: 0
});
const isTrue = computed(() => {
    const allFieldsNotEmpty = Object.values(formState.value).every((value) => value !== "");
    if (allFieldsNotEmpty) {
        return true;
    } else {
        return false;
    }
});
const validateInput = (value: number) => {
    // 将当前输入值转为数字
    const numValue = Number(value);
    // 如果输入的值不在 0 到 16 的范围内，则重置为 0
    if (numValue < 0 || numValue > 16) {
        formState.value.redisselect = 0;
    } else {
        formState.value.redisselect = numValue;
    }
};
const { t } = useI18n();
// 表单通过验证后提交
const onSubmit = async () => {
    await formRef.value.validate();
    try {
        await updateData(formState.value);
        message.success(t("操作成功"));
        emit("onDisabled", { type: "over", userInfo: { account: formState.value.account, password: formState.value.password } });
    } catch (error: any) {
        message.error(error.message);
    }
};
onMounted(() => {
    emit("onDisabled", true);
});
defineExpose({ onSubmit });
</script>
<style lang="less" scoped>
.create-container {
    width: 1190px;
    .tit {
        padding: 30px 0;
        color: #333;
        text-align: center;
        font-size: 18px;
    }
    .content {
        font-size: 14px;
        padding: 0 30%;
        line-height: 28px;
        .tips {
            margin-top: 5px;
            line-height: 13px;
            font-size: 13px;
            color: #999;
        }
        :deep(.el-form-item--large) {
            margin-bottom: 15px;
        }
    }
}
</style>
