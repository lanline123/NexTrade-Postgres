<template>
    <!-- 邀请弹框 -->
    <el-dialog v-model="showInviteDialog" :title="$t('通过链接邀请')" width="600">
        <div class="invite-dialog-content">
            <el-input v-model="inviteLink" :disabled="true" size="large" class="invite-dialog-input" />
            <el-button type="primary" @click="copyInviteLink">{{ $t('复制链接') }}</el-button>
        </div>
        <template #footer>
            <div class="dialog-footer">
                <el-button @click="showInviteDialog = false">{{ $t('关闭') }}</el-button>
            </div>
        </template>
    </el-dialog>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { message } from "ant-design-vue";

const { t } = useI18n();
const showInviteDialog = ref<boolean>(false);
const inviteLink = ref<string>('');

const show = (text: string) => {
    showInviteDialog.value = true;
    inviteLink.value = text;
};

const copyInviteLink = async () => {
    copyText(inviteLink.value, () => {
        message.success(t("复制成功"));
    }, () => {
        message.error(t("复制失败，请手动复制"));
    });
    showInviteDialog.value = false;
};

defineExpose( { show })
</script>

<style scoped lang="less">
.invite-dialog-content {
    min-height: 100px;
    display: flex;
    align-items: center;
    gap: 10px;

    .invite-dialog-input {
        width: 100%;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
}
</style>
