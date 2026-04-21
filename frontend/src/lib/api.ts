import {
  browserLocation,
  ensureResponseIsNotAuthRedirect,
  fetchHubMenuItems as fetchSharedHubMenuItems,
  fetchJson as fetchSharedJson,
  fetchNoAccessData as fetchSharedNoAccessData,
  fetchShellData as fetchSharedShellData,
  isJsonResponse,
  parseMenuItems,
  readJsonResponse,
} from "@pushkind/frontend-shell/shellApi";

export { browserLocation };
import type {
  ApiFieldError,
  ApiMutationError,
  AssignVendorUserInput,
  AuthUserSearchItem,
  AddLocalUserInput,
  CategoryCollectionData,
  CategoryDetailsData,
  CategoryMutationInput,
  CategoryMutationSuccess,
  CategoryTreeNode,
  ClientPriceLevelAssignmentInput,
  ClientPriceLevelAssignments,
  CrmClientListItem,
  NoAccessData,
  OrderApprovalUpdateInput,
  OrderCollectionData,
  OrderCollectionFilters,
  OrderCustomerSummary,
  OrderDetailsData,
  OrderListItem,
  OrderMutationSuccess,
  OrderPagination,
  OrderProductItem,
  OrderUpdateInput,
  ProductCollectionData,
  ProductCollectionFilters,
  ProductDetailsData,
  ProductEditorOptions,
  ProductListItem,
  ProductMutationInput,
  ProductMutationSuccess,
  ProductNamedOption,
  ProductPagination,
  ProductPriceLevelRate,
  PriceLevelCollectionData,
  PriceLevelDetailsData,
  PriceLevelEditorOptions,
  PriceLevelMutationInput,
  PriceLevelMutationSuccess,
  PriceLevelUpdateInput,
  ProductUploadSuccess,
  ShellData,
  TagCollectionData,
  TagDetailsData,
  TagListItem,
  TagMutationInput,
  TagMutationSuccess,
  LocalUserCollectionData,
  LocalUserListItem,
  UserMenuItem,
  VendorCollectionData,
  VendorDetailsData,
  VendorListItem,
  VendorMutationInput,
  VendorMutationSuccess,
} from "./models";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readString(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error(`Invalid API response: expected string at ${key}.`);
  }

  return value;
}

function readNullableString(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (value === null) {
    return null;
  }

  if (typeof value !== "string") {
    throw new Error(`Invalid API response: expected string|null at ${key}.`);
  }

  return value;
}

function readNumber(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (typeof value !== "number") {
    throw new Error(`Invalid API response: expected number at ${key}.`);
  }

  return value;
}

function readNullableNumber(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (value === null) {
    return null;
  }

  if (typeof value !== "number") {
    throw new Error(`Invalid API response: expected number|null at ${key}.`);
  }

  return value;
}

function readBoolean(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (typeof value !== "boolean") {
    throw new Error(`Invalid API response: expected boolean at ${key}.`);
  }

  return value;
}

function readStringArray(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) {
    throw new Error(`Invalid API response: expected string[] at ${key}.`);
  }

  return value;
}

function parseApiFieldErrors(payload: unknown): ApiFieldError[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid mutation field errors payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid mutation field error payload.");
    }

    return {
      field: readString(item, "field"),
      message: readString(item, "message"),
    };
  });
}

function parseApiMutationError(payload: unknown): ApiMutationError {
  if (!isRecord(payload)) {
    throw new Error("Invalid mutation error payload.");
  }

  return {
    message: readString(payload, "message"),
    field_errors: parseApiFieldErrors(payload.field_errors),
  };
}

function statusMutationError(response: Response): ApiMutationError {
  if (response.status === 401) {
    return {
      message: "Сессия истекла. Войдите снова и повторите действие.",
      field_errors: [],
    };
  }

  if (response.status === 403) {
    return {
      message: "Недостаточно прав для выполнения действия.",
      field_errors: [],
    };
  }

  return {
    message: `Запрос не выполнен. Статус: ${response.status}.`,
    field_errors: [],
  };
}

async function readMutationPayload(
  response: Response,
  endpoint: string,
): Promise<unknown> {
  ensureResponseIsNotAuthRedirect(response);

  if (!response.ok && !isJsonResponse(response)) {
    throw statusMutationError(response);
  }

  const payload = await readJsonResponse<unknown>(response, endpoint);

  if (!response.ok) {
    throw parseApiMutationError(payload);
  }

  return payload;
}

function parseOrderListItems(payload: unknown): OrderListItem[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid orders collection items payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid order list item payload.");
    }

    return {
      id: readNumber(item, "id"),
      reference: readNullableString(item, "reference"),
      status: readString(item, "status"),
      createdAt: readString(item, "created_at"),
      updatedAt: readString(item, "updated_at"),
      totalCents: readNumber(item, "total_cents"),
      currency: readString(item, "currency"),
      productsCount: readNumber(item, "products_count"),
    };
  });
}

function parseOrderPagination(payload: unknown): OrderPagination {
  if (!isRecord(payload)) {
    throw new Error("Invalid orders pagination payload.");
  }

  return {
    page: readNumber(payload, "page"),
    perPage: readNumber(payload, "per_page"),
    totalItems: readNumber(payload, "total_items"),
    totalPages: readNumber(payload, "total_pages"),
    hasPreviousPage: readBoolean(payload, "has_previous_page"),
    hasNextPage: readBoolean(payload, "has_next_page"),
  };
}

function parseOrderCollectionFilters(payload: unknown): OrderCollectionFilters {
  if (!isRecord(payload)) {
    throw new Error("Invalid active filters payload.");
  }

  return {
    search: readNullableString(payload, "search"),
    status: readNullableString(payload, "status"),
    updatedAfter: readNullableString(payload, "updated_after"),
    updatedBefore: readNullableString(payload, "updated_before"),
  };
}

function parseOrderCollectionData(payload: unknown): OrderCollectionData {
  if (!isRecord(payload)) {
    throw new Error("Invalid orders collection payload.");
  }

  return {
    items: parseOrderListItems(payload.items),
    pagination: parseOrderPagination(payload.pagination),
    activeFilters: parseOrderCollectionFilters(payload.active_filters),
  };
}

function parseOrderCustomerSummary(
  payload: unknown,
): OrderCustomerSummary | null {
  if (payload === null) {
    return null;
  }

  if (!isRecord(payload)) {
    throw new Error("Invalid order customer payload.");
  }

  return {
    id: readNumber(payload, "id"),
    name: readString(payload, "name"),
    phone: readString(payload, "phone"),
    publicId: readNullableString(payload, "public_id"),
  };
}

function parseOrderProductItems(payload: unknown): OrderProductItem[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid order products payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid order product payload.");
    }

    return {
      productId: readNullableNumber(item, "product_id"),
      name: readString(item, "name"),
      sku: readNullableString(item, "sku"),
      quantity: readNumber(item, "quantity"),
      approvedQuantity: readNumber(item, "approved_quantity"),
      priceCents: readNumber(item, "price_cents"),
      currency: readString(item, "currency"),
      defaultPriceCents: readNullableNumber(item, "default_price_cents"),
    };
  });
}

function parseOrderDetailsData(payload: unknown): OrderDetailsData {
  if (!isRecord(payload)) {
    throw new Error("Invalid order details payload.");
  }

  return {
    id: readNumber(payload, "id"),
    customerId: readNullableNumber(payload, "customer_id"),
    reference: readNullableString(payload, "reference"),
    status: readString(payload, "status"),
    createdAt: readString(payload, "created_at"),
    updatedAt: readString(payload, "updated_at"),
    totalCents: readNumber(payload, "total_cents"),
    currency: readString(payload, "currency"),
    notes: readNullableString(payload, "notes"),
    shippingAddress: readNullableString(payload, "shipping_address"),
    consignee: readNullableString(payload, "consignee"),
    deliveryNotes: readNullableString(payload, "delivery_notes"),
    payer: readNullableString(payload, "payer"),
    customer: parseOrderCustomerSummary(payload.customer),
    crmServiceUrl: readString(payload, "crm_service_url"),
    products: parseOrderProductItems(payload.products),
  };
}

function parseOrderMutationSuccess(payload: unknown): OrderMutationSuccess {
  if (!isRecord(payload)) {
    throw new Error("Invalid order mutation payload.");
  }

  return {
    message: readString(payload, "message"),
    order: parseOrderDetailsData(payload.order),
  };
}

function parseProductNamedOption(payload: unknown): ProductNamedOption {
  if (!isRecord(payload)) {
    throw new Error("Invalid product option payload.");
  }

  return {
    id: readNumber(payload, "id"),
    name: readString(payload, "name"),
  };
}

function parseProductNamedOptions(payload: unknown): ProductNamedOption[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid product options payload.");
  }

  return payload.map(parseProductNamedOption);
}

function parseProductPriceLevelRate(payload: unknown): ProductPriceLevelRate {
  if (!isRecord(payload)) {
    throw new Error("Invalid product price level payload.");
  }

  return {
    priceLevelId: readNumber(payload, "price_level_id"),
    priceLevelName: readString(payload, "price_level_name"),
    priceCents: readNumber(payload, "price_cents"),
  };
}

function parseProductPriceLevelRates(
  payload: unknown,
): ProductPriceLevelRate[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid product price levels payload.");
  }

  return payload.map(parseProductPriceLevelRate);
}

function parseProductEditorOptions(payload: unknown): ProductEditorOptions {
  if (!isRecord(payload)) {
    throw new Error("Invalid product editor options payload.");
  }

  return {
    categories: parseProductNamedOptions(payload.categories),
    tags: parseProductNamedOptions(payload.tags),
    priceLevels: parseProductNamedOptions(payload.price_levels),
    vendors: parseProductNamedOptions(payload.vendors),
  };
}

function parseProductListItem(payload: unknown): ProductListItem {
  if (!isRecord(payload)) {
    throw new Error("Invalid product list item payload.");
  }

  return {
    id: readNumber(payload, "id"),
    name: readString(payload, "name"),
    sku: readNullableString(payload, "sku"),
    descriptionHtml: readNullableString(payload, "description_html"),
    units: readNullableString(payload, "units"),
    amount: readNullableString(payload, "amount"),
    currency: readString(payload, "currency"),
    isArchived: readBoolean(payload, "is_archived"),
    category:
      payload.category === null
        ? null
        : parseProductNamedOption(payload.category),
    vendor:
      payload.vendor === null ? null : parseProductNamedOption(payload.vendor),
    updatedAt: readString(payload, "updated_at"),
    imageUrls: readStringArray(payload, "image_urls"),
    tags: parseProductNamedOptions(payload.tags),
    priceLevels: parseProductPriceLevelRates(payload.price_levels),
  };
}

function parseProductListItems(payload: unknown): ProductListItem[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid product list items payload.");
  }

  return payload.map(parseProductListItem);
}

function parseProductPagination(payload: unknown): ProductPagination {
  if (!isRecord(payload)) {
    throw new Error("Invalid product pagination payload.");
  }

  return {
    page: readNumber(payload, "page"),
    perPage: readNumber(payload, "per_page"),
    totalItems: readNumber(payload, "total_items"),
    totalPages: readNumber(payload, "total_pages"),
    hasPreviousPage: readBoolean(payload, "has_previous_page"),
    hasNextPage: readBoolean(payload, "has_next_page"),
  };
}

function parseProductCollectionFilters(
  payload: unknown,
): ProductCollectionFilters {
  if (!isRecord(payload)) {
    throw new Error("Invalid product filters payload.");
  }

  return {
    search: readNullableString(payload, "search"),
    showArchived: readBoolean(payload, "show_archived"),
  };
}

function parseProductCollectionData(payload: unknown): ProductCollectionData {
  if (!isRecord(payload)) {
    throw new Error("Invalid products collection payload.");
  }

  return {
    items: parseProductListItems(payload.items),
    pagination: parseProductPagination(payload.pagination),
    activeFilters: parseProductCollectionFilters(payload.active_filters),
    editorOptions: parseProductEditorOptions(payload.editor_options),
    filesServiceUrl: readString(payload, "files_service_url"),
  };
}

function parseProductDetailsData(payload: unknown): ProductDetailsData {
  if (!isRecord(payload)) {
    throw new Error("Invalid product details payload.");
  }

  return {
    id: readNumber(payload, "id"),
    name: readString(payload, "name"),
    sku: readNullableString(payload, "sku"),
    descriptionHtml: readNullableString(payload, "description_html"),
    units: readNullableString(payload, "units"),
    amount: readNullableString(payload, "amount"),
    currency: readString(payload, "currency"),
    isArchived: readBoolean(payload, "is_archived"),
    categoryId: readNullableNumber(payload, "category_id"),
    vendorId: readNullableNumber(payload, "vendor_id"),
    tagIds: ((value) => {
      if (
        !Array.isArray(value) ||
        value.some((item) => typeof item !== "number")
      ) {
        throw new Error("Invalid API response: expected number[] at tag_ids.");
      }

      return value;
    })(payload.tag_ids),
    imageUrls: readStringArray(payload, "image_urls"),
    priceLevels: parseProductPriceLevelRates(payload.price_levels),
    updatedAt: readString(payload, "updated_at"),
    editorOptions: parseProductEditorOptions(payload.editor_options),
    filesServiceUrl: readString(payload, "files_service_url"),
  };
}

function parseProductMutationSuccess(payload: unknown): ProductMutationSuccess {
  if (!isRecord(payload)) {
    throw new Error("Invalid product mutation payload.");
  }

  return {
    message: readString(payload, "message"),
    product: parseProductDetailsData(payload.product),
  };
}

function parseProductUploadSuccess(payload: unknown): ProductUploadSuccess {
  if (!isRecord(payload)) {
    throw new Error("Invalid product upload payload.");
  }

  return {
    message: readString(payload, "message"),
    createdCount: readNumber(payload, "created_count"),
  };
}

function parseCategoryTreeNode(payload: unknown): CategoryTreeNode {
  if (!isRecord(payload)) {
    throw new Error("Invalid category tree node payload.");
  }

  return {
    id: readNumber(payload, "id"),
    parentId: readNullableNumber(payload, "parent_id"),
    name: readString(payload, "name"),
    description: readNullableString(payload, "description"),
    isArchived: readBoolean(payload, "is_archived"),
    imageUrl: readNullableString(payload, "image_url"),
    updatedAt: readString(payload, "updated_at"),
    children: parseCategoryTreeNodes(payload.children),
  };
}

function parseCategoryTreeNodes(payload: unknown): CategoryTreeNode[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid categories collection payload.");
  }

  return payload.map(parseCategoryTreeNode);
}

function parseCategoryCollectionData(payload: unknown): CategoryCollectionData {
  if (!isRecord(payload)) {
    throw new Error("Invalid category collection payload.");
  }

  return { items: parseCategoryTreeNodes(payload.items) };
}

function parseCategoryDetailsData(payload: unknown): CategoryDetailsData {
  if (!isRecord(payload)) {
    throw new Error("Invalid category details payload.");
  }

  return {
    id: readNumber(payload, "id"),
    parentId: readNullableNumber(payload, "parent_id"),
    name: readString(payload, "name"),
    description: readNullableString(payload, "description"),
    isArchived: readBoolean(payload, "is_archived"),
    imageUrl: readNullableString(payload, "image_url"),
    updatedAt: readString(payload, "updated_at"),
  };
}

function parseCategoryMutationSuccess(
  payload: unknown,
): CategoryMutationSuccess {
  if (!isRecord(payload)) {
    throw new Error("Invalid category mutation payload.");
  }

  return {
    message: readString(payload, "message"),
    category: parseCategoryDetailsData(payload.category),
  };
}

function parseTagListItem(payload: unknown): TagListItem {
  if (!isRecord(payload)) {
    throw new Error("Invalid tag list item payload.");
  }

  return {
    id: readNumber(payload, "id"),
    name: readString(payload, "name"),
    createdAt: readString(payload, "created_at"),
    updatedAt: readString(payload, "updated_at"),
  };
}

function parseTagCollectionData(payload: unknown): TagCollectionData {
  if (!isRecord(payload)) {
    throw new Error("Invalid tag collection payload.");
  }

  return {
    items: Array.isArray(payload.items)
      ? payload.items.map(parseTagListItem)
      : [],
    pagination: parseProductPagination(payload.pagination),
    activeFilters: parseOrderCollectionFilters(payload.active_filters),
  };
}

function parseTagDetailsData(payload: unknown): TagDetailsData {
  if (!isRecord(payload)) {
    throw new Error("Invalid tag details payload.");
  }

  return {
    id: readNumber(payload, "id"),
    name: readString(payload, "name"),
    updatedAt: readString(payload, "updated_at"),
  };
}

function parseTagMutationSuccess(payload: unknown): TagMutationSuccess {
  if (!isRecord(payload)) {
    throw new Error("Invalid tag mutation payload.");
  }

  return {
    message: readString(payload, "message"),
    tag: parseTagDetailsData(payload.tag),
  };
}

function parsePriceLevelListItem(payload: unknown) {
  if (!isRecord(payload)) {
    throw new Error("Invalid price level list item payload.");
  }

  return {
    id: readNumber(payload, "id"),
    name: readString(payload, "name"),
    isDefault: readBoolean(payload, "is_default"),
    createdAt: readString(payload, "created_at"),
    updatedAt: readString(payload, "updated_at"),
  };
}

function parsePriceLevelEditorOptions(
  payload: unknown,
): PriceLevelEditorOptions {
  if (!isRecord(payload)) {
    throw new Error("Invalid price level editor options payload.");
  }

  return {
    basePriceLevels: parseProductNamedOptions(payload.base_price_levels),
    categories: parseProductNamedOptions(payload.categories),
  };
}

function parsePriceLevelCollectionData(
  payload: unknown,
): PriceLevelCollectionData {
  if (!isRecord(payload)) {
    throw new Error("Invalid price level collection payload.");
  }

  return {
    items: Array.isArray(payload.items)
      ? payload.items.map(parsePriceLevelListItem)
      : [],
    activeFilters: parseOrderCollectionFilters(payload.active_filters),
    editorOptions: parsePriceLevelEditorOptions(payload.editor_options),
    crmServiceUrl: readString(payload, "crm_service_url"),
  };
}

function parsePriceLevelDetailsData(payload: unknown): PriceLevelDetailsData {
  if (!isRecord(payload)) {
    throw new Error("Invalid price level details payload.");
  }

  return {
    id: readNumber(payload, "id"),
    name: readString(payload, "name"),
    isDefault: readBoolean(payload, "is_default"),
    updatedAt: readString(payload, "updated_at"),
  };
}

function parsePriceLevelMutationSuccess(
  payload: unknown,
): PriceLevelMutationSuccess {
  if (!isRecord(payload)) {
    throw new Error("Invalid price level mutation payload.");
  }

  return {
    message: readString(payload, "message"),
    priceLevel: parsePriceLevelDetailsData(payload.price_level),
  };
}

function parseClientPriceLevelAssignments(
  payload: unknown,
): ClientPriceLevelAssignments {
  if (!isRecord(payload)) {
    throw new Error("Invalid client price level assignments payload.");
  }

  const assignments = payload.assignments;
  if (!Array.isArray(assignments)) {
    throw new Error("Invalid client price level assignments list.");
  }

  return {
    hubId: readNumber(payload, "hub_id"),
    defaultPriceLevelId: readNullableNumber(payload, "default_price_level_id"),
    assignments: assignments.map((item) => {
      if (!isRecord(item)) {
        throw new Error("Invalid client price level assignment payload.");
      }

      return {
        phone: readString(item, "phone"),
        priceLevelId: readNullableNumber(item, "price_level_id"),
      };
    }),
  };
}

function parseCrmClientListItem(payload: unknown): CrmClientListItem {
  if (!isRecord(payload)) {
    throw new Error("Invalid CRM client payload.");
  }

  return {
    id: readNumber(payload, "id"),
    publicId: readNullableString(payload, "public_id"),
    name: readString(payload, "name"),
    email: readNullableString(payload, "email"),
    phone: readNullableString(payload, "phone"),
  };
}

function parseVendorListItem(payload: unknown): VendorListItem {
  if (!isRecord(payload)) {
    throw new Error("Invalid vendor list item payload.");
  }

  return {
    id: readNumber(payload, "id"),
    name: readString(payload, "name"),
    createdAt: readString(payload, "created_at"),
    updatedAt: readString(payload, "updated_at"),
  };
}

function parseVendorCollectionData(payload: unknown): VendorCollectionData {
  if (!isRecord(payload)) {
    throw new Error("Invalid vendor collection payload.");
  }

  return {
    items: Array.isArray(payload.items)
      ? payload.items.map(parseVendorListItem)
      : [],
    pagination: parseOrderPagination(payload.pagination),
    activeFilters: parseOrderCollectionFilters(payload.active_filters),
  };
}

function parseVendorDetailsData(payload: unknown): VendorDetailsData {
  if (!isRecord(payload)) {
    throw new Error("Invalid vendor details payload.");
  }

  return {
    id: readNumber(payload, "id"),
    name: readString(payload, "name"),
    createdAt: readString(payload, "created_at"),
    updatedAt: readString(payload, "updated_at"),
  };
}

function parseVendorMutationSuccess(payload: unknown): VendorMutationSuccess {
  if (!isRecord(payload)) {
    throw new Error("Invalid vendor mutation payload.");
  }

  return {
    message: readString(payload, "message"),
    vendor: parseVendorDetailsData(payload.vendor),
  };
}

function parseLocalUserListItem(payload: unknown): LocalUserListItem {
  if (!isRecord(payload)) {
    throw new Error("Invalid local user payload.");
  }

  return {
    userId: readNumber(payload, "user_id"),
    name: readString(payload, "name"),
    email: readString(payload, "email"),
    vendorId: readNullableNumber(payload, "vendor_id"),
    vendorName: readNullableString(payload, "vendor_name"),
  };
}

function parseLocalUserCollectionData(
  payload: unknown,
): LocalUserCollectionData {
  if (!isRecord(payload)) {
    throw new Error("Invalid local users payload.");
  }

  const items = payload.items;
  if (!Array.isArray(items)) {
    throw new Error("Invalid local users items payload.");
  }

  return {
    items: items.map(parseLocalUserListItem),
  };
}

function parseAuthUsers(payload: unknown): AuthUserSearchItem[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid auth users payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid auth user payload.");
    }

    return {
      sub: readString(item, "sub"),
      name: readString(item, "name"),
      email: readString(item, "email"),
    };
  });
}

function withBaseUrl(baseUrl: string, path: string) {
  return new URL(path, baseUrl).toString();
}

export function toFieldErrorMap(
  error: ApiMutationError,
): Record<string, string> {
  return Object.fromEntries(
    error.field_errors.map((fieldError) => [
      fieldError.field,
      fieldError.message,
    ]),
  );
}

export function isApiMutationError(error: unknown): error is ApiMutationError {
  if (!isRecord(error)) {
    return false;
  }

  return (
    typeof error.message === "string" &&
    Array.isArray(error.field_errors) &&
    error.field_errors.every((fieldError) => {
      return (
        isRecord(fieldError) &&
        typeof fieldError.field === "string" &&
        typeof fieldError.message === "string"
      );
    })
  );
}

async function fetchJson(
  url: string,
  options?: {
    notFoundMessage?: string;
  },
) {
  return fetchSharedJson(url, {
    unauthorizedMessage: "Недостаточно прав для доступа к Orders.",
    notFoundMessage: options?.notFoundMessage,
  });
}

async function putJson(
  endpoint: string,
  body: unknown,
): Promise<OrderMutationSuccess> {
  const response = await fetch(endpoint, {
    method: "PUT",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    credentials: "include",
    body: JSON.stringify(body),
  });

  const payload = await readMutationPayload(response, endpoint);
  return parseOrderMutationSuccess(payload);
}

async function postJson<T>(
  endpoint: string,
  body: unknown,
  parseSuccess: (payload: unknown) => T,
): Promise<T> {
  const response = await fetch(endpoint, {
    method: "POST",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    credentials: "include",
    body: JSON.stringify(body),
  });

  const payload = await readMutationPayload(response, endpoint);
  return parseSuccess(payload);
}

async function putJsonWithParser<T>(
  endpoint: string,
  body: unknown,
  parseSuccess: (payload: unknown) => T,
): Promise<T> {
  const response = await fetch(endpoint, {
    method: "PUT",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    credentials: "include",
    body: JSON.stringify(body),
  });

  const payload = await readMutationPayload(response, endpoint);
  return parseSuccess(payload);
}

async function postFormData<T>(
  endpoint: string,
  body: FormData,
  parseSuccess: (payload: unknown) => T,
): Promise<T> {
  const response = await fetch(endpoint, {
    method: "POST",
    headers: {
      Accept: "application/json",
    },
    credentials: "include",
    body,
  });

  const payload = await readMutationPayload(response, endpoint);
  return parseSuccess(payload);
}

async function deleteJson(endpoint: string): Promise<{ message: string }> {
  const response = await fetch(endpoint, {
    method: "DELETE",
    headers: {
      Accept: "application/json",
    },
    credentials: "include",
  });

  const payload = await readMutationPayload(response, endpoint);
  if (!isRecord(payload)) {
    throw new Error("Invalid delete mutation payload.");
  }

  return { message: readString(payload, "message") };
}

export async function fetchShellData(): Promise<ShellData> {
  return fetchSharedShellData<ShellData>(
    "/api/v1/iam",
    "Недостаточно прав для доступа к Orders.",
  );
}

export async function fetchNoAccessData(): Promise<NoAccessData> {
  const payload = await fetchSharedNoAccessData<NoAccessData>(
    "/api/v1/no-access",
    "Недостаточно прав для доступа к Orders.",
  );
  return {
    ...payload,
    requiredRole: payload.requiredRole ?? "",
  };
}

export async function fetchOrdersCollection(params?: {
  search?: string | null;
  status?: string | null;
  updatedAfter?: string | null;
  updatedBefore?: string | null;
  page?: number;
}): Promise<OrderCollectionData> {
  const searchParams = new URLSearchParams();

  if (params?.search) {
    searchParams.set("search", params.search);
  }

  if (params?.status) {
    searchParams.set("status", params.status);
  }

  if (params?.updatedAfter) {
    searchParams.set("updated_after", params.updatedAfter);
  }

  if (params?.updatedBefore) {
    searchParams.set("updated_before", params.updatedBefore);
  }

  if (params?.page && params.page > 1) {
    searchParams.set("page", String(params.page));
  }

  const queryString = searchParams.toString();
  const endpoint = queryString
    ? `/api/v1/orders?${queryString}`
    : "/api/v1/orders";
  const payload = await fetchJson(endpoint);
  return parseOrderCollectionData(payload);
}

export async function fetchOrderDetails(
  orderId: number,
): Promise<OrderDetailsData> {
  const payload = await fetchJson(`/api/v1/orders/${orderId}`, {
    notFoundMessage: "Заказ не найден.",
  });
  return parseOrderDetailsData(payload);
}

export async function updateOrder(
  orderId: number,
  input: OrderUpdateInput,
): Promise<OrderMutationSuccess> {
  return putJson(`/api/v1/orders/${orderId}`, {
    order_id: input.orderId,
    status: input.status,
    reference: input.reference,
    notes: input.notes,
    shipping_address: input.shippingAddress,
    consignee: input.consignee,
    delivery_notes: input.deliveryNotes,
    payer: input.payer,
  });
}

export async function updateOrderProductApprovals(
  orderId: number,
  input: OrderApprovalUpdateInput,
): Promise<OrderMutationSuccess> {
  return putJson(`/api/v1/orders/${orderId}/products/approvals`, {
    approvals: input.approvals.map((approval) => ({
      product_id: approval.productId,
      approved_quantity: approval.approvedQuantity,
    })),
  });
}

export async function fetchProductsCollection(params?: {
  search?: string | null;
  page?: number;
  showArchived?: boolean;
}): Promise<ProductCollectionData> {
  const searchParams = new URLSearchParams();

  if (params?.search) {
    searchParams.set("search", params.search);
  }

  if (params?.page && params.page > 1) {
    searchParams.set("page", String(params.page));
  }

  if (params?.showArchived) {
    searchParams.set("show_archived", "true");
  }

  const queryString = searchParams.toString();
  const endpoint = queryString
    ? `/api/v1/products?${queryString}`
    : "/api/v1/products";
  const payload = await fetchJson(endpoint);
  return parseProductCollectionData(payload);
}

export async function fetchProductDetails(
  productId: number,
): Promise<ProductDetailsData> {
  const payload = await fetchJson(`/api/v1/products/${productId}`, {
    notFoundMessage: "Товар не найден.",
  });
  return parseProductDetailsData(payload);
}

export async function createProduct(
  input: ProductMutationInput,
): Promise<ProductMutationSuccess> {
  return postJson(
    "/api/v1/products",
    {
      name: input.name,
      sku: input.sku,
      description: input.descriptionHtml,
      units: input.units,
      currency: input.currency,
      category_id: input.categoryId,
      vendor_id: input.vendorId,
      tag_ids: input.tagIds,
      image_urls: input.imageUrls.join("\n"),
      price_levels: input.priceLevels.map((level) => ({
        price_level_id: level.priceLevelId,
        price: level.price,
      })),
      amount: input.amount,
    },
    parseProductMutationSuccess,
  );
}

export async function updateProduct(
  productId: number,
  input: ProductMutationInput,
): Promise<ProductMutationSuccess> {
  return putJsonWithParser(
    `/api/v1/products/${productId}`,
    {
      product_id: input.productId ?? productId,
      name: input.name,
      sku: input.sku,
      description: input.descriptionHtml,
      units: input.units,
      currency: input.currency,
      image_urls: input.imageUrls.join("\n"),
      is_archived: input.isArchived,
      category_id: input.categoryId,
      vendor_id: input.vendorId,
      tag_ids: input.tagIds,
      price_levels: input.priceLevels.map((level) => ({
        price_level_id: level.priceLevelId,
        price: level.price.length > 0 ? level.price : null,
      })),
      amount: input.amount,
    },
    parseProductMutationSuccess,
  );
}

export async function uploadProducts(
  file: File,
): Promise<ProductUploadSuccess> {
  const formData = new FormData();
  formData.set("csv", file);

  return postFormData(
    "/api/v1/products/upload",
    formData,
    parseProductUploadSuccess,
  );
}

export async function fetchCategoriesCollection(): Promise<CategoryCollectionData> {
  const payload = await fetchJson("/api/v1/categories");
  return parseCategoryCollectionData(payload);
}

export async function fetchCategoryDetails(
  categoryId: number,
): Promise<CategoryDetailsData> {
  const payload = await fetchJson(`/api/v1/categories/${categoryId}`, {
    notFoundMessage: "Категория не найдена.",
  });
  return parseCategoryDetailsData(payload);
}

export async function createCategory(
  input: CategoryMutationInput,
): Promise<CategoryMutationSuccess> {
  return postJson(
    "/api/v1/categories",
    {
      name: input.name,
      description: input.description,
      parent_id: input.parentId,
      image_url: input.imageUrl,
    },
    parseCategoryMutationSuccess,
  );
}

export async function updateCategory(
  categoryId: number,
  input: CategoryMutationInput,
): Promise<CategoryMutationSuccess> {
  return putJsonWithParser(
    `/api/v1/categories/${categoryId}`,
    {
      name: input.name,
      description: input.description,
      image_url: input.imageUrl,
      is_archived: input.isArchived ?? false,
    },
    parseCategoryMutationSuccess,
  );
}

export async function deleteCategory(
  categoryId: number,
): Promise<{ message: string }> {
  return deleteJson(`/api/v1/categories/${categoryId}`);
}

export async function fetchTagsCollection(params?: {
  search?: string | null;
  page?: number;
}): Promise<TagCollectionData> {
  const searchParams = new URLSearchParams();
  if (params?.search) {
    searchParams.set("search", params.search);
  }
  if (params?.page && params.page > 1) {
    searchParams.set("page", String(params.page));
  }
  const queryString = searchParams.toString();
  const payload = await fetchJson(
    queryString ? `/api/v1/tags?${queryString}` : "/api/v1/tags",
  );
  return parseTagCollectionData(payload);
}

export async function fetchTagDetails(tagId: number): Promise<TagDetailsData> {
  const payload = await fetchJson(`/api/v1/tags/${tagId}`, {
    notFoundMessage: "Тег не найден.",
  });
  return parseTagDetailsData(payload);
}

export async function createTag(
  input: TagMutationInput,
): Promise<TagMutationSuccess> {
  return postJson(
    "/api/v1/tags",
    { name: input.name },
    parseTagMutationSuccess,
  );
}

export async function updateTag(
  tagId: number,
  input: TagMutationInput,
): Promise<TagMutationSuccess> {
  return putJsonWithParser(
    `/api/v1/tags/${tagId}`,
    { tag_id: input.tagId ?? tagId, name: input.name },
    parseTagMutationSuccess,
  );
}

export async function deleteTag(tagId: number): Promise<{ message: string }> {
  return deleteJson(`/api/v1/tags/${tagId}`);
}

export async function fetchPriceLevelsCollection(params?: {
  search?: string | null;
}): Promise<PriceLevelCollectionData> {
  const searchParams = new URLSearchParams();
  if (params?.search) {
    searchParams.set("search", params.search);
  }
  const queryString = searchParams.toString();
  const payload = await fetchJson(
    queryString
      ? `/api/v1/price-levels?${queryString}`
      : "/api/v1/price-levels",
  );
  return parsePriceLevelCollectionData(payload);
}

export async function fetchPriceLevelDetails(
  priceLevelId: number,
): Promise<PriceLevelDetailsData> {
  const payload = await fetchJson(`/api/v1/price-levels/${priceLevelId}`, {
    notFoundMessage: "Уровень цен не найден.",
  });
  return parsePriceLevelDetailsData(payload);
}

export async function createPriceLevel(
  input: PriceLevelMutationInput,
): Promise<PriceLevelMutationSuccess> {
  return postJson(
    "/api/v1/price-levels",
    {
      name: input.name,
      default: input.default,
      base_price_level_id: input.basePriceLevelId,
      price_modifier: input.priceModifier,
      price_modifier_kind: input.priceModifierKind,
      excluded_category_ids: input.excludedCategoryIds,
      excluded_product_ids: input.excludedProductIds,
      included_product_ids: input.includedProductIds,
    },
    parsePriceLevelMutationSuccess,
  );
}

export async function updatePriceLevel(
  priceLevelId: number,
  input: PriceLevelUpdateInput,
): Promise<PriceLevelMutationSuccess> {
  return putJsonWithParser(
    `/api/v1/price-levels/${priceLevelId}`,
    {
      name: input.name,
      default: input.default,
    },
    parsePriceLevelMutationSuccess,
  );
}

export async function deletePriceLevel(
  priceLevelId: number,
): Promise<{ message: string }> {
  return deleteJson(`/api/v1/price-levels/${priceLevelId}`);
}

export async function fetchClientPriceLevelAssignments(): Promise<ClientPriceLevelAssignments> {
  const payload = await fetchJson("/api/v1/client-price-levels");
  return parseClientPriceLevelAssignments(payload);
}

export async function updateClientPriceLevel(
  input: ClientPriceLevelAssignmentInput,
): Promise<{ message: string }> {
  return putJsonWithParser(
    "/api/v1/client-price-levels",
    {
      name: input.name,
      phone: input.phone,
      public_id: input.publicId,
      price_level_id: input.priceLevelId,
    },
    (payload) => {
      if (!isRecord(payload)) {
        throw new Error("Invalid client price level mutation payload.");
      }

      return { message: readString(payload, "message") };
    },
  );
}

export async function fetchCrmClients(
  crmServiceUrl: string,
): Promise<CrmClientListItem[]> {
  const payload = await fetchJson(
    withBaseUrl(crmServiceUrl, "/api/v1/clients"),
  );
  if (!Array.isArray(payload)) {
    throw new Error("Invalid CRM clients payload.");
  }

  return payload.map(parseCrmClientListItem);
}

export async function fetchVendorsCollection(params?: {
  search?: string | null;
  page?: number;
}): Promise<VendorCollectionData> {
  const searchParams = new URLSearchParams();

  if (params?.search) {
    searchParams.set("search", params.search);
  }

  if (params?.page && params.page > 1) {
    searchParams.set("page", String(params.page));
  }

  const queryString = searchParams.toString();
  const payload = await fetchJson(
    queryString ? `/api/v1/vendors?${queryString}` : "/api/v1/vendors",
  );
  return parseVendorCollectionData(payload);
}

export async function fetchVendorDetails(
  vendorId: number,
): Promise<VendorDetailsData> {
  const payload = await fetchJson(`/api/v1/vendors/${vendorId}`, {
    notFoundMessage: "Поставщик не найден.",
  });
  return parseVendorDetailsData(payload);
}

export async function createVendor(
  input: VendorMutationInput,
): Promise<VendorMutationSuccess> {
  return postJson(
    "/api/v1/vendors",
    { name: input.name },
    parseVendorMutationSuccess,
  );
}

export async function updateVendor(
  vendorId: number,
  input: VendorMutationInput,
): Promise<VendorMutationSuccess> {
  return putJsonWithParser(
    `/api/v1/vendors/${vendorId}`,
    { vendor_id: input.vendorId ?? vendorId, name: input.name },
    parseVendorMutationSuccess,
  );
}

export async function deleteVendor(
  vendorId: number,
): Promise<{ message: string }> {
  return deleteJson(`/api/v1/vendors/${vendorId}`);
}

export async function fetchLocalUsers(): Promise<LocalUserCollectionData> {
  const payload = await fetchJson("/api/v1/users");
  return parseLocalUserCollectionData(payload);
}

export async function createLocalUser(
  input: AddLocalUserInput,
): Promise<{ message: string }> {
  return postJson(
    "/api/v1/users",
    { name: input.name, email: input.email },
    (payload) => {
      if (!isRecord(payload)) {
        throw new Error("Invalid local user mutation payload.");
      }

      return { message: readString(payload, "message") };
    },
  );
}

export async function assignVendorUser(
  input: AssignVendorUserInput,
): Promise<{ message: string }> {
  return postJson(
    "/api/v1/vendors/assignments",
    { user_id: input.userId, vendor_id: input.vendorId },
    (payload) => {
      if (!isRecord(payload)) {
        throw new Error("Invalid vendor assignment mutation payload.");
      }

      return { message: readString(payload, "message") };
    },
  );
}

export async function clearVendorUser(
  userId: number,
): Promise<{ message: string }> {
  return deleteJson(`/api/v1/vendors/assignments/${userId}`);
}

export async function fetchAuthVendorUsers(
  authBaseUrl: string,
  query: string,
): Promise<AuthUserSearchItem[]> {
  const payload = await fetchJson(
    withBaseUrl(
      authBaseUrl,
      `/api/v1/users?page=1&role=orders_vendor&query=${encodeURIComponent(query)}`,
    ),
  );

  return parseAuthUsers(payload).slice(0, 20);
}

export async function fetchHubMenuItems(
  authBaseUrl: string,
  hubId: number,
): Promise<UserMenuItem[]> {
  return fetchSharedHubMenuItems<UserMenuItem>(
    withBaseUrl(authBaseUrl, `/api/v1/hubs/${hubId}/menu-items`),
    "Недостаточно прав для доступа к Orders.",
  );
}
