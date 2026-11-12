export const buildCancelOrderPayload = (id, reason) => {
    const description = String(reason ?? "").trim();
    return description ? { id, description } : { id };
};

export const keepValidSelection = (selectedReason, reasons) => (reasons.includes(selectedReason) ? selectedReason : "");

export const shouldShowCancelRemark = (cancelRemark) => Boolean(String(cancelRemark ?? "").trim());
