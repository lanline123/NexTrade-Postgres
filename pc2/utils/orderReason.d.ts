export declare const buildCancelOrderPayload: (id: number, reason?: string) => { id: number; description?: string };
export declare const keepValidSelection: (selectedReason: string, reasons: string[]) => string;
export declare const shouldShowCancelRemark: (cancelRemark?: string) => boolean;
