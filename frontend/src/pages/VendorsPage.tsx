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

function buildPaginationPages(totalPages: number, currentPage: number) {
  if (totalPages <= 1) {
    return [];
  }

  const pages = new Set<number>();
  pages.add(1);
  pages.add(totalPages);

  for (let page = currentPage - 1; page <= currentPage + 1; page += 1) {
    if (page >= 1 && page <= totalPages) {
      pages.add(page);
    }
  }

  const orderedPages = [...pages].sort((left, right) => left - right);
  const sequence: Array<number | null> = [];

  orderedPages.forEach((page, index) => {
    const previous = orderedPages[index - 1];
    if (previous != null && page - previous > 1) {
      sequence.push(null);
    }
    sequence.push(page);
  });

  return sequence;
}

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

  const paginationPages =
    vendorsState.status === "ready"
      ? buildPaginationPages(
          vendorsState.data.pagination.totalPages,
          vendorsState.data.pagination.page,
        )
      : [];

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

  const handleDeleteVendorInline = async (vendorId: number) => {
    setIsDeletingVendor(true);
    try {
      const response = await deleteVendor(vendorId);
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
          className="d-flex w-100"
          role="search"
          action="/vendors"
          onSubmit={(event) => {
            event.preventDefault();
            setQuery({ search: searchDraft.trim() || null, page: 1 });
          }}
        >
          <div className="input-group me-2">
            <input
              required
              name="search"
              className="form-control"
              type="search"
              placeholder="Поиск"
              aria-label="Search"
              value={searchDraft}
              onChange={(event) => setSearchDraft(event.currentTarget.value)}
            />
            <button className="btn btn-outline-secondary" type="submit">
              <i className="bi bi-search" />
            </button>
          </div>
        </form>
      }
    >
      <div className="container bg-white border rounded my-2">
        <div className="row mb-3">
          <div className="col text-center add-item-container">
            <button type="button" className="btn btn-link" onClick={openCreate}>
              <i className="bi bi-plus-circle" />
            </button>
          </div>
        </div>
        {vendorsState.status === "error" ? (
          <div className="alert alert-danger mb-0" role="alert">
            {vendorsState.message}
          </div>
        ) : vendorsState.status === "loading" ? (
          <div className="alert alert-info mb-0">Загрузка поставщиков...</div>
        ) : (
          <>
            <div className="row d-none d-sm-flex fw-bold">
              <div className="col-sm overflow-hidden">Название</div>
              <div className="col-sm overflow-hidden">Добавлено</div>
              <div className="col-sm overflow-hidden">Обновлено</div>
              <div className="col-sm-2 overflow-hidden text-sm-end">
                Действия
              </div>
            </div>
            <div id="vendorList">
              {vendorsState.data.items.length > 0 ? (
                vendorsState.data.items.map((vendor) => (
                  <div
                    key={vendor.id}
                    className="row my-1 py-1 border-top"
                    data-id={vendor.id}
                  >
                    <div className="col-sm">
                      <span className="d-sm-none fw-bold">Название:</span>
                      {vendor.name}
                    </div>
                    <div className="col-sm">
                      <span className="d-sm-none fw-bold">Добавлено:</span>
                      {vendor.createdAt}
                    </div>
                    <div className="col-sm">
                      <span className="d-sm-none fw-bold">Обновлено:</span>
                      {vendor.updatedAt}
                    </div>
                    <div className="col-sm-2 col-12 d-flex justify-content-sm-end align-items-center mt-2 mt-sm-0">
                      <span className="d-sm-none fw-bold me-2">Действия:</span>
                      <button
                        type="button"
                        className="btn btn-sm btn-outline-warning d-flex align-items-center gap-2 me-2"
                        onClick={() => openEdit(vendor.id)}
                      >
                        <i className="bi bi-pen" />
                        <span className="d-none d-sm-inline">Изменить</span>
                      </button>
                      <button
                        type="button"
                        className="btn btn-sm btn-outline-danger d-flex align-items-center gap-2"
                        onClick={() => void handleDeleteVendorInline(vendor.id)}
                        disabled={isDeletingVendor}
                      >
                        <i className="bi bi-trash" />
                        <span className="d-none d-sm-inline">Удалить</span>
                      </button>
                    </div>
                  </div>
                ))
              ) : (
                <div className="alert alert-warning my-2" role="alert">
                  Нет поставщиков для отображения.
                </div>
              )}
            </div>
            {paginationPages.length > 0 ? (
              <nav aria-label="pagination">
                <ul
                  className="pagination justify-content-center flex-wrap"
                  id="pagination"
                >
                  {paginationPages.map((page, index) =>
                    page == null ? (
                      <li key={`ellipsis-${index}`} className="page-item">
                        <span className="ellipsis page-link border-0 bg-transparent">
                          …
                        </span>
                      </li>
                    ) : page !== query.page ? (
                      <li key={page} className="page-item">
                        <a
                          className="page-link"
                          href={buildUrl({ ...query, page })}
                        >
                          {page}
                        </a>
                      </li>
                    ) : (
                      <li
                        key={page}
                        className="page-item active"
                        aria-current="page"
                      >
                        <span className="page-link">{page}</span>
                      </li>
                    ),
                  )}
                </ul>
              </nav>
            ) : null}
          </>
        )}
      </div>

      <div className="container bg-white border rounded my-2">
        <div className="row">
          <div className="col">
            <form method="post" onSubmit={handleCreateLocalUser}>
              <div className="row">
                <div className="col position-relative">
                  <input
                    className={`form-control mt-2 ${userFieldErrors.email ? "is-invalid" : ""}`}
                    placeholder="Добавить пользователя c ролью поставщик"
                    value={selectedAuthUser?.email ?? authSearchQuery}
                    onChange={(event) => {
                      setSelectedAuthUser(null);
                      setAuthSearchQuery(event.currentTarget.value);
                    }}
                  />
                  {userFieldErrors.email ? (
                    <div className="invalid-feedback d-block">
                      {userFieldErrors.email}
                    </div>
                  ) : null}
                  {!selectedAuthUser && authSearchResults.length > 0 ? (
                    <div
                      className="list-group position-absolute start-0 end-0 shadow-sm mt-1"
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
                          <strong>
                            {item.name} ({item.email})
                          </strong>
                        </button>
                      ))}
                    </div>
                  ) : null}
                </div>
                <div className="col-auto">
                  <button
                    type="submit"
                    className="btn btn-primary my-1"
                    disabled={isCreatingUser}
                  >
                    <i className="bi bi-plus" />
                  </button>
                </div>
              </div>
              {!selectedAuthUser && isSearchingUsers ? (
                <div className="form-text">Поиск пользователей…</div>
              ) : null}
            </form>
          </div>
        </div>

        <h5 className="mt-3">Привязка пользователей</h5>

        {localUsersState.status === "error" ? (
          <div className="alert alert-danger mb-0" role="alert">
            {localUsersState.message}
          </div>
        ) : localUsersState.status === "loading" ? (
          <div className="text-muted">Загрузка пользователей…</div>
        ) : (
          <>
            <div className="row d-none d-sm-flex fw-bold">
              <div className="col-sm overflow-hidden">Пользователь</div>
              <div className="col-sm overflow-hidden">Email</div>
              <div className="col-sm overflow-hidden">Поставщик</div>
              <div className="col-sm-3 overflow-hidden text-sm-end">
                Действия
              </div>
            </div>
            {localUsersState.data.items.length > 0 ? (
              localUsersState.data.items.map((item) => (
                <div key={item.userId} className="row my-1 py-2 border-top">
                  <div className="col-sm">
                    <span className="d-sm-none fw-bold">Пользователь:</span>
                    {item.name}
                  </div>
                  <div className="col-sm">
                    <span className="d-sm-none fw-bold">Email:</span>
                    {item.email}
                  </div>
                  <div className="col-sm">
                    <span className="d-sm-none fw-bold">Поставщик:</span>
                    {item.vendorName ?? "—"}
                  </div>
                  <div className="col-sm-3 col-12 d-flex justify-content-sm-end align-items-center gap-2 mt-2 mt-sm-0">
                    <div className="d-flex align-items-center gap-2">
                      <select
                        className="form-select form-select-sm"
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
                    </div>
                    {item.vendorId != null ? (
                      <button
                        type="button"
                        className="btn btn-sm btn-outline-secondary"
                        disabled={activeAssignmentUserId === item.userId}
                        onClick={() => void handleClear(item.userId)}
                      >
                        Снять
                      </button>
                    ) : null}
                  </div>
                </div>
              ))
            ) : (
              <div className="alert alert-warning my-2" role="alert">
                Нет пользователей для отображения.
              </div>
            )}
          </>
        )}
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
