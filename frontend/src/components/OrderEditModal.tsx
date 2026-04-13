import { useEffect, useState } from "react";
import type { FormEvent, RefObject } from "react";

import { isApiMutationError, toFieldErrorMap, updateOrder } from "../lib/api";
import { hideBootstrapModal } from "../lib/bootstrap";
import type {
  OrderDetailsData,
  OrderMutationSuccess,
  OrderUpdateInput,
} from "../lib/models";

type OrderEditModalProps = {
  modalRef: RefObject<HTMLDivElement | null>;
  order: OrderDetailsData;
  onUpdated: (result: OrderMutationSuccess) => void;
};

const orderEditFieldNames = {
  status: "status",
  reference: "reference",
  notes: "notes",
  shippingAddress: "shipping_address",
  consignee: "consignee",
  deliveryNotes: "delivery_notes",
  payer: "payer",
} as const;

function buildOrderUpdateInput(order: OrderDetailsData): OrderUpdateInput {
  return {
    orderId: order.id,
    status: order.status,
    reference: order.reference,
    notes: order.notes,
    shippingAddress: order.shippingAddress,
    consignee: order.consignee,
    deliveryNotes: order.deliveryNotes,
    payer: order.payer,
  };
}

function normalizeOptionalText(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export function OrderEditModal({
  modalRef,
  order,
  onUpdated,
}: OrderEditModalProps) {
  const [form, setForm] = useState<OrderUpdateInput>(() =>
    buildOrderUpdateInput(order),
  );
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    setForm(buildOrderUpdateInput(order));
    setFieldErrors({});
    setFormError(null);
  }, [order]);

  useEffect(() => {
    const modalElement = modalRef.current;
    if (modalElement == null) {
      return undefined;
    }

    const handleHidden = () => {
      setForm(buildOrderUpdateInput(order));
      setFieldErrors({});
      setFormError(null);
      setIsSubmitting(false);
    };

    modalElement.addEventListener("hidden.bs.modal", handleHidden);

    return () => {
      modalElement.removeEventListener("hidden.bs.modal", handleHidden);
    };
  }, [modalRef, order]);

  function updateFormField<K extends keyof OrderUpdateInput>(
    field: K,
    value: OrderUpdateInput[K],
  ) {
    setForm((current) => ({
      ...current,
      [field]: value,
    }));

    const apiFieldName =
      field in orderEditFieldNames
        ? orderEditFieldNames[field as keyof typeof orderEditFieldNames]
        : null;
    if (apiFieldName) {
      setFieldErrors((current) => ({
        ...current,
        [apiFieldName]: "",
      }));
    }
    setFormError(null);
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsSubmitting(true);
    setFieldErrors({});
    setFormError(null);

    try {
      const result = await updateOrder(order.id, form);
      onUpdated(result);
      hideBootstrapModal(modalRef.current);
    } catch (error) {
      if (isApiMutationError(error)) {
        setFieldErrors(toFieldErrorMap(error));
        setFormError(error.message);
      } else {
        setFormError(
          error instanceof Error ? error.message : "Не удалось обновить заказ.",
        );
      }
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <div
      className="modal fade"
      id="editOrderModal"
      ref={modalRef}
      tabIndex={-1}
      aria-labelledby="editOrderModalLabel"
      aria-hidden="true"
    >
      <div className="modal-dialog modal-lg modal-dialog-centered modal-dialog-scrollable">
        <div className="modal-content">
          <form onSubmit={handleSubmit}>
            <div className="modal-header">
              <h1 className="modal-title fs-5" id="editOrderModalLabel">
                Редактировать заказ
              </h1>
              <button
                type="button"
                className="btn-close"
                data-bs-dismiss="modal"
                aria-label="Закрыть"
              />
            </div>
            <div className="modal-body">
              {formError ? (
                <div className="alert alert-danger" role="alert">
                  {formError}
                </div>
              ) : null}

              <div className="row g-3">
                <div className="col-md-6">
                  <label htmlFor="editOrderStatus" className="form-label">
                    Статус
                  </label>
                  <select
                    id="editOrderStatus"
                    className={`form-select${fieldErrors.status ? " is-invalid" : ""}`}
                    value={form.status}
                    onChange={(event) =>
                      updateFormField("status", event.currentTarget.value)
                    }
                    disabled={isSubmitting}
                  >
                    <option value="Draft">Черновик</option>
                    <option value="Pending">Ожидает</option>
                    <option value="Processing">В обработке</option>
                    <option value="Completed">Завершён</option>
                    <option value="Cancelled">Отменён</option>
                  </select>
                  {fieldErrors.status ? (
                    <div className="invalid-feedback">{fieldErrors.status}</div>
                  ) : null}
                </div>

                <div className="col-md-6">
                  <label htmlFor="editOrderReference" className="form-label">
                    Внешний номер
                  </label>
                  <input
                    id="editOrderReference"
                    type="text"
                    className={`form-control${fieldErrors.reference ? " is-invalid" : ""}`}
                    value={form.reference ?? ""}
                    onChange={(event) =>
                      updateFormField(
                        "reference",
                        normalizeOptionalText(event.currentTarget.value),
                      )
                    }
                    placeholder="Номер из 1С, договора и т.д."
                    disabled={isSubmitting}
                  />
                  {fieldErrors.reference ? (
                    <div className="invalid-feedback">
                      {fieldErrors.reference}
                    </div>
                  ) : null}
                </div>
              </div>

              <div className="mt-3">
                <label htmlFor="editOrderNotes" className="form-label">
                  Заметки
                </label>
                <textarea
                  id="editOrderNotes"
                  className={`form-control${fieldErrors.notes ? " is-invalid" : ""}`}
                  rows={3}
                  value={form.notes ?? ""}
                  onChange={(event) =>
                    updateFormField(
                      "notes",
                      normalizeOptionalText(event.currentTarget.value),
                    )
                  }
                  placeholder="Обновите внутренние заметки по заказу"
                  disabled={isSubmitting}
                />
                {fieldErrors.notes ? (
                  <div className="invalid-feedback">{fieldErrors.notes}</div>
                ) : null}
              </div>

              <div className="row g-3 mt-1">
                <div className="col-md-6">
                  <label
                    htmlFor="editOrderShippingAddress"
                    className="form-label"
                  >
                    Адрес доставки
                  </label>
                  <textarea
                    id="editOrderShippingAddress"
                    className={`form-control${fieldErrors.shipping_address ? " is-invalid" : ""}`}
                    rows={2}
                    value={form.shippingAddress ?? ""}
                    onChange={(event) =>
                      updateFormField(
                        "shippingAddress",
                        normalizeOptionalText(event.currentTarget.value),
                      )
                    }
                    placeholder="Город, улица, дом/офис"
                    disabled={isSubmitting}
                  />
                  {fieldErrors.shipping_address ? (
                    <div className="invalid-feedback">
                      {fieldErrors.shipping_address}
                    </div>
                  ) : null}
                </div>

                <div className="col-md-6">
                  <label htmlFor="editOrderConsignee" className="form-label">
                    Получатель
                  </label>
                  <input
                    id="editOrderConsignee"
                    type="text"
                    className={`form-control${fieldErrors.consignee ? " is-invalid" : ""}`}
                    value={form.consignee ?? ""}
                    onChange={(event) =>
                      updateFormField(
                        "consignee",
                        normalizeOptionalText(event.currentTarget.value),
                      )
                    }
                    placeholder="Имя получателя"
                    disabled={isSubmitting}
                  />
                  {fieldErrors.consignee ? (
                    <div className="invalid-feedback">
                      {fieldErrors.consignee}
                    </div>
                  ) : null}
                </div>

                <div className="col-md-6">
                  <label
                    htmlFor="editOrderDeliveryNotes"
                    className="form-label"
                  >
                    Инструкции по доставке
                  </label>
                  <textarea
                    id="editOrderDeliveryNotes"
                    className={`form-control${fieldErrors.delivery_notes ? " is-invalid" : ""}`}
                    rows={2}
                    value={form.deliveryNotes ?? ""}
                    onChange={(event) =>
                      updateFormField(
                        "deliveryNotes",
                        normalizeOptionalText(event.currentTarget.value),
                      )
                    }
                    placeholder="Уточнение времени или точки вручения"
                    disabled={isSubmitting}
                  />
                  {fieldErrors.delivery_notes ? (
                    <div className="invalid-feedback">
                      {fieldErrors.delivery_notes}
                    </div>
                  ) : null}
                </div>

                <div className="col-md-6">
                  <label htmlFor="editOrderPayer" className="form-label">
                    Плательщик
                  </label>
                  <input
                    id="editOrderPayer"
                    type="text"
                    className={`form-control${fieldErrors.payer ? " is-invalid" : ""}`}
                    value={form.payer ?? ""}
                    onChange={(event) =>
                      updateFormField(
                        "payer",
                        normalizeOptionalText(event.currentTarget.value),
                      )
                    }
                    placeholder="Название компании или физлица"
                    disabled={isSubmitting}
                  />
                  {fieldErrors.payer ? (
                    <div className="invalid-feedback">{fieldErrors.payer}</div>
                  ) : null}
                </div>
              </div>
            </div>
            <div className="modal-footer">
              <button
                type="button"
                className="btn btn-outline-secondary"
                data-bs-dismiss="modal"
                disabled={isSubmitting}
              >
                Отмена
              </button>
              <button
                type="submit"
                className="btn btn-primary"
                disabled={isSubmitting}
              >
                {isSubmitting ? "Сохранение..." : "Сохранить"}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}
