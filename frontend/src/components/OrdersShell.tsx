import { ModalFlashShell } from "@pushkind/frontend-shell/ModalFlashShell";
import type { ReactNode } from "react";

import { OrdersNavbar } from "./OrdersNavbar";
import type { NavigationItem, UserMenuItem } from "../lib/models";

type OrdersShellProps = {
  navigation: NavigationItem[];
  currentUserEmail: string;
  homeUrl: string;
  localMenuItems: UserMenuItem[];
  fetchedMenuItems: UserMenuItem[];
  search?: ReactNode;
  children: ReactNode;
};

export function OrdersShell({
  navigation,
  currentUserEmail,
  homeUrl,
  localMenuItems,
  fetchedMenuItems,
  search,
  children,
}: OrdersShellProps) {
  return (
    <ModalFlashShell
      navbar={
        <OrdersNavbar
          navigation={navigation}
          currentUserEmail={currentUserEmail}
          homeUrl={homeUrl}
          localMenuItems={localMenuItems}
          fetchedMenuItems={fetchedMenuItems}
          search={search}
        />
      }
      enablePopovers
    >
      {children}
    </ModalFlashShell>
  );
}
