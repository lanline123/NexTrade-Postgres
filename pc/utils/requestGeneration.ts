export const createRequestGenerationGuard = () => {
    let generation = 0;
    const begin = () => ++generation;
    const invalidate = () => {
        generation++;
    };
    const snapshot = () => generation;
    const isCurrent = (requestGeneration: number) => requestGeneration === generation;
    return { begin, invalidate, snapshot, isCurrent };
};
