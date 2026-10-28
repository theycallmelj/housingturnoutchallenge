# Installed dependency license review

Reviewed local installed package metadata. No paid services, model libraries or data dependencies. All installed packages declare permissive licenses; retain notices when distributing. Final challenge dependency approval and a formal SBOM are still required.

| Package | Version | Declared license | Purpose |
| --- | --- | --- | --- |
| react | 19.2.0 | MIT | UI runtime |
| react-dom | 19.2.0 | MIT | Browser rendering |
| scheduler | 0.27.0 | MIT | React transitive runtime |
| typescript | 5.9.3 | Apache-2.0 | Build-time type checking |
| vite | 7.1.12 | MIT | Local preview and bundling |
| @types/react | 19.2.2 | MIT | Type definitions |
| @types/react-dom | 19.2.2 | MIT | Type definitions |
| csstype | 3.2.3 | MIT | Type definitions |
| esbuild, @esbuild/win32-x64 | 0.25.12 | MIT | Build tooling |
| rollup, @rollup/rollup-win32-x64-gnu, @rollup/rollup-win32-x64-msvc | 4.64.6 | MIT | Build tooling |
| @types/estree | 1.0.9 | MIT | Tooling types |
| fdir | 6.5.0 | MIT | Tooling file traversal |
| picomatch | 4.0.7 | MIT | Tooling path matching |
| tinyglobby | 0.2.17 | MIT | Tooling path matching |
| postcss | 8.5.30 | MIT | CSS tooling |
| nanoid | 3.3.20 | MIT | Tooling identifiers |
| picocolors | 1.1.1 | ISC | Terminal tooling |
| source-map-js | 1.2.2 | BSD-3-Clause | Build source maps |

The lockfile also describes optional binaries for other platforms; only the installed Windows variants were inspected. The team's original source code is licensed under Apache-2.0 (see the repository root `LICENSE`).
