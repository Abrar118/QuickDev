# Repository Guidelines

## Project Structure & Module Organization
`src/` contains the React + TypeScript UI. Keep routed screens in `src/pages/`, shared UI in `src/components/`, reusable helpers in `src/lib/`, hooks in `src/hooks/`, and shared types in `src/types/`. Static assets live in `public/` and `src/assets/`.  

`src-tauri/` contains the desktop shell and native logic. Rust entry points live in `src-tauri/src/`, Tauri commands are grouped under `src-tauri/src/commands/`, database setup lives in `src-tauri/src/db.rs`, and SQLite migrations belong in `src-tauri/migrations/`. Styling is primarily in `src/index.css` and `styles/`.

## Build, Test, and Development Commands
Use npm for the frontend and Cargo through Tauri for the desktop app:

- `npm install` installs frontend dependencies.
- `npm run dev` starts the Vite dev server for the React UI.
- `npm run build` runs `tsc` and produces a production web build.
- `npm run tauri dev` launches the desktop app with the Tauri backend.
- `npm run tauri build` creates a packaged desktop build.
- `cargo test --manifest-path src-tauri/Cargo.toml` runs Rust tests when present.

## Coding Style & Naming Conventions
Follow the existing style: 2-space indentation in TypeScript/TSX and standard Rust formatting in `src-tauri/`. Use PascalCase for React components (`ProjectCard.tsx`), kebab-case for route files (`project-management.tsx`), and camelCase for functions and variables. Prefer small, focused modules and keep Tauri command names descriptive, for example `create_project` and `get_projects`.

This repo uses TypeScript, Vite, Tailwind CSS v4, shadcn/ui conventions, and Tauri 2. There is no dedicated lint or formatter script yet, so run `cargo fmt --manifest-path src-tauri/Cargo.toml` for Rust and keep frontend changes consistent with nearby files before opening a PR.

## Testing Guidelines
Frontend test tooling is not configured yet, so validate UI changes manually with `npm run dev` or `npm run tauri dev`. For backend logic, add focused Rust unit or integration tests near the affected module and run `cargo test --manifest-path src-tauri/Cargo.toml`. Name tests after behavior, for example `creates_project_with_valid_path`.

## Commit & Pull Request Guidelines
Git history is minimal and currently includes only `#first-commit`, so use short, imperative commit messages until a stricter convention is adopted, for example `Add project creation command`. Keep commits scoped to one change.

Pull requests should include a clear summary, testing notes, linked issues if applicable, and screenshots or short recordings for UI changes. Call out schema or migration changes explicitly when touching `src-tauri/migrations/` or SQLite-related code.
