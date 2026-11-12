export const useJoinStore = defineStore("join", () => {
    const module1Loaded = ref(false);
    const module2Loaded = ref(false);
    const module3Loaded = ref(false);
    const module4Loaded = ref(false);
    const module1 = ref({});
    const module2 = ref([]);
    const module3 = ref([]);
    const module4 = ref([]);

    return {
        module1Loaded,
        module2Loaded,
        module3Loaded,
        module4Loaded,
        module1,
        module2,
        module3,
        module4
    };
});
