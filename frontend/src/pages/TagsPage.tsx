import { useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent } from "react";

import { OrdersShell } from "../components/OrdersShell";
import { OrdersShellFatalState } from "../components/OrdersShellFatalState";
import {
  createTag,
  deleteTag,
  fetchHubMenuItems,
  fetchShellData,
  fetchTagDetails,
  fetchTagsCollection,
  isApiMutationError,
  toFieldErrorMap,
  updateTag,
} from "../lib/api";
import { hideBootstrapModal, showBootstrapModal } from "../lib/bootstrap";
import type { ShellData, TagCollectionData, UserMenuItem } from "../lib/models";
import { useServiceShell } from "@pushkind/frontend-shell/useServiceShell";

type CollectionState =
  | { status: "loading" }
  | { status: "ready"; data: TagCollectionData }
  | { status: "error"; message: string };

type TagsQuery = {
  search: string | null;
  page: number;
};

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

function readQuery(): TagsQuery {
  const params = new URLSearchParams(window.location.search);
  const page = Number.parseInt(params.get("page") ?? "1", 10);
  const search = params.get("search")?.trim() ?? "";
  return {
    search: search || null,
    page: Number.isInteger(page) && page > 0 ? page : 1,
  };
}

function buildUrl(query: TagsQuery) {
  const params = new URLSearchParams();
  if (query.search) {
    params.set("search", query.search);
  }
  if (query.page > 1) {
    params.set("page", String(query.page));
  }
  const value = params.toString();
  return value ? `/tags?${value}` : "/tags";
}

export function TagsPage() {
  const shellState = useServiceShell<ShellData, UserMenuItem>({
    errorMessage: "Не удалось загрузить оболочку Orders.",
    menuLoadWarning:
      "Failed to load auth navigation menu. Falling back to local Orders menu only.",
    fetchShellData,
    fetchHubMenuItems,
  });
  const [query, setQuery] = useState<TagsQuery>(readQuery);
  const [searchDraft, setSearchDraft] = useState<string>(
    () => readQuery().search ?? "",
  );
  const [collectionState, setCollectionState] = useState<CollectionState>({
    status: "loading",
  });
  const [createName, setCreateName] = useState("");
  const [editId, setEditId] = useState<number | null>(null);
  const [editName, setEditName] = useState("");
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const createModalRef = useRef<HTMLDivElement | null>(null);
  const editModalRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const nextUrl = buildUrl(query);
    window.history.replaceState(null, "", nextUrl);

    void fetchTagsCollection(query)
      .then((data) => setCollectionState({ status: "ready", data }))
      .catch((error) =>
        setCollectionState({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить теги.",
        }),
      );
  }, [query]);

  const isAdmin =
    shellState.status === "ready" &&
    shellState.shell.currentUser.roles.includes("orders_admin");

  const openCreate = () => {
    setCreateName("");
    setFieldErrors({});
    showBootstrapModal(createModalRef.current);
  };

  const openEdit = (tagId: number) => {
    setEditId(tagId);
    setFieldErrors({});
    showBootstrapModal(editModalRef.current);
    void fetchTagDetails(tagId)
      .then((data) => setEditName(data.name))
      .catch((error) => {
        window.showFlashMessage?.(
          error instanceof Error ? error.message : "Не удалось загрузить тег.",
          "danger",
        );
        hideBootstrapModal(editModalRef.current);
      });
  };

  const handleCreate = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsSubmitting(true);
    setFieldErrors({});
    try {
      const response = await createTag({ name: createName });
      hideBootstrapModal(createModalRef.current);
      window.showFlashMessage?.(response.message, "primary");
      setQuery((current) => ({ ...current }));
    } catch (error) {
      if (isApiMutationError(error)) {
        setFieldErrors(toFieldErrorMap(error));
      }
      window.showFlashMessage?.(
        error instanceof Error ? error.message : "Не удалось создать тег.",
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
      const response = await updateTag(editId, {
        tagId: editId,
        name: editName,
      });
      hideBootstrapModal(editModalRef.current);
      window.showFlashMessage?.(response.message, "primary");
      setQuery((current) => ({ ...current }));
    } catch (error) {
      if (isApiMutationError(error)) {
        setFieldErrors(toFieldErrorMap(error));
      }
      window.showFlashMessage?.(
        error instanceof Error ? error.message : "Не удалось обновить тег.",
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
      const response = await deleteTag(editId);
      hideBootstrapModal(editModalRef.current);
      window.showFlashMessage?.(response.message, "primary");
      setQuery((current) => ({ ...current }));
    } catch (error) {
      window.showFlashMessage?.(
        error instanceof Error ? error.message : "Не удалось удалить тег.",
        "danger",
      );
    } finally {
      setIsDeleting(false);
    }
  };

  const handleDeleteInline = async (tagId: number) => {
    setIsDeleting(true);
    try {
      const response = await deleteTag(tagId);
      window.showFlashMessage?.(response.message, "primary");
      setQuery((current) => ({ ...current }));
    } catch (error) {
      window.showFlashMessage?.(
        error instanceof Error ? error.message : "Не удалось удалить тег.",
        "danger",
      );
    } finally {
      setIsDeleting(false);
    }
  };

  const rows = useMemo(() => {
    if (collectionState.status !== "ready") {
      return null;
    }

    if (collectionState.data.items.length === 0) {
      return (
        <div className="alert alert-warning my-2">
          Нет тегов для отображения.
        </div>
      );
    }

    return collectionState.data.items.map((tag) => (
      <div key={tag.id} className="row my-1 py-1 border-top" data-id={tag.id}>
        <div className="col-sm">
          <span className="d-sm-none fw-bold">Название:</span>
          {tag.name}
        </div>
        <div className="col-sm">
          <span className="d-sm-none fw-bold">Добавлено:</span>
          {tag.createdAt}
        </div>
        <div className="col-sm">
          <span className="d-sm-none fw-bold">Обновлено:</span>
          {tag.updatedAt}
        </div>
        <div className="col-sm-2 col-12 d-flex justify-content-sm-end align-items-center mt-2 mt-sm-0">
          <span className="d-sm-none fw-bold me-2">Действия:</span>
          {isAdmin ? (
            <>
              <button
                type="button"
                className="btn btn-sm btn-outline-warning d-flex align-items-center gap-2 me-2"
                onClick={() => openEdit(tag.id)}
              >
                <i className="bi bi-pen" />
                <span className="d-none d-sm-inline">Изменить</span>
              </button>
              <button
                type="button"
                className="btn btn-sm btn-outline-danger d-flex align-items-center gap-2"
                onClick={() => void handleDeleteInline(tag.id)}
                disabled={isDeleting}
              >
                <i className="bi bi-trash" />
                <span className="d-none d-sm-inline">Удалить</span>
              </button>
            </>
          ) : (
            <span className="text-muted">Только просмотр</span>
          )}
        </div>
      </div>
    ));
  }, [collectionState, isAdmin]);

  const paginationPages =
    collectionState.status === "ready"
      ? buildPaginationPages(
          collectionState.data.pagination.totalPages,
          collectionState.data.pagination.page,
        )
      : [];

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
          className="d-flex w-100"
          role="search"
          action="/tags"
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
          <div className="alert alert-info">Загрузка тегов...</div>
        ) : collectionState.status === "error" ? (
          <div className="alert alert-danger">{collectionState.message}</div>
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
            {rows}
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

      <div className="modal fade" tabIndex={-1} ref={createModalRef}>
        <div className="modal-dialog modal-dialog-centered">
          <div className="modal-content">
            <form onSubmit={handleCreate}>
              <div className="modal-header">
                <h5 className="modal-title">Новый тег</h5>
                <button
                  type="button"
                  className="btn-close"
                  onClick={() => hideBootstrapModal(createModalRef.current)}
                />
              </div>
              <div className="modal-body">
                <label className="form-label">Название</label>
                <input
                  className="form-control"
                  value={createName}
                  onChange={(event) => setCreateName(event.currentTarget.value)}
                />
                {fieldErrors.name ? (
                  <div className="text-danger small mt-1">
                    {fieldErrors.name}
                  </div>
                ) : null}
              </div>
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
                <h5 className="modal-title">Изменить тег</h5>
                <button
                  type="button"
                  className="btn-close"
                  onClick={() => hideBootstrapModal(editModalRef.current)}
                />
              </div>
              <div className="modal-body">
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
