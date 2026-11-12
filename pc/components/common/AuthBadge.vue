<template>
    <!-- 企业认证标识：图标 + 可翻译文字（原图片内嵌中文，跨境无法翻译） -->
    <span class="auth-badge" :class="{ 'auth-badge--off': !verified }" :style="{ '--auth-badge-height': `${height}px` }">
        <span class="auth-badge__icon">
            <!-- 按原 auth.png 描摹：V 由左右两笔断开组成 -->
            <svg viewBox="0 0 48 48" aria-hidden="true">
                <path d="M12.5 15h6.3l9.2 15.6L25.2 36z" fill="#fff" />
                <path d="M32.2 15h5.4l-7 12.8-3-5.4z" fill="#fff" />
            </svg>
        </span>
        <span class="auth-badge__text">{{ verified ? $t("已认证") : $t("未认证") }}</span>
    </span>
</template>

<script setup lang="ts">
withDefaults(
    defineProps<{
        /** 是否已认证 */
        verified?: boolean;
        /** 标识高度（px） */
        height?: number;
    }>(),
    {
        verified: false,
        height: 20
    }
);
</script>

<style lang="less" scoped>
.auth-badge {
    --auth-badge-color: #f8aa19;
    display: inline-flex;
    align-items: center;
    box-sizing: border-box;
    height: var(--auth-badge-height);
    padding-right: calc(var(--auth-badge-height) * 0.35);
    border: 1px solid var(--auth-badge-color);
    border-radius: 100px;
    color: var(--auth-badge-color);
    white-space: nowrap;
    vertical-align: middle;
    cursor: pointer;

    &--off {
        --auth-badge-color: #c6c6c6;
        color: #999;
    }
}

.auth-badge__icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: var(--auth-badge-height);
    height: var(--auth-badge-height);
    margin: -1px 0 -1px -1px;
    border-radius: 50%;
    background: var(--auth-badge-color);

    svg {
        width: 100%;
        height: 100%;
    }
}

.auth-badge__text {
    margin-left: calc(var(--auth-badge-height) * 0.2);
    font-size: calc(var(--auth-badge-height) * 0.6);
    line-height: 1;
}
</style>
