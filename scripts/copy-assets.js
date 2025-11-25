#!/usr/bin/env node

/**
 * Copies built assets to static directory with versioned paths
 * Creates directory structure: static/{package}/{version}/{files}
 *
 * Reads asset definitions from each package's package.json "blog.assets" field
 */

import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const rootDir = join(__dirname, "..");
const packagesDir = join(rootDir, "packages");
const staticDir = join(rootDir, "static");

// Parse --filter argument (e.g., --filter=styles)
const filterArg = process.argv.find((arg) => arg.startsWith("--filter="));
const filter = filterArg ? filterArg.split("=")[1] : null;

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

            if (packageJson.blog?.assets) {
                packages.push({
                    name: dir,
                    dir: join(packagesDir, dir),
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

function copyPackageAssets(pkg) {
    const srcDir = join(pkg.dir, "dist");
    const destDir = join(staticDir, pkg.name, pkg.version);

    if (!existsSync(srcDir)) {
        console.warn(`⚠️  ${pkg.name}: dist directory not found, skipping`);
        return;
    }

    mkdirSync(destDir, { recursive: true });

    let copied = 0;
    for (const [key, filename] of Object.entries(pkg.assets)) {
        const srcFile = join(srcDir, filename);
        const destFile = join(destDir, filename);

        if (existsSync(srcFile)) {
            cpSync(srcFile, destFile);
            copied++;
        } else {
            console.warn(`⚠️  ${pkg.name}: ${filename} (${key}) not found`);
        }
    }

    console.log(
        `✅ ${pkg.name}@${pkg.version}: copied ${copied} files to static/${pkg.name}/${pkg.version}/`
    );
}

function main() {
    console.log("📦 Copying assets to static directory...\n");

    let packages = discoverPackages();

    // Filter packages if --filter argument provided
    if (filter) {
        packages = packages.filter((pkg) => pkg.name === filter);
        if (packages.length === 0) {
            console.log(`No package found matching filter: ${filter}`);
            return;
        }
    }

    if (packages.length === 0) {
        console.log("No packages with blog.assets found.");
        return;
    }

    for (const pkg of packages) {
        copyPackageAssets(pkg);
    }

    console.log("\n✅ Done!");
}

main();
