import { useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent } from "react";

import { DropdownMultiSelect } from "../components/DropdownMultiSelect";
import { OrdersShell } from "../components/OrdersShell";
import { OrdersShellFatalState } from "../components/OrdersShellFatalState";
import {
  createPriceLevel,
  deletePriceLevel,
  fetchClientPriceLevelAssignments,
  fetchCrmClients,
  fetchPriceLevelDetails,
  fetchPriceLevelsCollection,
  fetchProductsCollection,
  isApiMutationError,
  toFieldErrorMap,
  updateClientPriceLevel,
  updatePriceLevel,
} from "../lib/api";
import { hideBootstrapModal, showBootstrapModal } from "../lib/bootstrap";
import type {
  ClientPriceLevelAssignments,
  CrmClientListItem,
  PriceLevelCollectionData,
  PriceLevelMutationInput,
  ProductListItem,
  ProductNamedOption,
} from "../lib/models";
import { useOrdersShell } from "../lib/useOrdersShell";

type CollectionState =
  | { status: "loading" }
  | { status: "ready"; data: PriceLevelCollectionData }
  | { status: "error"; message: string };

type ClientsState =
  | { status: "idle" }
  | {
      status: "ready";
      clients: CrmClientListItem[];
      assignments: ClientPriceLevelAssignments;
    }
  | { status: "error"; message: string };

type PriceLevelFormState = {
  name: string;
  default: boolean;
  basePriceLevelId: string;
  priceModifier: string;
  priceModifierKind: "percent" | "fixed";
  excludedCategoryIds: string[];
  excludedProducts: ProductNamedOption[];
  includedProducts: ProductNamedOption[];
};

function buildEmptyForm(
  options: PriceLevelCollectionData["editorOptions"],
): PriceLevelFormState {
  return {
    name: "",
    default: false,
    basePriceLevelId: options.basePriceLevels[0]?.id
      ? String(options.basePriceLevels[0].id)
      : "",
    priceModifier: "0",
    priceModifierKind: "percent",
    excludedCategoryIds: [],
    excludedProducts: [],
    includedProducts: [],
  };
}

function ProductSearchPicker({
  label,
  selected,
  onChange,
}: {
  label: string;
  selected: ProductNamedOption[];
  onChange: (items: ProductNamedOption[]) => void;
}) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ProductListItem[]>([]);

  useEffect(() => {
    if (query.trim().length < 2) {
      setResults([]);
      return;
    }

    let active = true;
    void fetchProductsCollection({ search: query.trim(), page: 1 })
      .then((data) => {
        if (!active) {
          return;
        }
        setResults(data.items.slice(0, 10));
      })
      .catch(() => {
        if (active) {
          setResults([]);
        }
      });

    return () => {
      active = false;
    };
  }, [query]);

  const addProduct = (product: ProductListItem) => {
    if (selected.some((item) => item.id === product.id)) {
      return;
    }

    onChange([...selected, { id: product.id, name: product.name }]);
    setQuery("");
    setResults([]);
  };

  return (
    <div>
      <label className="form-label">{label}</label>
      <input
        className="form-control"
        placeholder="Начните вводить название товара"
        value={query}
        onChange={(event) => setQuery(event.currentTarget.value)}
      />
      {results.length > 0 ? (
        <div className="list-group mt-2">
          {results.map((product) => (
            <button
              key={product.id}
              type="button"
              className="list-group-item list-group-item-action"
              onClick={() => addProduct(product)}
            >
              <strong>{product.name}</strong>
              {product.sku ? (
                <span className="text-muted ms-2">{product.sku}</span>
              ) : null}
            </button>
          ))}
        </div>
      ) : null}
      <div className="d-flex flex-wrap gap-2 mt-2">
        {selected.map((item) => (
          <span
            key={item.id}
            className="badge text-bg-light border d-inline-flex gap-2 p-2"
          >
            {item.name}
            <button
              type="button"
              className="btn btn-sm p-0 border-0 bg-transparent"
              onClick={() =>
                onChange(
                  selected.filter(
                    (selectedItem) => selectedItem.id !== item.id,
                  ),
                )
              }
            >
              <i className="bi bi-x-lg" />
            </button>
          </span>
        ))}
      </div>
    </div>
  );
}

export function PriceLevelsPage() {
  const shellState = useOrdersShell("Не удалось загрузить оболочку Orders.");
  const [collectionState, setCollectionState] = useState<CollectionState>({
    status: "loading",
  });
  const [clientsState, setClientsState] = useState<ClientsState>({
    status: "idle",
  });
  const [searchDraft, setSearchDraft] = useState("");
  const [clientFilter, setClientFilter] = useState("");
  const [createForm, setCreateForm] = useState<PriceLevelFormState | null>(
    null,
  );
  const [editId, setEditId] = useState<number | null>(null);
  const [editName, setEditName] = useState("");
  const [editDefault, setEditDefault] = useState(false);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const createModalRef = useRef<HTMLDivElement | null>(null);
  const editModalRef = useRef<HTMLDivElement | null>(null);

  const isAdmin =
    shellState.status === "ready" &&
    shellState.shell.currentUser.roles.includes("orders_admin");

  const loadCollection = async (search?: string | null) => {
    const data = await fetchPriceLevelsCollection({ search });
    setCollectionState({ status: "ready", data });
    setCreateForm(buildEmptyForm(data.editorOptions));
    return data;
  };

  useEffect(() => {
    void loadCollection()
      .then((data) => {
        if (!isAdmin) {
          return;
        }

        return Promise.all([
          fetchClientPriceLevelAssignments(),
          fetchCrmClients(data.crmServiceUrl),
        ]).then(([assignments, clients]) =>
          setClientsState({ status: "ready", assignments, clients }),
        );
      })
      .catch((error) =>
        setCollectionState({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить уровни цен.",
        }),
      );
  }, [isAdmin]);

  const categoryOptions =
    collectionState.status === "ready"
      ? collectionState.data.editorOptions.categories.map((category) => ({
          value: String(category.id),
          label: category.name,
        }))
      : [];

  const filteredClients = useMemo(() => {
    if (clientsState.status !== "ready") {
      return [];
    }

    const normalized = clientFilter.trim().toLowerCase();
    if (normalized.length === 0) {
      return clientsState.clients;
    }

    return clientsState.clients.filter((client) =>
      [client.name, client.phone ?? "", client.publicId ?? ""]
        .join(" ")
        .toLowerCase()
        .includes(normalized),
    );
  }, [clientFilter, clientsState]);

  const openCreate = () => {
    if (collectionState.status !== "ready") {
      return;
    }
    setFieldErrors({});
    setCreateForm(buildEmptyForm(collectionState.data.editorOptions));
    showBootstrapModal(createModalRef.current);
  };

  const openEdit = (priceLevelId: number) => {
    setEditId(priceLevelId);
    setFieldErrors({});
    showBootstrapModal(editModalRef.current);
    void fetchPriceLevelDetails(priceLevelId)
      .then((data) => {
        setEditName(data.name);
        setEditDefault(data.isDefault);
      })
      .catch((error) => {
        window.showFlashMessage?.(
          error instanceof Error
            ? error.message
            : "Не удалось загрузить уровень цен.",
          "danger",
        );
        hideBootstrapModal(editModalRef.current);
      });
  };

  const handleCreate = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (createForm == null) {
      return;
    }

    setIsSubmitting(true);
    setFieldErrors({});

    const input: PriceLevelMutationInput = {
      name: createForm.name,
      default: createForm.default,
      basePriceLevelId: Number.parseInt(createForm.basePriceLevelId, 10),
      priceModifier: Number.parseInt(createForm.priceModifier, 10),
      priceModifierKind: createForm.priceModifierKind,
      excludedCategoryIds: createForm.excludedCategoryIds.map((value) =>
        Number.parseInt(value, 10),
      ),
      excludedProductIds: createForm.excludedProducts.map((item) => item.id),
      includedProductIds: createForm.includedProducts.map((item) => item.id),
    };

    try {
      const response = await createPriceLevel(input);
      const data = await loadCollection(searchDraft.trim() || null);
      if (isAdmin) {
        const [assignments, clients] = await Promise.all([
          fetchClientPriceLevelAssignments(),
          fetchCrmClients(data.crmServiceUrl),
        ]);
        setClientsState({ status: "ready", assignments, clients });
      }
      hideBootstrapModal(createModalRef.current);
      window.showFlashMessage?.(response.message, "primary");
    } catch (error) {
      if (isApiMutationError(error)) {
        setFieldErrors(toFieldErrorMap(error));
      }
      window.showFlashMessage?.(
        error instanceof Error
          ? error.message
          : "Не удалось создать уровень цен.",
        "danger",
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleEdit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (editId == null) {
      return;
    }

    setIsSubmitting(true);
    setFieldErrors({});
    try {
      const response = await updatePriceLevel(editId, {
        name: editName,
        default: editDefault,
      });
      const data = await loadCollection(searchDraft.trim() || null);
      if (isAdmin && clientsState.status === "ready") {
        setClientsState({ ...clientsState });
        void fetchCrmClients(data.crmServiceUrl);
      }
      hideBootstrapModal(editModalRef.current);
      window.showFlashMessage?.(response.message, "primary");
    } catch (error) {
      if (isApiMutationError(error)) {
        setFieldErrors(toFieldErrorMap(error));
      }
      window.showFlashMessage?.(
        error instanceof Error
          ? error.message
          : "Не удалось обновить уровень цен.",
        "danger",
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDelete = async () => {
    if (editId == null) {
      return;
    }

    setIsDeleting(true);
    try {
      const response = await deletePriceLevel(editId);
      await loadCollection(searchDraft.trim() || null);
      hideBootstrapModal(editModalRef.current);
      window.showFlashMessage?.(response.message, "primary");
    } catch (error) {
      window.showFlashMessage?.(
        error instanceof Error
          ? error.message
          : "Не удалось удалить уровень цен.",
        "danger",
      );
    } finally {
      setIsDeleting(false);
    }
  };

  const handleClientAssignment = async (
    client: CrmClientListItem,
    priceLevelId: string,
  ) => {
    if (!client.phone || !client.publicId) {
      return;
    }

    try {
      const response = await updateClientPriceLevel({
        name: client.name,
        phone: client.phone,
        publicId: client.publicId,
        priceLevelId:
          priceLevelId.length > 0 ? Number.parseInt(priceLevelId, 10) : null,
      });
      if (clientsState.status === "ready") {
        setClientsState({
          status: "ready",
          clients: clientsState.clients,
          assignments: {
            ...clientsState.assignments,
            assignments: clientsState.assignments.assignments.map(
              (assignment) =>
                assignment.phone === client.phone
                  ? {
                      ...assignment,
                      priceLevelId:
                        priceLevelId.length > 0
                          ? Number.parseInt(priceLevelId, 10)
                          : null,
                    }
                  : assignment,
            ),
          },
        });
      }
      window.showFlashMessage?.(response.message, "primary");
    } catch (error) {
      window.showFlashMessage?.(
        error instanceof Error
          ? error.message
          : "Не удалось обновить назначение клиента.",
        "danger",
      );
    }
  };

  if (shellState.status === "loading") {
    return (
      <div className="container py-5 text-center text-muted">Загрузка...</div>
    );
  }

  if (shellState.status === "error") {
    return <OrdersShellFatalState message={shellState.message} />;
  }

  return (
    <OrdersShell
      navigation={shellState.shell.navigation}
      currentUserEmail={shellState.shell.currentUser.email}
      homeUrl={shellState.shell.homeUrl}
      localMenuItems={shellState.shell.localMenuItems}
      fetchedMenuItems={shellState.authMenuItems}
      search={
        <form
          className="d-flex"
          onSubmit={(event) => {
            event.preventDefault();
            void loadCollection(searchDraft.trim() || null);
          }}
        >
          <input
            className="form-control"
            placeholder="Поиск уровней цен"
            value={searchDraft}
            onChange={(event) => setSearchDraft(event.currentTarget.value)}
          />
        </form>
      }
    >
      <div className="container bg-white border rounded my-2 py-3">
        <div className="row">
          <div className="col text-center add-item-container">
            {isAdmin ? (
              <button
                type="button"
                className="btn btn-link"
                onClick={openCreate}
              >
                <i className="bi bi-plus-circle" />
              </button>
            ) : null}
          </div>
        </div>

        {collectionState.status === "loading" ? (
          <div className="alert alert-info mt-3">Загрузка уровней цен...</div>
        ) : collectionState.status === "error" ? (
          <div className="alert alert-danger mt-3">
            {collectionState.message}
          </div>
        ) : (
          <div className="d-flex flex-column gap-2 mt-3">
            {collectionState.data.items.map((item) => (
              <div
                key={item.id}
                className={`d-flex justify-content-between align-items-center border rounded p-3 ${item.isDefault ? "border-success bg-success-subtle" : ""}`}
              >
                <div>
                  <strong>{item.name}</strong>
                  <div className="small text-muted">{item.updatedAt}</div>
                </div>
                {isAdmin ? (
                  <button
                    type="button"
                    className="btn btn-sm btn-outline-primary"
                    onClick={() => openEdit(item.id)}
                  >
                    Изменить
                  </button>
                ) : null}
              </div>
            ))}
          </div>
        )}
      </div>

      {isAdmin ? (
        <div className="container bg-white border rounded my-2 py-3">
          <h2 className="h5">Клиенты</h2>
          <div className="mb-3">
            <input
              className="form-control"
              placeholder="Фильтр по имени, телефону или public id"
              value={clientFilter}
              onChange={(event) => setClientFilter(event.currentTarget.value)}
            />
          </div>
          {clientsState.status === "error" ? (
            <div className="alert alert-danger">{clientsState.message}</div>
          ) : clientsState.status !== "ready" ? (
            <div className="alert alert-info">Загрузка списка клиентов...</div>
          ) : (
            <div className="table-responsive">
              <table className="table table-striped align-middle mb-0">
                <thead>
                  <tr>
                    <th>Название</th>
                    <th>Телефон</th>
                    <th>Уровень</th>
                  </tr>
                </thead>
                <tbody>
                  {filteredClients.map((client) => {
                    const assigned = clientsState.assignments.assignments.find(
                      (assignment) => assignment.phone === client.phone,
                    );
                    const selectedValue =
                      assigned?.priceLevelId != null
                        ? String(assigned.priceLevelId)
                        : "";

                    return (
                      <tr key={client.id}>
                        <td>
                          <div>{client.name}</div>
                          <div className="small text-muted">
                            {client.publicId ?? "Без public id"}
                          </div>
                        </td>
                        <td>{client.phone ?? "Без телефона"}</td>
                        <td>
                          <select
                            className="form-select form-select-sm"
                            value={selectedValue}
                            disabled={!client.phone || !client.publicId}
                            onChange={(event) =>
                              void handleClientAssignment(
                                client,
                                event.currentTarget.value,
                              )
                            }
                          >
                            <option value="">По умолчанию</option>
                            {collectionState.status === "ready"
                              ? collectionState.data.items.map((item) => (
                                  <option key={item.id} value={item.id}>
                                    {item.name}
                                  </option>
                                ))
                              : null}
                          </select>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </div>
      ) : null}

      <div className="modal fade" tabIndex={-1} ref={createModalRef}>
        <div className="modal-dialog modal-lg modal-dialog-centered">
          <div className="modal-content">
            <form onSubmit={handleCreate}>
              <div className="modal-header">
                <h5 className="modal-title">Новый уровень цен</h5>
                <button
                  type="button"
                  className="btn-close"
                  onClick={() => hideBootstrapModal(createModalRef.current)}
                />
              </div>
              {createForm != null ? (
                <div className="modal-body d-flex flex-column gap-3">
                  <div>
                    <label className="form-label">Название</label>
                    <input
                      className="form-control"
                      value={createForm.name}
                      onChange={(event) =>
                        setCreateForm({
                          ...createForm,
                          name: event.currentTarget.value,
                        })
                      }
                    />
                    {fieldErrors.name ? (
                      <div className="text-danger small mt-1">
                        {fieldErrors.name}
                      </div>
                    ) : null}
                  </div>
                  <div className="form-check">
                    <input
                      id="price-level-default"
                      type="checkbox"
                      className="form-check-input"
                      checked={createForm.default}
                      onChange={(event) =>
                        setCreateForm({
                          ...createForm,
                          default: event.currentTarget.checked,
                        })
                      }
                    />
                    <label
                      className="form-check-label"
                      htmlFor="price-level-default"
                    >
                      Уровень по умолчанию
                    </label>
                  </div>
                  <div className="row g-3">
                    <div className="col-md-6">
                      <label className="form-label">Базовый уровень</label>
                      <select
                        className="form-select"
                        value={createForm.basePriceLevelId}
                        onChange={(event) =>
                          setCreateForm({
                            ...createForm,
                            basePriceLevelId: event.currentTarget.value,
                          })
                        }
                      >
                        {collectionState.status === "ready"
                          ? collectionState.data.editorOptions.basePriceLevels.map(
                              (item) => (
                                <option key={item.id} value={item.id}>
                                  {item.name}
                                </option>
                              ),
                            )
                          : null}
                      </select>
                    </div>
                    <div className="col-md-3">
                      <label className="form-label">Модификатор</label>
                      <input
                        className="form-control"
                        value={createForm.priceModifier}
                        onChange={(event) =>
                          setCreateForm({
                            ...createForm,
                            priceModifier: event.currentTarget.value,
                          })
                        }
                      />
                    </div>
                    <div className="col-md-3">
                      <label className="form-label">Тип</label>
                      <select
                        className="form-select"
                        value={createForm.priceModifierKind}
                        onChange={(event) =>
                          setCreateForm({
                            ...createForm,
                            priceModifierKind: event.currentTarget.value as
                              | "percent"
                              | "fixed",
                          })
                        }
                      >
                        <option value="percent">Процент</option>
                        <option value="fixed">Фиксированно</option>
                      </select>
                    </div>
                  </div>
                  <DropdownMultiSelect
                    options={categoryOptions}
                    selectedValues={createForm.excludedCategoryIds}
                    onChange={(values) =>
                      setCreateForm({
                        ...createForm,
                        excludedCategoryIds: values,
                      })
                    }
                    placeholder="Исключить категории"
                    searchPlaceholder="Фильтр категорий"
                    clearable
                  />
                  <ProductSearchPicker
                    label="Исключить товары"
                    selected={createForm.excludedProducts}
                    onChange={(excludedProducts) =>
                      setCreateForm({ ...createForm, excludedProducts })
                    }
                  />
                  <ProductSearchPicker
                    label="Всегда включать товары"
                    selected={createForm.includedProducts}
                    onChange={(includedProducts) =>
                      setCreateForm({ ...createForm, includedProducts })
                    }
                  />
                </div>
              ) : null}
              <div className="modal-footer">
                <button
                  type="button"
                  className="btn btn-outline-secondary"
                  onClick={() => hideBootstrapModal(createModalRef.current)}
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

      <div className="modal fade" tabIndex={-1} ref={editModalRef}>
        <div className="modal-dialog modal-dialog-centered">
          <div className="modal-content">
            <form onSubmit={handleEdit}>
              <div className="modal-header">
                <h5 className="modal-title">Изменить уровень цен</h5>
                <button
                  type="button"
                  className="btn-close"
                  onClick={() => hideBootstrapModal(editModalRef.current)}
                />
              </div>
              <div className="modal-body">
                <div className="mb-3">
                  <label className="form-label">Название</label>
                  <input
                    className="form-control"
                    value={editName}
                    onChange={(event) => setEditName(event.currentTarget.value)}
                  />
                  {fieldErrors.name ? (
                    <div className="text-danger small mt-1">
                      {fieldErrors.name}
                    </div>
                  ) : null}
                </div>
                <div className="form-check">
                  <input
                    id="price-level-edit-default"
                    type="checkbox"
                    className="form-check-input"
                    checked={editDefault}
                    onChange={(event) =>
                      setEditDefault(event.currentTarget.checked)
                    }
                  />
                  <label
                    className="form-check-label"
                    htmlFor="price-level-edit-default"
                  >
                    Уровень по умолчанию
                  </label>
                </div>
              </div>
              <div className="modal-footer justify-content-between">
                <button
                  type="button"
                  className="btn btn-outline-danger"
                  onClick={handleDelete}
                  disabled={isDeleting}
                >
                  {isDeleting ? "Удаление..." : "Удалить"}
                </button>
                <div className="d-flex gap-2">
                  <button
                    type="button"
                    className="btn btn-outline-secondary"
                    onClick={() => hideBootstrapModal(editModalRef.current)}
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
              </div>
            </form>
          </div>
        </div>
      </div>
    </OrdersShell>
  );
}
