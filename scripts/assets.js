#!/usr/bin/env node

/**
 * Orchestrates frontend asset builds with optional package filtering
 *
 * Usage:
 *   pnpm assets                         # Build all packages
 *   pnpm assets --filter @blog/styles   # Build only styles
 *   pnpm assets --filter @blog/scripts  # Build only scripts
 *   pnpm assets --filter @blog/icon     # Build only icons
 */

import { execSync } from "child_process";

const filterArg = process.argv.find((arg) => arg.startsWith("--filter"));
const filterValue = filterArg ? process.argv[process.argv.indexOf(filterArg) + 1] : null;

const pnpmFilter = filterValue ? `--filter ${filterValue}` : "-r";

console.log("🔨 Building assets...\n");

// Build packages (filtered or all)
execSync(`pnpm ${pnpmFilter} build`, { stdio: "inherit" });

// Generate manifest (always full - needed for Rust SSG)
execSync("pnpm manifest", { stdio: "inherit" });

// Copy assets (filtered or all)
const copyFilter = filterValue ? `--filter=${filterValue.replace("@blog/", "")}` : "";
execSync(`node scripts/copy-assets.js ${copyFilter}`, { stdio: "inherit" });
