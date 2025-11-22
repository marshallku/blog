import { defineConfig } from "tsup";

export default defineConfig({
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
});
