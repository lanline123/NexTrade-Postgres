<template>
    <div class="custom-player" :class="{ 'has-cover-bg': useCoverBg && poster }" :style="coverBgStyle" @mousemove="showControls" @mouseleave="startHideControlsTimer">
        <div class="video-box">
            <video
                ref="videoEl"
                muted
                :poster="useCoverBg && poster ? imageFormat(poster) : undefined"
                @timeupdate="() => updateProgress()"
                @loadedmetadata="onLoaded"
                @ended="onEnded"
                @error="onError"
                @play="onPlay"
                @pause="onPause"
                @click="togglePlay"
            >
                <source :src="imageFormat(src)" :type="type" />
                {{ $t("您的浏览器不支持 HTML5 视频") }}
            </video>
        </div>

        <div class="video-overlay" @click="togglePlay" v-show="isPaused && !isLoading">
            <div class="play-icon"></div>
        </div>

        <div class="loading-indicator" v-show="isLoading && !hasError">
            <div class="spinner"></div>
        </div>

        <div class="error-message" v-show="hasError">
            <p>{{ $t(errorMessage) }}</p>
            <!-- <button @click="retryLoad">重试</button> -->
        </div>

        <div class="controls" :class="{ hidden: !showControlsFlag }">
            <div class="progress-container">
                <!-- 修改进度条，确保事件委托正确 -->
                <div class="progress-bar" ref="progressBar" @click="seek" @mousemove="updateHoverTime" @mouseleave="hideHoverTime">
                    <div class="buffer-progress" :style="{ width: `${bufferProgress}%` }"></div>
                    <div class="play-progress" :style="{ width: `${progress}%` }"></div>
                    <div class="hover-time" :style="{ left: `${hoverPosition}px` }" v-show="showHoverTime">{{ hoverTimeText }}</div>
                </div>
            </div>

            <div class="buttons">
                <button class="control-button" @click="togglePlay" :aria-label="$t('播放或暂停')">
                    <template v-if="isPaused">
                        <svg viewBox="0 0 24 24" width="24" height="24">
                            <path fill="white" d="M8 5v14l11-7z" />
                        </svg>
                    </template>
                    <template v-else>
                        <svg viewBox="0 0 24 24" width="24" height="24">
                            <path fill="white" d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" />
                        </svg>
                    </template>
                </button>

                <div class="volume-control">
                    <button class="control-button" @click="toggleMute" :aria-label="$t('静音或取消静音')">
                        <svg viewBox="0 0 24 24" width="22" height="22">
                            <path fill="white" :d="volumeIconPath" />
                        </svg>
                    </button>
                </div>

                <div class="time">
                    <span>{{ currentTime }}</span>
                    <span class="time-separator">/</span>
                    <span>{{ duration }}</span>
                </div>

                <div class="right-controls">
                    <button class="control-button" @click="toggleFullscreen" :aria-label="$t('全屏')">
                        <svg viewBox="0 0 24 24" width="24" height="24">
                            <path
                                fill="white"
                                :d="
                                    isFullScreen
                                        ? 'M5 16h3v3h2v-5H5v2zm3-8H5v2h5V5H8v3zm6 11h2v-3h3v-2h-5v5zm2-11V5h-2v5h5V8h-3z'
                                        : 'M7 14H5v5h5v-2H7v-3zm-2-4h2V7h3V5H5v5zm12 7h-3v2h5v-5h-2v3zM14 5v2h3v3h2V5h-5z'
                                "
                            />
                        </svg>
                    </button>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { imageFormat } from "~/utils/format";

interface VideoProps {
    src: string;
    type?: string;
    autoplay?: boolean;
    defaultVolume?: number;
    showVideo?: boolean;
    // 封面图地址，配合 useCoverBg 使用
    poster?: string;
    // 为 true 时使用封面图作为视频背景（替代默认的黑色背景）
    useCoverBg?: boolean;
}

const props = withDefaults(defineProps<VideoProps>(), {
    type: "video/mp4",
    autoplay: false,
    defaultVolume: 1,
    showVideo: true,
    poster: "",
    useCoverBg: false
});

// 开启封面背景时，用封面图填充视频区域，避免出现黑边
// 通过 CSS 变量传给 ::before 伪元素，单独对背景做模糊，避免视频被一起模糊
const coverBgStyle = computed(() => {
    if (!props.useCoverBg || !props.poster) return {};
    return {
        "--cover-bg-url": `url(${imageFormat(props.poster)})`
    };
});

const emit = defineEmits(["play", "pause", "error", "loaded", "ended"]);

// 视频元素引用和状态
const videoEl = ref<HTMLVideoElement | null>(null);
const isPaused = ref(true);
const isMuted = ref(true);
const volume = ref(props.defaultVolume);
const currentTimeValue = ref(0);
const durationValue = ref(0);
const isLoading = ref(true);
const hasError = ref(false);
const errorMessage = ref("视频加载失败");
const isFullScreen = ref(false);
const playbackRate = ref(1);
const bufferProgress = ref(0);

// 进度条相关状态
const progress = computed(() => (currentTimeValue.value / durationValue.value) * 100 || 0);
const showHoverTime = ref(false);
const hoverPosition = ref(0);
const hoverTimeText = ref("00:00");

// 控制栏显示状态
const showControlsFlag = ref(true);
const controlsTimer = ref<any>(null);

// 时间格式化
const formatTime = (seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
};

const currentTime = computed(() => formatTime(currentTimeValue.value));
const duration = computed(() => formatTime(durationValue.value));

// 音量图标根据音量状态动态变化
const volumeIconPath = computed(() => {
    if (isMuted.value || volume.value === 0) {
        return "M16.5 12c0-1.77-1.02-3.29-2.5-4.03v2.21l2.45 2.45c.03-.2.05-.41.05-.63zm2.5 0c0 .94-.2 1.82-.54 2.64l1.51 1.51C20.63 14.91 21 13.5 21 12c0-4.28-2.99-7.86-7-8.77v2.06c2.89.86 5 3.54 5 6.71zM4.27 3L3 4.27 7.73 9H3v6h4l5 5v-6.73l4.25 4.25c-.67.52-1.42.93-2.25 1.18v2.06c1.38-.31 2.63-.95 3.69-1.81L19.73 21 21 19.73l-9-9L4.27 3zM12 4L9.91 6.09 12 8.18V4z";
    } else if (volume.value < 0.5) {
        return "M5 9v6h4l5 5V4L9 9H5zm7-.17v6.34L9.83 13H7v-2h2.83L12 8.83z";
    } else {
        return "M3 9v6h4l5 5V4L7 9H3zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02zM14 3.23v2.06c2.89.86 5 3.54 5 6.71s-2.11 5.85-5 6.71v2.06c4.01-.91 7-4.49 7-8.77s-2.99-7.86-7-8.77z";
    }
});

// 播放控制
const togglePlay = (): void => {
    if (!videoEl.value) return;
    if (videoEl.value.paused) {
        videoEl.value
            .play()
            .then(() => {
                isPaused.value = false;
                isLoading.value = false;
            })
            .catch((err) => {
                console.error("播放失败:", err);
                hasError.value = true;
                errorMessage.value = "视频播放失败，请重试";
            });
    } else {
        videoEl.value.pause();
        isPaused.value = true;
    }
};

// 同步原生播放状态，保证外部直接操作 video 时图标状态正确
const onPlay = (): void => {
    isPaused.value = false;
    isLoading.value = false;
};

const onPause = (): void => {
    isPaused.value = true;
    showControlsFlag.value = true;
};

// 音量控制
const toggleMute = (): void => {
    if (!videoEl.value) return;
    videoEl.value.muted = !videoEl.value.muted;
    isMuted.value = videoEl.value.muted;
};

const onVolumeChange = (): void => {
    if (!videoEl.value) return;
    videoEl.value.volume = volume.value;
    isMuted.value = volume.value === 0;
};

let lastUpdateTime = 0;
const updateProgress = (immediate = false): void => {
    const now = Date.now();
    if (!immediate && now - lastUpdateTime < 200) return; // 200ms节流，但允许立即更新
    lastUpdateTime = now;

    if (!videoEl.value) return;
    currentTimeValue.value = videoEl.value.currentTime;

    // 更新缓冲进度
    if (videoEl.value.buffered.length > 0) {
        const bufferedEnd = videoEl.value.buffered.end(videoEl.value.buffered.length - 1);
        bufferProgress.value = (bufferedEnd / videoEl.value.duration) * 100;
    }
};

const progressBar = ref<HTMLElement | null>(null);

// 修改 seek 函数，确保使用进度条元素而不是可能的子元素
const seek = (event: MouseEvent): void => {
    if (!videoEl.value || !progressBar.value) return;

    const rect = progressBar.value.getBoundingClientRect();
    const pos = (event.clientX - rect.left) / rect.width;
    videoEl.value.currentTime = pos * videoEl.value.duration;
    currentTimeValue.value = videoEl.value.currentTime;

    // 立即更新进度，不等待下一次 timeupdate 事件
    updateProgress(true);
};

const updateHoverTime = (event: MouseEvent): void => {
    if (!videoEl.value || !videoEl.value.duration || !progressBar.value) return;

    const rect = progressBar.value.getBoundingClientRect();
    const pos = (event.clientX - rect.left) / rect.width;
    const time = pos * videoEl.value.duration;

    hoverPosition.value = event.clientX - rect.left;
    hoverTimeText.value = formatTime(time);
    showHoverTime.value = true;
};

const hideHoverTime = (): void => {
    showHoverTime.value = false;
};

// 全屏控制
const toggleFullscreen = async (): Promise<void> => {
    const container = document.querySelector(".custom-player") as HTMLElement;
    if (!container) return;

    try {
        if (!document.fullscreenElement) {
            await container.requestFullscreen();
            isFullScreen.value = true;
        } else {
            await document.exitFullscreen();
            isFullScreen.value = false;
        }
    } catch (err) {
        console.error("全屏切换失败:", err);
    }
};

// 控制栏显示控制
const showControls = (): void => {
    showControlsFlag.value = true;

    if (controlsTimer.value) {
        clearTimeout(controlsTimer.value);
    }

    startHideControlsTimer();
};

const startHideControlsTimer = (): void => {
    if (controlsTimer.value) {
        clearTimeout(controlsTimer.value);
    }

    if (!isPaused.value) {
        controlsTimer.value = setTimeout(() => {
            showControlsFlag.value = false;
        }, 2000);
    }
};

// 错误处理
const onError = (): void => {
    hasError.value = true;
    isLoading.value = false;
    errorMessage.value = "视频加载失败，请检查网络或视频源";
};

const retryLoad = (): void => {
    if (!videoEl.value) return;
    hasError.value = false;
    isLoading.value = true;
    videoEl.value.load();
};

// 生命周期事件处理
const onLoaded = (): void => {
    if (!videoEl.value) return;
    durationValue.value = videoEl.value.duration;
    isLoading.value = false;

    if (props.autoplay) {
        togglePlay();
    }
};

const onEnded = async () => {
    emit("ended");
    isPaused.value = true;
    showControlsFlag.value = true; // 视频结束时显示控制栏
    if(document.fullscreenElement) {
        await document.exitFullscreen();
        isFullScreen.value = false;
    }
};

onMounted(() => {
    if (!videoEl.value) return;

    videoEl.value.volume = volume.value;
});

onUnmounted(() => {
    if (controlsTimer.value) {
        clearTimeout(controlsTimer.value);
    }
});

// 监听音量变化
watchEffect(() => {
    if (props.showVideo === false) {
        isPaused.value = true;
        videoEl.value?.pause();
    } else {
        isPaused.value = false;
        videoEl.value?.play();
    }
});
</script>

<style scoped lang="less">
.custom-player {
    position: relative;
    width: 100%;
    max-width: 800px;
    margin: 0 auto;
    background-color: #000;
    overflow: hidden;

    .video-box {
        display: flex;
        align-items: center;
        height: 100%;
        overflow: hidden;
    }
}

video {
    width: 100%;
    height: 100%;
}

/* 使用封面图作为背景：视频区域填充封面图，视频等比缩放显示在其上，避免黑边 */
.custom-player.has-cover-bg {
    &::before {
        content: "";
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background-image: var(--cover-bg-url);
        background-size: cover;
        background-position: center;
        background-repeat: no-repeat;
        -webkit-filter: blur(60px);
        filter: blur(60px);
        transform: scale(1.1);
        z-index: 0;
        pointer-events: none;
    }

    .video-box {
        position: relative;
        z-index: 1;
    }

    /* 保持原有 absolute 定位，只抬高层级，避免控制栏被背景盖住或错位 */
    .video-overlay,
    .controls,
    .loading-indicator,
    .error-message {
        z-index: 2;
    }

    video {
        object-fit: contain;
        background-color: transparent;
    }
}

.video-overlay {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: rgba(0, 0, 0, 0.3);
    cursor: pointer;
}

.play-icon {
    border-radius: 50%;
    width: 68px;
    height: 68px;
    background-image: url("~/assets/images/product/video-playing.png");
    background-size: cover;
}

.loading-indicator {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
}

.spinner {
    width: 40px;
    height: 40px;
    border: 4px solid rgba(255, 255, 255, 0.3);
    border-top: 4px solid var(--general);
    border-radius: 50%;
    animation: spin 1s linear infinite;
}

@keyframes spin {
    0% {
        transform: rotate(0deg);
    }
    100% {
        transform: rotate(360deg);
    }
}

.error-message {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    background-color: rgba(0, 0, 0, 0.7);
    color: #fff;
}

.error-message button {
    margin-top: 10px;
    padding: 8px 16px;
    background-color: #ff4d4d;
    border: none;
    border-radius: 4px;
    color: white;
    cursor: pointer;
}

.controls {
    position: absolute;
    bottom: 0;
    left: 0;
    width: 100%;
    background: linear-gradient(transparent, rgba(0, 0, 0, 0.8));
    color: white;
    transition: transform 0.3s ease-in-out;
}

.controls.hidden {
    pointer-events: none;
    transform: translateY(38px);
}

.progress-bar {
    position: relative;
    height: 0.5em;
    background: rgba(255, 255, 255, 0.2);
    cursor: pointer;
    border-radius: 5px;
}

.buffer-progress,
.play-progress {
    position: absolute;
    height: 100%;
    top: 0;
    left: 0;
    pointer-events: none;
}

.buffer-progress {
    background: rgba(255, 255, 255, 0.4);
    transition: width 0.1s;
}

.play-progress {
    background: var(--general);
    transition: width 0.1s;
}

.hover-time {
    position: absolute;
    background: rgba(0, 0, 0, 0.7);
    color: white;
    padding: 2px 6px;
    min-width: 3em;
    border-radius: 3px;
    font-size: 12px;
    transform: translateX(-50%) translateY(-25px);
    pointer-events: none;
    z-index: 10;
    text-align: center;
}

.buttons {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 10px;
    height: 38px;
}

.control-button {
    background: none;
    border: none;
    color: white;
    cursor: pointer;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
}

.volume-control {
    display: flex;
    align-items: center;
    gap: 8px;
}

.volume-slider {
    width: 70px;
    height: 5px;
    -webkit-appearance: none;
    appearance: none;
    background: rgba(255, 255, 255, 0.3);
    outline: none;
    border-radius: 2.5px;
}

.volume-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: white;
    cursor: pointer;
}

.time {
    font-size: 14px;
    display: flex;
    align-items: center;
    gap: 3px;
}

.time-separator {
    opacity: 0.7;
}

.right-controls {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 15px;
}

.playback-rate select {
    background-color: transparent;
    color: white;
    border: 1px solid rgba(255, 255, 255, 0.3);
    border-radius: 3px;
    padding: 2px 5px;
    font-size: 12px;
    cursor: pointer;
    outline: none;
}

.playback-rate select option {
    background-color: rgba(0, 0, 0, 0.9);
}
</style>
