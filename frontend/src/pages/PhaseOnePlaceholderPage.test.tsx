import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { PhaseOnePlaceholderPage } from "./PhaseOnePlaceholderPage";

describe("PhaseOnePlaceholderPage", () => {
  it("renders the provided status content", () => {
    const markup = renderToStaticMarkup(
      <PhaseOnePlaceholderPage
        badge="Phase 1"
        title="Orders frontend scaffold is ready"
        description="Scaffold description"
        routeLabel="GET /"
      />,
    );

    expect(markup).toContain("Orders frontend scaffold is ready");
    expect(markup).toContain("GET /");
    expect(markup).toContain("Phase 1");
  });
});
