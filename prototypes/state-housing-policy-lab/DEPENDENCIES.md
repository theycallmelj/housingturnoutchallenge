# Installed dependency license review

Reviewed local installed package metadata. No paid services, model libraries or data dependencies. All installed packages declare permissive licenses; retain notices when distributing.

**SBOM:** [`sbom.cdx.json`](sbom.cdx.json) (CycloneDX 1.5, regenerated 2026-11-23 after the React 19.3 and TypeScript 7 upgrades, with `@cyclonedx/cdxgen@12.9.0` from `pnpm-lock.yaml`). All 91 components are permissive: 68 MIT, 21 Apache-2.0 (TypeScript and its 20 platform binaries), 1 ISC, 1 BSD-3-Clause. CI (`web.yml`) fails if it stops matching the lockfile. To regenerate:

```bash
npx --yes @cyclonedx/cdxgen@12.9.0 -t pnpm --spec-version 1.5 --no-recurse -o sbom.cdx.json .
```

| Package | Version | Declared license | Purpose |
| --- | --- | --- | --- |
| react | 19.3.1 | MIT | UI runtime |
| react-dom | 19.3.1 | MIT | Browser rendering |
| scheduler | 0.28.0 | MIT | React transitive runtime |
| typescript (+ 20 platform binaries, `@typescript/typescript-*`) | 7.0.2 | Apache-2.0 | Build-time type checking (native compiler) |
| vite | 7.3.7 | MIT | Local preview and bundling (upgraded from 7.1.12 on 2026-11-15 to fix 20 Dependabot alerts, 6 high) |
| @types/react | 19.3.0 | MIT | Type definitions |
| @types/react-dom | 19.3.0 | MIT | Type definitions |
| csstype | 3.2.3 | MIT | Type definitions |
| esbuild (+ platform binaries) | 0.28.2 | MIT | Build tooling |
| rollup, @rollup/rollup-win32-x64-gnu, @rollup/rollup-win32-x64-msvc | 4.64.6 | MIT | Build tooling |
| @types/estree | 1.0.9 | MIT | Tooling types |
| fdir | 6.5.0 | MIT | Tooling file traversal |
| picomatch | 4.0.7 | MIT | Tooling path matching |
| tinyglobby | 0.2.17 | MIT | Tooling path matching |
| postcss | 8.5.30 | MIT | CSS tooling |
| nanoid | 3.3.20 | MIT | Tooling identifiers |
| picocolors | 1.1.1 | ISC | Terminal tooling |
| source-map-js | 1.2.2 | BSD-3-Clause | Build source maps |

The lockfile also describes optional binaries for other platforms; only the installed Windows variants were inspected (macOS arm64 for the TypeScript 7 binaries).

**Not upgraded: Vite 8.** Vite 8 depends directly on `lightningcss`, licensed MPL-2.0, which the repository rules (AGENTS.md §1) do not allow. Vite stays on 7.x, where `lightningcss` is only an optional peer dependency that is not installed. The team's original source code is licensed under Apache-2.0 (see the repository root `LICENSE`).
