import { useEffect, useMemo, useRef, useState } from "react";
import type { ChangeEvent, FormEvent, RefObject } from "react";

import { DropdownMultiSelect } from "@pushkind/frontend-shell/DropdownMultiSelect";
import {
  MarkdownComposer,
  renderMarkdownToHtml,
} from "@pushkind/frontend-shell/markdown";
import { OrdersShell } from "../components/OrdersShell";
import { OrdersShellFatalState } from "../components/OrdersShellFatalState";
import {
  createProduct,
  fetchHubMenuItems,
  fetchProductDetails,
  fetchProductsCollection,
  fetchShellData,
  isApiMutationError,
  toFieldErrorMap,
  updateProduct,
  uploadProducts,
} from "../lib/api";
import {
  disposeBootstrapModal,
  hideBootstrapModal,
  showBootstrapModal,
} from "../lib/bootstrap";
import type {
  ProductCollectionData,
  ProductDetailsData,
  ProductEditorOptions,
  ProductMutationInput,
  ProductMutationSuccess,
  ProductNamedOption,
  ProductPriceLevelInput,
  ShellData,
  UserMenuItem,
} from "../lib/models";
import { useServiceShell } from "@pushkind/frontend-shell/useServiceShell";

type ProductsCollectionState =
  | { status: "loading" }
  | { status: "ready"; data: ProductCollectionData }
  | { status: "error"; message: string };

type ProductDetailsState =
  | { status: "idle" }
  | { status: "loading"; productId: number }
  | { status: "ready"; data: ProductDetailsData }
  | { status: "error"; productId: number; message: string };

type ProductsQuery = {
  search: string | null;
  page: number;
  showArchived: boolean;
};

type ProductFormState = {
  name: string;
  sku: string;
  descriptionSource: string;
  units: string;
  amount: string;
  currency: string;
  categoryId: string;
  vendorId: string;
  tagIds: string[];
  imageUrls: string;
  isArchived: boolean;
  priceLevels: Record<number, string>;
};

const formatterCache = new Map<string, Intl.NumberFormat | null>();

function readProductsQueryFromLocation(): ProductsQuery {
  if (typeof window === "undefined") {
    return { search: null, page: 1, showArchived: false };
  }

  const params = new URLSearchParams(window.location.search);
  const rawSearch = params.get("search")?.trim() ?? "";
  const rawPage = Number(params.get("page") ?? "1");
  const page = Number.isInteger(rawPage) && rawPage > 0 ? rawPage : 1;
  const showArchived = params.get("show_archived") === "true";

  return {
    search: rawSearch.length > 0 ? rawSearch : null,
    page,
    showArchived,
  };
}

export function buildProductsPageUrl(
  page: number,
  search: string | null,
  showArchived: boolean,
) {
  const params = new URLSearchParams();

  if (search) {
    params.set("search", search);
  }

  if (page > 1) {
    params.set("page", String(page));
  }

  if (showArchived) {
    params.set("show_archived", "true");
  }

  const queryString = params.toString();
  return queryString ? `/products?${queryString}` : "/products";
}

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

function normalizeOptionalText(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function createPriceLevelDrafts(
  editorOptions: ProductEditorOptions,
  priceLevels?: ProductDetailsData["priceLevels"],
) {
  const submittedValues = new Map(
    (priceLevels ?? []).map((level) => [
      level.priceLevelId,
      (level.priceCents / 100).toFixed(2),
    ]),
  );

  return Object.fromEntries(
    editorOptions.priceLevels.map((level) => [
      level.id,
      submittedValues.get(level.id) ?? "",
    ]),
  );
}

function buildCreateProductForm(
  options: ProductEditorOptions,
): ProductFormState {
  return {
    name: "",
    sku: "",
    descriptionSource: "",
    units: "",
    amount: "",
    currency: "RUB",
    categoryId: "",
    vendorId: options.vendors.length > 0 ? "0" : "",
    tagIds: [],
    imageUrls: "",
    isArchived: false,
    priceLevels: createPriceLevelDrafts(options),
  };
}

function buildEditProductForm(product: ProductDetailsData): ProductFormState {
  return {
    name: product.name,
    sku: product.sku ?? "",
    descriptionSource: product.descriptionHtml ?? "",
    units: product.units ?? "",
    amount: product.amount ?? "",
    currency: product.currency,
    categoryId: product.categoryId == null ? "0" : String(product.categoryId),
    vendorId:
      product.editorOptions.vendors.length > 0
        ? String(product.vendorId ?? 0)
        : "",
    tagIds: product.tagIds.map(String),
    imageUrls: product.imageUrls.join("\n"),
    isArchived: product.isArchived,
    priceLevels: createPriceLevelDrafts(
      product.editorOptions,
      product.priceLevels,
    ),
  };
}

function buildProductMutationInput(
  form: ProductFormState,
  editorOptions: ProductEditorOptions,
  productId?: number,
): ProductMutationInput {
  const amount = normalizeOptionalText(form.amount);
  const imageUrls = form.imageUrls
    .split("\n")
    .map((value) => value.trim())
    .filter((value) => value.length > 0);

  const priceLevels: ProductPriceLevelInput[] = editorOptions.priceLevels.map(
    (level) => ({
      priceLevelId: level.id,
      price: form.priceLevels[level.id]?.trim() ?? "",
    }),
  );

  return {
    productId,
    name: form.name,
    sku: normalizeOptionalText(form.sku),
    descriptionHtml: normalizeOptionalText(
      renderMarkdownToHtml(form.descriptionSource),
    ),
    units: normalizeOptionalText(form.units),
    amount: amount == null ? null : Number.parseFloat(amount),
    currency: form.currency,
    isArchived: form.isArchived,
    categoryId:
      form.categoryId.trim().length === 0
        ? null
        : Number.parseInt(form.categoryId, 10),
    vendorId:
      form.vendorId.trim().length === 0
        ? null
        : Number.parseInt(form.vendorId, 10),
    tagIds: form.tagIds.map((tagId) => Number.parseInt(tagId, 10)),
    imageUrls,
    priceLevels,
  };
}

function hasActiveProductFilters(query: ProductsQuery) {
  return query.showArchived || query.search != null;
}

function previewImageUrl(imageUrls: string[]) {
  return imageUrls[0] ?? "/assets/placeholder.png";
}

function reloadProductsCollection(
  query: ProductsQuery,
  setState: (state: ProductsCollectionState) => void,
) {
  setState({ status: "loading" });

  return fetchProductsCollection({
    search: query.search,
    page: query.page,
    showArchived: query.showArchived,
  })
    .then((data) => {
      setState({ status: "ready", data });
      return data;
    })
    .catch((error) => {
      setState({
        status: "error",
        message:
          error instanceof Error
            ? error.message
            : "Не удалось загрузить список товаров.",
      });
      throw error;
    });
}

function ProductMarkdownEditor({
  idPrefix,
  value,
  onChange,
  disabled,
  filesServiceUrl,
}: {
  idPrefix: string;
  value: string;
  onChange: (value: string) => void;
  disabled: boolean;
  filesServiceUrl: string;
}) {
  return (
    <MarkdownComposer
      id={`${idPrefix}-markdown`}
      value={value}
      onChange={onChange}
      rows={10}
      disabled={disabled}
      placeholder="Описание в формате markdown"
      editorLabel="Маркдаун"
      previewLabel="Превью"
      fileBrowserLabel="Файлы"
      previewClassName="orders-markdown-preview"
      emptyPreviewLabel="Введите markdown, чтобы увидеть превью."
      fileBrowser={
        filesServiceUrl
          ? {
              baseUrl: filesServiceUrl,
              helpText:
                "Загрузите или найдите файл, скопируйте ссылку и вставьте её в markdown.",
            }
          : undefined
      }
    />
  );
}

function ProductFormFields({
  idPrefix,
  editorOptions,
  form,
  fieldErrors,
  isSubmitting,
  showArchived,
  filesServiceUrl,
  onFieldChange,
  onTagIdsChange,
}: {
  idPrefix: string;
  editorOptions: ProductEditorOptions;
  form: ProductFormState;
  fieldErrors: Record<string, string>;
  isSubmitting: boolean;
  showArchived: boolean;
  filesServiceUrl: string;
  onFieldChange: <K extends keyof ProductFormState>(
    field: K,
    value: ProductFormState[K],
  ) => void;
  onTagIdsChange: (value: string[]) => void;
}) {
  const tagOptions = useMemo(
    () =>
      editorOptions.tags.map((tag) => ({
        value: String(tag.id),
        label: tag.name,
      })),
    [editorOptions.tags],
  );

  return (
    <>
      <div className="row mb-3">
        <label htmlFor={`${idPrefix}-name`} className="col-md-3 col-form-label">
          Название
        </label>
        <div className="col-md-9">
          <input
            id={`${idPrefix}-name`}
            type="text"
            className={`form-control${fieldErrors.name ? " is-invalid" : ""}`}
            placeholder="Например: «Кофе арабика»"
            maxLength={128}
            value={form.name}
            onChange={(event) =>
              onFieldChange("name", event.currentTarget.value)
            }
            disabled={isSubmitting}
            required
          />
          {fieldErrors.name ? (
            <div className="invalid-feedback">{fieldErrors.name}</div>
          ) : null}
        </div>
      </div>

      <div className="row mb-3">
        <label htmlFor={`${idPrefix}-sku`} className="col-md-3 col-form-label">
          Артикул
        </label>
        <div className="col-md-9">
          <input
            id={`${idPrefix}-sku`}
            type="text"
            className={`form-control${fieldErrors.sku ? " is-invalid" : ""}`}
            placeholder="Необязательно"
            maxLength={64}
            value={form.sku}
            onChange={(event) =>
              onFieldChange("sku", event.currentTarget.value)
            }
            disabled={isSubmitting}
          />
          <div className="form-text">
            {idPrefix === "edit-product"
              ? "Очистите поле, чтобы убрать SKU."
              : "Если не задано, SKU останется пустым."}
          </div>
          {fieldErrors.sku ? (
            <div className="invalid-feedback d-block">{fieldErrors.sku}</div>
          ) : null}
        </div>
      </div>

      <div className="row mb-3">
        <label
          htmlFor={`${idPrefix}-image-urls`}
          className="col-md-3 col-form-label"
        >
          Ссылки на изображения
        </label>
        <div className="col-md-9">
          <textarea
            id={`${idPrefix}-image-urls`}
            className="form-control"
            rows={4}
            value={form.imageUrls}
            onChange={(event) =>
              onFieldChange("imageUrls", event.currentTarget.value)
            }
            placeholder="Каждая ссылка с новой строки"
            disabled={isSubmitting}
          />
          <div className="form-text">
            Укажите один URL на строку. Оставьте поле пустым, если изображений
            нет.
          </div>
        </div>
      </div>

      <div className="row mb-3">
        <label
          htmlFor={`${idPrefix}-category`}
          className="col-md-3 col-form-label"
        >
          Категория
        </label>
        <div className="col-md-5 col-lg-4">
          <select
            id={`${idPrefix}-category`}
            className={`form-select${fieldErrors.category_id ? " is-invalid" : ""}`}
            value={form.categoryId}
            onChange={(event) =>
              onFieldChange("categoryId", event.currentTarget.value)
            }
            disabled={isSubmitting}
          >
            <option value={idPrefix === "edit-product" ? "0" : ""}>
              Без категории
            </option>
            {editorOptions.categories.map((category) => (
              <option key={category.id} value={category.id}>
                {category.name}
              </option>
            ))}
          </select>
          <div className="form-text">
            Выберите категорию, чтобы сгруппировать товар.
          </div>
          {fieldErrors.category_id ? (
            <div className="invalid-feedback">{fieldErrors.category_id}</div>
          ) : null}
        </div>
      </div>

      {editorOptions.vendors.length > 0 ? (
        <div className="row mb-3">
          <label
            htmlFor={`${idPrefix}-vendor`}
            className="col-md-3 col-form-label"
          >
            Поставщик
          </label>
          <div className="col-md-5 col-lg-4">
            <select
              id={`${idPrefix}-vendor`}
              className={`form-select${fieldErrors.vendor_id ? " is-invalid" : ""}`}
              value={form.vendorId}
              onChange={(event) =>
                onFieldChange("vendorId", event.currentTarget.value)
              }
              disabled={isSubmitting}
            >
              <option value="0">Без поставщика</option>
              {editorOptions.vendors.map((vendor) => (
                <option key={vendor.id} value={vendor.id}>
                  {vendor.name}
                </option>
              ))}
            </select>
            <div className="form-text">
              Выберите поставщика, чтобы связать товар.
            </div>
            {fieldErrors.vendor_id ? (
              <div className="invalid-feedback">{fieldErrors.vendor_id}</div>
            ) : null}
          </div>
        </div>
      ) : null}

      <div className="row mb-3">
        <label htmlFor={`${idPrefix}-tags`} className="col-md-3 col-form-label">
          Теги
        </label>
        <div className="col-md-9">
          {tagOptions.length > 0 ? (
            <>
              <DropdownMultiSelect
                id={`${idPrefix}-tags`}
                options={tagOptions}
                selectedValues={form.tagIds}
                onChange={onTagIdsChange}
                placeholder="Выберите теги"
                searchPlaceholder="Поиск тегов"
                emptyResultsLabel="Теги не найдены"
                menuHeightClassName="shell-dropdown-multiselect-options-md"
                clearable
              />
              <div className="form-text">
                Выберите один или несколько тегов. Оставьте поле пустым, чтобы
                не добавлять теги.
              </div>
            </>
          ) : (
            <div
              className="alert alert-secondary py-2 mb-0 small"
              role="status"
            >
              Теги ещё не созданы. Добавьте их на вкладке «Теги», чтобы
              привязать к товарам.
            </div>
          )}
        </div>
      </div>

      <div className="row mb-3">
        <label
          htmlFor={`${idPrefix}-units`}
          className="col-md-3 col-form-label"
        >
          Единица измерения
        </label>
        <div className="col-md-5 col-lg-4">
          <input
            id={`${idPrefix}-units`}
            type="text"
            className={`form-control${fieldErrors.units ? " is-invalid" : ""}`}
            placeholder="Например: кг"
            maxLength={32}
            value={form.units}
            onChange={(event) =>
              onFieldChange("units", event.currentTarget.value)
            }
            disabled={isSubmitting}
          />
          <div className="form-text">
            Отображается рядом с ценой (оставьте пустым, если не требуется).
          </div>
          {fieldErrors.units ? (
            <div className="invalid-feedback">{fieldErrors.units}</div>
          ) : null}
        </div>
      </div>

      <div className="row mb-3">
        <label
          htmlFor={`${idPrefix}-amount`}
          className="col-md-3 col-form-label"
        >
          Объём
        </label>
        <div className="col-md-5 col-lg-4">
          <input
            id={`${idPrefix}-amount`}
            type="number"
            step="0.01"
            className={`form-control${fieldErrors.amount ? " is-invalid" : ""}`}
            placeholder="Например: 1.0"
            value={form.amount}
            onChange={(event) =>
              onFieldChange("amount", event.currentTarget.value)
            }
            disabled={isSubmitting}
          />
          <div className="form-text">
            Отображается рядом с ценой (оставьте пустым, если не требуется).
          </div>
          {fieldErrors.amount ? (
            <div className="invalid-feedback">{fieldErrors.amount}</div>
          ) : null}
        </div>
      </div>

      <div className="row mb-3">
        <label
          htmlFor={`${idPrefix}-currency`}
          className="col-md-3 col-form-label"
        >
          Валюта
        </label>
        <div className="col-md-5 col-lg-4">
          <input
            id={`${idPrefix}-currency`}
            type="text"
            className={`form-control text-uppercase${fieldErrors.currency ? " is-invalid" : ""}`}
            placeholder="Например: USD"
            maxLength={3}
            value={form.currency}
            onChange={(event) =>
              onFieldChange("currency", event.currentTarget.value)
            }
            disabled={isSubmitting}
            required
          />
          <div className="form-text">Трёхсимвольный код ISO 4217.</div>
          {fieldErrors.currency ? (
            <div className="invalid-feedback">{fieldErrors.currency}</div>
          ) : null}
        </div>
      </div>

      {editorOptions.priceLevels.length > 0 ? (
        <div className="mb-3">
          <label className="form-label">Цены по уровням</label>
          <div className="row g-2">
            {editorOptions.priceLevels.map((level) => (
              <div key={level.id} className="col-12 col-lg-6">
                <div className="input-group mb-2">
                  <span className="input-group-text">{level.name}</span>
                  <input
                    type="number"
                    step="0.01"
                    className={`form-control${fieldErrors.price_levels ? " is-invalid" : ""}`}
                    placeholder="Например: 9.99"
                    value={form.priceLevels[level.id] ?? ""}
                    onChange={(event) =>
                      onFieldChange("priceLevels", {
                        ...form.priceLevels,
                        [level.id]: event.currentTarget.value,
                      })
                    }
                    disabled={isSubmitting}
                  />
                </div>
              </div>
            ))}
          </div>
          <div className="form-text">
            Укажите цену в валюте товара для каждого уровня. Оставьте поле
            пустым, чтобы пропустить уровень.
          </div>
          {fieldErrors.price_levels ? (
            <div className="invalid-feedback d-block">
              {fieldErrors.price_levels}
            </div>
          ) : null}
        </div>
      ) : null}

      <div className="mb-3">
        <label className="form-label">Описание</label>
        <ProductMarkdownEditor
          idPrefix={idPrefix}
          value={form.descriptionSource}
          onChange={(value) => onFieldChange("descriptionSource", value)}
          disabled={isSubmitting}
          filesServiceUrl={filesServiceUrl}
        />
        {fieldErrors.description ? (
          <div className="invalid-feedback d-block">
            {fieldErrors.description}
          </div>
        ) : null}
      </div>

      {showArchived ? (
        <div className="row">
          <div className="col">
            <div className="form-check">
              <input
                className="form-check-input"
                type="checkbox"
                id={`${idPrefix}-archived`}
                checked={form.isArchived}
                onChange={(event) =>
                  onFieldChange("isArchived", event.currentTarget.checked)
                }
                disabled={isSubmitting}
              />
              <label
                className="form-check-label"
                htmlFor={`${idPrefix}-archived`}
              >
                Архивировать товар
              </label>
            </div>
            <div className="form-text">
              Архивированные товары не отображаются покупателям, но сохраняются
              в истории.
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}

function ProductsFiltersModal({ query }: { query: ProductsQuery }) {
  return (
    <div
      className="modal fade"
      id="productsFiltersModal"
      tabIndex={-1}
      aria-labelledby="productsFiltersModalLabel"
      aria-hidden="true"
    >
      <div className="modal-dialog modal-lg modal-dialog-centered">
        <div className="modal-content">
          <div className="modal-header">
            <h1 className="modal-title fs-5" id="productsFiltersModalLabel">
              Фильтры товаров
            </h1>
            <button
              type="button"
              className="btn-close"
              data-bs-dismiss="modal"
              aria-label="Закрыть"
            />
          </div>
          <form className="modal-body row g-3" method="get" action="/products">
            <div className="col-12">
              <label htmlFor="productsSearch" className="form-label">
                Поиск
              </label>
              <input
                id="productsSearch"
                type="search"
                name="search"
                className="form-control"
                defaultValue={query.search ?? ""}
                placeholder="Название, артикул, категория, поставщик"
              />
            </div>
            <div className="col-12">
              <div className="form-check">
                <input
                  className="form-check-input"
                  type="checkbox"
                  value="true"
                  id="filterArchived"
                  name="show_archived"
                  defaultChecked={query.showArchived}
                />
                <label className="form-check-label" htmlFor="filterArchived">
                  Показывать архивированные товары
                </label>
              </div>
            </div>
            <div className="col-12 d-flex flex-wrap gap-2 justify-content-end pt-3">
              <button type="submit" className="btn btn-primary">
                Применить
              </button>
              <a href="/products" className="btn btn-outline-secondary">
                Сбросить
              </a>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}

function ProductCreateModal({
  modalRef,
  editorOptions,
  filesServiceUrl,
  onCreated,
  onUploaded,
}: {
  modalRef: RefObject<HTMLDivElement | null>;
  editorOptions: ProductEditorOptions;
  filesServiceUrl: string;
  onCreated: (result: ProductMutationSuccess) => void;
  onUploaded: (message: string) => void;
}) {
  const [form, setForm] = useState<ProductFormState>(() =>
    buildCreateProductForm(editorOptions),
  );
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isUploading, setIsUploading] = useState(false);
  const [uploadFieldError, setUploadFieldError] = useState<string | null>(null);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);

  useEffect(() => {
    setForm(buildCreateProductForm(editorOptions));
  }, [editorOptions]);

  useEffect(() => {
    const modalElement = modalRef.current;
    if (modalElement == null) {
      return undefined;
    }

    const handleHidden = () => {
      setForm(buildCreateProductForm(editorOptions));
      setFieldErrors({});
      setFormError(null);
      setUploadFieldError(null);
      setSelectedFile(null);
      setIsSubmitting(false);
      setIsUploading(false);
    };

    modalElement.addEventListener("hidden.bs.modal", handleHidden);
    return () => {
      modalElement.removeEventListener("hidden.bs.modal", handleHidden);
    };
  }, [editorOptions, modalRef]);

  function updateField<K extends keyof ProductFormState>(
    field: K,
    value: ProductFormState[K],
  ) {
    setForm((current) => ({
      ...current,
      [field]: value,
    }));
    setFieldErrors((current) => ({ ...current, [field]: "" }));
    setFormError(null);
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsSubmitting(true);
    setFieldErrors({});
    setFormError(null);

    try {
      const result = await createProduct(
        buildProductMutationInput(form, editorOptions),
      );
      onCreated(result);
      hideBootstrapModal(modalRef.current);
    } catch (error) {
      if (isApiMutationError(error)) {
        setFieldErrors(toFieldErrorMap(error));
        setFormError(error.message);
      } else {
        setFormError(
          error instanceof Error ? error.message : "Не удалось создать товар.",
        );
      }
    } finally {
      setIsSubmitting(false);
    }
  }

  async function handleUploadSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (selectedFile == null) {
      setUploadFieldError("Выберите CSV-файл для загрузки.");
      return;
    }

    setIsUploading(true);
    setUploadFieldError(null);

    try {
      const result = await uploadProducts(selectedFile);
      onUploaded(result.message);
      hideBootstrapModal(modalRef.current);
    } catch (error) {
      if (isApiMutationError(error)) {
        setUploadFieldError(error.field_errors[0]?.message ?? error.message);
      } else {
        setUploadFieldError(
          error instanceof Error
            ? error.message
            : "Не удалось загрузить товары.",
        );
      }
    } finally {
      setIsUploading(false);
    }
  }

  return (
    <div
      className="modal fade"
      id="productModal"
      tabIndex={-1}
      aria-labelledby="productModalLabel"
      aria-hidden="true"
      ref={modalRef}
    >
      <div className="modal-dialog modal-lg">
        <div className="modal-content">
          <div className="modal-header">
            <h1 className="modal-title fs-5" id="productModalLabel">
              Добавить товар
            </h1>
            <button
              type="button"
              className="btn-close"
              data-bs-dismiss="modal"
              aria-label="Закрыть"
            />
          </div>
          <div className="modal-body">
            <form onSubmit={handleSubmit}>
              {formError ? (
                <div className="alert alert-danger" role="alert">
                  {formError}
                </div>
              ) : null}
              <ProductFormFields
                idPrefix="create-product"
                editorOptions={editorOptions}
                form={form}
                fieldErrors={fieldErrors}
                isSubmitting={isSubmitting}
                showArchived={false}
                filesServiceUrl={filesServiceUrl}
                onFieldChange={updateField}
                onTagIdsChange={(value) => updateField("tagIds", value)}
              />
              <div className="row mb-3">
                <div className="col">
                  <button
                    className="btn btn-primary"
                    type="submit"
                    disabled={isSubmitting || isUploading}
                  >
                    {isSubmitting ? "Сохранение..." : "Сохранить"}
                  </button>
                </div>
              </div>
            </form>
          </div>
          <div className="modal-footer">
            <form className="w-100" onSubmit={handleUploadSubmit}>
              <div className="row g-2 align-items-center">
                <div className="col">
                  <input
                    className={`form-control${uploadFieldError ? " is-invalid" : ""}`}
                    type="file"
                    accept=".csv"
                    onChange={(event: ChangeEvent<HTMLInputElement>) => {
                      setSelectedFile(event.currentTarget.files?.[0] ?? null);
                      setUploadFieldError(null);
                    }}
                    disabled={isUploading || isSubmitting}
                  />
                  {uploadFieldError ? (
                    <div className="invalid-feedback d-block">
                      {uploadFieldError}
                    </div>
                  ) : null}
                </div>
                <div className="col-auto">
                  <button
                    className="btn btn-success"
                    type="submit"
                    disabled={isUploading || isSubmitting}
                  >
                    {isUploading ? "Загрузка..." : "Загрузить CSV"}
                  </button>
                </div>
              </div>
              <div className="form-text">
                Ожидаются столбцы <code>name</code>, <code>currency</code>,
                опционально <code>sku</code>, <code>description</code>,
                <code>units</code> и цены по именам уровней.
              </div>
            </form>
          </div>
        </div>
      </div>
    </div>
  );
}

function ProductEditModal({
  modalRef,
  productState,
  onUpdated,
}: {
  modalRef: RefObject<HTMLDivElement | null>;
  productState: ProductDetailsState;
  onUpdated: (result: ProductMutationSuccess) => void;
}) {
  const [form, setForm] = useState<ProductFormState | null>(null);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (productState.status === "ready") {
      setForm(buildEditProductForm(productState.data));
      setFieldErrors({});
      setFormError(null);
      setIsSubmitting(false);
    }
  }, [productState]);

  useEffect(() => {
    const modalElement = modalRef.current;
    if (modalElement == null) {
      return undefined;
    }

    const handleHidden = () => {
      setFieldErrors({});
      setFormError(null);
      setIsSubmitting(false);
    };

    modalElement.addEventListener("hidden.bs.modal", handleHidden);
    return () => {
      modalElement.removeEventListener("hidden.bs.modal", handleHidden);
    };
  }, [modalRef]);

  function updateField<K extends keyof ProductFormState>(
    field: K,
    value: ProductFormState[K],
  ) {
    if (form == null) {
      return;
    }

    setForm({
      ...form,
      [field]: value,
    });
    setFieldErrors((current) => ({ ...current, [field]: "" }));
    setFormError(null);
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (productState.status !== "ready" || form == null) {
      return;
    }

    setIsSubmitting(true);
    setFieldErrors({});
    setFormError(null);

    try {
      const result = await updateProduct(
        productState.data.id,
        buildProductMutationInput(
          form,
          productState.data.editorOptions,
          productState.data.id,
        ),
      );
      onUpdated(result);
      hideBootstrapModal(modalRef.current);
    } catch (error) {
      if (isApiMutationError(error)) {
        setFieldErrors(toFieldErrorMap(error));
        setFormError(error.message);
      } else {
        setFormError(
          error instanceof Error ? error.message : "Не удалось обновить товар.",
        );
      }
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <div
      className="modal fade"
      id="editProductModal"
      tabIndex={-1}
      aria-labelledby="editProductModalLabel"
      aria-hidden="true"
      ref={modalRef}
    >
      <div className="modal-dialog modal-lg modal-dialog-centered">
        <div className="modal-content">
          <form onSubmit={handleSubmit}>
            <div className="modal-header">
              <h1 className="modal-title fs-5" id="editProductModalLabel">
                Редактировать товар
              </h1>
              <button
                type="button"
                className="btn-close"
                data-bs-dismiss="modal"
                aria-label="Закрыть"
              />
            </div>
            <div className="modal-body">
              {productState.status === "loading" ? (
                <div className="py-4 text-center text-secondary">
                  Загружаем товар...
                </div>
              ) : null}

              {productState.status === "error" ? (
                <div className="alert alert-danger mb-0" role="alert">
                  {productState.message}
                </div>
              ) : null}

              {productState.status === "ready" && form != null ? (
                <>
                  {formError ? (
                    <div className="alert alert-danger" role="alert">
                      {formError}
                    </div>
                  ) : null}
                  <ProductFormFields
                    idPrefix="edit-product"
                    editorOptions={productState.data.editorOptions}
                    form={form}
                    fieldErrors={fieldErrors}
                    isSubmitting={isSubmitting}
                    showArchived
                    filesServiceUrl={productState.data.filesServiceUrl}
                    onFieldChange={updateField}
                    onTagIdsChange={(value) => updateField("tagIds", value)}
                  />
                </>
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
                disabled={productState.status !== "ready" || isSubmitting}
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

function ProductsPagination({
  query,
  totalPages,
}: {
  query: ProductsQuery;
  totalPages: number;
}) {
  const pages = buildPaginationPages(totalPages, query.page);

  if (pages.length === 0) {
    return null;
  }

  return (
    <nav aria-label="pagination">
      <ul
        className="pagination justify-content-center flex-wrap"
        id="pagination"
      >
        {pages.map((page, index) =>
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
                href={buildProductsPageUrl(
                  page,
                  query.search,
                  query.showArchived,
                )}
              >
                {page}
              </a>
            </li>
          ) : (
            <li key={page} className="page-item active" aria-current="page">
              <span className="page-link">{page}</span>
            </li>
          ),
        )}
      </ul>
    </nav>
  );
}

export function ProductsEmptyState() {
  return (
    <div className="alert alert-warning my-2" role="alert">
      Нет товаров для отображения.
    </div>
  );
}

export function ProductsPage() {
  const shellState = useServiceShell<ShellData, UserMenuItem>({
    errorMessage: "Не удалось загрузить оболочку Orders.",
    menuLoadWarning:
      "Failed to load auth navigation menu. Falling back to local Orders menu only.",
    fetchShellData,
    fetchHubMenuItems,
  });
  const query = readProductsQueryFromLocation();
  const addModalRef = useRef<HTMLDivElement | null>(null);
  const editModalRef = useRef<HTMLDivElement | null>(null);
  const [productsState, setProductsState] = useState<ProductsCollectionState>({
    status: "loading",
  });
  const [productState, setProductState] = useState<ProductDetailsState>({
    status: "idle",
  });

  useEffect(() => {
    return () => {
      disposeBootstrapModal(addModalRef.current);
      disposeBootstrapModal(editModalRef.current);
    };
  }, []);

  useEffect(() => {
    let active = true;
    setProductsState({ status: "loading" });

    void fetchProductsCollection({
      search: query.search,
      page: query.page,
      showArchived: query.showArchived,
    })
      .then((data) => {
        if (!active) {
          return;
        }

        setProductsState({ status: "ready", data });
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setProductsState({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить список товаров.",
        });
      });

    return () => {
      active = false;
    };
  }, [query.page, query.search, query.showArchived]);

  if (shellState.status === "error") {
    return <OrdersShellFatalState message={shellState.message} />;
  }

  if (shellState.status === "loading") {
    return null;
  }

  async function refreshCollection(showMessage?: string) {
    try {
      await reloadProductsCollection(query, setProductsState);
      if (showMessage) {
        window.showFlashMessage?.(showMessage, "success");
      }
    } catch {
      if (showMessage) {
        window.showFlashMessage?.(showMessage, "success");
      }
    }
  }

  function handleCreateSuccess(result: ProductMutationSuccess) {
    void refreshCollection(result.message);
  }

  function handleUploadSuccess(message: string) {
    void refreshCollection(message);
  }

  function handleEditSuccess(result: ProductMutationSuccess) {
    setProductState({ status: "ready", data: result.product });
    void refreshCollection(result.message);
  }

  function handleProductRowClick(productId: number) {
    setProductState({ status: "loading", productId });
    showBootstrapModal(editModalRef.current);

    void fetchProductDetails(productId)
      .then((data) => {
        setProductState({ status: "ready", data });
      })
      .catch((error) => {
        setProductState({
          status: "error",
          productId,
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить товар.",
        });
      });
  }

  const editorOptions =
    productsState.status === "ready"
      ? productsState.data.editorOptions
      : {
          categories: [],
          tags: [],
          priceLevels: [],
          vendors: [],
        };
  const filesServiceUrl =
    productsState.status === "ready" ? productsState.data.filesServiceUrl : "";

  const searchForm = (
    <form className="d-flex w-100" role="search" action="/products">
      {query.showArchived ? (
        <input type="hidden" name="show_archived" value="true" />
      ) : null}
      <div className="input-group me-2">
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
          <div className="row mb-3">
            <div className="col text-center add-item-container">
              <button
                className="btn btn-link"
                type="button"
                onClick={() => showBootstrapModal(addModalRef.current)}
              >
                <i className="bi bi-plus-circle" />
              </button>
            </div>
            <div className="col-auto">
              <button
                className="btn btn-sm btn-outline-secondary d-flex align-items-center gap-2 mt-1"
                type="button"
                data-bs-toggle="modal"
                data-bs-target="#productsFiltersModal"
              >
                <i className="bi bi-funnel" />
                <span
                  id="activeFiltersBadge"
                  className={`badge text-bg-primary ${hasActiveProductFilters(query) ? "" : "d-none"}`}
                >
                  •
                </span>
              </button>
            </div>
          </div>

          {productsState.status === "loading" ? (
            <div className="alert alert-info my-2" role="status">
              Загрузка списка товаров...
            </div>
          ) : null}

          {productsState.status === "error" ? (
            <div className="alert alert-danger my-2" role="alert">
              {productsState.message}
            </div>
          ) : null}

          {productsState.status === "ready" ? (
            <>
              <div className="row d-none d-lg-flex fw-bold">
                <div className="col-lg-4 overflow-hidden">Название</div>
                <div className="col-lg-2 overflow-hidden">Артикул</div>
                <div className="col-lg overflow-hidden">Описание</div>
                <div className="col-lg-1 overflow-hidden">Ед. изм.</div>
              </div>
              <div id="productList">
                {productsState.data.items.length > 0 ? (
                  productsState.data.items.map((product) => (
                    <div
                      key={product.id}
                      className={`row my-1 py-2 border-top selectable ${product.isArchived ? "product-archived" : ""}`}
                      data-id={product.id}
                      onClick={() => handleProductRowClick(product.id)}
                      onKeyDown={(event) => {
                        if (event.key === "Enter" || event.key === " ") {
                          event.preventDefault();
                          handleProductRowClick(product.id);
                        }
                      }}
                      role="button"
                      tabIndex={0}
                    >
                      <div className="col-lg-4 col-12 d-flex justify-content-between align-items-start gap-2">
                        <div className="d-flex align-items-start gap-2 flex-grow-1">
                          <img
                            src={previewImageUrl(product.imageUrls)}
                            alt={`Изображение ${product.name}`}
                            width="32"
                            height="32"
                            className="rounded border"
                            loading="lazy"
                            style={{ objectFit: "cover" }}
                          />
                          <div className="flex-grow-1">
                            <span className="d-lg-none fw-bold">Название:</span>{" "}
                            {product.name}
                            {product.category ? (
                              <div className="text-muted small mt-1">
                                Категория: {product.category.name}
                              </div>
                            ) : null}
                            {product.vendor ? (
                              <div className="text-muted small mt-1">
                                Поставщик: {product.vendor.name}
                              </div>
                            ) : null}
                            <div className="text-muted small mt-1">
                              Обновлён {product.updatedAt}
                              {product.isArchived ? " · Архивирован" : ""}
                            </div>
                          </div>
                        </div>
                      </div>
                      <div className="col-lg-2 col-12">
                        <span className="d-lg-none fw-bold">Артикул:</span>{" "}
                        {product.sku ?? "—"}
                      </div>
                      <div className="col-lg col-12">
                        <span className="d-lg-none fw-bold">Описание:</span>
                        {product.descriptionHtml ? (
                          <div
                            dangerouslySetInnerHTML={{
                              __html: product.descriptionHtml,
                            }}
                          />
                        ) : null}
                      </div>
                      <div className="col-lg-1 col-6">
                        <span className="d-lg-none fw-bold">Ед. изм.:</span>{" "}
                        {product.amount ?? ""} {product.units ?? ""}
                      </div>
                      {product.priceLevels.length > 0 ? (
                        <div className="col-12 mt-2">
                          <div className="d-flex flex-wrap gap-2 small">
                            {product.priceLevels.map((level) => (
                              <span
                                key={`${product.id}-${level.priceLevelId}`}
                                className="badge bg-secondary-subtle text-body-secondary border"
                              >
                                {level.priceLevelName} —{" "}
                                {formatMoney(
                                  level.priceCents,
                                  product.currency,
                                )}
                                {product.units ? (
                                  <>
                                    {" "}
                                    / {product.amount ?? ""} {product.units}
                                  </>
                                ) : null}
                              </span>
                            ))}
                          </div>
                        </div>
                      ) : null}
                      {product.tags.length > 0 ? (
                        <div className="col-12 mt-2">
                          <div className="d-flex flex-wrap gap-2 small">
                            {product.tags.map((tag) => (
                              <span
                                key={`${product.id}-tag-${tag.id}`}
                                className="badge bg-info-subtle text-body-secondary border"
                              >
                                {tag.name}
                              </span>
                            ))}
                          </div>
                        </div>
                      ) : null}
                    </div>
                  ))
                ) : (
                  <ProductsEmptyState />
                )}
              </div>

              <ProductsPagination
                query={query}
                totalPages={productsState.data.pagination.totalPages}
              />
            </>
          ) : null}
        </div>
      </main>

      <ProductCreateModal
        modalRef={addModalRef}
        editorOptions={editorOptions}
        filesServiceUrl={filesServiceUrl}
        onCreated={handleCreateSuccess}
        onUploaded={handleUploadSuccess}
      />
      <ProductEditModal
        modalRef={editModalRef}
        productState={productState}
        onUpdated={handleEditSuccess}
      />
      <ProductsFiltersModal query={query} />
    </OrdersShell>
  );
}
