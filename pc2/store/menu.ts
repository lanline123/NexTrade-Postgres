export const useMenusStore = defineStore("menus", () => {
    const mainMenu = ref(null);

    function setMenus(data: any) {
        mainMenu.value = data;
    }

    return {
        mainMenu,
        setMenus
    };
});
