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
  status: string | null;
  updatedAfter: string | null;
  updatedBefore: string | null;
  page: number;
};

const formatterCache = new Map<string, Intl.NumberFormat | null>();

function readIndexQueryFromLocation(): OrdersIndexQuery {
  if (typeof window === "undefined") {
    return {
      search: null,
      status: null,
      updatedAfter: null,
      updatedBefore: null,
      page: 1,
    };
  }

  const params = new URLSearchParams(window.location.search);
  const rawSearch = params.get("search")?.trim() ?? "";
  const rawStatus = params.get("status")?.trim() ?? "";
  const rawUpdatedAfter = params.get("updated_after")?.trim() ?? "";
  const rawUpdatedBefore = params.get("updated_before")?.trim() ?? "";
  const rawPage = Number(params.get("page") ?? "1");
  const page = Number.isInteger(rawPage) && rawPage > 0 ? rawPage : 1;

  return {
    search: rawSearch.length > 0 ? rawSearch : null,
    status: rawStatus.length > 0 ? rawStatus : null,
    updatedAfter: rawUpdatedAfter.length > 0 ? rawUpdatedAfter : null,
    updatedBefore: rawUpdatedBefore.length > 0 ? rawUpdatedBefore : null,
    page,
  };
}

export function buildOrdersIndexPageUrl(
  page: number,
  search: string | null,
  status: string | null,
  updatedAfter: string | null,
  updatedBefore: string | null,
) {
  const params = new URLSearchParams();

  if (search) {
    params.set("search", search);
  }

  if (status) {
    params.set("status", status);
  }

  if (updatedAfter) {
    params.set("updated_after", updatedAfter);
  }

  if (updatedBefore) {
    params.set("updated_before", updatedBefore);
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

function hasActiveFilters(filters: {
  status: string | null;
  updatedAfter: string | null;
  updatedBefore: string | null;
}) {
  return Boolean(
    filters.status || filters.updatedAfter || filters.updatedBefore,
  );
}

function renderOrderRow(item: OrderListItem) {
  return (
    <a
      key={item.id}
      href={`/order/${item.id}`}
      className="row my-1 py-2 border-top selectable text-decoration-none text-reset align-items-center"
    >
      <div className="col-12 col-md-2">
        <strong>{item.id}</strong>
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
        <div
          className="fw-semibold"
          data-order-total-cents={item.totalCents}
          data-order-currency={item.currency}
        >
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
    <div className="alert alert-warning my-2" role="alert">
      Нет заказов для отображения.
    </div>
  );
}

function OrdersPagination({
  page,
  totalPages,
  search,
  status,
  updatedAfter,
  updatedBefore,
}: {
  page: number;
  totalPages: number;
  search: string | null;
  status: string | null;
  updatedAfter: string | null;
  updatedBefore: string | null;
}) {
  if (totalPages <= 1) {
    return null;
  }

  return (
    <nav aria-label="pagination">
      <ul
        className="pagination justify-content-center flex-wrap"
        id="pagination"
      >
        {Array.from({ length: totalPages }, (_, index) => index + 1).map(
          (candidatePage) =>
            candidatePage !== page ? (
              <li key={candidatePage} className="page-item">
                <a
                  className="page-link"
                  href={buildOrdersIndexPageUrl(
                    candidatePage,
                    search,
                    status,
                    updatedAfter,
                    updatedBefore,
                  )}
                >
                  {candidatePage}
                </a>
              </li>
            ) : (
              <li
                key={candidatePage}
                className="page-item active"
                aria-current="page"
              >
                <span className="page-link">{candidatePage}</span>
              </li>
            ),
        )}
      </ul>
    </nav>
  );
}

export function OrdersIndexBootstrapPage() {
  const shellState = useOrdersShell("Не удалось загрузить оболочку Orders.");
  const query = readIndexQueryFromLocation();
  const [ordersState, setOrdersState] = useState<OrdersCollectionState>({
    status: "loading",
  });
  const activeFilters =
    ordersState.status === "ready"
      ? hasActiveFilters({
          status: ordersState.data.activeFilters.status,
          updatedAfter: ordersState.data.activeFilters.updatedAfter,
          updatedBefore: ordersState.data.activeFilters.updatedBefore,
        })
      : hasActiveFilters(query);

  useEffect(() => {
    let active = true;

    setOrdersState({ status: "loading" });

    void fetchOrdersCollection({
      search: query.search,
      status: query.status,
      updatedAfter: query.updatedAfter,
      updatedBefore: query.updatedBefore,
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
  }, [
    query.page,
    query.search,
    query.status,
    query.updatedAfter,
    query.updatedBefore,
  ]);

  if (shellState.status === "error") {
    return <OrdersShellFatalState message={shellState.message} />;
  }

  if (shellState.status === "loading") {
    return null;
  }

  const searchForm = (
    <form className="d-flex w-100" role="search" action="/">
      <div className="input-group me-2">
        {query.status ? (
          <input type="hidden" name="status" value={query.status} />
        ) : null}
        {query.updatedAfter ? (
          <input
            type="hidden"
            name="updated_after"
            value={query.updatedAfter}
          />
        ) : null}
        {query.updatedBefore ? (
          <input
            type="hidden"
            name="updated_before"
            value={query.updatedBefore}
          />
        ) : null}
        <input
          required
          name="search"
          className="form-control"
          type="search"
          placeholder="Поиск"
          aria-label="Search"
          defaultValue={query.search ?? ""}
        />
        <button className="btn btn-outline-secondary" type="submit">
          <i className="bi bi-search" />
        </button>
      </div>
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
      <main>
        <div className="container bg-white border rounded my-2">
          <div className="row justify-content-end">
            <div className="col-auto align-self-end">
              <button
                className="btn btn-sm btn-outline-secondary d-flex align-items-center gap-2 mt-1"
                type="button"
                data-bs-toggle="modal"
                data-bs-target="#filtersModal"
              >
                <i className="bi bi-funnel" />
                <span
                  id="activeFiltersBadge"
                  className={`badge text-bg-primary ${activeFilters ? "" : "d-none"}`}
                >
                  •
                </span>
              </button>
            </div>
          </div>

          {ordersState.status === "loading" ? (
            <div className="alert alert-info my-2" role="status">
              Загрузка списка заказов...
            </div>
          ) : null}

          {ordersState.status === "error" ? (
            <div className="alert alert-danger my-2" role="alert">
              {ordersState.message}
            </div>
          ) : null}

          {ordersState.status === "ready" ? (
            <>
              <div id="orderList">
                {ordersState.data.items.length > 0 ? (
                  ordersState.data.items.map(renderOrderRow)
                ) : (
                  <OrdersIndexEmptyState />
                )}
              </div>
              <OrdersPagination
                page={ordersState.data.pagination.page}
                totalPages={ordersState.data.pagination.totalPages}
                search={ordersState.data.activeFilters.search}
                status={ordersState.data.activeFilters.status}
                updatedAfter={ordersState.data.activeFilters.updatedAfter}
                updatedBefore={ordersState.data.activeFilters.updatedBefore}
              />
            </>
          ) : null}
        </div>

        <div
          className="modal fade"
          id="filtersModal"
          tabIndex={-1}
          aria-labelledby="filtersModalLabel"
          aria-hidden="true"
        >
          <div className="modal-dialog modal-lg modal-dialog-centered">
            <div className="modal-content">
              <div className="modal-header">
                <h1 className="modal-title fs-5" id="filtersModalLabel">
                  Фильтры заказов
                </h1>
                <button
                  type="button"
                  className="btn-close"
                  data-bs-dismiss="modal"
                  aria-label="Закрыть"
                />
              </div>
              <form className="modal-body row g-3" method="get" action="/">
                {query.search ? (
                  <input type="hidden" name="search" value={query.search} />
                ) : null}
                <div className="col-12">
                  <label
                    htmlFor="filterStatus"
                    className="form-label small text-uppercase text-muted mb-1"
                  >
                    Статус
                  </label>
                  <select
                    id="filterStatus"
                    name="status"
                    className="form-select"
                    defaultValue={query.status ?? ""}
                  >
                    <option value="">Все</option>
                    <option value="Pending">В ожидании</option>
                    <option value="Processing">В работе</option>
                    <option value="Completed">Завершена</option>
                    <option value="Cancelled">Отменена</option>
                    <option value="Draft">Черновик</option>
                  </select>
                </div>
                <div className="col-12 col-md-6">
                  <label
                    htmlFor="filterUpdatedAfter"
                    className="form-label small text-uppercase text-muted mb-1"
                  >
                    Обновлена после
                  </label>
                  <input
                    id="filterUpdatedAfter"
                    name="updated_after"
                    type="date"
                    className="form-control"
                    defaultValue={query.updatedAfter ?? ""}
                  />
                </div>
                <div className="col-12 col-md-6">
                  <label
                    htmlFor="filterUpdatedBefore"
                    className="form-label small text-uppercase text-muted mb-1"
                  >
                    Обновлена до
                  </label>
                  <input
                    id="filterUpdatedBefore"
                    name="updated_before"
                    type="date"
                    className="form-control"
                    defaultValue={query.updatedBefore ?? ""}
                  />
                </div>
                <div className="col-12 d-flex flex-wrap gap-2 justify-content-end pt-3">
                  <button type="submit" className="btn btn-primary">
                    Применить
                  </button>
                  <a href="/" className="btn btn-outline-secondary">
                    Сбросить
                  </a>
                </div>
              </form>
            </div>
          </div>
        </div>
      </main>
    </OrdersShell>
  );
}
