export type BootstrapModalInstance = {
  hide: () => void;
  show: () => void;
  dispose?: () => void;
};

declare global {
  interface Window {
    bootstrap?: {
      Modal: {
        getOrCreateInstance: (
          element: string | Element,
          options?: object,
        ) => BootstrapModalInstance;
      };
      Popover?: new (element: Element) => { dispose?: () => void };
    };
  }
}

export function getBootstrapModalInstance(
  element: Element | null,
): BootstrapModalInstance | null {
  if (element == null) {
    return null;
  }

  return window.bootstrap?.Modal.getOrCreateInstance(element) ?? null;
}

export function showBootstrapModal(element: Element | null): void {
  getBootstrapModalInstance(element)?.show();
}

export function hideBootstrapModal(element: Element | null): void {
  getBootstrapModalInstance(element)?.hide();
}

export function disposeBootstrapModal(element: Element | null): void {
  getBootstrapModalInstance(element)?.dispose?.();
}
