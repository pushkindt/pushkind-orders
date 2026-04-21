import { ServiceNavbar } from "@pushkind/frontend-shell/ServiceNavbar";
import type { ReactNode } from "react";

import type { NavigationItem, UserMenuItem } from "../lib/models";

type OrdersNavbarProps = {
  navigation: NavigationItem[];
  currentUserEmail: string;
  homeUrl: string;
  localMenuItems: UserMenuItem[];
  fetchedMenuItems: UserMenuItem[];
  search?: ReactNode;
};

export function OrdersNavbar({
  navigation,
  currentUserEmail,
  homeUrl,
  localMenuItems,
  fetchedMenuItems,
  search,
}: OrdersNavbarProps) {
  return (
    <ServiceNavbar
      brand="Orders"
      collapseId="orders-foundation-navbar"
      navigation={navigation}
      currentUserEmail={currentUserEmail}
      homeUrl={homeUrl}
      localMenuItems={localMenuItems}
      fetchedMenuItems={fetchedMenuItems}
      logoutAction="/logout"
      userMenuWrapperClassName="dropdown-center"
      search={search}
      fallbackSearch={
        <form className="d-flex w-100" role="search" action="/">
          <div className="input-group me-2">
            <input
              required
              name="search"
              className="form-control"
              type="search"
              placeholder="Поиск"
              aria-label="Search"
            />
            <button className="btn btn-outline-secondary" type="submit">
              <i className="bi bi-search" />
            </button>
          </div>
        </form>
      }
    />
  );
}
