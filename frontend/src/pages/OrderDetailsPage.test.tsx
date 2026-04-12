import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { OrderStatusBadge } from "../components/OrderStatusBadge";
import { readOrderIdFromPathname } from "./OrderDetailsPage";
import {
  OrdersIndexEmptyState,
  buildOrdersIndexPageUrl,
} from "./OrdersIndexBootstrapPage";

describe("orders page helpers", () => {
  it("parses the order id from the native details route", () => {
    expect(readOrderIdFromPathname("/order/42")).toBe(42);
    expect(readOrderIdFromPathname("/order/42/")).toBe(42);
    expect(readOrderIdFromPathname("/orders/42")).toBeNull();
    expect(readOrderIdFromPathname("/order/not-a-number")).toBeNull();
  });

  it("renders localized order status badges", () => {
    const markup = renderToStaticMarkup(
      <OrderStatusBadge status="Processing" />,
    );

    expect(markup).toContain("В обработке");
    expect(markup).toContain("text-bg-primary");
  });

  it("renders the dashboard empty state copy", () => {
    const markup = renderToStaticMarkup(<OrdersIndexEmptyState />);

    expect(markup).toContain("Заказы не найдены");
    expect(markup).toContain("Попробуйте изменить поиск");
  });

  it("builds native pagination links for the dashboard", () => {
    expect(buildOrdersIndexPageUrl(1, null)).toBe("/");
    expect(buildOrdersIndexPageUrl(2, "coffee")).toBe("/?search=coffee&page=2");
  });
});
