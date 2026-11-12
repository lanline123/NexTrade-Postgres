export const runLogoutCleanup = async (remoteLogout: () => Promise<unknown>, cleanup: () => void | Promise<void>) => {
    try {
        await remoteLogout();
    } finally {
        await cleanup();
    }
};
