import { useEffect, useRef, useState } from "react";

import { OrderEditModal } from "../components/OrderEditModal";
import { OrderStatusBadge } from "../components/OrderStatusBadge";
import { OrdersShell } from "../components/OrdersShell";
import { OrdersShellFatalState } from "../components/OrdersShellFatalState";
import {
  fetchHubMenuItems,
  fetchOrderDetails,
  fetchShellData,
  isApiMutationError,
  updateOrderProductApprovals,
} from "../lib/api";
import { disposeBootstrapModal, showBootstrapModal } from "../lib/bootstrap";
import type {
  OrderApprovalUpdateInput,
  OrderDetailsData,
  OrderMutationSuccess,
  OrderProductItem,
  ShellData,
  UserMenuItem,
} from "../lib/models";
import { useServiceShell } from "@pushkind/frontend-shell/useServiceShell";

type OrderDetailsState =
  | { status: "loading" }
  | { status: "ready"; data: OrderDetailsData }
  | { status: "error"; message: string };

type ApprovalStatus = {
  variant: "danger" | "info" | "success" | "warning";
  message: string;
} | null;

const formatterCache = new Map<string, Intl.NumberFormat | null>();

export function readOrderIdFromPathname(pathname: string): number | null {
  const match = pathname.match(/^\/order\/(\d+)\/?$/);
  if (match == null) {
    return null;
  }

  const orderId = Number(match[1]);
  return Number.isInteger(orderId) && orderId > 0 ? orderId : null;
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

function buildCustomerHref(order: OrderDetailsData): string | null {
  if (order.customer?.publicId == null) {
    return null;
  }

  return `${order.crmServiceUrl}?public_id=${encodeURIComponent(order.customer.publicId)}`;
}

function productUnitPrice(product: OrderProductItem): string {
  const divisor = product.approvedQuantity || product.quantity || 1;
  const unitPriceCents = Math.round(product.priceCents / divisor);
  return formatMoney(unitPriceCents, product.currency);
}

export function OrderDetailsPage() {
  const shellState = useServiceShell<ShellData, UserMenuItem>({
    errorMessage: "Не удалось загрузить оболочку Orders.",
    menuLoadWarning:
      "Failed to load auth navigation menu. Falling back to local Orders menu only.",
    fetchShellData,
    fetchHubMenuItems,
  });
  const orderId =
    typeof window === "undefined"
      ? null
      : readOrderIdFromPathname(window.location.pathname);
  const editModalRef = useRef<HTMLDivElement | null>(null);
  const [orderState, setOrderState] = useState<OrderDetailsState>({
    status: "loading",
  });
  const [approvalDrafts, setApprovalDrafts] = useState<Record<number, string>>(
    {},
  );
  const [approvalStatus, setApprovalStatus] = useState<ApprovalStatus>(null);
  const [savingProductId, setSavingProductId] = useState<number | null>(null);

  useEffect(() => {
    return () => {
      disposeBootstrapModal(editModalRef.current);
    };
  }, []);

  useEffect(() => {
    if (orderId == null) {
      setOrderState({
        status: "error",
        message: "Не удалось определить заказ по адресу страницы.",
      });
      return;
    }

    let active = true;
    setOrderState({ status: "loading" });

    void fetchOrderDetails(orderId)
      .then((data) => {
        if (!active) {
          return;
        }

        setOrderState({ status: "ready", data });
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setOrderState({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить заказ.",
        });
      });

    return () => {
      active = false;
    };
  }, [orderId]);

  useEffect(() => {
    if (orderState.status !== "ready") {
      return;
    }

    setApprovalDrafts(
      Object.fromEntries(
        orderState.data.products
          .filter((product) => product.productId !== null)
          .map((product) => [
            product.productId as number,
            String(product.approvedQuantity),
          ]),
      ),
    );
  }, [orderState]);

  if (shellState.status === "error") {
    return <OrdersShellFatalState message={shellState.message} />;
  }

  if (shellState.status === "loading") {
    return null;
  }

  async function handleApprovalSave(product: OrderProductItem) {
    if (orderId == null || product.productId == null) {
      setApprovalStatus({
        variant: "danger",
        message: "Позиция недоступна для обновления.",
      });
      return;
    }

    const draftValue = approvalDrafts[product.productId] ?? "";
    const approvedQuantity = Number.parseInt(draftValue, 10);

    if (!Number.isInteger(approvedQuantity) || approvedQuantity <= 0) {
      setApprovalStatus({
        variant: "warning",
        message: "Количество должно быть положительным целым.",
      });
      return;
    }

    const payload: OrderApprovalUpdateInput = {
      approvals: [
        {
          productId: product.productId,
          approvedQuantity,
        },
      ],
    };

    setSavingProductId(product.productId);
    setApprovalStatus({ variant: "info", message: "Сохранение..." });

    try {
      const result = await updateOrderProductApprovals(orderId, payload);
      setOrderState({ status: "ready", data: result.order });
      setApprovalStatus({ variant: "success", message: result.message });
    } catch (error) {
      if (isApiMutationError(error)) {
        const fieldMessage = error.field_errors[0]?.message;
        setApprovalStatus({
          variant: "danger",
          message: fieldMessage ?? error.message,
        });
      } else {
        setApprovalStatus({
          variant: "danger",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось обновить заказ.",
        });
      }
    } finally {
      setSavingProductId(null);
    }
  }

  function handleOrderUpdated(result: OrderMutationSuccess) {
    setOrderState({ status: "ready", data: result.order });
    setApprovalStatus(null);
    window.showFlashMessage?.(result.message, "success");
  }

  const searchForm = (
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
      <main className="container my-3">
        {orderState.status === "loading" ? (
          <div className="alert alert-info" role="status">
            Загрузка заказа...
          </div>
        ) : null}

        {orderState.status === "error" ? (
          <div className="alert alert-danger" role="alert">
            {orderState.message}
          </div>
        ) : null}

        {orderState.status === "ready" ? (
          <>
            <div className="d-flex flex-wrap justify-content-between align-items-center gap-2">
              <div>
                <p className="text-muted text-uppercase small mb-1">Заказ</p>
                <h1 className="h3 mb-0">
                  #{orderState.data.id}
                  {orderState.data.reference ? (
                    <span className="text-muted ms-2">
                      {orderState.data.reference}
                    </span>
                  ) : null}
                </h1>
              </div>
              <div className="d-flex flex-wrap gap-2">
                <button
                  className="btn btn-primary d-flex align-items-center gap-1"
                  type="button"
                  onClick={() => showBootstrapModal(editModalRef.current)}
                >
                  <i className="bi bi-pencil" aria-hidden="true" />
                  Редактировать
                </button>
                <a className="btn btn-outline-secondary" href="/">
                  <i className="bi bi-arrow-left me-2" aria-hidden="true" />К
                  списку заказов
                </a>
              </div>
            </div>

            <div className="row g-3 mt-2">
              <div className="col-lg-4">
                <div className="card h-100 shadow-sm">
                  <div className="card-body">
                    <div className="d-flex align-items-center justify-content-between mb-3">
                      <div>
                        <div className="text-muted text-uppercase small">
                          Статус
                        </div>
                        <div className="d-flex align-items-center gap-2">
                          <OrderStatusBadge status={orderState.data.status} />
                        </div>
                      </div>
                      <div className="text-end">
                        <div className="text-muted text-uppercase small">
                          Итого
                        </div>
                        <div className="fs-4 fw-semibold">
                          {formatMoney(
                            orderState.data.totalCents,
                            orderState.data.currency,
                          )}
                        </div>
                      </div>
                    </div>

                    <dl className="row mb-0 small">
                      <dt className="col-5 text-muted">Создан</dt>
                      <dd className="col-7 mb-2">
                        {orderState.data.createdAt}
                      </dd>

                      <dt className="col-5 text-muted">Обновлён</dt>
                      <dd className="col-7 mb-2">
                        {orderState.data.updatedAt}
                      </dd>

                      <dt className="col-5 text-muted">Валюта</dt>
                      <dd className="col-7 mb-2 text-uppercase">
                        {orderState.data.currency}
                      </dd>

                      <dt className="col-5 text-muted">Клиент</dt>
                      <dd className="col-7 mb-2">
                        {orderState.data.customer ? (
                          <>
                            <div className="fw-semibold">
                              {buildCustomerHref(orderState.data) ? (
                                <a
                                  href={
                                    buildCustomerHref(orderState.data) ??
                                    undefined
                                  }
                                >
                                  {orderState.data.customer.name}
                                </a>
                              ) : (
                                orderState.data.customer.name
                              )}
                            </div>
                            <div className="text-muted small">
                              {orderState.data.customer.phone}
                            </div>
                          </>
                        ) : orderState.data.customerId != null ? (
                          <span className="text-muted">Клиент не найден</span>
                        ) : (
                          <span className="text-muted">Не указан</span>
                        )}
                      </dd>

                      <dt className="col-5 text-muted">Заметки</dt>
                      <dd className="col-7 mb-2">
                        {orderState.data.notes ? (
                          orderState.data.notes
                        ) : (
                          <span className="text-muted">Нет заметок</span>
                        )}
                      </dd>

                      <dt className="col-5 text-muted">Адрес доставки</dt>
                      <dd className="col-7 mb-2">
                        {orderState.data.shippingAddress ? (
                          orderState.data.shippingAddress
                        ) : (
                          <span className="text-muted">Не указан</span>
                        )}
                      </dd>

                      <dt className="col-5 text-muted">Получатель</dt>
                      <dd className="col-7 mb-2">
                        {orderState.data.consignee ? (
                          orderState.data.consignee
                        ) : (
                          <span className="text-muted">Не указан</span>
                        )}
                      </dd>

                      <dt className="col-5 text-muted">
                        Информация о доставке
                      </dt>
                      <dd className="col-7 mb-2">
                        {orderState.data.deliveryNotes ? (
                          orderState.data.deliveryNotes
                        ) : (
                          <span className="text-muted">Нет данных</span>
                        )}
                      </dd>

                      <dt className="col-5 text-muted">Плательщик</dt>
                      <dd className="col-7 mb-2">
                        {orderState.data.payer ? (
                          orderState.data.payer
                        ) : (
                          <span className="text-muted">Не указан</span>
                        )}
                      </dd>
                    </dl>
                  </div>
                </div>
              </div>

              <div className="col-lg-8">
                <div className="card shadow-sm h-100">
                  <div className="card-header d-flex justify-content-between align-items-center">
                    <div>
                      <div className="text-muted text-uppercase small">
                        Товары
                      </div>
                      <div className="fw-semibold">
                        {orderState.data.products.length} позиций
                      </div>
                    </div>
                    <div className="text-end">
                      <div className="text-muted text-uppercase small">
                        Сумма заказа
                      </div>
                      <div className="fw-semibold">
                        {formatMoney(
                          orderState.data.totalCents,
                          orderState.data.currency,
                        )}
                      </div>
                    </div>
                  </div>

                  <div className="px-3 pt-3">
                    {approvalStatus ? (
                      <div
                        className={`alert alert-${approvalStatus.variant} mb-0`}
                        role="alert"
                      >
                        {approvalStatus.message}
                      </div>
                    ) : null}
                  </div>

                  <div className="table-responsive">
                    <table className="table align-middle mb-0">
                      <thead className="table-light">
                        <tr>
                          <th scope="col">Название</th>
                          <th scope="col" className="text-nowrap">
                            Цена
                          </th>
                          <th scope="col" className="text-end">
                            Заказано
                          </th>
                          <th scope="col" className="text-end">
                            Одобрено
                          </th>
                          <th scope="col" className="text-end">
                            Сумма
                          </th>
                        </tr>
                      </thead>
                      <tbody>
                        {orderState.data.products.length > 0 ? (
                          orderState.data.products.map((product) => {
                            const isEditable = product.productId !== null;
                            const isSaving =
                              isEditable &&
                              savingProductId !== null &&
                              savingProductId === product.productId;

                            return (
                              <tr
                                key={`${product.productId ?? "snapshot"}-${product.name}-${product.sku ?? ""}`}
                              >
                                <td>
                                  <div className="fw-semibold">
                                    {product.name}
                                  </div>
                                  {product.sku ? (
                                    <span className="text-muted">
                                      {product.sku}
                                    </span>
                                  ) : null}
                                </td>
                                <td className="text-muted">
                                  <div>
                                    Клиент:{" "}
                                    <span>{productUnitPrice(product)}</span>
                                  </div>
                                  <div>
                                    Базовая:{" "}
                                    <span>
                                      {product.defaultPriceCents != null
                                        ? formatMoney(
                                            product.defaultPriceCents,
                                            product.currency,
                                          )
                                        : "-"}
                                    </span>
                                  </div>
                                </td>
                                <td className="text-end">
                                  <div className="fw-semibold">
                                    {product.quantity}
                                  </div>
                                  <div className="text-muted small">
                                    Заказано
                                  </div>
                                </td>
                                <td className="text-end">
                                  <div className="input-group input-group-sm justify-content-end">
                                    <input
                                      type="number"
                                      className="form-control form-control-sm text-end"
                                      min={1}
                                      value={
                                        product.productId != null
                                          ? (approvalDrafts[
                                              product.productId
                                            ] ??
                                            String(product.approvedQuantity))
                                          : String(product.approvedQuantity)
                                      }
                                      onChange={(event) => {
                                        if (product.productId == null) {
                                          return;
                                        }

                                        const value = event.currentTarget.value;
                                        setApprovalDrafts((current) => ({
                                          ...current,
                                          [product.productId as number]: value,
                                        }));
                                        setApprovalStatus(null);
                                      }}
                                      disabled={!isEditable || isSaving}
                                      title={
                                        !isEditable
                                          ? "Позиция недоступна для обновления"
                                          : undefined
                                      }
                                    />
                                    <button
                                      className="btn btn-outline-primary"
                                      type="button"
                                      onClick={() =>
                                        void handleApprovalSave(product)
                                      }
                                      disabled={!isEditable || isSaving}
                                    >
                                      {isSaving ? "Сохранение..." : "Сохранить"}
                                    </button>
                                  </div>
                                  <div className="text-muted small">
                                    Одобрено
                                  </div>
                                </td>
                                <td className="text-end fw-semibold">
                                  {formatMoney(
                                    product.priceCents,
                                    product.currency,
                                  )}
                                </td>
                              </tr>
                            );
                          })
                        ) : (
                          <tr>
                            <td
                              colSpan={5}
                              className="text-center text-muted py-4"
                            >
                              В заказе нет товаров.
                            </td>
                          </tr>
                        )}
                      </tbody>
                    </table>
                  </div>
                </div>
              </div>
            </div>

            <OrderEditModal
              modalRef={editModalRef}
              order={orderState.data}
              onUpdated={handleOrderUpdated}
            />
          </>
        ) : null}
      </main>
    </OrdersShell>
  );
}
