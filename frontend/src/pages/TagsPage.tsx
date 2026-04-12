import { useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent } from "react";

import { OrdersShell } from "../components/OrdersShell";
import { OrdersShellFatalState } from "../components/OrdersShellFatalState";
import {
  createTag,
  deleteTag,
  fetchTagDetails,
  fetchTagsCollection,
  isApiMutationError,
  toFieldErrorMap,
  updateTag,
} from "../lib/api";
import { hideBootstrapModal, showBootstrapModal } from "../lib/bootstrap";
import type { TagCollectionData } from "../lib/models";
import { useOrdersShell } from "../lib/useOrdersShell";

type CollectionState =
  | { status: "loading" }
  | { status: "ready"; data: TagCollectionData }
  | { status: "error"; message: string };

type TagsQuery = {
  search: string | null;
  page: number;
};

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
  const shellState = useOrdersShell("Не удалось загрузить оболочку Orders.");
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
      <div key={tag.id} className="row my-1 py-2 border-top align-items-center">
        <div className="col-sm">{tag.name}</div>
        <div className="col-sm">{tag.updatedAt}</div>
        <div className="col-sm-3 d-flex justify-content-sm-end mt-2 mt-sm-0">
          {isAdmin ? (
            <>
              <button
                type="button"
                className="btn btn-sm btn-outline-warning me-2"
                onClick={() => openEdit(tag.id)}
              >
                Изменить
              </button>
              <button
                type="button"
                className="btn btn-sm btn-outline-danger"
                onClick={() => openEdit(tag.id)}
              >
                Удалить
              </button>
            </>
          ) : (
            <span className="text-muted">Только просмотр</span>
          )}
        </div>
      </div>
    ));
  }, [collectionState, isAdmin]);

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
            setQuery({ search: searchDraft.trim() || null, page: 1 });
          }}
        >
          <input
            className="form-control"
            placeholder="Поиск тегов"
            value={searchDraft}
            onChange={(event) => setSearchDraft(event.currentTarget.value)}
          />
        </form>
      }
    >
      <div className="container bg-white border rounded my-2 py-3">
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
              <div className="col-sm">Название</div>
              <div className="col-sm">Обновлено</div>
              <div className="col-sm-3 text-sm-end">Действия</div>
            </div>
            {rows}
            <div className="d-flex justify-content-between align-items-center border-top pt-3 mt-3">
              <span className="text-muted small">
                Страница {collectionState.data.pagination.page} из{" "}
                {collectionState.data.pagination.totalPages || 1}
              </span>
              <div className="btn-group">
                <button
                  type="button"
                  className="btn btn-outline-secondary btn-sm"
                  disabled={!collectionState.data.pagination.hasPreviousPage}
                  onClick={() =>
                    setQuery((current) => ({
                      ...current,
                      page: current.page - 1,
                    }))
                  }
                >
                  Назад
                </button>
                <button
                  type="button"
                  className="btn btn-outline-secondary btn-sm"
                  disabled={!collectionState.data.pagination.hasNextPage}
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
            </div>
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
