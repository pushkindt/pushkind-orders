export type NavigationItem = {
  name: string;
  url: string;
};

export type ApiFieldError = {
  field: string;
  message: string;
};

export type ApiMutationError = {
  message: string;
  field_errors: ApiFieldError[];
};

export type UserMenuItem = {
  name: string;
  url: string;
  iconClass?: string;
};

export type CurrentUser = {
  email: string;
  name: string;
  hubId: number;
  roles: string[];
};

export type ShellData = {
  currentUser: CurrentUser;
  homeUrl: string;
  navigation: NavigationItem[];
  localMenuItems: UserMenuItem[];
};

export type NoAccessData = {
  currentUser: CurrentUser;
  homeUrl: string;
  requiredRole: string;
};

export type OrderListItem = {
  id: number;
  reference: string | null;
  status: string;
  createdAt: string;
  updatedAt: string;
  totalCents: number;
  currency: string;
  productsCount: number;
};

export type OrderPagination = {
  page: number;
  perPage: number;
  totalItems: number;
  totalPages: number;
  hasPreviousPage: boolean;
  hasNextPage: boolean;
};

export type OrderCollectionFilters = {
  search: string | null;
};

export type OrderCollectionData = {
  items: OrderListItem[];
  pagination: OrderPagination;
  activeFilters: OrderCollectionFilters;
};

export type OrderCustomerSummary = {
  id: number;
  name: string;
  phone: string;
  publicId: string | null;
};

export type OrderProductItem = {
  productId: number | null;
  name: string;
  sku: string | null;
  quantity: number;
  approvedQuantity: number;
  priceCents: number;
  currency: string;
  defaultPriceCents: number | null;
};

export type OrderDetailsData = {
  id: number;
  customerId: number | null;
  reference: string | null;
  status: string;
  createdAt: string;
  updatedAt: string;
  totalCents: number;
  currency: string;
  notes: string | null;
  shippingAddress: string | null;
  consignee: string | null;
  deliveryNotes: string | null;
  payer: string | null;
  customer: OrderCustomerSummary | null;
  crmServiceUrl: string;
  products: OrderProductItem[];
};

export type OrderUpdateInput = {
  orderId: number;
  status: string;
  reference: string | null;
  notes: string | null;
  shippingAddress: string | null;
  consignee: string | null;
  deliveryNotes: string | null;
  payer: string | null;
};

export type OrderApprovalUpdateItemInput = {
  productId: number;
  approvedQuantity: number;
};

export type OrderApprovalUpdateInput = {
  approvals: OrderApprovalUpdateItemInput[];
};

export type OrderMutationSuccess = {
  message: string;
  order: OrderDetailsData;
};

export type ProductNamedOption = {
  id: number;
  name: string;
};

export type ProductPriceLevelRate = {
  priceLevelId: number;
  priceLevelName: string;
  priceCents: number;
};

export type ProductEditorOptions = {
  categories: ProductNamedOption[];
  tags: ProductNamedOption[];
  priceLevels: ProductNamedOption[];
  vendors: ProductNamedOption[];
};

export type ProductListItem = {
  id: number;
  name: string;
  sku: string | null;
  descriptionHtml: string | null;
  units: string | null;
  amount: string | null;
  currency: string;
  isArchived: boolean;
  category: ProductNamedOption | null;
  vendor: ProductNamedOption | null;
  updatedAt: string;
  imageUrls: string[];
  tags: ProductNamedOption[];
  priceLevels: ProductPriceLevelRate[];
};

export type ProductPagination = {
  page: number;
  perPage: number;
  totalItems: number;
  totalPages: number;
  hasPreviousPage: boolean;
  hasNextPage: boolean;
};

export type ProductCollectionFilters = {
  search: string | null;
  showArchived: boolean;
};

export type ProductCollectionData = {
  items: ProductListItem[];
  pagination: ProductPagination;
  activeFilters: ProductCollectionFilters;
  editorOptions: ProductEditorOptions;
};

export type ProductDetailsData = {
  id: number;
  name: string;
  sku: string | null;
  descriptionHtml: string | null;
  units: string | null;
  amount: string | null;
  currency: string;
  isArchived: boolean;
  categoryId: number | null;
  vendorId: number | null;
  tagIds: number[];
  imageUrls: string[];
  priceLevels: ProductPriceLevelRate[];
  updatedAt: string;
  editorOptions: ProductEditorOptions;
};

export type ProductPriceLevelInput = {
  priceLevelId: number;
  price: string;
};

export type ProductMutationInput = {
  productId?: number;
  name: string;
  sku: string | null;
  descriptionHtml: string | null;
  units: string | null;
  amount: number | null;
  currency: string;
  isArchived: boolean;
  categoryId: number | null;
  vendorId: number | null;
  tagIds: number[];
  imageUrls: string[];
  priceLevels: ProductPriceLevelInput[];
};

export type ProductMutationSuccess = {
  message: string;
  product: ProductDetailsData;
};

export type ProductUploadSuccess = {
  message: string;
  createdCount: number;
};

export type CategoryTreeNode = {
  id: number;
  parentId: number | null;
  name: string;
  description: string | null;
  isArchived: boolean;
  imageUrl: string | null;
  updatedAt: string;
  children: CategoryTreeNode[];
};

export type CategoryCollectionData = {
  items: CategoryTreeNode[];
};

export type CategoryDetailsData = {
  id: number;
  parentId: number | null;
  name: string;
  description: string | null;
  isArchived: boolean;
  imageUrl: string | null;
  updatedAt: string;
};

export type CategoryMutationInput = {
  name: string;
  description: string | null;
  parentId: number | null;
  imageUrl: string | null;
  isArchived?: boolean;
};

export type CategoryMutationSuccess = {
  message: string;
  category: CategoryDetailsData;
};

export type TagListItem = {
  id: number;
  name: string;
  updatedAt: string;
};

export type TagCollectionData = {
  items: TagListItem[];
  pagination: ProductPagination;
  activeFilters: OrderCollectionFilters;
};

export type TagDetailsData = {
  id: number;
  name: string;
  updatedAt: string;
};

export type TagMutationInput = {
  tagId?: number;
  name: string;
};

export type TagMutationSuccess = {
  message: string;
  tag: TagDetailsData;
};

export type PriceLevelListItem = {
  id: number;
  name: string;
  isDefault: boolean;
  updatedAt: string;
};

export type PriceLevelEditorOptions = {
  basePriceLevels: ProductNamedOption[];
  categories: ProductNamedOption[];
};

export type PriceLevelCollectionData = {
  items: PriceLevelListItem[];
  activeFilters: OrderCollectionFilters;
  editorOptions: PriceLevelEditorOptions;
  crmServiceUrl: string;
};

export type PriceLevelDetailsData = {
  id: number;
  name: string;
  isDefault: boolean;
  updatedAt: string;
};

export type PriceLevelMutationInput = {
  name: string;
  default: boolean;
  basePriceLevelId: number;
  priceModifier: number;
  priceModifierKind: "percent" | "fixed";
  excludedCategoryIds: number[];
  excludedProductIds: number[];
  includedProductIds: number[];
};

export type VendorListItem = {
  id: number;
  name: string;
  createdAt: string;
  updatedAt: string;
};

export type VendorCollectionData = {
  items: VendorListItem[];
  pagination: ProductPagination;
  activeFilters: OrderCollectionFilters;
};

export type VendorDetailsData = {
  id: number;
  name: string;
  createdAt: string;
  updatedAt: string;
};

export type VendorMutationInput = {
  vendorId?: number;
  name: string;
};

export type VendorMutationSuccess = {
  message: string;
  vendor: VendorDetailsData;
};

export type LocalUserListItem = {
  userId: number;
  name: string;
  email: string;
  vendorId: number | null;
  vendorName: string | null;
};

export type LocalUserCollectionData = {
  items: LocalUserListItem[];
};

export type AddLocalUserInput = {
  name: string;
  email: string;
};

export type AssignVendorUserInput = {
  userId: number;
  vendorId: number;
};

export type AuthUserSearchItem = {
  sub: string;
  name: string;
  email: string;
};

export type PriceLevelUpdateInput = {
  name: string;
  default: boolean;
};

export type PriceLevelMutationSuccess = {
  message: string;
  priceLevel: PriceLevelDetailsData;
};

export type ClientPriceLevelAssignment = {
  phone: string;
  priceLevelId: number | null;
};

export type ClientPriceLevelAssignments = {
  hubId: number;
  defaultPriceLevelId: number | null;
  assignments: ClientPriceLevelAssignment[];
};

export type ClientPriceLevelAssignmentInput = {
  name: string;
  phone: string;
  publicId: string;
  priceLevelId: number | null;
};

export type CrmClientListItem = {
  id: number;
  publicId: string | null;
  name: string;
  email: string | null;
  phone: string | null;
};
