import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

vi.mock("@pushkind/frontend-shell/useServiceShell", () => ({
  useServiceShell: () => ({
    status: "error" as const,
    message: "Shell failed",
  }),
}));

import {
  ProductsEmptyState,
  ProductsPage,
  buildProductsPageUrl,
} from "./ProductsPage";

describe("ProductsPage", () => {
  it("renders the shell fatal state when shell loading fails", () => {
    const markup = renderToStaticMarkup(<ProductsPage />);

    expect(markup).toContain("Не удалось загрузить страницу");
    expect(markup).toContain("Shell failed");
  });

  it("renders the empty state copy", () => {
    const markup = renderToStaticMarkup(<ProductsEmptyState />);

    expect(markup).toContain("Нет товаров для отображения.");
  });

  it("builds native products pagination links", () => {
    expect(buildProductsPageUrl(1, null, false)).toBe("/products");
    expect(buildProductsPageUrl(2, "coffee", true)).toBe(
      "/products?search=coffee&page=2&show_archived=true",
    );
  });
});
