import type { ReactNode } from "react";

import { UserMenuDropdown } from "./UserMenuDropdown";
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
    <div className="container">
      <nav className="navbar navbar-expand-sm bg-body-tertiary">
        <div className="container-fluid">
          <a className="navbar-brand" href="/">
            Orders
          </a>
          <button
            className="navbar-toggler"
            type="button"
            data-bs-toggle="collapse"
            data-bs-target="#orders-foundation-navbar"
            aria-controls="orders-foundation-navbar"
            aria-expanded="false"
            aria-label="Toggle navigation"
          >
            <span className="navbar-toggler-icon" />
          </button>
          <div
            className="collapse navbar-collapse"
            id="orders-foundation-navbar"
          >
            <ul className="navbar-nav me-auto">
              {navigation.map((item) => (
                <li className="nav-item" key={item.url}>
                  <a className="nav-link" href={item.url}>
                    {item.name}
                  </a>
                </li>
              ))}
            </ul>
            {search ?? null}
          </div>
          <UserMenuDropdown
            currentUserEmail={currentUserEmail}
            localItems={[{ name: "Домой", url: homeUrl }, ...localMenuItems]}
            fetchedItems={fetchedMenuItems}
            logoutAction="/logout"
          />
        </div>
      </nav>
    </div>
  );
}
