import { useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent } from "react";

import { OrdersShell } from "../components/OrdersShell";
import { OrdersShellFatalState } from "../components/OrdersShellFatalState";
import {
  assignVendorUser,
  clearVendorUser,
  createLocalUser,
  createVendor,
  deleteVendor,
  fetchAuthVendorUsers,
  fetchLocalUsers,
  fetchVendorDetails,
  fetchVendorsCollection,
  isApiMutationError,
  toFieldErrorMap,
  updateVendor,
} from "../lib/api";
import { hideBootstrapModal, showBootstrapModal } from "../lib/bootstrap";
import type {
  AuthUserSearchItem,
  LocalUserCollectionData,
  VendorCollectionData,
} from "../lib/models";
import { useOrdersShell } from "../lib/useOrdersShell";

type VendorsQuery = {
  search: string | null;
  page: number;
};

type VendorsState =
  | { status: "loading" }
  | { status: "ready"; data: VendorCollectionData }
  | { status: "error"; message: string };

type LocalUsersState =
  | { status: "loading" }
  | { status: "ready"; data: LocalUserCollectionData }
  | { status: "error"; message: string };

function readQuery(): VendorsQuery {
  const params = new URLSearchParams(window.location.search);
  const page = Number.parseInt(params.get("page") ?? "1", 10);
  const search = params.get("search")?.trim() ?? "";

  return {
    search: search || null,
    page: Number.isInteger(page) && page > 0 ? page : 1,
  };
}

function buildUrl(query: VendorsQuery) {
  const params = new URLSearchParams();

  if (query.search) {
    params.set("search", query.search);
  }

  if (query.page > 1) {
    params.set("page", String(query.page));
  }

  const value = params.toString();
  return value ? `/vendors?${value}` : "/vendors";
}

export function VendorsPage() {
  const shellState = useOrdersShell("Не удалось загрузить оболочку Orders.");
  const [query, setQuery] = useState<VendorsQuery>(readQuery);
  const [searchDraft, setSearchDraft] = useState<string>(
    () => readQuery().search ?? "",
  );
  const [vendorsState, setVendorsState] = useState<VendorsState>({
    status: "loading",
  });
  const [localUsersState, setLocalUsersState] = useState<LocalUsersState>({
    status: "loading",
  });
  const [createName, setCreateName] = useState("");
  const [editId, setEditId] = useState<number | null>(null);
  const [editName, setEditName] = useState("");
  const [vendorFieldErrors, setVendorFieldErrors] = useState<
    Record<string, string>
  >({});
  const [userFieldErrors, setUserFieldErrors] = useState<
    Record<string, string>
  >({});
  const [isSubmittingVendor, setIsSubmittingVendor] = useState(false);
  const [isDeletingVendor, setIsDeletingVendor] = useState(false);
  const [selectedAuthUser, setSelectedAuthUser] =
    useState<AuthUserSearchItem | null>(null);
  const [authSearchQuery, setAuthSearchQuery] = useState("");
  const [authSearchResults, setAuthSearchResults] = useState<
    AuthUserSearchItem[]
  >([]);
  const [isSearchingUsers, setIsSearchingUsers] = useState(false);
  const [assignments, setAssignments] = useState<Record<number, string>>({});
  const [activeAssignmentUserId, setActiveAssignmentUserId] = useState<
    number | null
  >(null);
  const [isCreatingUser, setIsCreatingUser] = useState(false);
  const createModalRef = useRef<HTMLDivElement | null>(null);
  const editModalRef = useRef<HTMLDivElement | null>(null);

  const loadVendors = async (nextQuery: VendorsQuery) => {
    const data = await fetchVendorsCollection(nextQuery);
    setVendorsState({ status: "ready", data });
  };

  const loadLocalUsers = async () => {
    const data = await fetchLocalUsers();
    setLocalUsersState({ status: "ready", data });
    setAssignments(
      Object.fromEntries(
        data.items.map((item) => [
          item.userId,
          item.vendorId != null ? String(item.vendorId) : "",
        ]),
      ),
    );
  };

  const reloadAll = async (nextQuery = query) => {
    await Promise.all([loadVendors(nextQuery), loadLocalUsers()]);
  };

  useEffect(() => {
    const nextUrl = buildUrl(query);
    window.history.replaceState(null, "", nextUrl);

    void reloadAll(query).catch((error) => {
      const message =
        error instanceof Error
          ? error.message
          : "Не удалось загрузить поставщиков.";

      setVendorsState({ status: "error", message });
      setLocalUsersState({ status: "error", message });
    });
  }, [query]);

  useEffect(() => {
    if (
      shellState.status !== "ready" ||
      authSearchQuery.trim().length < 2 ||
      selectedAuthUser?.email === authSearchQuery.trim()
    ) {
      setAuthSearchResults([]);
      return;
    }

    let active = true;
    setIsSearchingUsers(true);
    void fetchAuthVendorUsers(shellState.shell.homeUrl, authSearchQuery.trim())
      .then((items) => {
        if (active) {
          setAuthSearchResults(items);
        }
      })
      .catch(() => {
        if (active) {
          setAuthSearchResults([]);
        }
      })
      .finally(() => {
        if (active) {
          setIsSearchingUsers(false);
        }
      });

    return () => {
      active = false;
    };
  }, [authSearchQuery, selectedAuthUser, shellState]);

  const vendorOptions = useMemo(() => {
    if (vendorsState.status !== "ready") {
      return [];
    }

    return vendorsState.data.items;
  }, [vendorsState]);

  const openCreate = () => {
    setCreateName("");
    setVendorFieldErrors({});
    showBootstrapModal(createModalRef.current);
  };

  const openEdit = (vendorId: number) => {
    setEditId(vendorId);
    setVendorFieldErrors({});
    showBootstrapModal(editModalRef.current);
    void fetchVendorDetails(vendorId)
      .then((data) => setEditName(data.name))
      .catch((error) => {
        window.showFlashMessage?.(
          error instanceof Error
            ? error.message
            : "Не удалось загрузить поставщика.",
          "danger",
        );
        hideBootstrapModal(editModalRef.current);
      });
  };

  const handleCreateVendor = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsSubmittingVendor(true);
    setVendorFieldErrors({});

    try {
      const response = await createVendor({ name: createName });
      hideBootstrapModal(createModalRef.current);
      window.showFlashMessage?.(response.message, "primary");
      await reloadAll();
    } catch (error) {
      if (isApiMutationError(error)) {
        setVendorFieldErrors(toFieldErrorMap(error));
      }
      window.showFlashMessage?.(
        error instanceof Error
          ? error.message
          : "Не удалось создать поставщика.",
        "danger",
      );
    } finally {
      setIsSubmittingVendor(false);
    }
  };

  const handleEditVendor = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (editId == null) {
      return;
    }

    setIsSubmittingVendor(true);
    setVendorFieldErrors({});

    try {
      const response = await updateVendor(editId, {
        vendorId: editId,
        name: editName,
      });
      hideBootstrapModal(editModalRef.current);
      window.showFlashMessage?.(response.message, "primary");
      await reloadAll();
    } catch (error) {
      if (isApiMutationError(error)) {
        setVendorFieldErrors(toFieldErrorMap(error));
      }
      window.showFlashMessage?.(
        error instanceof Error
          ? error.message
          : "Не удалось обновить поставщика.",
        "danger",
      );
    } finally {
      setIsSubmittingVendor(false);
    }
  };

  const handleDeleteVendor = async () => {
    if (editId == null) {
      return;
    }

    setIsDeletingVendor(true);
    try {
      const response = await deleteVendor(editId);
      hideBootstrapModal(editModalRef.current);
      window.showFlashMessage?.(response.message, "primary");
      await reloadAll();
    } catch (error) {
      window.showFlashMessage?.(
        error instanceof Error
          ? error.message
          : "Не удалось удалить поставщика.",
        "danger",
      );
    } finally {
      setIsDeletingVendor(false);
    }
  };

  const handleCreateLocalUser = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (selectedAuthUser == null) {
      setUserFieldErrors({
        email: "Выберите пользователя из выпадающего списка.",
      });
      return;
    }

    setIsCreatingUser(true);
    setUserFieldErrors({});

    try {
      const response = await createLocalUser({
        name: selectedAuthUser.name,
        email: selectedAuthUser.email,
      });
      window.showFlashMessage?.(response.message, "primary");
      setSelectedAuthUser(null);
      setAuthSearchQuery("");
      setAuthSearchResults([]);
      await loadLocalUsers();
    } catch (error) {
      if (isApiMutationError(error)) {
        setUserFieldErrors(toFieldErrorMap(error));
      }
      window.showFlashMessage?.(
        error instanceof Error
          ? error.message
          : "Не удалось добавить пользователя.",
        "danger",
      );
    } finally {
      setIsCreatingUser(false);
    }
  };

  const handleAssign = async (userId: number) => {
    const vendorId = Number.parseInt(assignments[userId] ?? "", 10);
    if (!Number.isInteger(vendorId) || vendorId <= 0) {
      window.showFlashMessage?.("Выберите поставщика.", "danger");
      return;
    }

    setActiveAssignmentUserId(userId);
    try {
      const response = await assignVendorUser({ userId, vendorId });
      window.showFlashMessage?.(response.message, "primary");
      await loadLocalUsers();
    } catch (error) {
      window.showFlashMessage?.(
        error instanceof Error
          ? error.message
          : "Не удалось привязать пользователя.",
        "danger",
      );
    } finally {
      setActiveAssignmentUserId(null);
    }
  };

  const handleClear = async (userId: number) => {
    setActiveAssignmentUserId(userId);
    try {
      const response = await clearVendorUser(userId);
      window.showFlashMessage?.(response.message, "primary");
      await loadLocalUsers();
    } catch (error) {
      window.showFlashMessage?.(
        error instanceof Error ? error.message : "Не удалось снять привязку.",
        "danger",
      );
    } finally {
      setActiveAssignmentUserId(null);
    }
  };

  if (shellState.status === "loading") {
    return (
      <div className="container py-5 text-center text-muted">Загрузка…</div>
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
            setQuery({ search: searchDraft.trim() || null, page: 1 });
          }}
        >
          <input
            className="form-control"
            placeholder="Поиск поставщиков"
            value={searchDraft}
            onChange={(event) => setSearchDraft(event.currentTarget.value)}
          />
        </form>
      }
    >
      <div className="container py-3">
        <div className="bg-white border rounded p-3 mb-3">
          <div className="d-flex justify-content-between align-items-center mb-3">
            <h2 className="h4 mb-0">Поставщики</h2>
            <button
              type="button"
              className="btn btn-primary"
              onClick={openCreate}
            >
              <i className="bi bi-plus-lg me-2" />
              Добавить поставщика
            </button>
          </div>

          {vendorsState.status === "error" ? (
            <div className="alert alert-danger mb-0" role="alert">
              {vendorsState.message}
            </div>
          ) : vendorsState.status === "loading" ? (
            <div className="text-muted">Загрузка поставщиков…</div>
          ) : (
            <>
              <div className="table-responsive">
                <table className="table align-middle mb-0">
                  <thead>
                    <tr>
                      <th>Название</th>
                      <th>Добавлено</th>
                      <th>Обновлено</th>
                      <th className="text-end">Действия</th>
                    </tr>
                  </thead>
                  <tbody>
                    {vendorsState.data.items.length > 0 ? (
                      vendorsState.data.items.map((vendor) => (
                        <tr key={vendor.id}>
                          <td>{vendor.name}</td>
                          <td>{vendor.createdAt}</td>
                          <td>{vendor.updatedAt}</td>
                          <td className="text-end">
                            <button
                              type="button"
                              className="btn btn-sm btn-outline-primary"
                              onClick={() => openEdit(vendor.id)}
                            >
                              Изменить
                            </button>
                          </td>
                        </tr>
                      ))
                    ) : (
                      <tr>
                        <td colSpan={4} className="text-muted text-center py-4">
                          Нет поставщиков для отображения.
                        </td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>

              {vendorsState.data.pagination.totalPages > 1 ? (
                <div className="d-flex justify-content-between align-items-center mt-3">
                  <button
                    type="button"
                    className="btn btn-outline-secondary"
                    disabled={!vendorsState.data.pagination.hasPreviousPage}
                    onClick={() =>
                      setQuery((current) => ({
                        ...current,
                        page: current.page - 1,
                      }))
                    }
                  >
                    Назад
                  </button>
                  <span className="text-muted">
                    Страница {vendorsState.data.pagination.page} из{" "}
                    {vendorsState.data.pagination.totalPages}
                  </span>
                  <button
                    type="button"
                    className="btn btn-outline-secondary"
                    disabled={!vendorsState.data.pagination.hasNextPage}
                    onClick={() =>
                      setQuery((current) => ({
                        ...current,
                        page: current.page + 1,
                      }))
                    }
                  >
                    Вперёд
                  </button>
                </div>
              ) : null}
            </>
          )}
        </div>

        <div className="bg-white border rounded p-3">
          <h2 className="h4 mb-3">Пользователи поставщиков</h2>
          <form
            className="mb-4 position-relative"
            onSubmit={handleCreateLocalUser}
          >
            <label className="form-label">
              Добавить пользователя с ролью поставщика
            </label>
            <div className="input-group">
              <input
                className={`form-control ${userFieldErrors.email ? "is-invalid" : ""}`}
                placeholder="Начните вводить имя или email"
                value={selectedAuthUser?.email ?? authSearchQuery}
                onChange={(event) => {
                  setSelectedAuthUser(null);
                  setAuthSearchQuery(event.currentTarget.value);
                }}
              />
              <button
                type="submit"
                className="btn btn-primary"
                disabled={isCreatingUser}
              >
                {isCreatingUser ? "Добавление…" : "Добавить"}
              </button>
            </div>
            {userFieldErrors.email ? (
              <div className="invalid-feedback d-block">
                {userFieldErrors.email}
              </div>
            ) : null}
            {!selectedAuthUser && authSearchResults.length > 0 ? (
              <div
                className="list-group position-absolute w-100 shadow-sm"
                style={{ zIndex: 10 }}
              >
                {authSearchResults.map((item) => (
                  <button
                    key={item.sub}
                    type="button"
                    className="list-group-item list-group-item-action"
                    onClick={() => {
                      setSelectedAuthUser(item);
                      setAuthSearchQuery(item.email);
                      setAuthSearchResults([]);
                    }}
                  >
                    <strong>{item.name}</strong>
                    <span className="d-block text-muted">{item.email}</span>
                  </button>
                ))}
              </div>
            ) : null}
            {!selectedAuthUser && isSearchingUsers ? (
              <div className="form-text">Поиск пользователей…</div>
            ) : null}
          </form>

          {localUsersState.status === "error" ? (
            <div className="alert alert-danger mb-0" role="alert">
              {localUsersState.message}
            </div>
          ) : localUsersState.status === "loading" ? (
            <div className="text-muted">Загрузка пользователей…</div>
          ) : (
            <div className="table-responsive">
              <table className="table align-middle mb-0">
                <thead>
                  <tr>
                    <th>Пользователь</th>
                    <th>Email</th>
                    <th>Поставщик</th>
                    <th className="text-end">Действия</th>
                  </tr>
                </thead>
                <tbody>
                  {localUsersState.data.items.length > 0 ? (
                    localUsersState.data.items.map((item) => (
                      <tr key={item.userId}>
                        <td>{item.name}</td>
                        <td>{item.email}</td>
                        <td>{item.vendorName ?? "—"}</td>
                        <td className="text-end">
                          <div className="d-flex gap-2 justify-content-end">
                            <select
                              className="form-select form-select-sm"
                              style={{ maxWidth: 260 }}
                              value={assignments[item.userId] ?? ""}
                              onChange={(event) =>
                                setAssignments((current) => ({
                                  ...current,
                                  [item.userId]: event.currentTarget.value,
                                }))
                              }
                            >
                              <option value="">Выберите поставщика</option>
                              {vendorOptions.map((vendor) => (
                                <option key={vendor.id} value={vendor.id}>
                                  {vendor.name}
                                </option>
                              ))}
                            </select>
                            <button
                              type="button"
                              className="btn btn-sm btn-outline-primary"
                              disabled={activeAssignmentUserId === item.userId}
                              onClick={() => void handleAssign(item.userId)}
                            >
                              Назначить
                            </button>
                            {item.vendorId != null ? (
                              <button
                                type="button"
                                className="btn btn-sm btn-outline-secondary"
                                disabled={
                                  activeAssignmentUserId === item.userId
                                }
                                onClick={() => void handleClear(item.userId)}
                              >
                                Снять
                              </button>
                            ) : null}
                          </div>
                        </td>
                      </tr>
                    ))
                  ) : (
                    <tr>
                      <td colSpan={4} className="text-muted text-center py-4">
                        Нет пользователей для отображения.
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </div>

      <div className="modal fade" tabIndex={-1} ref={createModalRef}>
        <div className="modal-dialog modal-dialog-centered">
          <div className="modal-content">
            <form onSubmit={handleCreateVendor}>
              <div className="modal-header">
                <h2 className="modal-title fs-5">Добавить поставщика</h2>
                <button
                  type="button"
                  className="btn-close"
                  data-bs-dismiss="modal"
                />
              </div>
              <div className="modal-body">
                <label className="form-label">Название</label>
                <input
                  className={`form-control ${vendorFieldErrors.name ? "is-invalid" : ""}`}
                  value={createName}
                  onChange={(event) => setCreateName(event.currentTarget.value)}
                />
                {vendorFieldErrors.name ? (
                  <div className="invalid-feedback d-block">
                    {vendorFieldErrors.name}
                  </div>
                ) : null}
              </div>
              <div className="modal-footer">
                <button
                  type="button"
                  className="btn btn-outline-secondary"
                  data-bs-dismiss="modal"
                >
                  Отмена
                </button>
                <button
                  type="submit"
                  className="btn btn-primary"
                  disabled={isSubmittingVendor}
                >
                  {isSubmittingVendor ? "Сохранение…" : "Сохранить"}
                </button>
              </div>
            </form>
          </div>
        </div>
      </div>

      <div className="modal fade" tabIndex={-1} ref={editModalRef}>
        <div className="modal-dialog modal-dialog-centered">
          <div className="modal-content">
            <form onSubmit={handleEditVendor}>
              <div className="modal-header">
                <h2 className="modal-title fs-5">Изменить поставщика</h2>
                <button
                  type="button"
                  className="btn-close"
                  data-bs-dismiss="modal"
                />
              </div>
              <div className="modal-body">
                <label className="form-label">Название</label>
                <input
                  className={`form-control ${vendorFieldErrors.name ? "is-invalid" : ""}`}
                  value={editName}
                  onChange={(event) => setEditName(event.currentTarget.value)}
                />
                {vendorFieldErrors.name ? (
                  <div className="invalid-feedback d-block">
                    {vendorFieldErrors.name}
                  </div>
                ) : null}
              </div>
              <div className="modal-footer justify-content-between">
                <button
                  type="button"
                  className="btn btn-outline-danger"
                  disabled={isDeletingVendor}
                  onClick={() => void handleDeleteVendor()}
                >
                  {isDeletingVendor ? "Удаление…" : "Удалить"}
                </button>
                <div className="d-flex gap-2">
                  <button
                    type="button"
                    className="btn btn-outline-secondary"
                    data-bs-dismiss="modal"
                  >
                    Отмена
                  </button>
                  <button
                    type="submit"
                    className="btn btn-primary"
                    disabled={isSubmittingVendor}
                  >
                    {isSubmittingVendor ? "Сохранение…" : "Сохранить"}
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
