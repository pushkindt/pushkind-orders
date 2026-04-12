import { useEffect, useState } from "react";

import { OrderStatusBadge } from "../components/OrderStatusBadge";
import { OrdersShell } from "../components/OrdersShell";
import { OrdersShellFatalState } from "../components/OrdersShellFatalState";
import { fetchOrdersCollection } from "../lib/api";
import type { OrderCollectionData, OrderListItem } from "../lib/models";
import { useOrdersShell } from "../lib/useOrdersShell";

type OrdersCollectionState =
  | { status: "loading" }
  | { status: "ready"; data: OrderCollectionData }
  | { status: "error"; message: string };

type OrdersIndexQuery = {
  search: string | null;
  page: number;
};

const formatterCache = new Map<string, Intl.NumberFormat | null>();

function readIndexQueryFromLocation(): OrdersIndexQuery {
  if (typeof window === "undefined") {
    return { search: null, page: 1 };
  }

  const params = new URLSearchParams(window.location.search);
  const rawSearch = params.get("search")?.trim() ?? "";
  const rawPage = Number(params.get("page") ?? "1");
  const page = Number.isInteger(rawPage) && rawPage > 0 ? rawPage : 1;

  return {
    search: rawSearch.length > 0 ? rawSearch : null,
    page,
  };
}

export function buildOrdersIndexPageUrl(page: number, search: string | null) {
  const params = new URLSearchParams();

  if (search) {
    params.set("search", search);
  }

  if (page > 1) {
    params.set("page", String(page));
  }

  const queryString = params.toString();
  return queryString ? `/?${queryString}` : "/";
}

function formatMoney(totalCents: number, currency: string) {
  let formatter = formatterCache.get(currency);

  if (formatter === undefined) {
    try {
      formatter = new Intl.NumberFormat("ru-RU", {
        style: "currency",
        currency,
      });
    } catch {
      formatter = null;
    }

    formatterCache.set(currency, formatter);
  }

  const total = totalCents / 100;
  return formatter
    ? formatter.format(total)
    : `${total.toFixed(2)} ${currency}`;
}

function renderOrderRow(item: OrderListItem) {
  return (
    <a
      key={item.id}
      href={`/order/${item.id}`}
      className="row my-1 py-2 border-top text-decoration-none text-reset align-items-center"
    >
      <div className="col-12 col-md-2">
        <strong>{item.id}</strong>
        {item.reference ? (
          <div className="text-muted small">{item.reference}</div>
        ) : null}
        <div className="text-muted small">
          <i className="bi bi-clock-history" aria-hidden="true" />{" "}
          {item.updatedAt}
        </div>
      </div>
      <div className="col-6 col-md-2 mt-2 mt-md-0">
        <div className="text-muted small">Создан</div>
        <div>{item.createdAt}</div>
      </div>
      <div className="col-6 col-md-3 mt-2 mt-md-0">
        <div className="text-muted small">Сумма</div>
        <div className="fw-semibold">
          {formatMoney(item.totalCents, item.currency)}
        </div>
      </div>
      <div className="col-6 col-md-2 mt-2 mt-md-0">
        <div className="text-muted small">Товаров</div>
        <div className="fw-semibold">{item.productsCount}</div>
      </div>
      <div className="col-6 col-md-3 mt-2 mt-md-0">
        <div className="text-muted small">Статус</div>
        <div className="d-flex align-items-center gap-2">
          <OrderStatusBadge status={item.status} />
        </div>
      </div>
    </a>
  );
}

export function OrdersIndexEmptyState() {
  return (
    <div className="card-body p-4">
      <h2 className="h5 mb-2">Заказы не найдены</h2>
      <p className="text-secondary mb-0">
        Попробуйте изменить поиск или открыть первую страницу списка.
      </p>
    </div>
  );
}

export function OrdersIndexBootstrapPage() {
  const shellState = useOrdersShell("Не удалось загрузить оболочку Orders.");
  const query = readIndexQueryFromLocation();
  const [ordersState, setOrdersState] = useState<OrdersCollectionState>({
    status: "loading",
  });

  useEffect(() => {
    let active = true;

    setOrdersState({ status: "loading" });

    void fetchOrdersCollection({
      search: query.search,
      page: query.page,
    })
      .then((data) => {
        if (!active) {
          return;
        }

        setOrdersState({ status: "ready", data });
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setOrdersState({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить список заказов.",
        });
      });

    return () => {
      active = false;
    };
  }, [query.page, query.search]);

  if (shellState.status === "error") {
    return <OrdersShellFatalState message={shellState.message} />;
  }

  if (shellState.status === "loading") {
    return null;
  }

  const searchForm = (
    <form method="get" action="/" className="d-flex gap-2 my-2 my-sm-0">
      <input
        type="search"
        name="search"
        className="form-control"
        defaultValue={query.search ?? ""}
        placeholder="Поиск заказов"
        aria-label="Поиск заказов"
      />
      <button className="btn btn-outline-secondary" type="submit">
        Найти
      </button>
    </form>
  );

  return (
    <OrdersShell
      navigation={shellState.shell.navigation}
      currentUserEmail={shellState.shell.currentUser.email}
      homeUrl={shellState.shell.homeUrl}
      localMenuItems={shellState.shell.localMenuItems}
      fetchedMenuItems={shellState.authMenuItems}
      search={searchForm}
    >
      <main className="container py-3">
        {ordersState.status === "loading" ? (
          <div className="card shadow-sm">
            <div className="card-body p-4">
              <p className="text-uppercase text-secondary small mb-2">Orders</p>
              <h1 className="h4 mb-2">Загружаем заказы</h1>
              <p className="text-secondary mb-0">
                React-страница инициализирует список заказов из
                `/api/v1/orders`.
              </p>
            </div>
          </div>
        ) : null}

        {ordersState.status === "error" ? (
          <div className="card shadow-sm">
            <div className="card-body p-4">
              <p className="text-uppercase text-secondary small mb-2">Orders</p>
              <h1 className="h4 mb-2">Не удалось загрузить список заказов</h1>
              <p className="text-danger mb-0">{ordersState.message}</p>
            </div>
          </div>
        ) : null}

        {ordersState.status === "ready" ? (
          <>
            <div className="d-flex flex-column flex-md-row justify-content-between align-items-md-center gap-3 mb-3">
              <div>
                <p className="text-uppercase text-secondary small mb-1">
                  Orders
                </p>
                <h1 className="h4 mb-1">Заказы</h1>
                <p className="text-secondary mb-0">
                  Всего: {ordersState.data.pagination.totalItems}
                  {ordersState.data.activeFilters.search ? (
                    <>
                      {" "}
                      | Поиск:{" "}
                      <strong>{ordersState.data.activeFilters.search}</strong>
                    </>
                  ) : null}
                </p>
              </div>
            </div>

            <div className="bg-white border rounded my-2">
              {ordersState.data.items.length > 0 ? (
                ordersState.data.items.map(renderOrderRow)
              ) : (
                <OrdersIndexEmptyState />
              )}
            </div>

            {ordersState.data.pagination.totalPages > 1 ? (
              <nav
                aria-label="Навигация по страницам заказов"
                className="d-flex flex-column flex-sm-row justify-content-between align-items-sm-center gap-2 mt-3"
              >
                <span className="text-secondary">
                  Страница {ordersState.data.pagination.page} из{" "}
                  {ordersState.data.pagination.totalPages}
                </span>
                <div className="d-flex gap-2">
                  {ordersState.data.pagination.hasPreviousPage ? (
                    <a
                      className="btn btn-outline-secondary"
                      href={buildOrdersIndexPageUrl(
                        Math.max(1, ordersState.data.pagination.page - 1),
                        ordersState.data.activeFilters.search,
                      )}
                    >
                      Назад
                    </a>
                  ) : (
                    <span
                      className="btn btn-outline-secondary disabled"
                      aria-disabled="true"
                    >
                      Назад
                    </span>
                  )}
                  {ordersState.data.pagination.hasNextPage ? (
                    <a
                      className="btn btn-outline-secondary"
                      href={buildOrdersIndexPageUrl(
                        ordersState.data.pagination.page + 1,
                        ordersState.data.activeFilters.search,
                      )}
                    >
                      Вперёд
                    </a>
                  ) : (
                    <span
                      className="btn btn-outline-secondary disabled"
                      aria-disabled="true"
                    >
                      Вперёд
                    </span>
                  )}
                </div>
              </nav>
            ) : null}
          </>
        ) : null}
      </main>
    </OrdersShell>
  );
}
