<!-- Cookie 隐私偏好设置。 -->
<template>
    <Teleport to="body">
        <!-- 首次弹窗（底部 banner） -->
        <transition name="fade">
            <div v-if="bannerVisible" class="privacy-consent-mask">
                <transition name="slide-up">
                    <div v-if="bannerVisible" class="privacy-consent-panel">
                        <div class="privacy-consent-content">
                            <div class="privacy-consent-title">{{ $t("隐私设置") }}</div>
                            <div class="privacy-consent-desc">
                                {{
                                    $t(
                                        "当您访问任何网站或应用程序时，它可能会在您的浏览器上存储或检索信息，主要以cookie和类似技术的形式。这些信息可能与您、您的偏好或您的设备有关，主要用于使网站按您的期望工作。这些信息通常不能直接识别你的身份，但它可以给你一个更个性化的网络体验。由于我们尊重您的隐私权，您可以选择不允许使用某些类型的cookie和类似技术。"
                                    )
                                }}
                                <!-- <NuxtLink class="privacy-consent-link" :to="moreInfoLink" target="_blank">
                                    {{ $t("更多的信息") }}
                                </NuxtLink> -->
                            </div>
                        </div>
                        <div class="privacy-consent-actions">
                            <button class="privacy-btn privacy-btn-outline" @click="handleOpenCustomize">
                                {{ $t("自定义") }}
                            </button>
                            <button class="privacy-btn privacy-btn-primary" @click="handleAcceptAll">
                                {{ $t("同意") }}
                            </button>
                        </div>
                    </div>
                </transition>
            </div>
        </transition>

        <!-- 自定义详细设置面板（右侧滑动抽屉） -->
        <transition name="fade">
            <div v-if="customizeVisible" class="privacy-customize-mask" @click.self="closeCustomize">
                <transition name="slide-right">
                    <div v-if="customizeVisible" class="privacy-customize-drawer">
                        <div class="privacy-customize-header">
                            <div class="title">{{ $t("隐私设置") }}</div>
                            <button class="close-btn" :aria-label="$t('关闭')" @click="closeCustomize">
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" viewBox="0 0 16 16">
                                    <path d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708z"/>
                                </svg>
                            </button>
                        </div>

                        <div class="privacy-customize-body">
                            <div class="privacy-customize-intro">
                                {{
                                    $t(
                                        "当您访问任何网站或应用程序时，它可能会在您的浏览器上存储或检索信息，主要以cookie和类似技术的形式。这些信息可能与您、您的偏好或您的设备有关，主要用于使网站按您的期望工作。这些信息通常不能直接识别你的身份，但它可以给你一个更个性化的网络体验。由于我们尊重您的隐私权，您可以选择不允许使用某些类型的cookie和类似技术。"
                                    )
                                }}
                                <!-- <NuxtLink class="privacy-consent-link" :to="moreInfoLink" target="_blank">
                                    {{ $t("更多的信息") }}
                                </NuxtLink> -->
                            </div>

                            <!-- 必要性Cookies -->
                            <div class="privacy-customize-item">
                                <div class="privacy-customize-row">
                                    <span class="name">{{ $t("必要性Cookies") }}</span>
                                    <span class="default-on">{{ $t("默认打开") }}</span>
                                </div>
                                <div class="privacy-customize-desc">
                                    {{
                                        $t(
                                            "这些 Cookie 对我们服务的运行至关重要，也是我们与您之间服务条款的一部分。例如，它们包括用于让您登录安全区域、使用购物车、执行服务交付步骤，以及保持页面正确布局和导航的 Cookie。如果您禁用这些 Cookie，某些服务功能可能无法正常运行。必要的 Cookie 可在未经您同意的情况下被使用。"
                                        )
                                    }}
                                </div>
                            </div>

                            <!-- 功能性 cookies -->
                            <div class="privacy-customize-item">
                                <div class="privacy-customize-row">
                                    <span class="name">{{ $t("功能性 cookies") }}</span>
                                    <el-switch v-model="customSettings.functional" active-color="#000000" />
                                </div>
                                <div class="privacy-customize-desc">
                                    {{ $t("这类 cookies 让本网站可以记住您做过的选择，借此为您提供优化后的内容。") }}
                                </div>
                            </div>

                            <!-- 分析性Cookies -->
                            <div class="privacy-customize-item">
                                <div class="privacy-customize-row">
                                    <span class="name">{{ $t("分析性Cookies") }}</span>
                                    <el-switch v-model="customSettings.analytics" active-color="#000000" />
                                </div>
                                <div class="privacy-customize-desc">
                                    {{ $t("这类 cookies 收集您使用本网站的信息，如您经常访问的页面，或您是否有收到任何报错信息。通过分析这些数据，网站可以进行优化和改进。") }}
                                </div>
                            </div>

                            <!-- 广告性Cookies -->
                            <div class="privacy-customize-item">
                                <div class="privacy-customize-row">
                                    <span class="name">{{ $t("广告性Cookies") }}</span>
                                    <el-switch v-model="customSettings.marketing" active-color="#000000" />
                                </div>
                                <div class="privacy-customize-desc">
                                    {{ $t("这些 cookies 用于提供个性化的广告和内容，通常由第三方广告网络设置。它们能够追踪用户的浏览行为，以便向用户展示相关的广告。") }}
                                </div>
                            </div>
                        </div>

                        <div class="privacy-customize-footer">
                            <button class="privacy-btn privacy-btn-outline" @click="handleSaveCustomize">
                                {{ $t("确认我的选择") }}
                            </button>
                            <button class="privacy-btn privacy-btn-primary" @click="handleAcceptAll">
                                {{ $t("同意") }}
                            </button>
                        </div>
                    </div>
                </transition>
            </div>
        </transition>
    </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";

const { needRedecide, detail, showSettings, acceptAll, saveCustom, closeSettings } = useConsent();

const moreInfoLink = "/article/issue/info?categorySn=privacy";

const bannerMounted = ref(false);

onMounted(() => {
    bannerMounted.value = true;
    window.addEventListener("keydown", handleKeyDown);
});

onUnmounted(() => {
    window.removeEventListener("keydown", handleKeyDown);
    if (import.meta.client) {
        document.body.style.overflow = "";
    }
});

// 首次弹窗（底部 banner）显示条件：未做决定 + 客户端已挂载
const bannerVisible = computed(() => bannerMounted.value && needRedecide.value);
const customizeVisible = computed(() => bannerMounted.value && showSettings.value);
const anyVisible = computed(() => bannerVisible.value || customizeVisible.value);

const customSettings = ref({
    functional: false,
    analytics: false,
    marketing: false
});

// 键盘 ESC 键关闭抽屉
const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape" && customizeVisible.value) {
        closeCustomize();
    }
};

// 监听弹窗/抽屉显示，打开时锁定页面背景滚动以优化体验，不显示浏览器滚动条
watch(
    anyVisible,
    (visible) => {
        if (import.meta.client) {
            if (visible) {
                document.body.style.overflow = "hidden";
            } else {
                document.body.style.overflow = "";
            }
        }
    },
    { immediate: true }
);

watch(
    () => showSettings.value,
    (open) => {
        if (open) {
            if (detail.value) {
                customSettings.value = {
                    functional: !!detail.value.functional,
                    analytics: !!detail.value.analytics,
                    marketing: !!detail.value.marketing
                };
            } else {
                customSettings.value = {
                    functional: false,
                    analytics: false,
                    marketing: false
                };
            }
        }
    }
);

const handleAcceptAll = () => {
    acceptAll();
    closeSettings();
};

const handleOpenCustomize = () => {
    if (detail.value) {
        customSettings.value = {
            functional: !!detail.value.functional,
            analytics: !!detail.value.analytics,
            marketing: !!detail.value.marketing
        };
    } else {
        customSettings.value = {
            functional: false,
            analytics: false,
            marketing: false
        };
    }
    showSettings.value = true;
};

const closeCustomize = () => {
    closeSettings();
};

const handleSaveCustomize = () => {
    saveCustom({ ...customSettings.value });
    closeSettings();
};
</script>

<style lang="less" scoped>
// ============== 动画 Transition ==============
.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.3s ease;
}
.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}

.slide-up-enter-active,
.slide-up-leave-active {
    transition: transform 0.35s cubic-bezier(0.25, 1, 0.5, 1), opacity 0.3s ease;
}
.slide-up-enter-from,
.slide-up-leave-to {
    transform: translateY(30px);
    opacity: 0;
}

.slide-right-enter-active,
.slide-right-leave-active {
    transition: transform 0.35s cubic-bezier(0.16, 1, 0.3, 1);
}
.slide-right-enter-from,
.slide-right-leave-to {
    transform: translateX(100%);
}

// ============== 底部 banner ==============
.privacy-consent-mask {
    position: fixed;
    inset: 0;
    z-index: 9998;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding: 32px 24px;
}

.privacy-consent-panel {
    width: 100%;
    max-width: 960px;
    background: #fff;
    border-radius: 12px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
    padding: 24px 32px;
    display: flex;
    flex-direction: column;
    gap: 20px;
}

.privacy-consent-content {
    width: 100%;
}

.privacy-consent-title {
    font-size: 18px;
    font-weight: 700;
    color: #1a1a1a;
    margin-bottom: 12px;
    letter-spacing: -0.2px;
}

.privacy-consent-desc {
    font-size: 13.5px;
    line-height: 1.7;
    color: #1a1a1a;
    text-align: justify;
}

.privacy-consent-link {
    display: inline-block;
    color: #1d6fdc;
    font-size: 13.5px;
    text-decoration: underline;
    margin-left: 4px;
    font-weight: 500;
    transition: opacity 0.2s;

    &:hover {
        opacity: 0.8;
    }
}

.privacy-consent-actions {
    display: flex;
    gap: 12px;
    justify-content: flex-end;
}

// ============== 右侧滑动抽屉 ==============
.privacy-customize-mask {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    justify-content: flex-end;
}

.privacy-customize-drawer {
    width: 480px;
    max-width: 100%;
    height: 100%;
    background: #fff;
    box-shadow: -8px 0 32px rgba(0, 0, 0, 0.15);
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.privacy-customize-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 24px 28px 16px;
    flex-shrink: 0;

    .title {
        font-size: 18px;
        font-weight: 700;
        color: #1a1a1a;
        letter-spacing: -0.2px;
    }

    .close-btn {
        background: transparent;
        border: none;
        cursor: pointer;
        padding: 6px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        color: #666;
        border-radius: 50%;
        transition: background-color 0.2s, color 0.2s;

        &:hover {
            background-color: #f5f5f5;
            color: #000;
        }
    }
}

.privacy-customize-body {
    padding: 0 28px;
    overflow-y: auto;
    flex: 1;

    &::-webkit-scrollbar {
        width: 6px;
    }
    &::-webkit-scrollbar-thumb {
        background-color: rgba(0, 0, 0, 0.15);
        border-radius: 3px;
    }
    &::-webkit-scrollbar-track {
        background-color: transparent;
    }
}

.privacy-customize-intro {
    font-size: 13.5px;
    line-height: 1.7;
    color: #1a1a1a;
    padding-bottom: 24px;
    border-bottom: 1px solid #e8e8e8;
    text-align: justify;
}

.privacy-customize-item {
    padding: 24px 0;
    border-bottom: 1px solid #e8e8e8;

    &:last-child {
        border-bottom: none;
        padding-bottom: 32px;
    }
}

.privacy-customize-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;

    .name {
        font-size: 15px;
        font-weight: 700;
        color: #1a1a1a;
    }

    .default-on {
        font-size: 13px;
        color: #666;
        font-weight: 500;
    }
}

.privacy-customize-desc {
    font-size: 13px;
    line-height: 1.65;
    color: #4a4a4a;
    text-align: justify;
}

.privacy-customize-footer {
    padding: 20px 28px 28px;
    display: flex;
    gap: 12px;
    border-top: 1px solid #e8e8e8;
    background: #fff;
    flex-shrink: 0;
}

// ============== 按钮及组件微调 ==============
.privacy-btn {
    height: 40px;
    min-width: 120px;
    padding: 0 24px;
    border-radius: 4px;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;

    &:active {
        transform: scale(0.97);
    }
}

.privacy-btn-outline {
    background: #fff;
    color: #1a1a1a;
    border: 1px solid #1a1a1a;

    &:hover {
        background: #f5f5f5;
        border-color: #000;
    }
}

.privacy-btn-primary {
    background: #1a1a1a;
    color: #fff;
    border: 1px solid #1a1a1a;

    &:hover {
        background: #333;
        border-color: #333;
    }
}

// 保证 Element Switch 与整体高品质设计风格契合
:deep(.el-switch) {
    --el-switch-on-color: #000000;
    --el-switch-off-color: #dcdfe6;
    
    .el-switch__core {
        border-color: transparent !important;
    }
}

@media (max-width: 768px) {
    .privacy-consent-mask {
        padding: 16px;
    }

    .privacy-consent-panel {
        padding: 16px 20px;
        gap: 16px;
    }

    .privacy-consent-actions {
        width: 100%;
        .privacy-btn {
            flex: 1;
        }
    }

    .privacy-customize-drawer {
        width: 100%;
    }

    .privacy-customize-footer {
        .privacy-btn {
            flex: 1;
        }
    }
}
</style>
