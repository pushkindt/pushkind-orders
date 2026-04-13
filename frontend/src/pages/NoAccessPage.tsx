import {
  NoAccessCard,
  useNoAccessPageData,
} from "@pushkind/frontend-shell/noAccess";

import { OrdersShell } from "../components/OrdersShell";
import { OrdersShellFatalState } from "../components/OrdersShellFatalState";
import { fetchNoAccessData } from "../lib/api";
import type { NoAccessData } from "../lib/models";
import { useOrdersShell } from "../lib/useOrdersShell";

export function NoAccessPage() {
  const shellState = useOrdersShell("Не удалось загрузить оболочку Orders.");
  const noAccessState = useNoAccessPageData<NoAccessData>({
    errorMessage: "Не удалось загрузить страницу.",
    fetchNoAccessData,
  });

  if (shellState.status === "error") {
    return <OrdersShellFatalState message={shellState.message} />;
  }

  if (shellState.status === "loading" || noAccessState.status === "loading") {
    return null;
  }

  if (noAccessState.status === "error") {
    return <OrdersShellFatalState message={noAccessState.message} />;
  }

  return (
    <OrdersShell
      navigation={shellState.shell.navigation}
      currentUserEmail={shellState.shell.currentUser.email}
      homeUrl={shellState.shell.homeUrl}
      localMenuItems={shellState.shell.localMenuItems}
      fetchedMenuItems={shellState.authMenuItems}
    >
      <NoAccessCard
        className="container py-5 orders-shell-content"
        serviceLabel="Orders"
        currentUserName={noAccessState.data.currentUser.name}
        currentUserEmail={noAccessState.data.currentUser.email}
        homeUrl={noAccessState.data.homeUrl}
        requiredRole={noAccessState.data.requiredRole}
        logoutAction="/logout"
      />
    </OrdersShell>
  );
}
