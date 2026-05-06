import { reactive } from "vue";

export interface ConfirmDialogOptions {
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  danger?: boolean;
}

export function useConfirmDialog() {
  let resolver: ((confirmed: boolean) => void) | null = null;
  const dialog = reactive({
    open: false,
    title: "",
    message: "",
    confirmText: "确认",
    cancelText: "取消",
    danger: false,
  });

  function confirm(options: ConfirmDialogOptions) {
    if (resolver) {
      resolver(false);
    }

    Object.assign(dialog, {
      open: true,
      title: options.title,
      message: options.message,
      confirmText: options.confirmText ?? "确认",
      cancelText: options.cancelText ?? "取消",
      danger: options.danger ?? false,
    });

    return new Promise<boolean>((resolve) => {
      resolver = resolve;
    });
  }

  function close(confirmed: boolean) {
    dialog.open = false;
    resolver?.(confirmed);
    resolver = null;
  }

  return {
    dialog,
    confirm,
    cancelConfirm: () => close(false),
    acceptConfirm: () => close(true),
  };
}
