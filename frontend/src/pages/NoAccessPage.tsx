import { useEffect, useState } from "react";

import { OrdersShell } from "../components/OrdersShell";
import { OrdersShellFatalState } from "../components/OrdersShellFatalState";
import { fetchNoAccessData } from "../lib/api";
import type { NoAccessData } from "../lib/models";
import { useOrdersShell } from "../lib/useOrdersShell";

type NoAccessState =
  | { status: "loading" }
  | { status: "ready"; data: NoAccessData }
  | { status: "error"; message: string };

export function NoAccessPage() {
  const shellState = useOrdersShell("Не удалось загрузить оболочку Orders.");
  const [noAccessState, setNoAccessState] = useState<NoAccessState>({
    status: "loading",
  });

  useEffect(() => {
    let active = true;

    void fetchNoAccessData()
      .then((data) => {
        if (!active) {
          return;
        }

        setNoAccessState({ status: "ready", data });
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setNoAccessState({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить страницу.",
        });
      });

    return () => {
      active = false;
    };
  }, []);

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
      <main className="container py-5 orders-shell-content">
        <div className="card shadow-sm">
          <div className="card-body p-4">
            <p className="text-uppercase text-secondary small mb-2">Orders</p>
            <h1 className="h3 mb-3">Недостаточно прав для доступа к сервису</h1>
            <p className="text-secondary mb-3">
              Пользователь{" "}
              <strong>{noAccessState.data.currentUser.name}</strong> не имеет
              роли <code>{noAccessState.data.requiredRole}</code>.
            </p>
            <p className="text-secondary mb-4">
              Текущий email:{" "}
              <strong>{noAccessState.data.currentUser.email}</strong>
            </p>
            <div className="d-flex flex-column flex-sm-row gap-2">
              <a className="btn btn-primary" href={noAccessState.data.homeUrl}>
                Домой
              </a>
              <form method="POST" action="/logout">
                <button className="btn btn-outline-secondary" type="submit">
                  Выйти
                </button>
              </form>
            </div>
          </div>
        </div>
      </main>
    </OrdersShell>
  );
}
