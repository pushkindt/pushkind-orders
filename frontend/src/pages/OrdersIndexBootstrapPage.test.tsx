import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

vi.mock("@pushkind/frontend-shell/useServiceShell", () => ({
  useServiceShell: () => ({
    status: "error" as const,
    message: "Shell failed",
  }),
}));

import { OrdersIndexBootstrapPage } from "./OrdersIndexBootstrapPage";

describe("OrdersIndexBootstrapPage", () => {
  it("renders the shell fatal state when shell loading fails", () => {
    const markup = renderToStaticMarkup(<OrdersIndexBootstrapPage />);

    expect(markup).toContain("Не удалось загрузить страницу");
    expect(markup).toContain("Shell failed");
  });
});
