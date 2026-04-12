import { useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent } from "react";

import { OrdersShell } from "../components/OrdersShell";
import { OrdersShellFatalState } from "../components/OrdersShellFatalState";
import {
  createCategory,
  deleteCategory,
  fetchCategoriesCollection,
  fetchCategoryDetails,
  isApiMutationError,
  toFieldErrorMap,
  updateCategory,
} from "../lib/api";
import { hideBootstrapModal, showBootstrapModal } from "../lib/bootstrap";
import type {
  CategoryCollectionData,
  CategoryDetailsData,
  CategoryMutationInput,
  CategoryTreeNode,
} from "../lib/models";
import { useOrdersShell } from "../lib/useOrdersShell";

type CollectionState =
  | { status: "loading" }
  | { status: "ready"; data: CategoryCollectionData }
  | { status: "error"; message: string };

type CategoryFormState = {
  name: string;
  description: string;
  imageUrl: string;
  isArchived: boolean;
  parentId: number | null;
  parentName: string | null;
};

const emptyCreateForm = (): CategoryFormState => ({
  name: "",
  description: "",
  imageUrl: "",
  isArchived: false,
  parentId: null,
  parentName: null,
});

function buildEditForm(category: CategoryDetailsData): CategoryFormState {
  return {
    name: category.name,
    description: category.description ?? "",
    imageUrl: category.imageUrl ?? "",
    isArchived: category.isArchived,
    parentId: category.parentId,
    parentName: null,
  };
}

function toMutationInput(form: CategoryFormState): CategoryMutationInput {
  return {
    name: form.name,
    description: form.description.trim() || null,
    imageUrl: form.imageUrl.trim() || null,
    parentId: form.parentId,
    isArchived: form.isArchived,
  };
}

function CategoryTree({
  nodes,
  canCreate,
  isAdmin,
  onAddChild,
  onEdit,
}: {
  nodes: CategoryTreeNode[];
  canCreate: boolean;
  isAdmin: boolean;
  onAddChild: (node: CategoryTreeNode) => void;
  onEdit: (node: CategoryTreeNode) => void;
}) {
  return (
    <div className="d-flex flex-column gap-3">
      {nodes.map((node) => (
        <CategoryTreeNodeCard
          key={node.id}
          node={node}
          canCreate={canCreate}
          isAdmin={isAdmin}
          onAddChild={onAddChild}
          onEdit={onEdit}
        />
      ))}
    </div>
  );
}

function CategoryTreeNodeCard({
  node,
  canCreate,
  isAdmin,
  onAddChild,
  onEdit,
}: {
  node: CategoryTreeNode;
  canCreate: boolean;
  isAdmin: boolean;
  onAddChild: (node: CategoryTreeNode) => void;
  onEdit: (node: CategoryTreeNode) => void;
}) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div>
      <div
        className={`d-flex align-items-start gap-3 rounded-3 border bg-white p-3 shadow-sm ${node.isArchived ? "opacity-75" : ""}`}
      >
        {node.children.length > 0 ? (
          <button
            type="button"
            className="btn btn-sm btn-link px-0 text-secondary"
            onClick={() => setExpanded((value) => !value)}
          >
            <i
              className={`bi ${expanded ? "bi-chevron-down" : "bi-chevron-right"}`}
            />
          </button>
        ) : (
          <span className="text-secondary px-1">
            <i className="bi bi-dot" />
          </span>
        )}
        <div className="flex-grow-1">
          <div className="d-flex flex-wrap align-items-center gap-2">
            {node.imageUrl ? (
              <img src={node.imageUrl} alt="" width={18} height={18} />
            ) : null}
            <strong>{node.name}</strong>
            {node.isArchived ? (
              <span className="badge text-bg-warning-subtle text-warning-emphasis">
                Архивирована
              </span>
            ) : null}
          </div>
          {node.description ? (
            <div className="small text-muted mt-1">{node.description}</div>
          ) : null}
        </div>
        <div className="d-flex align-items-center gap-2">
          {canCreate ? (
            <button
              type="button"
              className="btn btn-sm btn-outline-success"
              onClick={() => onAddChild(node)}
            >
              <i className="bi bi-plus-lg" />
            </button>
          ) : null}
          {isAdmin ? (
            <button
              type="button"
              className="btn btn-sm btn-outline-primary"
              onClick={() => onEdit(node)}
            >
              <i className="bi bi-pencil-square" />
            </button>
          ) : null}
        </div>
      </div>
      {expanded && node.children.length > 0 ? (
        <div className="mt-3 ms-4">
          <CategoryTree
            nodes={node.children}
            canCreate={canCreate}
            isAdmin={isAdmin}
            onAddChild={onAddChild}
            onEdit={onEdit}
          />
        </div>
      ) : null}
    </div>
  );
}

export function CategoriesPage() {
  const shellState = useOrdersShell("Не удалось загрузить оболочку Orders.");
  const [collectionState, setCollectionState] = useState<CollectionState>({
    status: "loading",
  });
  const [createForm, setCreateForm] =
    useState<CategoryFormState>(emptyCreateForm);
  const [createFieldErrors, setCreateFieldErrors] = useState<
    Record<string, string>
  >({});
  const [isCreating, setIsCreating] = useState(false);
  const [editingCategoryId, setEditingCategoryId] = useState<number | null>(
    null,
  );
  const [editForm, setEditForm] = useState<CategoryFormState>(emptyCreateForm);
  const [editFieldErrors, setEditFieldErrors] = useState<
    Record<string, string>
  >({});
  const [isEditLoading, setIsEditLoading] = useState(false);
  const [isUpdating, setIsUpdating] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const createModalRef = useRef<HTMLDivElement | null>(null);
  const editModalRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    void fetchCategoriesCollection()
      .then((data) => setCollectionState({ status: "ready", data }))
      .catch((error) =>
        setCollectionState({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить категории.",
        }),
      );
  }, []);

  const roles =
    shellState.status === "ready" ? shellState.shell.currentUser.roles : [];
  const isAdmin = roles.includes("orders_admin");
  const canCreate = isAdmin || roles.includes("orders_vendor");

  const openCreateModal = (parent?: CategoryTreeNode) => {
    setCreateForm({
      ...emptyCreateForm(),
      parentId: parent?.id ?? null,
      parentName: parent?.name ?? null,
    });
    setCreateFieldErrors({});
    showBootstrapModal(createModalRef.current);
  };

  const openEditModal = (node: CategoryTreeNode) => {
    setEditingCategoryId(node.id);
    setEditFieldErrors({});
    setIsEditLoading(true);
    showBootstrapModal(editModalRef.current);
    void fetchCategoryDetails(node.id)
      .then((category) => {
        setEditForm(buildEditForm(category));
      })
      .catch((error) => {
        window.showFlashMessage?.(
          error instanceof Error
            ? error.message
            : "Не удалось загрузить категорию.",
          "danger",
        );
        hideBootstrapModal(editModalRef.current);
      })
      .finally(() => setIsEditLoading(false));
  };

  const reload = async () => {
    const data = await fetchCategoriesCollection();
    setCollectionState({ status: "ready", data });
  };

  const handleCreateSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsCreating(true);
    setCreateFieldErrors({});

    try {
      const response = await createCategory(toMutationInput(createForm));
      await reload();
      hideBootstrapModal(createModalRef.current);
      setCreateForm(emptyCreateForm());
      window.showFlashMessage?.(response.message, "primary");
    } catch (error) {
      if (isApiMutationError(error)) {
        setCreateFieldErrors(toFieldErrorMap(error));
      }
      window.showFlashMessage?.(
        error instanceof Error
          ? error.message
          : "Не удалось создать категорию.",
        "danger",
      );
    } finally {
      setIsCreating(false);
    }
  };

  const handleEditSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (editingCategoryId == null) {
      return;
    }

    setIsUpdating(true);
    setEditFieldErrors({});

    try {
      const response = await updateCategory(
        editingCategoryId,
        toMutationInput(editForm),
      );
      await reload();
      hideBootstrapModal(editModalRef.current);
      window.showFlashMessage?.(response.message, "primary");
    } catch (error) {
      if (isApiMutationError(error)) {
        setEditFieldErrors(toFieldErrorMap(error));
      }
      window.showFlashMessage?.(
        error instanceof Error
          ? error.message
          : "Не удалось обновить категорию.",
        "danger",
      );
    } finally {
      setIsUpdating(false);
    }
  };

  const handleDelete = async () => {
    if (editingCategoryId == null) {
      return;
    }

    setIsDeleting(true);
    try {
      const response = await deleteCategory(editingCategoryId);
      await reload();
      hideBootstrapModal(editModalRef.current);
      window.showFlashMessage?.(response.message, "primary");
    } catch (error) {
      window.showFlashMessage?.(
        error instanceof Error
          ? error.message
          : "Не удалось удалить категорию.",
        "danger",
      );
    } finally {
      setIsDeleting(false);
    }
  };

  const content = useMemo(() => {
    if (collectionState.status === "loading") {
      return <div className="alert alert-info">Загрузка категорий...</div>;
    }

    if (collectionState.status === "error") {
      return (
        <div className="alert alert-danger">{collectionState.message}</div>
      );
    }

    if (collectionState.data.items.length === 0) {
      return (
        <div className="alert alert-warning">Категории пока не созданы.</div>
      );
    }

    return (
      <CategoryTree
        nodes={collectionState.data.items}
        canCreate={canCreate}
        isAdmin={isAdmin}
        onAddChild={openCreateModal}
        onEdit={openEditModal}
      />
    );
  }, [canCreate, collectionState, isAdmin]);

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
    >
      <div className="container bg-white border rounded my-2 py-3">
        <div className="row mb-3">
          <div className="col text-center add-item-container">
            {canCreate ? (
              <button
                type="button"
                className="btn btn-link"
                onClick={() => openCreateModal()}
              >
                <i className="bi bi-plus-circle" />
              </button>
            ) : null}
          </div>
        </div>
        {content}
      </div>

      <div className="modal fade" tabIndex={-1} ref={createModalRef}>
        <div className="modal-dialog modal-lg modal-dialog-centered">
          <div className="modal-content">
            <form onSubmit={handleCreateSubmit}>
              <div className="modal-header">
                <h5 className="modal-title">
                  Новая категория
                  {createForm.parentName ? ` для ${createForm.parentName}` : ""}
                </h5>
                <button
                  type="button"
                  className="btn-close"
                  onClick={() => hideBootstrapModal(createModalRef.current)}
                />
              </div>
              <div className="modal-body">
                <div className="mb-3">
                  <label className="form-label">Название</label>
                  <input
                    className="form-control"
                    value={createForm.name}
                    onChange={(event) =>
                      setCreateForm((current) => ({
                        ...current,
                        name: event.currentTarget.value,
                      }))
                    }
                  />
                  {createFieldErrors.name ? (
                    <div className="text-danger small mt-1">
                      {createFieldErrors.name}
                    </div>
                  ) : null}
                </div>
                <div className="mb-3">
                  <label className="form-label">Описание</label>
                  <textarea
                    className="form-control"
                    rows={3}
                    value={createForm.description}
                    onChange={(event) =>
                      setCreateForm((current) => ({
                        ...current,
                        description: event.currentTarget.value,
                      }))
                    }
                  />
                </div>
                <div className="mb-0">
                  <label className="form-label">Ссылка на изображение</label>
                  <input
                    className="form-control"
                    value={createForm.imageUrl}
                    onChange={(event) =>
                      setCreateForm((current) => ({
                        ...current,
                        imageUrl: event.currentTarget.value,
                      }))
                    }
                  />
                  {createFieldErrors.image_url ? (
                    <div className="text-danger small mt-1">
                      {createFieldErrors.image_url}
                    </div>
                  ) : null}
                </div>
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
                  disabled={isCreating}
                >
                  {isCreating ? "Сохранение..." : "Сохранить"}
                </button>
              </div>
            </form>
          </div>
        </div>
      </div>

      <div className="modal fade" tabIndex={-1} ref={editModalRef}>
        <div className="modal-dialog modal-lg modal-dialog-centered">
          <div className="modal-content">
            {isEditLoading ? (
              <div className="modal-body">Загрузка...</div>
            ) : (
              <form onSubmit={handleEditSubmit}>
                <div className="modal-header">
                  <h5 className="modal-title">Изменить категорию</h5>
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
                      value={editForm.name}
                      onChange={(event) =>
                        setEditForm((current) => ({
                          ...current,
                          name: event.currentTarget.value,
                        }))
                      }
                    />
                    {editFieldErrors.name ? (
                      <div className="text-danger small mt-1">
                        {editFieldErrors.name}
                      </div>
                    ) : null}
                  </div>
                  <div className="mb-3">
                    <label className="form-label">Описание</label>
                    <textarea
                      className="form-control"
                      rows={3}
                      value={editForm.description}
                      onChange={(event) =>
                        setEditForm((current) => ({
                          ...current,
                          description: event.currentTarget.value,
                        }))
                      }
                    />
                  </div>
                  <div className="mb-3">
                    <label className="form-label">Ссылка на изображение</label>
                    <input
                      className="form-control"
                      value={editForm.imageUrl}
                      onChange={(event) =>
                        setEditForm((current) => ({
                          ...current,
                          imageUrl: event.currentTarget.value,
                        }))
                      }
                    />
                    {editFieldErrors.image_url ? (
                      <div className="text-danger small mt-1">
                        {editFieldErrors.image_url}
                      </div>
                    ) : null}
                  </div>
                  <div className="form-check">
                    <input
                      id="category-is-archived"
                      type="checkbox"
                      className="form-check-input"
                      checked={editForm.isArchived}
                      onChange={(event) =>
                        setEditForm((current) => ({
                          ...current,
                          isArchived: event.currentTarget.checked,
                        }))
                      }
                    />
                    <label
                      className="form-check-label"
                      htmlFor="category-is-archived"
                    >
                      Архивировать
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
                      disabled={isUpdating}
                    >
                      {isUpdating ? "Сохранение..." : "Сохранить"}
                    </button>
                  </div>
                </div>
              </form>
            )}
          </div>
        </div>
      </div>
    </OrdersShell>
  );
}
