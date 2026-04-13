import { useServiceShell } from "@pushkind/frontend-shell/useServiceShell";

import { fetchHubMenuItems, fetchShellData } from "./api";
import type { ShellData, UserMenuItem } from "./models";

export function useOrdersShell(errorMessage: string) {
  return useServiceShell<ShellData, UserMenuItem>({
    errorMessage,
    menuLoadWarning:
      "Failed to load auth navigation menu. Falling back to local Orders menu only.",
    fetchShellData,
    fetchHubMenuItems,
  });
}
