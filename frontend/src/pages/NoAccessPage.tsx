import { ServiceNoAccessPage } from "@pushkind/frontend-shell/noAccess";

import { OrdersShell } from "../components/OrdersShell";
import { OrdersShellFatalState } from "../components/OrdersShellFatalState";
import {
  fetchHubMenuItems,
  fetchNoAccessData,
  fetchShellData,
} from "../lib/api";
import type { NoAccessData, ShellData, UserMenuItem } from "../lib/models";

export function NoAccessPage() {
  return (
    <ServiceNoAccessPage<NoAccessData, ShellData, UserMenuItem>
      serviceLabel="Orders"
      fetchShellData={fetchShellData}
      fetchHubMenuItems={fetchHubMenuItems}
      fetchNoAccessData={fetchNoAccessData}
      ShellComponent={OrdersShell}
      FatalStateComponent={OrdersShellFatalState}
      menuLoadWarning="Failed to load auth navigation menu. Falling back to local Orders menu only."
    />
  );
}
