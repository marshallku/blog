import { defineConfig } from "tsup";

export default defineConfig([
    {
        entry: {
            index: "src/index.ts",
            bundle: "src/bundle.ts",
        },
        format: ["esm"],
        dts: true,
        clean: true,
        minify: true,
        outDir: "dist",
        target: "es2022",
        splitting: false,
    },
    {
        entry: {
            comment: "src/comment.ts",
            like: "src/like.ts",
        },
        format: ["iife"],
        dts: false,
        minify: true,
        outDir: "dist",
        target: "es2022",
    },
    {
        entry: {
            islands: "src/islands/index.ts",
        },
        format: ["esm"],
        dts: false,
        minify: true,
        outDir: "dist",
        target: "es2022",
        splitting: true,
        noExternal: ["react", "react-dom"],
        define: {
            "process.env.NODE_ENV": '"production"',
        },
        esbuildOptions(options) {
            options.jsx = "automatic";
            options.jsxImportSource = "react";
        },
    },
]);
