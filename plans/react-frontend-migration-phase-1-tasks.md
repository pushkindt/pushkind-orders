# Tasks: React Frontend Migration Phase 1

## Scope
This task file covers only Phase 1 from
[react-frontend-migration.md](../plans/react-frontend-migration.md):

- create the React + TypeScript + Vite frontend workspace
- emit build output into `assets/dist/`
- add backend helpers for loading the Vite manifest and opening built HTML
  documents
- add the required ADR because this work changes frontend runtime architecture
- document how to install dependencies and build frontend assets
- keep the live application on the current Tera runtime path

Do not start Phase 2 in this file. Phase 1 is complete only when the
repository can build frontend assets, Rust can read the built frontend
artifacts needed for later route cutover, and the live routes still render
through the current Tera templates.

## References
- Service baseline:
  [../SPEC.md](../SPEC.md)
- Feature spec:
  [../specs/features/react-frontend-migration.md](../specs/features/react-frontend-migration.md)
- Migration plan:
  [../plans/react-frontend-migration.md](../plans/react-frontend-migration.md)
- Contributor rules:
  [../AGENTS.md](../AGENTS.md)
- Existing app wiring:
  [../src/lib.rs](../src/lib.rs)
- Existing documentation:
  [../README.md](../README.md)

## Preconditions
- Work in `/home/matrizaev/pushkind/pushkind-orders`.
- Treat the feature spec and migration plan as the source of truth.
- Do not change current route behavior in this phase.
- Do not add new `/api/v1/...` endpoints in this phase.
- Do not remove Tera templates or flash-message middleware in this phase.
- Do not cut over `GET /`, `GET /order/{order_id}`, `GET /products`,
  `GET /categories`, `GET /tags`, `GET /price-levels`, `GET /vendors`, or
  `GET /na` to built HTML in this phase.
- Do not change the Store API under `/api/v1/store/{hub_id}` in this phase.

## What You Will Change In Phase 1
You will change only these repository areas:

- create `specs/decisions/0001-react-frontend-runtime.md`
- create `frontend/` and its initial source tree
- create `src/frontend.rs`
- edit `src/lib.rs` to expose the new backend helper module
- edit `.gitignore`
- edit `README.md`

If you find yourself changing routes, templates, forms, DTOs, services, or
Store API behavior, stop. That belongs to later phases.

## Deliverables
- `frontend/` exists with a working React + TypeScript + Vite setup.
- `assets/dist/` is the configured production build output directory.
- `assets/dist/manifest.json` is produced by `npm run build`.
- `src/frontend.rs` can:
  define built document paths for later phases,
  read and parse the Vite manifest,
  resolve a manifest entry by name,
  open a built HTML document.
- `README.md` explains how to install frontend dependencies and build assets.
- The application still renders the current Tera UI at runtime.

## Step 0: Baseline Snapshot
Run these commands first so you know what changed later:

```bash
pwd
git status --short
find frontend -maxdepth 3 -type f 2>/dev/null
find specs/decisions -maxdepth 2 -type f 2>/dev/null
cargo build --all-features --verbose
```

Expected result before you start:
- there is no `frontend/` directory in this repo yet
- there may be no `specs/decisions/` directory yet
- the Rust project builds successfully before any React work is added

## Task 1: Add The Required ADR First
`pushkind-orders/AGENTS.md` requires an ADR for architecture-affecting work.
This migration changes frontend runtime architecture, so create the ADR before
adding the scaffold.

### 1.1 Create The Decisions Directory
Run:

```bash
mkdir -p specs/decisions
```

### 1.2 Create `specs/decisions/0001-react-frontend-runtime.md`
Create [../specs/decisions/0001-react-frontend-runtime.md](../specs/decisions/0001-react-frontend-runtime.md)
with exactly this content:

```md
# ADR 0001: Adopt Incremental React Frontend With Vite-Built Documents

## Status
Proposed

## Context
`pushkind-orders` currently renders its authenticated hub pages with Tera
templates and augments that markup with Bootstrap behaviors, modal fragments,
and flash-driven redirect flows.

The approved frontend migration goal is to move the hub UI to React while
preserving:
- the existing server-routed URLs
- the non-SPA navigation model
- Bootstrap styling
- Russian copy
- backend-owned authorization, validation, pricing, vendor scoping, approval,
  and persistence rules

The customer-facing Store API under `/api/v1/store/{hub_id}` is not part of
this migration and must remain backend-owned.

## Decision
- Keep Actix routes and server-side request handling as the source of truth for
  navigation, redirects, authentication, and authorization.
- Introduce React incrementally on the existing hub URLs.
- Do not introduce client-side routing.
- Place frontend source code under `frontend/`.
- Build frontend assets and HTML documents with Vite into `assets/dist/`.
- Let Rust serve built HTML documents after performing route-level access
  checks.
- Move hub page initialization to typed `/api/v1/...` JSON APIs instead of
  embedding more page data into server-generated HTML.
- Keep Tera only as a migration wrapper until React equivalents are shipped and
  verified.
- Keep flash-message middleware only until React-owned mutation flows replace
  redirect-based feedback.
- Leave the Store API contract and Store API routing model unchanged.

## Consequences

### Positive
- React can be introduced without rewriting the backend architecture.
- The migration can proceed incrementally by page and interaction.
- Built frontend artifacts are served directly by the Rust application.
- The final runtime model becomes clearer: Rust owns routes and APIs, React
  owns page UI.

### Negative
- The service will temporarily carry both Tera and React concerns.
- A Node-based frontend toolchain becomes part of local development and CI.
- Some endpoints and flows will temporarily exist in both legacy and migrated
  forms during rollout.

## Rejected Alternatives
- Full SPA rewrite:
  rejected because it conflicts with the approved spec and would widen scope
  beyond a frontend migration.
- Continue with Tera + modal fragments + flash redirects:
  rejected because it does not achieve the approved React migration target.
- Expand this migration to include the Store API:
  rejected because the approved spec keeps storefront behavior out of scope.
- Keep Rust assembling HTML document shells permanently:
  rejected because the target state explicitly gives frontend document
  ownership to Vite-built static HTML.
```

### 1.3 Verify The ADR Exists
Run:

```bash
sed -n '1,240p' specs/decisions/0001-react-frontend-runtime.md
```

## Task 2: Create The Frontend Workspace
The goal here is to make the repository capable of building frontend assets
without changing live runtime behavior yet.

### 2.1 Create The Directory Tree
Run:

```bash
mkdir -p frontend/app
mkdir -p frontend/src/entries
mkdir -p frontend/src/components
mkdir -p frontend/src/pages
mkdir -p frontend/src/styles
mkdir -p frontend/src/lib
```

### 2.2 Create `frontend/package.json`
Create [../frontend/package.json](../frontend/package.json) with exactly this
content:

```json
{
  "name": "pushkind-orders-frontend",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "lint": "tsc --noEmit",
    "typecheck": "tsc --noEmit",
    "format": "prettier --write .",
    "format:check": "prettier --check ."
  },
  "dependencies": {
    "react": "^19.2.4",
    "react-dom": "^19.2.4"
  },
  "devDependencies": {
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^6.0.1",
    "jsdom": "^29.0.1",
    "prettier": "^3.8.1",
    "typescript": "^6.0.2",
    "vite": "^8.0.1",
    "vitest": "^4.1.0"
  }
}
```

### 2.3 Create `frontend/tsconfig.json`
Create [../frontend/tsconfig.json](../frontend/tsconfig.json) with exactly
this content:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "lib": ["DOM", "DOM.Iterable", "ES2022"],
    "allowJs": false,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "strict": true,
    "forceConsistentCasingInFileNames": true,
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx"
  },
  "include": ["src"]
}
```

### 2.4 Create `frontend/vite.config.ts`
Create [../frontend/vite.config.ts](../frontend/vite.config.ts) with exactly
this content:

```ts
import { resolve } from "node:path";

import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  base: "/assets/dist/",
  plugins: [react()],
  test: {
    environment: "jsdom",
    environmentOptions: {
      jsdom: {
        url: "http://localhost/",
      },
    },
    include: ["src/**/*.test.ts?(x)"],
  },
  build: {
    manifest: "manifest.json",
    outDir: resolve(__dirname, "../assets/dist"),
    emptyOutDir: true,
    rollupOptions: {
      input: {
        "app/index.html": resolve(__dirname, "app/index.html"),
      },
      output: {
        entryFileNames: "entries/[name]-[hash].js",
        chunkFileNames: "chunks/[name]-[hash].js",
        assetFileNames: ({ name }) => {
          if (name?.endsWith(".css")) {
            return "styles/[name]-[hash][extname]";
          }

          return "assets/[name]-[hash][extname]";
        },
      },
    },
  },
});
```

### 2.5 Create The Minimal Placeholder Frontend
Create these files exactly as written.

Create [../frontend/app/index.html](../frontend/app/index.html):

```html
<!doctype html>
<html lang="ru">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Orders</title>
    <link rel="icon" href="/assets/favicon.ico" type="image/x-icon" />
    <link
      href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/css/bootstrap.min.css"
      rel="stylesheet"
      integrity="sha384-QWTKZyjpPEjISv5WaRU9OFeRpok6YctnYmDr5pNlyT2bRjXh0JMhjY6hW+ALEwIH"
      crossorigin="anonymous"
    />
    <link
      rel="stylesheet"
      href="https://cdn.jsdelivr.net/npm/bootstrap-icons@1.11.3/font/bootstrap-icons.min.css"
    />
    <script type="module" src="/src/entries/index.tsx"></script>
  </head>
  <body class="bg-light">
    <div id="react-root"></div>
    <script
      src="https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/js/bootstrap.bundle.min.js"
      integrity="sha384-YvpcrYf0tY3lHB60NNkmXc5s9fDVZLESaAA55NDzOxhy9GkcIdslK1eN7N6jIeHz"
      crossorigin="anonymous"
    ></script>
  </body>
</html>
```

Create [../frontend/src/entries/index.tsx](../frontend/src/entries/index.tsx):

```tsx
import { mountPage } from "../lib/mount";
import { PhaseOnePlaceholderPage } from "../pages/PhaseOnePlaceholderPage";

mountPage(
  "react-root",
  <PhaseOnePlaceholderPage
    badge="Phase 1"
    title="Orders frontend scaffold is ready"
    description="Vite can now build the future React frontend for pushkind-orders. Live hub routes still render through Tera until later phases."
    routeLabel="GET /"
  />,
);
```

Create [../frontend/src/lib/mount.tsx](../frontend/src/lib/mount.tsx):

```tsx
import type { ReactNode } from "react";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "../styles/foundation.css";

export function mountPage(elementId: string, page: ReactNode): void {
  const rootElement = document.getElementById(elementId);

  if (!rootElement) {
    throw new Error(`Missing React mount node: #${elementId}`);
  }

  const root = createRoot(rootElement);
  root.render(<StrictMode>{page}</StrictMode>);
}
```

Create [../frontend/src/components/PhaseOneStatusCard.tsx](../frontend/src/components/PhaseOneStatusCard.tsx):

```tsx
type PhaseOneStatusCardProps = {
  badge: string;
  title: string;
  description: string;
  routeLabel: string;
};

export function PhaseOneStatusCard({
  badge,
  title,
  description,
  routeLabel,
}: PhaseOneStatusCardProps) {
  return (
    <div className="card border-0 shadow-sm">
      <div className="card-body p-4 p-lg-5">
        <span className="badge text-bg-secondary mb-3">{badge}</span>
        <h1 className="h3 mb-3">{title}</h1>
        <p className="text-body-secondary mb-4">{description}</p>
        <dl className="row mb-0">
          <dt className="col-sm-4">Маршрут</dt>
          <dd className="col-sm-8">
            <code className="phase-one-code">{routeLabel}</code>
          </dd>
          <dt className="col-sm-4">Статус</dt>
          <dd className="col-sm-8">
            В Phase 1 этот экран существует только как проверка frontend build
            pipeline. Живая страница по-прежнему рендерится через Tera.
          </dd>
        </dl>
      </div>
    </div>
  );
}
```

Create [../frontend/src/pages/PhaseOnePlaceholderPage.tsx](../frontend/src/pages/PhaseOnePlaceholderPage.tsx):

```tsx
import { PhaseOneStatusCard } from "../components/PhaseOneStatusCard";

type PhaseOnePlaceholderPageProps = {
  badge: string;
  title: string;
  description: string;
  routeLabel: string;
};

export function PhaseOnePlaceholderPage({
  badge,
  title,
  description,
  routeLabel,
}: PhaseOnePlaceholderPageProps) {
  return (
    <main className="phase-one-placeholder container py-4 py-lg-5">
      <div className="row justify-content-center">
        <div className="col-12 col-xl-8">
          <PhaseOneStatusCard
            badge={badge}
            title={title}
            description={description}
            routeLabel={routeLabel}
          />
        </div>
      </div>
    </main>
  );
}
```

Create [../frontend/src/styles/foundation.css](../frontend/src/styles/foundation.css):

```css
:root {
  color-scheme: light;
}

body {
  min-height: 100vh;
}

.phase-one-placeholder {
  min-height: 100vh;
}

.phase-one-code {
  font-family: var(--bs-font-monospace, monospace);
}
```

Create [../frontend/src/vite-env.d.ts](../frontend/src/vite-env.d.ts):

```ts
/// <reference types="vite/client" />
```

### 2.6 Install Frontend Dependencies
Run:

```bash
cd frontend
npm install
```

This must create `frontend/package-lock.json`.

### 2.7 Verify The Frontend Scaffold
Run:

```bash
cd frontend
npm run typecheck
npm run test
npm run build
```

Expected result:
- `frontend/package-lock.json` exists
- `assets/dist/manifest.json` exists
- `assets/dist/app/index.html` exists

## Task 3: Add Backend Frontend-Asset Infrastructure
The goal here is to make Rust understand the future built frontend artifacts
without cutting over any route yet.

### 3.1 Create `src/frontend.rs`
Create [../src/frontend.rs](../src/frontend.rs) with exactly this content:

```rust
//! Helpers for loading compiled frontend assets and opening built HTML documents.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use actix_files::NamedFile;
use serde::Deserialize;
use thiserror::Error;

/// Root directory for built frontend artifacts emitted by Vite.
pub const FRONTEND_DIST_DIR: &str = "assets/dist";

/// Relative path of the Vite manifest inside [`FRONTEND_DIST_DIR`].
pub const FRONTEND_MANIFEST_FILE: &str = "manifest.json";

/// Built HTML document that will eventually back `GET /`.
pub const FRONTEND_INDEX_DOCUMENT: &str = "app/index.html";

/// Minimal subset of a Vite manifest entry needed by the backend.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct FrontendManifestEntry {
    pub file: String,
    #[serde(default)]
    pub css: Vec<String>,
    #[serde(default)]
    pub imports: Vec<String>,
}

/// Vite manifest keyed by source entry name such as `app/index.html`.
pub type FrontendManifest = BTreeMap<String, FrontendManifestEntry>;

/// Errors raised while loading frontend assets.
#[derive(Debug, Error)]
pub enum FrontendAssetError {
    #[error("failed to read frontend manifest: {0}")]
    ManifestRead(std::io::Error),
    #[error("failed to parse frontend manifest: {0}")]
    ManifestParse(serde_json::Error),
    #[error("frontend manifest entry not found: {0}")]
    MissingEntry(String),
    #[error("failed to open frontend document: {0}")]
    Read(#[from] std::io::Error),
}

/// Absolute filesystem path for the Vite manifest.
pub fn frontend_manifest_path() -> PathBuf {
    Path::new(FRONTEND_DIST_DIR).join(FRONTEND_MANIFEST_FILE)
}

/// Absolute filesystem path for a built frontend HTML document.
pub fn frontend_document_path(document: &str) -> PathBuf {
    Path::new(FRONTEND_DIST_DIR).join(document)
}

/// Load and parse the Vite manifest file.
pub fn load_frontend_manifest(path: impl AsRef<Path>) -> Result<FrontendManifest, FrontendAssetError> {
    let manifest_bytes = std::fs::read(path).map_err(FrontendAssetError::ManifestRead)?;
    serde_json::from_slice(&manifest_bytes).map_err(FrontendAssetError::ManifestParse)
}

/// Resolve a named Vite entry such as `app/index.html`.
pub fn resolve_frontend_entry<'a>(
    manifest: &'a FrontendManifest,
    entry_name: &str,
) -> Result<&'a FrontendManifestEntry, FrontendAssetError> {
    manifest
        .get(entry_name)
        .ok_or_else(|| FrontendAssetError::MissingEntry(entry_name.to_owned()))
}

/// Open a Vite-built HTML document for a future React-owned route.
pub async fn open_frontend_html(path: impl AsRef<Path>) -> Result<NamedFile, FrontendAssetError> {
    let file = NamedFile::open_async(path).await?;
    Ok(file.use_last_modified(true).prefer_utf8(true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn loads_and_resolves_manifest_entry() {
        let dir = tempdir().expect("tempdir should be created");
        let manifest_path = dir.path().join("manifest.json");

        std::fs::write(
            &manifest_path,
            r#"{
  "app/index.html": {
    "file": "entries/app/index.html-abc123.js",
    "css": ["styles/app/index.html-abc123.css"],
    "imports": ["_shared-vendor-xyz.js"],
    "isEntry": true
  }
}"#,
        )
        .expect("manifest should be written");

        let manifest = load_frontend_manifest(&manifest_path).expect("manifest should parse");
        let entry =
            resolve_frontend_entry(&manifest, "app/index.html").expect("entry should exist");

        assert_eq!(entry.file, "entries/app/index.html-abc123.js");
        assert_eq!(entry.css, vec!["styles/app/index.html-abc123.css"]);
        assert_eq!(entry.imports, vec!["_shared-vendor-xyz.js"]);
    }

    #[test]
    fn missing_entry_returns_error() {
        let manifest = FrontendManifest::new();

        let error = resolve_frontend_entry(&manifest, "app/index.html")
            .expect_err("missing entry should return an error");

        assert!(matches!(
            error,
            FrontendAssetError::MissingEntry(name) if name == "app/index.html"
        ));
    }

    #[test]
    fn can_open_existing_file() {
        let dir = tempdir().expect("tempdir should be created");
        let html_path = dir.path().join("index.html");
        std::fs::write(&html_path, "<!doctype html><title>ok</title>")
            .expect("html file should be written");

        let result = actix_web::rt::System::new().block_on(open_frontend_html(&html_path));
        assert!(result.is_ok());
    }

    #[test]
    fn missing_document_returns_read_error() {
        let error = actix_web::rt::System::new()
            .block_on(open_frontend_html("assets/dist/does-not-exist.html"))
            .expect_err("missing file should return an error");

        assert!(matches!(error, FrontendAssetError::Read(_)));
    }
}
```

### 3.2 Expose The Module In `src/lib.rs`
Edit [../src/lib.rs](../src/lib.rs) and add exactly this line alongside the
other public module declarations:

```rust
pub mod frontend;
```

Do not use the helper from any route yet. Phase 1 is infrastructure only.

### 3.3 Verify The Backend Helper
Run:

```bash
cargo test frontend --all-features
cargo build --all-features --verbose
```

Expected result:
- the helper tests pass
- no route behavior changes

## Task 4: Keep Runtime Behavior Unchanged
Land Phase 1 without partially migrating the UI.

Checks:
1. Confirm [../src/lib.rs](../src/lib.rs) still serves the
   current Tera-based hub pages.
2. Confirm no route starts using the new built HTML helper yet.
3. Confirm no hub UI behavior is moved to React in this phase.
4. Confirm no new `/api/v1/...` endpoints are introduced in this phase.
5. Confirm the Store API under `/api/v1/store/{hub_id}` is untouched.

Acceptance checks:
- existing Tera templates remain the active rendering path
- existing modal fragment routes remain active
- existing flash-message behavior remains active

## Task 5: Update `.gitignore`
Edit [../.gitignore](../.gitignore) and add these lines:

```gitignore
frontend/node_modules/
assets/dist/
```

Keep the existing entries as they are.

Acceptance checks:
- generated frontend dependencies are ignored
- generated build output is ignored

## Task 6: Update `README.md`
Edit [../README.md](../README.md) and add a new subsection under
`## Getting Started` or `## Development` with this exact content:

````md
### Frontend Assets

Phase 1 of the React migration adds a frontend workspace under `frontend/`.
The live hub UI still renders through Tera in this phase; the frontend build is
scaffolded now so later phases can cut routes over incrementally.

Install frontend dependencies with:

```bash
cd frontend
npm install
```

Build frontend assets with:

```bash
cd frontend
npm run build
```

The production build output is written to `assets/dist/`.

In Phase 1, `cargo run` still uses the existing Tera runtime path, so a
frontend build is not required just to start the current application. Build the
frontend now to verify the scaffold and to prepare for later route cutover.
````

Acceptance checks:
- a contributor can read `README.md` and reproduce the Phase 1 setup
- the README clearly states that runtime is still Tera in Phase 1

## Task 7: Verify Phase 1 End To End
Run these commands from `pushkind-orders` unless noted otherwise:

1. `cd frontend && npm run typecheck`
2. `cd frontend && npm run test`
3. `cd frontend && npm run build`
4. `cargo build --all-features --verbose`
5. `cargo test --all-features --verbose`
6. `cargo clippy --all-features --tests -- -Dwarnings`
7. `cargo fmt --all -- --check`

What to confirm:
- frontend build succeeds
- backend still builds cleanly
- new frontend helper tests pass
- no route behavior changed yet

## Phase 1 Exit Checklist
Mark Phase 1 done only if all of the following are true:

- `specs/decisions/0001-react-frontend-runtime.md` exists.
- `frontend/` exists with React, TypeScript, and Vite configured.
- `frontend/package-lock.json` is committed.
- Vite emits build artifacts into `assets/dist/`.
- `assets/dist/manifest.json` is produced by the build.
- Rust has tested helpers for manifest loading, manifest entry resolution, and
  built HTML opening.
- `.gitignore` excludes `frontend/node_modules/` and `assets/dist/`.
- `README.md` explains how to build frontend assets.
- The service still renders the existing Tera UI at runtime.
- The Store API remains unchanged.

## Explicit Non-Goals For This Task File
Do not do these here:

- switch any route to Vite-built HTML
- add `/api/v1/iam`
- add `/api/v1/orders/{order_id}`
- add `/api/v1/products`
- add `/api/v1/categories`
- add `/api/v1/tags`
- add `/api/v1/price-levels`
- add `/api/v1/vendors`
- add `/api/v1/users`
- add `/api/v1/no-access`
- convert flash-message POST handlers to JSON mutation responses
- delete Tera templates
- remove `tera`
- remove `actix-web-flash-messages`
- change any Store API route or DTO
