import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  browserLocation,
  createProduct,
  fetchProductDetails,
  fetchProductsCollection,
  fetchOrderDetails,
  fetchOrdersCollection,
  updateOrder,
  updateProduct,
  uploadProducts,
} from "./api";

describe("orders api helpers", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("parses the orders collection payload", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({
          items: [
            {
              id: 101,
              reference: "ORD-101",
              status: "Pending",
              created_at: "2024-01-01 10:30",
              updated_at: "2024-01-02 11:15",
              total_cents: 15500,
              currency: "RUB",
              products_count: 3,
            },
          ],
          pagination: {
            page: 2,
            per_page: 20,
            total_items: 41,
            total_pages: 3,
            has_previous_page: true,
            has_next_page: true,
          },
          active_filters: {
            search: "ord",
          },
        }),
        {
          status: 200,
          headers: { "Content-Type": "application/json" },
        },
      ),
    );

    const result = await fetchOrdersCollection({ search: "ord", page: 2 });

    expect(result.items).toHaveLength(1);
    expect(result.items[0]).toMatchObject({
      id: 101,
      reference: "ORD-101",
      status: "Pending",
      totalCents: 15500,
      productsCount: 3,
    });
    expect(result.pagination).toMatchObject({
      page: 2,
      perPage: 20,
      totalItems: 41,
      totalPages: 3,
      hasPreviousPage: true,
      hasNextPage: true,
    });
    expect(result.activeFilters.search).toBe("ord");
    expect(fetch).toHaveBeenCalledWith("/api/v1/orders?search=ord&page=2", {
      headers: {
        Accept: "application/json",
      },
      cache: "no-store",
      credentials: "include",
    });
  });

  it("rejects malformed orders collection payloads", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({
          items: [
            {
              id: 101,
              reference: "ORD-101",
              status: "Pending",
              created_at: "2024-01-01 10:30",
              updated_at: "2024-01-02 11:15",
              total_cents: 15500,
              currency: "RUB",
              products_count: "3",
            },
          ],
          pagination: {
            page: 1,
            per_page: 20,
            total_items: 1,
            total_pages: 1,
            has_previous_page: false,
            has_next_page: false,
          },
          active_filters: {
            search: null,
          },
        }),
        {
          status: 200,
          headers: { "Content-Type": "application/json" },
        },
      ),
    );

    await expect(fetchOrdersCollection()).rejects.toThrow(
      "Invalid API response: expected number at products_count.",
    );
  });

  it("parses order details payloads", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({
          id: 101,
          customer_id: 7,
          reference: "ORD-101",
          status: "Processing",
          created_at: "2024-01-01 10:30",
          updated_at: "2024-01-02 11:15",
          total_cents: 15500,
          currency: "RUB",
          notes: "Комментарий",
          shipping_address: "Москва",
          consignee: "Иван",
          delivery_notes: "Позвонить заранее",
          payer: "ООО Ромашка",
          crm_service_url: "https://crm.example.com",
          customer: {
            id: 7,
            name: "ООО Ромашка",
            phone: "+79990000000",
            public_id: "customer-public-id",
          },
          products: [
            {
              product_id: 8,
              name: "Яблоки",
              sku: "APL-1",
              quantity: 2,
              approved_quantity: 1,
              price_cents: 5000,
              currency: "RUB",
              default_price_cents: 6000,
            },
          ],
        }),
        {
          status: 200,
          headers: { "Content-Type": "application/json" },
        },
      ),
    );

    const result = await fetchOrderDetails(101);

    expect(result).toMatchObject({
      id: 101,
      customerId: 7,
      reference: "ORD-101",
      status: "Processing",
      totalCents: 15500,
      shippingAddress: "Москва",
      payer: "ООО Ромашка",
      crmServiceUrl: "https://crm.example.com",
    });
    expect(result.customer).toMatchObject({
      id: 7,
      name: "ООО Ромашка",
      publicId: "customer-public-id",
    });
    expect(result.products[0]).toMatchObject({
      productId: 8,
      name: "Яблоки",
      approvedQuantity: 1,
      defaultPriceCents: 6000,
    });
    expect(fetch).toHaveBeenCalledWith("/api/v1/orders/101", {
      headers: {
        Accept: "application/json",
      },
      cache: "no-store",
      credentials: "include",
    });
  });

  it("parses order mutation success payloads", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({
          message: "Заказ обновлён.",
          order: {
            id: 101,
            customer_id: 7,
            reference: "ORD-101",
            status: "Processing",
            created_at: "2024-01-01 10:30",
            updated_at: "2024-01-02 11:15",
            total_cents: 15500,
            currency: "RUB",
            notes: "Комментарий",
            shipping_address: "Москва",
            consignee: "Иван",
            delivery_notes: "Позвонить заранее",
            payer: "ООО Ромашка",
            crm_service_url: "https://crm.example.com",
            customer: {
              id: 7,
              name: "ООО Ромашка",
              phone: "+79990000000",
              public_id: "customer-public-id",
            },
            products: [
              {
                product_id: 8,
                name: "Яблоки",
                sku: "APL-1",
                quantity: 2,
                approved_quantity: 1,
                price_cents: 5000,
                currency: "RUB",
                default_price_cents: 6000,
              },
            ],
          },
        }),
        {
          status: 200,
          headers: { "Content-Type": "application/json" },
        },
      ),
    );

    const result = await updateOrder(101, {
      orderId: 101,
      status: "Processing",
      reference: "ORD-101",
      notes: "Комментарий",
      shippingAddress: "Москва",
      consignee: "Иван",
      deliveryNotes: "Позвонить заранее",
      payer: "ООО Ромашка",
    });

    expect(result.message).toBe("Заказ обновлён.");
    expect(result.order.crmServiceUrl).toBe("https://crm.example.com");
    expect(fetch).toHaveBeenCalledWith("/api/v1/orders/101", {
      method: "PUT",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      credentials: "include",
      body: JSON.stringify({
        order_id: 101,
        status: "Processing",
        reference: "ORD-101",
        notes: "Комментарий",
        shipping_address: "Москва",
        consignee: "Иван",
        delivery_notes: "Позвонить заранее",
        payer: "ООО Ромашка",
      }),
    });
  });

  it("parses field-addressable mutation errors", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({
          message: "Ошибка валидации формы.",
          field_errors: [
            {
              field: "status",
              message: "Выберите статус заказа.",
            },
          ],
        }),
        {
          status: 422,
          headers: { "Content-Type": "application/json" },
        },
      ),
    );

    await expect(
      updateOrder(101, {
        orderId: 101,
        status: "",
        reference: null,
        notes: null,
        shippingAddress: null,
        consignee: null,
        deliveryNotes: null,
        payer: null,
      }),
    ).rejects.toMatchObject({
      message: "Ошибка валидации формы.",
      field_errors: [
        {
          field: "status",
          message: "Выберите статус заказа.",
        },
      ],
    });
  });

  it("redirects to login on non-json auth redirects during mutations", async () => {
    const response = new Response("<html>signin</html>", {
      status: 200,
      headers: { "Content-Type": "text/html" },
    });
    Object.defineProperty(response, "redirected", { value: true });
    Object.defineProperty(response, "url", { value: "/auth/signin" });
    const assignSpy = vi
      .spyOn(browserLocation, "assign")
      .mockImplementation(() => undefined);

    vi.mocked(fetch).mockResolvedValue(response);

    await expect(
      updateOrder(101, {
        orderId: 101,
        status: "Processing",
        reference: null,
        notes: null,
        shippingAddress: null,
        consignee: null,
        deliveryNotes: null,
        payer: null,
      }),
    ).rejects.toThrow("Сессия истекла. Выполняется переход на страницу входа.");

    expect(assignSpy).toHaveBeenCalledWith("/auth/signin");
  });

  it("parses the products collection payload", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({
          items: [
            {
              id: 9,
              name: "Coffee",
              sku: "COF-1",
              description_html: "<p>Arabica</p>",
              units: "кг",
              amount: "1",
              currency: "RUB",
              is_archived: false,
              category: { id: 4, name: "Напитки" },
              vendor: { id: 7, name: "Поставщик" },
              updated_at: "2024-01-02 11:15",
              image_urls: ["https://cdn.example.com/coffee.png"],
              tags: [{ id: 3, name: "Сезон" }],
              price_levels: [
                {
                  price_level_id: 1,
                  price_level_name: "Retail",
                  price_cents: 1250,
                },
              ],
            },
          ],
          pagination: {
            page: 2,
            per_page: 20,
            total_items: 41,
            total_pages: 3,
            has_previous_page: true,
            has_next_page: true,
          },
          active_filters: {
            search: "coffee",
            show_archived: true,
          },
          editor_options: {
            categories: [{ id: 4, name: "Напитки" }],
            tags: [{ id: 3, name: "Сезон" }],
            price_levels: [{ id: 1, name: "Retail" }],
            vendors: [{ id: 7, name: "Поставщик" }],
          },
        }),
        {
          status: 200,
          headers: { "Content-Type": "application/json" },
        },
      ),
    );

    const result = await fetchProductsCollection({
      search: "coffee",
      page: 2,
      showArchived: true,
    });

    expect(result.items[0]).toMatchObject({
      id: 9,
      name: "Coffee",
      sku: "COF-1",
      descriptionHtml: "<p>Arabica</p>",
      currency: "RUB",
      isArchived: false,
    });
    expect(result.items[0].category?.name).toBe("Напитки");
    expect(result.items[0].vendor?.name).toBe("Поставщик");
    expect(result.items[0].priceLevels[0]).toMatchObject({
      priceLevelId: 1,
      priceLevelName: "Retail",
      priceCents: 1250,
    });
    expect(result.activeFilters).toMatchObject({
      search: "coffee",
      showArchived: true,
    });
    expect(result.editorOptions.tags[0].name).toBe("Сезон");
    expect(fetch).toHaveBeenCalledWith(
      "/api/v1/products?search=coffee&page=2&show_archived=true",
      {
        headers: {
          Accept: "application/json",
        },
        cache: "no-store",
        credentials: "include",
      },
    );
  });

  it("parses product details payloads", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({
          id: 9,
          name: "Coffee",
          sku: "COF-1",
          description_html: "<p>Arabica</p>",
          units: "кг",
          amount: "1",
          currency: "RUB",
          is_archived: false,
          category_id: 4,
          vendor_id: 7,
          tag_ids: [3, 5],
          image_urls: ["https://cdn.example.com/coffee.png"],
          price_levels: [
            {
              price_level_id: 1,
              price_level_name: "Retail",
              price_cents: 1250,
            },
          ],
          updated_at: "2024-01-02 11:15",
          editor_options: {
            categories: [{ id: 4, name: "Напитки" }],
            tags: [
              { id: 3, name: "Сезон" },
              { id: 5, name: "Новинка" },
            ],
            price_levels: [{ id: 1, name: "Retail" }],
            vendors: [{ id: 7, name: "Поставщик" }],
          },
        }),
        {
          status: 200,
          headers: { "Content-Type": "application/json" },
        },
      ),
    );

    const result = await fetchProductDetails(9);

    expect(result).toMatchObject({
      id: 9,
      name: "Coffee",
      descriptionHtml: "<p>Arabica</p>",
      categoryId: 4,
      vendorId: 7,
      tagIds: [3, 5],
    });
    expect(result.editorOptions.vendors[0].name).toBe("Поставщик");
  });

  it("parses product mutation success payloads", async () => {
    const productMutationPayload = {
      message: "Товар добавлен.",
      product: {
        id: 9,
        name: "Coffee",
        sku: "COF-1",
        description_html: "<p>Arabica</p>",
        units: "кг",
        amount: "1",
        currency: "RUB",
        is_archived: false,
        category_id: 4,
        vendor_id: 7,
        tag_ids: [3],
        image_urls: ["https://cdn.example.com/coffee.png"],
        price_levels: [
          {
            price_level_id: 1,
            price_level_name: "Retail",
            price_cents: 1250,
          },
        ],
        updated_at: "2024-01-02 11:15",
        editor_options: {
          categories: [{ id: 4, name: "Напитки" }],
          tags: [{ id: 3, name: "Сезон" }],
          price_levels: [{ id: 1, name: "Retail" }],
          vendors: [{ id: 7, name: "Поставщик" }],
        },
      },
    };

    vi.mocked(fetch)
      .mockResolvedValueOnce(
        new Response(JSON.stringify(productMutationPayload), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify(productMutationPayload), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );

    const createResult = await createProduct({
      name: "Coffee",
      sku: "COF-1",
      descriptionHtml: "<p>Arabica</p>",
      units: "кг",
      amount: 1,
      currency: "RUB",
      isArchived: false,
      categoryId: 4,
      vendorId: 7,
      tagIds: [3],
      imageUrls: ["https://cdn.example.com/coffee.png"],
      priceLevels: [{ priceLevelId: 1, price: "12.50" }],
    });

    expect(createResult.message).toBe("Товар добавлен.");
    expect(createResult.product.id).toBe(9);

    const updateResult = await updateProduct(9, {
      productId: 9,
      name: "Coffee",
      sku: "COF-1",
      descriptionHtml: "<p>Arabica</p>",
      units: "кг",
      amount: 1,
      currency: "RUB",
      isArchived: true,
      categoryId: 4,
      vendorId: 7,
      tagIds: [3],
      imageUrls: ["https://cdn.example.com/coffee.png"],
      priceLevels: [{ priceLevelId: 1, price: "12.50" }],
    });

    expect(updateResult.product.isArchived).toBe(false);
    expect(fetch).toHaveBeenCalledWith("/api/v1/products", {
      method: "POST",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      credentials: "include",
      body: JSON.stringify({
        name: "Coffee",
        sku: "COF-1",
        description: "<p>Arabica</p>",
        units: "кг",
        currency: "RUB",
        category_id: 4,
        vendor_id: 7,
        tag_ids: [3],
        image_urls: "https://cdn.example.com/coffee.png",
        price_levels: [{ price_level_id: 1, price: "12.50" }],
        amount: 1,
      }),
    });
    expect(fetch).toHaveBeenCalledWith("/api/v1/products/9", {
      method: "PUT",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      credentials: "include",
      body: JSON.stringify({
        product_id: 9,
        name: "Coffee",
        sku: "COF-1",
        description: "<p>Arabica</p>",
        units: "кг",
        currency: "RUB",
        image_urls: "https://cdn.example.com/coffee.png",
        is_archived: true,
        category_id: 4,
        vendor_id: 7,
        tag_ids: [3],
        price_levels: [{ price_level_id: 1, price: "12.50" }],
        amount: 1,
      }),
    });
  });

  it("uploads product csv files via multipart form data", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({
          message: "Загружено товаров: 2.",
          created_count: 2,
        }),
        {
          status: 200,
          headers: { "Content-Type": "application/json" },
        },
      ),
    );

    const file = new File(["name,currency\nCoffee,RUB\n"], "products.csv", {
      type: "text/csv",
    });
    const result = await uploadProducts(file);

    expect(result).toMatchObject({
      message: "Загружено товаров: 2.",
      createdCount: 2,
    });
    expect(fetch).toHaveBeenCalledWith("/api/v1/products/upload", {
      method: "POST",
      headers: {
        Accept: "application/json",
      },
      credentials: "include",
      body: expect.any(FormData),
    });
  });
});
