#!/usr/bin/env node

/**
 * Generates manifest.json from package versions and assets
 *
 * Each package defines its assets in package.json under "blog.assets":
 * {
 *   "blog": {
 *     "assets": {
 *       "bundle": "bundle.js",
 *       "search": "search.js"
 *     }
 *   }
 * }
 *
 * Output: manifest.json with versioned paths
 * { "scripts": { "version": "0.1.0", "bundle": "/scripts/0.1.0/bundle.js" } }
 */

import { readdirSync, readFileSync, writeFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const rootDir = join(__dirname, "..");
const packagesDir = join(rootDir, "packages");

function discoverPackages() {
    const packages = [];

    const dirs = readdirSync(packagesDir, { withFileTypes: true })
        .filter((d) => d.isDirectory())
        .map((d) => d.name);

    for (const dir of dirs) {
        const packageJsonPath = join(packagesDir, dir, "package.json");
        try {
            const packageJson = JSON.parse(
                readFileSync(packageJsonPath, "utf-8")
            );

            // Check for blog.assets field
            if (packageJson.blog?.assets) {
                packages.push({
                    name: dir,
                    dir: `packages/${dir}`,
                    version: packageJson.version,
                    assets: packageJson.blog.assets,
                });
            }
        } catch {
            // Skip if package.json doesn't exist or is invalid
        }
    }

    return packages;
}

function generateManifest() {
    const packages = discoverPackages();
    const manifest = {};

    for (const pkg of packages) {
        const assets = {};

        for (const [key, filename] of Object.entries(pkg.assets)) {
            assets[key] = `/${pkg.name}/${pkg.version}/${filename}`;
        }

        manifest[pkg.name] = {
            version: pkg.version,
            ...assets,
        };
    }

    return manifest;
}

const manifest = generateManifest();
const outputPath = join(rootDir, "manifest.json");

writeFileSync(outputPath, JSON.stringify(manifest, null, 2) + "\n");

console.log("✅ Generated manifest.json:");
console.log(JSON.stringify(manifest, null, 2));
