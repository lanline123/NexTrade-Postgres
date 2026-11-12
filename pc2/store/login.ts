export const useLoginStore = defineStore("login", () => {
    const moduleLoaded = ref(false);
    const backgroundPicture = ref("");

    return {
        moduleLoaded,
        backgroundPicture
    };
});
